# OTV2-20260818-target-coordinate-reconciliation

```yaml
task_id: OTV2-20260818-target-coordinate-reconciliation
title: Reconcile Oteryn-Game target repository identity
mode: MIGRATE
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: docs/otv2-20260818-target-coordinate-reconciliation
pr: 4
base_sha: 16afdf31a15bd49d454cdbcdd98fa7ec72213ef9
head_sha: a481d2b8b2d24a8dca784ee648ff5b785a6441bf
final_head_sha: null
final_head_frozen_at: null
owner: chat-github-20260818-target-coordinate-reconciliation
created_at: 2026-08-18T11:06:00Z
updated_at: 2026-08-18T11:15:00Z
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
  - ordinary autonomous work using target-local governance until this PR merges
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
- the first workflow mirror attempt was fail-closed by missing workflow-file token scope and did not replace source history; exact connector ref creation/update then completed the copy with original SHAs.
- temporary bootstrap/import/cleanup branches and temporary workflows were removed or closed without merge.

## Acceptance criteria

- [x] Root and machine-readable governance candidate identify `Oteryn/Oteryn-Game` as the sole routine write repository.
- [x] `blakinio/Oteryn-v2` is explicitly retained as read-only legacy/migration provenance rather than silently rewritten.
- [x] current governance README/repository map/task template/project lanes reflect the target coordinate.
- [x] current repository link metadata (`Cargo.toml`, issue security URL, root README) reflects the target coordinate/name.
- [x] governance validator expects and passed for `Oteryn/Oteryn-Game` in the one-off exact replacement workflow.
- [x] archived tasks/evidence/ADRs and immutable historical references were not mass-rewritten.
- [x] temporary coordinate-reconciliation workflow was removed from the candidate branch before final review.
- [ ] exact changed-file scope and full-diff self-review pass on one frozen final head.
- [ ] exact-head applicable PR workflows pass with clean review hygiene.
- [ ] squash merge and lifecycle closeout complete.

## Excluded scope

No runtime/protocol/persistence/gameplay behavior change. No Platform/META/Atlas write. No source repository deletion, archive or mutation. No cross-repository contract-lock semantic rewrite in this task. No use or copying of source repository secrets.

## Validation

### Focused

- history-preserving target main identity: PASS (`16afdf31a15bd49d454cdbcdd98fa7ec72213ef9`)
- source repository existence/identity after copy: PASS (`blakinio/Oteryn-v2`, repository ID `1323412342`)
- target source-snapshot branch reconstruction: PASS for all 36 source heads at exact source commit SHAs
- target coordinate replacement workflow: PASS
- `python tools/agents/validate_governance.py`: PASS inside replacement workflow for target repository identity
- temporary workflow removal: PASS

### Component/integration

- repository-object history preservation: PASS by exact target main/source commit identity and exact recreated branch object SHAs
- live GitHub repository settings policy reconciliation: NOT_INCLUDED; the target currently has default GitHub settings and source `REPO_ADMIN_TOKEN` is not authorized for reuse/copy

### E2E

- scenario: NOT_APPLICABLE; migration/governance identity only, no game runtime behavior changed
- result: NOT_APPLICABLE

### Exact-head CI

- final head: pending this checkpoint commit
- trigger source: pull_request #4
- workflow/run/job: pending final generation
- runner assignment: pending
- classification: pending
- result: pending

## Self-review

- exact head: pending final checkpoint commit
- method/reviewer: implementing/coordinating agent
- material findings: pending final full-diff review
- verdict: pending

## Independent review

- required: NO; this task changes repository coordinate identity after an explicitly owner-authorized copy, preserves the same number of routine-write repositories, does not weaken a safety gate and executes no security/protocol/durable-data/runtime mutation
- exact head: NOT_APPLICABLE
- method/auditor: NOT_APPLICABLE
- verdict: NOT_APPLICABLE

## PR and closeout

- PR: #4 Draft
- changed-file review: pending final exact inventory
- unresolved review threads/comments: pending final check
- related bootstrap PR #1: closed without merge
- related cleanup PR #3: closed without merge
- merge: pending exact-head validation
- ownership release: pending lifecycle closeout

## Context checkpoint

```yaml
last_progress: target coordinate replacements applied and validated; temporary workflow removed; final candidate task checkpoint prepared
status: validating
branch: docs/otv2-20260818-target-coordinate-reconciliation
head_sha: a481d2b8b2d24a8dca784ee648ff5b785a6441bf
pr: 4
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request
ci_check_generation: pending_final_checkpoint
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
next_action: inspect the exact final changed-file set and full diff, freeze the resulting head in PR evidence, then complete exact-head workflows and merge if all gates pass
```
