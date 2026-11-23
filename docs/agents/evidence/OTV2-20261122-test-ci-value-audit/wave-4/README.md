# Wave 4 — portable content tests, missing historical fixture, and real cake-provider execution

This is a substantive evidence-only continuation of [draft PR #3430](https://github.com/Oteryn/Oteryn-Game/pull/3430) / [issue #3429](https://github.com/Oteryn/Oteryn-Game/issues/3429). The root explicitly reopened **AUTHORING** after the earlier freeze to preserve this verified read-only audit batch. Source baseline is a **new, clean, detached worktree** at exact head `2749acb3295b536f0b88dc04c25fe0f6afcbd526` on authorized `oteryn-vps`, Python 3.14.4. The earlier staged checkouts were not reset or edited.

## Fully read scope and meaningful oracles

Ten complete test source files (listed with exact blob hashes in [semantic_scope.tsv](semantic_scope.tsv)) contain **50 literal unittest methods**. Each file was read in full and classified by its production/authoring consumer, fixture, independent oracle, unique failure and disposition. **Nine are KEEP, one is REWRITE.** They cover native item custody, Cake source provenance and composed loot, corpus integrity, JSON round-trip, explicit source-evidence qualification, immutable population merge, and refusal to erase unresolved semantics. A test's path or historical name alone is not evidence of obsolescence.

The ten exact filenames have **zero literal matches** in all 78 committed `.github/workflows/*.yml`/YAML bodies at this head. Their relevant producers/callers were searched separately: `cake_bridge_reference` is used by the world projector and item-reference validator; `cake_native_successor` is used by current monster composition and spell registration; `compact_record_json` has several current source producer consumers. `complete_captured_loot_alias_delta` has no other Python caller found under the searched `tools/` and `.github/` paths, and its producer references historical absolute snapshots. **UNKNOWN:** indirect dynamic invocations, full execution frequency and any unindexed external runner. Zero literal workflow matches is not proof that a test never executes.

## Real test execution, not static counts

Commands were run with `cwd=tools/content-migration` on the clean detached worktree:

- `python3 -m unittest -v <all ten modules>`: **50 tests, 48 PASS + 2 ERROR**, exit 1, 84.408 seconds. The errors are independently diagnosed below; there were **no assertion failures** in this run.
- Five focused modules (`test_classify_monster_population`, `test_compact_record_json`, `test_complete_creature_dependencies`, `test_complete_familiar_population`, `test_complete_loot_quality_fields`): **29/29 PASS**, exit 0, 0.070 seconds.
- `python3 -m unittest -v test_complete_captured_loot_alias_delta`: **5 PASS, 1 ERROR**, exit 1. The error is the same historical-fixture omission.
- `python3 test_cake_bridge_reference.py`: exit 0 with **zero output and zero tests executed**; importing it through `unittest` instead executes **3/3 PASS**.
- The Cake generated-provider test was rerun with a **real, installed, requirements-pinned** `zstandard==0.25.0` interpreter (read-only existing Python 3.14 venv), without any stubs: `python3 -m unittest -v test_cake_native_successor.CakeNativeSuccessorTests.test_actual_generated_provider_contains_complete_canonical_cake`: **1/1 PASS**, exit 0, **170.596 seconds**. This is an independent replay of only the previously dependency-blocked test, **not** a complete rerun of all 50 in one interpreter.

Thus **49 distinct methods have an observed PASS across these commands** and one historical-file dependent method is still unqualified. Do not call the 50-test slice fully green. The existing source worktree remained clean; no original test/producer/CI code or fixtures were changed.

## PROVEN finding W4-01 — historical absolute source makes a test non-portable

`tools/content-migration/test_complete_captured_loot_alias_delta.py:32` unconditionally opens `/workspace/monster-field-next-20261002/loot-alias-delta-final/field-patches.json`. That path does **not** exist on the authorized VPS. The exact test fails with `FileNotFoundError` before evaluating the promised historical `294`-comparison and actor-flag assertions. The production authoring CLI `complete_captured_loot_alias_delta.py:build` also depends on several fixed `/workspace/monster-field-*` and `/workspace/monster-round7-output` captures; no checked-in replacement evidence was established. Its five portable source-corroboration/mutation checks **PASS** and should stay.

**REWRITE, not RETIRE:** split the portable checks from the frozen historical-packet assertion. Supply the real snapshot as an explicit, version/digest-verified opt-in fixture or restore it as an owned repository artifact if provenance permits. A missing artifact must be an explicit missing qualification, not a fake PASS. Do not fabricate the 294-row golden or silently skip its assertions. Verify this test's actual CI selection before wiring it into a required lane.

## PROVEN finding W4-02 — direct-entrypoint silent success

`tools/content-migration/test_cake_bridge_reference.py` defines three useful refusal/idempotence tests but has **no `unittest.main()` guard**. Running it as a script returns rc=0 and executes none; `-m unittest` executes all three. A direct script success is not evidence of these checks. **KEEP** the cases; make the intended CI discovery/entrypoint explicit, with a negative selection regression. Adding a main guard is safe only together with verified CI wiring, not as a substitute for CI discovery.

## Environment and CI facts

The initial Cake-provider error is **`ModuleNotFoundError: zstandard`** via `register_spell_families.outputs → quest_callback_profiles → world_region_codec`, not a failed Cake content assertion. The version `0.25.0` comes from `tools/content-schema/world-authoring/requirements.txt`, and the actual pinned interpreter rerun passed. The composition took 170.596 seconds even in this warmed worktree. Treat its cost as measured for this one machine/call, **not** a whole-CI cost projection.

On PR head `2749acb3295b536f0b88dc04c25fe0f6afcbd526`, `PR PostgreSQL harness` heavy-gate returned success **after deliberately skipping the PostgreSQL archive, legacy, and node-boot jobs** (`IS_ELIGIBLE=false`, `IS_HEAVY=false`). `Merge gate` had Rust jobs in progress when read. Neither is proof of live PostgreSQL/Game/Platform qualification. The previously proposed BB-02 exact indexed-row completeness patch remains **unapplied**; it still needs representative live output/capture-only/no-op qualification before adoption. No second Codex review request was sent; the only completed provider review covers original code-bearing head `978d40a02e...` exclusively.

## Remaining queue and boundaries

The precise [content-tool queue delta](remaining_queue_overlay.md) reduces **677 → 667** not-yet-fully-read content-tool paths: **423 STATIC_AST_ONLY + 10 PARTIALLY_REVIEWED + 234 INVENTORY_ONLY**. This is a subtraction of **these ten exact paths only** from the previous verified queue, with no claim of global completion. The original separate 130 aggregate groups and all other lanes remain outside this arithmetic. The one REWRITE path is read, **not a qualified PASS or retired test**.

**UNKNOWN:** historical capture custody and portable relocation, full dynamic CI test selection, complete head CI including PG/Game/Platform/Windows/GPU, and whether the BB-02 patch correctly handles real capture-only/no-op rows. No source code changes, no merge, no auto-merge, no weakened tests, and no invented PASS.
