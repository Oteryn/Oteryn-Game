# Content audit checkpoint

Baseline: `245ba7838e58318a2d851857b6c2b2d7c757aa09`, PR #3430, task
`OTV2-20261122-test-ci-value-audit`. This checkpoint records verified test repairs,
source review, and unresolved findings. It does not claim the complete content
audit or a complete regenerated production pipeline has passed.

## Exact source accounting

`accounting.json` reconciles the original 695-path content queue with exact paths.
The historical reports name **59 distinct paths**, not 68: nine wave-7 paths had
already appeared in the initial ten. The correct starting remainder was 636,
not 627. Historical reports were retained unchanged.

This lane fully read another **150 distinct migration test files**, with **1,084
literal test-case identifiers**. There is no overlap with those 59 historical
paths. The resulting union is **209/695 paths**, leaving **486** paths outside
this checkpoint, including 206 quest-authoring paths. `semantic_review.json`
lists every current source path, source hash, read extent, literal case identity,
fixture/oracle analysis, and the limits of its production-consumer review.
Literal counts are inventory metadata; they are not executed-test counts. A
direct import/call of a producer is not a claim that its entire source was read.

## Verified repair and selection results

The actual `test_world_project_v2_to_tree.check_inventory(workflow_checks=True)`
selects each of the following modules once. Its real commands were executed by
`parallel_checks.run_checks(..., jobs=2)`: **68 distinct cases passed** across
**13 modules**. Exact
executed identities, extracted from the execution output, are preserved in
`selected_inventory_case_ids.json`; commands, output, and exit status are in
`selected_inventory_final.json`.

| Executed module identity | Passed cases |
| --- | ---: |
| `test_boss_corpse_terminal_reference` | 1 |
| `test_cake_bridge_reference` | 3 |
| `test_coin_converter_profiles` | 1 |
| `test_creature_level_rating` | 4 |
| `test_forge_sql_admission` | 2 |
| `test_forge_sql_admission_forward` | 1 |
| `test_harvest_item_profiles` | 2 |
| `test_itemhold_q` | 9 |
| `test_monster_reference_level` | 4 |
| `test_official_creature_balance` | 4 |
| `test_qengine_reward_item_profiles` | 25 |
| `test_questpc4_treasure_reward_profiles` | 9 |
| `test_quick_loot_item_profiles` | 3 |
| **Total** | **68** |

The five `test_parallel_checks` runner regressions also passed, including the
exact inventory identities, independent-process overlap, failure continuation,
invalid-plan rejection, and aggregate qualification failure.
The earlier 12-module/43-case run is retained as `selected_inventory_twelve.json`;
`selected_inventory_final.json` is the subsequent complete 13-module run.

| Repair | Independent evidence |
| --- | --- |
| Coin and QuickLoot tests now anchor their finite identity sets independently of producer enumeration. Creature calibration anchors its three actual hunts. | Original empty/reduced enumerations survived; repaired assertions reject them. `oracle_before.json`, `oracle_after.json`, `quick_loot_before.json`, `quick_loot_after.json`. |
| Official creature balance now anchors the exact 33 `(news_id, creature, field)` observations. | Four original cases passed an empty or reduced override list; repaired cases reject empty, missing-first and duplicate mutations. `official_balance_before.json`, `official_balance_after.json`. |
| Boss corpse and Cake direct runners now execute their existing cases. Boss corpse checks current index consistency and preserves its exact canonical identity/ordinal. | Direct invocation went from zero cases to one/three. The stale global count 34117 was replaced by consistency with the current index; identity assertions remain. `oracle_before.json`, `oracle_after.json`. |
| Forge test uses its existing current-migration owner. Harvest compares complete rows after separately owned House/Nail enrichment. | Actual selected suites pass. Harvest empty/singleton producer sets already fail, so no redundant identity guard was added. `harvest_identity_mutants.json`. |
| Monster reference test applies the same existing Primal overlay as production, and isolates every current dispatcher producer before testing `--no-stage`. | Actual four-case module passes. An unconditional staging mutation is rejected. `creature_roundtrip_mismatch.json`, `no_stage_dispatch_probe.json`. |
| Itemhold retains the exact raw shield-charge discrepancy while recognizing the existing qualified successor clears its live hold. Treasure CLI fixture now removes only its exact current cohort from the real ledger. | Both real nine-case modules pass. Five targeted mutations reject restored hold, changed raw charge, missing witness, omitted cohort refresh, or write-before-check-refusal. `stale_expectations_after.json`. |
| QENGINE's isolated synthetic capture packet now supplies the current `custody_promotions` member as an empty list. Existing test assertions are unchanged. | Baseline: 25 cases, 20 pass and five fixture `KeyError`s. Repaired full module: 25/25 pass; actual selected CI caller also executes all 25 successfully. `qengine_fixture_repair_execution.json` records the exact source hash; `qengine_fixture_proposal.json` is the earlier in-memory proof. |

### Direct runner identities

`tools/content-schema/item-authoring/test_engine_items.py` defined 108 test
functions but its actual `main()` selected 107. The omitted identity was
`test_frost_cannon_routes_non_pickupable_blocking_prop`. After adding that exact
call, an instrumented actual-main identity proof finds **108/108**, with no
missing, extra, or duplicate identity. The real restored case passes and its
empty production-route mutation is rejected. The separately executed real CLI
passes **603 checks**, followed by its existing **3 + 3 + 4** unittest cases.
Evidence: `item_runner_before.json`, `item_runner_after.json`, and
`item_runner_execution.json`. The selection recorder is explicitly distinguished
from real execution; no function-count floor substitutes for either.

`test_regenerate_content.py` already had a complete **36/36** manual selection.
Its cohort test had an incomplete boundary: the real Buddel installer escaped
its mocks. A safe temporary-root baseline traps that exact escape. The repair
reuses the existing isolated dispatcher harness and asserts Thaian then Buddel
both run before cohort measurement. Omitted-Buddel and premature-measurement
mutants fail. The script's existing CI inventory command is selected once and
its real execution passes **36 checks plus 4 Item bootstrap tests**.
Evidence: `regenerate_boundary_before.json`, `regenerate_boundary_after.json`,
and reproducible `regenerate_boundary_probe.py`.

## Unresolved findings

These assertions were retained. No production rule, source fixture, content
output, digest, strict validator, or environment skip was changed to obtain a
passing test.

| Finding | Verified boundary and consequence |
| --- | --- |
| M22 antler packet differs from current shared capture. | `qmech_npcfix2_item_profiles.py --check` exits 1 with `M22_ITEM_PROFILE_DRIFT`. Current capture adds global custody promotions 5805/5806, House flags, and taxonomy beyond its committed packet. The production registration path reads that committed packet. Exact differences: `qmech_capture_mismatch.json`. Available history reaches shallow root `3e099164`; later `b878e746` changes unrelated shared cohorts and does not explain the original M22 drift. A product/packet owner must decide the correct finite projection. |
| Cyclopedia native weights disagree with authenticated client facts. | Item 43860: client 200, canonical UNKNOWN, native KNOWN 0. Item 50255: client/canonical 12800, native 128000. One of seven module cases fails. `cyclopedia_native_weight_mismatch.json`. |
| Committed native ranged overlay is incomplete for `oteryn:item.tibia.i40535`. | A real in-memory overlay replay preserves identity sets but adds the missing THROWING/range-3/attack-1 profile to that single row. Registration invokes ranged overlay before the later Galthen owner admits 40535. One of nine cases fails; exact byte regeneration remains enforced. `ranged_native_overlay_mismatch.json`. |

Runtime qualification is also incomplete where a required boundary is absent or
outside this lightweight lane. Two PostgreSQL scripts require the real service;
the SpyRat boundary script requires its external donor C++ input/compiler;
Primal reference repair requires five external donor directories and cache.
The real non-corpse fixture case explicitly skips because its 46-item donor
fixture is absent. Event, Seven-spell, and callback test setup call the complete
native generator; the full power-score regeneration case was likewise deferred.
Writable, Soulpit, Lion and equipment-pin suites exceeded a **55-second bounded
probe**, so their incomplete runs are not passes. The individual commands and
outcomes are retained in `reviewed_batch1_execution.json` through
`reviewed_batch7_execution.json` and `equipment_pin_execution.json`.

## Reproduction and limits

Verified commands used the isolated interpreter
`/workspace/scratch/738970310ba7/content-audit-venv/bin/python`, with the repository's
existing declared dependencies (including `rfc3339-validator==0.1.4`); no dependency
manifest was changed. From the repository root:

```text
<python> docs/agents/evidence/OTV2-20261122-test-ci-value-audit/completion/content/selected_inventory_probe.py
<python> docs/agents/evidence/OTV2-20261122-test-ci-value-audit/completion/content/regenerate_boundary_probe.py
<python> tools/content-schema/item-authoring/test_engine_items.py
```

All 16 source files changed by this lane passed `git diff --check`; their exact
hashes are recorded in `changed_source_hashes.json`. The exact
commands/results above apply to the current working changes on the stated
baseline, not to an untested future commit. Quest-authoring source review
continues separately; unvisited paths remain explicit in `accounting.json`.
The subsequent read-only quest ledger currently names another 156 distinct paths
and 819 literal case IDs in `quest_semantic_review.json`; it is separate from the
150-path migration checkpoint accounting above. Ten confirmed future oracle
proposals (empty Taming input, a substituted standalone chest quest identity,
the reward-capture test's empty candidate domain, a missing Emperor flag,
empty outfit/grant output, empty QREADY1 binding capture, dropped M35 proof cases, missing Thais plate identities, a truncated Galthen completion plan, and a truncated completion attachment)
remain unapplied in `quest_taming_oracle_proposal.json`,
`quest_chest_oracle_proposal.json`, `quest_reward_capture_oracle_proposal.json`,
`quest_emperor_oracle_proposal.json`, `quest_outfit_oracle_proposal.json`,
`quest_qready_capture_oracle_proposal.json`, `quest_m35_rebind_oracle_proposal.json`, `quest_thais_effect_oracle_proposal.json`,
`quest_galthen_plan_oracle_proposal.json`, and `quest_completion_attach_oracle_proposal.json`.
The subsequent 12-module read-only check in `quest_batch_96_107_execution.json`
records 74 passing cases, one existing optional donor-fixture skip, and one
missing historical worktree error; it is not a fully passing batch.
`quest_batch_108_119_execution.json` records another 11 modules / 79 passing
cases. Its chosen-progress module uses the declared environment parameters
with the existing exact-hash main-state fixture; QREADY source bindings require
explicit unittest selection because direct invocation runs no cases.
`quest_dialogue_held_path_gap.json` records two held-path-only methods whose
current fixture domain is empty; those passing methods execute no assertion.

An independent read-only review of the BB02/S3B deltas and their actual callers
is recorded in `peer_blackbox_review.json`, including passing 33-case matrix,
9-case prebuilt, and focused actual PR/MQ aggregate checks. Its approval is
limited to the documented offline contracts.

`quest_batch_120_132_execution.json` records thirteen additional modules with 113 passing cases. The source-fix unmutated control passed with and without an explicit root; no false setup rejection was inferred. The bounded Thais and Galthen plan mutations above survived their real module assertions and remain unapplied test improvements.

`quest_batch_133_143_execution.json` records eleven additional modules /99 passing cases through explicit unittest selection. `quest_direct_entry_gaps_135_143.json` distinguishes the two direct-entry omissions from those executed cases. The completion attachment surviving mutant is limited to the one preservation method; the full unmutated nine-case module passes.

`quest_batch_144_155_execution.json` records twelve additional modules /100 case outcomes:98PASS and two existing beside-test donor packet skips. The corresponding committed Other-component packet exists at samples/donor-source/components248/other.json.gz; `quest_other_component_fixture_proposal.json` proves both original skipped cases pass against those exact bytes/schema in26.702s with no source change.

### Quest batch 156–167

Full test bodies and bounded producer/schema/consumer contracts for another 12 paths were read. All 12 actual modules passed, with 87 cases and no skips. Exact execution is in `quest_batch_156_167_execution.json`. The quest ledger now has **168 distinct paths / 906 literal case identities**; the content union is **377/695**, leaving **318 paths** (38 quest paths plus 280 other content paths). These remain source-review counts, not complete runtime or CI coverage.

### Quest batch 168–179

The next 12 full test files and bounded consumers are recorded in `quest_semantic_review.json`. Actual execution: **102 case outcomes = 97 PASS + 5 existing external-manifest SKIP**; see `quest_batch_168_179_execution.json`. Quest totals are **180 distinct paths / 1,008 literal IDs**; content union **389/695**, leaving **306 paths** (26 quest + 280 other content). Four further direct entry gaps are reproduced in `quest_direct_entry_gaps_173_178.json`; their explicit unittest suites pass. `quest_collect_placement_path_gap.json` records one targeted method passing with zero assertions because its actual45multi-target rows contain no cover rows. Source edits remain paused.

### Quest batch 180–191

Another 12 full sources and bounded consumers are recorded. Nine complete modules passed **94 cases**; three modules exceeded the50second bound (`joins_next/test_builder.py`, `test_quest_donor_attachment.py`, `test_quest_completion_import.py`) and are not reported as passing. The joins_next synthetic ParserControls separately passed11/11. Quest totals: **192 paths / 1,137 literal IDs**. Content union: **401/695**, leaving **294 paths** (14quest +280othercontent). New evidence records the seven-ID Itemhold qualification vacuity and one additional direct-run gap. `regenerate_merge_safe_proposal.json` separately records the one-line mock decorator proposal, actual36+4PASS and clean exact-base merge with current main; no source edits made by this lane.

## Quest continuation: queue192–201

Reviewed every body in10additional test files plus bounded producers/fixtures/consumers. Quest ledger now **202files/1326literal IDs**; inherited695queue union **411/695**, with **284remaining**. Actual completed8modules executed141casesPASS; bundle_authoring and quest_tree_authoring reached50second bounds, with partial results retained. No fullPASS claim for timed-out modules. See quest_batch_192_197_execution.json and quest_batch_198_201_execution.json. Fullbaseline content inventory is709; separate inventory_scope_reconciliation.json accounts for14historically excluded paths and24new content-census/format candidates.
