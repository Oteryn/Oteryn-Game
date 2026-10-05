# OTV2-20261004-house-rt-inbox-fix

```yaml
task_id: OTV2-20261004-house-rt-inbox-fix
title: "ARCH-HOUSE-RT-INBOX-FIX-1: deferred #1776 review findings"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/arch-house-rt-inbox-fix-20261004
issue: 162
pr: 1780
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
owner: claude-code-session_01YL1cQaLL3BquJajKivZVhw (Sol Supervising Architect)
created_at: 2026-10-04
updated_at: 2026-10-04
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_HOUSE_RT_INBOX_PACKETS_2026-10-04.md
  - docs/architecture/reviews/OTERYN_GAME_SOCIAL_MAP_PACKETS_2026-10-04.md
  - docs/agents/tasks/archive/OTV2-20261004-house-rt-inbox-packets.md
  - docs/agents/tasks/archive/OTV2-20261004-house-rt-inbox-fix.md
public_contracts: []
depends_on: [OTV2-20261004-house-rt-inbox-packets]
blocks: [INBOX-1a, HOUSE-RUNTIME-1b, HOUSE-1a]
cross_repository_coordination_id: null
external_repositories: []
```

## Scope

Fixes the three findings that #1776 deferred under D245: 4178297984, 4178297987 and 4178297990.

## Outcome

- 4178297984: the Inbox `cause_kind` is a foreign key to the registry table
  `game_character_inbox_cause_kinds`. It is no longer a closed CHECK. Each caller inserts its own
  kind row, so the callers compose in any merge order (packet §2.4, §3).
- 4178297987: HOUSE-1a waits on the acceptance of HOUSE-RUNTIME-1b and 1c, with the exit gate
  lifted, not only on their merge. HOUSE-1b, HOUSE-ACL-1, HOUSE-WIRE-1 and GUILDHALL-1 inherit
  that gate through HOUSE-1a (packet §0.2, §1.5, §3; #1773 §0.1 and the HOUSE-1a base).
- 4178297990: the #1776 record's outcome now states HOUSE-1a's full dependency and the composed
  access parts.
- No code, migration number, contract or wire change.

## Validation

- `python tools/agents/validate_governance.py`: pass
- `python tools/repository/validate_repository_policy.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass
