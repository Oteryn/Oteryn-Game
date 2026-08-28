# GitHub-only execution

```yaml
github_only_execution_policy_version: 3.3-oteryn-v2
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

## Connector-triggered event limitations

A successful GitHub connector/API mutation proves only that repository state changed. It does not prove that GitHub Actions emitted or accepted a `push`, `pull_request`, `synchronize`, `reopened` or other workflow event for that exact head.

After the final branch/PR mutation:

1. record the exact head SHA;
2. wait only the configured event grace period;
3. query check suites, check runs and workflow runs for that SHA once;
4. classify no run as `EVENT_SUPPRESSED`;
5. classify a queued/unassigned job with `runner_id = 0`, no runner name and no started steps beyond the configured threshold as `RUNNER_STARVATION`;
6. do not mutate repository content merely to generate another event.

Closing/reopening a PR, creating a replacement PR, rewinding a branch, changing a title or writing a timestamp is not a valid CI recovery mechanism unless that mutation is independently required by the task.

## Trusted manual dispatch recovery

When the required workflow supports manual recovery, the dispatch contract must be exact and fail closed:

- the operator selects the current PR head branch;
- the input names the open pull request number;
- the input names the full 40-character expected head SHA;
- the workflow verifies that the selected ref resolves to that SHA;
- the workflow verifies that the PR is open, targets `main`, uses a same-repository head and still has the expected SHA;
- the workflow checks out and validates exactly that SHA;
- the required job/check name remains unchanged.

A manual dispatch that does not validate PR identity and exact head cannot substitute for the protected exact-head check.

If the active connector does not expose workflow dispatch or run cancellation, this is a valid external-operation blocker. Record the exact UI/API action, branch, PR, SHA and expected check; configure protected auto-merge when every non-CI gate is complete; stop without weakening the ruleset.

## Temporary workflow rule

A workflow removed before merge cannot prove the removal commit. Final exact-head proof must come from a retained trusted workflow, trusted-base reusable/dispatch workflow checking out the exact SHA, or another repository-approved immutable validator.

Do not add `pull_request_target` execution of PR-controlled code merely to work around suppressed connector events. Any status-reporting workflow must preserve least privilege and must not allow an untrusted PR to mint its own required success context.

## Merge

Protected auto-merge may be configured when the exact head is frozen, all non-CI gates are complete and protection guarantees required checks. Otherwise direct squash merge is allowed only after every exact-head check, audit, E2E, review and ownership gate passes.

Never force, bypass, administratively override or merge a moved head.

## Valid blockers

A GitHub-only task may stop for an unavailable connector operation, permission, secret, protected environment, physical device, architecture decision, anti-stall limit, suppressed workflow event, runner starvation or unauthorized live operation. Record the exact unavailable action, attempted route, current branch/PR/head, validation, CI classification and one next action.
