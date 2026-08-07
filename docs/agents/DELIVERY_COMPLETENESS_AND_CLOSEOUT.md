# Delivery completeness and closeout

## Completion principle

A task is complete only when the requested observable outcome is delivered, not merely when code or documentation exists.

Classify every task as one or more of:

- architecture/contract;
- implementation;
- audit;
- repair;
- migration;
- operational/release;
- research/discovery.

Apply only relevant layers, but do not omit a required layer to reduce effort.

## Completeness layers

Where applicable verify:

- durable model/persistence;
- authoritative server/domain behavior;
- protocol/API contract;
- client/UI observable behavior;
- integration and failure behavior;
- tests and exact-head CI;
- E2E/user-observable acceptance;
- security/limits/privacy;
- documentation, migration and rollback;
- operational readiness and diagnostics.

For docs-only governance/architecture tasks, runtime layers may be `NOT_APPLICABLE` only with a concrete reason.

## Required outcome evidence

Before readiness:

1. inspect the full changed-file list and diff;
2. map each acceptance criterion to evidence;
3. run focused/component checks;
4. run required real or synthetic E2E with truthful classification;
5. prepare all known task and PR closeout metadata;
6. freeze and record the exact final head in immutable PR/task-tracker evidence;
7. run an independent audit against that exact final diff;
8. run required checks on the same unchanged head;
9. verify no unresolved review threads/requested changes;
10. reconcile related/superseded PRs;
11. verify rollback or safe failure path where relevant.

## Final-head freeze and evidence placement

The delivery head is frozen only after implementation, task metadata, PR title/body and known scope declarations are complete.

After the freeze:

- do not add a task/checkpoint/timestamp commit merely to record an audit verdict, run ID, CI state or merge readiness;
- record audit and CI evidence in PR reviews, workflow runs, artifacts or another immutable evidence channel;
- do not close/reopen, rewind, amend, force-push or replace the PR merely to regenerate a required check;
- move the head only for a material repair, then repeat final diff review, audit and exact-head CI;
- treat absent events and unassigned runners as classified infrastructure states, not as justification for content churn.

A commit cannot contain its own SHA. Do not create a self-referential follow-up commit merely to populate an exact-head field in the repository task record.

Post-merge archive movement may use a separate bounded closeout change when the merge commit was unknowable before the delivery PR merged.

## Independent audit

The audit must challenge, not repeat, the implementer's conclusion. Review at minimum:

- omitted paths/layers;
- architecture and ownership violations;
- security, concurrency and failure paths;
- unsupported claims;
- stale validation after head changes;
- cross-repository/current-vs-target mismatches;
- dead temporary workflows/instrumentation;
- unresolved compatibility/rollout holds.

Record auditor identity/method, exact head and material findings. `PASS` requires zero open material findings.

## E2E classification

- `PASS` — named scenario executed against named build/environment and observed required result.
- `NOT_APPLICABLE` — concrete explanation why task has no executable user/runtime outcome.
- `BLOCKED` — exact unavailable dependency/environment and nearest safe evidence recorded.
- `FAIL` — scenario executed and acceptance not met.

Do not call synthetic unit/integration tests real E2E.

## Merge gate

Merge only when exact final head is unchanged and all repository gates pass. A later build-affecting commit invalidates prior build/E2E evidence. A later docs-only commit still requires applicable governance/document validation on the new head.

A required check cannot be replaced by a local command, PR comment or manually asserted success. Trusted manual dispatch is acceptable only when it validates the open PR and exact frozen head and produces the repository-required context on that SHA.

## Closeout

After merge:

- record merge commit and resulting state;
- archive task and release owned paths/leases;
- close/update linked Issue/programme barriers;
- mark superseded PRs intentionally terminal;
- preserve evidence and one next programme action, if any;
- do not leave an active task merely because the implementation PR merged.
