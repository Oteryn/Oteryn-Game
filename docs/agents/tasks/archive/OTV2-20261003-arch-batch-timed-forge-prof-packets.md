# OTV2-20261003-arch-batch-timed-forge-prof-packets

```yaml
task_id: OTV2-20261003-arch-batch-timed-forge-prof-packets
title: "Architect batch: TIMED-RT-1b/1c, TIMED-WIRE-1, FORGE-1a/1b and PROF-SHAPE-1b packets"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: arch/timed-forge-prof-packets-20261003
issue: 162
pr: 1687
head_sha: "exact frozen head in the FREEZE_SHA entry"
final_head_sha: "exact frozen head in the FREEZE_SHA entry"
owner: claude-code-session_016c5MQoe5CoMk9fxmuuMcFJ (Sol Supervising Architect)
created_at: 2026-10-03
updated_at: 2026-10-03
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_ARCH_BATCH_TIMED_FORGE_PROF_PACKETS_2026-10-03.md
  - docs/agents/tasks/archive/OTV2-20261003-arch-batch-timed-forge-prof-packets.md
public_contracts: []
depends_on: []
blocks: [FORGE-1a, TIMED-RT-1b, PROF-SHAPE-1b, TIMED-WIRE-1, FORGE-1b, TIMED-RT-1c]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

- **TIMED-RT-1 is split again.**
  - RT-1b covers expiry (causes 2 and 3) and the hosting runtime for items already in their slot
    and for the exercise binding. It needs #1681 and TIMED-CONTENT-1 only.
  - RT-1c covers the deadline, put-out, equip and light forms after the move slices.
  - EQUIP-RT-1 is not an RT-1b dependency.
- **`show_count` (D397).** TIMED-WIRE-1 adds a server-authoritative-only item group, so the
  client projection is unchanged. It starts after carrier #1675.
- **FORGE-1 is split at the dust asset.** FORGE-1a covers the balance, ledger, composition,
  DustLimit and the gain and spend APIs; the dust limit price is a ruleset formula. It can start
  now. FORGE-1b keeps the original dependencies. PROF-SHAPE-1b follows FORGE-1a and #1685.
- **Proposed leases.** Migrations 0058 (RT-1b), 0059 (FORGE-1a), 0060 (PROF-SHAPE-1b),
  0061 (FORGE-1b) and 0062 (RT-1c). TIMED-WIRE-1 keeps capability 11 (D353). Cap 12, cmd 21 and
  domain 14 stay free.
- **Shared files.** Code paths are disjoint. Four append-only registers are shared line-wise and
  merged by union (§0).
- **Codex round 1 (3 × P1), fixed in one push.** 4174637304: §1.5 adds the decision analysis
  and test for each ruling. 4174637294: FORGE-1a registers `IMBFORGE0-RL-03`,
  `DUR03-RL-03-FORGE` and the DustLimit `DUR03-RL-06` rows. 4174637298: PROF-SHAPE-1b lists
  `DUR03-RL-06-PROF` (1 / 4).

## Validation

- `python3 tools/agents/validate_governance.py`: pass
- `python3 tools/repository/validate_repository_policy.py`: pass
- `python -m unittest discover -s tools/agents/tests`: 54 tests, OK
- `git diff --cached --check`: pass (no output)
