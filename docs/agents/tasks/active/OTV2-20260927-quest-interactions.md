# OTV2-20260927-quest-interactions

```yaml
task_id: OTV2-20260927-quest-interactions
title: Quest format slice 4 - interaction definitions (D36) and The Queen of the Banshees pilot from Canary + CrystalServer scripts
mode: CONTRACT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/zealous-edison-3ttg1s
issue: 162
pr: null
base_sha: 53abffba06ab8001e94748cfc8b928cdc5458cd6
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude-code-session-01AVd6BKKTRbeW1Pub9bg9Jk
created_at: 2026-09-27
updated_at: 2026-09-27
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/OTV2-20260927-quest-interactions.md
  - docs/architecture/OTERYN_QUEST_AUTHORING_FORMAT_V1.md
  - tools/content-schema/quest-authoring/**
public_contracts: []
depends_on:
  - OTV2-20260927-quest-staged-missions
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Owner decision D36 (2026-09-27, this session, "kontynuuj" after the consistency check): quest
movement, action and creature-event scripts become interaction definitions in
`content/interactions/`, compiled to GAME-INTERACTION-01 plans. An edge, read-only conditions and
children executed by their existing owners: quest transitions and world state (D35, D29), ability
summons (GAME-ABILITY-01), item handouts (DUR-03). Teleports and map-object changes stay as blocked
children until movement and world-object owner contracts exist; encounters keep D27 outcomes.

The pilot transcribes the 16 scripts of The Queen of the Banshees in both servers into 18
interactions; the seven seal flames request the seven movement transitions of slice 3. Runtime,
persistence and `content/**` stay unchanged.

## Architecture and source of truth

- `PROVEN`: the pinned Canary and CrystalServer revisions of slices 1-3; script blob ids in
  `samples/interactions/the_queen_of_the_banshees/manifest.json`.
- `DERIVED`: transition names through the slice-3 quest-log sample.
- `CONFLICT`: 3 interactions (first seal lever item ids, first seal magic walls) until the wiki decides.
- `UNKNOWN`: 6 statements and 15 conditions outside the transcribed vocabulary; 9 progress tracks
  written but not yet declared by the quest catalogue.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: offline authoring evidence and Python tooling only; no production mutation,
fence, session, authority or persisted-recovery evidence is touched.

## Acceptance and evidence

- `verify_quest_schema.py` 90/90 with the cause of each negative case checked.
- `ots_interactions.py` is deterministic; interactions validate against the quest catalogue and
  progress tracks together with the claims and gates; slices 1-3 regenerate unchanged.
- No narrative text is committed (LICENSE-ASSETS.md).
- `validate_governance.py` and `validate_repository_policy.py` pass.
- Jira: mapping to a programme Story not resolved in this session (pending).
