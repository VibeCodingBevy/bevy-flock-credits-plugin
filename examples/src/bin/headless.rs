use std::time::Duration;

use bevy::input::ButtonState;
use bevy::input::InputPlugin;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy::time::TimeUpdateStrategy;
use bevy_flock_credits_plugin::{CreditsConfig, CreditsPlugin, CreditsSection, CreditsState};

const FPS: f32 = 60.0;
const FRAME_SECS: f32 = 1.0 / FPS;
const HOLD_SECS: f32 = 3.0;
const HOLD_FRAMES: u32 = (HOLD_SECS * FPS) as u32;
const MAX_FRAMES: u32 = 2000;

fn main() {
    let mut failures = Vec::new();

    if let Err(reason) = natural_completion() {
        failures.push(reason);
    }
    if let Err(reason) = hold_to_skip_ends_early() {
        failures.push(reason);
    }
    if let Err(reason) = release_cancels_hold() {
        failures.push(reason);
    }

    if failures.is_empty() {
        println!("PASS: all credits scenarios behaved as expected.");
    } else {
        for failure in &failures {
            println!("FAIL: {failure}");
        }
        std::process::exit(1);
    }
}

fn build_app(credits_speed: f32) -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, StatesPlugin, InputPlugin))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            FRAME_SECS,
        )))
        .insert_resource(config(credits_speed))
        .add_plugins(CreditsPlugin)
        .add_systems(Startup, start_credits);
    app
}

fn state(app: &App) -> CreditsState {
    *app.world().resource::<State<CreditsState>>().get()
}

fn send_key(app: &mut App, key_state: ButtonState) {
    app.world_mut().write_message(KeyboardInput {
        key_code: KeyCode::Space,
        logical_key: Key::Space,
        state: key_state,
        text: None,
        repeat: false,
        window: Entity::PLACEHOLDER,
    });
}

/// The roll still reaches `Idle` on its own without any input.
fn natural_completion() -> Result<(), String> {
    let mut app = build_app(200.0);
    let mut saw_active = false;

    for frame in 0..MAX_FRAMES {
        app.update();

        let current = state(&app);
        if current == CreditsState::Active {
            saw_active = true;
        }
        if current == CreditsState::Idle && saw_active {
            println!("natural: scrolled to completion after {frame} frames");
            return Ok(());
        }
    }

    Err(format!(
        "natural: expected Idle -> Active -> Idle, saw_active={saw_active}"
    ))
}

/// Holding the skip key for `HOLD_SECS` ends the roll far before it would end naturally.
fn hold_to_skip_ends_early() -> Result<(), String> {
    let mut app = build_app(20.0);
    send_key(&mut app, ButtonState::Pressed);

    for frame in 0..MAX_FRAMES {
        app.update();

        if state(&app) == CreditsState::Idle {
            println!("hold: ended after {frame} frames of holding Space");
            if frame < HOLD_FRAMES - 10 {
                return Err(format!(
                    "hold: ended too early at frame {frame}, expected around {HOLD_FRAMES}"
                ));
            }
            if frame > HOLD_FRAMES + 15 {
                return Err(format!(
                    "hold: ended too late at frame {frame}, expected around {HOLD_FRAMES}"
                ));
            }
            return Ok(());
        }
    }

    Err("hold: credits never ended while holding Space".into())
}

/// Releasing before the threshold cancels the hold; a second full hold still skips.
fn release_cancels_hold() -> Result<(), String> {
    let mut app = build_app(20.0);

    send_key(&mut app, ButtonState::Pressed);
    for _ in 0..(HOLD_SECS * FPS) as u32 - 6 {
        app.update();
        if state(&app) == CreditsState::Idle {
            return Err("release: credits ended before the hold threshold".into());
        }
    }

    send_key(&mut app, ButtonState::Released);
    app.update();
    if state(&app) == CreditsState::Idle {
        return Err("release: credits ended on the release frame".into());
    }

    send_key(&mut app, ButtonState::Pressed);
    for _ in 0..HOLD_FRAMES + 15 {
        app.update();
        if state(&app) == CreditsState::Idle {
            println!("release: re-hold ended credits as expected");
            return Ok(());
        }
    }

    Err("release: credits never ended after a second full hold".into())
}

fn config(credits_speed: f32) -> CreditsConfig {
    CreditsConfig {
        font_size: 24.0,
        credits_speed,
        screen_height: 100.0,
        skip_key: "space".into(),
        skip_hold_seconds: HOLD_SECS,
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
