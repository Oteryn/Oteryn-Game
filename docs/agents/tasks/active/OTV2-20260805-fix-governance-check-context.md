# OTV2-20260805-fix-governance-check-context

```yaml
task_id: OTV2-20260805-fix-governance-check-context
title: Align governance job name with required ruleset context
mode: REPAIR
status: implementing
repository: blakinio/Oteryn-v2
base_branch: main
branch: fix/governance-check-context-20260805
pr: null
base_sha: 30324872af421d0d2bdcb91b360a76a3d44a2592
head_sha: null
owner: repository-governance-repair
created_at: 2026-08-05T16:06:00+02:00
updated_at: 2026-08-05T16:06:00+02:00
execution_budget_minutes: 30
large_budget_reason: null
owned_paths:
  - .github/workflows/agent-governance.yml
  - docs/agents/tasks/active/OTV2-20260805-fix-governance-check-context.md
public_contracts:
  - .github/repository-policy.json
  - .github/workflows/agent-governance.yml
depends_on:
  - PR #23 repository governance hardening
blocks:
  - all protected main-branch pull request merges
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Make the GitHub Actions check-run name exactly match the active ruleset context `Agent governance / validate` without weakening or bypassing the ruleset.

## Evidence

- The active `Protect main` ruleset requires `Agent governance / validate`.
- The current workflow publishes the job check as `validate` because the job has no explicit `name`.
- Successful reruns remain reported as `validate`, leaving the required context permanently `expected` and blocking every merge.

## Acceptance criteria

- [ ] The governance job publishes the exact required check-run name.
- [ ] Existing governance validation steps and permissions remain unchanged.
- [ ] Agent governance passes on the exact PR head.
- [ ] The PR becomes mergeable under the active ruleset with no bypass.
- [ ] No product or architecture document changes.

## Excluded scope

- no weakening, removal or bypass of the main ruleset;
- no change to required review or squash policy;
- no product/runtime/architecture change;
- no external repository change.

## Validation

Pending exact-head PR validation.

## Context checkpoint

```yaml
last_progress: Confirmed the active ruleset expects Agent governance / validate while the workflow publishes validate.
status: implementing
branch: fix/governance-check-context-20260805
head_sha: null
pr: null
blocker: null
next_action: Add the explicit governance job name and validate it on a pull request.
```
