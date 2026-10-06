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
- Owner questions §7 items 1–4 are routed through the control plane. Recorded answers 1a
  (D831, against the final §2 owned-path list; replaces D824 1a) and 2a (D824); items 3 and 4
  are open.

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
- [x] The trust command fences the high-water directory to the run's own per-run directory, and
      the shared-guard paths are in the write grant (review round 2); it accepts only the
      retained per-run SQLite file so the directory is bound to the current run (round 3).
- [x] Both commands refuse a symlinked or non-canonical per-run database directory before any
      write, with a refusal test for each command (P1 4194718086).
- [x] The mandatory decision test is answered (decision §6).
- [x] Owner answer 1a (D831, final §2 path list) and 2a (D824) are recorded; the Synology shape of Option B is noted as not
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
- Review round 2 (Codex, review 5425633904 on 06ff7a77), both P1 accepted and fixed:
  - the shared guard was outside the write grant: the owned paths now list the new
    `DisposableNativeStore.php` and `NativeTopologyRegistry.php` (call-site change only);
  - the high-water directory was unfenced: the trust command now requires it to be a canonical,
    non-symlink directory directly beneath the run's own per-run directory before any write, with
    refusal tests.
- Review round 3 (Codex, review 5426212787 on e089e18d), one P1 accepted and fixed: with
  `:memory:` (or the loopback MySQL store) the high-water directory had no binding to the current
  run. The trust command now accepts only the retained per-run SQLite file and requires the
  directory beneath that file's own per-run directory, with refusal tests.
- Also: D824 answers recorded; the control plane's Synology proposal is noted as the likely
  Option B shape with its guard tension, not as a decision.
- Codex P1 4193785205 (answer 1a predated `DisposableNativeStore.php` and the
  `NativeTopologyRegistry.php` guard call): the control plane asked the owner again with the
  final §2 path list; the owner answered 1a (D831) and confirmed it directly to this architect
  session. Decision status, §2, §7 and §8 now rest answer 1a on D831 and bound the Platform write
  to exactly that list.
- Codex P1 4194207250 (status said nothing takes effect until every §7 item is answered, while §8
  relies on item 1): status, §7 and §8 now say each §7 item takes effect independently once its
  answer is recorded; 1a and 2a are in effect, items 3 and 4 grant nothing.
- Codex P1 4194718086 on e5b6d970: the shared predicate checks only the database file, so a
  per-run directory that is a symlink to a persistent directory passes it, and the route command
  could publish into the target. Fixed with a run-directory check in `DisposableNativeStore` that
  both commands call before any write: the per-run directory is not a symlink and is canonical
  directly beneath the canonical system temporary root, and the database file is canonical inside
  it. A refusal test covers each command. `isolatedConnection()` and the D831 path list are
  unchanged, so answer 1a still covers the Platform write.

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
- material findings: none open after the independent-effect fix
- verdict: ready for independent review

## Independent review

- required: YES, an authority request that shapes a Platform trust-registration path
- exact head: the frozen head
- method/auditor: the control plane's review route
- material findings: round 1 above; later rounds in PR #1871
- verdict: in PR #1871

## PR and closeout

- changed-file review: two owned paths
- unresolved review threads: none after the run-directory reply
- related/superseded PRs: none
- protected auto-merge: control plane
- merge commit/result: in PR #1871
- ownership release: on merge

## Context checkpoint

```yaml
last_progress: run-directory check for both commands (P1 4194718086)
status: completed
branch: cand/preprod-route-publish-auth-1
pr: 1871
blocker: null
next_action: control plane freezes the head and requests review
```
