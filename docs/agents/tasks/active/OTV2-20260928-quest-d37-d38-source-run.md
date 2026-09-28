# OTV2-20260928-quest-d37-d38-source-run

```yaml
task_id: OTV2-20260928-quest-d37-d38-source-run
title: Quest format - merge #1053 with main and run its D37/D38 converter on the pinned sources
mode: CONTRACT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/zealous-edison-3ttg1s
issue: 162
pr: null
base_sha: 00691b5bdc4b96fe41954e0b62b99353e0b03bf0
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude-code-session-01AVd6BKKTRbeW1Pub9bg9Jk
created_at: 2026-09-28
updated_at: 2026-09-28
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/OTV2-20260928-quest-d37-d38-source-run.md
  - docs/agents/tasks/active/OTV2-20260928-quest-relocation-worldobject-transcription.md
  - docs/architecture/OTERYN_QUEST_AUTHORING_FORMAT_V1.md
  - tools/content-schema/quest-authoring/**
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Owner allocation (2026-09-28, this session): finish #1053. That PR's converter types the D37
relocation and D38 world-object overlay children, but its session had no source checkouts and #1041
and #1067 (this session) later conflicted with it. This branch:
- merges the #1053 head (`a799e3a4`, commits unchanged) with main and resolves the additive
  conflicts;
- runs `ots_interactions.py` on pinned Canary `47dfd51f` and CrystalServer `ff7ede59`;
- lets the receiver of `transform`/`createItem`/`setActionId` be a call chain, and a
  `remove`/`removeItem` argument hold one nested call, so those calls are not lost.

Result:
- Movement: 217 relocations to an anchor, 203 to the previous tile, 380 blocked.
- WorldObject: 493 `TRANSFORM`, 42 `CREATE`, 174 `REMOVE`, 39 `RETAG`, 117 blocked.
- Mapped interactions stay at 268.
- Unresolved statements go from 1,908 to 1,913. The five extra lines are indexed removals and Lua
  `table.remove` calls that the old transcription wrongly counted as world-object children.

The #1053 task record rides along unchanged; once this merges, #1053 can be closed as superseded.

## Architecture and source of truth

- `PROVEN`: pinned Canary and CrystalServer revisions.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: offline tooling and a CANDIDATE format document.

## Acceptance and evidence

- `verify_quest_schema.py` 219/219; `validate_quest_content.py` 0 errors; converters deterministic.
- `validate_governance.py` and `validate_repository_policy.py` pass.
- Jira: no mapped Story (pending).
