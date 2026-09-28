# OTV2-20260928-cw1-duke-teleporter-entry-placement

```yaml
task_id: OTV2-20260928-cw1-duke-teleporter-entry-placement
title: Temporary duke teleporter placement in the native entry room (live-boot §7/§9 binding)
mode: IMPLEMENT
status: validating
repository: Oteryn/Oteryn-Game
issue: 162
base_branch: main
branch: claude/cw1-duke-teleporter-entry-placement
pr: null
base_sha: 4e65a5e450f7f6f301abbd45fa2457d56a628ba5
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Oteryn: content world runtime
created_at: 2026-09-28T20:28:06Z
updated_at: 2026-09-28T20:28:06Z
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/content/project/native_entry.rs (and native_entry_room.json; both unchanged)
  - apps/game-server/src/content/activation.rs
  - apps/game-server/src/world_runtime.rs (only the boot-time placement injection)
  - docs/agents/tasks/active/OTV2-20260928-cw1-duke-teleporter-entry-placement.md
public_contracts: []
depends_on:
  - "allocation comment on issue #162: ALLOCATION: OTV2-20260928-cw1-duke-teleporter-entry-placement"
  - "§9 lowering (PR #1133) and §7 timed revert runtime (PR #1144), both on main"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Owner decision (#162): the `the_duke_of_the_depths` timed teleporter is placed temporarily in the
live native entry room, Channel scope, so the §7/§9 mechanism binds at live boot before the
Dangerous Depth map exists. The sample's `instance_per_party` is served on the shared Channel for
this slice.

- `world_runtime.rs`, next to the door injection:
  - `native_entry_duke_teleporter_content` builds the anchor object from the sample's own ItemRefs
    (states `canary:item/1949` and `canary:item/22761`, collision Absent, one TRANSFORM edge). It
    lowers the authored sample (`include_str!`) through `lower_map_item_transforms`, applies it,
    and links it with the qualified room's package, Content Lock, WorldId and frame. Then it
    injects synthetic, non-promotable placements for the anchor and each destination marker.
  - `bind_native_entry_duke_teleporter` binds that placement through `LocalObjectRuntime::bind`
    under the room's fence. It binds every transition the lowered content declares for the
    teleporter definition, so it also consumes a later re-arm edge (T1, D90) without change.
  - `bind_native_entry_door` returns the same door runtime as before. It now also binds the
    teleporter, fail-closed, on every Channel activation (`node/serve.rs` and the WP5 qualification
    harness both call it).
- `content/activation.rs`: a boot test (below). `native_entry.rs` and `native_entry_room.json` are
  unchanged, so no digest or lock is regenerated.

## Placement choice

Option (b), a boot-time injection like the door's. Option (a) would add records, cells or bounds
to the room. That contradicts #940's accepted binding (exactly three cells plus the one door,
bounds `(0,-1,2,1)`, one door definition) and changes the qualified room identity and digests.
Option (b) touches no accepted binding: the fence hashes only the package, Content Lock, WorldId
and frame, so the teleporter content binds under the door's own fence.

| Anchor | Room cell | Why |
|---|---|---|
| `exit_teleporter` | `oteryn:cell/entry-east` (1,0) | walkable, reachable, not the login cell |
| `reward_destination` | `oteryn:cell/entry-door` (1,-1) | walkable, behind the door |
| `warzone_exit` | `oteryn:cell/entry-start` (0,0) | the exit returns to the start |

## Tests

`content::activation::tests::native_entry_boot_binds_the_duke_teleporter_beside_the_unchanged_door`:
activate the room as the node boot does, then `into_channel_parts`, build the activation fence, and
check:

- the door content is unchanged (one definition, no placements);
- the door binds closed and USE opens it;
- the teleporter binds at its anchor in `canary:item/1949`, with `revert_after_ms` 1,200,000 and
  the lowered `/revert` inverse;
- `attributes()` is `None` in the natural state;
- USE returns `NothingToUse` with no state or revision change;
- each anchor and destination resolves to exactly one placement on its cell, in the Channel's World
  and the room's frame.

Existing seam, USE qualification and node-boot tests pass unchanged.

## Excluded scope (other lanes)

- T2: the encounter death trigger that commits the forward through `ScopeRevertDriver`.
- T4: revert wake hosting. Nothing holds the bound teleporter runtime yet. Holding it needs the
  Channel activation owner (`node/serve.rs`, not owned here) to keep
  `bind_native_entry_duke_teleporter`'s result.
- T5–T8: relocation and step-on consuming `attributes()`, the server push, and client work.
- D91 typed transition origin: once open, the untimed bound inverse stays USE-selectable (the
  existing #1144 carry-over (b)).
- Removing this temporary placement when the real Dangerous Depth map lands.

## Validation

- `cargo +1.94.0 fmt --all --check`: pass.
- `cargo +1.94.0 clippy --locked -p oteryn-game-server --all-targets -- -D warnings`: pass.
- `cargo +1.94.0 test --locked -p oteryn-game-server` (the full package): pass.
- `cargo +1.94.0 run --locked -p oteryn-architecture-check -- workspace .`: PASS.
- `python3 tools/agents/validate_governance.py`, `python3 tools/repository/validate_repository_policy.py`: pass.
- No content roundtrip or lock validator applies: no content, room source or lock file changed.

## Context checkpoint

last_progress: authored and validated; PR opening
jira: pending (no mapped Story resolved in this worker session)
