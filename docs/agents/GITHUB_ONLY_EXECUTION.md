# GitHub-only execution

```yaml
github_only_execution_policy_version: 3.2-oteryn-v2
```

The absence of Codex or a local terminal is not itself a blocker. Use the GitHub connection for repository reads/writes and GitHub Actions for permitted remote execution and validation.

## Authority and safety

- Work only in repositories explicitly authorized by the owner/trusted policy.
- Governance on an unmerged branch cannot expand authority.
- Repository change authority is distinct from production/live-operation authority.
- Never use Actions to bypass branch/review/environment/secret protection.
- Temporary databases/services must be isolated and disposable.
- Never expose secrets in logs, artifacts, task records or PRs.

## Execution pattern

1. Inspect exact repository/task/PR/workflow state.
2. Use a dedicated task branch.
3. Implement the smallest complete result.
4. Prefer existing workflows; add a minimal temporary validation workflow only when existing trusted workflows cannot prove the task.
5. On failure inspect logs, form a hypothesis, make one targeted repair and run the smallest proving check.
6. Preserve required artifacts/evidence.
7. Inspect only related/blocking PRs unless broader cleanup is separately scoped.
8. Continue through safe steps rather than returning only a plan.

## Temporary workflow rule

A workflow removed before merge cannot prove the removal commit. Final exact-head proof must come from a retained trusted workflow, trusted-base reusable/dispatch workflow checking out the exact SHA, or another repository-approved immutable validator.

## Merge

Protected auto-merge may be configured when the exact head is frozen, all non-CI gates are complete and protection guarantees required checks. Otherwise direct squash merge is allowed only after every exact-head check, audit, E2E, review and ownership gate passes.

Never force, bypass, administratively override or merge a moved head.

## Valid blockers

A GitHub-only task may stop for an unavailable connector operation, permission, secret, protected environment, physical device, architecture decision, anti-stall limit or unauthorized live operation. Record the exact unavailable action, attempted route, current branch/PR/head, validation and one next action.
