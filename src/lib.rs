// copyright 2022 Remi Bernotavicius

use bevy::app::ScheduleRunnerSettings;
use bevy::prelude::*;
use bevy::utils::Duration;
use input::InputStream;
use renderer::{CanvasRenderer, RENDER_RECT};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast as _;

mod game;
mod graphics;
mod input;
mod menu;
mod net;
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

fn resize_canvas() {
    let canvas = canvas();
    let canvas_rect = RENDER_RECT * renderer::PIXEL_SCALE;
    canvas.set_width(canvas_rect.size.width as u32);
    canvas.set_height(canvas_rect.size.height as u32);
}

#[derive(Default, Debug, Clone, Copy, Hash, PartialEq, Eq)]
enum AppState {
    #[default]
    Menu,
    Game,
}

#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
    wasm_logger::init(wasm_logger::Config::default());

    log::info!("Frog Quest Battle Starting");

    resize_canvas();

    App::new()
        .init_non_send_resource::<CanvasRenderer>()
        .init_non_send_resource::<InputStream>()
        .insert_resource(ScheduleRunnerSettings::run_loop(Duration::from_millis(16)))
        .add_state(AppState::default())
        .add_plugins(MinimalPlugins)
        .add_plugin(input::Plugin)
        .add_plugin(net::Plugin)
        .add_plugin(graphics::Plugin)
        .add_plugin(menu::Plugin)
        .add_plugin(game::Plugin)
        .run();
}

fn despawn_screen<T: Component>(to_despawn: Query<Entity, With<T>>, mut commands: Commands) {
    for entity in to_despawn.iter() {
        commands.entity(entity).despawn_recursive();
    }
}
