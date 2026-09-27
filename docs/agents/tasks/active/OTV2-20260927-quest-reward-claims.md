# OTV2-20260927-quest-reward-claims

```yaml
task_id: OTV2-20260927-quest-reward-claims
title: Quest authoring format v1 (reward claims, D32-D33) and the Canary + CrystalServer chest transcription
mode: CONTRACT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/zealous-edison-3ttg1s
issue: 162
pr: 976
base_sha: 419a7cbc8539c9c98bd83d9220e331255e7e8f12
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude-code-session-01AVd6BKKTRbeW1Pub9bg9Jk
created_at: 2026-09-27
updated_at: 2026-09-27
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/OTV2-20260927-quest-reward-claims.md
  - docs/architecture/OTERYN_QUEST_AUTHORING_FORMAT_V1.md
  - tools/content-schema/quest-authoring/**
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Owner decisions D32 (a reward chest is a world interaction with a lightweight `reward_only` quest
record) and D33 (Canary and CrystalServer are both reference sources, joined by map position),
accepted 2026-09-27 in this session. A CANDIDATE quest format covers reward claims. The first
transcription covers the chests of both servers. A wiki coverage sample compares 373 Fandom quests
with both servers. Runtime, persistence and `content/**` stay unchanged and unallocated.

## Architecture and source of truth

- `PROVEN`: opentibiabr/canary `47dfd51f45280a59a1d3e50ba7edd573d7234446` and
  zimbadev/crystalserver `ff7ede593c69d4c658b382c97443e8155926924a` (the revisions already pinned
  by the monster, encounter and CW2 tooling); blob ids of `chest.lua` and
  `quest_reward_common.lua` in the manifest.
- `DERIVED`: wiki coverage from the Fandom API at the recorded page revisions.
- `UNKNOWN`: TibiaWiki BR (Cloudflare challenge on the capture host); map-only containers in
  `world.otbm`.
- `CONFLICT`: two chests (goblet reward, corpse appearance) left for the reference-date wiki.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: offline authoring evidence and Python tooling only; no production mutation,
fence, session, authority or persisted-recovery evidence is touched.

## Acceptance and evidence

- `verify_quest_schema.py` 28/28, including the cause of each negative case.
- `ots_chests.py` is deterministic across repeated runs; the samples validate with catalog and
  manifest: 336 claims, 359 placements, 2 conflicts, 134 approved omissions.
- `validate_governance.py` and `validate_repository_policy.py` pass.
- Jira: mapping to a programme Story not resolved in this session (pending).
