# OTV2-20261003-highscores0: HIGHSCORES-0 World Highscores decision

```yaml
task_id: OTV2-20261003-highscores0
title: HIGHSCORES-0 World Highscores decision
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: arch/highscores0-decision-20261003
pr: null
base_sha: 3d8c14d0f6326e278cf09e97acd2453b6af31926
head_sha: exact frozen head in the #1622 FREEZE_SHA entry
final_head_sha: exact frozen head in the #1622 FREEZE_SHA entry
final_head_frozen_at: null
owner: claude-code-session-016c5MQoe5CoMk9fxmuuMcFJ (Sol Supervising Architect)
created_at: 2026-10-03
updated_at: 2026-10-03
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_HIGHSCORES0_WORLD_HIGHSCORES_DECISION_2026-10-03.md
  - docs/agents/tasks/archive/OTV2-20261003-highscores0.md
public_contracts: []
depends_on: [D296, D297]
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Candidate decision `HIGHSCORES0-WORLD-HIGHSCORES-V1`: World-scoped periodic durable snapshots (30
minutes), categories with a durable source now (Experience, Magic Level, seven skills, Achievement
Points per account, Charm Points), 1,000 rows per list, 50 per page, competition ranks, a paged
query under capability `HIGHSCORES_V1`. Children HS-1 and HS-WIRE-1.

## Architecture and source of truth

- PROVEN: scope matrix Ranking row; ADR-0004 §6; migrations `0009`, `0020`, `0021`, `0030`;
  achievement owner contract §4; BOSS-RAID-0 §10.2.
- CIPSOFT_OFFICIAL: manual `interface.md` §3.6.26, `achievements.md`, `accounts.md`, `combat.md`.
- TIBIAWIKI_STRUCTURED: Highscores revid 1190424.
- OTS_HYPOTHESIS_ONLY: none used.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: docs only.

## Acceptance criteria

- [ ] Exact-head validation and independent review of the frozen head.
- [ ] Protected integration through Merge Queue.
