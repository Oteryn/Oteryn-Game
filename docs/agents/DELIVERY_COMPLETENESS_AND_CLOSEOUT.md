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
7. run mandatory full-diff self-review against that exact final head;
8. run an independent review/audit on the same exact head when required by the risk policy, owner or governing contract;
9. run required checks on the same unchanged head;
10. verify no unresolved review threads/requested changes;
11. reconcile related/superseded PRs;
12. verify rollback or safe failure path where relevant.

## Final-head freeze and evidence placement

The delivery head is frozen only after implementation, task metadata, PR title/body and known scope declarations are complete.

After the freeze:

- do not add a task/checkpoint/timestamp commit merely to record a review/audit verdict, run ID, CI state or merge readiness;
- record review/audit and CI evidence in PR reviews, workflow runs, artifacts or another immutable evidence channel;
- do not close/reopen, rewind, amend, force-push or replace the PR merely to regenerate a required check;
- move the head only for a material repair, then repeat final diff self-review, applicable independent review and exact-head CI;
- treat absent events and unassigned runners as classified infrastructure states, not as justification for content churn.

A commit cannot contain its own SHA. Do not create a self-referential follow-up commit merely to populate an exact-head field in the repository task record.

Post-merge archive movement may use a separate bounded closeout change when the merge commit was unknowable before the delivery PR merged.

## Review and audit policy

### Self-review

Self-review is mandatory for every delivery. It must challenge, not merely summarize, the implementer's result. Review at minimum:

- omitted paths/layers;
- architecture and ownership violations;
- security, concurrency and failure paths;
- unsupported claims;
- stale validation after head changes;
- cross-repository/current-vs-target mismatches;
- dead temporary workflows/instrumentation;
- unresolved compatibility/rollout holds.

Record reviewer identity/method, exact head and material findings. `PASS` requires zero open material findings.

### Independent review

A second genuinely independent reviewer is required when the trusted-base risk policy, owner or owning contract requires it. This includes high-risk authority/security/protocol/durable-data/production changes, safety-reducing governance changes and cases where material uncertainty/complexity makes self-review insufficient.

Independent review may be performed by a qualified human, a separate non-authoring agent/session, Codex or a dedicated independent audit workflow. Codex is not mandatory by name and should not be invoked routinely for low-risk work.

A review by the same agent/session that implemented or materially authored the change is self-review, not independent review. If independence is required and no genuine independent mechanism is available, the task is blocked rather than silently downgraded.

## E2E classification

- `PASS` — named scenario executed against named build/environment and observed required result.
- `NOT_APPLICABLE` — concrete explanation why task has no executable user/runtime outcome.
- `BLOCKED` — exact unavailable dependency/environment and nearest safe evidence recorded.
- `FAIL` — scenario executed and acceptance not met.

Do not call synthetic unit/integration tests real E2E.

## Merge gate

Merge only when exact final head is unchanged and all applicable repository gates pass. A later build-affecting commit invalidates prior build/E2E evidence. A later docs-only commit still requires applicable governance/document validation on the new head.

A required check cannot be replaced by a local command, PR comment or manually asserted success. Trusted manual dispatch is acceptable only when it validates the open PR and exact frozen head and produces the repository-required context on that SHA.

Mandatory self-review must be clean. When independent review is required by the risk policy, that review must also be clean on the exact final head.

## Closeout

After merge:

- record merge commit and resulting state;
- archive task and release owned paths/leases;
- close/update linked Issue/programme barriers;
- mark superseded PRs intentionally terminal;
- preserve evidence and one next programme action, if any;
- do not leave an active task merely because the implementation PR merged.
