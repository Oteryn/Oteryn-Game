> Retained lane report at the pinned baseline. The root [audit status](README.md) records which proposals were applied and the latest validation. Historical scratch overlays and proposal patches are evidence, not additional production changes.

# CI and repository-test value audit

Source: `Oteryn/Oteryn-Game@8e9f0e16631aab1beb8416f8d5ca707aa3031083` (main as observed on 2026-11-22). Read-only lane; candidate edits are separate scratch patches and have not been applied to the shared repository. **Scope is structural inventory of every committed workflow and repository-tool test module, plus focused semantic investigations. It is not a claim that every test case was executed or fully reviewed.**

## Verified basis

- 78 committed workflow YAML files, 143 job definitions. Matrix expansion and reusable-workflow invocations are not included in the job count.
- Live GitHub workflow registration, paginated at 2026-11-22 22:12:43 UTC: **44 active, 34 disabled_manually** among those 78 files. GitHub listed 134 workflow registrations total; 56 paths are absent at this source revision and must not be counted as current workflows.
- 52 files under `tools/repository/`, including 32 `test_*.py` entrypoint modules. Static AST finds 336 functions/methods named `test_*`; this is not the runtime test count because several scripts use custom `main()` assertions and subcases.
- PR [#3391](https://github.com/Oteryn/Oteryn-Game/pull/3391) is **merged**, not pending: merge time 2026-11-22 21:38:12 UTC, squash `8e9f0e16631aab1beb8416f8d5ca707aa3031083`, PR head `a4de225e3c8956c2d3d7ef20a5ebc5a982e94773`. Its cache-only `--lib --no-run`, PG scheduling, Windows fail-fast and shared-dispatch restore fixes are already in the audited source.
- `codeql.yml` and the post-merge CodeQL job are **not duplicated on main push**: `.github/workflows/merge-group-gate.yml:349-352` restricts its CodeQL copy to `merge_group`; `codeql.yml` covers push to main.

## Findings and concrete proposals

### CI-01 — confirmed redundant repository-test invocations (P2; CHANGE)

`.github/workflows/merge-gate.yml:397-417` explicitly invokes two entrypoints and then loops over every other `tools/repository/test_*.py`. However:

1. `tools/repository/test_validate_pr_gate_pg_sim.py:665-676` additionally invokes `test_validate_merge_group_pg_sim.py`, `test_classify_pr_test_lanes.py`, and `test_classify_content_routing.py`. The outer loop later runs all three again.
2. `tools/repository/test_validate_pr_gate_pg_sim.py:623-632` explicitly runs `ArchiveShard.test_test_failures_complete_but_runner_errors_cannot_reuse_a_stale_report`, although the loop also runs its owning `test_run_rust_fast_archive.py`.
3. `tools/repository/test_agent_governance_pr_targets.py:11-12` imports two TestCase classes, so `unittest.main()` collects 5 own cases plus 10 main-applicability and 1 merge-authority case. The loop subsequently runs those 11 cases from their owner modules again.

The last wrapper is also used alone by `.github/workflows/agent-governance.yml:211-212`. Removing imports without updating this caller would remove its applicability checks. Patch B therefore explicitly runs the same three owning modules there. No test body, assertion, expected output, game gate or protection file changes.

**Patch A:** `ci_dedup_A.patch`, remove redundant cascading sibling entrypoints and the nested stale-JUnit invocation. Every owner entrypoint remains in the existing merge-gate loop.

**Patch B:** `ci_dedup_B.patch`, remove imported TestCase classes from the metadata module, and make Agent governance run the same three modules explicitly.

**Evidence:** static invocation audit finds 35 module invocations becoming 32 with the same 32 unique module entrypoints; `merge-gate.yml` bytes are unchanged. Actual execution of five affected unittest modules, including nested cases, finds **54 executions before / 42 after; exactly the same 42 normalized unique case IDs; no missing/added cases; zero failures, errors or skips in either run**. The standalone Agent governance caller retains its same 16 unique cases (5 + 10 + 1). One sequential local sample was 10.513 seconds before and 7.182 seconds after. This is a small focused-slice measurement, not a whole-CI or monetary saving forecast.

The three unchanged custom-`main()` siblings are preserved by dispatch/source evidence but were not executed in this focused comparison. Full governance validation additionally needs genuine PowerShell (`pwsh`) for `_powershell_native_canaries`; the local shell did not have it. Do not replace those canaries with stubs or report full governance green from the focused probe.

### CI-02 — test tied to presentation labels (P2; REWRITE, KEEP PURPOSE)

`tools/repository/test_protected_main_cache.py:131-156` chooses build steps by their human-readable `name` and asserts there are five. In a temporary copy, changing only the display label `Build workspace` to `Compile workspace` makes `test_linker_matches_the_archive_producer_without_adding_cargo_flags` fail with `AssertionError: 4 != 5`; all executable commands, env and ordering stay byte-identical. Baseline passes; label-only mutation fails. This confirms a false-positive maintenance cost rather than a product regression.

Recommendation: locate meaningful build operations by parsed command/step identity and assert the linker environment and ordering on those operations; retain checksum rejection and real shell failure-propagation tests. Do not discard the entire cache test module: its actual shell probes catch corrupted archives and publication after failure.

Evidence: `ci_snapshot/label_mutation_probe.json`; baseline 1 pass, temporary label-only variant 1 expected failure. No production source mutation.

### CI-03 — 34 disabled definitions are not present-day CI spend (P2; ARCHIVE CANDIDATES, NO IMMEDIATE REMOVAL)

The apparent G4/package duplication is already disabled operationally. `g4-canonical-worldproject-package-seed.yml` is `disabled_manually`; its last returned run is [39556098799](https://github.com/Oteryn/Oteryn-Game/actions/runs/39556098799), dated 2026-10-21. `world-project-package.yml` is active and extends the predecessor pipeline with native canonical completion, generated World pins and additional NPC cases. The former is an archive candidate only after checking all policy/evidence references. Deleting it now cannot be credited with current runner savings.

The same distinction applies to disabled capture/census/Atlas and WP5 workflows. Their exact states are listed below. `synology-game-deploy.yml` and runner acceptance are disabled operations, not evidence that the underlying test semantics are obsolete. None was enabled, disabled or removed during this lane.

### CI-04 — weak inventory floor (P2; REPLACE ASSERTION WHEN RUNNER IS REWORKED)

`tools/repository/test_repository_tests_run.py:31-34` asserts that there are at least 18 repository test files. There are 32 at this source revision. That numeric floor does not establish that all required tests execute and can stay true after deleting up to 14 files. The useful checks in the same module inspect loop coverage, explicit skip handling and failure propagation. Retain that purpose; use the actual unique entrypoint/case inventory when improving the runner, instead of an arbitrary floor. This assertion is cheap; no performance benefit is claimed.

### CI-05 — active nightly lane has a health issue, not a proven retirement case (KEEP; REPAIR WITH E2E OWNER)

`node-boot-real-clock-nightly.yml` is active and sets `OTERYN_NODE_BOOT_REAL_CLOCK=1`, so it is semantically distinct from deterministic fake-clock smoke. The last three returned scheduled runs concluded `failure`: [45009122985](https://github.com/Oteryn/Oteryn-Game/actions/runs/45009122985), [44891407829](https://github.com/Oteryn/Oteryn-Game/actions/runs/44891407829), [44715436566](https://github.com/Oteryn/Oteryn-Game/actions/runs/44715436566). Run start/update timestamps are not runner-duration measurements and no cost claim is made. Repair/classify the failing verdicts with the existing E2E owner; failure itself is not a reason to delete the lane.

## Black-box direction for CI infrastructure

Use behavioral shell/CLI fixtures for command failure propagation, output/result aggregation, stale artifact rejection, empty-selection rejection, runner/image identity and checksum checks. Existing examples in `test_protected_main_cache.py`, `test_shared_dispatch_archive.py`, `test_collect_rust_fast_results.py`, and `test_run_rust_fast_archive.py` have that value. Migrate label/whitespace/string-count checks to structural or behavioral assertions incrementally. Keep necessary policy invariants (exact head, read-only/protected publication, fail-closed results) and do not replace deterministic safety checks wholesale with slow game E2E.

## Complete workflow structural inventory

`KEEP/REVIEW` means the structural route was inspected but no evidence supports removing the workflow; it is not a certification that every assertion is necessary. `ARCHIVE?` requires consumer/policy/reference proof. `OPERATION` is not a product-test lane.

| Workflow | Live state | Triggers | Job definitions | Triage |
|---|---|---|---:|---|
| `.github/workflows/achievement-authoring-schema.yml` | active | pull_request, workflow_dispatch | 1 | KEEP/REVIEW |
| `.github/workflows/agent-governance.yml` | active | pull_request, workflow_dispatch | 1 | KEEP; deduplicate |
| `.github/workflows/architecture-semantic-audit.yml` | active | pull_request | 1 | KEEP/REVIEW |
| `.github/workflows/area-authoring-schema.yml` | active | pull_request, workflow_dispatch | 1 | KEEP/REVIEW |
| `.github/workflows/auto-archive-merged-tasks.yml` | active | push, workflow_dispatch | 1 | OPERATION |
| `.github/workflows/charm-authoring-schema.yml` | active | pull_request, workflow_dispatch | 1 | KEEP/REVIEW |
| `.github/workflows/claude-review.yml` | disabled_manually | pull_request | 1 | ARCHIVE? |
| `.github/workflows/codeql.yml` | active | push, schedule, workflow_dispatch | 1 | KEEP/REVIEW |
| `.github/workflows/codex-review-gate.yml` | active | pull_request, pull_request_review, issue_comment, schedule, workflow_dispatch | 1 | KEEP/REVIEW |
| `.github/workflows/content-tree-migration.yml` | active | pull_request, workflow_dispatch | 4 | KEEP/REVIEW |
| `.github/workflows/content-world-b6-interaction-bindings.yml` | active | pull_request | 1 | KEEP/REVIEW |
| `.github/workflows/craft-authoring.yml` | active | pull_request, workflow_dispatch | 1 | KEEP/REVIEW |
| `.github/workflows/full-content-source-discovery.yml` | disabled_manually | pull_request, workflow_dispatch | 1 | ARCHIVE? |
| `.github/workflows/g4-canonical-worldproject-package-seed.yml` | disabled_manually | pull_request | 1 | ARCHIVE? |
| `.github/workflows/g4-direct-nonitem-family-crosswalk.yml` | disabled_manually | pull_request, workflow_dispatch | 1 | ARCHIVE? |
| `.github/workflows/g4-item-binding-pilot.yml` | disabled_manually | pull_request, workflow_dispatch | 1 | ARCHIVE? |
| `.github/workflows/g4-item-crystal-bindings.yml` | active | pull_request, workflow_dispatch | 1 | KEEP/REVIEW |
| `.github/workflows/g4-item-source-capture.yml` | disabled_manually | pull_request, workflow_dispatch | 1 | ARCHIVE? |
| `.github/workflows/g4-item-wave1-capture.yml` | disabled_manually | pull_request, workflow_dispatch | 1 | ARCHIVE? |
| `.github/workflows/g4-non-item-source-capture.yml` | disabled_manually | pull_request, workflow_dispatch | 1 | ARCHIVE? |
| `.github/workflows/g4-nonitem-bulk-crosswalk.yml` | disabled_manually | pull_request, workflow_dispatch | 1 | ARCHIVE? |
| `.github/workflows/g4-nonitem-revision-revalidation.yml` | disabled_manually | push, pull_request, workflow_dispatch | 1 | ARCHIVE? |
| `.github/workflows/game-atlas-animated-appearances.yml` | disabled_manually | pull_request, workflow_dispatch | 1 | ARCHIVE? |
| `.github/workflows/game-atlas-creature-gameplay-profiles.yml` | disabled_manually | pull_request, workflow_dispatch | 1 | ARCHIVE? |
| `.github/workflows/game-atlas-profile-spike.yml` | disabled_manually | pull_request | 1 | ARCHIVE? |
| `.github/workflows/game-atlas-semantic-search.yml` | disabled_manually | pull_request, push | 1 | ARCHIVE? |
| `.github/workflows/game-atlas-static-creatures.yml` | disabled_manually | pull_request, workflow_dispatch | 1 | ARCHIVE? |
| `.github/workflows/game-atlas-thais-fixture.yml` | disabled_manually | pull_request | 1 | ARCHIVE? |
| `.github/workflows/gameplay-server-seam.yml` | active | workflow_dispatch | 1 | KEEP/REVIEW |
| `.github/workflows/global-source-overlap.yml` | disabled_manually | pull_request, workflow_dispatch | 2 | ARCHIVE? |
| `.github/workflows/house-authoring-schema.yml` | active | pull_request, workflow_dispatch | 1 | KEEP/REVIEW |
| `.github/workflows/house-tibiawiki-br-capture.yml` | disabled_manually | pull_request, workflow_dispatch | 1 | ARCHIVE? |
| `.github/workflows/integration-regressions.yml` | active | pull_request, workflow_dispatch | 2 | KEEP/REVIEW |
| `.github/workflows/integration-shards-dispatch.yml` | active | workflow_dispatch | 3 | KEEP/REVIEW |
| `.github/workflows/item-authoring-schema.yml` | active | pull_request, workflow_dispatch | 1 | KEEP/REVIEW |
| `.github/workflows/item-content-continuity.yml` | disabled_manually | pull_request, workflow_dispatch | 1 | ARCHIVE? |
| `.github/workflows/item-content-promotion.yml` | disabled_manually | pull_request, workflow_dispatch | 1 | ARCHIVE? |
| `.github/workflows/item-content-verification.yml` | disabled_manually | pull_request, workflow_dispatch | 1 | ARCHIVE? |
| `.github/workflows/item-wiki-first-census.yml` | disabled_manually | pull_request, workflow_dispatch | 1 | ARCHIVE? |
| `.github/workflows/merge-authority-audit.yml` | active | pull_request_target | 1 | KEEP/REVIEW |
| `.github/workflows/merge-gate.yml` | active | pull_request | 15 | KEEP; deduplicate |
| `.github/workflows/merge-group-gate.yml` | active | merge_group, push | 20 | KEEP/REVIEW |
| `.github/workflows/molehill-fallback.yml` | active | schedule, workflow_dispatch | 1 | OPERATION |
| `.github/workflows/monster-wiki-capture.yml` | disabled_manually | pull_request, workflow_dispatch | 1 | ARCHIVE? |
| `.github/workflows/native-entry-room-qualification.yml` | active | pull_request, workflow_dispatch | 1 | KEEP/REVIEW |
| `.github/workflows/node-boot-build.yml` | active | workflow_call | 1 | KEEP/REVIEW |
| `.github/workflows/node-boot-qualification.yml` | active | workflow_dispatch, schedule | 5 | KEEP/REVIEW |
| `.github/workflows/node-boot-real-clock-nightly.yml` | active | schedule, workflow_dispatch | 1 | KEEP; real clock |
| `.github/workflows/npc-tibiawiki-br-capture.yml` | disabled_manually | pull_request, workflow_dispatch | 1 | ARCHIVE? |
| `.github/workflows/oracle-operations.yml` | active | workflow_dispatch | 2 | OPERATION |
| `.github/workflows/pg-cargo-dependency-cache.yml` | active | push, schedule, workflow_dispatch | 1 | KEEP/REVIEW |
| `.github/workflows/pg-test-rerun.yml` | active | workflow_dispatch | 1 | KEEP/REVIEW |
| `.github/workflows/postgres-test-partitions.yml` | active | workflow_call | 2 | KEEP/REVIEW |
| `.github/workflows/pr-postgres-harness.yml` | active | pull_request | 9 | KEEP/REVIEW |
| `.github/workflows/proficiency-authoring-schema.yml` | active | pull_request, workflow_dispatch | 1 | KEEP/REVIEW |
| `.github/workflows/quest-authoring-schema.yml` | active | pull_request, workflow_dispatch | 1 | KEEP/REVIEW |
| `.github/workflows/quest-container-source.yml` | disabled_manually | pull_request, workflow_dispatch | 1 | ARCHIVE? |
| `.github/workflows/repository-configuration.yml` | active | push, workflow_dispatch | 1 | OPERATION |
| `.github/workflows/reward-claim-authoring.yml` | active | pull_request, workflow_dispatch | 1 | KEEP/REVIEW |
| `.github/workflows/rust-cache-pilot.yml` | active | workflow_dispatch | 1 | KEEP/REVIEW |
| `.github/workflows/rust-fast-archive.yml` | active | workflow_call | 2 | KEEP/REVIEW |
| `.github/workflows/rust.yml` | active | push, workflow_dispatch | 3 | KEEP/REVIEW |
| `.github/workflows/source-family-classification.yml` | disabled_manually | pull_request, workflow_dispatch | 2 | ARCHIVE? |
| `.github/workflows/spell-wiki-capture.yml` | active | pull_request, workflow_dispatch | 3 | KEEP/REVIEW |
| `.github/workflows/store-authoring-schema.yml` | active | pull_request, workflow_dispatch | 1 | KEEP/REVIEW |
| `.github/workflows/synology-game-deploy.yml` | disabled_manually | workflow_dispatch | 2 | ARCHIVE? |
| `.github/workflows/synology-game-runner-acceptance.yml` | disabled_manually | push, workflow_dispatch | 1 | ARCHIVE? |
| `.github/workflows/terminal-branch-lifecycle.yml` | active | pull_request, pull_request_target, push, schedule, workflow_dispatch | 3 | OPERATION |
| `.github/workflows/tibia-reference-tools.yml` | disabled_manually | pull_request, workflow_dispatch | 1 | ARCHIVE? |
| `.github/workflows/tibiacom-snapshot-verify.yml` | active | pull_request, workflow_dispatch | 1 | KEEP/REVIEW |
| `.github/workflows/unknown-closure.yml` | disabled_manually | pull_request, workflow_dispatch | 1 | ARCHIVE? |
| `.github/workflows/unknown-disposition-ledger.yml` | disabled_manually | pull_request, workflow_dispatch | 1 | ARCHIVE? |
| `.github/workflows/wheel-authoring-schema.yml` | active | pull_request, workflow_dispatch | 1 | KEEP/REVIEW |
| `.github/workflows/world-metadata-authoring.yml` | active | pull_request, workflow_dispatch | 1 | KEEP/REVIEW |
| `.github/workflows/world-project-package.yml` | active | pull_request | 1 | KEEP/REVIEW |
| `.github/workflows/worldproject-v2-full-cardinality-scale.yml` | disabled_manually | pull_request, workflow_dispatch | 3 | ARCHIVE? |
| `.github/workflows/wp5-s3a-real-interop.yml` | disabled_manually | push, pull_request, workflow_dispatch | 1 | ARCHIVE? |
| `.github/workflows/wp5-s3b-composition.yml` | disabled_manually | push, pull_request, workflow_dispatch | 1 | ARCHIVE? |

## Complete repository-test module structural inventory

Counts below are static `def test_*` counts; custom `main()` assertion suites and dynamic subtests make runtime counts different. Purpose is the source module's first docstring line, not a semantic certification.

| Module | Static test functions | Source purpose |
|---|---:|---|
| `test_agent_governance_pr_targets.py` | 5 | Execute the hosted metadata check against live-PR response fixtures. |
| `test_benchmark_rust_cache.py` | 11 | Prove benchmark isolation and honest sample grouping without compiling Rust. |
| `test_check_dispatch_pg_shards.py` | 10 | No module docstring |
| `test_check_duplicate_test_sources.py` | 14 | Regressions for the duplicate test source guard and the Merge Queue test shard planner. |
| `test_classify_content_routing.py` | 7 | Regressions for content/import exact-consumer impact routing. |
| `test_classify_post_merge_lanes.py` | 0 | Protected-main regressions for exact-candidate impact routing. |
| `test_classify_pr_test_lanes.py` | 21 | Behavioral regressions for exact-candidate impact routing. |
| `test_classify_quest_authoring_scope.py` | 30 | Routing regressions for the Quest-authoring scope classifier and its workflow wiring. |
| `test_collect_rust_fast_results.py` | 21 | No module docstring |
| `test_heavy_gate.py` | 26 | Exercise the actual heavy-gate script and PostgreSQL runner expressions. |
| `test_job_result_key.py` | 28 | No module docstring |
| `test_main_job_applicability.py` | 10 | Main qualification stays applicable when stack events ignore branch filters. |
| `test_merge_authority_applicability.py` | 1 | The protected main audit must not reject prepared native stack events. |
| `test_normalize_cargo_mtimes.py` | 8 | No module docstring |
| `test_pg_archive_affinity.py` | 5 | Exercise archive-affinity coverage and same-run artifact custody. |
| `test_pg_test_durations.py` | 4 | No module docstring |
| `test_plan_pg_partitions.py` | 9 | Check that PostgreSQL archives produce one complete, balanced test plan. |
| `test_protected_main_cache.py` | 7 | The protected cache writer must finish and publish only a bounded archive. |
| `test_refresh_ci_workflow_pins.py` | 3 | No module docstring |
| `test_reliability_checklist.py` | 8 | The reliability checklist stays loadable, sourced and self-testing. |
| `test_repository_tests_run.py` | 2 | Every tools/repository test runs in the merge gate's governance job. |
| `test_required_status_checks.py` | 2 | Both required aggregates must survive policy validation and API readback. |
| `test_run_rust_fast_archive.py` | 9 | One archive can finish independently without certifying another archive. |
| `test_rust_cache_pilot_workflow.py` | 3 | Exercise the pilot's actual source-binding step against a small Git history. |
| `test_rust_fast_archive_routing.py` | 2 | The shared archive producer contributes its exact-candidate helper consumers. |
| `test_select_postgres_packages.py` | 10 | Regression coverage for PostgreSQL package selection on exact Git inputs. |
| `test_shared_dispatch_archive.py` | 39 | No module docstring |
| `test_trim_cargo_cache.py` | 13 | No module docstring |
| `test_validate_coded_output.py` | 4 | Coded failure output of the two governance validators (ERR-TOOLS-3). |
| `test_validate_game_atlas_semantic_search_triggers.py` | 6 | Protect the Atlas workflows' trigger closure and exact-source oracles. |
| `test_validate_merge_group_pg_sim.py` | 1 | Exercise the required Merge Queue contract and its credibility controls. |
| `test_validate_pr_gate_pg_sim.py` | 17 | Prove that removing PR PG/SIM duplication preserves MQ qualification. |

## Remaining uncertainty and handoff

- No full Rust build, database test run, game E2E run or complete per-case semantic classification was performed by this lane.
- No measured whole-pipeline or runner-minute saving. The only timing claim is the bounded sequential unittest sample above.
- Live API states are a timestamped observation; repository definitions alone do not reveal active/disabled registration state.
- Candidate patches need parent review and required repository policy/CI checks before any merge; root is the only writer. Direct candidate `test_validate_pr_gate_pg_sim.py` passed 17 tests and `test_repository_tests_run.py` passed 2. The standalone policy validator was attempted against the scratch overlay and stopped with E8002 because that overlay has no readable `origin/main` for its append-only check; this is an environment limitation, not a policy PASS. Log: `ci_dedup_policy_after.log`.
- Runtime black-box scenario outcome completeness and client capture/replay are owned by the parallel black-box audit lane. A search of all 78 workflow definitions found no `E2E_ALL_CAPTURE_DIR` reference; caller conclusions beyond that search belong to that lane.

Evidence files: `ci_lane_inventory.json` (all jobs, needs, matrix, commands and triggers); `ci_snapshot/workflow_live_state.json` (all paginated registrations); `ci_snapshot/selected_recent_runs.json`; `ci_dedup_before.json`, `ci_dedup_after.json`, `ci_dedup_summary.json` (executed canonical IDs); `ci_dedup_entrypoints.json` (static dispatcher closure); `ci_dedup_probe.py` (reproducible focused command); `ci_dedup_A.patch`, `ci_dedup_B.patch`.
