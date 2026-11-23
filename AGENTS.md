# AGENTS.md

Bevy 0.20 plugin crate that renders a scrolling end-credits roll. Library only — the package itself has no binary. Present as a virtual workspace. Consumer setup, config format, and example usage live in the root `README.md`; keep it in sync when the public API or example commands change. The canonical lifecycle spec is `specs/credits-plugin-lifecycle-api.md`.

## Workspace layout
- Root `Cargo.toml` is a virtual workspace: members are `bevy-flock-credits-plugin` and `examples`.
- `bevy-flock-credits-plugin/` — the library consumers depend on.
- `examples/` — separate crate (`bevy-flock-credits-examples`, `publish = false`) with a windowed showcase (`src/main.rs`) and a headless logic check (`src/bin/headless.rs`).

## Commands
Run from repo root (workspace-wide). The package alone is not runnable.
- `cargo check` — primary verification
- `cargo clippy -- -D warnings`
- `cargo fmt --check` / `cargo fmt`
- `cargo test` — unit tests: `skip_key` parsing, countdown/visibility, zero-hold skip, request lifecycle (start/restart/stop/no-op/last-wins), finish reasons (Completed/Skipped/Cancelled), legacy state writes, background config
- `cargo run -p bevy-flock-credits-examples --bin showcase` — windowed demo. Reads `credits_config.toml` from the current directory (`./credits_config.toml`) and spawns its own `Camera2d`.
- `cargo run -p bevy-flock-credits-examples --bin headless` — headless checks: natural completion, hold-to-skip, release-cancels-hold, request-based stop/restart, and finish reasons. Exits non-zero on failure.
- `cargo check --target wasm32-unknown-unknown -p bevy-flock-credits-plugin` — optional WASM check for the library (target must be installed; the examples use `std::fs` and are native-only).

## Toolchain & deps
- Rust edition 2024 → requires Rust 1.85+. Bevy 0.20 itself requires Rust 1.97.1+, so that is the effective minimum toolchain.
- Package `bevy` dep is `default-features = false, features = ["ui", "default_font", "keyboard"]` — deliberately NOT the full default set (no 2d/3d/audio/gltf). `keyboard` is required for `KeyCode`/`ButtonInput<KeyCode>` (Bevy 0.20 does not enable it by default) and is enabled for consumers through feature unification. Only UI/ecs/time/input/state/message APIs should be referenced from `lib.rs`; reverify with `cargo check` if the feature set changes.
- `examples` enables bevy's full default features so `DefaultPlugins`/windowing work without feature-gate guesswork. The windowed example needs a real display and reads the repo-root `credits_config.toml` (`screen_height` 600.0 there, vs the 1280x720 default window; it also sets an opaque `background`). The showcase example must spawn a `Camera2d` itself — Bevy renders UI only when a camera exists, and the plugin intentionally does not own a camera. It starts via `CreditsRequest::Start` and writes `AppExit::Success` on `CreditsFinished`. That exit handler is demo-only, not part of the plugin.
- Consumers add the dependency via the **repo root** git URL (`bevy-flock-credits-plugin = { git = "https://github.com/<user>/bevy-flock-credits-plugin.git" }`). Cargo traverses the repo tree to find the crate in its subfolder — no `package`/`path` keys needed, and the examples crate is never pulled into consumer builds.

## Architecture
Two source files:

- `bevy-flock-credits-plugin/src/config.rs` — `CreditsConfig` (serde `Deserialize` + Bevy `Resource`) and `CreditsSection`. The consuming game deserializes a config and inserts it as a resource. `skip_key` (default `"space"`), `skip_hold_seconds` (default `3.0`), and `background` (`Option<[f32; 4]>`, default `None`) are `serde(default)` fields; `parse_skip_key` maps human-readable names to `KeyCode` (private, unit-tested).
- `bevy-flock-credits-plugin/src/lib.rs` — `CreditsPlugin`, `CreditsState` (`Idle` default, `Active`), and the message API:
  - `CreditsRequest::{Start, Stop}` (host → plugin) and `CreditsFinished { reason }` / `CreditsFinishedReason::{Completed, Skipped, Cancelled}` (plugin → host). Both messages are registered with `add_message` in `build`.
  - `RollProgress` resource carries the hold timer and the reason for the pending transition. Priority is `Cancelled > Skipped > Completed` via first-writer-wins `set_pending_finish`; `handle_requests` runs before `scroll_credits`/`hold_to_skip` (which are chained for deterministic `Completed`/`Skipped` arbitration).
  - `Update` (ungated) → `handle_requests`: only the last buffered request is applied. `Start` always calls `NextState::set(Active)`; in Bevy 0.20 that forces exit/enter schedules even for an identity transition, so this also restarts an active roll. `Stop` only writes `Idle` when the roll is active; it cancels a pending start without emitting a finish and leaves an already-queued finish reason alone.
  - `OnEnter(Active)` → `show_credits`: computes total height, spawns a clipping root `Node` tagged `CreditsText` with a child `ScrollTarget { total_height }` that starts just below `screen_height`, plus a hidden `SkipCountdown` text node pinned bottom-right. `background`, when set, is applied as `BackgroundColor(Color::srgba(...))` on the root.
  - `OnEnter(Active)` → `reset_skip_hold`: zeroes `RollProgress::held_secs`.
  - `Update` (`.run_if(in_state(Active))`) → `scroll_credits`: moves `node.top` up by `credits_speed * time.delta_secs()`, queues `Completed` and sets state to `Idle` when `top < -total_height`.
  - `Update` (same run condition) → `hold_to_skip`: accumulates held time for `skip_key`, resets on release, queues `Skipped` and sets `Idle` once `skip_hold_seconds` is reached, and shows `ceil(remaining)` (3/2/1) in the countdown while held. Parsed key is cached in a `Local`.
  - `OnExit(Active)` → `hide_credits`: despawns all `CreditsText` entities (in Bevy 0.20 `despawn()` removes descendants, so the countdown goes with them).
  - `OnTransition { exited: Active, entered: Idle }` → `emit_finished`: writes one `CreditsFinished` using `RollProgress::pending_finish`, defaulting to `Completed` for legacy state writes. `OnTransition` is used instead of `OnExit` so identity restarts do not emit a finish.

`CreditsText`, `ScrollTarget`, `SkipCountdown`, and `RollProgress` are private. Examples observe behavior via public `CreditsRequest`/`CreditsFinished` messages; `CreditsState` remains public for legacy hosts.

## Gotchas
- Usage contract: consumer must insert `CreditsConfig`, add `CreditsPlugin`, and have a camera (e.g. `Camera2d`) in the app — UI renders nothing otherwise. Preferred start is `CreditsRequest::Start`; writing `NextState<CreditsState> = Active` remains supported.
- The plugin never exits the app, closes the window, or changes a game phase; it only sets `CreditsState::Idle`, despawns its own entities, and writes `CreditsFinished`. Consumers own their phase and destination. An empty window after the roll is the host app running with the plugin's UI gone, not a hang.
- `CreditsFinished` is emitted during the state transition; a `MessageReader` in `Update` sees it in the same frame. No `OnEnter(Idle)` guards or `in_state` checks are needed. No message is emitted at startup or on a restart.
- Adding `background` to `CreditsConfig` is source-breaking for exhaustive Rust struct literals — add `background: None` to in-code constructors. TOML configs are compatible via `#[serde(default)]`.
- Bevy's `Node` requires a `BackgroundColor` component (transparent by default), so "no background configured" means the transparent default, not absence of the component.
- Skip input is optional: `hold_to_skip` takes `Option<Res<ButtonInput<KeyCode>>>`, so apps without Bevy's `InputPlugin` (e.g. `MinimalPlugins`) run the roll without skipping instead of panicking.
- Text is hardcoded white with `LineBreak::NoWrap` — long lines will not wrap.
- Height math lives in `show_credits` (constants `HEADER_FONT_SCALE` 1.5, `LINE_HEIGHT_FACTOR` 1.3, `SECTION_SPACING` 40.0, `START_OFFSET` 50.0). Countdown constants are `SKIP_COUNTDOWN_MARGIN` 24.0 and `SKIP_COUNTDOWN_FONT_SCALE` 1.0. Keep in sync if layout or spacing changes.
- Only one `ScrollTarget` entity is expected; `scroll_credits` iterates but will normally find one.
- No validation: empty `sections` renders nothing; an inaccurate `screen_height` mispositions the starting scroll; an unknown `skip_key` warns and falls back to `space`; `skip_hold_seconds <= 0` (or `NaN`) skips on the first press.
