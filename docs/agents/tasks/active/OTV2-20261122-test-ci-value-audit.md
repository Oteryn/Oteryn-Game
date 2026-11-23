# OTV2-20261122-test-ci-value-audit

```yaml
task_id: OTV2-20261122-test-ci-value-audit
title: Audit test value and CI consumers; strengthen black-box evidence
mode: AUDIT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/test-ci-value-audit-20261122
pr: 3430
issue: 3429
base_sha: 8e9f0e16631aab1beb8416f8d5ca707aa3031083
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT root; exclusive publication writer
created_at: 2026-11-22T22:44:25Z
updated_at: 2026-11-23T13:33:50.372482+00:00
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/tests/ai_bootstrap.rs
  - apps/game-server/tests/native_admission_source_postgres.rs
  - apps/game-server/tests/support/bank_postgres_cases.rs
  - crates/client-runtime/src/features/0064-target-move/tests/e2e_m01.rs
  - crates/client-runtime/src/features/0287-actor-spell-catalog/tests/e2e_c14.rs
  - crates/client-runtime/src/features/local-spell-action-bar/tests/e2e_c04.rs
  - crates/client-runtime/tests/gap_house.rs
  - crates/client-ui/src/features/0117-boss-encounter/tests/state.rs
  - crates/client-ui/src/features/0165-hireling/mod.rs
  - crates/client-ui/tests/chat_log.rs
  - crates/gameplay/durability/src/durability/bank.rs
  - crates/gameplay/durability/src/durability/core_modules_linkage_tests.rs
  - crates/gameplay/durability/src/durability/db_holder_wait_tests.rs
  - crates/gameplay/durability/src/durability/db_tests/holder_round_trip_tests.rs
  - crates/gameplay/foundation/src/scheduled_events.rs
  - crates/gameplay/monster-ai/src/ai/mod.rs
  - crates/gameplay/monster-ai/src/ai/perception.rs
  - crates/gameplay/monster-ai/src/ai_think.rs
  - crates/identity/src/lib.rs
  - crates/input-platform/tests/native_winit.rs
  - crates/protocol-oteryn/src/analyser_catalogue.rs
  - crates/protocol-oteryn/src/analyser_party.rs
  - crates/protocol-oteryn/src/appearance_tests/mappings_tests.rs
  - crates/protocol-oteryn/src/atelier.rs
  - crates/protocol-oteryn/src/attack_tests.rs
  - crates/protocol-oteryn/src/battle_metadata/tests.rs
  - crates/protocol-oteryn/src/bestiary.rs
  - crates/protocol-oteryn/src/bestiary_catalogue.rs
  - crates/protocol-oteryn/src/boss_difficulty_tests.rs
  - crates/protocol-oteryn/src/boss_encounter.rs
  - crates/protocol-oteryn/src/boss_encounter/catalogue_tests.rs
  - crates/protocol-oteryn/src/boss_encounter_tests.rs
  - crates/protocol-oteryn/src/boss_rewards_tests.rs
  - crates/protocol-oteryn/src/contract_registries.rs
  - crates/protocol-oteryn/src/cyclopedia.rs
  - crates/protocol-oteryn/src/daily_boost.rs
  - crates/protocol-oteryn/src/guild_war.rs
  - crates/renderer/src/batch.rs
  - docs/agents/evidence/OTV2-20261122-test-ci-value-audit/
  - docs/agents/tasks/active/OTV2-20261122-test-ci-value-audit.md
  - tools/agents/tests/test_enqueue_readiness.py
  - tools/content-migration/test_boss_corpse_terminal_reference.py
  - tools/content-migration/test_cake_bridge_reference.py
  - tools/content-migration/test_coin_converter_profiles.py
  - tools/content-migration/test_creature_level_rating.py
  - tools/content-migration/test_forge_sql_admission.py
  - tools/content-migration/test_harvest_item_profiles.py
  - tools/content-migration/test_itemhold_q.py
  - tools/content-migration/test_monster_reference_level.py
  - tools/content-migration/test_official_creature_balance.py
  - tools/content-migration/test_parallel_checks.py
  - tools/content-migration/test_qengine_reward_item_profiles.py
  - tools/content-migration/test_questpc4_treasure_reward_profiles.py
  - tools/content-migration/test_quick_loot_item_profiles.py
  - tools/content-migration/test_regenerate_content.py
  - tools/content-migration/test_world_project_v2_to_tree.py
  - tools/content-schema/craft-authoring/test_tainted_souls.py
  - tools/content-schema/item-authoring/test_engine_items.py
  - tools/qualification/node_boot/known_fail_rows.txt
  - tools/qualification/node_boot/matrix.py
  - tools/qualification/node_boot/run.sh
  - tools/qualification/node_boot/test_diagnostics.py
  - tools/qualification/node_boot/test_matrix.py
  - tools/qualification/node_boot/test_node_boot_tools.py
  - tools/qualification/node_boot/test_prebuilt.py
  - tools/qualification/wp5_s3b/run.sh
  - tools/repository/check_duplicate_test_sources.py
  - tools/repository/test_heavy_gate.py
  - tools/repository/test_wp5_s3b_verdict.py
  - tools/repository/test_protected_main_cache.py
  - tools/repository/test_repository_tests_run.py
  - tools/repository/test_validate_pr_gate_pg_sim.py
  - tools/synthetic-client-harness/src/main.rs
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome and authorization

The user requested an autonomous audit of all tests and CI, justified consolidation/repair, black-box proposals and draft publication, then explicitly requested completion. Root owns one draft branch; no merge is authorized. Canonical tracking: [issue #3429](https://github.com/Oteryn/Oteryn-Game/issues/3429), [draft PR #3430](https://github.com/Oteryn/Oteryn-Game/pull/3430), [current evidence](../../evidence/OTV2-20261122-test-ci-value-audit/README.md). Whole-repository semantic review remains open; the inventory is not completion evidence.

Root is the sole Git/PR publication writer. Independent lanes own disjoint test/evidence paths. The immutable META policy is bound by `docs/agents/META_AGENT_POLICY_BINDING.json`. Missing native push credentials select only its API-native **new-candidate** exception: fresh predecessor read, one complete bounded tree, one successor with that predecessor as sole parent, one expected-SHA non-force ref update, immediate readback, then exact-head freeze/qualification. No selected/frozen local commit is reconstructed. Remote machines are used for isolated validation, not publication.

## Acceptance criteria

- [x] Inventory all baseline source candidates and every committed CI definition, with exact source identity and scope limits.
- [x] Preserve unique fixture/oracle/failure-mode coverage when consolidating duplicate work.
- [x] Reproduce and repair known-failure masking, omitted execution, vacuous expectations and independent-boundary masking; preserve negative controls and failures.
- [x] Publish a draft PR and retain the first exact-head review/CI history.
- [ ] Finish semantic review of all remaining indexed cases; bounded reads and static scans do not complete unvisited scopes.
- [ ] Publish/freeze this substantive checkpoint and qualify the exact remote head; finish its remaining authoring checks first.
- [ ] Complete authorized audit work and report final findings, executed checks and explicit unresolved external/product dependencies.

## Implementation and evidence

The [current report](../../evidence/OTV2-20261122-test-ci-value-audit/README.md) and hash-bound lane manifests carry detailed findings and history. Authoring began at `245ba7838e58318a2d851857b6c2b2d7c757aa09`; the current publication predecessor is `0623fbd59c372bbf479f3965031f47393c42cd39`. Its wave9 repairs are preserved; affected integrated callers are requalified.

Applied areas: exact known-failure metadata; selected-row completeness; actual S3B case/body evidence; behavioral CI loop/cache checks; proven duplicate/discovery repairs; protocol/identity boundary oracles; supported Windows selection; client/synthetic/renderer/Unicode checks; content fixtures, finite identities and execution; durability/native test oracles. Production gameplay, data and authority rules are unchanged. Wave5 restored the protected workflow; wave9 subsequently consolidated governance ownership without editing that workflow, preserving42 unique focused cases without11 repeated executions. Unresolved product discrepancies and unreviewed cases remain explicit.

## High-risk authority/recovery qualification

Production authority/recovery behavior is unchanged. Tests touching these boundaries are qualified against the actual service rather than credited for environment-guard early returns. The isolated VPS database was PostgreSQL18.6 (`server_version_num=180006`), using task-local credentials omitted from all evidence. Existing services and permission settings are unchanged. Holder-lock and bank rollback/replay cases actually execute; subsequent native-source probes have separate exact-source receipts. No production or Platform write is performed.

## Reference parity

NOT_APPLICABLE to production mechanics: no reference balance or gameplay rule is changed. Test expectations that involve content identities are grounded in the existing owning producer/ledger. Unresolved M22 packet, Cyclopedia weight and ranged-overlay discrepancies are retained as findings rather than normalized away.

## Validation and review

Hash-bound commands, initial failures, positive/negative results and limits are under `completion/`. Selected actual results: node-boot109; protocol911; identity7; foundation394; runtime59; Windows input23; finalUI728 (670+58, one identical case removed); renderer49; content68+5 runner cases; item603+3+3+4; regeneration36+4. UI missing-command/missing-row mutations prove old PASS/new FAIL. Native PG records6 positive passes+2 expected mutant failures and restored production bytes. AI retains all7 integration and171 owner IDs:7 integration PASS,167 owner PASS+4ignored; the external-manifest adapter does not run an arena without input. Owner/server Clippy/default builds/fmt and14 duplicate-guard regressions PASS. These are bounded results, not full-workspace execution.

The reconciled VPS sweep passed38 commands, including31 repository test entrypoints, governance134 and policy/pins/shell gates. Final source hashes and exact published-head checks remain distinct. Root source review covers all277 UI candidates; family/helper audit of remaining app/session/dispatch/content scopes continues. Independent AST/callable probes verify extraction only.

Peer reviews cover changed callers and oracles. Paid review was clean only at978d40a02e (comment7087293938); no optional duplicate paid trigger is issued.

## Current CI and publication state

Published0623 has119 checks:108 success,11 skipped; both pages verified. Historical245 archive cancellation/failure evidence remains in the report; cause unknown. Latest main real-clock run45158952176 fails saved-position reload (59 rows:8PASS,50FAIL,1MISSING-SERVER); cause unproven. Skipped heavy jobs do not qualify PostgreSQL/node boot.

State is AUTHORING for the current substantive repair batch. After the final write, verify the complete owned delta, publish through the selected API-native route, freeze the exact returned/live head and run candidate-specific checks. If a material repair is necessary, explicitly reopen AUTHORING and qualify its successor. Keep reviewing remaining scopes while CI runs. Do not merge, force-push, modify main or create status-only commits.

## Context checkpoint

```yaml
last_progress: Current source repairs have hash-bound execution and mutation evidence; all changed-source authoring gates complete; publish checkpoint while family audit continues
status: implementing
lifecycle: AUTHORING
branch: codex/test-ci-value-audit-20261122
head_sha: null
pr: 3430
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: null
ci_check_generation: null
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: authoring_validation
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: Publish and freeze qualified successor, validate exact head, complete remaining family/helper audit
```
