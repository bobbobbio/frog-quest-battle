// copyright 2022 Remi Bernotavicius

use super::renderer::{CanvasRenderer, Color, Pixels, RENDER_RECT};
use super::{Input, RenderFlag};
use bevy::prelude::*;
use bevy::reflect::impl_reflect_value;
use bevy_ggrs::*;
use enumset::EnumSet;
use euclid::{Point2D, Rect, Size2D, Vector2D};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher as _};

#[derive(Component)]
pub struct Player {
    pub handle: usize,
}

#[derive(Component, Clone, Default)]
pub struct Bounds(Rect<i32, Pixels>);

impl_reflect_value!(Bounds);

#[derive(Component, Clone, Default)]
pub struct Velocity(Vector2D<i32, Pixels>);

impl_reflect_value!(Velocity);

#[derive(Component)]
pub struct ColorComponent(Color);

impl ColorComponent {
    fn arbitrary(h: &impl Hash) -> Self {
        let mut s = DefaultHasher::new();
        h.hash(&mut s);
        let [r, g, b, ..] = s.finish().to_le_bytes();
        Self(Color { r, g, b })
    }
}

pub(crate) fn spawn_players(mut commands: Commands, mut rip: ResMut<RollbackIdProvider>) {
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

pub(crate) fn draw(
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

pub(crate) fn physics(mut query: Query<(&mut Bounds, &mut Velocity, &Player)>) {
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
