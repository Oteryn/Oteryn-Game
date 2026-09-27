# OTV2-20260927-quest-tracks-dialogue-owners

```yaml
task_id: OTV2-20260927-quest-tracks-dialogue-owners
title: Quest format - declared auxiliary tracks, NPC dialogue binding of transitions, relocation and world-object owner proposal
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
  - docs/agents/tasks/active/OTV2-20260927-quest-tracks-dialogue-owners.md
  - docs/architecture/OTERYN_QUEST_AUTHORING_FORMAT_V1.md
  - docs/architecture/OTERYN_INTERACTION_RELOCATION_AND_WORLD_OBJECT_OWNERS_PROPOSAL_V1.md
  - tools/content-schema/quest-authoring/**
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Owner request (2026-09-27, this session): do three next steps.

1. **Declared tracks.** The quest catalogue declares every progress track that quest scripts write
   outside missions: 445 auxiliary tracks, each with its owning quest. No interaction writes an
   undeclared track any more. Storage aliases are expanded in both converters, which adds 216
   transitions to slice 3.
2. **Owner proposal.** `OTERYN_INTERACTION_RELOCATION_AND_WORLD_OBJECT_OWNERS_PROPOSAL_V1.md` is
   an owner decision package (CANDIDATE). It covers the 800 Movement and 1,117 WorldObject children
   that D36 blocks. It reuses VSL-MOVE-01 and the local transition candidate and adds no domain.
3. **NPC dialogue binding.** NPC transitions (1,442) name their dialogue in `requested_by`: the NPC
   authoring bundle key, the player keywords and the dialogue topics. A longer phrase becomes a
   text reference. No NPC-owned file changes.

Runtime, persistence and `content/**` stay unchanged.

## Architecture and source of truth

- `PROVEN`: pinned Canary and CrystalServer revisions; the NPC authoring census for NPC keys (read
  only).
- `DERIVED`: track owners from mission-track prefixes, script directories and `track_owners.json`.
- `UNKNOWN`: 81 auxiliary tracks belong to wiki quests not yet in the catalogue; 8 belong to no quest.

## High-risk authority/recovery qualification

`NOT_APPLICABLE` for the tooling. The owner proposal is a CANDIDATE document that changes no
authority; accepting it needs owner decisions and independent review.

## Acceptance and evidence

- The four converters are deterministic and validate together; `verify_quest_schema.py` 102/102.
- Every one of the 1,442 NPC transitions names an NPC present in the NPC census.
- No narrative text is committed: keyword phrases longer than two words are text references.
- `validate_governance.py` and `validate_repository_policy.py` pass.
- Jira: mapping to a programme Story not resolved in this session (pending).
