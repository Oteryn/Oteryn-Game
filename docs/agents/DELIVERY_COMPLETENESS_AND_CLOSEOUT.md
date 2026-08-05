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
5. run an independent audit against the exact final diff;
6. run required checks on the exact final head;
7. verify no unresolved review threads/requested changes;
8. reconcile related/superseded PRs;
9. update task, ADRs/contracts and migration notes;
10. verify rollback or safe failure path where relevant.

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

## Closeout

After merge:

- record merge commit and resulting state;
- archive task and release owned paths/leases;
- close/update linked Issue/programme barriers;
- mark superseded PRs intentionally terminal;
- preserve evidence and one next programme action, if any;
- do not leave an active task merely because the implementation PR merged.
