# OTV2-20261004-arch-kill-reward-logout-1

```yaml
task_id: OTV2-20261004-arch-kill-reward-logout-1
title: "ARCH-KILL-REWARD-LOGOUT-1: KILL-REWARD-COMP-1 and LOGOUT-WIRE-1 packets"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/arch-kill-reward-logout-20261004
issue: 162
pr: 1802
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
owner: claude-code-session_01YL1cQaLL3BquJajKivZVhw (Sol Supervising Architect)
created_at: 2026-10-04
updated_at: 2026-10-04
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_ARCH_KILL_REWARD_LOGOUT_PACKETS_2026-10-04.md
  - docs/agents/tasks/archive/OTV2-20261004-arch-kill-reward-logout-1.md
public_contracts: []
depends_on: []
blocks: [KILL-REWARD-COMP-1, LOGOUT-WIRE-1]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

- KILL-REWARD-COMP-1 packet (hard, persistence review), after ATTACK-1b #1798 and D3-7:
  - the creature reward binding: an optional pinned `loot_tables` section in the native gameplay
    manifest and a per-generation `CreatureRewardTable` (XP from `profile.experience`, the
    corpse through the D3-7 `i00005801` alias, the loot table, the Bestiary race), fail closed
    per creature;
  - the settle functions take owned `ProjectedCreatureDeathFacts` read under the runtime lock,
    and every durable write and slot acquisition runs after the channel guards are released;
  - a per-Channel settlement queue (`KILLRW-RL-01` = 64), drained by the reward principal's own
    session, and before its terminal release;
  - auto-attack and spell kills (the `CombatBatchReceipt` walk) feed the same queue;
  - a live-path PG test.
- LOGOUT-WIRE-1 packet (hard, protocol review), after ATTACK-1b #1798: `LOGOUT_INTENT` under
  `LOGOUT_V1` (numbers proposed, leased by the control plane), results `ACCEPTED`, `IN_FIGHT`
  with `retry_after_ms` and `BUSY`; an accepted logout runs `TerminalRelease::Logout` at once,
  with no grace and no PvE re-entry protection; the client binds Ctrl+L and Ctrl+Q.
- No code, registry or contract change.

## Validation

- `python tools/agents/validate_governance.py`: pass
- `python tools/repository/validate_repository_policy.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass
