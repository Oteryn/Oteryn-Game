# CRAFT-CONTENT-1 — author craft recipes and lower them into the World Project

```yaml
task_id: CRAFT-CONTENT-1
mode: IMPLEMENT
title: "CRAFT-CONTENT-1: author craft recipes and lower them into the World Project"
status: ready
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/happy-davinci-ijz3wy
pr: null
base_sha: 9f814e72a3a32ad2a6ed67787c2d67a2d39d4af3
parent_task: CRAFT-1
updated_at: 2026-10-22
needs_migration: false
needs_registry: false
needs_content_regeneration: false
merge_authority: CP_ONLY
```

## Planned module layout and scope

L1 `content/project/v2/item_conversion.rs` owns authored types/admission;
`item_conversion/digging.rs` owns C5 return checks. `v2.rs` is registration only;
`item_conversion_tests.rs` is mounted once. Offline lowering lives in
`tools/content-schema/craft-authoring/`, with `digging.py`/`test_digging.py` children.
Integration tests live in `content_world_project_v2_craft.rs` and its one
`digging_tests.rs` child. Five existing helpers moved verbatim to the helper-only
`tests/support/content_world_project_v2_fixture.rs`, shared by both targets.
The original integration target shrank 1718→1629 lines; no duplicate tests.

Scope remains the packet plus the authorized fixture paths; derived files/pins
are tooling-only. No runtime, SQL, migration, protocol, dependency, dispatch or
neighbouring-lane changes. Bound META policy 3.1.0 was refreshed read-only at
`1bfb5ff98c8aa156e73669a14e083a1d464c29fb`. No publication or paid review.

## Final implementation and integrator decisions

The amended C3a ceiling is 2–24 alternatives: schema/compiler accept 24, reject 25.
The decision file is unchanged. **125 recipes** cover 64 baking/carried pairs,
46 fishing rows, dirty water, both corpse tables and **12 digging rows**.
The mechanical corpse retains all 19 alternatives, exact total weight 3000.

All duration-backed shovel (3457, 5710) and pick (3456) branches are present:
593→594 (60000 ms), 606→607 for shovel / 615 for pick (300000 ms), 608→609
(300000 ms), 867→868 (600000 ms). Generation/admission reject XML return
mismatches; real content dispatch round-trips all 12 rows. FLOOR-1 LocalObject
ownership stays explicit; no permanent map writes, yields or invented timer.

**Accepted open follow-ups, unchanged:** 7200→7236 and 21341→21342 remain
fact-only: **"pinned XML has no return duration"**. Both forms have null duration
and decay target. CRAFT-DIG-1 needs source/architecture resolution before activation.

Exact expectation changes (no assertion removed, loosened, ignored or skipped):

- `test_world_project_v2_to_tree.py`: exact family map with no conversion entry
  → the same exact map plus `"item_conversion": 125`, matching all generated rows.
- `content_world_project_repository.rs`: directory scan budget 231 → **232**,
  exactly the one additional `content/craft/` sibling. Existing digests, byte pins
  and document inventory remain unchanged after regeneration.
- Relocated craft round-trip assertions and Python deterministic count: **113 → 125**,
  accounting only for the 12 completed digging rows. All original assertions survive.
- Generated index/manifest/lock family counts: **113 → 125**, via the authoring tool.

## Provenance, reuse and CI

Canary Lua facts are pinned at `47dfd51f45280a59a1d3e50ba7edd573d7234446`;
digging is `register_actions.lua:401–410,606–625`. XML revision is independently
`04b83b512114bfd888000d6e1433ed8ecaec7c5b`, SHA-256
`1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2`.
Snapshot bytes remain pinned; C1/C4 parity differences remain recorded. No script
or appearance assets were redistributed; source-only Items do not grant admission.

Existing code reused (repository searches preceded new helper/test scaffolding):

- `world_project_v2_to_tree.py`: canonical_bytes/load/write/write_registry and
  materialization; `regenerate_content.py`: registrations and pins.
- `lower_item_consumption_packet.py`: digest-verified items_xml_lines for timers/facts.
- `engine_items.py`: load_appearance_objects for the earlier source capture.
- `reward_claim_authoring.py`: neutral item_ref via the existing craft thin adapter.
- `ReferenceItemSemantics` / TimedDefinitionSource: timed predicate/conflict checks.
- Existing integration core/limits/reference/canonical/rebind_manifest_and_lock:
  moved verbatim to 97-line test-only shared support, with pub(crate) visibility.
- Existing item_conversion_candidate and cached real repository Item records:
  reused by digging tests; no new harness/fixture copy.
- native_entry_first_slice_limits for the existing leaf unit boundary test.

No new externally public function or trait method. Compiler entry points have
production callers and real compiler/parser integration coverage; workspace
consumers use existing variants or explicit fallbacks. No missing caller updates.

CI: `content-tree-migration.yml` / `validate` reaches the craft `content --check`
through `validate_world_project_v2_to_tree.py`, but its path filter omits
`tools/content-schema/craft-authoring/**` and it runs no craft unit tests. The
dedicated `craft-authoring.yml` (same shape as `reward-claim-authoring.yml`) runs
`content --check` and the craft `unittest` suite on any craft source, helper or
registration change. `merge-group-gate.yml` / `rust_linux` / `integration-rest`
discovers the craft Rust target via plan_rust_test_shards.py.

The Python tests use `unittest`, like every other authoring tool in CI; the six
weighted-ceiling sizes are `subTest` cases of one test, so no case was dropped.

## Exact validation results

| Command | Result |
| --- | --- |
| cargo +1.95.0 test --locked -p oteryn-gameplay-content --lib item_conversion --quiet | PASS: 1 passed, 340 filtered; nonzero filter. |
| cargo +1.95.0 test --locked -p oteryn-game-server --test content_world_project_v2 --quiet | PASS: 19 passed, none ignored/filtered. |
| cargo +1.95.0 test --locked -p oteryn-game-server --test content_world_project_v2_craft --quiet | PASS: 7 passed, none ignored/filtered. |
| cargo +1.95.0 test --locked -p oteryn-game-server --test content_world_project_repository --quiet | PASS: 5 passed, none ignored/filtered. |
| python3 -m unittest discover -s tools/content-schema/craft-authoring -p 'test_*.py' -v | PASS: 10 tests (6 ceiling subTests). |
| python3 tools/content-schema/craft-authoring/craft_authoring.py content --check | PASS: 125 rows, deterministic bytes/registration. |
| python3 tools/content-migration/regenerate_content.py | PASS: byte-identical regeneration, no file changed. |
| python3 tools/content-migration/regenerate_content.py --check | PASS: all registered checks. |
| python3 tools/content-schema/item-authoring/item_weapon_proficiency.py --check | PASS: sample up to date. |
| python3 tools/content-census/g4_item_crystal_binding_generator.py --check | PASS: 33971 bindings, 9456726 bytes. |
| cargo +1.95.0 fmt --all --check | PASS. |
| cargo +1.95.0 check --locked -p oteryn-gameplay-content --quiet | PASS. |
| cargo +1.95.0 check --locked -p oteryn-game-server --quiet | PASS. |
| cargo +1.95.0 clippy --locked -p oteryn-gameplay-content --all-targets --quiet -- -D warnings | PASS. |
| cargo +1.95.0 clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings | PASS. |
| python3 tools/agents/validate_governance.py | PASS: 22 policy documents, 9 lanes. |
| python3 -m unittest discover -s tools/agents/tests | PASS: 124 tests. |
| git diff --check | PASS. |

Both pipeline runs passed spell-import **9** tests and tree-migration nested
unittest groups **8+5+28+13+11+6 = 71**, plus 27 RewardClaim function cases,
4 StarterKit cases and the script assertions. Managed files: **231**; materialized
tree: **97/97**, 10 world markers, 1255 successor files. Item-key census: 230 files,
5 history files, 286919 references. All six registered authoring checks passed.
The unchanged vendored Tokio warning does not fail clippy. PG is not applicable.
Run 1e whole-diff review against the owner's CODEX-CHECKLIST found no craft code
defect. All 19 original v2 tests retain their names/order; the five shared helper
bodies match the pre-CRAFT baseline verbatim. No test/assertion was weakened.
Workspace rg rechecked all 16 new types, 23 variants (including the enclosing
ItemConversion registration), 47 public fields and the parent enum consumers.

## File sizes and handback

Python commands ran with the pinned `item-authoring/requirements.txt` set
(jsonschema 4.26.0, rfc3339-validator 0.1.4). No dependency or requirements
change and no preflight or test bypass.

The earlier `regenerate_content.py` Item preflight failure was sandbox-only:
`check_tibiawiki165_historical_context.py` creates its temporary directory in
`root.parent`, which the lane sandbox mounts read-only. Both regenerate commands
pass outside the sandbox, so that helper stays unchanged and out of scope.

Full-task line counts: **base 9f814e72a → final**.

| File | Before → after |
| --- | ---: |
| `apps/game-server/tests/content_world_project_repository.rs` | 2510 → 2511 |
| `apps/game-server/tests/content_world_project_v2.rs` | 1718 → 1629 |
| `apps/game-server/tests/content_world_project_v2_craft.rs` | 0 → 486 |
| `apps/game-server/tests/content_world_project_v2_craft/digging_tests.rs` | 0 → 66 |
| `apps/game-server/tests/support/content_world_project_v2_fixture.rs` | 0 → 97 |
| `content/content.lock.json` | 315 → 316 |
| `content/craft/index.json` | 0 → 1 |
| `content/craft/map-facts.json` | 0 → 1 |
| `content/craft/recipes.json` | 0 → 1 |
| `content/craft/sources.json` | 0 → 573 |
| `content/manifest.json` | 1013 → 1029 |
| `content/project.json` | 39 → 40 |
| `crates/gameplay/content/src/content/project/v2.rs` | 3666 → 3683 |
| `crates/gameplay/content/src/content/project/v2/item_conversion.rs` | 0 → 636 |
| `crates/gameplay/content/src/content/project/v2/item_conversion/digging.rs` | 0 → 40 |
| `crates/gameplay/content/src/content/project/v2/item_conversion_tests.rs` | 0 → 34 |
| `.github/workflows/craft-authoring.yml` | 0 → 56 |
| `docs/agents/tasks/CRAFT-CONTENT-1.md` | 0 → 191 |
| `tools/content-migration/regenerate_content.py` | 373 → 382 |
| `tools/content-migration/test_world_project_v2_to_tree.py` | 275 → 275 |
| `tools/content-migration/validate_world_project_v2_to_tree.py` | 847 → 851 |
| `tools/content-migration/world_project_v2_to_tree.py` | 663 → 666 |
| `tools/content-schema/craft-authoring/craft_authoring.py` | 0 → 229 |
| `tools/content-schema/craft-authoring/digging.py` | 0 → 41 |
| `tools/content-schema/craft-authoring/schema.json` | 0 → 451 |
| `tools/content-schema/craft-authoring/test_craft_authoring.py` | 0 → 100 |
| `tools/content-schema/craft-authoring/test_digging.py` | 0 → 47 |

Full-family files with more than 200 added lines (relative to a9b388fae):

- item_conversion.rs: new 636 lines; L1 authored types/interpretation only, with
  C5 validation and unit tests extracted into named children.
- content_world_project_v2_craft.rs: new 486 lines; dedicated real content
  compiler/parser integration owner, reusing the one shared fixture owner.
- craft_authoring.py: new 229 lines; offline conversion lowering and existing
  generator registration; digging is a named child, no runtime or alternate tree writer.
- schema.json: new 451 lines; closed authored-data schema in its authoring layer.
- content/craft/sources.json: new 573 lines; extracted source/provenance facts,
  not logic. Generated recipe/index/map-fact files use canonical single-line JSON.
