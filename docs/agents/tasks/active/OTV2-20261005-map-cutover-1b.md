# OTV2-20261005-map-cutover-1b

```yaml
task_id: OTV2-20261005-map-cutover-1b
title: Bundle World domain 17 cutover (MAP-CUTOVER-1b, capability 18)
mode: IMPLEMENT
status: waiting
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/map-cutover-1b-20261005
issue: 1622
pr: null
base_sha: f6894e793c162d9d8a43578332f3a6f77d2936a3
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: writer session_018jwcjAQqagaVVh5pYjw3yG
created_at: 2026-10-05T00:00:00Z
updated_at: 2026-10-06T00:00:00Z
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/map/facts.rs
  - apps/game-server/src/map/mod.rs
  - apps/game-server/src/map/boot.rs
  - apps/game-server/src/node/serve.rs
  - apps/game-server/src/gameplay_transport/mod.rs
  - apps/game-server/src/gameplay_transport/connection.rs
  - apps/game-server/src/gameplay_transport/capabilities.rs
  - apps/game-server/src/gameplay_transport/world_map.rs
  - apps/game-server/src/gameplay_transport/world_map_tests.rs
  - apps/game-server/src/gameplay_transport/capabilities_tests.rs
  - apps/game-server/tests/map_cutover_boot.rs
  - apps/game-server/tests/map_cutover_view.rs
  - crates/protocol-oteryn/src/world_map.rs
  - docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json
  - docs/contracts/OTERYN_GAME_ERROR_CODE_REGISTRY.json
  - docs/agents/tasks/active/OTV2-20261005-map-cutover-1b.md
  - docs/agents/tasks/archive/OTV2-20261005-map-cutover-1b.md
public_contracts:
  - protocol-oteryn v1 capability 18 WORLD_MAP_VIEW_V1 offer gate, state domain 17
depends_on: ["#1622", "MAP-ITEM-REF-1 part A (#1906)", "MAP-ITEM-REF-1 part B"]
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

A node configured with a world bundle checks it before durability (`map::boot::check`) and boots
it after content activation into a bundle World (`CheckedBundle::boot`) whose facts are the
production `BundleFacts`. Such a node offers `BUNDLE_WORLD_OFFERED_CAPABILITIES` (the production
set plus capability 18) and requires capability 18: a client that does not select it is refused
with `CAPABILITY_MISMATCH`. Domain 17 is planned from the bundle overlay and facts at the runtime
position under the step's runtime lock. A fixture World offers exactly the production set and has
no domain 17. The registry entry of capability 18 stays `offered: false`.

## Architecture and source of truth

- `docs/architecture/reviews/OTERYN_GAME_ARCH_MAP_TRACK_PACKETS_2026-10-05.md` §2.4, rulings
  §1.2 (empty overlay, reset epoch 0, plain step, `use_object` and field paths refused), §1.4
  (plan under the step lock) and §1.6 (Item reference 1 + palette id equal to the index entry,
  Terrain 1 + palette id, id 0 sent as 1). PROVEN.
- Runtime floor `z` is `0..=15`; the native floor is `-z`.
- Until MAP-ITEM-REF-1 part A's index lands on `main`, `serve.rs` derives the §1.6 index from the
  served item definitions (Item keys sorted by bytes); the candidate is switched to part A's API
  before freeze.

## Dependencies

- MAP-ITEM-REF-1 part B (CP decision on #1906): capability 4 is offered in the production set only
  with part B. `a_bundle_world_admits_only_a_client_that_selects_18` needs capability 4 in the
  production set and fails until part B merges; it is the only test that depends on part B.
- The candidate is frozen only after part B merges and `main` is merged into this branch.

## Out-of-path edits (accepted by the control plane)

- `gameplay_transport/capabilities_tests.rs`: the negotiation fixture gains the required
  capability so the bundle offer set, the capability 18 admission gate and the unchanged fixture
  offer set are tested where the other capability tests live.
- `tests/map_cutover_boot.rs`: `boot` now takes the item definition closure that §1.6 requires,
  so the MAP-CUTOVER-1a boot tests pass one.
- `OTERYN_GAME_ERROR_CODE_REGISTRY.json`: 2013 (the bundle World refusal `WorldBundleUnserved`)
  moves from ACTIVE to RETIRED because a configured bundle is now served; the code is not reused.

## High-risk authority/recovery qualification

APPLICABLE (protocol): the capability 18 offer and requirement are bound to a bundle World only;
the fixture offer set is unchanged and tested. Character writes, session-generation fencing and
persistence are untouched. No live serve harness exists for a full TLS/PostgreSQL admission; the
admission path is proven by the capability gate tests and the domain 17 path test, which runs the
production `bundle_world_map` against a `ChannelRuntimeV1` and a real movement step.

## Acceptance criteria

- [x] A bundle config is checked before durability and a fixture config skips the check.
- [x] A palette Item the §1.6 index lacks or places elsewhere refuses the boot.
- [x] Terrain and Item palette id 0 are sent as reference 1.
- [x] Placement bindings, stack counts, charges and house tiles reach domain 17 facts.
- [x] A bundle World walk uses the bundle collision index.
- [x] Domain 17 at join equals a fresh snapshot at the native position, and a step sends the delta
      a fresh view's move sends.
- [x] A fixture World offers exactly the production set; a bundle World offers it plus 18.
- [ ] A bundle World admits only a client that selects 18 (needs part B).

## Excluded scope

The Channel overlay stays empty and the reset epoch is 0 until MAP-CUTOVER-1c. Capability 4 is
not offered by this task.

## Validation

Pending on the frozen candidate.

## Self-review

- exact head: pending freeze
- verdict: pending

## Independent review

- required: determined by the bound review policy and the control plane
- verdict: pending

## PR and closeout

- merge commit/result: pending
- ownership release: on merge

## Context checkpoint

```yaml
last_progress: implementation and tests complete; waiting on MAP-ITEM-REF-1 part B
status: waiting
branch: agent/map-cutover-1b-20261005
head_sha: null
pr: null
```
