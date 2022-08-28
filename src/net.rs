// copyright 2022 Remi Bernotavicius

use super::{game, graphics, input, AppState};
use bevy::prelude::*;
use bevy::tasks::IoTaskPool;
use bevy_ggrs::*;
use enumset::EnumSet;
use ggrs::PlayerType;
use input::InputStream;
use matchbox_socket::WebRtcNonBlockingSocket;
use std::{fmt, mem};

const NUM_PLAYERS: u32 = 2;

#[derive(Clone, Copy, Default)]
pub enum ConnectionStatus {
    #[default]
    WaitingForPlayers,
    Connected,
}

impl fmt::Display for ConnectionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WaitingForPlayers => write!(f, "waiting for players"),
            Self::Connected => write!(f, "connected"),
        }
    }
}

fn input(_: In<ggrs::PlayerHandle>, mut input_stream: NonSendMut<InputStream>) -> Vec<u8> {
    let mut set = EnumSet::new();

    while let Some(i) = input_stream.get() {
        set.insert(i);
    }

    vec![set.as_u8()]
}

fn move_sprites(
    inputs: Res<Vec<ggrs::GameInput>>,
    mut object_query: Query<(&mut graphics::Bounds, &mut game::Velocity, &game::Player)>,
) {
    for (_, mut velocity, player) in object_query.iter_mut() {
        let input = EnumSet::from_u8(inputs[player.handle as usize].buffer[0]);
        game::move_player(input, player, &mut velocity);
    }

    game::physics(object_query);
}

fn start_matchbox_socket(mut commands: Commands, task_pool: Res<IoTaskPool>) {
    let room_url = "ws://remi.party:3536/next_2";
    log::info!("connecting to matchbox server: {:?}", room_url);
    let (socket, message_loop) = WebRtcNonBlockingSocket::new(room_url);

    // The message loop needs to be awaited, or nothing will happen.
    // We do this here using bevy's task system.
    task_pool.spawn(message_loop).detach();

    commands.insert_resource(Some(socket));
}

fn wait_for_players(
    mut commands: Commands,
    mut connection_status: ResMut<ConnectionStatus>,
    mut socket: ResMut<Option<WebRtcNonBlockingSocket>>,
) {
    let socket = socket.as_mut();

    // If there is no socket we've already started the game
    if socket.is_none() {
        return;
    }

    // Check for new connections
    socket.as_mut().unwrap().accept_new_connections();
    let players = socket.as_ref().unwrap().players();

    if players.len() < NUM_PLAYERS as usize {
        return; // wait for more players
    }

    *connection_status = ConnectionStatus::Connected;

    log::info!("All peers have joined, going in-game");

    // consume the socket (currently required because GGRS takes ownership of its socket)
    let socket = socket.take().unwrap();

    let max_prediction = 12;

    // create a GGRS P2P session
    let mut p2p_session = ggrs::P2PSession::new_with_socket(
        NUM_PLAYERS,
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

fn spawn_players(mut commands: Commands, mut rip: ResMut<RollbackIdProvider>) {
    for handle in 0..2 {
        game::Player::spawn(&mut commands, handle).insert(Rollback::new(rip.next_id()));
    }
}

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ConnectionStatus>()
            .add_plugin(GGRSPlugin)
            .with_rollback_schedule(Schedule::default().with_stage(
                "ROLLBACK_STAGE",
                SystemStage::single_threaded().with_system(move_sprites),
            ))
            .with_input_system(input)
            .add_system_set(
                SystemSet::on_enter(AppState::MultiplayerGame).with_system(spawn_players),
            )
            .add_system_set(
                SystemSet::on_enter(AppState::MultiplayerGame).with_system(start_matchbox_socket),
            )
            .add_system_set(
                SystemSet::on_update(AppState::MultiplayerGame).with_system(wait_for_players),
            );
    }
}
