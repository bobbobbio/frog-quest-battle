// copyright 2022 Remi Bernotavicius

use bevy::app::ScheduleRunnerSettings;
use bevy::prelude::*;
use bevy::reflect::impl_reflect_value;
use bevy::tasks::IoTaskPool;
use bevy::utils::Duration;
use bevy_ggrs::*;
use euclid::{Point2D, Rect, Size2D, Vector2D};
use ggrs::PlayerType;
use matchbox_socket::WebRtcNonBlockingSocket;
use renderer::{CanvasRenderer, Color, Pixels, RENDER_RECT};
use std::cell::RefCell;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher as _};
use std::mem;
use std::rc::Rc;
use std::sync::mpsc::{channel, Receiver};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast as _;

const INPUT_SIZE: usize = std::mem::size_of::<u8>();

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
struct Player {
    handle: usize,
}

#[derive(Component, Clone, Default)]
struct Bounds(Rect<i32, Pixels>);

impl_reflect_value!(Bounds);

#[derive(Component, Clone, Default)]
struct Velocity(Vector2D<i32, Pixels>);

impl_reflect_value!(Velocity);

#[derive(Component)]
struct ColorComponent(Color);

impl ColorComponent {
    fn arbitrary(h: &impl Hash) -> Self {
        let mut s = DefaultHasher::new();
        h.hash(&mut s);
        let [r, g, b, ..] = s.finish().to_le_bytes();
        Self(Color { r, g, b })
    }
}

fn spawn_players(mut commands: Commands, mut rip: ResMut<RollbackIdProvider>) {
    commands
        .spawn()
        .insert(Player { handle: 0 })
        .insert(Bounds(Rect::new(Point2D::new(10, 10), Size2D::new(10, 10))))
        .insert(Velocity(Vector2D::zero()))
        .insert(ColorComponent::arbitrary(&0))
        .insert(Rollback::new(rip.next_id()));

    commands
        .spawn()
        .insert(Player { handle: 1 })
        .insert(Bounds(Rect::new(Point2D::new(30, 10), Size2D::new(10, 10))))
        .insert(Velocity(Vector2D::zero()))
        .insert(ColorComponent::arbitrary(&1))
        .insert(Rollback::new(rip.next_id()));
}

fn draw(
    mut renderer: NonSendMut<CanvasRenderer>,
    render_flag: NonSend<RenderFlag>,
    query: Query<(&Bounds, &ColorComponent), With<Player>>,
) {
    if !render_flag.should_render() {
        return;
    }

    for y in 0..RENDER_RECT.size.height as i32 {
        for x in 0..RENDER_RECT.size.width as i32 {
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

const INPUT_UP: u8 = 1 << 0;
const INPUT_DOWN: u8 = 1 << 1;
const INPUT_LEFT: u8 = 1 << 2;
const INPUT_RIGHT: u8 = 1 << 3;

fn input(_: In<ggrs::PlayerHandle>, mut input_stream: NonSendMut<InputStream>) -> Vec<u8> {
    let mut input = 0u8;

    while let Some(i) = input_stream.get() {
        match i {
            KeyboardEvent::Down(e) if e.code() == "ArrowUp" => {
                input |= INPUT_UP;
            }
            KeyboardEvent::Down(e) if e.code() == "ArrowDown" => {
                input |= INPUT_DOWN;
            }
            KeyboardEvent::Down(e) if e.code() == "ArrowLeft" => {
                input |= INPUT_LEFT;
            }
            KeyboardEvent::Down(e) if e.code() == "ArrowRight" => {
                input |= INPUT_RIGHT;
            }
            _ => {}
        }
    }

    vec![input]
}

fn move_player(
    inputs: Res<Vec<ggrs::GameInput>>,
    mut player_query: Query<(&mut Bounds, &mut Velocity, &Player)>,
) {
    for (_, mut velocity, player) in player_query.iter_mut() {
        let mut direction = Vector2D::new(0, 0);

        let input = inputs[player.handle].buffer[0];

        if input & INPUT_UP != 0 {
            direction.y -= 1;
        }
        if input & INPUT_DOWN != 0 {
            direction.y += 1;
        }
        if input & INPUT_RIGHT != 0 {
            direction.x += 1;
        }
        if input & INPUT_LEFT != 0 {
            direction.x -= 1;
        }

        velocity.0 += direction;
    }

    physics(player_query);
}

fn physics(mut query: Query<(&mut Bounds, &mut Velocity, &Player)>) {
    for (mut b, mut v, _) in query.iter_mut() {
        b.0.origin += v.0;

        if b.0.origin.y <= 0 || b.0.origin.y + b.0.size.height > RENDER_RECT.size.height {
            v.0.y *= -1
        }

        if !RENDER_RECT.intersects(&b.0) {
            b.0.origin.x = -b.0.size.width;
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
    let mut p2p_session =
        ggrs::P2PSession::new_with_socket(num_players as u32, INPUT_SIZE, max_prediction, socket);

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
            SystemStage::single_threaded().with_system(move_player),
        ))
        .register_rollback_type::<Bounds>()
        .register_rollback_type::<Velocity>()
        .with_input_system(input)
        .add_startup_system(start_matchbox_socket)
        .add_startup_system(spawn_players)
        .add_system(wait_for_players)
        .add_system(draw)
        .run();
}
