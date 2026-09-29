# OTV2-20260927-quest-reward-claims

```yaml
task_id: OTV2-20260927-quest-reward-claims
title: Quest authoring format v1 (reward claims, D32-D33) and the Canary + CrystalServer chest transcription
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/zealous-edison-3ttg1s
issue: 162
pr: 976
base_sha: 419a7cbc8539c9c98bd83d9220e331255e7e8f12
head_sha: 5a4423d5d2c26e487ef8c2f80f3b092b88374813
final_head_sha: 5a4423d5d2c26e487ef8c2f80f3b092b88374813
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

## Postmerge closeout

`PROVEN`: [PR #976](https://github.com/Oteryn/Oteryn-Game/pull/976) merged
`5a4423d5d2c26e487ef8c2f80f3b092b88374813` as
`46666ef527cb63836f660a3b2e12a23bcdb7ea12` at `2026-09-27T11:19:47Z`.
[Reconciliation allocation](https://github.com/Oteryn/Oteryn-Game/issues/162#issuecomment-5855464134)
confirms the prior lease is terminal/released and allocates this separate archive move.
The bounded authoring delivery is completed; earlier acceptance results, provenance,
conflicts, unknowns and owned paths remain historical evidence. Runtime, persistence
and `content/**` remain unallocated; the recorded Jira mapping remains pending.
`final_head_sha` identifies the merged PR head; its original freeze timestamp is
`UNKNOWN` and stays null. No new review, MQ, E2E or runtime qualification is claimed.
