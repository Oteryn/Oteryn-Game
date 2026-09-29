# OTV2-20260929-cw1-od9-create-teleporters

```yaml
task_id: OTV2-20260929-cw1-od9-create-teleporters
title: OD9 pre-authored CREATE teleporters - synthesized absent state, create/remove pair and the Q2=b re-kill re-arm (proposal §10.2/§10.4)
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
issue: 162
allocation_comment: 5884353101
base_branch: main
branch: claude/cw1-od9-create-teleporters
pr: null
base_sha: 8dfd069594e0b6bd4d4cd9df7226b4b8dcd69d8c
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Oteryn: content world runtime
created_at: 2026-09-29T05:40:00Z
updated_at: 2026-09-29T06:00:00Z
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/content/encounter_map_item.rs
  - apps/game-server/src/content/reference_playable.rs (the absent-state marker and its validation)
  - apps/game-server/src/world_runtime.rs (the `is_absent` accessor; `absent: false` in test literals)
  - apps/game-server/src/world_object_revert.rs (tests; the Q2=b re-arm, see below)
  - apps/game-server/src/content/project.rs (amendment 5884393736; literal-only `absent: false`)
  - apps/game-server/tests/content_reference_playable.rs (amendment 5884393736; literal-only `absent: false`)
  - docs/architecture/OTERYN_INTERACTION_RELOCATION_AND_WORLD_OBJECT_OWNERS_PROPOSAL_V1.md (the §10 status line; the §10.4 kill-while-open bullet, Q2=b)
  - docs/agents/tasks/active/OTV2-20260929-cw1-od9-create-teleporters.md
  - docs/agents/tasks/archive/OTV2-20260928-cw1-od8-od9-design.md (closeout of #1182)
  - docs/agents/tasks/archive/OTV2-20260928-cw2-encounter-vocabulary-extensions.md (closeout of #1183)
  - docs/agents/tasks/archive/OTV2-20260928-cw1-d91-typed-transition-origin.md (closeout of #1187)
public_contracts: []
depends_on:
  - "§10 design merged as 4758b981 (#1182); D91 merged as cf45e5e2 (#1187)"
  - "allocation comment on issue #162: 5884353101, and its amendment 5884393736"
  - "owner decision Q2=b on issue #162: comment 5884513528"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

This task implements §10.2 and §10.4 of
`docs/architecture/OTERYN_INTERACTION_RELOCATION_AND_WORLD_OBJECT_OWNERS_PROPOSAL_V1.md` for
`death_priest_shargon` and `the_ravager`.

- **Absent marker (`reference_playable.rs`).** `LocalObjectStateDefinition` gains `absent: bool`,
  which is `false` for every existing and authored state.
  - At link time, `validate_definition_shape` requires an absent state to be `collision: Absent`
    and not an attribute variant. It also refuses an attribute variant whose base state is absent.
  - `validate_local_object_placement_attributes` refuses any per-placement attributes for an
    absent state. It runs at link time and again at `bind`.
- **Lowering (`encounter_map_item.rs`).** A `map_item create` at an `anchor`, with `destination`
  and `revert_after_ms`, is admitted. `effect` is ignored, as for transforms. It lowers to:
  - the absent state `<action>/absent`;
  - the forward `<action>/create` (absent → the `ItemRef` state, CREATE family);
  - the inverse `<action>/remove` (REMOVE family);
  - the placement tables: `initial_state = absent`, `destination` on the present state, one
    `revert_after_ms` entry, and both edges `EVENT(<encounter key>)`.
  - The revert lands on the absent initial state, so the create re-arms without a `/rearm` edge
    (D90). `bind` accepts `/remove` as the unique inverse under the unwidened rule.
- **Still rejected, each with a named error:**
  - `revert_destination` on a create (`CreateRevertDestinationNotAdmitted`);
  - `interaction` (`InteractionNotAdmitted`);
  - `into` or any other field (`UnsupportedField`);
  - a create without `destination` or without `revert_after_ms` (`CreateWithoutDestination`,
    `CreateWithoutRevert`);
  - an item that is not in the anchor object's vocabulary (`CreatedItemNotDeclared`);
  - a create anchor shared with another `map_item` action (`CreateAnchorShared`);
  - OD8 `at: death_position` (`DeathPositionNotAdmitted`), in delayed rules too.
  - `CreateWithDestinationNotAdmitted` is retired.
- **Runtime (`world_runtime.rs`).** `LocalObjectRuntime::is_absent()` is the server-side read that
  tells the projection to render no object. Absence of collision and of a destination follows from
  the state's `collision: Absent` and its empty attributes.
- **Owner decision Q2=b (#162 comment 5884513528), which replaces the §10 no-op default.** A kill
  while the teleporter is open re-arms the pending revert to the full `revert_after_ms`, counted
  from the new kill. There is no second object and no second record. In `world_object_revert.rs`:
  - `select_open_create` finds the timed CREATE forward whose target is the current state. It is
    `None` for any TRANSFORM forward, so the duke keeps owner decision 3's no-op.
  - `ScopeRevertDriver::rearm_open_create` runs the D91 owner check (`check_scope_owner`) first. It
    then finds the unique `PENDING` record that matches the placement, incarnation, content
    generation, scope generation, inverse, owner, current state and revision. It mints one ordinal
    for the accepted re-kill, and moves the record's deadline and scheduling ordinal to now plus
    `revert_after_ms`.
  - With no unique `PENDING` record, it fails closed (`RevertError::NoOpenRevert`) and changes
    nothing. `WOBJ-RL-04/05` are unaffected, since no record is added.
  - **Scope note:** the allocation owned `world_object_revert.rs` for tests only. The Q2=b message
    asked for the re-arm "in your owned files", and the driver's records exist only in this
    module. The production change is therefore limited to these two functions and one error
    variant.
- **Proposal doc.** The §10 status line now says §10.2/§10.4 (OD9) ACCEPTED and §10.3 (OD8)
  CANDIDATE, blocked on D2a and the interaction typed target. The §10.4 kill-while-open bullet now
  states Q2=b.
- **Closeouts.** Three records are archived with terminal fields and a "Terminal integration"
  section: `OTV2-20260928-cw1-od8-od9-design` (#1182, `4758b981`),
  `OTV2-20260928-cw2-encounter-vocabulary-extensions` (#1183, `86116adf`) and
  `OTV2-20260928-cw1-d91-typed-transition-origin` (#1187, `cf45e5e2`).

## Tests

- `encounter_map_item::tests`:
  - `od9_samples_lower_to_an_absent_start_and_bind_the_create_remove_pair`, for both samples:
    - the lowered shape;
    - the CREATE/REMOVE families and the event owners;
    - `bind` picks `/remove` as the inverse;
    - before any kill the placement is absent, `attributes()` is `None` and it blocks nothing;
    - USE returns `NothingToUse`, and a session command is refused.
  - `od9_create_rejections_and_od8_in_a_delayed_rule_stay_named`.
  - `the_absent_marker_is_validated_fail_closed_at_link_and_bind`.
  - `od8_samples_stay_rejected_and_od9_and_covered_samples_are_admitted` (renamed and updated).
  - `uncovered_map_item_shapes_are_rejected_with_named_errors` (updated for the create case).
- `world_object_revert::tests`:
  - `od9_create_teleporter_opens_a_re_kill_re_arms_its_revert_and_it_re_opens_after_closing`, for
    both samples, on a `ManualClock`:
    - kill 1 at t=0 opens the teleporter with its destination;
    - a re-kill at t=4 min re-arms the same record, mints one ordinal and adds no record;
    - at t=5 min it is still open;
    - it closes at t=4+5 min;
    - kill 2 re-opens it through `/create` with a new record;
    - the run ends with two distinct `TERMINAL` records.
  - `od9_create_edges_refuse_use_session_and_foreign_owners`: USE and session are refused in both
    states. A foreign owner's create, remove and re-arm are refused with `EventOwnerMismatch`,
    with no ordinal, record or deadline change.
  - `a_re_kill_without_one_pending_revert_fails_closed_and_the_duke_keeps_its_no_op`.
- The existing duke and §9 tests are unchanged, apart from the `absent: false` literals.

## Excluded scope

- OD8 runtime placements (§10.3), which are gated on protocol D2a and the interaction typed target.
- The transport or client rendering of `absent`, the encounter-trigger wiring that calls
  `select_timed_forward`/`select_open_create`, and the teleport consumer.
- The same-origin condition in `bind`'s inverse search (§10.2). It matters only for OD8's
  `/consume` edge; OD9 lowers no second REMOVE edge.
- §12 of the encounter format, and #1165. The coordinator records #1165's carry-over (the
  `absent: false` literal and `event_transitions`) on #162.
- `content/` is unchanged.

## Validation

- `cargo +1.94.0 fmt --all --check`
- `cargo +1.94.0 clippy --locked --workspace --all-targets -- -D warnings`
- `cargo +1.94.0 test --locked -p oteryn-game-server`
- `cargo +1.94.0 run --locked -p oteryn-architecture-check -- workspace .`
- `python3 tools/agents/validate_governance.py`
- `python3 tools/repository/validate_repository_policy.py`
- `git diff origin/main -- content/`: empty

The results are in the PR.

## Context checkpoint

last_progress: implemented and validated locally; PR pending
jira: pending (no mapped Story resolved in this worker session)
