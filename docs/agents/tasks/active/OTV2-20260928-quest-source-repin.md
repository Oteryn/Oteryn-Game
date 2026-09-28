# OTV2-20260928-quest-source-repin

```yaml
task_id: OTV2-20260928-quest-source-repin
title: Quest format - re-pin the Canary and CrystalServer reference revisions to their main heads
mode: CONTRACT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/zealous-edison-3ttg1s
issue: 162
pr: null
base_sha: 3b41c0f4c0b3d4480a392d1c0ea4c4b40f7c16ea
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude-code-session-01AVd6BKKTRbeW1Pub9bg9Jk
created_at: 2026-09-28
updated_at: 2026-09-28
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/OTV2-20260928-quest-source-repin.md
  - docs/architecture/OTERYN_QUEST_AUTHORING_FORMAT_V1.md
  - tools/content-schema/quest-authoring/**
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Owner decision (2026-09-28, this session): re-pin the quest transcriptions to the current `main`
heads, Canary `04b83b51` and CrystalServer `9f5a72c6`. Of their merged changes, two touch quest
content, both in CrystalServer: the Summer Court NPC storage key and a guard in the Wrath of the
Emperor mission 2 teleport repair.

All converters, the map check and the readiness map were rerun. Apart from the revision identity,
the samples change only in shifted source lines, and in the anchor count of the map check (313 to
332, all present), which catches up with #1071.

Unmerged upstream branches are not sources.

## Architecture and source of truth

- `PROVEN`: Canary `04b83b512114bfd888000d6e1433ed8ecaec7c5b` and CrystalServer
  `9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d`. The Canary v3.6.1 map and the CrystalServer map are
  unchanged (the same sha256).

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: offline tooling and a CANDIDATE format document.

## Acceptance and evidence

- `verify_quest_schema.py` 219/219; `validate_quest_content.py` 0 errors; converters deterministic.
- `validate_governance.py` and `validate_repository_policy.py` pass.
- Jira: no mapped Story (pending).
