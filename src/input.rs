// copyright 2022 Remi Bernotavicius

use super::window;
use std::sync::mpsc::{channel, Receiver};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast as _;

pub enum KeyboardEvent {
    Up(web_sys::KeyboardEvent),
    Down(web_sys::KeyboardEvent),
}

pub struct InputStream(Receiver<KeyboardEvent>);

impl InputStream {
    pub fn new() -> Self {
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

    pub fn get(&mut self) -> Option<KeyboardEvent> {
        self.0.try_recv().ok()
    }
}

impl Default for InputStream {
    fn default() -> Self {
        Self::new()
    }
}