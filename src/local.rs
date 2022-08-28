// copyright 2022 Remi Bernotavicius

use super::{game, graphics, input, AppState};
use bevy::prelude::*;
use input::InputStream;
use std::iter;

fn move_sprites(
    mut input_stream: NonSendMut<InputStream>,
    mut object_query: Query<(&mut graphics::Bounds, &mut game::Velocity, &game::Player)>,
) {
    let input = iter::from_fn(|| input_stream.get()).collect();

    for (_, mut velocity, player) in object_query.iter_mut() {
        game::move_player(input, player, &mut velocity);
    }

    game::physics(object_query);
}

fn spawn_player(mut commands: Commands) {
    game::Player::spawn(&mut commands, 0);
}

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_system_set(
            SystemSet::on_update(AppState::SinglePlayerGame).with_system(move_sprites),
        )
        .add_system_set(SystemSet::on_enter(AppState::SinglePlayerGame).with_system(spawn_player));
    }
}
