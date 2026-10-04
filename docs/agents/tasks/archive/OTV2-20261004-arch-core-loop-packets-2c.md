# OTV2-20261004-arch-core-loop-packets-2c

```yaml
task_id: OTV2-20261004-arch-core-loop-packets-2c
title: "ARCH-CORE-LOOP-PACKETS-2 part C: Premium gameplay wiring and the first house, guild and party packets"
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
blocks: [PREM-WIRE-1, ECON-RET-0, BED-CONTENT-1, SCOPE-HANDOFF-1, CHAT-2, GUILD-1, PARTY-1]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

- **D486 item 4:** PREM-WIRE-1 (§2.1) adds three things:
  - a fail-closed trusted clock from the kernel NTP state (`ntp_adjtime`, `maxerror`), which reads
    as `None` when unsynchronized or not on Linux (§1.1);
  - one per-command `premium_current(account_id)` seam (§1.2);
  - the PREM-4 spell cast check.

  The yell gate is wired by the later of CHAT-1b-2b and PREM-WIRE-1. PREM-2b waits for PREM-5,
  because nothing can be promoted yet (§1.3, §3).
- **D486 item 5:** packets for ECON-RET-0 (the guild and market retention profiles in one
  privacy review; BANK-RET-0 stays with #1733 §2.3, with no duplicate), BED-CONTENT-1, SCOPE-HANDOFF-1, CHAT-2, GUILD-1 and PARTY-1. Each packet
  lists what it needs for acceptance (§1.5, §2), and §3 lists every held child with what releases
  it.
- Control plane queue (§5):
  1. BANK-RET-0 (#1733) and ECON-RET-0 first.
  2. ADMIT-0 acceptance.
  3. PREM-5 together with PREM-2b.
  4. Relay key authority.
- **#1738 P1 4176929764:** the Premium seam is gated on `PremiumActivation`, which defaults to
  `None` in production. A configured snapshot source cannot make `premium_current` true before
  PREM-1's activation record and separate owner authority (PREMIUM-DELIVERY-0 §10.3) (§1.2, §2.1).
- No code, contract, wire or migration change.

## Validation

- `python tools/agents/validate_governance.py`: pass
- `python tools/repository/validate_repository_policy.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass
