# OTV2-20260818-target-coordinate-reconciliation

```yaml
task_id: OTV2-20260818-target-coordinate-reconciliation
title: Reconcile Oteryn-Game target repository identity
mode: MIGRATE
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: docs/otv2-20260818-target-coordinate-reconciliation
pr: null
base_sha: 16afdf31a15bd49d454cdbcdd98fa7ec72213ef9
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: chat-github-20260818-target-coordinate-reconciliation
created_at: 2026-08-18T11:06:00Z
updated_at: 2026-08-18T11:06:00Z
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
  - ordinary autonomous work using target-local governance
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
- all 36 source snapshot branch refs were recreated with the original commit SHAs.
- `blakinio/Oteryn-v2` remains unchanged and continues to exist as the legacy/migration source.
- verified external backup `Oteryn-v2-full-git-backup-2026-08-18-final.zip` exists on connected Google Drive.

## Acceptance criteria

- [ ] Root and machine-readable governance identify `Oteryn/Oteryn-Game` as the sole routine write repository.
- [ ] `blakinio/Oteryn-v2` is explicitly retained as read-only legacy/migration provenance rather than silently rewritten.
- [ ] current governance README/repository map/task template/project lanes reflect the target coordinate.
- [ ] current repository link metadata (`Cargo.toml`, issue security URL, root README) reflects the target coordinate/name.
- [ ] governance validator expects and passes for `Oteryn/Oteryn-Game`.
- [ ] archived tasks/evidence/ADRs and immutable historical references are not mass-rewritten.
- [ ] exact changed-file scope and full-diff self-review pass.
- [ ] exact-head applicable PR workflows pass with clean review hygiene.
- [ ] squash merge and lifecycle closeout complete.

## Excluded scope

No runtime/protocol/persistence/gameplay behavior change. No Platform/META/Atlas write. No source repository deletion, archive or mutation. No cross-repository contract-lock semantic rewrite in this task.

## Context checkpoint

```yaml
last_progress: target history and source branch snapshot are present; dedicated target-coordinate reconciliation branch and task created
status: implementing
branch: docs/otv2-20260818-target-coordinate-reconciliation
head_sha: null
pr: null
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request
ci_check_generation: null
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: not_started
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: apply exact target-coordinate replacements only in declared live governance/navigation files, then open Draft PR and validate the final diff
```
