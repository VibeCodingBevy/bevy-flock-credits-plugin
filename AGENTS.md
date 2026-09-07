# AGENTS.md

Bevy 0.18 plugin crate that renders a scrolling end-credits roll. Library only — no binary, examples, README, or tests.

## Commands
- `cargo check` — primary verification (nothing is runnable; there is no `main.rs` or `examples/`)
- `cargo clippy -- -D warnings`
- `cargo fmt --check` / `cargo fmt`
- `cargo test` — currently runs zero tests

## Toolchain
- Rust edition 2024 → requires Rust 1.85+.
- Bevy deps are minimal: features `["ui", "default_font"]` only. Bevy APIs from disabled features (windowing, rendering, asset, input) are unavailable — check before using.

## Architecture
Two source files:

- `src/config.rs` — `CreditsConfig` (serde `Deserialize` + Bevy `Resource`) and `CreditsSection`. The consuming game deserializes a config and inserts it as a resource.
- `src/lib.rs` — `CreditsPlugin` plus `CreditsState` (`Idle` default, `Active`).
  - `OnEnter(Active)` → `show_credits`: computes total height, spawns a clipping root `Node` tagged `CreditsText` with a child `ScrollTarget { total_height }` that starts just below `screen_height`.
  - `Update` (`.run_if(in_state(Active))`) → `scroll_credits`: moves `node.top` up by `credits_speed * time.delta_secs()`, sets state back to `Idle` when `top < -total_height`.
  - `OnExit(Active)` → `hide_credits`: despawns all `CreditsText` entities.

## Gotchas
- Usage contract: consumer must insert `CreditsConfig`, add `CreditsPlugin`, then set `CreditsState::Active` via `NextState` to trigger the roll.
- Text is hardcoded white with `LineBreak::NoWrap` — long lines will not wrap.
- Height math lives in `show_credits` (constants `HEADER_FONT_SCALE` 1.5, `LINE_HEIGHT_FACTOR` 1.3, `SECTION_SPACING` 40.0, `START_OFFSET` 50.0). Keep in sync if layout or spacing changes.
- Only one `ScrollTarget` entity is expected; `scroll_credits` iterates but will normally find one.
- No validation: empty `sections` renders nothing; an inaccurate `screen_height` mispositions the starting scroll.