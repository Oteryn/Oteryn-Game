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
blocks: [PLATFORM-NATIVE-PREPROD-OPS-1]
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
  - current route: the disposable stack of the merged RUNBOOK-1, which calls the Platform methods
    through `php -r` with no Platform change.
  - A (deferred, pending the owner's ruling): the same stack, plus one Platform PR adding two
    commands limited to testing and preproduction. A backlog entry with no frozen design; the
    design and authority are reassessed in an amendment if the owner proceeds under 1a or a §6
    trigger occurs.
  - B (deferred, undecided): persistent private preproduction. A backlog entry with subject,
    safety constraints and a reopening trigger (step 7 scheduled and needing an environment that
    outlives one run); no technology or topology is chosen, and no answer authorizes it now.
  - C: public staging, rejected.
- Owner questions §7 items 1–4 are routed through the control plane. Recorded answers 1a
  (D831, against the owned-path list of its question; replaces D824 1a) and 2a (D824) stay
  recorded, but no Platform PR is allocated under them while Option A is deferred pending the
  owner's ruling (deferral versus proceeding under 1a); items 3 and 4 are open and only confirm
  a backlog entry and a refusal, so no answer to them grants authority.

## Architecture and source of truth

- PROVEN: Oteryn/Oteryn-Platform `origin/main` 3896bcd, read only: `deploy/synology/` with
  `APP_ENV=staging`, `NativeTopologyRegistry::publishRouteForPreproduction` and
  `isolatedConnection()`, `NativeSigningTrustRegistry::publishTrustedKey` (default connection, no
  store guard), the commands `game-auth:world:ensure` and `game-auth:native-topology:issue`.
- PROVEN: Platform native gateway login contract §14 steps 6 and 7; `ARCH-LOGIN-FIRST-PACKETS-V1`
  §2.7 (RUNBOOK-1 plan).
- DERIVED: a persistent MariaDB in `preproduction` is refused by the unchanged guard.
- PROVEN: Game `main` 6560803cf (RUNBOOK-1 files unchanged since 36c586516): RUNBOOK-1 (`tools/qualification/login_local/run.sh:163-182`)
  calls `issueForPreproduction`, `publishRouteForPreproduction` and `publishTrustedKey` through
  `php -r` in its throwaway `APP_ENV=preproduction` Platform container; Platform 3896bcd has no
  route or trust command.
- DERIVED: that runbook's Platform store is MariaDB `oteryn_s3a` on host `db`
  (`tools/qualification/wp5_s3a/compose.yml:37-40`), which `isolatedConnection()` refuses in
  `preproduction` (`NativeTopologyRegistry.php:162-168`), so its topology step fails (F7).
- UNKNOWN: Platform hosts outside the repository; whether RUNBOOK-1 has run anywhere (no
  `LOGIN_LOCAL_RESULT` evidence on Game `main`); whether the rest of the Platform stack runs on
  the per-run SQLite profile.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: this task changes only a decision document and this record. It performs no
mutation and grants no authority. If Option A goes ahead, its Platform PR touches trust
registration and gets its own independent review.

## Acceptance criteria

- [x] Findings F1–F7 carry evidence classes and file:line citations (decision §1).
- [x] The safety facts from review rounds 1–7 (unguarded trust write and its high-water lock
      before the transaction, permanent issuance, the run-directory check, the `testing` stores
      the guard admits) are kept as constraints for a later reassessment (decision §2, F3–F5).
- [x] Option A is a backlog entry with no frozen class, command, flag, test or path design, and
      no pre-allocated Platform write (P1 4197089671).
- [x] Whether RUNBOOK-1 has run is UNKNOWN, not PROVEN (P2 4197089687).
- [x] The mandatory decision test is answered (decision §6).
- [x] Owner answer 1a (D831, the path list of its question, kept in §7) and 2a (D824) are
      recorded (decision §7).
- [x] Option B is a backlog entry with no chosen technology, topology, guard change or workflow,
      and no §7 answer authorizes its scope now (P1 4197646953).
- [x] F6 and §6 reassessed against the merged RUNBOOK-1: Option A deferred, pending the owner's
      ruling, with reopen triggers; no new Platform command authorized now (P1 4196578216).
- [ ] The owner's ruling on Option A (deferral versus proceeding under 1a).
- [ ] Owner answers to items 3 and 4 (neither grants authority).

## Excluded scope

No Platform write, code, migration, deployment, secret, runner, Cloudflare or database change. No
guard relaxation. No decision on Option B's design, technology, topology or authority.

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
- Review round 6, Codex P1 4194718086 (open on 40519cc1) and P1 4195238904 on 40519cc1
  (issuance writes permanent IDs through a symlinked per-run directory before the route command
  refuses). Re-read at Platform `origin/main` 3896bcd. The round-5 premise was wrong.
  `isolatedConnection()` already refuses a symlinked per-run directory: it requires
  `realpath(dirname($database))` to equal `dirname($database)` and its parent to equal
  `realpath(sys_get_temp_dir())` (`NativeTopologyRegistry.php:181-189`).
  `NativeTopologyRegistryTest.php:269,293-297` proves the refusal for issuance and readback.
  - Issuance is fenced: `IssueNativeTopology.php:29` calls `issueForPreproduction`, which calls
    the guard (`:16`; environment `:152`) before its transaction (`:18`).
  - F3 is corrected. The run-directory check is now named as part of the shared guard, which
    moves unchanged into `DisposableNativeStore`. So issuance, readback, the route command and
    the trust command all apply it. There is no separate command-only check.
  - Tests: issuer refusal rows (symlinked per-run directory; `local`, `staging`, `production`;
    a non-disposable store) with nothing written.
  - `isolatedConnection()` semantics and the D831 path list are unchanged.
- Review round 7, Codex P1 4195923710 on c6127d3d (in `testing` the guard admits the loopback
  `oteryn_concurrency` MariaDB, so issuance and the route command could write there, against the
  per-run-SQLite blast radius). Confirmed in `isolatedConnection()` (MySQL clause; `:memory:` is
  admitted in `testing` too).
  - Option A now runs only with `APP_ENV=preproduction`, in CI too. There the unchanged guard
    admits only the retained per-run SQLite file, so issuance is fenced to it without touching
    `IssueNativeTopology.php`, which is outside the D831 list.
  - Both new commands require the retained per-run SQLite file in `testing` as in
    `preproduction` (a `DisposableNativeStore` check, formerly trust-command only).
  - Tests: issuer refusal in `preproduction` for `:memory:` and the loopback store; each new
    command refuses both in `testing`; the happy path runs in `preproduction`.
  - The issue command in `testing` still admits those test stores; that is unchanged Platform
    behaviour that Option A does not use, and no route or trust state can follow it.
- Codex P1 4196578216 (after the merge of `main` 3d297c9d, RUNBOOK-1 calls the three methods
  from the shell and says nothing is pending PLATFORM-NATIVE-PREPROD-OPS-1, so F6 and §6 "Must
  decide now? YES" were stale). Accepted; option (b).
  - F6 rewritten from Game `main` 36c586516: the runbook reaches the methods through `php -r`
    (`run.sh:163,166,170,182`), the same pattern as node_boot; the commands do not exist at
    Platform 3896bcd; no protected preproduction environment exists where that path is
    inadmissible.
  - New F7: the runbook's Platform store is MariaDB, which the guard refuses in `preproduction`,
    so its first run fails at `run.sh:166`. The Option A commands would not fix it. The fix is
    RUNBOOK-1-FU work in Game paths.
  - §6: Option A deferred; nothing is blocked on it. Reopen triggers listed (protected or
    persistent preproduction, a CI gate needing stable flags, a security finding on the
    unguarded trust write, Platform removing the methods). Status, §0, §2, §5, §7 and §8 follow;
    answers 1a (control plane D831) and 2a (D824) stay recorded, with no Platform PR allocated.
- Codex P1 4197089671 (a `NO` decision still froze Option A: classes, flags, tests and a write
  grant in §2) and P2 4197089687 (the absence of a run was classed PROVEN on README wording that
  covers one environment). Both accepted.
  - §2 is now a backlog entry: subject, status, the safety constraints from F2–F5 for a later
    reassessment, and authority. The design is reassessed in an amendment if the owner proceeds
    under 1a or a §6 trigger occurs. The D831 path list moves to §7 as the record of what 1a
    answered, not as an allocation. §5, §6, §8 and the status follow.
  - F6: whether the runbook has run is UNKNOWN; Game `main` has no `LOGIN_LOCAL_RESULT` evidence,
    and RUNBOOK-1-FU (D834) is to record it. F7's failure stays DERIVED.
  - The doc reads "Option A deferred, pending the owner's ruling"; the owner was asked, through
    the control plane, about deferral versus 1a, and nothing records it as decided.
  - Self-review on merged `main` 6560803cf and Platform 3896bcd: F1 named the wrong runner (the
    staging deploy runs on `platform-runners`/`oteryn-platform`, environment `synology-staging`;
    `oteryn-staging` is the compose project); F3 omitted the refusal of other drivers; F4 now
    states that the trust write creates its high-water lock file before its transaction; F7 notes
    that `game-auth:world:ensure` writes before issuance is refused; Option B no longer refers to
    removed Option A flags and credential lists; every factual claim carries a file:line.
- Codex P1 4197646953 (Option B, also `Must decide now? NO`, still preselected Synology Compose,
  MariaDB, Redis, networking, ports, a guard relaxation and a workflow, and §7 answer 3a would
  have authorized that scope). Accepted.
  - §3 is now a backlog entry: subject, status, safety constraints from F1, F3–F5, and a
    reopening trigger (step 7 scheduled and needing an environment that outlives one run). The
    owner's Synology preference and the control plane's proposal are history only.
  - §7 item 3 now has only a non-authorizing option (keep the backlog entry); any answer recorded
    to an earlier "authorize now" form is history only. Status, §0, §5, §6 and §8 follow.
  - Sweep for other `NO` items that froze a design: §6 item 5 no longer names a Synology shape;
    the §6 Option A trigger cites §3, not item 3; F7's SQLite store fix is marked as one example,
    not a choice. Answers 1a and 2a and the D831 path list stay recorded as given.

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
- material findings: none open after the P1 4197089671, P2 4197089687 and P1 4197646953 fixes
- verdict: ready for independent review

## Independent review

- required: YES, an authority request that shapes a Platform trust-registration path
- exact head: the frozen head
- method/auditor: the control plane's review route
- material findings: round 1 above; later rounds in PR #1871
- verdict: in PR #1871

## PR and closeout

- changed-file review: two owned paths
- unresolved review threads: none after the replies to 4197089671, 4197089687 and 4197646953
- related/superseded PRs: none
- protected auto-merge: control plane
- merge commit/result: in PR #1871
- ownership release: on merge

## Context checkpoint

```yaml
last_progress: P1 4197646953 (§3 Option B reduced to a backlog entry; §7 item 3 cannot authorize; sweep of other NO items)
status: completed
branch: cand/preprod-route-publish-auth-1
pr: 1871
blocker: null
next_action: control plane freezes the head and requests review
```
