# Active shared-engine requests

Quest ownership stays with questpc4. Shared runtime features belong to qengine2. Existing native source programs and individual qualified interactions remain intact; whole quest status is not promoted by item custody.

- Native NPC teleport plus outfit/addon/achievement effects: The Paradox Tower Quest; The Order of the Stag Quest; In Service of Yalahar Quest. Existing NPC evaluator/item exchange effect whitelist does not support those effects. Reuse source programs; extend the shared owner.
- Native NPC minimum-level predicates: The Hidden City of Beregar Quest and its existing source program. Keep the source guard; do not drop it to force a planner match.
- Subtype-aware NPC item grants and guards: In Service of Yalahar Quest, rum flask5552. `crates/gameplay/quests/src/quest/npc_rules/item_exchange.rs` admits subtypeNone only; no empty/RUM substitution.
- Shared world-object reset epochs and map-container pickup custody: Dreamer's Challenge Quest; Lion's Rock Quest; Machinery of War Quest; Skeleton Decoration Quest. `crates/gameplay/quests/src/quest/world_callbacks.rs`; personal once-reward delivery does not model shared reset custody. Source blockers retained in LANE-L6.json.
- Improper-level weapon penalties (`unproperly=true`): The Outlaw Camp Quest, bright sword3295. `crates/gameplay/dispatch/src/gameplay_transport/equipment_move.rs`; pinned Crystal201189f `data/items/items.xml` item3295. Do not silently narrow or erase the source rule.

Reuse check: qengine2 fetched3babb30bd. Authored trigger/delivery bindings including native boot admission are QUALIFIED (614d00506,192de4928; seven realPG confirmations). Definition-ready claim use is still being reconciled with mainQREADY1+2. Reuse the qualified engine path after this item batch; do not merge unqualified WIP claim work solely to erase holds.
