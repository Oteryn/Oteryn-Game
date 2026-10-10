# NPC-RT-1

```yaml
task_id: NPC-RT-1
title: "NPC-RT-1 (narrowed, CP D607): runtime NPC actors in the Channel carrier and the Npc wire kind"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/npc-rt-1-20261008
pr: "the ready PR opened from this branch"
base_sha: 340278d8
head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
final_head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
owner: claude-code-session-01EWURhWcGesM2qWSZG6qbrr
control_plane: claude-code-session-01CwP6d84eCPvpgoEuyci8Tx
coordination: "#1622 (plan comment 6056843803, row N2)"
created_at: 2026-10-08
updated_at: 2026-10-08
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/foundation/runtime_actor_carrier.rs
  - apps/game-server/src/foundation/npc_actor.rs
  - apps/game-server/src/world_runtime.rs
  - apps/game-server/src/gameplay_transport/world_spatial.rs
  - docs/agents/tasks/archive/NPC-RT-1.md
public_contracts: []
depends_on: []
blocks:
  - "NPC-RT-1 follow-up: join-snapshot hunk and boot wiring, after NPC-PLACE-1b"
```

## Outcome

A Channel runtime can create one runtime-only NPC actor per placement (NPC-BEHAVIOUR-0 §3.1)
from a typed placement table in the NPC-PLACE-1 decision §3.2 shape, and the session view maps
an NPC to `EntityKind::Npc`.

## Architecture and source of truth

- The carrier stays the one actor resource: a new `Slot::NpcOccupied { generation, key,
  position }` in the fixed-slot carrier, allocated like `admit_inner` (free-list head,
  generation + 1, `occupied` index). Ids follow frame order: key, then native floor, `y`, `x`.
- An NPC blocks movement (`position_occupied_by_other`, `cell_occupied`), is listed in
  `VisibleRuntimeEntities::npcs`, is never a creature (`lookup` gives `NotCreature`), is never
  removable (`PlanConflict`) and is outside the combat/AI census and the position read/commit
  paths. A stale generation fails closed.
- `NpcPlacementTable::validate` applies the §3.2/§3.3 shape rules: key prefix `oteryn:npc.`,
  ASCII 0x21..=0x7E, at most 128 bytes, strictly ascending keys; 1..=16 placements per NPC
  (RL-02), at most 2,048 per bundle (RL-01); placements strictly ascending by (floor, y, x); no
  shared cell; native floor in -15..=0, runtime floor = `-floor`.
- `ChannelRuntimeV1::place_npcs(table, admitted, production)` runs once, all or nothing
  (snapshot and restore of `slots`, `free_head`, `occupied`). §7: an NPC the catalogue predicate
  refuses refuses the table in production and is skipped and returned otherwise.

## High-risk authority / recovery qualification

No protocol, persistence, identity or fencing change. `EntityKind::Npc` already exists in
`protocol-oteryn`; the wire carries health 100 for an NPC as its decoder requires. Nothing is
durable.

## Acceptance criteria

- `cargo fmt --all --check`, `cargo clippy --locked -p oteryn-game-server --all-targets -- -D
  warnings`, `cargo test --locked -p oteryn-game-server` (Postgres targets in CI) and
  `content_world_project_v2_npc_admission` pass.

## Excluded scope (runtime-caller gap)

`place_npcs` and `VisibleKind::Npc` have no runtime caller yet. The catalogue-digest check, the
format v4 reader (NPC-PLACE-1b), boot wiring (`map/boot.rs`, `node/serve.rs`) and the
join-snapshot hunk (`gameplay_transport/mod.rs`) are the follow-up packet after NPC-PLACE-1b,
per CP D607.

## Implementation / findings

`world_runtime.rs` needed no change.
