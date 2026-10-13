# OTV2-20261013-step-pace-wire-1

```yaml
task_id: STEP-PACE-WIRE-1
title: "STEP-PACE-WIRE-1: server emits step_wait_ms (field 2) under capability 13"
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/step-pace-wire-1-20261013
pr: null
base_sha: 59f9fb121562174b71dcd666ae760182c38e63be
head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
final_head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
owner: claude-code-session-01XYC239givrMonrYoBwwECV
control_plane: "#1622 (CP session_01XYC239givrMonrYoBwwECV)"
governing_issue: 1622
issue: 1622
created_at: 2026-10-13
updated_at: 2026-10-13
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/gameplay_transport/connection.rs
  - apps/game-server/src/gameplay_transport/world_spatial.rs
  - deploy/synology-game/README.md
  - docs/agents/tasks/active/OTV2-20261013-step-pace-wire-1.md
  - docs/agents/tasks/archive/OTV2-20261013-step-pace-wire-1.md
public_contracts:
  - docs/contracts/protocol-oteryn/v1/world_spatial_v1.proto (WorldActorStepResultV1 field 2, unchanged)
  - docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json (capability 13, command type 1 bound, unchanged)
depends_on:
  - "#2034 STEP-PACE-WIRE-0 (merged 59f9fb12)"
blocks:
  - STEP-PACE-CLIENT-1
external_repositories: []
```

## Outcome

The server writes the `step_wait_ms` pacing receipt (field 2 of `WorldActorStepResultV1`) that
STEP-PACE-WIRE-0 (#2034) specified, and only to a session that selected capability 13
`PACED_MOVEMENT_V1` (owner OQ-1, R1, #1622):

- `MOVED`: the exact step duration the connection's pacing clock just recorded.
- `TOO_EARLY`: the time left until the pacing clock is due, after the buffered step ran, rounded
  up to whole milliseconds; absent when that step did not move (no deadline runs).
- `BLOCKED`, `REJECTED`: never.

A session without capability 13 gets byte-identical results to before. A receipt outside the
STEP-PACE-WIRE-0 contract is dropped and the result goes out without field 2.

## Rollout switch (CP constraint, #1622 comment 6145031564)

Emission is behind the server-side rollout switch `OTERYN_STEP_WAIT_RECEIPTS`. It defaults OFF.
The `serve` process reads it from its environment once, and only exactly `1` turns it on. The
trait method `FreshAdmissionAuthority::step_wait_receipts` reads it once per connection, and
test fixtures override that method. With the switch off, every step result keeps today's bytes,
including results to capability 13 sessions. Capability 13 client builds from before #2034 (9c,
Windows 6c3) advertise capability 13 but use the strict pre-receipt decoder and fail closed on
field 2. The operator therefore turns the switch on only after the client with the updated
capability 13 decoder is released and the older builds are retired. Ops documentation is in
`deploy/synology-game/README.md`, section *Step pacing receipts*.

## Architecture and source of truth

FIRST-CONTROL-WIRE-V1 command type 1, SPEED-1 / CONDITIONS-0 §4.3 (the per-connection pacer
stays the only pacer), STEP-PACE-WIRE-0 proto field 2 rules and the registry bound (8 bytes under
capability 13). No protocol, registry, ID, capability, persistence or Movement change.

## Acceptance criteria

- [x] Switch off (the default): bytes identical to today, with and without capability 13.
- [x] Switch on and capability 13: field 2 present. Switch on without capability 13: no field 2.
- [x] The switch is on only for exactly `1`; unset and every other value are off.
- [ ] Rollout gate (operator, not this PR): enable the switch only after the updated capability 13
  decoder client is released.
- [x] Field 2 only when capability 13 is selected; without it the exact receipt-less bytes.
- [x] Golden bytes with and without field 2; round trip through `decode_step_result_receipt`.
- [x] A proto3 reader that ignores field 2 (old-client semantics) still decodes every result.
- [x] Out-of-contract receipts (off the beat, above 111,150, on BLOCKED/REJECTED) are dropped.
- [x] Connection: a capability 13 MOVED carries the paced duration and the TOO_EARLY the time
  left; without capability 13 the frames are unchanged.

## Excluded scope

Client and Session consumers (STEP-PACE-CLIENT-1); protocol and registry (STEP-PACE-WIRE-0);
Movement, `mod.rs`, `actor_movement.rs`; rollout or deployment of the emission.

## Validation

Recorded in the archived record.
