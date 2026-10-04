# OTV2-20261004-house-rt-inbox-packets

```yaml
task_id: OTV2-20261004-house-rt-inbox-packets
title: "ARCH-HOUSE-RT-INBOX-PACKETS-1: HOUSE-RUNTIME-1 and INBOX-1 packets"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/arch-house-rt-inbox-packets-20261004
issue: 162
pr: 1776
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
owner: claude-code-session_01YL1cQaLL3BquJajKivZVhw (Sol Supervising Architect)
created_at: 2026-10-04
updated_at: 2026-10-04
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_HOUSE_RT_INBOX_PACKETS_2026-10-04.md
  - docs/architecture/reviews/OTERYN_GAME_SOCIAL_MAP_PACKETS_2026-10-04.md
  - docs/architecture/reviews/OTERYN_GAME_ARCH_BATCH_PREMIUM_SOCIAL_PACKETS_2026-10-04.md
  - docs/architecture/reviews/OTERYN_GAME_HOUSE_RUNTIME0_HOUSE_INTERIOR_RUNTIME_DECISION_2026-09-30.md
  - docs/agents/tasks/archive/OTV2-20261004-house-rt-inbox-packets.md
public_contracts: []
depends_on: []
blocks: [HOUSE-RUNTIME-1a, HOUSE-RUNTIME-1b, HOUSE-RUNTIME-1c, INBOX-1a, INBOX-1b, HOUSE-1a, HOUSE-1b, MAP-OVERLAY-1c]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

- HOUSE-RUNTIME-1 is split by root. 1a holds the item shapes and waits on SCOPE-HANDOFF-1. 1b
  holds the live interior and also waits on MAP-LOAD-1. 1c holds the house position and also
  waits on CHAR-POSITION-1.
- INBOX-1 is split the same way. 1a holds the family, the counter and the unreserved delivery,
  and waits on nothing unmerged. 1b holds the out-shapes and waits on DEPOT-1 and its client.
- One replaceable access function keeps the HOUSE-CUSTODY-0 §3.5 closure in the database before
  HOUSE-1a. While SCOPE-HANDOFF-1's exit gate is closed, house entry stays closed.
- Amended:
  - #1773 dependencies: HOUSE-1a and MAP-OVERLAY-1c wait on HOUSE-RUNTIME-1a; HOUSE-1b waits on
    INBOX-1a. HOUSE-1a also replaces the access bodies;
  - #1738 §2.3: SCOPE-HANDOFF-1 creates the stub;
  - the HOUSE-RUNTIME-0 child table.
- No code, migration number, contract or wire change.

## Validation

- `python tools/agents/validate_governance.py`: pass
- `python tools/repository/validate_repository_policy.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass
