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
            .add_system_set(SystemSet::on_exit(self.state).with_system(despawn_screen::<OnGame>));
    }

    fn name(&self) -> &str {
        "main game"
    }
}

#[derive(Component)]
pub struct Player {
    pub handle: u32,
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
    pub fn spawn<'a, 'w, 's>(
        commands: &'a mut Commands<'w, 's>,
        handle: u32,
    ) -> EntityCommands<'w, 's, 'a> {
        let mut entity = commands.spawn();
        entity
            .insert(Self { handle: 0 })
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

pub(crate) fn move_player(input: EnumSet<Input>, _player: &Player, velocity: &mut Velocity) {
    let mut direction = Vector2D::new(0, 0);
    if input.contains(Input::Up) {
        direction.y -= 1;
    }
    if input.contains(Input::Down) {
        direction.y += 1;
    }
    if input.contains(Input::Left) {
        direction.x -= 1;
    }
    if input.contains(Input::Right) {
        direction.x += 1;
    }

    velocity.0 += direction;
}

pub fn physics(mut query: Query<(&mut Bounds, &mut Velocity, &Player)>) {
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
