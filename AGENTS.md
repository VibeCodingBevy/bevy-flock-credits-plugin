# AGENTS.md

Bevy 0.18 plugin crate that renders a scrolling end-credits roll. Library only — the package itself has no binary, README, or tests. Present as a virtual workspace.

## Workspace layout
- Root `Cargo.toml` is a virtual workspace: members are `bevy-flock-credits-plugin` and `examples`.
- `bevy-flock-credits-plugin/` — the library consumers depend on.
- `examples/` — separate crate (`bevy-flock-credits-examples`, `publish = false`) with a windowed showcase (`main.rs`) and a headless logic check (`bin/headless.rs`).

## Commands
Run from repo root (workspace-wide). The package alone is not runnable.
- `cargo check` — primary verification
- `cargo clippy -- -D warnings`
- `cargo fmt --check` / `cargo fmt`
- `cargo test` — currently runs zero tests
- `cargo run -p bevy-flock-credits-examples --bin showcase` — windowed demo (opens a window)
- `cargo run -p bevy-flock-credits-examples --bin headless` — headless Idle→Active→Idle pipeline check, exits non-zero on failure

## Toolchain & deps
- Rust edition 2024 → requires Rust 1.85+.
- Package `bevy` dep is `default-features = false, features = ["ui", "default_font"]` — deliberately NOT the full default set (no 2d/3d/audio/gltf). Only UI/ecs/time APIs should be referenced from `lib.rs`; reverify with `cargo check` if the feature set changes.
- `examples` enables bevy's full default features so `DefaultPlugins`/windowing work without feature-gate guesswork. The windowed example needs a real display; `screen_height` in its inline JSON matches Bevy's default 1280x720 window.
- Consumers add the dependency via the **repo root** git URL (`bevy-flock-credits-plugin = { git = "https://github.com/<user>/bevy-flock-credits-plugin.git" }`). Cargo traverses the repo tree to find the crate in its subfolder — no `package`/`path` keys needed, and the examples crate is never pulled into consumer builds.

## Architecture
Two source files:

- `bevy-flock-credits-plugin/src/config.rs` — `CreditsConfig` (serde `Deserialize` + Bevy `Resource`) and `CreditsSection`. The consuming game deserializes a config and inserts it as a resource.
- `bevy-flock-credits-plugin/src/lib.rs` — `CreditsPlugin` plus `CreditsState` (`Idle` default, `Active`).
  - `OnEnter(Active)` → `show_credits`: computes total height, spawns a clipping root `Node` tagged `CreditsText` with a child `ScrollTarget { total_height }` that starts just below `screen_height`.
  - `Update` (`.run_if(in_state(Active))`) → `scroll_credits`: moves `node.top` up by `credits_speed * time.delta_secs()`, sets state back to `Idle` when `top < -total_height`.
  - `OnExit(Active)` → `hide_credits`: despawns all `CreditsText` entities.

`CreditsText` and `ScrollTarget` are private. Examples observe behavior via public `CreditsState` transitions, not those components.

## Gotchas
- Usage contract: consumer must insert `CreditsConfig`, add `CreditsPlugin`, then set `CreditsState::Active` via `NextState` to trigger the roll.
- Text is hardcoded white with `LineBreak::NoWrap` — long lines will not wrap.
- Height math lives in `show_credits` (constants `HEADER_FONT_SCALE` 1.5, `LINE_HEIGHT_FACTOR` 1.3, `SECTION_SPACING` 40.0, `START_OFFSET` 50.0). Keep in sync if layout or spacing changes.
- Only one `ScrollTarget` entity is expected; `scroll_credits` iterates but will normally find one.
- No validation: empty `sections` renders nothing; an inaccurate `screen_height` mispositions the starting scroll.