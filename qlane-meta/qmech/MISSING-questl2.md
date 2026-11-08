# L2 missing mechanics

Checked against qmech `e278b09a75f13e089d3e903bfb2ee399b1fdc9dd` plus this lane's M39 implementation.
This is a mechanic report, not a claim that every quest in the older packet is qualified.

M39 now implements `any_topic` in `npc_command/reward_callbacks/offer_topic.rs`,
inclusive inventory guards in `quest/reward_callbacks/inventory.rs`, and their
transactional enforcement in `durability/quest_world_callback/reward_inventory.rs`.
Scoped refusal replies are handled by `npc_command/quest_dialogue.rs`.
Into the Bone Pit is wired through the two `m39-stories.json` shards; both native
PG journeys passed (78.255105046 s / 78.718498797 s, PostgreSQL 18.6), including
canonical-writer rejection of a false loss claim and literal command replay.
Tomes of Knowledge's authoring is not enabled by this slice; its missing
`any_topic` engine capability is now available and covered by the M39 native conversation test.

| quest | missing mechanic (server or client) | evidence (quest file and step, missing hook/table/command) | blocks fully or partly |
|---|---|---|---|
| Unnatural Selection | Server and client: native diagonal movement commands for the complete dance route. | `content/quests/world-callbacks.json:8719` onward contains dance callbacks citing `data-global/scripts/quests/unnatural_selection/movements_mission3_dance_dance_evolution.lua:1-67`. `postgres_tests/quest_world_callbacks/unnatural_selection.rs:1-6` explicitly leaves diagonal movement unqualified and arranges fifteen setup steps through the writer. `crates/protocol-oteryn/src/world_spatial.rs:41-46` still defines only cardinal `StepDirection` variants. | Partly: the complete native dance journey. |

Current coordinator-assigned migration range: **2020-2029**; M39 uses **2020**
for the existing reward trigger's quantity-variable collision with the independent-output SQL alias.
