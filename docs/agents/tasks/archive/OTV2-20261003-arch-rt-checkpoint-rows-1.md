# OTV2-20261003-arch-rt-checkpoint-rows-1

```yaml
task_id: OTV2-20261003-arch-rt-checkpoint-rows-1
title: "ARCH-RT-CHECKPOINT-ROWS-1: the §12 checkpoint rows become TIMED-RT-1b's first item"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/arch-rt-checkpoint-rows-1
issue: 1622
pr: 1692
head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
owner: Sol Supervising Architect
created_at: 2026-10-03
updated_at: 2026-10-03
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_ARCH_BATCH_TIMED_FORGE_PROF_PACKETS_2026-10-03.md
  - docs/agents/tasks/archive/OTV2-20261003-arch-rt-checkpoint-rows-1.md
public_contracts: []
external_repositories: []
```

## Outcome

This task carries #1689's round-3 P1 4174803489. The control plane routed it to this follow-up
under D317. #1681 (RT-1a) was already in the Merge Queue, and it merged its checkpoint writer
without the §12 checkpoint and composed checkpoint shape rows.

- §2.3 makes both rows TIMED-RT-1b's first item. RT-1b's first commit registers them, with max and
  max+1 tests against RT-1a's merged writer, before any other RT-1b work. The rest of RT-1b is not
  reviewed until they are on its branch.
- §2.3's Rows list keeps the three expiry shapes. The fallback for `TIMEDITEM0B-RL-01/-02/-04/-05`
  now reads against `main` rather than an open #1681.

This changes no code, no contract, no wire and no resource value.

## Validation

- `python3 tools/agents/validate_governance.py`: pass
- `python3 -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass
