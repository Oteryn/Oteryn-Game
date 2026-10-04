# QUEST-LOWER-1

```yaml
task_id: QUEST-LOWER-1
title: "QUEST-LOWER-1 quest tracks and transitions lowered from source progress, and the catalogue loader"
mode: IMPLEMENT
status: in_progress
repository: Oteryn/Oteryn-Game
issue: 162
lane_id: content
base_branch: main
branch: claude/quest-lower-1
pr: null
base_sha: 53a60a6
owner: claude-code-session-01D5nNBpM7TsqwtTbqf5rePu (content worker)
control_plane: claude-code-session-013KJX6mv8LQveCKKXYgAX94
created_at: 2026-10-04
updated_at: 2026-10-04
packet: "docs/architecture/reviews/OTERYN_GAME_QUEST_STATE0_QUEST_PROGRESS_STORE_DECISION_2026-09-30.md row QUEST-LOWER-1 and §13.2 (owner decision D477)"
owned_paths:
  - tools/content-schema/quest-authoring/quest_state_lowering.py
  - tools/content-schema/quest-authoring/test_quest_state_lowering.py
  - tools/content-schema/quest-authoring/run_checks.py
  - content/quests/missions/quest-state.json
  - content/quests/missions/index.json
  - apps/game-server/src/quest/loader.rs
  - apps/game-server/src/quest/mod.rs   # `pub mod loader;` and two module-doc sentences only
  - docs/agents/tasks/QUEST-LOWER-1.md
depends_on:
  - "QUEST-STATE-1 merged (#1684, the catalogue type)"
  - "QUEST-CONTENT-1 in part (#1596, the Quest definitions)"
public_contracts: []
external_repositories: []
```

## Outcome

- **Generator** `tools/content-schema/quest-authoring/quest_state_lowering.py` (`--check` mode,
  wired into `run_checks.py`): reads the committed Quest definition shards and writes
  `content/quests/missions/quest-state.json`, flipping that directory's marker to `POPULATED`. The
  definition shards and the content manifest and lock are unchanged.
- **Lowered content:** 96 quests with source progress, 1,330 tracks and 3,263 transitions. Effects:
  2,889 `SET`, 133 `ADD`, 73 `SET_NOW` (`computed: timestamp`) and 168 `COMPUTED`
  (`computed: expression`, refused `NOT_SUPPORTED`, §4). 2,243 transitions carry `requested_by`.
- **Keys:** `oteryn:quest-progress/<path>` for Canary tracks, and
  `oteryn:quest-progress/crystalserver/<path>` for CrystalServer tracks. Transitions are
  `oteryn:quest-transition/<path>/<source key>`. Each source key stays as `source_key` or
  `source.key` beside the record and never reaches the store.
- **Loader** `apps/game-server/src/quest/loader.rs`: embeds the file and parses it into
  `QuestStateCatalogue` for the caller's content revision. It keeps the source track bindings and
  `requested_by` beside the catalogue. Any unknown field, comparison or effect, a record naming
  another quest, or a catalogue refusal fails the whole load closed. Production wiring of the
  catalogue is left to its callers.

## Lowering rules (stated assumptions)

- **Initial value:** -1 where the track's source data uses -1 (in a write, a comparison or a
  mission value); otherwise 0 (§3).
- **Bounds:** from the least to the greatest of the initial value and every written, compared
  and mission value. Two exceptions: an `ADD` or computed track is bounded above by the source
  storage width (2^31 - 1), and a `SET_NOW` track by `i64::MAX`.
- **Inexact comparisons:** a `from` comparison the source reads as not exact (a compound
  condition) is kept, so it fails closed. It is marked `from_exact: false`; there are 221.
- **Completion:** `completes: true` only where every mission of a quest reads one track, and only
  for a `SET` of that track to the greatest mission end value. That gives 6 quests and 4
  transitions. For the other 52 storyline quests completion needs all their mission tracks, so
  it stays unlowered (`NOT_LOWERED_MULTI_TRACK`). The 38 script-only quests have no missions
  (`NOT_LOWERED_NO_MISSIONS`).
- **Shared track:** the one track two definitions list (Shadows of Yalahar mission 13) belongs to
  the quest of its first mission.
- **Out of scope:** XP (`experience`, QUEST-XP-1), gates and triggers (QUEST-CONTENT-2), chest
  transition bindings.

## Validation

## Review
