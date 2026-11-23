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

Insert a `CreditsConfig`, add `CreditsPlugin`, then write a `CreditsRequest::Start` to start the roll. Read `CreditsFinished` to learn why it ended. The plugin owns `CreditsState`; the host owns its own game phase and decides what happens after the roll. UI only renders if your app has a camera (e.g. `Camera2d`) — the plugin does not spawn one.

```rust
use bevy::prelude::*;
use bevy_flock_credits_plugin::{CreditsConfig, CreditsFinished, CreditsPlugin, CreditsRequest};

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
        .add_systems(Update, credits_finished)
        .run();
}

fn setup(mut commands: Commands, mut requests: MessageWriter<CreditsRequest>) {
    commands.spawn(Camera2d);
    requests.write(CreditsRequest::Start);
}

fn credits_finished(mut finished: MessageReader<CreditsFinished>) {
    for finished in finished.read() {
        println!("credits ended: {:?}", finished.reason);
    }
}
```

Legacy state-only usage still works: write `NextState<CreditsState> = Active` to start and observe `OnExit(CreditsState::Active)` for completion. Such hosts are treated as natural completions (`CreditsFinishedReason::Completed`).

### Config format

`CreditsConfig` implements `serde::Deserialize`, so it can be loaded from TOML, JSON, RON, etc.

```toml
font_size = 24.0
credits_speed = 60.0
screen_height = 720.0
skip_key = "space"       # optional, defaults to "space"
skip_hold_seconds = 3.0  # optional, defaults to 3.0
background = [0.0, 0.0, 0.0, 1.0]  # optional RGBA, defaults to transparent

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
| `background` | Optional full-screen backdrop as `[r, g, b, a]` in `0..=1`. Omitted keeps the transparent default, so the host scene shows through. Type: `Option<[f32; 4]>`; in-code `CreditsConfig` literals must include the field. |
| `sections` | `{ header, text }` pairs; multiline `text` is supported |

### Hold to skip

While the roll is active, holding `skip_key` for `skip_hold_seconds` ends the credits immediately. A countdown (`3`, `2`, `1`) appears in the bottom-right corner while the key is held and disappears when it is released; releasing early resets the hold. The roll keeps scrolling during the hold, and the hold counter resets every time the credits are (re)started.

The overlay uses the body `font_size` at 1×, white, inset 24px from the corner. Input is optional: if the app has no `ButtonInput<KeyCode>` resource (e.g. a headless test without Bevy's `InputPlugin`), the skip feature is simply inert.

### Lifecycle API

| Situation | Result |
| --- | --- |
| `CreditsRequest::Start` from `Idle` | Roll starts from the top |
| `CreditsRequest::Start` while active | Roll restarts from the top, no finish emitted |
| `CreditsRequest::Stop` while active | UI despawned, one `CreditsFinished { reason: Cancelled }` |
| `CreditsRequest::Stop` from `Idle` | No-op, no message |
| Roll scrolls off screen | `CreditsFinished { reason: Completed }` |
| Skip hold reached | `CreditsFinished { reason: Skipped }` |
| Startup | No message |
| Legacy `NextState` write | Same roll as before; finish reason `Completed` |

If several requests are buffered in one frame, the last one wins. `CreditsFinished` is emitted exactly once per `Active -> Idle` transition, during the state transition; a `MessageReader` in `Update` sees it in the same frame. `CreditsState` stays public for legacy hosts, but new hosts should not need it. The full specification lives in `specs/credits-plugin-lifecycle-api.md`.

### After the credits end

The plugin never exits the app, closes the window, or changes your game phase. It only sets `CreditsState::Idle`, despawns its own UI, and writes `CreditsFinished`. Your host decides what happens next:

```rust
fn credits_finished(
    mut finished: MessageReader<CreditsFinished>,
    mut next: ResMut<NextState<GameState>>,
) {
    for finished in finished.read() {
        // e.g. Skipped -> skip an unlock, Completed -> mark as seen
        next.set(GameState::Menu);
    }
}
```

For external cancellation (a "back" key, scene change, or leaving your own credits phase), write `CreditsRequest::Stop` instead of touching plugin state. The `showcase` example writes `AppExit::Success` when it receives `CreditsFinished`, so `cargo run` returns to the shell when the roll ends.

## Notes

- Your app needs at least one camera (`Camera2d`) for the credits UI to be rendered.
- The plugin does not own a camera/window or clear the screen for you. Set `background` for a self-contained full-screen takeover; omitted means transparent and the host scene shows through.
- Set `screen_height` to match your window. Bevy's default window is 1280×720, so `720.0` matches out of the box. The repo's `credits_config.toml` sets `600.0`, so with the default window the roll starts 120px higher than in production — adjust it to match your window.
- Text is hardcoded white with no line wrapping — long lines will extend beyond the screen width.
- Empty `sections` renders nothing. There is no runtime validation.
- To trigger again, write `CreditsRequest::Start` (legacy: set `CreditsState::Active` again). Both restart from the top and reset the hold counter.
- Adding `background` is source-breaking for exhaustive `CreditsConfig` struct literals: add `background: None` (or your colour) to in-code constructors. TOML configs need no change.

## Running the examples

The repo ships a `credits_config.toml` at the root plus two examples under `examples/`. The `showcase` reads `credits_config.toml` from the **current working directory** (not the crate), so run it from the repo root — or copy your own config into whatever directory you run from. A missing or malformed file panics at startup.

```sh
# Windowed demo: reads ./credits_config.toml from the CWD, scrolls a 1280x720 window,
# and exits when the roll ends (naturally or via hold-to-skip)
cargo run -p bevy-flock-credits-examples --bin showcase

# Headless pipeline check: verifies natural completion, hold-to-skip, release-cancels-hold,
# request-based stop/restart, and finish reasons. Exits non-zero on failure
cargo run -p bevy-flock-credits-examples --bin headless
```