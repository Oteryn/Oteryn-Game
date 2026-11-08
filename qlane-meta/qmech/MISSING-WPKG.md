# WPKG checked missing mechanics

Scope: the three L2-ready journeys reused from `66aef906b`, prepared for QMECH; its runtime transfer is still in local validation.
The source journeys already have reward/replay PostgreSQL stories. This report
separately checks personal terminal completion; it does not infer client gaps
or certify any other collector member. No migration is needed for the existing
completion primitive.

| quest | missing mechanic (server or client) | evidence (the quest file and step, plus the missing hook/table/command) | blocks the quest fully or partly |
|---|---|---|---|
| Djinn War Efreet | Server wiring: final NPC reward does not mark its owning quest personally complete. The existing completion mechanic can be reused. | `content/quests/definitions/quests-00200-00299.json`, record 19, final Malor mission report; `content/quests/npc-callbacks/l2-ready-stories.json`, callback `oteryn:quest-reward/l2-efreet-finish`; producer `questl2_djinn.py::append` writes mission03 2→3 and Efreet Ally but omits `completes`. `RewardCallback.completes` defaults false in `crates/gameplay/quests/src/quest/reward_callbacks.rs`, then enters the transition plan there. Missing hook: `completes: true` on this exact terminal callback, with a PG assertion on `game_character_quest_states.completed_character_revision`. | Partly: reward, achievement and trade access work; the personal terminal marker is not requested. |
| Djinn War Marid | Server wiring: final NPC reward does not mark its owning quest personally complete. The existing completion mechanic can be reused. | Same quest shard, record 20, final Gabel mission report; `l2-ready-stories.json`, callback `oteryn:quest-reward/l2-marid-finish`; `questl2_djinn.py::append` writes mission03 2→3 and Marid Ally but omits `completes`. Same existing transition-plan hook and completion table as above. | Partly: reward, achievement and trade access work; the personal terminal marker is not requested. |
| White Raven Monastery | Server wiring: the final diary-for-Ankh hand-in does not mark its owning quest personally complete. The existing completion mechanic can be reused. | Same quest shard, record 48, Costello diary hand-in; `l2-ready-stories.json`, callback `oteryn:quest-reward/l2-white-raven-diary`; producer `questl2_passages.py::append` consumes diary3212, grants Ankh3214 and advances diary1→2 but omits `completes`. Same existing transition-plan hook and completion table as above. | Partly: paid passage, catacomb access and final reward work; the personal terminal marker is not requested. |

The candidate’s carried-item, quest-gated shop and committed world-stage synchronization
paths are present in this L2 transfer. The shop writer rechecks source quest
requirements under its existing Character transaction in
`durability/npc_trade/quest_shop.rs::permitted`; display eligibility alone grants
no trade authority. No additional generic engine or client mechanic was found
missing in the three checked source reward journeys.
