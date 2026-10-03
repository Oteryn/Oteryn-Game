# OTV2-20261003-analysers0: ANALYSERS-0 Hunting Analysers decision

```yaml
task_id: OTV2-20261003-analysers0
title: ANALYSERS-0 Hunting Analysers decision
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: arch/analysers0-decision-20261003
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
  - docs/architecture/reviews/OTERYN_GAME_ANALYSERS0_HUNTING_ANALYSERS_DECISION_2026-10-03.md
  - docs/agents/tasks/archive/OTV2-20261003-analysers0.md
public_contracts: []
depends_on: [D296, D298]
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Candidate decision `ANALYSERS0-HUNTING-ANALYSERS-V1`: the analysers are client tools; the server
sends own-session hunting facts (experience, kill loot, supplies used, impact, damage input) on a
separate domain `ACTOR_ANALYSER` under capability `ANALYSER_V1`, emitted after commit and never
stored. Party Hunt Analyser and market prices later. Children ANALYSER-WIRE-1, ANALYSER-EMIT-1,
ANALYSER-CLIENT-1.

## Architecture and source of truth

- PROVEN: SPELL-PRESENT-0 §4 and §9; D3; PARTY-PVP-0 §5; migration `0009`; BOSS-RAID-0 §6.3 and
  §13.
- CIPSOFT_OFFICIAL: manual `interface.md` §3.6.12.
- TIBIAWIKI_STRUCTURED: none used.
- OTS_HYPOTHESIS_ONLY: none used.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: docs only.

## Acceptance criteria

- [ ] Exact-head validation and independent review of the frozen head.
- [ ] Protected integration through Merge Queue.
