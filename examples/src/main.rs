use bevy::prelude::*;
use bevy_flock_credits_plugin::{CreditsConfig, CreditsPlugin, CreditsState};

fn main() {
    let credits_config = std::fs::read_to_string("credits_config.toml")
        .expect("credits_config.toml should exist in the current directory");
    let credits_config: CreditsConfig =
        toml::from_str(&credits_config).expect("credits_config.toml should parse");

    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(credits_config)
        .add_plugins(CreditsPlugin)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, mut next_state: ResMut<NextState<CreditsState>>) {
    commands.spawn(Camera2d);
    next_state.set(CreditsState::Active);
}
