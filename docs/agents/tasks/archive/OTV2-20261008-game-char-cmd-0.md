# OTV2-20261008-game-char-cmd-0

```yaml
task_id: OTV2-20261008-game-char-cmd-0
title: "GAME-CHAR-CMD-0: Character Authority command wire v1 candidate (CreateCharacter, TransferCharacterOwnership)"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/game-char-cmd-0-20261008
issue: 1622
pr: 1936
head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
final_head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
owner: claude-code-session_01MStsmUQkSquvrbq3JZLJij (P4 hard worker)
created_at: 2026-10-08
updated_at: 2026-10-08
execution_policy: continuous_progress
owned_paths:
  - docs/contracts/OTERYN_GAME_CHARACTER_AUTHORITY_COMMANDS_V1.md
  - docs/contracts/CROSS_REPOSITORY_CONTRACT_LOCK.json
  - docs/agents/tasks/archive/OTV2-20261008-game-char-cmd-0.md
public_contracts:
  - docs/contracts/OTERYN_GAME_CHARACTER_AUTHORITY_COMMANDS_V1.md
depends_on: []
blocks: [P6, P7]
cross_repository_coordination_id: GAME-CHAR-CMD-0
external_repositories: []
```

## Outcome

- Packet P4 (rows C7, C8) of the #1622 plan (comment 6056637549); Platform mirror
  Oteryn/Oteryn-Platform#1476.
- Candidate contract `oteryn-game-character-authority-commands-v1`, revision 1, not accepted.
- Owner rulings applied: D963 Q2=A (Platform creation and transfer stay disabled until this
  contract and its Game implementation exist), D965 Q5=A (creation on the Platform account page;
  the client entry opens it; no client form).
- Lock entry `GAME-CHAR-CMD-0` in `PENDING_CANONICAL_MERGE` naming PR #1936.

## Architecture and source of truth

- PROVEN: `CHARACTER_AUTHORITY_PLATFORM_BOUNDARY.md` (accepted) §5, §6, §6.1, §10, §13, §16
  define the semantics; this contract adds only the wire, idempotency, receipts and codes.
- PROVEN: the bootstrap intent decision (2026-09-23) reserves `PLATFORM_USER_CREATE` for a
  separate explicit contract and fixes the intent model, prerequisites and 300 s TTL.
- PROVEN: LCFA v1 rev 2 (D828) fixes the Game→Platform mTLS client, bounds and projection
  revisions; LCFA §3 Decision P1 makes an inbound Game listener a separate architecture decision,
  so v1 is Game-initiated pull and needs no new decision.
- DERIVED: out-of-order `source_revision` is accepted for the user variant (contract §5.3); flagged
  for acceptance review.
- UNKNOWN: U-CC1..U-CC6 (contract §13).

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: docs only. The contract specifies future authority writes and fencing but this
task performs no mutation, authorizes no implementation and enables nothing.

## Acceptance criteria

- [x] Wire for both commands, idempotency keys, receipts and result codes (contract §4, §5, §7).
- [x] Name rules via CHAR-NAME-1 (§4.2, §6.1).
- [x] mTLS/auth profile consistent with LCFA (§3).
- [x] Session-generation fencing for transfer (§6.2).
- [x] Conformance scenarios covering boundary §16 (§11).
- [x] Client deep link and ListCharactersForAccount refresh (§9).
- [x] Contract lock entry (`PENDING_CANONICAL_MERGE`).
- [ ] Owner acceptance after independent review of the frozen head.

## Excluded scope

No code, migration, configuration, topology or Oteryn/Oteryn-Platform change. No resource-limit
registration (the implementation packet registers §12). No enablement.

## Implementation / findings

- New contract document and lock entry; no other file changed.
- Review round 1 (external review of the first frozen head, six threads) fixed in one push:
  future-dated intents rejected, resumable pending list, unknown-command receipt shape,
  configurable creation-page URL, shared account portfolio lock with a fixed lock order, and
  `operation_id` unique per issuer across commands; conformance scenarios 19 to 23 added.
- Review round 2: pending-list response bound raised to 8192 bytes (worst full page 5602) and
  list request to 512 bytes; compound continuation key `(source_revision, operation_id)`.
- Review round 3: `recovery_generation` on every receipt and restore reconciliation (§5.4,
  OPERABILITY §3 ruling 10); housing disposition proof before rebinding (§6.4, EXP-HOUSES-01
  §§13.5, 14.7) with `CHAR_CMD_HOUSING_NOT_PREPARED`; scenarios 24 and 25; unknowns U-CC7, U-CC8.

## Validation

### Focused

- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass

### Component/integration

- `NOT_APPLICABLE`: documentation only.

### E2E

- `NOT_APPLICABLE`: documentation only.

### Exact-head CI

- final head: the FREEZE_SHA report to the control plane
- result: the PR #1936 checks on that head

## Self-review

- exact head: the frozen head
- method/reviewer: task writer, against boundary §1–§16, LCFA and the bootstrap intent decision
- material findings: none open
- verdict: ready for independent review

## Independent review

- required: YES, a new public cross-repository contract
- exact head: the frozen head
- method/auditor: the control plane's review route
- material findings: in PR #1936
- verdict: in PR #1936

## PR and closeout

- changed-file review: three owned paths
- unresolved review threads: none
- related/superseded PRs: none
- protected auto-merge: control plane
- merge commit/result: in PR #1936
- ownership release: on merge

## Context checkpoint

```yaml
last_progress: candidate frozen
status: completed
branch: claude/game-char-cmd-0-20261008
pr: 1936
blocker: null
next_action: control plane requests independent review and owner acceptance
```
