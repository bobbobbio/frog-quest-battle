// copyright 2022 Remi Bernotavicius

use bevy::app::ScheduleRunnerSettings;
use bevy::prelude::*;
use bevy::utils::Duration;
use euclid::{Point2D, Rect, Size2D, Vector2D};
use renderer::{CanvasRenderer, Color, Pixels, RENDER_RECT};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast as _;

mod renderer;

fn window() -> web_sys::Window {
    web_sys::window().expect("no global `window` exists")
}

fn canvas() -> web_sys::HtmlCanvasElement {
    let document = window().document().unwrap();
    let canvas = document.get_element_by_id("canvas").unwrap();
    canvas
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .map_err(|_| ())
        .unwrap()
}

#[derive(Component)]
struct Sprite;

#[derive(Component)]
struct Bounds(Rect<i32, Pixels>);

#[derive(Component)]
struct Velocity(Vector2D<i32, Pixels>);

fn add_boxes(mut commands: Commands) {
    commands
        .spawn()
        .insert(Sprite)
        .insert(Bounds(Rect::new(Point2D::new(0, 0), Size2D::new(10, 10))))
        .insert(Velocity(Vector2D::new(1, 1)));
}

fn physics(mut query: Query<(&mut Bounds, &mut Velocity), With<Sprite>>) {
    for (mut b, mut v) in query.iter_mut() {
        b.0.origin += v.0;

        if b.0.origin.y <= 0 || b.0.origin.y + b.0.size.height > RENDER_RECT.size.height {
            v.0.y *= -1
        }

        if !RENDER_RECT.intersects(&b.0) {
            b.0.origin.x = -b.0.size.width;
        }
    }
}

fn draw(renderer: ResMut<Arc<Mutex<CanvasRenderer>>>, query: Query<&Bounds, With<Sprite>>) {
    let mut renderer = renderer.lock().unwrap();

    for y in 0..renderer::RENDER_RECT.size.height as i32 {
        for x in 0..renderer::RENDER_RECT.size.width as i32 {
            let p = Point2D::new(x, y);
            let color = if let Some(_) = query.iter().find(|o| o.0.contains(p)) {
                Color { r: 255, g: 0, b: 0 }
            } else {
                Color {
                    r: x as u8,
                    g: y as u8,
                    b: x as u8,
                }
            };
            renderer.color_pixel(p, color);
        }
    }

    renderer.present();
}

fn request_animation_frame(f: &Closure<dyn FnMut()>) {
    window()
        .request_animation_frame(f.as_ref().unchecked_ref())
        .expect("should register `requestAnimationFrame` OK");
}

fn set_up_rendering(renderer: Arc<Mutex<CanvasRenderer>>) {
    let f = Rc::new(RefCell::new(None));
    let g = f.clone();

    *g.borrow_mut() = Some(Closure::wrap(Box::new(move || {
        renderer.lock().unwrap().render();

        // Schedule ourselves for another requestAnimationFrame callback.
        request_animation_frame(f.borrow().as_ref().unwrap());
    }) as Box<dyn FnMut()>));

    request_animation_frame(g.borrow().as_ref().unwrap());
}

#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
    wasm_logger::init(wasm_logger::Config::default());

    log::info!("Frog Quest Battle Starting");

    let canvas = canvas();
    let canvas_rect = renderer::RENDER_RECT * renderer::PIXEL_SCALE;
    canvas.set_width(canvas_rect.size.width as u32);
    canvas.set_height(canvas_rect.size.height as u32);

    let renderer = Arc::new(Mutex::new(CanvasRenderer::new(&canvas)));

    set_up_rendering(renderer.clone());

    App::new()
        .insert_resource(renderer)
        .insert_resource(ScheduleRunnerSettings::run_loop(Duration::from_millis(16)))
        .add_plugins(MinimalPlugins)
        .add_startup_system(add_boxes)
        .add_system(physics)
        .add_system(draw)
        .run();
}
