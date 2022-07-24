// copyright 2022 Remi Bernotavicius

use super::{despawn_screen, graphics, input, AppState};
use bevy::prelude::*;
use euclid::{Point2D, Rect, Size2D};
use graphics::{Bounds, TextBox, PALLET};
use input::{InputStream, KeyboardEvent};

#[derive(Component)]
struct OnMenu;

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_system_set(SystemSet::on_enter(AppState::Menu).with_system(spawn_sprites))
            .add_system_set(SystemSet::on_update(AppState::Menu).with_system(drive_menu))
            .add_system_set(
                SystemSet::on_exit(AppState::Menu).with_system(despawn_screen::<OnMenu>),
            );
    }

    fn name(&self) -> &str {
        "main menu"
    }
}

fn spawn_sprites(mut commands: Commands) {
    commands
        .spawn()
        .insert(TextBox::new("frog quest battle", PALLET[2]))
        .insert(Bounds(Rect::new(
            Point2D::new(10, 40),
            Size2D::new(100, 10),
        )))
        .insert(OnMenu);

    commands
        .spawn()
        .insert(TextBox::new("press enter", PALLET[1]))
        .insert(Bounds(Rect::new(
            Point2D::new(15, 60),
            Size2D::new(100, 10),
        )))
        .insert(OnMenu);
}

fn drive_menu(mut input_stream: NonSendMut<InputStream>, mut app_state: ResMut<State<AppState>>) {
    while let Some(i) = input_stream.get() {
        match i {
            KeyboardEvent::Down(e) if e.code() == "Enter" => {
                app_state.set(AppState::Game).unwrap();
            }
            _ => {}
        }
    }
}
