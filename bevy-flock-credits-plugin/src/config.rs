use bevy::prelude::Resource;
use serde::Deserialize;

#[derive(Deserialize, Resource)]
pub struct CreditsConfig {
    pub font_size: f32,
    pub credits_speed: f32,
    pub screen_height: f32,
    pub sections: Vec<CreditsSection>,
}

#[derive(Deserialize)]
pub struct CreditsSection {
    pub header: String,
    pub text: String,
}
