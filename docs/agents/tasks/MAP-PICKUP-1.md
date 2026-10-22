# MAP-PICKUP-1

```yaml
task_id: MAP-PICKUP-1
title: "MAP-PICKUP-1: Command 9 from a movable base-map entry"
mode: IMPLEMENT
status: ready
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/happy-davinci-ijz3wy
issue: 1622
pr: null
base_sha: 305cfc7e21c8322d5e76e12a2cc60f85a9dd92dd
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md:301"
class: "hard; persistence review"
migration: yes
registry: no
owner: Codex lane 2 (2026-10-22)
control_plane: session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-22
updated_at: 2026-10-22
execution_policy: continuous_progress
merge_authority: CP_ONLY
owned_paths:
  - crates/gameplay/dispatch/src/gameplay_transport/loop_map_pickup.rs
  - crates/gameplay/dispatch/src/gameplay_transport/loop_map_pickup_tests.rs
  - crates/gameplay/dispatch/src/gameplay_transport/loop_move.rs
  - crates/gameplay/dispatch/src/gameplay_transport/item_move.rs
  - crates/gameplay/dispatch/src/gameplay_transport/item_move_tests.rs
  - apps/game-server/tests/support/map_item_mint_postgres_cases.rs
  - crates/gameplay/durability/src/durability/item_loop_move.rs
  - crates/gameplay/durability/src/durability/map_item_mint.rs
  - crates/gameplay/dispatch/src/gameplay_transport/mod.rs (fail-closed map-view read only)
  - apps/game-server/migrations/0130_item_loop_map_pickup.sql
  - crates/gameplay/world/src/map/facts.rs and tests (run 1d allocation)
  - crates/gameplay/content/src/content/item_ref.rs and tests (run 1d allocation)
  - crates/gameplay/dispatch/src/gameplay_transport/monk_save_postgres_tests.rs (test registration and reusable generation wrapper)
  - docs/agents/tasks/MAP-PICKUP-1.md
slot_only_paths: []
public_contracts: []
depends_on:
  - "MAP-WIRE-2: merged (ItemKey::MapBase handle)"
  - "MAP-OVERLAY-1b: merged (map MINT and overlay pickup)"
  - "ITEM-LOOP-1: merged (#2190)"
blocks:
  - GROUND-MOVE-1
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome and remaining integration

Implementation complete; validation results and remaining integration follow. Continue
from WIP `17e0f2cb7` / `5658a26ad`, base `305cfc7e2`. No Git writes, publication, paid review or dependency changes. Independent
persistence review, migration renumbering and PR integration remain with the
integrator. Reset epoch is 0, matching `BundleMapPosition::update`.

META 3.1.0 was refreshed read-only at the bound immutable revision. OPTION A
resolves eligibility/reference allocation.

## Resolved findings and implementation

- Migration 0130 and the loop owner accept only receipt-proven, unchanged map-MINT
  Ground rows of the current Channel. MINT retains its positive append ordinal;
  the base ordinal never substitutes for it. Other World/Channel, unproven Ground,
  stale quantity/revision, timed and temporal sources remain refused.
- Original boot attributes and the exact existing eligibility-wrapper result are
  additive facts. Normalized count, subtype, boundness and all existing view
  assertions remain unchanged. Compact references resolve through the exact
  generation index to typed definitions; appearance ids never supply keys.
- Command 9 admits MapBase only into the main backpack or an instance container.
  The spell permit and native-generation gate cover MINT plus the direct loop
  pickup. No intermediate-backpack workaround or B3-1 TRANSFER is used.
- The shared destination planner preflights room, merge, tree, nesting and weight
  before reservation; the move rechecks transactionally.
- Loop receipts replay before handle resolution. A hidden origin resumes only its
  own MINT receipt or pending reservation, then reconciles and moves the same item.
  A new command sees STALE without writing. Unknown MINTs retain their origin;
  proven non-commits clear it. Pending origins retain their CommandRef.
- Ground rebuild reads all taken placements for the bounded base entry count and
  re-hides them. Cache invalidation exposes both commits to the next map update.
  A failed Ground or placements read fails the map view closed, with no boot-map
  fallback. `world_map.rs` remains untouched.

## Coverage and review

- `map_pickup_dispatch_mints_moves_rehides_resends_and_replays_after_a_crash`:
  real command 9, issued domain-17 source and carried-bag target handles. An injected move
  failure leaves one Ground MINT. A recreated cache re-hides the origin; the
  same command reconciles, moves once and replays without writes.
- `map_pickup_dispatch_to_main_backpack_resends_its_tile`: fresh main-backpack
  pickup; success cases decode the origin-tile delta and prove one MINT location.
- `map_pickup_dispatch_refusals_write_nothing_and_preserve_the_handle`: full real
  backpack, TOO_FAR, wrong digest/epoch; includes reservation/work-unit footprint.
  Both durable read failures are exercised through the composed map-view method.
- `map_item_mint_loop_preflight_and_overlay_acceptance_batch`: existing SQL and
  overlay fixtures cover pickup, second-handle refusal, per-Channel taking,
  full-destination/reach refusal, durable rebuild and digest/epoch isolation.
  Existing recovery/admission cases cover provenance/refusals; unit cases cover
  exact boot eligibility, reference inversion and source routing.
- Reviewed the whole task diff, WIP and new files against both checklists. No
  existing assertion changed, weakened, removed or ignored; no new lint allowance.
  Workspace rg covered all Placement, LoopPlan, PlannedDestination and CacheState
  constructors, touched-type consumers and public API callers. No enum variant
  or trait method was added; production APIs have dispatch coverage.

## Reuse and growth

Reused `world/map/overlay/pickup.rs` eligibility, reach, freeze/settlement and
rehide; the existing content index; native move gate and spell lane; MINT
freeze/commit/reconcile; loop receipt/move/read owners; and the shared loop planner. Reused the existing
session Harness, Dependencies, production-generation activation, bundle fixture,
map-view codecs and map-MINT/overlay PostgreSQL fixtures. Also reused
`ground_cell`, `sha256_revision`, `MoveAuthority`, inventory/intent helpers,
and the content-index `document` fixture.

`monk_save_postgres_tests.rs` retains its activated pin through `production_generation_with_pin`; the original helper
delegates and preserves its return value. Tests are mounted once.
Overlay fixtures `base` and `coin_key` gain parent visibility. No fixture body was copied or path-loaded
again, and no new test target was registered.

Files growing >200 lines (`crates/gameplay/` unless `apps/`):

| File | Why this layer |
| --- | --- |
| `world/src/map/facts.rs` | Boot-time projection plus exhaustive eligibility/normalization tests; world owns the original tile facts. |
| `dispatch/src/gameplay_transport/loop_map_pickup.rs` | Command wiring of existing content, map, MINT and loop owners; no new SQL or pure rule implementation. |
| `dispatch/src/gameplay_transport/loop_map_pickup_tests.rs` | Real dispatch/PG acceptance uses the existing composed-authority harness. |
| `apps/game-server/tests/support/map_item_mint_postgres_cases.rs` | Appended persistence/overlay acceptance uses this existing PG case harness. |

## Validation

Queued PATH cargo uses locked/offline resolution and wrapper PostgreSQL. Existing
ignores stand. The initial parallel dispatch run hit two PostgreSQL Unavailable errors. The
three DB fixtures now serialize setup/teardown; assertions and deadlines stand. Final fmt/clippy and all three dispatch cases pass.

| Command | Result / output tail |
| --- | --- |
| `cargo +1.95.0 fmt --all --check` | PASS: empty output |
| `cargo +1.95.0 clippy --locked -p oteryn-gameplay-content -p oteryn-gameplay-world -p oteryn-gameplay-durability -p oteryn-gameplay-dispatch -p oteryn-game-server --all-targets --quiet -- -D warnings` | PASS (0): all five touched packages |
| `cargo +1.95.0 test --locked -p oteryn-gameplay-content --lib --quiet` | PASS: 339 passed, 1 existing ignore |
| `cargo +1.95.0 test --locked -p oteryn-gameplay-world --lib --quiet` | PASS: 173 passed, 1 existing ignore |
| `cargo +1.95.0 test --locked -p oteryn-gameplay-durability --lib --quiet` | PASS: 384 passed |
| `cargo +1.95.0 test --locked -p oteryn-gameplay-dispatch --lib --quiet` | One parallel run PASS: 887 passed, 14 existing ignores; last run FAIL: 886 passed, 1 RootUnavailable (unchanged monk-save test), 14 ignores |
| `cargo +1.95.0 test --locked -p oteryn-game-server --lib --quiet` | Known failure: 305 passed, 1 failed, 3 existing ignores; party block-limit test: Rejected |
| `cargo +1.95.0 run --locked -p oteryn-architecture-check --quiet -- workspace .` | PASS: workspace-boundaries: PASS |
| `cargo +1.95.0 test --locked -p oteryn-game-server --test map_item_mint_postgres --quiet` | PASS: 395 passed |
| `cargo +1.95.0 test --locked -p oteryn-game-server --test character_authority_postgres --quiet -- map_item_mint` | PASS: 20 passed |
| `cargo +1.95.0 test --locked -p oteryn-game-server --test character_authority_postgres --quiet -- item_loop` | PASS: 6 passed |
| `python3 tools/agents/validate_governance.py` | PASS: 22 policy documents and 9 lanes |
| `python3 -m unittest discover -s tools/agents/tests -p '*governance_lifecycle*.py' -v` | PASS: 25 tests |
| `python3 -m unittest tools.agents.tests.test_meta_agent_policy_adoption -v` | PASS: 30 tests |
| `python3 tools/agents/validate_inherited_prompt_policy.py` | PASS initially; final rerun HTTP 403 rate limit (allocated exception) |
| `python3 tools/repository/validate_repository_policy.py` | PASS: 23 files, 67 workflows |
| `python3 -m unittest discover -s tools/agents/tests` | PASS: 124 tests |
| `git diff --check` | PASS: empty output |

Serial dispatch: PASS, 887 passed / 14 existing ignores (690.16s). Final
`map_pickup_dispatch` filter: PASS, 3 passed (61.56s), after an explicit final
fmt/check and dispatch/server clippy joined with `&&`. The last full parallel run
failed only `boundary_unknown_monk_save_keeps_lease_until_store_recovers` with
RootUnavailable; its isolated rerun passed (1/1, 5.90s). Cause UNKNOWN; monk lifecycle unchanged.

Workspace clippy was not run: explicitly forbidden without Cargo manifest/lock
changes. The known party test was neither skipped nor changed.

## Exact focused cargo filters

- `cargo +1.95.0 test --locked -p oteryn-gameplay-dispatch --lib -- map_pickup_dispatch --test-threads=1`
- `cargo +1.95.0 test --locked -p oteryn-gameplay-dispatch --lib -- map_base_pickup_routes_only_to_the_carried_tree`
- `cargo +1.95.0 test --locked -p oteryn-gameplay-world --lib -- map_pickup_preserves_wrapper_refusals_and_original_view_normalization`
- `cargo +1.95.0 test --locked -p oteryn-gameplay-dispatch --lib -- a_definition_ref_names_its_typed_definition_back`
- `cargo +1.95.0 test --locked -p oteryn-game-server --test character_authority_postgres -- map_item_mint_loop_`
- `cargo +1.95.0 test --locked -p oteryn-game-server --test character_authority_postgres -- item_loop`

## Planned commits (integrator splits and commits)

1. `feat(map): preserve exact pickup facts and item references`:
   `world/src/map/facts.rs`, `content/src/content/item_ref.rs` and its tests.
2. `feat(items): preflight and admit proven map-MINT loop pickups`:
   migration 0130, `durability/item_loop_move.rs`, `durability/map_item_mint.rs`.
3. `feat(dispatch): pick up base-map items and rebuild durable origin hides`:
   `item_move.rs`, `loop_move.rs`, new `loop_map_pickup.rs`, allocated view read in `mod.rs`.
4. `test(map): cover pickup refusal, recovery and tile resend`:
   `item_move_tests.rs`, new `loop_map_pickup_tests.rs`, test registration/generation
   wrapper in `monk_save_postgres_tests.rs`, appended PG cases and this final card.

Gameplay paths above are relative to `crates/gameplay/`; dispatch files are under
`dispatch/src/gameplay_transport/`. The integrator archives this card in the final authoring commit.
