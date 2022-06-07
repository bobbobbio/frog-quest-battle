// copyright 2022 Remi Bernotavicius

use bevy::app::ScheduleRunnerSettings;
use bevy::prelude::*;
use bevy::utils::Duration;
use euclid::{Point2D, Rect, Size2D, Vector2D};
use renderer::{CanvasRenderer, Color, Pixels, RENDER_RECT};
use std::cell::RefCell;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher as _};
use std::mem;
use std::rc::Rc;
use std::sync::mpsc::{channel, Receiver};
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

#[derive(Component)]
struct ColorComponent(Color);

impl ColorComponent {
    fn random(h: &impl Hash) -> Self {
        let mut s = DefaultHasher::new();
        h.hash(&mut s);
        let [r, g, b, ..] = s.finish().to_le_bytes();
        Self(Color { r, g, b })
    }
}

fn add_boxes(mut commands: Commands) {
    commands
        .spawn()
        .insert(Sprite)
        .insert(Bounds(Rect::new(Point2D::new(0, 0), Size2D::new(10, 10))))
        .insert(Velocity(Vector2D::new(3, 3)))
        .insert(ColorComponent::random(&0));
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

fn draw(
    mut renderer: NonSendMut<CanvasRenderer>,
    render_flag: NonSend<RenderFlag>,
    mut frame_counter: ResMut<FrameCounter>,
    query: Query<(&Bounds, &ColorComponent), With<Sprite>>,
) {
    frame_counter.incr();

    if !render_flag.should_render() {
        return;
    }

    for y in 0..renderer::RENDER_RECT.size.height as i32 {
        for x in 0..renderer::RENDER_RECT.size.width as i32 {
            let p = Point2D::new(x, y);
            let color = if let Some(c) = query.iter().find_map(|(o, c)| o.0.contains(p).then(|| c))
            {
                c.0
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
    renderer.render();
}

fn request_animation_frame(f: &Closure<dyn FnMut()>) {
    window()
        .request_animation_frame(f.as_ref().unchecked_ref())
        .expect("should register `requestAnimationFrame` OK");
}

#[derive(Clone)]
struct RenderFlag(Rc<RefCell<bool>>);

impl RenderFlag {
    fn new() -> Self {
        let flag = Rc::new(RefCell::new(false));

        let f = Rc::new(RefCell::new(None));
        let g = f.clone();

        let their_flag = flag.clone();
        *g.borrow_mut() = Some(Closure::wrap(Box::new(move || {
            *their_flag.borrow_mut() = true;

            // Schedule ourselves for another requestAnimationFrame callback.
            request_animation_frame(f.borrow().as_ref().unwrap());
        }) as Box<dyn FnMut()>));

        request_animation_frame(g.borrow().as_ref().unwrap());

        Self(flag)
    }

    fn should_render(&self) -> bool {
        mem::replace(&mut *self.0.borrow_mut(), false)
    }
}

impl Default for RenderFlag {
    fn default() -> Self {
        Self::new()
    }
}

fn handle_input(world: &mut World) {
    let mut stream = world.non_send_resource_mut::<InputStream>();
    let mut events = vec![];
    while let Some(e) = stream.get() {
        events.push(e);
    }

    for e in events {
        match e {
            KeyboardEvent::Down(_) => {
                let color = ColorComponent::random(&*world.resource::<FrameCounter>());
                world
                    .spawn()
                    .insert(Sprite)
                    .insert(Bounds(Rect::new(Point2D::new(0, 0), Size2D::new(10, 10))))
                    .insert(Velocity(Vector2D::new(3, 3)))
                    .insert(color);
            }
            _ => {}
        }
    }
}

enum KeyboardEvent {
    Up(web_sys::KeyboardEvent),
    Down(web_sys::KeyboardEvent),
}

struct InputStream(Receiver<KeyboardEvent>);

impl InputStream {
    fn new() -> Self {
        let (send, recv) = channel();

        let window = window();

        let their_sender = send.clone();
        let on_key_down = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
            event.prevent_default();
            their_sender.send(KeyboardEvent::Down(event)).ok();
        }) as Box<dyn FnMut(_)>);

        let their_sender = send.clone();
        let on_key_up = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
            event.prevent_default();
            their_sender.send(KeyboardEvent::Up(event)).ok();
        }) as Box<dyn FnMut(_)>);

        window
            .add_event_listener_with_callback("keydown", on_key_down.as_ref().unchecked_ref())
            .unwrap();
        window
            .add_event_listener_with_callback("keyup", on_key_up.as_ref().unchecked_ref())
            .unwrap();
        on_key_down.forget();
        on_key_up.forget();

        Self(recv)
    }

    fn get(&mut self) -> Option<KeyboardEvent> {
        self.0.try_recv().ok()
    }
}

impl Default for InputStream {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Default, Hash)]
struct FrameCounter(u64);

impl FrameCounter {
    fn incr(&mut self) {
        self.0 += 1;
    }
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

    App::new()
        .init_resource::<FrameCounter>()
        .init_non_send_resource::<RenderFlag>()
        .init_non_send_resource::<CanvasRenderer>()
        .init_non_send_resource::<InputStream>()
        .insert_resource(ScheduleRunnerSettings::run_loop(Duration::from_millis(16)))
        .add_plugins(MinimalPlugins)
        .add_startup_system(add_boxes)
        .add_system(physics)
        .add_system(draw)
        .add_system(handle_input.exclusive_system())
        .run();
}
