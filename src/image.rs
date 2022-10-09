// copyright 2022 Remi Bernotavicius

use super::graphics::{PalletColor, PointIterExt as _, SpriteData, SpriteSheet};
use super::renderer::{Color, Pixels};
use euclid::{Point2D, Rect, Size2D};

const FONT: &'static [u8] = include_bytes!("../assets/ImprovGOLD-v1.bmp");
pub const BLACK: Color = Color { r: 0, g: 0, b: 0 };

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
pub struct TileNumber(u32);

impl TileNumber {
    pub fn new(value: u32) -> Self {
        assert!(value < i32::MAX as u32, "invalid TileNumber {}", value);
        Self(value)
    }

    fn from_ascii(c: char) -> Self {
        assert!(c.is_ascii(), "can't display non-ASCII");
        let special_chars = ".,;:?!-_~#z'&()[]{}^|`/\\@*+=z%z$zz<>";
        if c == ' ' {
            Self::new(259)
        } else if c >= 'a' && c <= 'z' {
            Self::new(c as u32 - 'a' as u32)
        } else if c >= '0' && c <= '9' {
            Self::new(c as u32 - '0' as u32 + 52)
        } else if let Some(i) = special_chars.find(c) {
            Self::new((i + 62) as u32)
        } else {
            panic!("{} not in tile-set", c);
        }
    }
}

struct Tiles;

struct ImageSpriteSheet {
    image: Image,
    tile_size: Size2D<i32, Pixels>,
    bounds: Rect<i32, Tiles>,
}

impl ImageSpriteSheet {
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

    fn tile_to_data(&self, tile: TileNumber) -> SpriteData {
        SpriteData {
            size: self.tile_size,
            data: self
                .tile_size
                .point_iter()
                .map(|tile_pixel| {
                    let tile_start = self.tile_start(tile);

                    let pixel = self.image.get_pixel(tile_start + tile_pixel.to_vector());

                    if pixel == BLACK {
                        PalletColor::Color2
                    } else {
                        PalletColor::Color1
                    }
                })
                .collect(),
        }
    }
}

#[allow(dead_code)]
pub fn save_font(window: &web_sys::Window) {
    let mut sheet = SpriteSheet::default();

    let font = ImageSpriteSheet::new(FONT, (16, 16));
    for c in "abcdefghijklmnopqrstuvwxyz0123456789 .,;:?!-_~#'&()[]{}^|`/\\@*+=$%<>".chars() {
        let tile = TileNumber::from_ascii(c);
        let mut sprite_data = font.tile_to_data(tile);

        let mut new_size = sprite_data.size.clone();
        new_size.width /= 2;
        let mut new_data = vec![];
        for p in new_size.point_iter() {
            new_data.push(sprite_data.get_pixel(p));
        }
        sprite_data.size = new_size;
        sprite_data.data = new_data;

        log::info!("saving sprite with key {c:?}");
        sheet.insert_sprite(c.into(), sprite_data);
    }

    sheet.save_to_file(window).unwrap();
}
