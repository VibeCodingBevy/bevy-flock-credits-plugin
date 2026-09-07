use bevy::prelude::*;
use bevy_flock_credits_plugin::{CreditsConfig, CreditsPlugin, CreditsState};

const CONFIG_JSON: &str = r#"{
    "font_size": 24.0,
    "credits_speed": 60.0,
    "screen_height": 720.0,
    "sections": [
        { "header": "FLOCK", "text": "a game about a flock\nof sheep" },
        { "header": "Programming", "text": "Mariusz" },
        { "header": "Art", "text": "Mariusz\nwith the sheep" },
        { "header": "Music", "text": "the pasture wind" },
        { "header": "Thanks for playing!", "text": "made with Bevy" }
    ]
}"#;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(parse_config(CONFIG_JSON))
        .add_plugins(CreditsPlugin)
        .add_systems(Startup, start_credits)
        .run();
}

fn parse_config(json: &str) -> CreditsConfig {
    serde_json::from_str(json).expect("credits config JSON should parse")
}

fn start_credits(mut next_state: ResMut<NextState<CreditsState>>) {
    next_state.set(CreditsState::Active);
}
