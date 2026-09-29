# OTV2-20260928-harness-live-c3c4

```yaml
task_id: OTV2-20260928-harness-live-c3c4
title: harness live mode - render the join snapshot, step and click-the-door through oteryn-dev-client
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
issue: 162
pr: 1174
allocation_comment: control-plane allocation on #162 (alias "impl interaction"); owner decision D93
base_branch: main
branch: claude/harness-live-c3c4
base_sha: be242b8fbcd2908ec7b2c58408354eddd414bc0f
head_sha: 22b42cfab80aaee7ea3cb2b1429f1e7a3ea9aa77
final_head_sha: 22b42cfab80aaee7ea3cb2b1429f1e7a3ea9aa77
final_head_frozen_at: 2026-09-28
owner: "Oteryn: impl interaction" (Claude Code)
created_at: 2026-09-28T00:00:00Z
updated_at: 2026-09-28T00:00:00Z
execution_policy: continuous_progress
owned_paths:
  - tools/synthetic-client-harness/**
  - Cargo.lock
  - workspace-boundaries.toml  # [edges] row of oteryn-synthetic-client-harness only
  - docs/agents/tasks/active/OTV2-20260928-harness-live-c3c4.md
  - docs/agents/tasks/archive/OTV2-20260928-dev-client-step-use-c2.md  # git mv, completed
public_contracts: []
depends_on:
  - OTV2-20260928-dev-client-step-use-c2
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

`execution_policy: continuous_progress` means productive authorized work has no wall-clock stop window.

## Outcome

Owner decision D93: the graphical dev client lives in the non-production
`tools/synthetic-client-harness`; `apps/client` stays fail-closed (ADR-0011). The harness gains a
**live mode** (`--live`) on top of `oteryn-dev-client` (`connect_session`, `step`, `use_object`,
`service_liveness`), kept thin: pure mapping plus a loop body.

- `live/model.rs` (pure): `RenderModel::from_snapshot(&JoinSnapshot)`, `apply_step(&StepOutcome)`,
  `apply_use(&UseOutcome)`; tile/pixel geometry (`tile_at_pixel`, `tile_centre_pixel`);
  `command_for_click` (click on the door tile -> `UseDoor` under the mirror's overlay revision,
  which starts at the join's `world_object_overlay_revision` and follows each delta's
  `new_revision`); `step_direction_for_action`; `render_text` (`@` actor, `+` closed door,
  `/` open door, north up). The viewport is configured through the harness renderer's
  `SurfaceState` (`Viewport::configure`), 32 px tiles, camera centred on the actor.
- `live/input.rs`: `LiveInput` routes keys through the shared `oteryn-input-actions` router
  (arrow keys and WASD, repeat allowed) and maps a primary-button press at the last pointer
  position through `command_for_click`.
- `live/controller.rs`: `LiveController` = session + model + input; `handle_event`, `dispatch`
  (applies outcome and deltas to the model), `idle` (`service_liveness`).
- `live/cli.rs`: argument/env parsing, `--live` terminal loop (stdin lines `up/down/left/right`,
  `use`, `click PX PY`, `quit`; `service_liveness` in 200 ms slices while no input is pending).
  Address, CA, character id and grant come from flags or `OTERYN_LIVE_*` env; the grant is only
  ever read from a file or the environment, never a flag, and nothing is committed.
- `main.rs`: without `--live` the existing synthetic run is unchanged.

The door's cell is not on the wire (placement key only), so the model fixes it as the entry
room's accepted cell (`accepted::DOOR_CELL`: x 1, y -1, floor 0). The renderer crate exposes
surface state only on Linux (wgpu is Windows-only, no draw primitives), so the frame is text
drawn from the render model; a GPU front end can consume the same `RenderModel`.

Boundary changes: new normal edges `oteryn-synthetic-client-harness -> oteryn-dev-client` and
`-> oteryn-protocol-oteryn` (the harness must name `StepDirection`, `CharacterId` and the overlay
types; `oteryn-dev-client` re-exports none). New external normal deps `rustls`, `tokio` (workspace
pins, already in the lock); dev deps `rcgen`, `tokio-rustls`. `Cargo.lock` changes only the
harness package entry.

## Architecture and source of truth

- `PROVEN`: merged dev client (#1166, `tools/dev-client`), `PROTOCOL_OTERYN_V1` step and USE
  wire types, `accepted::DOOR_CELL` in `apps/game-server/src/content/project/native_entry.rs`.
- `PROVEN`: neither production closure gained anything (see Validation).
- `UNKNOWN` (not exercised): a live run against a real server, and a windowed/GPU front end.

## High-risk authority/recovery qualification

```yaml
applicable: NOT_APPLICABLE
reason: >
  Non-production tool only. No production crate, protocol, session, lease, generation, authority
  or persistence code changes; the harness is a client of the already merged dev client and holds
  no server-side or recovery semantics. `apps/client` is untouched.
```

## Validation

### Focused

- run: `cargo +1.94.0 fmt --all --check`; `cargo clippy --offline --workspace --all-targets -- -D warnings`;
  `cargo test -p oteryn-synthetic-client-harness`; `cargo test -p oteryn-dev-client` (unchanged);
  `cargo run --locked -p oteryn-architecture-check -- workspace .`;
  `cargo tree --locked -p oteryn-client --edges normal,build` and the same for
  `oteryn-game-server` (no harness, dev-client or new package appears);
  `python3 tools/agents/validate_governance.py`;
  `python3 tools/repository/validate_repository_policy.py`; `git diff --check`
- result: all PASS locally. Harness 15/15 (14 pure mapping/parsing tests, 1 headless controller
  test), dev-client 32/32 unchanged.

### Component/integration

- run: `cargo test -p oteryn-synthetic-client-harness live_controller`
- result: PASS. The controller drives a scripted fake TLS 1.3 / ALPN server (same pattern as
  `tools/dev-client` tests, the server decodes each command with the protocol crate's own
  ingress): join (actor (0,0), door closed, overlay revision 2), arrow-right -> `step` East ->
  Moved to (1,0), idle `service_liveness` acks a probe, click on the door tile -> `use_object`
  naming revision 2 -> Committed, and the render model shows the door open at revision 3. No GPU
  or window.

### E2E

- scenario: live mode against a running game server with a real grant.
- result: NOT RUN - needs a server and an admission grant; out of scope for this packet.

### Exact-head CI

- candidate: the frozen final head of the PR; its live checks govern.
- result: pending

## Self-review

- exact head: the frozen final head of the PR
- method/reviewer: implementing agent (this session)
- material findings: none open.
- verdict: ready for independent review

## Independent review

- required: YES - adds an internal boundary edge to `oteryn-protocol-oteryn` from a synthetic
  tool and a shared `workspace-boundaries.toml` row.
- verdict: pending (requested through the control plane, no `@codex` from this worker)

## Excluded scope

`apps/client/**`, `.github/**`, `tools/repository/**`, `tools/dev-client/**`, every production
crate: unchanged. No `@codex`, no auto-merge, no review-thread resolution by this agent.

## Context checkpoint

```yaml
last_progress: PR #1174 merged via Merge Queue as afa402c; protected-main readback matched 22b42cf; record archived in the 2026-09-29 batch
status: completed
branch: claude/harness-live-c3c4
pr: 1174
final_head_sha: 22b42cfab80aaee7ea3cb2b1429f1e7a3ea9aa77
owner_action_required: null
blocker: null
next_action: none for this task
```

## Closeout

- merge commit/result: `afa402c` on protected `main` (#1174); the changed files are byte-identical to `22b42cf`
- ownership release: all leases released at merge
- archived in the batch archive of 2026-09-29
