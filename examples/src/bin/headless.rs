use std::time::Duration;

use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy::time::TimeUpdateStrategy;
use bevy_flock_credits_plugin::{CreditsConfig, CreditsPlugin, CreditsSection, CreditsState};

const FPS: f32 = 60.0;
const MAX_FRAMES: u32 = 1000;

fn main() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, StatesPlugin))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            1.0 / FPS,
        )))
        .insert_resource(config())
        .add_plugins(CreditsPlugin)
        .add_systems(Startup, start_credits);

    let mut saw_active = false;
    let mut final_state = None;

    for frame in 0..MAX_FRAMES {
        app.update();

        let state = *app.world().resource::<State<CreditsState>>().get();
        println!("frame {frame}: state = {state:?}");

        if state == CreditsState::Active {
            saw_active = true;
        }
        if state == CreditsState::Idle && saw_active {
            final_state = Some(state);
            break;
        }
    }

    match final_state {
        Some(CreditsState::Idle) if saw_active => {
            println!("PASS: credits started (Active) and scrolled to completion (Idle).");
        }
        _ => {
            println!(
                "FAIL: expected Idle -> Active -> Idle, saw_active={saw_active}, final={final_state:?}"
            );
            std::process::exit(1);
        }
    }
}

fn config() -> CreditsConfig {
    CreditsConfig {
        font_size: 24.0,
        credits_speed: 200.0,
        screen_height: 100.0,
        sections: vec![
            CreditsSection {
                header: "Flock".into(),
                text: "sheep\nand sheep".into(),
            },
            CreditsSection {
                header: "Done".into(),
                text: "roll finished".into(),
            },
        ],
    }
}

fn start_credits(mut next_state: ResMut<NextState<CreditsState>>) {
    next_state.set(CreditsState::Active);
}
