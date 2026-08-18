# OTV2-20260818-target-coordinate-reconciliation

```yaml
task_id: OTV2-20260818-target-coordinate-reconciliation
title: Reconcile Oteryn-Game target repository identity
mode: MIGRATE
status: blocked
repository: Oteryn/Oteryn-Game
base_branch: main
branch: docs/otv2-20260818-target-coordinate-reconciliation
pr: 4
base_sha: 16afdf31a15bd49d454cdbcdd98fa7ec72213ef9
head_sha: 6ff9e3e57c24ba510adfac330b7448bfb7a4c1bf
final_head_sha: null
final_head_frozen_at: null
owner: chat-github-20260818-target-coordinate-reconciliation
created_at: 2026-08-18T11:06:00Z
updated_at: 2026-08-18T11:23:00Z
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - AGENTS.md
  - README.md
  - Cargo.toml
  - .github/ISSUE_TEMPLATE/config.yml
  - docs/agents/README.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CROSS_REPO_CONTRACTS.md
  - docs/agents/GOVERNANCE_CONTRACT.json
  - docs/agents/PROJECT_LANES.json
  - docs/agents/tasks/TASK_TEMPLATE.md
  - tools/agents/validate_governance.py
  - docs/agents/tasks/active/OTV2-20260818-target-coordinate-reconciliation.md
public_contracts: []
depends_on:
  - Oteryn/Oteryn ADR 0001 ecosystem topology authority
  - history-preserving copy main@16afdf31a15bd49d454cdbcdd98fa7ec72213ef9
blocks:
  - ordinary autonomous work using target-local governance until PR 4 merges
  - exact-head merge gate until target Dependency graph is enabled
cross_repository_coordination_id: OTERYN-GAME-COPY-20260818
external_repositories:
  - blakinio/Oteryn-v2
  - Oteryn/Oteryn
```

## Outcome

Reconcile only live repository identity and current governance/navigation surfaces after the history-preserving copy from `blakinio/Oteryn-v2` to `Oteryn/Oteryn-Game`. Preserve historical ADRs, archived tasks, evidence and source-era provenance unchanged.

## Proven baseline

- `Oteryn/Oteryn-Game` exists as public repository ID `1338291140` with connector admin/write access.
- target `main` is the exact preserved source commit `16afdf31a15bd49d454cdbcdd98fa7ec72213ef9`.
- all 36 source snapshot branch refs were recreated with the original commit SHAs; target-generated Dependabot refs are post-import state, not source-history mismatch.
- `blakinio/Oteryn-v2` remains unchanged and continues to exist as the legacy/migration source.
- verified external backup `Oteryn-v2-full-git-backup-2026-08-18-final.zip` exists on connected Google Drive.
- the first workflow mirror attempt failed closed on workflow-file token scope; exact connector ref creation/update then completed the history copy with original SHAs.
- temporary bootstrap/import/cleanup branches and workflows were removed or closed without merge.

## Acceptance criteria

- [x] Root and machine-readable governance candidate identify `Oteryn/Oteryn-Game` as the sole routine write repository.
- [x] `blakinio/Oteryn-v2` is explicitly retained as read-only legacy/migration provenance rather than silently rewritten.
- [x] current governance README/repository map/task template/project lanes reflect the target coordinate.
- [x] current repository link metadata (`Cargo.toml`, issue security URL, root README) reflects the target coordinate/name.
- [x] governance validator expects and passed for `Oteryn/Oteryn-Game` in the exact replacement workflow and PR merge-gate governance job.
- [x] archived tasks/evidence/ADRs and immutable historical references were not mass-rewritten.
- [x] temporary coordinate-reconciliation and dependency-graph probe workflows were removed from the candidate branch.
- [x] exact changed-file inventory is limited to the 12 declared live governance/navigation/task paths.
- [x] full-diff self-review repaired one formatting-only churn finding in `PROJECT_LANES.json`; no material finding remains in the candidate diff.
- [ ] target Dependency graph is enabled so the mandatory dependency-review sub-gate can execute.
- [ ] one unchanged final head passes the complete merge gate and review hygiene.
- [ ] squash merge and lifecycle closeout complete.

## Excluded scope

No runtime/protocol/persistence/gameplay behavior change. No Platform/META/Atlas write. No source repository deletion, archive or mutation. No cross-repository contract-lock semantic rewrite in this task. No use or copying of source repository secrets.

## Validation

### Focused

- history-preserving target main identity: PASS (`16afdf31a15bd49d454cdbcdd98fa7ec72213ef9`)
- source repository existence/identity after copy: PASS (`blakinio/Oteryn-v2`, repository ID `1323412342`)
- target source-snapshot branch reconstruction: PASS for all 36 source heads at exact source commit SHAs
- target coordinate replacement workflow: PASS
- `python tools/agents/validate_governance.py`: PASS for target repository identity
- exact PR changed-file inventory: PASS, 12 declared paths
- full-diff self-review: PASS after repairing unrelated JSON formatting churn
- temporary workflow removal: PASS

### Component/integration

- repository-object history preservation: PASS by exact target main/source commit identity and exact recreated branch object SHAs
- target dependency-review infrastructure: BLOCKED; `actions/dependency-review-action` reported `Dependency review is not supported on this repository. Please ensure that Dependency graph is enabled`
- dependency submission probe with `contents: write`: FAIL_CLOSED with HTTP 404, independently confirming Dependency graph is currently unavailable to repository workflows
- live GitHub repository settings policy reconciliation: PENDING separate administration-capable path; target still has creation defaults and source `REPO_ADMIN_TOKEN` is not authorized for reuse/copy

### E2E

- scenario: NOT_APPLICABLE; migration/governance identity only, no game runtime behavior changed
- result: NOT_APPLICABLE

### Exact-head CI

- successful candidate-generation gates before infrastructure blocker: Agent governance, Architecture semantic audit, Merge authority audit, merge-gate scope/governance/Rust policy/Linux/CodeQL/supply-chain
- blocking sub-gate: Merge gate / dependency review
- first actionable failure: target Dependency graph unavailable
- classification: EXTERNAL_REPOSITORY_CONFIGURATION_BLOCKER
- final exact-head generation: deferred until owner enables Dependency graph

## Self-review

- method/reviewer: implementing/coordinating agent
- changed paths: exactly 12 declared paths after temporary workflow removal
- material findings: 0 open
- repaired finding: `PROJECT_LANES.json` was initially reformatted wholesale by a JSON serializer; restored original formatting so only repository identity changes
- verdict: PASS for content; merge readiness remains blocked by repository configuration

## Independent review

- required: NO; this task substitutes the canonical repository coordinate after an explicitly owner-authorized history-preserving copy, removes routine-write authority from the preserved source, does not increase the number of writable repositories, weaken a safety gate or mutate security/protocol/durable-data/runtime semantics
- exact head: NOT_APPLICABLE
- method/auditor: NOT_APPLICABLE
- verdict: NOT_APPLICABLE

## PR and closeout

- PR: #4 Draft
- changed-file inventory: exactly 12 declared paths
- related bootstrap PR #1: closed without merge
- related cleanup PR #3: closed without merge
- merge: BLOCKED on Dependency graph
- ownership release: pending lifecycle closeout after merge

## Context checkpoint

```yaml
last_progress: reproduced the only merge-gate failure as target Dependency graph unavailability, attempted a safe contents-write dependency-submission probe which failed closed with HTTP 404, and removed the temporary probe workflow
status: blocked
branch: docs/otv2-20260818-target-coordinate-reconciliation
head_sha: 6ff9e3e57c24ba510adfac330b7448bfb7a4c1bf
pr: 4
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request
ci_check_generation: blocked_dependency_graph
ci_checks_for_current_head: 1
ci_run_ids:
  - 32130772231
ci_job_ids:
  - 95691334715
runner_assignment_state: completed_failure_external_configuration
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 1
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 1
ci_recovery_actions_for_current_head: 1
stall_warnings: 0
owner_action_required: enable Dependency graph in Oteryn/Oteryn-Game Settings -> Advanced Security; do not weaken or skip dependency review
blocker: target Dependency graph is disabled/unavailable and the connected GitHub tool exposes no administration-write operation to enable it
next_action: after Dependency graph is enabled, refresh PR 4 live head and checks, freeze the exact final candidate and complete merge plus lifecycle closeout
```
