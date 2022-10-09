// copyright 2022 Remi Bernotavicius

use super::{despawn_screen, graphics, input, renderer, AppState};
use bevy::prelude::*;
use euclid::{Point2D, Rect, Size2D};
use graphics::{Bounds, SimpleSprite, TextBox, PALLET};
use input::{Input, InputStream};
use renderer::Pixels;
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
    entries: Vec<(Entity, AppState)>,
    marker: Entity,
}

impl Menu {
    fn new(entries: Vec<(Entity, AppState)>, marker: Entity) -> Self {
        assert!(entries.len() > 0);
        Self {
            pos: 0,
            entries,
            marker,
        }
    }

    fn current_text<'a>(&self, textboxes: &'a mut Query<&mut TextBox>) -> Mut<'a, TextBox> {
        textboxes.get_mut(self.entries[self.pos].0).unwrap()
    }

    fn current_app_state<'a>(&self) -> AppState {
        self.entries[self.pos].1
    }

    fn up(&mut self, marker_bounds: &mut Bounds, textboxes: &mut Query<&mut TextBox>) {
        if self.pos > 0 {
            self.current_text(textboxes).color = PALLET[1];
            self.pos -= 1;
            self.current_text(textboxes).color = PALLET[3];
            marker_bounds.0.origin.y -= 10;
        }
    }

    fn down(&mut self, marker_bounds: &mut Bounds, textboxes: &mut Query<&mut TextBox>) {
        if self.pos < self.entries.len() - 1 {
            self.current_text(textboxes).color = PALLET[1];
            self.pos += 1;
            self.current_text(textboxes).color = PALLET[3];
            marker_bounds.0.origin.y += 10;
        }
    }

    fn spawn(
        pos: impl Into<Point2D<i32, Pixels>>,
        items: &[(&str, AppState)],
        mut commands: Commands,
    ) {
        let menu_pos = pos.into();
        let mut text_pos = menu_pos.clone();
        text_pos.x += 5;

        let mut entries = vec![];
        let colors = iter::once(PALLET[3]).chain(iter::repeat(PALLET[1]));
        for (&(text, state), color) in items.into_iter().zip(colors) {
            entries.push((
                TextBox::spawn(&mut commands, text, text_pos, color)
                    .insert(OnMenu)
                    .insert(MenuText)
                    .id(),
                state,
            ));
            text_pos.y += 10;
        }

        let marker = commands
            .spawn()
            .insert(SimpleSprite {
                tile: '>'.into(),
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
                    app_state.set(self_.current_app_state()).unwrap();
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

fn spawn_sprites(mut commands: Commands) {
    TextBox::spawn(&mut commands, "frog quest battle", (10, 40), PALLET[2]).insert(OnMenu);
    Menu::spawn(
        (10, 60),
        &[
            ("single player", AppState::SinglePlayerGame),
            ("multiplayer", AppState::MultiplayerGame),
        ],
        commands,
    );
}
