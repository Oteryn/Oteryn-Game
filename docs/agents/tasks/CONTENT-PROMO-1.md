# CONTENT-PROMO-1 — extract offline content promotion

```yaml
task_id: CONTENT-PROMO-1
mode: IMPLEMENT
status: local_validation_incomplete
repository: Oteryn/Oteryn-Game
pr: null
publication: uncommitted (owner instruction)
base_sha: 6d090defd24f72d9bc6bfd3f9273fb2f758b189d
branch: detached worktree
frozen_sha: null
```

## Summary

Move offline item promotion and protected import adapters from
`oteryn-gameplay-content` into `oteryn-gameplay-content-promotion`. Preserve
adapter bodies, inline tests, exact embedded packet bytes, runtime behavior,
and the public runtime family-count path. No new public functions, enum
variants, struct fields or trait methods are introduced.

## Reason

STRUCTURE-TARGET-1 measured a clean dev server build at 79.42 s on this PC:
`content` took 24.87 s (frontend approximately 18.7 s), followed by entities,
durability, dispatch and game-server on the critical path. Average parallelism
was 3 of 8 slots; 17 workspace crates depend on `content`. The proposed split
isolates example/test-only code and embedded multi-megabyte packets from that
runtime dependency path. The owner permits fragmentation only for a measured
compile-time, merge-conflict or ownership benefit.

Before editing, `rg` searched all candidate module names and their public item
names throughout `crates/`, `apps/` (including every application `src/`) and
`tools/`. Runtime use is exactly `npc_catalogue.rs:53` consuming the full-family
count. The `item_identity` SQL column and unrelated methods in durability,
world and reference artifacts, and session's local `item_admission` test helper,
are name collisions rather than uses of the moved modules. A second search
found Python tools reading the relocated Rust sources; their paths are updated.

| Candidate | Original lines | Non-test, non-example users |
| --- | ---: | --- |
| `cw2_b1_import` | 3325 | `CW2_B1_FULL_ITEM_FAMILY_COUNT` only; retained in `content` |
| `cw2_b4_import` | 464 | None |
| `item_admission` | 1297 | None |
| `item_bed_promotion` | 388 | None |
| `item_capacity_promotion` | 162 | None |
| `item_consumption_promotion` | 200 | None |
| `item_description_promotion` | 253 | None |
| `item_description_wiki_promotion` | 20 | None |
| `item_document_promotion` | 323 | None |
| `item_elemental_magic_modifier_promotion` | 109 | None |
| `item_forge289_promotion` | 326 | None |
| `item_forge3332_promotion` | 197 | None |
| `item_hit_magic_promotion` | 389 | None |
| `item_identity` | 612 | None |
| `item_mantra_bond_modifier_promotion` | 109 | None |
| `item_market_true_promotion` | 218 | None |
| `item_movable_promotion` | 30 | None |
| `item_name15_promotion` | 269 | None |
| `item_name_promotion` | 272 | None |
| `item_numeric_modifier_promotion` | 365 | None |
| `item_physical_promotion` | 363 | None |
| `item_ranged_promotion` | 468 | None |
| `item_stack_default_promotion` | 84 | None |
| `item_stack_default_successor8_promotion` | 233 | None |
| `item_stack_false_promotion` | 196 | None |
| `item_stack_historical_promotion` | 30 | None |
| `item_stats_promotion` | 1550 | None |
| `item_timed_promotion` | 705 | None |
| `item_use_observation_promotion` | 292 | None |
| `item_weapon_metadata_promotion` | 362 | None |

## Scope

- Relocate 30 modules (13,611 lines) into
  `crates/gameplay/content-promotion/src/content/`. Keep the directory depth,
  so all `include_bytes!` / `include_str!` paths resolve the identical evidence
  and import packets without duplicating or rewriting those files.
- Retain `CW2_B1_FULL_ITEM_FAMILY_COUNT = 38_157` in the small `item_family.rs`
  module, re-exported through the existing `content` path. The offline adapter
  imports it. The numeric-modifier adapter imports the already-public
  `validate_item_definition` re-export instead of its former private module.
- Update both consuming game-server examples, six integration test files, and
  `world_runtime_tests` through dev dependencies only. Register the new member
  as a tool with an L2 dependency on L1 content in `workspace-boundaries.toml`.
- Content's private artifact unit tests retain test-only path mounts of the
  relocated CW2-B1 and item-identity sources: these tests need adapters compiled
  against the unit-test crate's own nominal types. This avoids a circular dev
  dependency and type mismatch without changing fixtures or exposing internals.
  Their inline tests remain intact and also run in the new crate. Neither mount
  exists in production builds.
- Update the source-relocation ledger's current item-identity path, preserving
  its exact digest and all historical/generated pins; update the census and
  alias-table source readers and retired-key allowlist.

Owned paths: root Cargo manifests and boundaries, `crates/gameplay/content/`,
`crates/gameplay/content-promotion/`, world's manifest and test imports,
game-server's manifest/example/test imports, the seven source-reader/reference
Python files listed in the diff, `docs/agents/tasks/MECHANICS_STATUS.md` (repair its relocated-source link),
and this task card. Ignored local evidence is
in `target/contentpromo/`; the requested fresh timing output is under
`/home/mole/oteryn-cache/target-codex/contentpromo-clean-after/`.

## Excluded scope

No gameplay, persistence, PostgreSQL, protocol, identity, admission or authority
semantics change. No content regeneration, evidence-packet edits, third-party
dependencies, Git mutations, publication, credentials or external writes.
`Cargo.lock` adds only the new workspace package and its two consumer edges;
all third-party entries are unchanged. Protected workflow and
`tools/repository/` files are not edited.

## Validation

All checks use locked offline dependencies and the supplied absolute Cargo
wrapper; rustfmt uses `/home/mole/.cargo/bin/cargo fmt` for the four touched
crates. No PostgreSQL code is touched and no PostgreSQL suite is run.

- Move proof: compare all relocated sources against `git show HEAD:<old path>`;
  all 30 match after only the two import adjustments and rustfmt whitespace.
  All original inline tests and embedded include paths remain present.
- `cargo check --locked --offline -p oteryn-gameplay-content-promotion --all-targets --quiet`: PASS.
- `cargo test --locked --offline -p oteryn-gameplay-content-promotion --quiet`: PASS, 106 tests.
- `cargo test --locked --offline -p oteryn-gameplay-content --lib --quiet`: PASS,
  247 passed, 1 pre-existing ignored test; no new ignores or exclusions.
- `cargo clippy --locked --offline -p oteryn-gameplay-content-promotion -p oteryn-gameplay-content -p oteryn-gameplay-world -p oteryn-game-server --all-targets --quiet -- -D warnings`: PASS.
  Only existing vendored Tokio/sqlx dependency warnings are emitted.
- `cargo fmt -p oteryn-gameplay-content-promotion -p oteryn-gameplay-content -p oteryn-gameplay-world -p oteryn-game-server -- --check`: PASS.
- Lockfile comparison against HEAD: only the new workspace package plus its
  two dev-consumer edges change; every third-party package record is identical.
- All 45 embedded include paths resolve to the exact original packet files.
- `python3 tools/agents/validate_governance.py`: PASS (including repaired
  relocated-source link in `MECHANICS_STATUS.md`).
- `python3 -m unittest discover -s tools/agents/tests`: PASS, 129 tests.
- `python3 -m unittest discover -s tools/content-migration -p test_source_relocations.py`: PASS, 3 tests.
- `python3 -m unittest discover -s tools/content-migration -p test_item_official_navigation.py`: PASS, 6 tests.
- `python3 tools/content-census/item_id_alias_table_self_test.py`: PASS.
- `python3 tools/content-census/g4_item_crystal_binding_generator.py --check`: PASS.
- `python3 tools/content-census/item_key_references.py`: PASS.
- `cargo run --locked --offline --quiet -p oteryn-architecture-check -- workspace .`: PASS.
- `cargo test --locked --offline --quiet -p oteryn-game-server --test content_item_identity --test content_reference_artifact --test content_world_cw2_b1_import --test content_world_cw2_b1_promotion_lowering --test content_world_project_repository`: PASS,
  47 tests across the five targets (14 / 9 / 16 / 3 / 5).
- `cargo test --locked --offline --quiet -p oteryn-game-server --test integration_rest -- content_world_cw2_b4_import`: PASS,
  4 tests; unrelated target cases are outside this change's filter.
- Materializer end-to-end: export the exact locally available
  `regenerate_content.py::PREDECESSOR_PIN` documents using read-only `git show`,
  then `cargo run --locked --offline --quiet -p oteryn-game-server --example materialize_content_world_project_v2 -- --output-root target/contentpromo/materialized-after --predecessor-root target/contentpromo/predecessor`:
  PASS. The resulting file set equals the registered 11 document locators;
  every output byte matches `content/world/`. No fetch or regeneration of
  tracked content occurred.
- `cargo test --locked --offline --quiet -p oteryn-gameplay-world --lib`: PASS,
  188 passed, 1 pre-existing ignored test.
- `cargo build --locked --offline --quiet -p oteryn-game-server --example materialize_content_world_project_v2`: PASS.
- `cargo test --locked --offline --quiet -p oteryn-game-server --example materialize_content_world_project_v2`:
  78 passed, 8 failed in unchanged NPC fixture capture before promotion logic.
  Every failure is `LimitExceeded { resource: "total directory entries scanned",
  actual: 202, limit: 201 }`. The unchanged fixture helpers hard-code 201;
  `git diff` / status show no changes to `content/` or `npc_materializer/`.
  Therefore this is an existing fixture/tree-bound mismatch, independent of
  relocating imports (DERIVED). Tests and bounds remain unchanged.
- Final clean build measurement, without concurrent local Cargo checks: PASS.
- Whole-diff review against `/home/mole/oteryn-cache/CODEX-CHECKLIST.md`: PASS
  for this extraction. All relocated tokens match after the two necessary
  import changes; no tests/assertions were removed or weakened. No added enum
  variants, struct fields, trait methods or public functions require caller
  updates beyond the symbol-user sweep and import changes already recorded.
- `git diff --check`: PASS.

Clean build measurements (requested dev build, separate fresh target directories):

```sh
/home/mole/oteryn-cache/codex-bin/cargo build -p oteryn-game-server --timings --locked --offline --target-dir /home/mole/oteryn-cache/target-codex/contentpromo-clean-after
/home/mole/oteryn-cache/codex-bin/cargo build -p oteryn-game-server --timings --locked --offline --target-dir /home/mole/oteryn-cache/target-codex/contentpromo-clean-final
```

| Measurement | Supplied before | First run (overlapping local validation) | Final run (no other local Cargo jobs) |
| --- | ---: | ---: | ---: |
| Content frontend | approximately 18.7 s | 29.23 s | 13.86 s |
| Content codegen | approximately 6.17 s (derived) | 15.31 s | 7.07 s |
| Content total | 24.87 s | 44.54 s | 20.93 s |
| Whole clean server build | 79.42 s | 99.7 s | 81.6 s |

The final run measures a 15.8% reduction in content total time and approximately
25.9% in its frontend time. It demonstrates the component compile-time benefit
for this split. Whole-build improvement was not demonstrated: 81.6 s is slower
than the supplied 79.42 s baseline. The first run overlapped other local
qualification; the final run avoids that source of interference. Other runs on
this shared PC remain an uncontrolled factor, so these are observations rather
than a controlled causal benchmark. Both timing reports confirm the promotion
crate is absent from the server's production build.

Reports:

- `/home/mole/oteryn-cache/target-codex/contentpromo-clean-after/cargo-timings/cargo-timing-20261024T102406222Z-189369e01629df27.html`
- `/home/mole/oteryn-cache/target-codex/contentpromo-clean-final/cargo-timings/cargo-timing-20261024T103934352Z-189369e01629df27.html`

## Unfinished

- Whole-build improvement was not demonstrated; content itself measured faster
  in the final run, as recorded above.
- Eight unchanged materializer NPC fixture tests fail with the 201-entry bound
  described above. Full names (same command/filter selects all of them):
  - `npc_bulk_enrichment::tests::enrichment_cannot_activate_services_or_change_actor_identity`
  - `npc_bulk_enrichment::tests::enrichment_preserves_disabled_features_and_rejects_duplicate_replacements`
  - `npc_bulk_enrichment::tests::packet_and_complete_predecessor_fences_are_atomic`
  - `npc_bulk_enrichment::tests::r22_successor_cannot_change_matching_or_add_conversation_flow`
  - `npc_bulk_enrichment::tests::r23_project_defaults_cannot_override_donor_or_foreign_presentations`
  - `npc_bulk_provisional::tests::duplicate_source_binding_fails_late_native_validation_atomically`
  - `npc_bulk_provisional::tests::injected_service_reference_rejects_without_mutation`
  - `npc_bulk_provisional::tests::packet_and_predecessor_fences_reject_without_mutation`
- `tools/repository/classify_pr_test_lanes.py` has a content-specific routing
  prefix for the old content directory, which should also cover the new
  `crates/gameplay/content-promotion/src/content/` directory in a separately
  authorized policy change. This file is owner-excluded. Existing generic
  gameplay routing and the manifests still select Rust qualification here.
- Optional Python lowering-packet tests could not load because the environment
  lacks `rfc3339-validator`; no dependency was downloaded.
- Publication remains excluded by the owner; all changes stay uncommitted.

## Changed files

The 30 module rows above each map
`crates/gameplay/content/src/content/<module>.rs` to
`crates/gameplay/content-promotion/src/content/<module>.rs`. Additional files:

- `Cargo.lock`
- `Cargo.toml`
- `apps/game-server/Cargo.toml`
- `apps/game-server/examples/export_reference_item_identity_map.rs`
- `apps/game-server/examples/materialize_content_world_project_v2.rs`
- `apps/game-server/tests/content_item_identity.rs`
- `apps/game-server/tests/content_reference_artifact.rs`
- `apps/game-server/tests/content_world_cw2_b1_import.rs`
- `apps/game-server/tests/content_world_cw2_b1_promotion_lowering.rs`
- `apps/game-server/tests/content_world_project_repository.rs`
- `apps/game-server/tests/integration_rest/content_world_cw2_b4_import.rs`
- `crates/gameplay/content/src/content/mod.rs`
- `crates/gameplay/world/Cargo.toml`
- `crates/gameplay/world/src/world_runtime_tests.rs`
- `docs/agents/tasks/MECHANICS_STATUS.md`
- `tools/content-census/g4_item_crystal_binding_generator.py`
- `tools/content-census/item_id_alias_table_self_test.py`
- `tools/content-census/item_key_references.py`
- `tools/content-migration/source_relocations.py`
- `tools/content-schema/item-authoring/donor_census.py`
- `tools/content-schema/item-authoring/lower_promotion_packet.py`
- `tools/content-schema/item-authoring/test_lower_promotion_packet.py`
- `workspace-boundaries.toml`
- `crates/gameplay/content/src/content/item_family.rs`
- `crates/gameplay/content-promotion/Cargo.toml`
- `crates/gameplay/content-promotion/src/lib.rs`
- `crates/gameplay/content-promotion/src/content/mod.rs`
- `docs/agents/tasks/CONTENT-PROMO-1.md`
