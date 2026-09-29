# OTV2-20260928-cw1-d91-typed-transition-origin

```yaml
task_id: OTV2-20260928-cw1-d91-typed-transition-origin
title: D91 event-owned transitions - a typed PLAYER_USE / EVENT(owner) origin per bound transition
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
issue: 162
base_branch: main
branch: claude/cw1-d91-typed-transition-origin
pr: 1187
base_sha: 3dcf3c82abc5d424542388a9f65ca1507e8746a4
head_sha: 9b80df1a42b911358c4e404a430cd087a30cdad5
final_head_sha: 9b80df1a42b911358c4e404a430cd087a30cdad5
final_head_frozen_at: null
owner: Oteryn: content world runtime
created_at: 2026-09-28T23:00:00Z
updated_at: 2026-09-29T06:00:00Z
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/world_runtime.rs
  - apps/game-server/src/content/reference_playable.rs
  - apps/game-server/src/content/encounter_map_item.rs
  - apps/game-server/src/world_object_revert.rs (tests and fixtures; round 2 amendment adds the production owner carry)
  - apps/game-server/tests/content_reference_playable.rs (literal additions only; amendment)
  - apps/game-server/tests/content_native_entry.rs (literal additions only; amendment)
  - docs/agents/tasks/active/OTV2-20260928-cw1-d91-typed-transition-origin.md
public_contracts: []
depends_on:
  - "owner decision D91 on #162 comment 5875958040 (§9 of the proposal doc)"
  - "D90 re-arm, PR #1164, merged as 195ef53a"
  - "allocation comment on issue #162: ALLOCATION: OTV2-20260928-cw1-d91-typed-transition-origin, plus its amendment"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Implements owner decision D91 (§9 of
`docs/architecture/OTERYN_INTERACTION_RELOCATION_AND_WORLD_OBJECT_OWNERS_PROPOSAL_V1.md`) as the
smallest slice. It closes the gap where USE on an open timed teleporter could select the untimed
`/revert` inverse and close it early.

- **Representation.** `PlacementRef` gains a per-placement table
  `local_object_event_transitions: BTreeMap<TransitionKey, TransitionEventOwner>`. A transition it
  names is `TransitionOrigin::Event(owner)`; any other is `TransitionOrigin::PlayerUse`. It is
  per placement, like `local_object_revert_after_ms`, because the lowered forward is a shared
  per-definition content transition. Nothing is serialized, so project and content bytes are
  unchanged, and existing content (including the native entry door) keeps every edge
  `PlayerUse`.
- **Lowering** (`content/encounter_map_item.rs`). `LoweredPlacementTables.event_transitions`
  marks the authored forward, the `/revert` inverse and the `/rearm` forward as
  `Event(<encounter key>)` at the anchor placement.
- **Linker** (`validate_local_object_placement_attributes`). A non-local-object placement must
  carry no event table. Each entry must name a transition of the same content and definition.
- **Runtime** (`world_runtime.rs`).
  - `bind` reads the table. It requires each timed forward and its validated inverse to be
    event-owned by one owner, which keeps the rule fail-closed.
  - USE selection, and the session `apply`/`resume_pending` path, consider only `PlayerUse`
    edges. An `Event` edge is refused with the existing "does not bind" error.
  - `apply_scope_operation` refuses a `PlayerUse` edge. It keeps the scheduling-capability rule
    for timed edges.
  - The interim #1144 rule, that a `revert_after_ms` edge is not USE-selectable or
    session-invocable, is subsumed.

## Tests

- **Gap regression:**
  `world_object_revert::tests::use_on_the_open_duke_teleporter_cannot_select_its_revert_and_an_early_event_revert_is_stale`.
  - After the kill opens the teleporter, USE returns `NothingToUse`. A session command naming
    `/revert` is refused and nothing mutates.
  - An early event-owned scope revert then makes the timer terminalize `STALE_STATE`. The
    defensive STALE handling is retained.
- **Door:** `world_runtime::tests::native_entry_door_edges_stay_player_use_and_scope_operations_refuse_them`.
  - Both door edges are `PlayerUse`.
  - A scope operation on the open edge is refused without mutation.
  - USE still opens and closes the door.
- **Bind:** `encounter_map_item::tests::a_timed_forward_and_its_inverse_must_share_one_event_owner`.
  - A player-use timed forward is rejected.
  - A player-use or foreign-owned inverse is rejected.
  - An untimed placement stays player-use.
  - A dangling event entry is refused by the re-link.
- **Lowering:** the duke lowering test asserts the three event-owned edges.
- **Duke bind test:** `/revert` is refused for USE and session, and commits through the event
  scope path.
- **Updated to D91 behaviour** (coordinator decision 3): the "MEND stays session-invocable"
  assertion, and the `/revert`-is-USE-selectable test. Timed wall fixtures mark CRACK and MEND
  event-owned (decision 2).

## Excluded scope and open points

- **Review round 2 (#1187, on 975838d7):**
  - **P1 4127644491, accepted and repaired.** `ScopeLocalObjectOperation` now carries the
    executing event's `TransitionEventOwner` as trusted execution evidence.
    `LocalObjectRuntime::check_scope_owner` refuses a player-use edge, and another owner's edge
    with the named `WorldRuntimeError::EventOwnerMismatch`. It runs in `apply_scope_operation`
    before `prepare`, and in `ScopeRevertDriver::apply_forward` before any ordinal is minted.
    `PendingRevert` records the forward's owner, and the timer-origin inverse executes on that
    owner.
  - **P2 4127644498, accepted and repaired.** `bind` validates the unextended
    `ReferenceContentGeneration` against the activation fence. It then extends the runtime's
    generation with a domain-separated digest of the placement's complete event-origin table.
    Every scope operation, session command, `RetainedBindingIdentity` and revert record fences
    on that generation, so a PLAYER_USE/EVENT or owner change is a different binding identity.
    An empty table leaves the generation unchanged, so the door's bytes and all content locks
    are unchanged. The Content generation itself never includes placements.
- **Open PR #1165:** its boot binding of the duke teleporter copies the lowered state and revert
  tables. It must also copy `tables.event_transitions`, or its `bind` fails closed ("not
  event-owned").
- **§10 absent-state design** (PR #1182): not considered here.

- **Round 2 tests:**
  - `world_object_revert::tests::scope_operations_commit_only_their_own_owners_event_edges`:
    a foreign-owner forward is refused before any ordinal or record, and a foreign-owner inverse
    is refused. The correct owner commits, and the driver fires the inverse on the same owner.
  - `encounter_map_item::tests::the_event_origin_table_is_part_of_the_binding_identity`:
    origin-differing and owner-differing content binds under different generations, while the
    Content generation is unchanged. An empty table equals the base, and an operation fenced on
    the other binding is `BINDING_MISMATCH`.

## Validation

- `cargo +1.94.0 fmt --all --check`: pass.
- `cargo +1.94.0 clippy --locked --workspace --all-targets -- -D warnings`: pass.
- `cargo +1.94.0 test --locked -p oteryn-game-server`: the full package passes.
- `cargo +1.94.0 run --locked -p oteryn-architecture-check -- workspace .`: see PR.
- `python3 tools/agents/validate_governance.py` and
  `python3 tools/repository/validate_repository_policy.py`: see PR.
- `git diff origin/main -- content/`: empty.

## Terminal integration

- **Final head:** `9b80df1a42b911358c4e404a430cd087a30cdad5`.
- **Integration:** merged into main as PR #1187, merge commit
  `cf45e5e2` (2026-09-28T23:03:38Z).
- **Closeout:** the record was archived by `OTV2-20260929-cw1-od9-create-teleporters` (issue #162,
  allocation comment 5884353101).
- **Owned paths:** released.
- **Follow-up (binding carry-over):** open PR #1165 must copy the lowered
  `tables.event_transitions` into its boot-bound duke placement and pass the executing event's
  owner, or its `bind` fails closed. The coordinator records the carry-over on #162. The
  `absent: false` literal that `OTV2-20260929-cw1-od9-create-teleporters` adds to
  `LocalObjectStateDefinition` applies to #1165 as well.

## Context checkpoint

last_progress: merged as cf45e5e2 via PR #1187; archived as completed
jira: pending (no mapped Story resolved in this worker session)
