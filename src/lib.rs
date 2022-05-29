// copyright 2022 Remi Bernotavicius

use program::Program;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

mod program;
mod renderer;
mod rpc;

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

fn request_animation_frame(f: &Closure<dyn FnMut()>) {
    window()
        .request_animation_frame(f.as_ref().unchecked_ref())
        .expect("should register `requestAnimationFrame` OK");
}

fn set_up_rendering(program: Rc<RefCell<Program>>) {
    let f = Rc::new(RefCell::new(None));
    let g = f.clone();

    *g.borrow_mut() = Some(Closure::wrap(Box::new(move || {
        program.borrow_mut().render();

        // Schedule ourselves for another requestAnimationFrame callback.
        request_animation_frame(f.borrow().as_ref().unwrap());
    }) as Box<dyn FnMut()>));

    request_animation_frame(g.borrow().as_ref().unwrap());
}

fn set_up_input(program: Rc<RefCell<Program>>) {
    let window = window();

    let their_program = program.clone();
    let on_key_down = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
        their_program.borrow_mut().on_key_down(&event.code());
        event.prevent_default();
    }) as Box<dyn FnMut(_)>);

    let their_program = program;
    let on_key_up = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
        their_program.borrow_mut().on_key_up(&event.code());
        event.prevent_default();
    }) as Box<dyn FnMut(_)>);

    window
        .add_event_listener_with_callback("keydown", on_key_down.as_ref().unchecked_ref())
        .unwrap();
    window
        .add_event_listener_with_callback("keyup", on_key_up.as_ref().unchecked_ref())
        .unwrap();
    on_key_down.forget();
    on_key_up.forget();
}

fn request_timeout(f: &Closure<dyn FnMut()>, from_now: i32) {
    window()
        .set_timeout_with_callback_and_timeout_and_arguments_0(f.as_ref().unchecked_ref(), from_now)
        .expect("should register `setTimeout` OK");
}

fn schedule<F: FnMut() -> i32 + 'static>(mut body: F, from_now: i32) {
    let f = Rc::new(RefCell::new(None));
    let g = f.clone();

    *g.borrow_mut() = Some(Closure::wrap(Box::new(move || {
        let next = body();
        request_timeout(f.borrow().as_ref().unwrap(), next);
    }) as Box<dyn FnMut()>));
    request_timeout(g.borrow().as_ref().unwrap(), from_now);
}

#[wasm_bindgen(start)]
pub async fn start() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    wasm_logger::init(wasm_logger::Config::default());

    log::info!("From Quest Battle Starting");

    let canvas = canvas();
    let canvas_rect = renderer::RENDER_RECT * renderer::PIXEL_SCALE;
    canvas.set_width(canvas_rect.size.width as u32);
    canvas.set_height(canvas_rect.size.height as u32);

    let program = Rc::new(RefCell::new(Program::new(&canvas)));
    set_up_rendering(program.clone());

    set_up_input(program.clone());

    schedule(move || program.borrow_mut().tick(), 0);

    rpc::start().await?;

    Ok(())
}
