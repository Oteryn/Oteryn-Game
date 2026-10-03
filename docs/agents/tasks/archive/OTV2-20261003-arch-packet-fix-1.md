# OTV2-20261003-arch-packet-fix-1

```yaml
task_id: OTV2-20261003-arch-packet-fix-1
title: "ARCH-PACKET-FIX-1: TIMED-RT-1b and RT-1c row coverage; FORGE-1b throughput measurement"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/charming-cray-1kk6ax
issue: 1622
pr: 1689
head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
owner: Sol Supervising Architect
created_at: 2026-10-03
updated_at: 2026-10-03
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_ARCH_BATCH_TIMED_FORGE_PROF_PACKETS_2026-10-03.md
  - docs/agents/tasks/archive/OTV2-20261003-arch-packet-fix-1.md
public_contracts: []
external_repositories: []
```

## Outcome

This task carries the two round-3 P1s on #1687's frozen head `bfa9114e`. The control plane split
them under D317 (replies 4174699543 and 4174699734). They must land before TIMED-RT-1b or FORGE-1b
implementation starts. The fixes amend the bundle's packets in place (§2.3, §2.5, §2.6). They
change no code, no contract, no wire and no resource value.

- **4174692081 (TIMED-RT-1b rows).**
  - §2.3 now registers the §12 checkpoint and composed checkpoint shape rows next to the expiry
    shapes, with max and max+1 tests. #1681 (RT-1a) left every DUR-03 shape ceiling to RT-1b.
  - `TIMEDITEM0B-RL-01`, `-02`, `-04` and `-05` are #1681's rows. RT-1b proves each one again at
    the host boundary it adds, and registers any of them that #1681 did not.
  - §2.6 gives the §12 use form and put out rows to RT-1c.
- **4174692089 (FORGE-1b throughput).** §2.5 adds `IMBFORGE0-RL-04` to FORGE-1b's registry rows.
  It also adds a PostgreSQL p99 measurement, which must exist before FORGE-1b merges
  (IMBUE-FORGE-0 §15). The measured budget is registered as a qualification budget, like
  `MAP01-VIEWPORT-US`. The architect accepts the measured value at FREEZE, so the worker does not
  pick it.

## Validation

- `python3 tools/agents/validate_governance.py`: pass
- `git diff --check`: pass
