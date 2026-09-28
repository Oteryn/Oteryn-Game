# OTV2-20260928-cw1-object-state-attributes-impl

```yaml
task_id: OTV2-20260928-cw1-object-state-attributes-impl
title: Implement §9 attribute-bearing local-object state and map_item transform lowering (task A of 2)
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
issue: 162
base_branch: main
branch: claude/cw1-object-state-attributes-impl
pr: null
base_sha: 0a3d795992a68f57c595baf3f5ced87d1224a60c
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Oteryn: content world runtime
created_at: 2026-09-28T00:00:00Z
updated_at: 2026-09-28T00:00:00Z
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/content/reference_playable.rs
  - apps/game-server/src/content/project.rs
  - apps/game-server/src/content/encounter_map_item.rs
  - apps/game-server/src/content/mod.rs
  - apps/game-server/src/world_runtime.rs
  - apps/game-server/tests/content_reference_playable.rs
  - apps/game-server/tests/content_native_entry.rs
  - docs/architecture/OTERYN_INTERACTION_RELOCATION_AND_WORLD_OBJECT_OWNERS_PROPOSAL_V1.md
  - docs/agents/tasks/active/OTV2-20260928-cw1-object-state-attributes-impl.md
public_contracts: []
depends_on:
  - "owner acceptance of §9, issue #162 issuecomment-5873353684 (2026-09-28)"
  - "allocation comment on issue #162: ALLOCATION: OTV2-20260928-cw1-object-state-attributes-impl"
blocks:
  - "task B: §7 durable revert record and driver, apply_scope_operation, timed end-to-end test"
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Implements the owner-accepted §9 of
`docs/architecture/OTERYN_INTERACTION_RELOCATION_AND_WORLD_OBJECT_OWNERS_PROPOSAL_V1.md` up to
`bind`, without the §7 timed revert driver (task B):

- `LocalObjectStateDefinition.attribute_variant_of`, validated fail-closed at link time (base must be
  declared in the same vocabulary with the same collision).
- `LocalObjectStateAttributes { destination }`, `LoweredActionId`, and the two per-placement tables
  on `PlacementRef` (`local_object_state_attributes`, `local_object_revert_after_ms`), validated by
  `validate_local_object_placement_attributes` at link time and again at `bind`.
- `LocalObjectRuntime` stores both tables; `bind` requires every revert transition to be bound at
  the placement with exactly one bound inverse under §7's widened rule
  (`states[candidate.target_state].attribute_variant_of == Some(forward.source_state)` or plain
  equality); `attributes()` is a pure read of `(table, current state)`.
- `content/encounter_map_item.rs`: pure lowering of `map_item transform` + anchor + `destination`
  [+ `revert_destination`] + `revert_after_ms` into the tables, a post-revert variant state and its
  dedicated inverse, with named fail-closed rejections for open decisions 8 and 9, `interaction`,
  other fields, non-transform operations and conflicting target attributes (round-5 P2 2).
- §9 `DecisionStatus` set to ACCEPTED; §7's inherited rejection obligations narrowed to exclude
  exactly §9's admitted shape (round-5 P2 1), keeping open decisions 8 and 9 rejected.

## Representation defaults chosen by the owning lane

§9 leaves these open; each is documented in the lowering module:

- anchor -> `PlacementKey` `<encounter>/anchor/<anchor>`; a destination anchor is one synthesized
  Generic marker placement (`marker_placement`), its spatial binding supplied by the caller (§8 item 2);
- a `LocalObject` state is keyed by the `ItemRef` key it renders as;
- `LoweredActionId` = `<encounter>/<rule>/<action index>` (plus `/<branch>/<index>` inside `one_of`);
- post-revert state `<action id>/post-revert`, dedicated inverse `<action id>/revert`.

## Deviations and open points

- Project documents do not author `attribute_variant_of` (lowered as `None`): adding a serde field to
  `LocalObjectStateDocument` would break a struct literal in `tests/content_world_project.rs`, which
  is outside the owned paths. Content, world and lock bytes are unchanged.
- The inverse check does not enforce §7's intent-family pairing: no canonical inverse-family
  vocabulary exists on main beyond RETAG. The widened state rule and uniqueness are enforced.
- Two `revert_destination` occurrences on one forward transition yield two qualifying inverses;
  `bind` rejects that as ambiguous (§9 design point 3 leaves the case undesigned).

## Acceptance and evidence

- Tests: `content::encounter_map_item::tests` (lowering, widened bind rule, attributes by state,
  rejections, corpus, per-action/per-placement durations) and
  `tests/content_reference_playable.rs` (link-time variant and placement-table validation).
- A reversed-predicate mutation of the widened rule fails two of the new tests.
- `git diff origin/main -- content/` is empty.

## Context checkpoint

last_progress: implementation and focused validation complete; PR pending
jira: pending (no mapped Story resolved in this worker session)
