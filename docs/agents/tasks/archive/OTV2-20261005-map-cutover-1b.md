# OTV2-20261005-map-cutover-1b

```yaml
task_id: OTV2-20261005-map-cutover-1b
title: Bundle World domain 17 cutover (MAP-CUTOVER-1b, capability 18)
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/map-cutover-1b-20261005
issue: 1622
pr: 1916
base_sha: f6894e793c162d9d8a43578332f3a6f77d2936a3
head_sha: "exact frozen head in the control-plane FREEZE_SHA entry"
final_head_sha: "exact frozen head in the control-plane FREEZE_SHA entry"
final_head_frozen_at: null
owner: writer session_018jwcjAQqagaVVh5pYjw3yG
created_at: 2026-10-05T00:00:00Z
updated_at: 2026-10-07T00:00:00Z
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
  - apps/game-server/src/content/item_ref.rs
  - crates/protocol-oteryn/src/world_map.rs
  - docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json
  - docs/contracts/OTERYN_GAME_ERROR_CODE_REGISTRY.json
  - docs/agents/tasks/active/OTV2-20261005-map-cutover-1b.md
  - docs/agents/tasks/archive/OTV2-20261005-map-cutover-1b.md
public_contracts:
  - protocol-oteryn v1 capability 18 WORLD_MAP_VIEW_V1 offer gate, state domain 17
depends_on: ["#1622", "MAP-ITEM-REF-1 part A (#1906)", "MAP-ITEM-REF-1 part B (#1909)"]
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

- MAP-ITEM-REF-1 part B (#1909, merged): capability 4 is offered in the Item view set
  `ITEM_VIEW_OFFERED_CAPABILITIES`. A bundle World offers that set plus 18, so
  `a_bundle_world_admits_only_a_client_that_selects_18` passes.
- `main` with part B is merged into this branch before freeze.

## Out-of-path edits (accepted by the control plane)

- `gameplay_transport/capabilities_tests.rs`: the negotiation fixture gains the required
  capability so the bundle offer set, the capability 18 admission gate and the unchanged fixture
  offer set are tested where the other capability tests live.
- `tests/map_cutover_boot.rs`: `boot` now takes the item definition closure that §1.6 requires,
  so the MAP-CUTOVER-1a boot tests pass one.
- `OTERYN_GAME_ERROR_CODE_REGISTRY.json`: 2013 (the bundle World refusal `WorldBundleUnserved`)
  moves from ACTIVE to RETIRED because a configured bundle is now served; the code is not reused.
- `content/item_ref.rs` (#1916 review repair): `ItemDefinitionIndex::revision` returns a key's
  pinned revision, so the bundle boot resolves an Item from the active generation's index.

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
- [x] A fixture World offers exactly the production set; a bundle World offers the Item view set
      plus 18.
- [x] A bundle World admits only a client that selects 18.

## Excluded scope

The Channel overlay stays empty and the reset epoch is 0 until MAP-CUTOVER-1c. Capability 4 is
offered by part B (#1909), not by this task.

## Validation

cargo fmt --all -- --check: pass
cargo clippy --locked --workspace --all-targets -- -D warnings: pass
cargo test --locked -p oteryn-protocol-oteryn world_map: pass
cargo test --locked -p oteryn-game-server: pass
cargo run --locked -p oteryn-architecture-check -- workspace .: pass
python tools/agents/validate_governance.py: pass
python -m unittest discover -s tools/agents/tests: OK
python tools/repository/validate_repository_policy.py: pass
git diff --check: pass

## Self-review

- exact head: the FREEZE_SHA entry
- verdict: pass; the diff since part B changes only the bundle offer set and its test

## Independent review

- required: determined by the bound review policy and the control plane
- verdict: pending; the control plane runs the review on the frozen head
- Codex on 112ff2e9, two P1s, both repaired in the review repair candidate:
  - 4204574347: the bundle Item lookup read the entry room content, which has no Item
    definitions. It now resolves the revision and reference from the generation's Item key set
    and the facts from its Item profile. `map_cutover_b_a_bundle_with_items_boots_from_the_active_generation`
    boots an item-bearing bundle on the real boot path.
  - 4204574356: domain 17 was composed and encoded under the Channel runtime lock. Only the
    actor position and content generation are now read under the lock; the update runs after
    release.

## PR and closeout

- merge commit/result: squash merge of #1916, with CI, review and the Merge Queue pending at authoring
- ownership release: on merge

## Context checkpoint

```yaml
last_progress: record archived with pr 1916; frozen for review
status: completed
branch: agent/map-cutover-1b-20261005
head_sha: "exact frozen head in the control-plane FREEZE_SHA entry"
pr: 1916
```
