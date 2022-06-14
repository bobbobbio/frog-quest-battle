// copyright 2022 Remi Bernotavicius

use bevy::app::ScheduleRunnerSettings;
use bevy::prelude::*;
use bevy::tasks::IoTaskPool;
use bevy::utils::Duration;
use bevy_ggrs::*;
use enumset::{EnumSet, EnumSetType};
use ggrs::PlayerType;
use matchbox_socket::WebRtcNonBlockingSocket;
use renderer::{CanvasRenderer, RENDER_RECT};
use std::cell::RefCell;
use std::mem;
use std::rc::Rc;
use std::sync::mpsc::{channel, Receiver};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast as _;

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

#[derive(EnumSetType)]
enum Input {
    Up,
    Down,
    Left,
    Right,
}

fn input(_: In<ggrs::PlayerHandle>, mut input_stream: NonSendMut<InputStream>) -> Vec<u8> {
    let mut set = EnumSet::new();

    while let Some(i) = input_stream.get() {
        match i {
            KeyboardEvent::Down(e) if e.code() == "ArrowUp" => {
                set.insert(Input::Up);
            }
            KeyboardEvent::Down(e) if e.code() == "ArrowDown" => {
                set.insert(Input::Down);
            }
            KeyboardEvent::Down(e) if e.code() == "ArrowLeft" => {
                set.insert(Input::Left);
            }
            KeyboardEvent::Down(e) if e.code() == "ArrowRight" => {
                set.insert(Input::Right);
            }
            _ => {}
        }
    }

    vec![set.as_u8()]
}

fn move_players(
    inputs: Res<Vec<ggrs::GameInput>>,
    mut player_query: Query<(&mut game::Bounds, &mut game::Velocity, &game::Player)>,
) {
    for (_, mut velocity, player) in player_query.iter_mut() {
        let input = EnumSet::from_u8(inputs[player.handle].buffer[0]);
        game::move_player(input, player, &mut velocity);
    }

    game::physics(player_query);
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

fn start_matchbox_socket(mut commands: Commands, task_pool: Res<IoTaskPool>) {
    let room_url = "ws://orange:3536/next_2";
    log::info!("connecting to matchbox server: {:?}", room_url);
    let (socket, message_loop) = WebRtcNonBlockingSocket::new(room_url);

    // The message loop needs to be awaited, or nothing will happen.
    // We do this here using bevy's task system.
    task_pool.spawn(message_loop).detach();

    commands.insert_resource(Some(socket));
}

fn wait_for_players(mut commands: Commands, mut socket: ResMut<Option<WebRtcNonBlockingSocket>>) {
    let socket = socket.as_mut();

    // If there is no socket we've already started the game
    if socket.is_none() {
        return;
    }

    // Check for new connections
    socket.as_mut().unwrap().accept_new_connections();
    let players = socket.as_ref().unwrap().players();

    let num_players = 2;
    if players.len() < num_players {
        return; // wait for more players
    }

    log::info!("All peers have joined, going in-game");

    // consume the socket (currently required because GGRS takes ownership of its socket)
    let socket = socket.take().unwrap();

    let max_prediction = 12;

    // create a GGRS P2P session
    let mut p2p_session = ggrs::P2PSession::new_with_socket(
        num_players as u32,
        mem::size_of::<u8>(),
        max_prediction,
        socket,
    );

    for (i, player) in players.into_iter().enumerate() {
        p2p_session
            .add_player(player, i)
            .expect("failed to add player");

        if player == PlayerType::Local {
            // set input delay for the local player
            p2p_session.set_frame_delay(2, i).unwrap();
        }
    }

    // start the GGRS session
    commands.start_p2p_session(p2p_session);
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
        .init_non_send_resource::<RenderFlag>()
        .init_non_send_resource::<CanvasRenderer>()
        .init_non_send_resource::<InputStream>()
        .insert_resource(ScheduleRunnerSettings::run_loop(Duration::from_millis(16)))
        .add_plugins(MinimalPlugins)
        .add_plugin(GGRSPlugin)
        .with_rollback_schedule(Schedule::default().with_stage(
            "ROLLBACK_STAGE",
            SystemStage::single_threaded().with_system(move_players),
        ))
        .register_rollback_type::<game::Bounds>()
        .register_rollback_type::<game::Velocity>()
        .with_input_system(input)
        .add_startup_system(start_matchbox_socket)
        .add_startup_system(game::spawn_players)
        .add_system(wait_for_players)
        .add_system(game::draw)
        .run();
}
