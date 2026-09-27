# OTV2-20260927-quest-conflict-decisions

```yaml
task_id: OTV2-20260927-quest-conflict-decisions
title: Quest format - D25 decisions for every Canary/CrystalServer conflict of the quest transcriptions
mode: CONTRACT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/zealous-edison-3ttg1s
issue: 162
pr: null
base_sha: 514d6de1aa2b560c50a4d2003f33744a9f02a066
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude-code-session-01AVd6BKKTRbeW1Pub9bg9Jk
created_at: 2026-09-27
updated_at: 2026-09-27
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/OTV2-20260927-quest-conflict-decisions.md
  - docs/agents/tasks/archive/OTV2-20260927-quest-interactions.md
  - docs/architecture/OTERYN_QUEST_AUTHORING_FORMAT_V1.md
  - tools/content-schema/quest-authoring/**
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Owner request (2026-09-27, this session): settle the server conflicts first. Every conflict of the
four transcriptions is decided under D25 and recorded in `conflict_decisions.json`:
- chests: 1;
- doors: 10;
- missions: 35;
- interactions: 37, down from 60 after fixing a callback join that paired callbacks by their position
  in a file.

Nineteen decisions choose CrystalServer, where the wiki shows Canary outdated or broken (for example
Hot Cuisine, The Way of the Monk, Heart of Destruction and Rottin Wood). The converters apply the
decisions and fail on a stale one. No manifest keeps a `conflict` status. Runtime, persistence and
`content/**` stay unchanged.

## Architecture and source of truth

- `PROVEN`: the pinned Canary and CrystalServer revisions; cited Fandom revisions per decision.
- `DERIVED`: equivalence and wording judgements summarised in our own words.
- `CONFLICT`: none left; 27 decisions keep Canary because the wiki is silent.
- `UNKNOWN`: unchanged from slice 4 (unresolved lines and conditions, undeclared tracks).

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: offline authoring evidence and Python tooling only; no production mutation,
fence, session, authority or persisted-recovery evidence is touched.

## Acceptance and evidence

- All four converters are deterministic and validate together; `verify_quest_schema.py` 96/96.
- Wiki-based decisions cite a Fandom revision; two were re-checked against the API (Percybald in
  Carlin, the 15th Hot Cuisine dish). No wiki, journal or dialogue text is committed.
- `validate_governance.py` and `validate_repository_policy.py` pass.
- Jira: mapping to a programme Story not resolved in this session (pending).
