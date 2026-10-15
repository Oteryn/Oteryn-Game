# OTV2-20261015-char-cmd-ucc9

```yaml
task_id: OTV2-20261015-char-cmd-ucc9
title: "CHAR-CMD-UCC9: close gate U-CC9 of Character Authority commands v1 (§14 step 4 enablement packet)"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/char-cmd-ucc9-enable
issue: 1622
pr: 2211
head_sha: "the head reported to the control plane"
final_head_sha: "the head reported to the control plane"
owner: claude-code worker for control plane session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-15
updated_at: 2026-10-15
execution_policy: continuous_progress
owned_paths:
  - docs/contracts/OTERYN_GAME_CHARACTER_AUTHORITY_COMMANDS_V1.md
  - docs/agents/tasks/archive/OTV2-20261015-char-cmd-ucc9.md
public_contracts:
  - docs/contracts/OTERYN_GAME_CHARACTER_AUTHORITY_COMMANDS_V1.md
depends_on: []
blocks: []
cross_repository_coordination_id: GAME-CHAR-CMD-0
external_repositories: [Oteryn/Oteryn-Platform]
```

## Outcome

- Gate U-CC9 (§13) is closed. It covers the cap on intents Platform commits behind the consumer's cursor per 30 s rescan interval.
- The enabled backlog is at most 40 unreceipted PENDING intents, so a forward burst stays inside the 41-key TTL bound. Platform enforces it in `CharacterCommandSettings`.
- Platform issuance commits no fresh intent behind the cursor, so the per-interval count is 0:
  - Platform draws the `source_revision` under its per-issuer lock and holds the lock until the outermost transaction commits.
  - It refuses and alarms on an intent whose `source_revision` is not above every stored key.
- Platform PR Oteryn/Oteryn-Platform#1632 carries:
  - the guard;
  - a PostgreSQL concurrency test with a forward-only consumer that never rescans;
  - the enablement runbook `docs/operations/CHARACTER_AUTHORITY_COMMANDS_ENABLEMENT.md`, which lists every flag and precondition.
- Only these parts of the contract change: the §4.1 rescan paragraph, the §13 U-CC9 row, §14 step 4, and a §15 note recording the editorial change.

## Architecture and source of truth

- PROVEN (Platform test): the race test runs 7 issuers × 24 intents, each issuer inside an outer transaction. A cursor that never moves back lists every stored intent in key order.
- PROVEN (Platform test): with the PostgreSQL lock made shared, the same test fails.
- PROVEN (Platform test): a sequence set below the table's maximum is refused, and no row is stored.
- DERIVED: §5.4 reopens keep their original times and are decided by §5.4, so they are outside the per-interval cap.
- DERIVED: a forward burst is bounded by the backlog. With 30 s rescan, 24 s listing and 6 s per read and receipt, the 300 s TTL covers 41 unseen keys, so the enabled backlog is at most 40.
- DECIDED (control plane, fix round 1 for Codex P1 4281164563): Platform's `max_pending_intents` defaults to 40, and a configured value above 41 makes the command surface unavailable. Platform PR #1632 enforces it with a test.

## High-risk authority/recovery qualification

`NOT_APPLICABLE` in this repository: the change is docs only. It changes no Rust, persistence, wire or regenerated output, and it enables nothing.

## Acceptance criteria

- [x] U-CC9 closed in §4.1 and §13, with the Platform evidence named.
- [x] Wire unchanged: no member, path, code or response edited, and `CHARCMD-PENDING-BACKLOG` stays 256. The enabled backlog of at most 40 is a Platform configuration limit inside it. The lock pin is unchanged.
- [x] §14 step 4 points to the enablement packet.

## Excluded scope

- No change to the consumer code, the node template or the lock entry.
- No enablement or production configuration.

## Validation

### Focused

- `git diff --check`: pass
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass

### Component/integration

- `NOT_APPLICABLE`: documentation only. The Platform PR carries the tests.

### E2E

- `NOT_APPLICABLE`: documentation only.

## Self-review

- method/reviewer: task writer. I checked the closure text against Platform `CharacterCommandClient::issue()` and `lockedPendingCount()`, against §5.3 and §5.4, and against the Game consumer's rescan.
- material findings: none open
- verdict: ready for review

## PR and closeout

- changed-file review: two owned paths
- protected auto-merge: control plane
- ownership release: on merge

## Context checkpoint

```yaml
last_progress: U-CC9 closed in the contract; Platform PR #1632 carries the proof and the runbook
status: completed
branch: claude/char-cmd-ucc9-enable
pr: 2211
blocker: null
next_action: control plane reviews and merges after Oteryn/Oteryn-Platform#1632
```
