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
updated_at: 2026-11-22T23:20:07Z
execution_policy: continuous_progress
owned_paths:
  - .github/workflows/agent-governance.yml
  - tools/repository/test_agent_governance_pr_targets.py
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

Applied: repository-test dispatch dedup with standalone caller preservation; craft duplicate removal/direct-discovery repair; exact scenario/cause/root baseline with fail-closed metadata; StageDump child-exit/final-status oracle; diagnostics single discovery and existing WorldFixture identity checks.

PROVEN: focused CI retains all42 unique IDs (54 executions before/42 after); craft retains seven unique bodies; actual-stage changed-root regression fails on original source and passes after repair. Peer QA found ambiguous metadata accepted by the initial parser; root added six red subcases and fixed it before publication. No Rust proposal is applied without compiler evidence.

Open: matrix expected-ID completeness; incomplete/capture-only drivers; Windows input selection; source-label/count oracles; main-only PG receipt-test ownership; all remaining semantic scopes. Exact findings/dispositions are retained under the evidence directory.

## Validation

Authoring commands/results: `../../evidence/OTV2-20261122-test-ci-value-audit/validation.json`. Diagnostics48 and affected CI42 pass; craft discovery/direct7 pass. Full node-boot discovery has seven environment errors from denied AF_UNIX creation; no provision tests were weakened/skipped. Final counts and policy/governance checks are in the evidence. Full repository sweep: 30/32 entrypoints pass; two require absent rustc/pwsh. Full qualification remains open.

Rust, PostgreSQL, Windows/GPU, real game/Platform and PowerShell execution remain UNKNOWN. First candidate `978d40a02e9e9d4b1d8c16fe7dea25aad586e108` was published, frozen and checked from a clean checkout; provider review completed without findings. These results belong to that exact head; the second evidence-only candidate needs its own relevant checks. Final SHA and positive PR number belong to live PR/check evidence; do not create a metadata-only successor to copy them here.

## Self-review and independent review

Root reviewed code, original/fixed reproductions, unique-case retention, caller closure, baseline compatibility and scope labels. Independent authoring peer QA checked parser ambiguity, discovery and assertion retention; its accepted P2 was fixed and rechecked. This is not external exact-head Codex Review. Root requested one review after live dedup; provider completed it without findings for `978d40a02e9e9d4b1d8c16fe7dea25aad586e108` (PR comment 7087293938). No second paid review is requested for unchanged code.

## PR and closeout

Draft PR #3430 is open. Root explicitly reopened AUTHORING from `978d40a02e9e9d4b1d8c16fe7dea25aad586e108` for the substantive evidence-only [second wave](../../evidence/OTV2-20261122-test-ci-value-audit/wave-2/README.md), on a separate checkout. The original code-bearing candidate remains preserved. Do not merge. Continue substantive work in coherent batches under a single-writer exact-head lifecycle. Unvisited cases and unavailable runtime checks stay open; they do not become passed or unnecessary by timeout.

## Context checkpoint

```yaml
last_progress: First draft validated and externally reviewed; substantive second-wave findings retained
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
next_action: Publish and freeze the evidence-only successor; validate its exact tree and continue the enabled overnight remaining-scope queue
```
