# Proposal: credits lifecycle API (start / finish / cancel) + backdrop

Status: accepted and implemented in this repository. Original proposal from the `pong` consumer; its migration remains a separate follow-up. See `README.md` for current host usage.

## Motivation

The current contract is state-only: the host inserts `CreditsConfig`, adds `CreditsPlugin`, then writes `NextState<CreditsState> = Active`. The plugin rolls, sets `Idle`, and despawns its UI. The host observes `OnExit(CreditsState::Active)`.

That works for a demo, but every real host ends up rebuilding the same glue, and the glue is where bugs live. Concrete case from the consumer project (a game with its own `GameState`):

- The host's credits action only set `CreditsState::Active` and left `GameState::Menu` alone. The menu stayed spawned underneath the roll. Because the credits root node has no background, the menu showed through and looked "blocked" (input was gated off to prevent clicks on a visible-but-dead menu).
- The game-over path set `GameState::Menu` **and** `CreditsState::Active` in the same frame, so the menu was spawned under the credits explicitly.
- Fixing it required a dedicated host state (`GameState::Credits`), a helper that writes both `NextState`s atomically, a system that infers completion from `OnEnter(CreditsState::Idle)`, and a safety-net system that cancels the roll when the host leaves its own credits phase.

That is ~50 lines of host code whose only job is to keep two state machines in sync. None of it is game-specific.

## Ownership recommendation

Most of this glue belongs in the credits plugin, but not all of it. Ownership should follow responsibility:

| Concern | Owner | Rationale |
|---|---|---|
| Scroll timing, skip input, roll lifetime | Plugin | These are implementation details of the roll. |
| `CreditsState` transitions | Plugin | The plugin must be the only writer of its own lifecycle state. |
| Start/stop request protocol | Plugin | The host should express intent, not manipulate plugin internals. |
| Completion signal and reason | Plugin | Only the plugin knows whether the roll completed, was skipped, or was cancelled. |
| Backdrop | Plugin | A full-screen takeover must be visually self-contained. |
| Host phase state, such as `GameState::Credits` | Host | Only the host knows which gameplay, input, and UI systems must pause. |
| Action mapping and navigation, such as menu selection or game-over escape | Host | These are host flow decisions. |
| Destination after credits finish | Host | Returning to a menu, unlocking content, or loading a scene is host policy. |

The existing consumer-side `CreditsFlowPlugin` exists only to compensate for the missing plugin API. Its `StartCredits` message is a useful decoupling step, but completion inference and cancellation still remain in the consumer. Those responsibilities should move into the plugin. The host should depend on the request and finish protocol, not on `CreditsState`.

## Problems with the current API

1. **Start is not a request, it is a raw state write.** The host must know `CreditsState` and write it. Hosts with their own phase state must write two `NextState`s in the same transition, or risk a frame where one side is out of sync.
2. **Completion must be inferred.** `OnExit(CreditsState::Active)` is the only signal. It does not say *why* the roll ended (scrolled off vs. skipped), and `OnEnter(CreditsState::Idle)` also fires at startup, so hosts need `in_state` guards to avoid reacting to it.
3. **There is no cancel path.** If the host leaves its credits phase while `CreditsState` is still `Active` (e.g. a "back" key, a window event, a scene change), the roll keeps running and the UI stays on screen. The host has to reach into the plugin's state to stop it.
4. **The roll is not visually self-contained.** `show_credits` spawns a full-screen `Node` with no `BackgroundColor`, so whatever the host renders behind the credits shows through. A plugin that takes over the screen should not depend on the host's scene state for legibility.

## Proposed API (additive, non-breaking)

### 1. `CreditsRequest` message

```rust
/// Requests a lifecycle action. Written by the host, consumed by the plugin.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub enum CreditsRequest {
    /// Start the roll. Restarts from the top if already active.
    Start,
    /// Stop the roll and tear down the UI.
    Stop,
}
```

### 2. `CreditsFinished` message

```rust
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct CreditsFinished {
    pub reason: CreditsFinishedReason,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CreditsFinishedReason {
    /// The roll scrolled past the last line.
    Completed,
    /// The viewer held the skip key for `skip_hold_seconds`.
    Skipped,
    /// The host requested `CreditsRequest::Stop`.
    Cancelled,
}
```

### 3. `background` config field

```toml
# Optional. RGBA in 0..=1. Omitted = transparent (current behaviour).
background = [0.0, 0.0, 0.0, 1.0]
```

Applied as `BackgroundColor` on the existing root `Node` in `show_credits`. Using an `[f32; 4]` keeps it `serde`-friendly with no colour-string parser.

### 4. Optional `CreditsStarted` message

```rust
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct CreditsStarted;
```

This is not required for the first version. Consider adding it if hosts want to drive their own phase state from the plugin lifecycle instead of setting that state in the same system that writes `CreditsRequest::Start`. If it is omitted, the host can write its phase state and `Start` in the same system; both state changes are applied at the next state-transition boundary, so there is no intermediate rendered frame. Start should be accepted from every state, including when the roll is already active.

Either way, the host phase state remains host-owned. The plugin should not become generic over that state.

## Semantics

| Situation | Plugin behaviour |
|---|---|
| `Start` while `Idle` | Set `Active`, spawn the roll, reset the skip hold |
| `Start` while `Active` | Despawn and respawn from the top, reset the skip hold (idempotent restart) |
| `Stop` while `Active` | Despawn, set `Idle`, emit `CreditsFinished { Cancelled }` |
| `Stop` while `Idle` | No-op, no message |
| Roll scrolls past the last line | Set `Idle`, emit `CreditsFinished { Completed }` |
| Skip hold reached | Set `Idle`, emit `CreditsFinished { Skipped }` |
| App startup | Nothing emitted (the initial `Idle` is not a finish) |
| Host writes `NextState<CreditsState>` directly (legacy) | Still works; the roll ends as today |

`CreditsFinished` is emitted exactly once per `Active → Idle` transition.

## Host usage after the change

```rust
fn open_credits(
    mut requests: MessageWriter<CreditsRequest>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    requests.write(CreditsRequest::Start);
    next_state.set(GameState::Credits);
}

fn credits_finished(
    mut finished: MessageReader<CreditsFinished>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    for event in finished.read() {
        // e.g. Skipped -> skip an unlock, Completed -> mark as seen
        let _ = event.reason;
        next_state.set(GameState::Menu);
    }
}

fn leave_credits(mut requests: MessageWriter<CreditsRequest>) {
    requests.write(CreditsRequest::Stop);
}
```

The host no longer imports `CreditsState` for start or cancel, and no longer needs `OnEnter(Idle)` guards. Hosts that add `CreditsStarted` can move the `GameState::Credits` write into a reader for that message instead.

### Consumer migration: `pong`

- Keep `GameState::Credits`: it is needed to pause gameplay and suppress menu and game-over input.
- Menu credit selection and game-over escape write `CreditsRequest::Start` and set `GameState::Credits`.
- A host system reads `CreditsFinished` and sets `GameState::Menu`.
- `OnExit(GameState::Credits)` writes `CreditsRequest::Stop` for external cancellation; the plugin owns the resulting teardown.
- Remove the local `StartCredits` message. Remove `CreditsFlowPlugin` entirely, or reduce it to a thin credits-navigation plugin if it still has host-specific responsibilities.
- Keep score reset and navigation policy in the game-over plugin; these are not plugin concerns.
- This specification is the canonical lifecycle documentation in the plugin repository (`specs/credits-plugin-lifecycle-api.md`). Pong-side changes are tracked separately.

## Implementation notes

- Register both messages with `add_message` in `CreditsPlugin::build`.
- Request handling goes in an ungated `Update` system (not `run_if(in_state(Active))`, or `Stop` from `Idle` and restart while active would be missed). It writes `NextState<CreditsState>`; the existing `OnEnter`/`OnExit` schedules keep doing the spawning/despawning. If several requests are buffered in one frame, the last one wins.
- Reasons are carried alongside the hold timer in a small `Resource` (`RollProgress`), set by the request handler and the two existing `Idle` writers with first-writer-wins priority (`Cancelled > Skipped > Completed`). Emission happens in `OnTransition { exited: Active, entered: Idle }`, not `OnExit`: in Bevy 0.20, `NextState::set` forces exit/enter schedules even for identity transitions, so `OnExit` would emit a spurious finish on every restart. Legacy hosts that write the state directly still get `Completed` by default.
- `CreditsState` stays public and unchanged; the message API is sugar on top. No host migration is required for compatibility. Adding `background` to `CreditsConfig` is source-breaking for exhaustive struct literals (TOML is compatible via `#[serde(default)]`); in-code constructors need the new field.
- `background` is opt-in so existing consumers see no visual change. Flip the default to opaque in the next minor release if that is the desired long-term behaviour.
- If the upstream plugin cannot change immediately, keep the consumer-side adapter temporarily. Add local stop and finish messages only as a compatibility layer, then delete them once the plugin exposes its own lifecycle API.

## Tests to add (plugin repo)

- `Start` from `Idle` → state `Active`, root node spawned.
- `Start` while `Active` → exactly one root node, skip hold reset.
- `Stop` while `Active` → state `Idle`, UI despawned, one `CreditsFinished { Cancelled }`.
- `Stop` while `Idle` → no message.
- Natural completion → `CreditsFinished { Completed }`.
- Hold-to-skip → `CreditsFinished { Skipped }`.
- No `CreditsFinished` at startup.
- `background` set → root node has that `BackgroundColor`; omitted → root keeps Bevy's transparent default (`Node` requires the component).
- Extend `examples/bin/headless.rs` to drive the roll through `CreditsRequest` and assert on reasons.

## Alternatives considered

- **Keep the state-only API.** Works, but forces every host to duplicate state and hand-synchronize transitions. This proposal exists because that glue caused a visible bug.
- **Let the host pass its own state type (`CreditsPlugin<HostState>`).** Makes the plugin generic over host flow, adds a type parameter to every use, and still does not solve completion reasons or cancellation.
- **Observers instead of messages.** Triggering an observer from inside a state transition is fine, but messages are the established input idiom in this plugin's host ecosystem and compose better with `MessageReader` in `Update`. Observers can be added later as an alternative emission path if needed.
- **Host-owned backdrop.** Rejected: rendering responsibility belongs to the thing that renders the roll. A transparent full-screen takeover is a plugin bug, not a host configuration problem.

## Effort

`config.rs` (field), `lib.rs` (two messages, request handling, reason plumbing, backdrop), `README.md`, `AGENTS.md`, `examples/bin/headless.rs`, `examples/src/main.rs`. No changes to the existing public items, so this can ship as a `0.1.x` minor.

Consumer migration is separate: replace the local `StartCredits` message with `CreditsRequest`, move completion handling to `CreditsFinished`, remove direct `CreditsState` coupling, and delete the compatibility adapter once downstream hosts have migrated.

## Implementation handoff

This is the handoff artifact for a separate implementation session. The session should be able to implement the change from this file alone, without the surrounding conversation.

### Target and baseline

- Target repository: `bevy-flock-credits-plugin` at `https://github.com/VibeCodingBevy/bevy-flock-credits-plugin.git`.
- Baseline revision: `0b3a0f791ea3bd4041004617cd729d4efb7cc470`, currently pinned by `pong/Cargo.toml`.
- Relevant files to inspect before editing: `bevy-flock-credits-plugin/src/config.rs`, `bevy-flock-credits-plugin/src/lib.rs`, `README.md`, `AGENTS.md`, `examples/src/bin/headless.rs`, `examples/src/main.rs`.
- Scope: plugin repository only. Pong migration is a follow-up described under “Consumer migration: `pong`”.

### Copyable task prompt

```text
Implement the credits lifecycle API described in specs/credits-plugin-lifecycle-api.md
against bevy-flock-credits-plugin at baseline revision
0b3a0f791ea3bd4041004617cd729d4efb7cc470.

Follow the accepted decisions and acceptance criteria in the "Implementation handoff"
section. Do not migrate Pong in this change.
```

### Accepted decisions

- `CreditsRequest::{Start, Stop}` and `CreditsFinished` are in scope.
- `CreditsStarted` is out of scope for the first implementation.
- `background` remains opt-in and transparent by default.
- Legacy direct writes to `NextState<CreditsState>` remain supported.
- Legacy completion emits `CreditsFinishedReason::Completed`.
- `CreditsState` remains public and unchanged.
- The plugin must not become generic over a host state type.
- Pong migration is a separate follow-up.

### Acceptance criteria

- [ ] Public items compile with the documented names and derives.
- [ ] Every row in the “Semantics” table is implemented.
- [ ] `CreditsFinished` is emitted exactly once per `Active → Idle` transition.
- [ ] No `CreditsFinished` is emitted at startup.
- [ ] Restart while active leaves exactly one credits root node and resets the skip hold.
- [ ] `background` is applied only when explicitly configured.
- [ ] Existing state-only consumers continue to work unchanged.
- [ ] All tests listed under “Tests to add (plugin repo)” pass.
- [ ] `cargo fmt`, `cargo check`, `cargo test`, and `cargo build` pass with zero warnings.
- [ ] Web/WASM compatibility is preserved.

### Verification commands

Run from the plugin repository root:

```bash
cargo fmt
cargo check
cargo test
cargo build
```
