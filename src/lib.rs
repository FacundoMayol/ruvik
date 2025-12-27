mod game;
mod main_menu;

use bevy::prelude::*;

#[derive(Clone, Copy, Default, Eq, PartialEq, Debug, Hash, States)]
enum GameState {
    #[default]
    Menu,
    Game,
}

#[derive(Resource)]
pub struct MainFont(Handle<Font>);

pub struct GameAppPlugin;

impl Plugin for GameAppPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(DefaultPlugins)
            .init_state::<GameState>()
            .add_systems(OnEnter(GameState::Menu), setup)
            .add_plugins((main_menu::plugin, game::plugin));
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/Montserrat-Thin.ttf");

    commands.insert_resource(MainFont(font));
}
