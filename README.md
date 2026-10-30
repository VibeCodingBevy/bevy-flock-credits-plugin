# bevy-flock-credits-plugin

A [Bevy](https://bevyengine.org) 0.20 plugin that renders a scrolling end-credits roll for your game.

## Requirements

- Rust 1.97.1+ (Bevy 0.20's MSRV; the crate itself uses edition 2024)
- Bevy 0.20

## Add to your project

```toml
[dependencies]
bevy = { version = "0.20", default-features = false, features = ["ui", "default_font", "keyboard"] }
bevy-flock-credits-plugin = { git = "https://github.com/VibeCodingBevy/bevy-flock-credits-plugin.git" }
```

Cargo finds the crate in the repo's `bevy-flock-credits-plugin/` subfolder automatically — no extra keys needed. The plugin needs Bevy's `ui`, `default_font`, and `keyboard` features (it enables them on its own `bevy` dependency, so your build gets them too); your game can enable whatever additional features it needs.

## Usage

The plugin is a small state machine. Insert a `CreditsConfig`, add `CreditsPlugin`, then flip `CreditsState` to `Active` to start the roll. It returns to `Idle` when the credits have scrolled off screen — or when the viewer holds the skip key long enough. UI only renders if your app has a camera (e.g. `Camera2d`) — the plugin does not spawn one.

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
skip_key = "space"       # optional, defaults to "space"
skip_hold_seconds = 3.0  # optional, defaults to 3.0

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
| `skip_key` | Key held to skip the roll. Case-insensitive name or single character: `space`, `enter`, `escape`, `tab`, `backspace`, `delete`, `insert`, arrows, `home`, `end`, `pageup`, `pagedown`, `shift`, `ctrl`, `alt`, `f1`–`f12`, `a`–`z`, `0`–`9`. Unknown names warn and fall back to `space`. Default `"space"`. |
| `skip_hold_seconds` | How long `skip_key` must be held before the roll ends. `0` or negative skips on the first press. Default `3.0`. |
| `sections` | `{ header, text }` pairs; multiline `text` is supported |

### Hold to skip

While the roll is active, holding `skip_key` for `skip_hold_seconds` ends the credits immediately. A countdown (`3`, `2`, `1`) appears in the bottom-right corner while the key is held and disappears when it is released; releasing early resets the hold. The roll keeps scrolling during the hold, and the hold counter resets every time the credits are (re)started.

The overlay uses the body `font_size` at 1×, white, inset 24px from the corner. Input is optional: if the app has no `ButtonInput<KeyCode>` resource (e.g. a headless test without Bevy's `InputPlugin`), the skip feature is simply inert.

### After the credits end

The plugin never exits the app, closes the window, or changes your game phase. It only sets `CreditsState::Idle` and despawns its own UI, so the host decides what happens next. React to `OnExit(CreditsState::Active)` — it fires for both natural completion and skips:

```rust
.add_systems(OnExit(CreditsState::Active), |mut next: ResMut<NextState<GameState>>| {
    next.set(GameState::PostCredits);
})
```

The `showcase` example does exactly this and writes `AppExit::Success`, so `cargo run` returns to the shell when the roll ends.

## Notes

- Your app needs at least one camera (`Camera2d`) for the credits UI to be rendered.
- The plugin does not clear the screen or own a camera/window. If the credits end and the window looks empty, that is just Bevy rendering the clear color after the plugin's UI was despawned — your game's own rendering is untouched.
- Set `screen_height` to match your window. Bevy's default window is 1280×720, so `720.0` matches out of the box. The repo's `credits_config.toml` sets `600.0`, so with the default window the roll starts 120px higher than in production — adjust it to match your window.
- Text is hardcoded white with no line wrapping — long lines will extend beyond the screen width.
- Empty `sections` renders nothing. There is no runtime validation.
- To trigger again, just set `CreditsState::Active` again (e.g. after a game over or menu transition).

## Running the examples

The repo ships a `credits_config.toml` at the root plus two examples under `examples/`. The `showcase` reads `credits_config.toml` from the **current working directory** (not the crate), so run it from the repo root — or copy your own config into whatever directory you run from. A missing or malformed file panics at startup.

```sh
# Windowed demo: reads ./credits_config.toml from the CWD, scrolls a 1280x720 window,
# and exits when the roll ends (naturally or via hold-to-skip)
cargo run -p bevy-flock-credits-examples --bin showcase

# Headless pipeline check: verifies natural completion, hold-to-skip, and release-cancels-hold
# without a window. Exits non-zero on failure
cargo run -p bevy-flock-credits-examples --bin headless
```