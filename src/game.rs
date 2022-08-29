// copyright 2022 Remi Bernotavicius

use super::renderer::{CanvasRenderer, Color, Pixels, RENDER_RECT};
use super::{despawn_screen, graphics, input, AppState};
use bevy::diagnostic::{Diagnostics, DiagnosticsPlugin, FrameTimeDiagnosticsPlugin};
use bevy::ecs::system::EntityCommands;
use bevy::prelude::*;
use bevy::reflect::impl_reflect_value;
use bevy_ggrs::*;
use enumset::EnumSet;
use euclid::{Point2D, Rect, Size2D, Vector2D};
use graphics::{draw_sprites, Assets, Bounds, PointIterExt as _, Sprite, TextBox, PALLET};
use input::Input;
use std::cmp;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher as _};

#[derive(Component)]
struct OnGame;

#[derive(Default)]
pub struct GameStatus(String);

impl GameStatus {
    pub fn set_message(&mut self, message: impl Into<String>) {
        self.0 = message.into();
    }
}

pub struct Plugin {
    state: AppState,
}

impl Plugin {
    pub(super) fn new(state: AppState) -> Self {
        Self { state }
    }
}

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameStatus>()
            .init_resource::<FrameCounter>()
            .register_rollback_type::<Velocity>()
            .add_plugin(DiagnosticsPlugin)
            .add_plugin(FrameTimeDiagnosticsPlugin)
            .add_system_set(SystemSet::on_enter(self.state).with_system(spawn_sprites))
            .add_system_set(
                SystemSet::on_update(self.state).with_system(
                    draw_sprites::<Player>
                        .after("draw_background")
                        .label("draw_sprites"),
                ),
            )
            .add_system_set(SystemSet::on_update(self.state).with_system(FpsCounterTextBox::update))
            .add_system_set(SystemSet::on_update(self.state).with_system(GameStatusTextBox::update))
            .add_system_set(SystemSet::on_update(self.state).with_system(FrameCounter::update))
            .add_system_set(SystemSet::on_exit(self.state).with_system(despawn_screen::<OnGame>));
    }

    fn name(&self) -> &str {
        "main game"
    }
}

#[derive(Component)]
pub struct Player {
    pub handle: u32,
    last_flap_frame: u64,
}

fn arbitrary_color(h: &impl Hash) -> Color {
    let mut s = DefaultHasher::new();
    h.hash(&mut s);
    let index = s.finish();
    let colors = &PALLET[1..];

    colors[index as usize % colors.len()]
}

impl Sprite for Player {
    fn draw(&self, bounds: &Bounds, _assets: &Assets, renderer: &mut CanvasRenderer) {
        let color = arbitrary_color(&self.handle);

        for p in bounds.0.point_iter() {
            if RENDER_RECT.contains(p) {
                renderer.color_pixel(p, color);
            }
        }
    }
}

impl Player {
    fn new(handle: u32) -> Self {
        Self {
            handle,
            last_flap_frame: 0,
        }
    }
    pub fn spawn<'a, 'w, 's>(
        commands: &'a mut Commands<'w, 's>,
        handle: u32,
    ) -> EntityCommands<'w, 's, 'a> {
        let mut entity = commands.spawn();
        entity
            .insert(Self::new(handle))
            .insert(Bounds(Rect::new(
                Point2D::new(10 + handle as i32 * 20, 10),
                Size2D::new(10, 10),
            )))
            .insert(Velocity(Vector2D::zero()))
            .insert(OnGame);
        entity
    }
}

#[derive(Component, Clone, Default)]
pub struct Velocity(Vector2D<i32, Pixels>);

impl_reflect_value!(Velocity);

#[derive(Component)]
struct FpsCounterTextBox;

impl FpsCounterTextBox {
    pub fn spawn<'a, 'w, 's>(
        commands: &'a mut Commands<'w, 's>,
        pos: impl Into<Point2D<i32, Pixels>>,
        color: Color,
    ) -> EntityCommands<'w, 's, 'a> {
        let mut entity = TextBox::spawn(commands, "fps", pos, color);
        entity.insert(Self);
        entity
    }

    fn update(diagnostics: Res<Diagnostics>, mut query: Query<&mut TextBox, With<Self>>) {
        let fps_diagnostic = diagnostics.get(FrameTimeDiagnosticsPlugin::FPS).unwrap();
        if let Some(fps_avg) = fps_diagnostic.average() {
            for mut tb in query.iter_mut() {
                tb.text = format!("{} fps", fps_avg as u32);
            }
        }
    }
}

#[derive(Component)]
pub struct GameStatusTextBox;

impl GameStatusTextBox {
    fn spawn<'a, 'w, 's>(
        commands: &'a mut Commands<'w, 's>,
        pos: impl Into<Point2D<i32, Pixels>>,
        color: Color,
    ) -> EntityCommands<'w, 's, 'a> {
        let mut entity = TextBox::spawn(commands, "", pos, color);
        entity.insert(Self);
        entity
    }

    fn update(status: Res<GameStatus>, mut query: Query<&mut TextBox, With<Self>>) {
        for mut tb in query.iter_mut() {
            tb.text = status.0.clone();
        }
    }
}

pub fn spawn_sprites(mut commands: Commands) {
    TextBox::spawn(&mut commands, "hello world", (10, 40), PALLET[2]).insert(OnGame);
    GameStatusTextBox::spawn(&mut commands, (10, 150), PALLET[2]).insert(OnGame);
    FpsCounterTextBox::spawn(&mut commands, (10, 100), PALLET[2]).insert(OnGame);
}

pub(crate) fn move_player(
    frame_counter: &FrameCounter,
    input: EnumSet<Input>,
    player: &mut Player,
    velocity: &mut Velocity,
) {
    let mut direction = Vector2D::new(0, 0);
    if input.contains(Input::Primary) {
        if frame_counter.0 - player.last_flap_frame > 5 {
            direction.y -= 2;
            player.last_flap_frame = frame_counter.0;
        }
    }
    if input.contains(Input::Left) {
        direction.x -= 1;
    }
    if input.contains(Input::Right) {
        direction.x += 1;
    }

    velocity.0 += direction;

    // clamp horizontal velocity to 2
    if velocity.0.x > 0 {
        velocity.0.x = cmp::min(2, velocity.0.x);
    } else {
        velocity.0.x = cmp::max(-2, velocity.0.x);
    }
}

#[derive(Default)]
pub struct FrameCounter(u64);

impl FrameCounter {
    fn update(mut self_: ResMut<Self>) {
        self_.0 += 1;
    }
}

// gravity of 1 pixel downward per frame ^2
const GRAVITY: Vector2D<i32, Pixels> = Vector2D::new(0, 1);

pub fn physics(
    frame_counter: &FrameCounter,
    mut query: Query<(&mut Bounds, &mut Velocity, &mut Player)>,
) {
    for (mut b, mut v, _) in query.iter_mut() {
        // apply the velocity
        b.0.origin += v.0;

        let above_ceiling = b.0.origin.y <= 0;
        let below_ground = b.0.origin.y + b.0.size.height > RENDER_RECT.size.height;

        // hitting the ceiling bounces you
        if above_ceiling {
            v.0.y *= -1;
            v.0.y /= 2;
            b.0.origin.y = 0;
        }

        if below_ground {
            // hitting the ground stops you from falling
            b.0.origin.y = RENDER_RECT.size.height - b.0.size.height;
            v.0.y = 0;
        }

        let on_ground = b.0.origin.y + b.0.size.height == RENDER_RECT.size.height;

        if on_ground {
            // being on the ground causes a degredation of lateral movement in
            // the direction of movement due to friction
            if frame_counter.0 % 20 == 0 {
                if v.0.x > 0 {
                    v.0.x -= 1;
                } else if v.0.x < 0 {
                    v.0.x += 1;
                }
            }
        } else if frame_counter.0 % 20 == 0 {
            // apply gravity to the velocity if not on the ground
            v.0 += GRAVITY;
        }

        // screen wrapping
        if b.0.origin.x > RENDER_RECT.width() {
            b.0.origin.x -= RENDER_RECT.width() + b.0.size.width;
        }
        if b.0.origin.x < -b.0.size.width {
            b.0.origin.x += RENDER_RECT.width() + b.0.size.width;
        }
    }
}
