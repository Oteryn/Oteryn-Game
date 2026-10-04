# OTV2-20261004-arch-core-loop-packets-2c

```yaml
task_id: OTV2-20261004-arch-core-loop-packets-2c
title: "ARCH-CORE-LOOP-PACKETS-2 part C: the first house, guild and party packets"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/arch-core-loop-packets-c-20261004
issue: 162
pr: 1738
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
owner: claude-code-session_01YL1cQaLL3BquJajKivZVhw (Sol Supervising Architect)
created_at: 2026-10-04
updated_at: 2026-10-04
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_ARCH_BATCH_PREMIUM_SOCIAL_PACKETS_2026-10-04.md
  - docs/agents/tasks/archive/OTV2-20261004-arch-core-loop-packets-2c.md
public_contracts: []
depends_on: []
blocks: [ECON-RET-0, BED-CONTENT-1, SCOPE-HANDOFF-1, CHAT-2, PARTY-1]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

- **D486 item 5:** this batch has packets for:
  - ECON-RET-0: the guild and market retention profiles in one privacy review. BANK-RET-0 stays
    with #1733 §2.3, with no duplicate.
  - BED-CONTENT-1, SCOPE-HANDOFF-1, CHAT-2 and PARTY-1.

  Each packet lists what it needs for acceptance (§1.2, §2), and §3 lists every held child with
  what releases it.
- **D490 split:** the Premium-gated part moves to PREMIUM-ACTIVATION-0 (#1743). That covers
  D486 item 4: the trusted clock, the `PremiumStatus` seam, the activation latch, the consumer
  table, PREM-WIRE-1 and GUILD-1. These findings move with it and are answered there:
  - P1 4176929764;
  - P1 4176947451;
  - P1 4176973984;
  - P1 4176993795;
  - P1 4176993801;
  - P2 4176993804.
- Control plane queue (§5):
  1. BANK-RET-0 (#1733) and ECON-RET-0 first.
  2. ADMIT-0 acceptance.
  3. Relay key authority.
- #1738 P2 4176947456: the five mandatory decision answers are in §6.
- #1738 P1 4177048585: HOUSE-RUNTIME-1 comes before HOUSE-1 and is tested on HOUSE-RUNTIME-0
  §10's operator-owned test house; HOUSE-1 waits on it (§1.1, §3, §4). The P2s are deferred
  (D493).
- No code, contract, wire or migration change.

## Validation

- `python tools/agents/validate_governance.py`: pass
- `python tools/repository/validate_repository_policy.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass
