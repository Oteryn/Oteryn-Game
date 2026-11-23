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
updated_at: 2026-11-23T11:47:37Z
execution_policy: continuous_progress
owned_paths:
  - tools/repository/test_agent_governance_pr_targets.py
  - tools/repository/governance_main_job_cases.py
  - tools/repository/governance_merge_authority_cases.py
  - tools/repository/test_main_job_applicability.py
  - tools/repository/test_merge_authority_applicability.py
  - tools/repository/test_heavy_gate.py
  - tools/repository/test_wp5_s3b_verdict.py
  - tools/qualification/wp5_s3b/run.sh
  - tools/qualification/node_boot/matrix.py
  - tools/qualification/node_boot/test_matrix.py
  - tools/content-migration/test_boss_corpse_terminal_reference.py
  - tools/content-migration/test_coin_converter_profiles.py
  - tools/content-migration/test_creature_level_rating.py
  - tools/content-migration/test_parallel_checks.py
  - tools/content-migration/test_world_project_v2_to_tree.py
  - tools/content-schema/item-authoring/test_engine_items.py
  - tools/repository/test_validate_pr_gate_pg_sim.py
  - tools/content-schema/craft-authoring/test_tainted_souls.py
  - tools/qualification/node_boot/run.sh
  - tools/qualification/node_boot/known_fail_rows.txt
  - tools/qualification/node_boot/test_diagnostics.py
  - tools/qualification/node_boot/test_node_boot_tools.py
  - docs/agents/evidence/OTV2-20261122-test-ci-value-audit/
  - docs/agents/tasks/active/OTV2-20261122-test-ci-value-audit.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome and authorization

The user requested an autonomous overnight audit of all tests and CI, justified consolidation/removal of redundant or obsolete work, black-box proposals and optionally a draft PR. Root owns one draft audit branch; no merge is authorized. Canonical tracking: [issue #3429](https://github.com/Oteryn/Oteryn-Game/issues/3429). [Audit report](../../evidence/OTV2-20261122-test-ci-value-audit/README.md) and lane manifests distinguish inventory, semantic review and runtime execution.

PROVEN: baseline is main's squash of PR #3391. Root is sole writer; independent lanes inspect CI, source-test value, client behavior and black-box harnesses. The separate E2E branch is not owned here. Immutable META policy is bound by `docs/agents/META_AGENT_POLICY_BINDING.json`. Root explicitly selects its API-native new-candidate exception because local Git push credentials are unavailable. No selected/frozen local commit is reconstructed: one complete bounded tree delta, one successor with the live predecessor as sole parent, one leased non-force ref update, immediate live readback, then freeze and requalification.

## High-risk authority/recovery qualification

NOT_APPLICABLE: this candidate changes test/CI dispatch and qualification-result parsing, not production session authority, PREPARE/COMMIT, persisted recovery or database fencing. Existing authority/value tests remain. Failure classification has negative regressions and actual-stage readback.

## Acceptance criteria

- [x] Inventory baseline source-test candidates and every committed CI definition; retain scope limits.
- [x] Prove repeated execution or equivalent fixture/oracle before removing duplicate work; preserve unique cases.
- [x] Reproduce known-failure masking and a false-positive test oracle, then repair with meaningful regressions.
- [x] Record black-box proposals and remaining semantic-review queue.
- [ ] Finish semantic review of remaining indexed cases; static scan does not complete them.
- [x] Publish draft PR #3430; freeze the first code-bearing head and retain exact-head qualification/review evidence. Full runtime qualification remains open.
- [ ] Continue authorized audit work and report morning progress, gaps and findings.

## Reference parity

NOT_APPLICABLE: no gameplay mechanic or reference balance value changes.

## Implementation and findings

The first candidate repaired craft duplicate/omitted execution, known-failure signature matching, StageDump's false-positive oracle and diagnostics discovery. Wave 5 restored the protected governance workflow byte-for-byte after its actual protected-base gate rejected the original workflow change; the focused original slice then had 53 executions / 42 unique cases.

Current [wave 9](../../evidence/OTV2-20261122-test-ci-value-audit/wave-9/README.md) applies the subsequent reviewed repairs: exact selected indexed-row identity coverage in node-boot; actual one-test execution required for S3B; governance owner consolidation preserving all 16 cases and protected workflow bytes; Frost Cannon registration; independent coin/calibration oracles; canonical reference no-allocation semantics; and three content modules connected to the real migration runner. The focused original slice now retains 42 unique cases without the 11 duplicate executions. Four newly added S3B tests are additional coverage, outside that original slice.

Open: basic/group/spells/Game–Platform completeness; incomplete/capture-only physical journeys; Windows input selection; source-label/count oracles; main-only PG receipt-test ownership; historical data fixtures; all remaining semantic scopes. The content-tool queue remains 627 paths and the separate global queue remains 130 groups. Repairs of already reviewed cases do not reduce either inventory.

## Validation

The wave-9 evidence distinguishes actual real CI artifact replay, compiled tiny Rust libtest outcomes, controlled shell fixtures and current gameplay qualification. Root verified integrated source hashes: all 31 repository test entrypoints, 109 node-boot tests, 135 governance tests, 11 focused migration tests, policy and pins passed on the isolated VPS. The engine's initial /tmp quota error was resolved by a same-filesystem TMPDIR, without source changes: 603 checks plus 3+3+4 subsidiary tests passed. Final authoring receipts are retained with the batch; exact frozen-head qualification is recorded in live PR/check evidence after publication.

PROVEN before publication: governance direct/discovery 27→16 executions with the same 16 unique cases; engine 599→603 checks with all 108 functions selected once; real migration caller executes the three added modules; invalid empty/missing calibration and coin inputs are rejected. Four real CI ZIP digests and all 302 source dispatcher contracts were independently compared. No successful live gameplay claim is inferred from those FAIL-only artifacts.

UNKNOWN: current-head full Game/Platform/PostgreSQL topology, Windows/GPU execution, whole-CI cost/flakiness and unvisited semantic value. Tiny Rust/libtest and real PowerShell execution are now available, superseding the first environment's absence; they are not full product qualification.

## Self-review and independent review

Root reviewed the integrated source delta and original/fixed reproductions. Independent lanes reviewed matrix/content repairs, caller closure, unique-case retention and real aggregate behavior. The stale aggregate positive fixture was found, repaired and replayed against both old and new summarizers. No protected YAML changed.

The native external review completed without findings only for `978d40a02e9e9d4b1d8c16fe7dea25aad586e108` (PR comment 7087293938). Do not reuse it as a wave-9 exact-head receipt or request duplicate paid review without the bound policy's qualifying condition.

## PR and closeout

Draft PR #3430 remains open and unmerged. Root reopened AUTHORING from `245ba7838e58318a2d851857b6c2b2d7c757aa09` for wave 9, preserving earlier checkouts. Publish one complete new candidate using the already selected META exception, with this live predecessor as sole parent and a non-force expected-head lease; then verify, freeze and requalify that exact returned SHA. There is no selected local commit to reconstruct. Final SHA and CI belong in live PR/check evidence, without a metadata-only successor.

The eight scheduled overnight runs finished; their automation is disabled with no next run. This wave follows the user's explicit continuation. Do not merge. Unvisited cases and unavailable runtime checks remain open; timeout never changes them to PASS or unnecessary.

## Context checkpoint

```yaml
last_progress: Wave 9 integrated concrete repairs and independent replay from source predecessor 245ba783
status: implementing
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
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: Publish one wave-9 candidate, freeze its exact SHA and requalify it; retain the remaining audit queue
```
