# OTV2-20260928-cw1-d91-typed-transition-origin

```yaml
task_id: OTV2-20260928-cw1-d91-typed-transition-origin
title: D91 event-owned transitions - a typed PLAYER_USE / EVENT(owner) origin per bound transition
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
issue: 162
base_branch: main
branch: claude/cw1-d91-typed-transition-origin
pr: null
base_sha: 3dcf3c82abc5d424542388a9f65ca1507e8746a4
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Oteryn: content world runtime
created_at: 2026-09-28T23:00:00Z
updated_at: 2026-09-28T23:00:00Z
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/world_runtime.rs
  - apps/game-server/src/content/reference_playable.rs
  - apps/game-server/src/content/encounter_map_item.rs
  - apps/game-server/src/world_object_revert.rs (tests and fixtures only; allocation amendment)
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

- **Owner check:** `apply_scope_operation` does not yet check the owner the caller names.
  `ScopeLocalObjectOperation` carries none, and adding one needs production edits in
  `world_object_revert.rs`, which is outside this allocation.
- **Binding identity:** the origin is fixed per bound runtime at `bind`. It is not added to
  `ReferenceContentGeneration` or `RetainedBindingIdentity`. Session commands only ever reach
  `PlayerUse` edges.
- **Open PR #1165:** its boot binding of the duke teleporter copies the lowered state and revert
  tables. It must also copy `tables.event_transitions`, or its `bind` fails closed ("not
  event-owned").
- **§10 absent-state design** (PR #1182): not considered here.

## Validation

- `cargo +1.94.0 fmt --all --check`: pass.
- `cargo +1.94.0 clippy --locked --workspace --all-targets -- -D warnings`: pass.
- `cargo +1.94.0 test --locked -p oteryn-game-server`: the full package passes.
- `cargo +1.94.0 run --locked -p oteryn-architecture-check -- workspace .`: see PR.
- `python3 tools/agents/validate_governance.py` and
  `python3 tools/repository/validate_repository_policy.py`: see PR.
- `git diff origin/main -- content/`: empty.

## Context checkpoint

last_progress: D91 implemented and validated locally; PR opened for CI and review
jira: pending (no mapped Story resolved in this worker session)
