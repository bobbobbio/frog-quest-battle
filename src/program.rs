// copyright 2022 Remi Bernotavicius

use crate::renderer::{CanvasRenderer, Color, Pixels, RENDER_RECT};
use euclid::{Point2D, Rect, Size2D, Vector2D};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash as _, Hasher as _};

struct Object {
    rect: Rect<i32, Pixels>,
    velocity: Vector2D<i32, Pixels>,
    color: Color,
}

pub struct Program {
    renderer: CanvasRenderer,
    objects: Vec<Object>,
    counter: u64,
}

impl Program {
    pub fn new(canvas: &web_sys::HtmlCanvasElement) -> Self {
        let mut s = Self {
            renderer: CanvasRenderer::new(canvas),
            objects: vec![],
            counter: 0,
        };
        s.add_object();
        s
    }

    fn add_object(&mut self) {
        let mut s = DefaultHasher::new();
        self.counter.hash(&mut s);
        let [r, g, b, ..] = s.finish().to_le_bytes();

        self.objects.push(Object {
            rect: Rect::new(Point2D::new(0, 0), Size2D::new(10, 10)),
            velocity: Vector2D::new(1, 1),
            color: Color { r, g, b },
        });
    }

    pub fn on_key_down(&mut self, _code: &str) {
        self.add_object()
    }

    pub fn on_key_up(&mut self, _code: &str) {}

    pub fn render(&self) {
        self.renderer.render();
    }

    fn draw(&mut self) {
        for y in 0..crate::renderer::RENDER_RECT.size.height as i32 {
            for x in 0..crate::renderer::RENDER_RECT.size.width as i32 {
                let p = Point2D::new(x, y);
                let color = if let Some(o) = self.objects.iter().find(|o| o.rect.contains(p)) {
                    o.color
                } else {
                    Color {
                        r: x as u8,
                        g: y as u8,
                        b: x as u8,
                    }
                };
                self.renderer.color_pixel(p, color);
            }
        }
        self.renderer.present();
    }

    pub fn tick(&mut self) -> i32 {
        self.draw();

        self.counter += 13;

        for o in &mut self.objects {
            o.rect.origin += o.velocity;

            if o.rect.origin.y <= 0
                || o.rect.origin.y + o.rect.size.height > RENDER_RECT.size.height
            {
                o.velocity.y *= -1
            }

            if !RENDER_RECT.intersects(&o.rect) {
                o.rect.origin.x = -o.rect.size.width;
            }
        }

        10
    }
}
