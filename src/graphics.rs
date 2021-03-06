// copyright 2022 Remi Bernotavicius

use super::renderer::{CanvasRenderer, Color, Pixels, BLACK, RENDER_RECT};
use bevy::prelude::*;
use bevy::reflect::impl_reflect_value;
use bevy_ggrs::*;
use euclid::{Point2D, Rect, Size2D};

const FONT: &'static [u8] = include_bytes!("../assets/ImprovGOLD-v1.bmp");

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
            .add_system(flip_buffer.after("draw_sprites"));
    }

    fn name(&self) -> &str {
        "draw"
    }
}

struct Image(bmp::Image);

impl Image {
    fn from_bytes(bytes: &[u8]) -> Self {
        let mut r = bytes;
        Self(bmp::from_reader(&mut r).unwrap())
    }

    fn get_pixel(&self, p: Point2D<i32, Pixels>) -> Color {
        self.0
            .get_pixel(p.x.try_into().unwrap(), p.y.try_into().unwrap())
            .into()
    }

    fn size(&self) -> Size2D<i32, Pixels> {
        Size2D::new(
            self.0.get_width().try_into().unwrap(),
            self.0.get_height().try_into().unwrap(),
        )
    }
}

impl From<bmp::Pixel> for Color {
    fn from(p: bmp::Pixel) -> Self {
        Self {
            r: p.r,
            g: p.g,
            b: p.b,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct TileNumber(u32);

impl TileNumber {
    fn new(value: u32) -> Self {
        assert!(value < i32::MAX as u32, "invalid TileNumber {}", value);
        Self(value)
    }

    fn from_ascii(c: char) -> Self {
        assert!(c.is_ascii(), "can't display non-ASCII");
        if c == ' ' {
            Self::new(259)
        } else if c >= 'a' && c <= 'z' {
            Self::new(c as u32 - 'a' as u32)
        } else if c >= '0' && c <= '9' {
            Self::new(c as u32 - '0' as u32 + 52)
        } else {
            panic!("{} not in tile-set", c);
        }
    }
}

struct Tiles;

struct SpriteSheet {
    image: Image,
    tile_size: Size2D<i32, Pixels>,
    bounds: Rect<i32, Tiles>,
}

impl SpriteSheet {
    fn new(image: &'static [u8], tile_size: impl Into<Size2D<i32, Pixels>>) -> Self {
        let image = Image::from_bytes(image);
        let image_size = image.size();
        let tile_size = tile_size.into();
        let bounds = Rect::new(
            Point2D::new(0, 0),
            Size2D::new(
                (image_size.width / tile_size.width).try_into().unwrap(),
                (image_size.height / tile_size.height).try_into().unwrap(),
            ),
        );

        Self {
            image,
            tile_size,
            bounds,
        }
    }

    fn tile_start(&self, tile: TileNumber) -> Point2D<i32, Pixels> {
        let tile_num: i32 = tile.0.try_into().unwrap();
        let tile_point = Point2D::new(
            tile_num % self.bounds.size.width,
            tile_num / self.bounds.size.width,
        );

        assert!(
            self.bounds.contains(tile_point),
            "{:?} is outside the sheet {:?}",
            tile,
            &self.bounds
        );

        Point2D::new(
            tile_point.x * self.tile_size.width,
            tile_point.y * self.tile_size.height,
        )
    }

    fn draw_tile(
        &self,
        tile: TileNumber,
        p: Point2D<i32, Pixels>,
        color: Color,
        renderer: &mut CanvasRenderer,
    ) {
        for tile_pixel in self.tile_size.point_iter() {
            let tile_start = self.tile_start(tile);

            let pixel = self.image.get_pixel(tile_start + tile_pixel.to_vector());

            if pixel == BLACK {
                renderer.color_pixel(p + tile_pixel.to_vector(), color);
            }
        }
    }
}

pub struct Assets {
    font: SpriteSheet,
}

impl Default for Assets {
    fn default() -> Self {
        Self {
            font: SpriteSheet::new(FONT, (16, 16)),
        }
    }
}

#[derive(Component)]
pub struct TextBox(pub String, pub Color);

impl TextBox {
    pub fn new(text: impl Into<String>, color: Color) -> Self {
        Self(text.into(), color)
    }
}

impl Sprite for TextBox {
    fn draw(&self, bounds: &Bounds, assets: &Assets, renderer: &mut CanvasRenderer) {
        for (i, c) in self.0.chars().enumerate() {
            let tile = TileNumber::from_ascii(c.to_ascii_lowercase());
            let mut p = bounds.0.origin.clone();
            p.x += assets.font.tile_size.width * i as i32 / 2;
            assets.font.draw_tile(tile, p, self.1, renderer);
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
