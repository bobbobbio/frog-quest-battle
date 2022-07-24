// copyright 2022 Remi Bernotavicius

use super::renderer::{CanvasRenderer, Color, Pixels, RENDER_RECT};
use super::{despawn_screen, graphics, net, AppState};
use bevy::diagnostic::{Diagnostics, DiagnosticsPlugin, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::reflect::impl_reflect_value;
use bevy_ggrs::*;
use enumset::EnumSet;
use euclid::{Point2D, Rect, Size2D, Vector2D};
use graphics::{draw_sprites, Assets, Bounds, PointIterExt as _, Sprite, TextBox, PALLET};
use net::Input;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher as _};

#[derive(Component)]
struct OnGame;

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.register_rollback_type::<Velocity>()
            .add_plugin(DiagnosticsPlugin)
            .add_plugin(FrameTimeDiagnosticsPlugin)
            .add_system_set(SystemSet::on_enter(AppState::Game).with_system(spawn_sprites))
            .add_system_set(
                SystemSet::on_update(AppState::Game).with_system(
                    draw_sprites::<Player>
                        .after("draw_background")
                        .label("draw_sprites"),
                ),
            )
            .add_system_set(
                SystemSet::on_update(AppState::Game).with_system(FpsCounterTextBox::update),
            )
            .add_system_set(
                SystemSet::on_update(AppState::Game).with_system(ConnectionStatusTextBox::update),
            )
            .add_system_set(
                SystemSet::on_exit(AppState::Menu).with_system(despawn_screen::<OnGame>),
            );
    }

    fn name(&self) -> &str {
        "main game"
    }
}

#[derive(Component)]
pub struct Player {
    pub handle: usize,
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

#[derive(Component, Clone, Default)]
pub struct Velocity(Vector2D<i32, Pixels>);

impl_reflect_value!(Velocity);

#[derive(Component)]
struct FpsCounterTextBox;

impl FpsCounterTextBox {
    fn update(diagnostics: Res<Diagnostics>, mut query: Query<&mut TextBox, With<Self>>) {
        let mut tb = query.iter_mut().next().unwrap();

        let fps_diagnostic = diagnostics.get(FrameTimeDiagnosticsPlugin::FPS).unwrap();
        if let Some(fps_avg) = fps_diagnostic.average() {
            tb.0 = format!("{} fps", fps_avg as u32);
        }
    }
}

#[derive(Component)]
struct ConnectionStatusTextBox;

impl ConnectionStatusTextBox {
    fn update(status: Res<net::ConnectionStatus>, mut query: Query<&mut TextBox, With<Self>>) {
        let mut tb = query.iter_mut().next().unwrap();
        tb.0 = status.to_string();
    }
}

pub fn spawn_sprites(mut commands: Commands, mut rip: ResMut<RollbackIdProvider>) {
    commands
        .spawn()
        .insert(Player { handle: 0 })
        .insert(Bounds(Rect::new(Point2D::new(10, 10), Size2D::new(10, 10))))
        .insert(Velocity(Vector2D::zero()))
        .insert(Rollback::new(rip.next_id()))
        .insert(OnGame);

    commands
        .spawn()
        .insert(Player { handle: 1 })
        .insert(Bounds(Rect::new(Point2D::new(30, 10), Size2D::new(10, 10))))
        .insert(Velocity(Vector2D::zero()))
        .insert(Rollback::new(rip.next_id()))
        .insert(OnGame);

    commands
        .spawn()
        .insert(TextBox::new("hello world", PALLET[2]))
        .insert(Bounds(Rect::new(
            Point2D::new(10, 40),
            Size2D::new(100, 10),
        )))
        .insert(OnGame);

    commands
        .spawn()
        .insert(TextBox::new("fps", PALLET[2]))
        .insert(FpsCounterTextBox)
        .insert(Bounds(Rect::new(
            Point2D::new(10, 100),
            Size2D::new(100, 10),
        )))
        .insert(OnGame);

    commands
        .spawn()
        .insert(TextBox::new("connecting", PALLET[2]))
        .insert(ConnectionStatusTextBox)
        .insert(Bounds(Rect::new(
            Point2D::new(10, 120),
            Size2D::new(100, 10),
        )))
        .insert(OnGame);
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
