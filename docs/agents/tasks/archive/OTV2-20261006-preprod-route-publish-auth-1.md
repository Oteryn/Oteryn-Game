# OTV2-20261006-preprod-route-publish-auth-1

```yaml
task_id: OTV2-20261006-preprod-route-publish-auth-1
title: "ARCH-PREPROD-ROUTE-PUBLISH-AUTH-V1: authority request for the preproduction route-publish operator path"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: cand/preprod-route-publish-auth-1
issue: 162
pr: 1871
head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
final_head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
owner: claude-code-session_01WQyZ8BUWVpmDLpSTpXHvn1 (Sol Supervising Architect)
created_at: 2026-10-06
updated_at: 2026-10-06
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_ARCH_PREPROD_ROUTE_PUBLISH_AUTH_2026-10-06.md
  - docs/agents/tasks/archive/OTV2-20261006-preprod-route-publish-auth-1.md
public_contracts: []
depends_on: []
blocks: [PLATFORM-NATIVE-PREPROD-OPS-1, RUNBOOK-1]
cross_repository_coordination_id: OTV2-20260929-N4P-NATIVE-GATEWAY-LOGIN
external_repositories: []
```

## Outcome

- Control plane D821 item 2 (#162, 2026-10-06). This is an authority request only. It grants
  nothing until the owner answers its §7.
- Findings from Platform `origin/main` 3896bcd:
  - no Platform preproduction deployment exists (Synology is public `APP_ENV=staging`);
  - `isolatedConnection()` admits only disposable stores;
  - neither `publishRouteForPreproduction` nor `publishTrustedKey` has an operator command;
  - issued WorldId and ChannelId are permanent.
- Options:
  - A (recommended): a disposable stack, plus one Platform PR adding two commands limited to
    testing and preproduction.
  - B: persistent private preproduction, deferred until rollout step 7.
  - C: public staging, rejected.
- Owner questions §7 items 1–4 are routed through the control plane. Recorded answers 1a and
  2a (D824); items 3 and 4 are open.

## Architecture and source of truth

- PROVEN: Oteryn/Oteryn-Platform `origin/main` 3896bcd, read only: `deploy/synology/` with
  `APP_ENV=staging`, `NativeTopologyRegistry::publishRouteForPreproduction` and
  `isolatedConnection()`, `NativeSigningTrustRegistry::publishTrustedKey` (default connection, no
  store guard), the commands `game-auth:world:ensure` and `game-auth:native-topology:issue`.
- PROVEN: Platform native gateway login contract §14 steps 6 and 7; `ARCH-LOGIN-FIRST-PACKETS-V1`
  §2.7 (RUNBOOK-1 plan).
- DERIVED: a persistent MariaDB in `preproduction` is refused by the unchanged guard.
- UNKNOWN: Platform hosts outside the repository; the toolchain of the `oteryn-synology-game`
  runner; RUNBOOK-1's final shape.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: this task changes only a decision document and this record. It performs no
mutation and grants no authority. The Platform PR it requests touches trust registration and gets
its own independent review under the item 1 grant.

## Acceptance criteria

- [x] Findings F1–F6 carry evidence classes (decision §1).
- [x] The trust-key command applies the same disposable-store guard as `isolatedConnection()`
      before any write, with a refusal test for a non-disposable store (decision §2).
- [x] The mandatory decision test is answered (decision §6).
- [x] Owner answers 1a and 2a are recorded; the Synology shape of Option B is noted as not
      decided (decision §3, §7).
- [ ] Owner answers to items 3 and 4.

## Excluded scope

No Platform write, code, migration, deployment, secret, runner, Cloudflare or database change. No
guard relaxation. No decision on Option B or its Synology shape.

## Implementation / findings

- Review round 1 (Codex, review 5425468749 on 6affd36a), all P1, all accepted and fixed:
  - trust writes not fenced to the disposable store: the command now applies the shared,
    unchanged `isolatedConnection()` predicate before any write, and the tests cover a
    non-disposable `preproduction` store;
  - mandatory decision test missing: added as decision §6;
  - findings not classified: F1–F6 now PROVEN/DERIVED/UNKNOWN.
- Also: D824 answers recorded; the control plane's Synology proposal is noted as the likely
  Option B shape with its guard tension, not as a decision.

## Validation

### Focused

- `python tools/agents/validate_governance.py`: pass
- `python tools/repository/validate_repository_policy.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass

### Component/integration

- `NOT_APPLICABLE`: documentation only.

### E2E

- `NOT_APPLICABLE`: documentation only.

### Exact-head CI

- final head: the FREEZE_SHA report to the control plane
- result: the PR #1871 checks on that head

## Self-review

- exact head: the frozen head
- method/reviewer: Sol Supervising Architect
- material findings: none open after round 1
- verdict: ready for independent review

## Independent review

- required: YES, an authority request that shapes a Platform trust-registration path
- exact head: the frozen head
- method/auditor: the control plane's review route
- material findings: round 1 above; later rounds in PR #1871
- verdict: in PR #1871

## PR and closeout

- changed-file review: two owned paths
- unresolved review threads: none after round 1 replies
- related/superseded PRs: none
- protected auto-merge: control plane
- merge commit/result: in PR #1871
- ownership release: on merge

## Context checkpoint

```yaml
last_progress: review round 1 fixed
status: completed
branch: cand/preprod-route-publish-auth-1
pr: 1871
blocker: null
next_action: control plane freezes the head and requests review
```
