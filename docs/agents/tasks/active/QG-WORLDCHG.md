# QG-WORLDCHG

```yaml
task_id: QG-WORLDCHG
title: Quest lane qg - mini world changes and world changes to Tibia global parity
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/happy-davinci-ijz3wy-qg
pr: null
issue: 1622
owner: quest build lane qg (Content session, owner instruction 2026-11-17)
created_at: 2026-11-17T00:00:00Z
updated_at: 2026-11-17T00:00:00Z
owned_paths:
  - crates/gameplay/world-change/src/daily.rs
  - apps/game-server/src/node/serve/daily_world_changes.rs
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/daily_world_gates.rs
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/worldchg_returns.rs
  - tools/content-schema/world-authoring/quest_startup_bindings.py
  - tools/content-schema/world-authoring/test_quest_startup_bindings.py
  - tools/content-schema/world-authoring/samples/quest-startup-bindings.json
  - tools/content-schema/world-authoring/samples/world-base-capture-v1.json
  - content/world/placements/
  - content/world/pins/
  - docs/agents/tasks/active/QG-WORLDCHG.md
public_contracts: []
depends_on: []
```

## Scope

The 18 world changes of lane qg: fury_gates, grimvale, horse_station, awash, bored,
chyllfroest, deeplings, nightmare_isles, oriental_trader, overhunting, stampede,
steamship, swamp_fever, thawing, the_mages_tower, the_mummys_curse, their_masters_voice,
thornfire. Done bar: E2E passes and triggers, effects and rewards match Tibia global
(TibiaWiki cited); global wins over a Canary/Crystal donor.

## Batch 1 parity table

| Rule | Source (class) | Donor | Winner |
|---|---|---|---|
| Fury Gates: the Fury Dungeon archway (776, aid 9715, 33290,31785,13) returns to the active gate city | TibiaWiki Fury Gates Mini World Change rev 1099676 (TIBIAWIKI_STRUCTURED) | Crystal 201189f `data-global/startup/tables/item.lua:1689`, `fury_gates.lua` (OTS_HYPOTHESIS_ONLY) | agree; the base `world.otbm` lacked the selector, restored by `quest_startup_bindings.py` |
| Nightmare Isles: the isle exit (10840, uid 35020, 33498,32613,8) returns to the active side | TibiaWiki Nightmare Isles Mini World Change rev 891482 | Crystal 201189f `data-global/startup/tables/teleport.lua:596`, `nightmare_isles.lua` | agree; selector restored the same way |
| Thawing: 15 Ice Flowers on the Ice Islands floor 5, the 15th south of the immovable Mammoth (Mapper 125.152,121.55 = 32152,31031,5) | TibiaWiki Thawing Mini World Change rev 890265 | Crystal `thawing.lua:4-19` lists 14 | global: 15 flowers |

Held, with evidence, outside this batch:

- Nightmare Isles ladders (aids 64103-64128): the donor gives destinations only, not the
  ladder source positions, and the real map carries none; no positions are invented.
- Si, Ariki! on trade open (TibiaWiki Si, Ariki!; Crystal `yasir.lua:820`): Oteryn grants it
  on the first committed trade (`npc_trade_achievement.rs`). Moving the grant to the
  window open needs a durable idempotent grant source without a TransactionId; sent to the
  server lane.
- Server mechanics for Awash, Steamship, Swamp Fever, Chyllfroest, Mummy's Curse, Thornfire,
  Horse Station, Bored, Deeplings and parts of The Mage's Tower and Their Master's Voice were
  sent to the server lane (session_01H5hdvRcH9cRdJH2RatxpTE).

## Validation

Recorded per batch in the READY message to the coordinator.
