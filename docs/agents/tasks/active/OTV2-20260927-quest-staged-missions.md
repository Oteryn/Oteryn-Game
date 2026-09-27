# OTV2-20260927-quest-staged-missions

```yaml
task_id: OTV2-20260927-quest-staged-missions
title: Quest format slice 3 - staged missions (D34), transitions (D35) and the Canary + CrystalServer quest-log transcription
mode: CONTRACT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/zealous-edison-3ttg1s
issue: 162
pr: null
base_sha: null
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude-code-session-01AVd6BKKTRbeW1Pub9bg9Jk
created_at: 2026-09-27
updated_at: 2026-09-27
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/OTV2-20260927-quest-staged-missions.md
  - docs/architecture/OTERYN_QUEST_AUTHORING_FORMAT_V1.md
  - tools/content-schema/quest-authoring/**
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Owner decision D34 (2026-09-27, this session): storyline quests use staged missions, option A of
`CONTENT-QUEST-01`. A mission is one integer progress track with a range and a journal text per
stage. Transitions are named events of their owners. A graph of typed objectives (option B) is
added only when a quest needs it. The quest logs of both servers become 58 storyline quests with
529 missions and a writer index per progress track. The output is one quest catalogue that
absorbs the matching reward-only quests.

Owner decision D35 (2026-09-27, this session): only the quest domain writes quest progress. Each
mission declares named transitions, and owners (NPC dialogue, movements, actions, creature events,
encounters, claims) only request them. This also settles the quest-state part of NPC decision O4.
The Lua writers of both servers become 1,616 candidate transitions over 355 missions, each with an
owner, an effect and, where the if-block shows it, the stage it starts from. Runtime, persistence
and `content/**` stay unchanged.

## Architecture and source of truth

- `PROVEN`: the pinned Canary and CrystalServer revisions of slices 1-2; quest-log sources and
  their blob ids in `samples/questlog/manifest.json`.
- `DERIVED`: wiki links through the slice-1 coverage sample.
- `CONFLICT`: 35 missions (mostly journal texts) keep the Canary value until the wiki decides.
- `UNKNOWN`: 179 progress tracks have no literal `setStorageValue` writer in either server.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: offline authoring evidence and Python tooling only; no production mutation,
fence, session, authority or persisted-recovery evidence is touched.

## Acceptance and evidence

- `verify_quest_schema.py` 63/63 with the cause of each negative case checked.
- The Queen of the Banshees seal transitions read correctly from both servers (movement, from an
  unset seal to 1; the last seal from the Queen's dialogue without a stage guard).
- Journal lines are text references only (LICENSE-ASSETS.md); no narrative text is committed.
- `ots_chests.py`, `ots_doors.py` and `ots_questlog.py` are deterministic; the catalogue, claims,
  gates and progress tracks validate together.
- `validate_governance.py` and `validate_repository_policy.py` pass.
- Jira: mapping to a programme Story not resolved in this session (pending).
