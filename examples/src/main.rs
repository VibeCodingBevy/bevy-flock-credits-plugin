use bevy::prelude::*;
use bevy_flock_credits_plugin::{CreditsConfig, CreditsFinished, CreditsPlugin, CreditsRequest};

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
        .add_systems(Update, exit_after_credits)
        .run();
}

fn setup(mut commands: Commands, mut requests: MessageWriter<CreditsRequest>) {
    commands.spawn(Camera2d);
    requests.write(CreditsRequest::Start);
}

fn exit_after_credits(
    mut finished: MessageReader<CreditsFinished>,
    mut exit: MessageWriter<AppExit>,
) {
    for finished in finished.read() {
        println!(
            "credits finished ({:?}) - exiting showcase",
            finished.reason
        );
        exit.write(AppExit::Success);
    }
}
