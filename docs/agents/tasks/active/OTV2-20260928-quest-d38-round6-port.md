# OTV2-20260928-quest-d38-round6-port

```yaml
task_id: OTV2-20260928-quest-d38-round6-port
title: Quest format - port the #1053 fail-closed repair rounds 5-6 and regenerate on the pinned sources
mode: CONTRACT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/zealous-edison-3ttg1s
issue: 162
pr: 1088
base_sha: 4b9b033bd5c43b8de6baa481b462e3b0afdae85d
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude-code-session-01AVd6BKKTRbeW1Pub9bg9Jk
created_at: 2026-09-28
updated_at: 2026-09-28
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/OTV2-20260928-quest-d38-round6-port.md
  - docs/agents/tasks/archive/OTV2-20260928-quest-relocation-worldobject-transcription.md
  - docs/architecture/OTERYN_QUEST_AUTHORING_FORMAT_V1.md
  - tools/content-schema/quest-authoring/**
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

This takes the handoff in #162 (comment 5866835509): #1071 did not carry the six fail-closed
converter fixes of #1053 repair rounds 5-6 (branch `claude/quest-relocation-worldobject-transcription`
at `64b32aea`), and porting them needs the pinned sources.

This branch merges that head and keeps its six fixes as written:
- a duplicate scheduled revert is blocked;
- a non-positive delay is blocked;
- D38 detection is gated on string-stripped code;
- a multi-line call is blocked;
- a positionless assigned `Game.createItem` is a reward item;
- a literal `Position(...):removeItem` receiver binds its anchor.

On top of that:
- a `transform`/`createItem`/`setActionId` whose receiver is reached through a call stays a
  blocked WorldObject child, instead of being typed (as #1071 did) or lost to unresolved;
- the samples are regenerated on Canary `04b83b51` and CrystalServer `9f5a72c6`;
- one new conflict is decided (D25, equivalent, Canary kept): CrystalServer #952 adds nil guards to the
  Wrath of the Emperor mission 2 teleport repair.

Result:
- D38: 479 `TRANSFORM`, 27 `CREATE`, 133 `REMOVE`, 39 `RETAG`, 166 blocked;
- D37 is unchanged;
- unresolved statements 1,902; mapped 267;
- `verify_quest_schema.py` 262/262.

## Architecture and source of truth

- `PROVEN`: Canary `04b83b51` and CrystalServer `9f5a72c6`.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: offline tooling and a CANDIDATE format document.

## Acceptance and evidence

- `verify_quest_schema.py` 262/262; `validate_quest_content.py` 0 errors; converters deterministic.
- `validate_governance.py` and `validate_repository_policy.py` pass.
- Jira: no mapped Story (pending).
