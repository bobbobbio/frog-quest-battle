// copyright 2022 Remi Bernotavicius

use bevy::app::ScheduleRunnerSettings;
use bevy::prelude::*;
use bevy::utils::Duration;
use renderer::{CanvasRenderer, RENDER_RECT};
use std::sync::mpsc::{channel, Receiver};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast as _;

mod net;
mod game;
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

#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
    wasm_logger::init(wasm_logger::Config::default());

    log::info!("Frog Quest Battle Starting");

    let canvas = canvas();
    let canvas_rect = RENDER_RECT * renderer::PIXEL_SCALE;
    canvas.set_width(canvas_rect.size.width as u32);
    canvas.set_height(canvas_rect.size.height as u32);

    App::new()
        .init_non_send_resource::<CanvasRenderer>()
        .init_non_send_resource::<InputStream>()
        .insert_resource(ScheduleRunnerSettings::run_loop(Duration::from_millis(16)))
        .add_plugins(MinimalPlugins)
        .add_plugin(net::Plugin)
        .run();
}
