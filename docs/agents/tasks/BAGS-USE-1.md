# BAGS-USE-1

```yaml
task_id: BAGS-USE-1
title: "BAGS-USE-1: Use from nested bags and the field-5 hotkey search"
mode: IMPLEMENT
status: ready
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/happy-davinci-ijz3wy
issue: 1622
pr: null
base_sha: 9e5770362a96b2e0e269abf1480f7de3a17163fe
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md § ITEM-LOOP-1: BAGS-USE-1 (BAGS0 §8)"
class: "impl; combat review, persistence review (migration)"
migration: yes
registry: yes
owner: Codex lane 3 (2026-10-22)
control_plane: session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-22
updated_at: 2026-10-22
execution_policy: continuous_progress
merge_authority: CP_ONLY
owned_paths:
  - crates/gameplay/items/src/hotkey_search.rs
  - crates/gameplay/items/src/hotkey_search/hotkey_search_tests.rs
  - crates/gameplay/items/src/lib.rs
  - crates/gameplay/dispatch/src/gameplay_transport/item_use.rs
  - crates/gameplay/dispatch/src/gameplay_transport/item_use/item_use_postgres_tests.rs
  - crates/gameplay/dispatch/src/gameplay_transport/item_use/item_use_postgres_tests/item_use_view_refresh_tests.rs
  - crates/gameplay/dispatch/src/gameplay_transport/item_use/item_use_postgres_tests/wire_dispatch.rs
  - crates/gameplay/dispatch/src/gameplay_transport/container_view.rs
  - crates/gameplay/dispatch/src/gameplay_transport/container_view/refresh.rs
  - crates/gameplay/dispatch/src/gameplay_transport/connection/item_use_view_refresh.rs
  - crates/gameplay/dispatch/src/gameplay_transport/bundle_gates_tests.rs
  - crates/gameplay/dispatch/src/gameplay_transport/connection.rs
  - crates/gameplay/dispatch/src/gameplay_transport/mod.rs
  - crates/gameplay/durability/src/durability/item_use.rs
  - crates/gameplay/durability/src/durability/item_use/carried_inventory.rs
  - crates/gameplay/durability/src/durability/item_use/carried_source.rs
  - crates/gameplay/durability/src/durability/item_use_audit.rs
  - apps/game-server/migrations/0131_item_use_nested.sql
  - apps/game-server/tests/support/item_use_postgres_cases.rs
  - apps/game-server/tests/support/item_use_postgres_cases/item_use_nested_tests.rs
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - content/items/definitions/spell-native-profiles.json
  - tools/content-migration/register_spell_families.py
  - tools/content-migration/test_monster_seven_spell_overlay.py
  - content/spells.manifest.json (generated)
  - content/items/index.json (generated)
  - content/manifest.json (generated)
  - content/content.lock.json (generated)
  - docs/agents/tasks/BAGS-USE-1.md
public_contracts: []
depends_on:
  - "ITEM-USE-1c: merged (#2082)"
  - "BAGS-1: merged (#2190, ITEM-LOOP-1)"
  - "BAGS0 ACCEPTED; ITEM-USE-0 amendment §8"
  - "Migration lease: one migration PR in the queue at a time (integrator)"
blocks:
  - RUNE-CAST-1
  - QUIVER-1
cross_repository_coordination_id: null
external_repositories: []
```

## Module layout, authority and continuation 1h

Pre-edit layout: pure BFS/tests in items/hotkey_search.rs + child; fenced reads/plans/locks
in durability/item_use/{carried_inventory,carried_source}.rs; dispatch item_use.rs = wiring,
item_use/item_use_postgres_tests.rs = wire test; support/item_use_postgres_cases/ child =
durable tests. Bodies moved once, never duplicated. Parent growth stays below 150 lines.

P1 repair pre-edit layout: the existing MOVE observation/refresh loop moves into
dispatch/container_view/refresh.rs; connection/item_use_view_refresh.rs owns USE's frame
wiring. connection.rs only delegates. Tests live in the existing PostgreSQL owner's
item_use_postgres_tests/item_use_view_refresh_tests.rs child; its existing wire exchange
moves into wire_dispatch.rs, shared by the original test and the new cases. All are L5
transport composition; no durable rule or migration changes.

AUTHORING, integrator publication; starting 20e6a3e50, main a9b388fae. Only the 1e-added
SeamRooms entry moves to its fixture child. Content allocation is 1g; monk belongs to
EQUIP-RT-1. 0131 follows MAP-PICKUP-1 0130. No Git/external writes or spending.

## Final behavior and reuse

Field 2 admits open carried-view handles; field 5 shares PREPARE/reservation/cooldowns/commit/
effects. BFS follows slots, main display order, then parent/level order; closed bags count,
reserved stacks skip, oversized input refuses whole.

Tree-read choice: no complete runtime projection exists. One fenced, root-locked snapshot
reads 500 tree items + nine slots, with overflow/depth-9 sentinels. Ancestors/source/flask
lock in UUID order, then slot; custody rechecks. Source depth 1..8; flask stays in main;
nested retirement frees no main entry. Foreign/non-carried/equipped/self-parent refuse.

Reused production: items::hotkey_search::first_unreserved, ItemDefinitionIndex::definition_at,
GAMEITEM01_CONTAINER_ENTRIES_MAX, game_item_loop_owner, existing plans/effects/cooldowns/eat_food.
Snapshot/flask policy moved into durability; old dispatch bodies removed.

Reused tests: bestiary_postgres_harness (single registration, runtime/Harness/configured_admin/
debug/id/item_fence), native_gameplay::test_support::activated_with_loot_tables, serve_admitted,
actor_spell::tests::FACTS, Use/run/item/uuid/flask_stack/flask_new. No new fixture wrapper.
Generator tests reuse AcceptedOverlayTests.base, producer.canonical_bytes, overlay.digest,
exact-output replay. Wire database names are unique; runtime/locks/duplex I/O are local.

## Migration chain

0131 starts from latest bodies: consistency 0083; removal 0095; nested proof 0128; placement
0113. Other proof arms stay; existing nested exclusivity stays. 0130 changes other functions.
Placement amendment binds fresh flasks to main. Main migrations <=0130 are byte-identical.

No migration digest registry/generator exists (MAP-PICKUP-1/tool/schema search).
SQLx GAME_MIGRATOR embeds/checksums all files; schema.rs checks them. 90 versions unique, ending at 0131; historic gaps preserved.

## Content evidence and generator

Local source: docs/agents/evidence/OTV2-20261004-item-consumption-facts-v1.json and its pinned
snapshot, described by OTV2-20261004-item-sem-use-2-consumption-sources.md.
- Meat i3577: FOOD regeneration_seconds=180, TibiaWiki Meat revision 1113663; corroborated by
  Canary 04b83b512114bfd888000d6e1433ed8ecaec7c5b foods.lua line 4 (15 times 12 seconds).
- Health Potion i266: HEALTH min=125/max=175, flask i285, no source level/vocation restriction.
  Range is OTS_HYPOTHESIS_ONLY Canary same revision potions.lua; flask is TibiaWiki Health Potion
  revision 1183252 / small-flask page. Canonical content/world/definitions/reference.json already
  admits i266/i285 with those semantics. All lane fixtures use these identities; no legacy alias.

The generator overlays meat's pinned fact and World i266/i285 policies via exact Crystal
bindings; unknown attributes stay unknown. Changed evidence/identity/semantics refuses before
writes; coin/door pins stay validated. Five outputs regenerated (413 -> 415 policies), using
PYTHONPATH=/home/mole/oteryn-cache/pylib; replay/content-tree checks pass.

## Findings and CI handoff

- cross_domain_finding (equipment): character_equipment.rs / 0083 admit no equipped burn or
  transform. First unreserved equipped match refuses NothingToUse without falling through.
- 1d negative assertion change: item_use_audit.rs different-parent refusal became self-parent
  refusal, because BAGS-0 lifts different-parent custody. Other old negative assertions stay.
- cross_domain_finding (P1, slot-only): connection.rs:3576 committed USE refreshes domains 9
  and vitals but never ContainerViewState. Existing wire command 4 consumes i266 in the open
  inner bag: DB quantity 2 -> 1, while domain 14 still displays 2; last-unit retirement also
  leaves its visible entry/handle. Applies to food/potions, handle/hotkey, in any open carried
  view. `views.refresh` is called only by ITEM_MOVE (connection.rs:3029).
  RESOLVED in the integrator-authorized P1 repair on 6946a3439: MOVE and USE now share the
  extracted container_view/refresh.rs path. Committed USE updates ContainerViewState,
  handles, continuity and changed domain 14 frames in the same turn. Unchanged views emit
  nothing; without selected capability 14 no container refresh or delta is produced.
  Original MOVE behavior and tests are unchanged. See the P1 validation receipts below.
- cross_domain_finding (CI): no dispatch *_postgres lib-test PG runner/data list/naming hook
  exists. merge-group-gate.yml:502 selects it via `not test(content::)` without PG, so it skips.
  PG jobs run integration targets only (same workflow:703-706; pr-postgres-harness.yml:223-226).
  REQUIRED INTEGRATOR CHANGE: in both workflows' durability_postgres/rest leg, after registered
  targets, run `cargo +1.95.0 test --locked -p oteryn-gameplay-dispatch --lib item_use` under
  their existing PostgreSQL service/env (guard `SHARD == rest`). Workflow files untouched.

## Validation receipts (1h)

Rust commands/logs: target/bags-use-1h-canonical/receipts.txt and matching logs.
Python commands/logs: target/bags-use-1h/python-receipts.txt.
Canonical cargo runs use the PATH queued wrapper in a non-login shell, +1.95.0, --locked
(except fmt), --quiet. Initial login-shell runs bypassed the wrapper/PG and are discarded.
PG admin URL comes from the wrapper; no credential/Docker access. Explicit fmt also covers
durability/item_use.rs and its children because cargo fmt skips the include! module tree.

PASS fmt (including explicit durability files), four-package all-target clippy -D warnings,
architecture workspace. Existing vendor Tokio missing_docs warning remains.
PASS --lib: items 29; durability 385; server 306/3 ignored; content 339/1 ignored.
Dispatch --lib x3: 902 passed/1 failed/14 ignored (206.29s); 903/0/14 (166.78s);
903/0/14 (132.21s). Only failure:
`gameplay_transport::monk_save::postgres_tests::boundary_unknown_monk_save_keeps_lease_until_store_recovers`
returned RootUnavailable; EQUIP-RT-1 owns it, no investigation here. Ignored tests pre-exist.
PASS focused: hotkey_search 4; item_use_audit 8; dispatch item_use 9; standalone wire 1 (18.95s).
PASS PG: item_use_postgres 399; check_function_privileges_postgres 1;
character_authority_postgres -- item_use 21 / -- item_loop 6;
durability_postgres --test-threads=4: 528 (319.00s). Migration-chain.log: 89 main files unchanged.
PASS generator --check (36 files); content-tree; overlay tests 12; import tests 9; duplicate-source guard unit tests 13.
PASS python3 tools/agents/validate_governance.py (22 documents / 9 lanes);
`python3 -m unittest discover -s tools/agents/tests` (124);
governance_lifecycle tests (25);
META adoption tests (30);
`python3 tools/repository/validate_repository_policy.py` (23 files / 67 workflows).
NOT RUN inherited prompt policy: integrator's network-only HTTP 403 exception; no retry/bypass.
Bound META 3.1.0 at 1bfb5ff98c8aa156e73669a14e083a1d464c29fb refreshed read-only by immutable URL.
NOT RUN --workspace clippy: D1466 permits only when Cargo.toml/Cargo.lock changes; none changed.

## Size and whole-diff review

Against origin/main a9b388fae, files growing >200 lines:
- migrations/0131_item_use_nested.sql: 0 -> 435; L4 database guards require latest full bodies.
- durability/item_use/carried_inventory.rs: 0 -> 259; L4 fenced snapshot and durable plans.
- dispatch/item_use/item_use_postgres_tests.rs: 0 -> 473; real wire owner/PG qualification.
- support/item_use_postgres_cases/item_use_nested_tests.rs: 0 -> 285; durable negative/replay cases.
- content/items/definitions/spell-native-profiles.json: 61416 -> 61712; generated source policies.
Touched files over 1500 lines: connection.rs 8261 -> 8282 (USE/authority routing only);
dispatch mod.rs 8167 -> 8176 (trait delegation only); RESOURCE_LIMITS_REGISTRY.json
12446 -> 12469 (one data registration); generated profiles as above (content data only).
Shared changes are wiring/data only. The USE view-refresh slot edit is resolved above.
CODEX-CHECKLIST review/apps+crates+tools scans cover new types/fields/trait/callers/E2E.
No test weakening, old migration edit, protocol change or new dependency.

## Planned commits (integrator only; no commits made)

1. `items: bounded hotkey selection`: hotkey_search.rs + child tests.
2. `durability: consume nested carried stacks`: item_use.rs, item_use_audit.rs,
   item_use/carried_inventory.rs, item_use/carried_source.rs, migrations/0131_item_use_nested.sql.
3. `dispatch: shared nested PREPARE`: item_use.rs, bundle_gates_tests.rs.
4. `tests: nested use and foreign custody`: dispatch item_use/item_use_postgres_tests.rs,
   support/item_use_postgres_cases.rs and item_use_postgres_cases/item_use_nested_tests.rs.
5. `content: register source-backed consumables`: register_spell_families.py,
   test_monster_seven_spell_overlay.py and five generated content files in owned_paths.
6. `docs: BAGS-USE-1h qualification`: this card.
7. Existing WIP wiring: connection.rs, dispatch mod.rs, items lib.rs, RESOURCE_LIMITS_REGISTRY.json.

## Validation receipts (P1 container refresh repair, 2026-10-22)

AUTHORING repair against committed HEAD 6946a3439; local files only, integrator publication.
Migration 0131 and existing item_move_tests.rs / container_view_tests.rs are byte-identical.
All Cargo commands use the queued wrapper in a non-login shell, +1.95.0 and --locked
(except fmt). PostgreSQL 17.6 runs here via the wrapper's existing admin environment.

PASS `cargo +1.95.0 clippy --locked -p oteryn-gameplay-dispatch --all-targets --quiet -- -D warnings`.
PASS `cargo +1.95.0 fmt --all --check` and `git diff --check`.
The existing vendor Tokio missing_docs warning remains; the requested Clippy command exits 0.

PASS `cargo +1.95.0 test --locked -p oteryn-gameplay-dispatch --lib item_use --quiet`:
13 passed, 0 failed, 0 ignored, including the original PG wire scenario and these new
real-owner/real-dispatch PostgreSQL cases under item_use_postgres_tests::item_use_view_refresh_tests:
- `item_use_last_charge_removes_the_entry_from_the_open_bag_in_the_same_turn`
- `item_use_potion_to_flask_updates_both_open_bags_in_the_same_turn`
- `item_use_in_a_closed_bag_emits_no_update_for_unchanged_open_views`
- `item_use_without_container_capability_emits_no_container_delta`

The potion case covers the existing durable potion-to-flask behavior: the nested source
retires, the flask appears in the open main bag, and both changes arrive in one domain 14
delta. Tests also assert command results, source quantities, receipts, revision chaining
and unchanged-entry handle continuity. Distinct harness tags isolate recovery stores.

PASS `cargo +1.95.0 test --locked -p oteryn-gameplay-dispatch --lib item_move --quiet`:
40 passed, 0 failed, 0 ignored; original tests unchanged.
PASS `cargo +1.95.0 test --locked -p oteryn-gameplay-dispatch --lib container_view --quiet`:
15 passed, 0 failed, 0 ignored; includes the existing MOVE refresh/closure tests.
PASS `python3 tools/agents/validate_governance.py` (22 documents, 9 lanes) and
`python3 -m unittest discover -s tools/agents/tests` (124 tests).

Reuse: MOVE's observation loop and ContainerViewState::refresh; observe_inventory,
inventory_committed, item_view_delta and write_frame; existing scenario/runtime/Harness,
WireOwner, native gameplay activation, configured_admin/debug/id/item_fence, and
serve_admitted. The old wire exchange body was moved once into the test-only
wire_dispatch::exchange seam; no fixture/harness body or test registration was copied.

Whole repair diff re-read against ~/oteryn-cache/CODEX-CHECKLIST.md. Workspace-wide
apps/crates/tools scans cover all four new test-only enum variants and every new helper
caller; no production enum, field or trait method was added. Both production refresh
functions have real dispatch callers and the PG E2E coverage above. No existing file grew
by more than 150 lines; no repair file grew by more than 200 lines (largest new file: 197).
The integrator-owned CI PostgreSQL routing finding is closed by CI-PG-LIB-1 (#2300), which runs
the dispatch library PostgreSQL tests in the harness;
no implementation or local validation is unfinished for this P1 repair.
