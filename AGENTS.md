# AGENTS.md

Bevy 0.20 plugin crate that renders a scrolling end-credits roll. Library only — the package itself has no binary. Present as a virtual workspace. Consumer setup, config format, and example usage live in the root `README.md`; keep it in sync when the public API or example commands change.

## Workspace layout
- Root `Cargo.toml` is a virtual workspace: members are `bevy-flock-credits-plugin` and `examples`.
- `bevy-flock-credits-plugin/` — the library consumers depend on.
- `examples/` — separate crate (`bevy-flock-credits-examples`, `publish = false`) with a windowed showcase (`main.rs`) and a headless logic check (`bin/headless.rs`).

## Commands
Run from repo root (workspace-wide). The package alone is not runnable.
- `cargo check` — primary verification
- `cargo clippy -- -D warnings`
- `cargo fmt --check` / `cargo fmt`
- `cargo test` — unit tests: `skip_key` parsing in `config.rs`, countdown/visibility and zero-hold skip in `lib.rs`
- `cargo run -p bevy-flock-credits-examples --bin showcase` — windowed demo. Reads `credits_config.toml` from the current directory (`./credits_config.toml`) and spawns its own `Camera2d`.
- `cargo run -p bevy-flock-credits-examples --bin headless` — headless checks: natural completion, hold-to-skip, release-cancels-hold. Exits non-zero on failure.

## Toolchain & deps
- Rust edition 2024 → requires Rust 1.85+. Bevy 0.20 itself requires Rust 1.97.1+, so that is the effective minimum toolchain.
- Package `bevy` dep is `default-features = false, features = ["ui", "default_font", "keyboard"]` — deliberately NOT the full default set (no 2d/3d/audio/gltf). `keyboard` is required for `KeyCode`/`ButtonInput<KeyCode>` (Bevy 0.20 does not enable it by default) and is enabled for consumers through feature unification. Only UI/ecs/time/input APIs should be referenced from `lib.rs`; reverify with `cargo check` if the feature set changes.
- `examples` enables bevy's full default features so `DefaultPlugins`/windowing work without feature-gate guesswork. The windowed example needs a real display and reads the repo-root `credits_config.toml` (`screen_height` 600.0 there, vs the 1280x720 default window). The showcase example must spawn a `Camera2d` itself — Bevy renders UI only when a camera exists, and the plugin intentionally does not own a camera. It also adds `OnExit(CreditsState::Active)` → `AppExit::Success`, so the demo returns to the shell when the roll ends (natural or skipped). That exit handler is demo-only, not part of the plugin.
- Consumers add the dependency via the **repo root** git URL (`bevy-flock-credits-plugin = { git = "https://github.com/<user>/bevy-flock-credits-plugin.git" }`). Cargo traverses the repo tree to find the crate in its subfolder — no `package`/`path` keys needed, and the examples crate is never pulled into consumer builds.

## Architecture
Two source files:

- `bevy-flock-credits-plugin/src/config.rs` — `CreditsConfig` (serde `Deserialize` + Bevy `Resource`) and `CreditsSection`. The consuming game deserializes a config and inserts it as a resource. `skip_key` (default `"space"`) and `skip_hold_seconds` (default `3.0`) are `serde(default)` fields; `parse_skip_key` maps human-readable names to `KeyCode` (private, unit-tested).
- `bevy-flock-credits-plugin/src/lib.rs` — `CreditsPlugin` plus `CreditsState` (`Idle` default, `Active`).
  - `OnEnter(Active)` → `show_credits`: computes total height, spawns a clipping root `Node` tagged `CreditsText` with a child `ScrollTarget { total_height }` that starts just below `screen_height`, plus a hidden `SkipCountdown` text node pinned bottom-right.
  - `OnEnter(Active)` → `reset_skip_hold`: zeroes `SkipHold::held_secs`.
  - `Update` (`.run_if(in_state(Active))`) → `scroll_credits`: moves `node.top` up by `credits_speed * time.delta_secs()`, sets state back to `Idle` when `top < -total_height`.
  - `Update` (same run condition) → `hold_to_skip`: accumulates held time for `skip_key`, resets on release, sets `Idle` once `skip_hold_seconds` is reached, and shows `ceil(remaining)` (3/2/1) in the countdown while held. Parsed key is cached in a `Local`.
  - `OnExit(Active)` → `hide_credits`: despawns all `CreditsText` entities (recursively, so the countdown goes with them).

`CreditsText`, `ScrollTarget`, `SkipCountdown`, and `SkipHold` are private. Examples observe behavior via public `CreditsState` transitions, not those components.

## Gotchas
- Usage contract: consumer must insert `CreditsConfig`, add `CreditsPlugin`, and have a camera (e.g. `Camera2d`) in the app — UI renders nothing otherwise. Then set `CreditsState::Active` via `NextState` to trigger the roll.
- The plugin never exits the app, closes the window, or changes a game phase; it only sets `CreditsState::Idle` and despawns its own entities. Consumers hook `OnExit(CreditsState::Active)` to advance their own state. An empty window after the roll is the host app running with the plugin's UI gone, not a hang.
- Skip input is optional: `hold_to_skip` takes `Option<Res<ButtonInput<KeyCode>>>`, so apps without Bevy's `InputPlugin` (e.g. `MinimalPlugins`) run the roll without skipping instead of panicking.
- Text is hardcoded white with `LineBreak::NoWrap` — long lines will not wrap.
- Height math lives in `show_credits` (constants `HEADER_FONT_SCALE` 1.5, `LINE_HEIGHT_FACTOR` 1.3, `SECTION_SPACING` 40.0, `START_OFFSET` 50.0). Countdown constants are `SKIP_COUNTDOWN_MARGIN` 24.0 and `SKIP_COUNTDOWN_FONT_SCALE` 1.0. Keep in sync if layout or spacing changes.
- Only one `ScrollTarget` entity is expected; `scroll_credits` iterates but will normally find one.
- `scroll_credits` and `hold_to_skip` both write `NextState<CreditsState>`, but only ever to `Idle`, so no ordering constraint.
- No validation: empty `sections` renders nothing; an inaccurate `screen_height` mispositions the starting scroll; an unknown `skip_key` warns and falls back to `space`; `skip_hold_seconds <= 0` (or `NaN`) skips on the first press.