# ENGNPC missing mechanics

**PARTIAL** — verified server NPC scope only; remaining requests are still being checked.

Checked qmech `7990616c93c1ee281fd93febbff0d2bfa95375af`. This report does not claim whole-quest qualification.

| quest | missing mechanic (server or client) | evidence (quest file and step, plus missing hook/table/command) | blocks the quest fully or partly |
|---|---|---|---|
| Elemental Spheres Quest | Server: fenced real-level and vocation eligibility in native NPC item programmes | `content/quests/definitions/quests-00100-00199.json#/records/37/definition/oteryn_recipe/payload/recipe/stages/0`: s1 asks Arkulius about the elemental-sphere mission. `crates/gameplay/durability/src/durability/quest_state/npc_program.rs::evaluate_source` supplies `level: 0` and `vocation: ""` instead of current Character facts. Its item planner also excludes `Level`, `Vocation` guards. Missing hook: pass live Character progression/build facts into preview and recheck the locked facts inside the existing NPC item/quest transaction. | Partly: Arkulius admission and guarded item exchanges; other stages are not assessed here. |
