// copyright 2022 Remi Bernotavicius

use super::window;
use bevy::prelude::*;
use enumset::EnumSetType;
use gilrs::ev::{Axis, Button, EventType};
use std::sync::mpsc::{channel, Receiver, Sender};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast as _;

#[derive(EnumSetType, Debug)]
pub enum Input {
    Up,
    Down,
    Left,
    Right,
    Primary,
}

pub struct InputStream {
    send: Sender<Input>,
    recv: Receiver<Input>,
}

fn input_from_keyboard_event(e: &web_sys::KeyboardEvent) -> Option<Input> {
    match e {
        e if e.code() == "ArrowUp" => Some(Input::Up),
        e if e.code() == "ArrowDown" => Some(Input::Down),
        e if e.code() == "ArrowLeft" => Some(Input::Left),
        e if e.code() == "ArrowRight" => Some(Input::Right),
        e if e.code() == "Enter" => Some(Input::Primary),
        _ => None,
    }
}

fn keyboard_source(send: Sender<Input>) {
    let window = window();

    let on_key_down = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
        if let Some(i) = input_from_keyboard_event(&event) {
            if send.send(i).is_ok() {
                event.prevent_default();
            }
        }
    }) as Box<dyn FnMut(_)>);

    window
        .add_event_listener_with_callback("keydown", on_key_down.as_ref().unchecked_ref())
        .unwrap();
    on_key_down.forget();
}

impl InputStream {
    pub fn new() -> Self {
        let (send, recv) = channel();

        keyboard_source(send.clone());

        Self { send, recv }
    }

    pub fn get(&mut self) -> Option<Input> {
        self.recv.try_recv().ok()
    }

    pub fn put(&mut self, input: Input) {
        self.send.send(input).ok();
    }
}

impl Default for InputStream {
    fn default() -> Self {
        Self::new()
    }
}

fn input_from_controller_button(button: gilrs::ev::Button) -> Option<Input> {
    match button {
        Button::East => Some(Input::Primary),
        Button::DPadUp => Some(Input::Up),
        Button::DPadDown => Some(Input::Down),
        Button::DPadLeft => Some(Input::Left),
        Button::DPadRight => Some(Input::Right),
        _ => None,
    }
}

fn drive_controller(mut input_stream: NonSendMut<InputStream>, mut grs: NonSendMut<gilrs::Gilrs>) {
    while let Some(event) = grs.next_event() {
        match event.event {
            EventType::ButtonPressed(button, _) => {
                if let Some(b) = input_from_controller_button(button) {
                    input_stream.put(b);
                }
            }
            EventType::AxisChanged(Axis::LeftStickX, v, _) if v > 0.0 => {
                input_stream.put(Input::Right);
            }
            EventType::AxisChanged(Axis::LeftStickX, v, _) if v < 0.0 => {
                input_stream.put(Input::Left);
            }
            EventType::AxisChanged(Axis::LeftStickY, v, _) if v > 0.0 => {
                input_stream.put(Input::Up);
            }
            EventType::AxisChanged(Axis::LeftStickY, v, _) if v < 0.0 => {
                input_stream.put(Input::Down);
            }
            _ => (),
        };
    }
}

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.init_non_send_resource::<InputStream>()
            .insert_non_send_resource(gilrs::Gilrs::new().unwrap())
            .add_system(drive_controller);
    }

    fn name(&self) -> &str {
        "input"
    }
}
