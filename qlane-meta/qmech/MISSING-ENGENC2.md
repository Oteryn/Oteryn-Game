# ENGENC2 — checked missing mechanics

| quest | missing mechanic (server or client) | evidence (the quest file and step, plus the missing hook/table/command) | blocks the quest fully or partly |
|---|---|---|---|
| Cults of Tibia | Server: production admission/death routing and durable participant settlement for the shared Essence phase owner | `tools/content-schema/quest-authoring/batches/quests4/cults_of_tibia.json`, `step_inventory[79]` (Essence boss room) and `[86]` (Essence death → Humans.Mission=2). `crates/gameplay/quests/src/quest/world_callbacks/effects.rs::Encounter` has monsters/timing but no phase selector; `crates/gameplay/encounters/src/bosses/phase_controller/README.md` records the missing production registration and durable settlement. The shared native controller is foundation only. | partly: boss closure remains unavailable; dialogue/symbol slices exist |
| The Order of the Lion | Server: native Fugue creature definition and encounter death settlement | `tools/content-schema/quest-authoring/batches/quests4/the_order_of_lion_quest.json`, `step_inventory[33]` (spawn Fugue + five warlocks), `[34]` (death → Mission=3 and FugueSpawned=0). `content/creatures/definitions/spell-native-profiles.json` has no Fugue definition; native entry cannot execute that encounter without the creature profile. | partly: Fugue closure before final report |

Scope: these are verified code gaps, not an exhaustive claim about the other assigned encounters. The foundation is not counted as a finished quest or completed row. No migrations used; ENGENC2 retains 1800–1819.
