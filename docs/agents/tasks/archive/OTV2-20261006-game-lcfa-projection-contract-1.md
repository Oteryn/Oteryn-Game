# OTV2-20261006-game-lcfa-projection-contract-1

```yaml
task_id: OTV2-20261006-game-lcfa-projection-contract-1
title: "ARCH-LCFA-PROJECTION-CONTRACT-V1: LCFA projection contract revision 2, PLATFORM-LCFA-1 and GAME-LCFA-ENABLE-1 packets"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: cand/lcfa-projection-contract-1
issue: 162
pr: 1870
head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
final_head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
owner: claude-code-session_01WQyZ8BUWVpmDLpSTpXHvn1 (Sol Supervising Architect)
created_at: 2026-10-06
updated_at: 2026-10-06
execution_policy: continuous_progress
owned_paths:
  - docs/contracts/OTERYN_GAME_LIST_CHARACTERS_FOR_ACCOUNT_PROJECTION_V1.md
  - docs/architecture/reviews/OTERYN_GAME_ARCH_LCFA_PROJECTION_CONTRACT_2026-10-06.md
  - docs/agents/tasks/archive/OTV2-20261006-game-lcfa-projection-contract-1.md
public_contracts:
  - docs/contracts/OTERYN_GAME_LIST_CHARACTERS_FOR_ACCOUNT_PROJECTION_V1.md
depends_on: []
blocks: [PLATFORM-LCFA-1, GAME-LCFA-ENABLE-1]
cross_repository_coordination_id: OTV2-20260929-N4P-NATIVE-GATEWAY-LOGIN
external_repositories: []
```

## Outcome

- Control plane D821 (#162, 2026-10-06); owner answers 1a, 2a, 3b. The owner accepts the
  contract revision after the independent review of the frozen head.
- Contract revision 2 keeps the revision 1 wire (LCFA-1 #1330, LCFA-1b #1389). It adds §2.2
  multichannel, §2.3 session-generation fencing, §2.4 owner and authority, the §10 U-LC1..U-LC6
  testing/preproduction rulings, §11 implementation status and §12 acceptance.
- Packets: PLATFORM-LCFA-1 (Oteryn/Oteryn-Platform, under the owner's one-PR write grant after
  acceptance) and GAME-LCFA-ENABLE-1 (this repository). Mode 33a is the release gate (§4).
- Not changed: `CROSS_REPOSITORY_CONTRACT_LOCK.json`. Its stale Game producer status is a control
  plane follow-up.

## Architecture and source of truth

- PROVEN: revision 1 wire, publisher and outbox exist in this repository (LCFA-1 #1330, LCFA-1b
  #1389; migrations 0024 and 0028).
- PROVEN: Oteryn/Oteryn-Platform at 3896bcd has no projection ingestion route and no read model;
  native issuance uses mode 33a (D171).
- PROVEN: owner decisions Q14–Q18 on #162 (comments 5899892092, 5899942821) and D821 answers 1a,
  2a, 3b.
- DERIVED: S = 30 s, clock uncertainty 1 s and push for `testing`/`preproduction` (§10 U-LC1,
  U-LC5); the release values stay open.
- UNKNOWN: the measured delivery lag of a real stack; it decides the release values.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: this task changes only a contract document, a decision document and this record.
It performs no mutation, authorizes no PREPARE/COMMIT and interprets no persisted recovery
evidence. The contract keeps the projection out of authority (§2.4): admission revalidates
ownership.

## Acceptance criteria

- [x] Revision 2 changes no member, type, path, bound or response of revision 1 (contract header).
- [x] Multichannel, fencing and authority rules are normative (§2.2–§2.4).
- [x] §5, §5.1 and §10 state the same U-LC2 and U-LC5 rulings (review round 1).
- [x] The D2 read answers `503` with no entry when the feed is stale or the account is `invalid`
      or below the highest epoch (decision §2 item 4, contract §5.1).
- [x] The decision document carries the mandatory decision test (decision §5).
- [x] Future-dated watermarks never make the feed live, and the epoch restore precondition,
      detection and recovery are stated (review round 2).
- [x] Owner acceptance after independent review of the frozen head (D821 1a): accepted
      2026-10-06, owner acceptance relayed by CP, D828.

## Excluded scope

No code, migration, configuration or deployment. No Oteryn/Oteryn-Platform write. No release-entry
ruling for U-LC1, U-LC5 or U-LC6, no production restore runbook, no PKI. No change to
`CROSS_REPOSITORY_CONTRACT_LOCK.json`.

## Implementation / findings

- Revision 2 of the contract and the two packets, decision `ARCH-LCFA-PROJECTION-CONTRACT-V1`.
- Review round 1 (Codex, review 5425479647 on 36b87e0f):
  - P1 task record missing template sections: accepted and fixed (this record).
  - P2 D2 reads not gated on freshness or epoch: fixed (decision §2 item 4 and its test, contract
    §5.1).
  - P2 §10 conflicts with §5/§5.1: fixed (§5 epoch bullet and §5.1 now cite U-LC2 and U-LC5).
  - Also fixed: the stale registry follow-up line in contract §8 (the limits are registered).
- Review round 2 (Codex, review 5425652116 on 8184c915), both P2 fixed:
  - future-dated watermarks: Platform refuses a watermark ahead of its clock by more than
    `clock_uncertainty` (or with `complete_through` after `observed_at`) with `400`, and a stored
    future `complete_through` reads as stale (decision §2 items 3 and its tests, contract §5.1, §9);
  - restored epochs: contract §5 states the restore precondition (synchronized clock), the
    fail-closed `superseded` detection and the recovery (raise again after a clock fix); the
    production runbook must keep an external epoch fence (§10 U-LC2, decision §3 command help
    and publisher test).

## Validation

### Focused

- `python tools/agents/validate_governance.py`: pass
- `python tools/repository/validate_repository_policy.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass

### Component/integration

- `NOT_APPLICABLE`: documentation only.

### E2E

- `NOT_APPLICABLE`: documentation only; the joint E2E belongs to the packets.

### Exact-head CI

- final head: the FREEZE_SHA report to the control plane
- result: the PR #1870 checks on that head

## Self-review

- exact head: the frozen head
- method/reviewer: Sol Supervising Architect
- material findings: none open after round 2
- verdict: ready for independent review

## Independent review

- required: YES, a public contract revision that the owner accepts (D821 1a)
- exact head: the frozen head
- method/auditor: the control plane's review route
- material findings: round 1 above; later rounds in PR #1870
- verdict: in PR #1870

## PR and closeout

- changed-file review: three owned paths
- unresolved review threads: none after round 2 replies
- related/superseded PRs: none
- protected auto-merge: control plane
- merge commit/result: in PR #1870
- ownership release: on merge

## Context checkpoint

```yaml
last_progress: owner acceptance recorded (D828)
status: completed
branch: cand/lcfa-projection-contract-1
pr: 1870
blocker: null
next_action: control plane freezes the head and requests review
```
