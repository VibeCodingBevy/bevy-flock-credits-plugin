# bevy-flock-credits-plugin

A [Bevy](https://bevyengine.org) 0.18 plugin that renders a scrolling end-credits roll for your game.

## Requirements

- Rust 1.85+ (crate uses edition 2024)
- Bevy 0.18

## Add to your project

```toml
[dependencies]
bevy = { version = "0.18", default-features = false, features = ["ui", "default_font"] }
bevy-flock-credits-plugin = { git = "https://github.com/<your-user>/bevy-flock-credits-plugin.git" }
```

Cargo finds the crate in the repo's `bevy-flock-credits-plugin/` subfolder automatically — no extra keys needed. The plugin only requires Bevy's `ui` + `default_font` features; your own game can enable whatever additional features it needs.

## Usage

The plugin is a small state machine. Insert a `CreditsConfig`, add `CreditsPlugin`, then flip `CreditsState` to `Active` to start the roll. It returns to `Idle` when the credits have scrolled off screen. UI only renders if your app has a camera (e.g. `Camera2d`) — the plugin does not spawn one.

```rust
use bevy::prelude::*;
use bevy_flock_credits_plugin::{CreditsConfig, CreditsPlugin, CreditsState};

fn main() {
    let credits_config =
        std::fs::read_to_string("credits_config.toml").expect("credits_config.toml to exist");
    let credits_config: CreditsConfig =
        toml::from_str(&credits_config).expect("credits_config.toml to parse");

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
```

### Config format

`CreditsConfig` implements `serde::Deserialize`, so it can be loaded from TOML, JSON, RON, etc.

```toml
font_size = 24.0
credits_speed = 60.0
screen_height = 720.0

[[sections]]
header = "FLOCK"
text = """\
a game about a flock
of sheep
"""

[[sections]]
header = "Thanks for playing!"
text = "made with Bevy"
```

| Field | Meaning |
| --- | --- |
| `font_size` | Body text size in pixels (headers render at 1.5×) |
| `credits_speed` | Scroll speed in pixels/second |
| `screen_height` | Viewport height — determines where the roll starts being visible |
| `sections` | `{ header, text }` pairs; multiline `text` is supported |

## Notes

- Your app needs at least one camera (`Camera2d`) for the credits UI to be rendered.
- Set `screen_height` to match your window. Bevy's default window is 1280×720, so `720.0` matches out of the box.
- Text is hardcoded white with no line wrapping — long lines will extend beyond the screen width.
- Empty `sections` renders nothing. There is no runtime validation.
- To trigger again, just set `CreditsState::Active` again (e.g. after a game over or menu transition).

## Running the examples

The repo ships a `credits_config.toml` at the root plus two examples under `examples/`. Run them from the repo root:

```sh
# Windowed demo: opens a 1280x720 window and scrolls the credits from credits_config.toml
cargo run -p bevy-flock-credits-examples --bin showcase

# Headless pipeline check: verifies Idle -> Active -> Idle without a window
# Exits non-zero on failure
cargo run -p bevy-flock-credits-examples --bin headless
```