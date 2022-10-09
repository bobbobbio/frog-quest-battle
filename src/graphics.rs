// copyright 2022 Remi Bernotavicius

use super::renderer::{CanvasRenderer, Color, Pixels, RENDER_RECT};
use bevy::ecs::system::EntityCommands;
use bevy::prelude::*;
use bevy::reflect::impl_reflect_value;
use bevy_ggrs::*;
use euclid::{Point2D, Rect, Size2D};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use wasm_bindgen::JsValue;

pub struct PointIterator<T, U> {
    i: Point2D<T, U>,
    rect: Rect<T, U>,
}

impl<T, U> PointIterator<T, U>
where
    T: num_traits::int::PrimInt,
{
    fn advance(&mut self) {
        use num_traits::identities::One;

        let right_side = self.rect.origin.x + self.rect.size.width - One::one();
        if self.i.x == right_side {
            self.i.x = self.rect.origin.x;
            self.i.y = self.i.y + One::one();
        } else {
            self.i.x = self.i.x + One::one();
        }
    }
}

impl<T, U> Iterator for PointIterator<T, U>
where
    T: num_traits::int::PrimInt,
{
    type Item = Point2D<T, U>;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.rect.contains(self.i) {
            None
        } else {
            let ret = self.i.clone();
            self.advance();
            Some(ret)
        }
    }
}

pub trait PointIterExt<T, U> {
    fn point_iter(&self) -> PointIterator<T, U>;
}

impl<T, U> PointIterExt<T, U> for Rect<T, U>
where
    T: num_traits::int::PrimInt,
{
    fn point_iter(&self) -> PointIterator<T, U> {
        PointIterator {
            i: self.origin.clone(),
            rect: self.clone(),
        }
    }
}

impl<T, U> PointIterExt<T, U> for Size2D<T, U>
where
    T: num_traits::int::PrimInt,
{
    fn point_iter(&self) -> PointIterator<T, U> {
        PointIterator {
            i: Point2D::origin(),
            rect: self.clone().into(),
        }
    }
}

#[derive(Component, Clone, Default)]
pub struct Bounds(pub Rect<i32, Pixels>);

impl_reflect_value!(Bounds);

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Assets>()
            .register_rollback_type::<Bounds>()
            .add_system(draw_background.label("draw_background"))
            .add_system(
                draw_sprites::<TextBox>
                    .after("draw_background")
                    .label("draw_sprites"),
            )
            .add_system(
                draw_sprites::<SimpleSprite>
                    .after("draw_background")
                    .label("draw_sprites"),
            )
            .add_system(flip_buffer.after("draw_sprites"));
    }

    fn name(&self) -> &str {
        "draw"
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PalletColor {
    Color1,
    Color2,
    Color3,
    Color4,
}

#[derive(Serialize, Deserialize)]
pub struct SpriteData {
    pub size: Size2D<i32, Pixels>,
    pub data: Vec<PalletColor>,
}

impl SpriteData {
    fn rect(&self) -> Rect<i32, Pixels> {
        self.size.into()
    }

    pub fn get_pixel(&self, pos: Point2D<i32, Pixels>) -> PalletColor {
        assert!(self.rect().contains(pos));
        self.data[usize::try_from(pos.y * self.size.width + pos.x).unwrap()]
    }
}

#[derive(Copy, Clone)]
pub enum TileKey {
    Char(char),
    Str(&'static str),
}

impl From<char> for TileKey {
    fn from(c: char) -> Self {
        Self::Char(c)
    }
}

impl From<&'static str> for TileKey {
    fn from(s: &'static str) -> Self {
        Self::Str(s)
    }
}

impl fmt::Display for TileKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Char(c) => write!(f, "{c}"),
            Self::Str(s) => write!(f, "{s}"),
        }
    }
}

#[derive(Default, Serialize, Deserialize)]
pub struct SpriteSheet {
    sprites: HashMap<String, SpriteData>,
}

impl SpriteSheet {
    pub fn save_to_file(&self, window: &web_sys::Window) -> Result<(), JsValue> {
        let bytes = bincode::serialize(self).unwrap();
        let u8_array = js_sys::Uint8Array::new_with_length(bytes.len() as u32);
        u8_array.copy_from(&bytes);
        let array = js_sys::Array::new_with_length(1);
        array.set(0, u8_array.buffer().into());
        let blob = web_sys::Blob::new_with_buffer_source_sequence_and_options(
            &array,
            web_sys::BlobPropertyBag::new().type_("application/octet-stream"),
        )?;
        let url = web_sys::Url::create_object_url_with_blob(&blob)?;
        window.location().set_href(&url)?;

        Ok(())
    }

    fn get_sprite_data(&self, tile: TileKey) -> Option<&SpriteData> {
        match tile {
            TileKey::Char(c) => self.sprites.get(&c.to_string()),
            TileKey::Str(s) => self.sprites.get(s),
        }
    }

    pub fn insert_sprite(&mut self, tile: TileKey, data: SpriteData) {
        match tile {
            TileKey::Char(c) => self.sprites.insert(c.to_string(), data),
            TileKey::Str(s) => self.sprites.insert(s.into(), data),
        };
    }

    fn draw_tile(
        &self,
        tile: TileKey,
        p: Point2D<i32, Pixels>,
        color: Color,
        renderer: &mut CanvasRenderer,
    ) -> Size2D<i32, Pixels> {
        let data = self
            .get_sprite_data(tile)
            .ok_or_else(|| format!("tile key {tile} not found"))
            .unwrap();

        for tile_pixel in data.size.point_iter() {
            let pixel = data.get_pixel(tile_pixel);
            if pixel == PalletColor::Color2 {
                renderer.color_pixel(p + tile_pixel.to_vector(), color);
            }
        }

        data.size
    }
}

pub struct Assets {
    font: SpriteSheet,
}

impl Default for Assets {
    fn default() -> Self {
        let s = Self {
            font: bincode::deserialize(include_bytes!("../assets/font.bin")).unwrap(),
        };
        let mut keys = s.font.sprites.keys().cloned().collect::<Vec<String>>();
        keys.sort();
        log::info!("loaded font with keys: {keys:?}",);
        s
    }
}

#[derive(Component)]
pub struct SimpleSprite {
    pub tile: TileKey,
    pub color: Color,
}

impl Sprite for SimpleSprite {
    fn draw(&self, bounds: &Bounds, assets: &Assets, renderer: &mut CanvasRenderer) {
        assets
            .font
            .draw_tile(self.tile, bounds.0.origin.clone(), self.color, renderer);
    }
}

#[derive(Component)]
pub struct TextBox {
    pub text: String,
    pub color: Color,
}

impl TextBox {
    pub fn new(text: impl Into<String>, color: Color) -> Self {
        Self {
            text: text.into(),
            color,
        }
    }

    pub fn spawn<'a, 'w, 's>(
        commands: &'a mut Commands<'w, 's>,
        text: impl Into<String>,
        pos: impl Into<Point2D<i32, Pixels>>,
        color: Color,
    ) -> EntityCommands<'w, 's, 'a> {
        let mut entity = commands.spawn();
        entity
            .insert(TextBox::new(text, color))
            .insert(Bounds(Rect::new(pos.into(), Size2D::new(100, 10))));
        entity
    }
}

impl Sprite for TextBox {
    fn draw(&self, bounds: &Bounds, assets: &Assets, renderer: &mut CanvasRenderer) {
        let mut p = bounds.0.origin.clone();
        for c in self.text.chars() {
            let size = assets.font.draw_tile(c.into(), p, self.color, renderer);
            p.x += size.width;
        }
    }
}

pub trait Sprite {
    fn draw(&self, bounds: &Bounds, assets: &Assets, renderer: &mut CanvasRenderer);
}

pub const PALLET: [Color; 4] = [
    Color { r: 6, g: 35, b: 39 },
    Color {
        r: 28,
        g: 124,
        b: 148,
    },
    Color {
        r: 254,
        g: 160,
        b: 0,
    },
    Color {
        r: 250,
        g: 232,
        b: 150,
    },
];

const BG_COLOR: Color = PALLET[0];

fn draw_background(mut renderer: NonSendMut<CanvasRenderer>) {
    for p in RENDER_RECT.point_iter() {
        renderer.color_pixel(p, BG_COLOR);
    }
}

pub fn draw_sprites<S: Sprite + Component>(
    assets: Res<Assets>,
    mut renderer: NonSendMut<CanvasRenderer>,
    query: Query<(&Bounds, &S)>,
) {
    for (b, s) in query.iter() {
        s.draw(b, &*assets, &mut *renderer);
    }
}

fn flip_buffer(mut renderer: NonSendMut<CanvasRenderer>) {
    renderer.present();
    renderer.render();
}
