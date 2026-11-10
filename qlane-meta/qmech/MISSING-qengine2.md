# QENGINE2 missing mechanics — PARTIAL

Checked against the qmech package source at `aca84855776f1f68e3d2d61bf720366c6b46d1cf`.
This is a partial code audit, not an end-to-end quest qualification. “Partly” means
that the named stage cannot currently execute its required mechanic; it does not
claim that the other stages are playable. Client mechanics have not been audited.
The qualified shared-engine transfer is pending; the later clock WIP is excluded.

| quest | missing mechanic (server or client) | evidence (the quest file and step, plus the missing hook/table/command) | blocks the quest fully or partly |
|---|---|---|---|
| 25 Years of Tibia Quest | Server: fenced live blessing and promotion predicates for world admission | `content/quests/definitions/quests-00200-00299.json`, identity `oteryn:quest.25_years_of_tibia_quest`, `oteryn_recipe.payload.recipe.stages[s1]` (time-machine entry). `crates/gameplay/quests/src/quest/world_callbacks.rs`, `Callback`, exposes `minimum_level` but no blessing/promotion requirement. `crates/gameplay/durability/src/durability/quest_world_callback.rs::consume` checks live progression level, not the held blessings or durable promotion. | Partly: admission prerequisites cannot be enforced through this callback. |
| The Pits of Inferno Quest | Server: live Premium predicate on initial world STEP admission | `content/quests/definitions/quests-00000-00099.json`, identity `oteryn:quest.the_pits_of_inferno_quest`, `oteryn_recipe.payload.recipe.stages[s1]` (infernal-hub entrance). The same `Callback` and `quest_world_callback.rs::consume` expose/check minimum level, but no Premium admission requirement. A reward callback's Premium check does not supply a STEP admission check. | Partly: initial admission. |
| Bewitched | Server: authoritative transaction-clock operations for world callbacks | `content/quests/definitions/quests-00200-00299.json`, identity `oteryn:quest.authored.bewitched`, `recipe.stages[s4]` (cauldron contribution) and `repeat`. `world_callbacks.rs::Callback` has fixed `Storage` predicates but no `state_operations` field; the package cannot author the existing reward-callback elapsed-time/SetNow operations on this world callback. | Partly: timed repeat contribution. |
| Goblin Merchant Quest | Server: carried-tool quest callback targeting an exact live creature, with atomic transform/removal and credit | `content/quests/definitions/quests-00100-00199.json`, identity `oteryn:quest.goblin_merchant_quest`, `oteryn_recipe.payload.recipe.stages[s3]` (live Rotworm sampling) and `[s4]` (Guardcatcher capture). `world_callbacks.rs::ItemOnTarget` models a carried item against the selected world object, with appearance replacement; it has no live creature occurrence/reference or removal/credit participant. | Partly: sampling and capture stages; existing world-object item use is insufficient. |
| Nomads Land Quest | Server: exact ordered mutable Ground stack predicate | `content/quests/definitions/quests-00300-00351.json`, identity `oteryn:quest.authored.nomads_land_quest`, `recipe.stages[s4]` (crates/amphora window obstruction). `world_callbacks.rs::WorldRequirement` and `world_callbacks/effects.rs::ItemPresence` contain position/appearance only, without quantity, stack ordinal or Ground custody identity. `dispatch/.../quest_callbacks/effects.rs::prepare_quest_projection` admits ItemPresence only for a known non-pickupable ladder, not portable crate/amphora stacks. | Partly: obstruction puzzle admission. |
| Targuna Quest | Server: committed personal map-marker effect from NPC quest dialogue | `content/quests/definitions/quests-00200-00299.json`, identity `oteryn:quest.targuna_quest`, `oteryn_recipe.payload.recipe.stages[s3]` (Sterling marks the digging site). `crates/gameplay/quests/src/quest/npc_rules.rs::NpcRuleEffect::MapMark` exists, but `npc_rules/evaluate.rs` groups it into the no-op effect arm. Searching dispatch and durability for `MapMark` found no handler; this NPC effect does not commit a personal map marker. | Partly: marked-site delivery. |

Evidence paths abbreviated as `world_callbacks.rs` above are under
`crates/gameplay/quests/src/quest/`; `dispatch/...` is
`crates/gameplay/dispatch/src/gameplay_transport/`.

The durable blessing/promotion owners already exist. The missing part reported
here is their use by the world callback admission owner, not a new ownership table.
