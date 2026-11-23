# Wave 3 — eight content-migration test files, real execution and orphaned failing oracle

This is an **evidence-only** continuation of [draft PR #3430](https://github.com/Oteryn/Oteryn-Game/pull/3430), tracked by [issue #3429](https://github.com/Oteryn/Oteryn-Game/issues/3429). Base of this wave: **exact clean detached checkout** `2952e155ad6f371d1e4b3b351acd57d4757e386e` on the authorized `oteryn-vps` machine (Python 3.14.4). No original checkout, index, producer or branch was modified. The working tree was clean before and after the probes. The contents below describe what was executed; they do not assert green full CI or gameplay runtime.

## What was actually reviewed

All eight complete source files in [semantic_scope.tsv](semantic_scope.tsv) were read, including their **25 literal unittest methods**; every reported source blob SHA is tied to the current audit candidate. Seven files (24 methods) have behaviorally useful source/provenance/custody assertions and passed on this machine. One file (one method) must be **REWRITE**, not RETIRE. These are test-data/producer checks, not proof of actual Game/Platform/PG gameplay.

The selection investigation independently inspected all 78 indexed workflow command bodies in `../ci_lane_inventory.json`, plus the actual `check_inventory()` and its exact allowlist in `tools/content-migration/test_world_project_v2_to_tree.py`, and `test_parallel_checks.py`. **None of these eight filenames appears as an explicit test invocation**. The checked workflow discovery patterns address other named files, not a generic discovery of these eight. An indirect invocation or a separate, unindexed runtime remains **UNKNOWN**; the executed `extend_records` producer is **not** proof that its unittest method ran in CI.

## PROVEN defect — stale corpus-size oracle + missing direct entry point

`tools/content-migration/test_boss_corpse_terminal_reference.py` has one test, `ExistingElderReferenceTests.test_existing_canonical_reference_is_retained_without_second_allocation`. It reads all Item shards and correctly verifies canonical key `oteryn:item.tibia.i43666` at ordinal **25377**, exact uniqueness and preservation after `terminal.extend_records`. Its final hardcoded count asserts **34117**.

The **current committed** `content/items/index.json` has **69 shards** and `record_count=34123`; the 25-test run reached the last assertion and failed **`AssertionError: 34123 != 34117`**. Both the test blob `67192d4e...`, the index blob `9f8d2909...`, and the production `boss_corpse_terminal_reference.py` blob `d03516c0...` are identical between pinned base `8e9f0e1...` and the head used for execution. GitHub's current `main` serves identical test and index blobs, but **we did not execute a main checkout**. This is not introduced by PR #3430.

The file lacks `unittest.main()`: a direct `python3 test_boss_corpse_terminal_reference.py` returned **rc=0 with no stdout/stderr and zero test execution** on the same checkout. `python3 -m unittest -v test_boss_corpse_terminal_reference` does run the actual test and fails. The verified cause of the assertion failure is an obsolete **global catalog cardinality** used to prove a much narrower **no-new-terminal-item** invariant, not an observed duplicate of item 43666.

The `publish()` authoring function in `boss_corpse_terminal_reference.py` also has a historical count guard for 34117/34118. It was **not invoked or modified**; whether its older-snapshot-only contract is still a live production regeneration path requires separate caller and revision analysis. Do not weaken that guard based solely on this test result.

## RED → GREEN scratch proposal (UNAPPLIED)

[terminal_reference_proposal.patch](terminal_reference_proposal.patch) replaces the hardcoded total with a before/after cardinality invariant, checks that the initial total matches the committed index, and adds a direct test entrypoint.

- Original eight-module `python3 -m unittest -v`: **25 tests, 24 PASS, 1 FAIL**, rc=1, 23.225 s.
- A scratch overlay with the count/oracle repair: **25/25 PASS**, rc=0, 23.323 s. The final guard-only addition does not alter unittest module discovery.
- Original direct script: **rc=0, output 0 bytes, 0 tests**.
- Scratch overlay with both the count repair and `unittest.main()`: **1/1 PASS**, rc=0, 0.488 s when invoked directly.
- Scratch checkout source was restored verbatim after each probe; `git status --porcelain` stayed empty at the end. [Machine-readable record](validation.json).

**RECOMMENDATION:** apply the narrow test fix in a separately owned, qualified code batch and connect this valuable test to an actual CI consumer with a new negative selection regression. Verify both the path and the post-change CI job. The patch is retained under evidence and **does not alter root production/test/CI sources** in this PR. Preserve the source/pin and other content-replay checks; do not fix the failure by pinning another arbitrary global total.

## Exact pending queue and review history

The preceding [wave-2 content queue](../wave-2/content_tools_remainingfiles.tsv) has **695** actual data rows: **451** `STATIC_AST_ONLY`, **10** `PARTIALLY_REVIEWED`, **234** `INVENTORY_ONLY`. Ten static-only paths were completely read and reported separately in [issue #3429 comment 7088506426](https://github.com/Oteryn/Oteryn-Game/issues/3429#issuecomment-7088506426), covering 37 literal definitions; this wave reads **eight different** static-only paths with 25 definitions. Removing these 18 **specific paths** from the wave-2 queue leaves **677 rows**: **433** static-only, **10** partial, **234** inventory-only. [Explicit exact-path queue overlay](remaining_queue_overlay.md). This calculation is only for that content-tool queue and is **not** a global completion percentage: the original 130 aggregate groups and all other audit lanes still require their independent ledgers. Some scoped readings may overlap across lanes.

Review history: exactly one native Codex review completed without findings for first code-bearing head `978d40a02e...`; previous evidence-only successor `2952e155ad6f371d1e4b3b351acd57d4757e386e` and this new evidence batch do not warrant a paid second round. Never treat the prior receipt as review of this new SHA.

**UNKNOWN:** any complete live PostgreSQL/Game/Platform/Windows/GPU validation, authoritative execution frequency of these eight test modules in all dynamic CI call paths, the historical rationale for the fixed global 34117 constant, and whether the prototype survives full protected CI after actual adoption. The draft remains unmerged; no CI status-only or faux PASS commit is justified.
