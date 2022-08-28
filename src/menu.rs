// copyright 2022 Remi Bernotavicius

use super::{despawn_screen, graphics, input, renderer, AppState};
use bevy::ecs::system::EntityCommands;
use bevy::prelude::*;
use euclid::{Point2D, Rect, Size2D};
use graphics::{Bounds, SimpleSprite, TextBox, TileNumber, PALLET};
use input::{Input, InputStream};
use renderer::{Color, Pixels};
use std::iter;

#[derive(Component)]
struct OnMenu;

#[derive(Component)]
struct MenuMarker;

#[derive(Component)]
struct MenuText;

#[derive(Component)]
struct Menu {
    pos: usize,
    entries: Vec<Entity>,
    marker: Entity,
}

impl Menu {
    fn new(entries: Vec<Entity>, marker: Entity) -> Self {
        assert!(entries.len() > 0);
        Self {
            pos: 0,
            entries,
            marker,
        }
    }

    fn up(&mut self, marker_bounds: &mut Bounds, textboxes: &mut Query<&mut TextBox>) {
        if self.pos > 0 {
            textboxes.get_mut(self.entries[self.pos]).unwrap().1 = PALLET[1];
            self.pos -= 1;
            textboxes.get_mut(self.entries[self.pos]).unwrap().1 = PALLET[3];
            marker_bounds.0.origin.y -= 10;
        }
    }

    fn down(&mut self, marker_bounds: &mut Bounds, textboxes: &mut Query<&mut TextBox>) {
        if self.pos < self.entries.len() - 1 {
            textboxes.get_mut(self.entries[self.pos]).unwrap().1 = PALLET[1];
            self.pos += 1;
            textboxes.get_mut(self.entries[self.pos]).unwrap().1 = PALLET[3];
            marker_bounds.0.origin.y += 10;
        }
    }

    fn spawn(pos: impl Into<Point2D<i32, Pixels>>, items: &[&str], mut commands: Commands) {
        let menu_pos = pos.into();
        let mut text_pos = menu_pos.clone();
        text_pos.x += 5;

        let mut entries = vec![];
        let colors = iter::once(PALLET[3]).chain(iter::repeat(PALLET[1]));
        for (text, color) in items.into_iter().zip(colors) {
            entries.push(
                spawn_text(&mut commands, text, text_pos, color)
                    .insert(MenuText)
                    .id(),
            );
            text_pos.y += 10;
        }

        let marker = commands
            .spawn()
            .insert(SimpleSprite {
                tile: TileNumber::new(97),
                color: PALLET[3],
            })
            .insert(Bounds(Rect::new(menu_pos, Size2D::new(10, 10))))
            .insert(MenuMarker)
            .insert(OnMenu)
            .id();

        commands
            .spawn()
            .insert(Menu::new(entries, marker))
            .insert(OnMenu);
    }

    fn update(
        mut self_query: Query<&mut Self>,
        mut marker_query: Query<&mut Bounds, With<MenuMarker>>,
        mut textboxes: Query<&mut TextBox>,
        mut input_stream: NonSendMut<InputStream>,
        mut app_state: ResMut<State<AppState>>,
    ) {
        let mut self_ = self_query.iter_mut().next().unwrap();
        let mut marker_bounds = marker_query.get_mut(self_.marker).unwrap();

        while let Some(i) = input_stream.get() {
            match i {
                Input::Primary => {
                    app_state.set(AppState::Game).unwrap();
                }
                Input::Up => self_.up(&mut *marker_bounds, &mut textboxes),
                Input::Down => self_.down(&mut *marker_bounds, &mut textboxes),
                _ => {}
            }
        }
    }
}

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_system_set(SystemSet::on_enter(AppState::Menu).with_system(spawn_sprites))
            .add_system_set(SystemSet::on_update(AppState::Menu).with_system(Menu::update))
            .add_system_set(
                SystemSet::on_exit(AppState::Menu).with_system(despawn_screen::<OnMenu>),
            );
    }

    fn name(&self) -> &str {
        "main menu"
    }
}

fn spawn_text<'a, 'w, 's>(
    commands: &'a mut Commands<'w, 's>,
    text: &str,
    pos: impl Into<Point2D<i32, Pixels>>,
    color: Color,
) -> EntityCommands<'w, 's, 'a> {
    let mut entity = commands.spawn();
    entity
        .insert(TextBox::new(text, color))
        .insert(Bounds(Rect::new(pos.into(), Size2D::new(100, 10))))
        .insert(OnMenu);
    entity
}

fn spawn_sprites(mut commands: Commands) {
    spawn_text(&mut commands, "frog quest battle", (10, 40), PALLET[2]);
    Menu::spawn((10, 60), &["single player", "multiplayer"], commands);
}
