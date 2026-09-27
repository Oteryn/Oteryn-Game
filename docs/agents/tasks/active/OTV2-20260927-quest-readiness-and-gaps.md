# OTV2-20260927-quest-readiness-and-gaps

```yaml
task_id: OTV2-20260927-quest-readiness-and-gaps
title: Quest format - readiness map, map check, dedicated doors, fewer unresolved interaction parts
mode: CONTRACT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/zealous-edison-3ttg1s
issue: 162
pr: 1041
base_sha: e68893a8352cc242c1b41ad2b33087c911ce63cb
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude-code-session-01AVd6BKKTRbeW1Pub9bg9Jk
created_at: 2026-09-27
updated_at: 2026-09-27
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/OTV2-20260927-quest-readiness-and-gaps.md
  - docs/agents/tasks/archive/OTV2-20260927-quest-tracks-dialogue-owners.md
  - docs/architecture/OTERYN_QUEST_AUTHORING_FORMAT_V1.md
  - tools/content-schema/quest-authoring/**
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Owner request (2026-09-27, this session): while the engine lacks the runtime owners, prepare the
quest data as far as possible, with a map of which engine features each quest needs.

1. **Readiness map** (`ots_readiness.py`, quest format §6.7): per quest, the features it needs, its
   data gaps and the unlock order. The `USE` trigger and reward claim alone complete 76 of 157
   quests on the engine side.
2. **Fewer unresolved interaction parts** (§3.3, §6.3): `actor_item_count` condition, getter-form
   object conditions, Outfit, Mount and Experience grants, map marks, reward container contents,
   static-table storage keys, and reasons that name the missing owner. Mapped interactions 236 to
   256; unresolved statements 2,059 to 1,908.
3. **Dedicated-script doors** (§6.1): the Katana lever door is a `lever` gate. The Dawnport
   vocation doors are an interaction, not a gate, and stay unresolved with that reason.
4. **Map check** (`ots_map_check.py`, §6.6): every chest item is on its tile, every anchor tile
   exists, and 284 of 349 trigger ids are on the map or stamped at startup. The two open map
   questions are answered.

Runtime, persistence, `content/**` and NPC-owned files stay unchanged. No map is committed.

## Architecture and source of truth

- `PROVEN`: pinned Canary and CrystalServer revisions; Canary `otservbr.otbm` v3.6.1 and
  CrystalServer `world.otbm`, both pinned by sha256.
- `DERIVED`: the readiness map and the map check, from the committed samples.
- `UNKNOWN`: 65 trigger ids not found on the startup map; 346 interactions that join no catalogue
  quest (mostly quests the catalogue does not have yet).

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: offline tooling and a CANDIDATE format document; no authority, persistence or
runtime change.

## Acceptance and evidence

- The converters, the readiness map and the map check are deterministic; `verify_quest_schema.py`
  passes; `validate_quest_content.py` reports 0 errors.
- No narrative text is committed: messages and map marks keep source lines only.
- `validate_governance.py` and `validate_repository_policy.py` pass.
- Jira: mapping to a programme Story not resolved in this session (pending).
