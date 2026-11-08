# NPCFIX — PARTIAL code audit

Only the entrypoint below has been checked so far. This is a missing binding in the audited shared-NPC path, not a claim that the existing fenced QuestState writer is missing. Other adapters and the remaining NPC scope still need inspection.

| quest | missing mechanic (server or client) | evidence (quest file and step, plus missing hook/table/command) | blocks the quest fully or partly |
|---|---|---|---|
| The Hidden City of Beregar — Way to Beregar | Server: no Maris (Mistrock) `mistrock` progress binding in the checked shared-NPC callback/offer registries. | `content/quests/missions/quest-state.json#/quests/70/tracks/12` declares `oteryn:quest-progress/quest/u8_4/the_hidden_city_of_beregar/way_to_beregar`. `content/quests/shared-reward-callbacks.json` and `content/quests/shared-reward-offers.json` each have zero `oteryn:npc.maris_mistrock` rows for the `NPC_TALK/mistrock` entrypoint. Crystal `201189f871d23597b82caf88fd611454a2ddefb8`, `data-global/npc/maris_mistrock.lua:68–82`, sha256 `4bcd7342494cb23d49db54e1dfd0026034a8df39e445aefb16f3a185d91d97b1`, sets this storage to1. | Partly: this checked NPC entrypoint lacks its registered transition; other quest entrypoints have not yet been audited. |

Migration range retained from NPCFIX's existing packet: **1230–1239**. No migration is proposed by this partial report.
