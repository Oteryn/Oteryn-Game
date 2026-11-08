# questpc3-n3 current engine handoff

Quest ownership stays with questpc3. This file makes the current requests discoverable under the coordinator's `qlane-meta/*/ENGINE-REQUESTS.*` convention. It does not claim engine delivery or new playable completion.

## Existing QENGINE2 queue: reuse these owners

Inspected `claude/happy-davinci-ijz3wy-qengine2` at `925d8480ef1cb88b16154a1dbf018a57ac22152a`, `qlane-meta/qengine2/ENGINE-QUEUE.json`. Do not create competing implementations.

| Owner | Feature key | Quest | Delivery state |
|---|---|---|---|
| QENGINE2 | ground_item_mint_and_ground_use_with | The Beginning Quest | QUEUED |
| QENGINE2 | world_callback_transaction_clock_operations | Threatened Dreams Quest | IN_PROGRESS; published WIP, not on main |
| QENGINE2 | annual_npc_dialogue_calendar_gate | The First Dragon Quest | QUEUED |
| QENGINE2 | conditional_world_reward_achievement | The First Dragon Quest | QUEUED |
| QENGINE2 | step_in_boss_cooldown_and_cleanup | The First Dragon Quest | QUEUED |
| QENGINE2 | live_creature_item_use_transform_and_credit | The Tainted Souls Quest | QUEUED |

ENGINE-REQUEST questpc3-n3: retain the six existing queue entries above through delivery; requesting lane binds/tests the complete source journeys | owner: QENGINE2 | where: gameplay/quests world_callbacks and gameplay/dispatch quest_callbacks

## Items/Content prerequisite: whole-target transformation

ENGINE-REQUEST questpc3-n3: qualify recipe-scoped captured-target quantity semantics before composing the Tainted talisman transaction | owner: Items/Content; ENGITEM composition only after frontend qualification | needed by: The Tainted Souls Quest | where: crates/gameplay/items/src/item_craft.rs:152-195 and the qualified ItemConversion content model

ENGITEM `183f49bc11a1e84ce1633b40e19b54a1b952da51` explicitly classifies this as legacy request15/B4. Its active queue covers other assigned lanes, and Items/Content are outside that allocation. This is a prerequisite/ownership boundary, not a claim that ENGITEM is currently implementing it. Coordinator routing must retain the frontend requirement; multiplying an output quantity inside durability is not an implementation.

Current main `carried_row` requires both operands' count to be1 and emits quantity1 for transformed operands. A whole captured target stack therefore cannot be represented by the existing fixed-unit recipe. Do not weaken the fixed-unit guard for unrelated recipes.

Retained donor evidence is recorded in prior pushed `dc9e763c856503bfd897e50238b99002f5791005:qlane-meta/questpc3/ENGINE-REQUESTS.md` (Tainted refinement) and its CONTRACT-NOTES.md:

- Crystal tree `9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d`, `data-global/scripts/quests/the_tainted_soul/actions_star_herb.lua:8-11`, blob `d76b2fcd9c9411ee9839f55e9d5d9c563a73e50d`, SHA256 `f3ffe07cf0538c6a072cc0145e340ea9bf1a74a83601e3e7c42c6a90d03f13e4`: verified the retained492-byte capture against the committed donor inventory. It transforms antlers10297 to talisman22008 without an explicit quantity and consumes one herb3736 (registration line18).
- `src/lua/functions/items/item_functions.cpp:802-841`, blob `86afbd32a4f0c12d5ae62b62e280d80c2d4b61c4`, captured SHA256 `48e27deb9bbfd53b0c7917183e1bdbccf7d1e14242eace2f0d96ef1b4ac11732`: omitted subtype/count is `-1`.
- `src/game/game.cpp:3125-3148`, blob `0d92f0ad2c25ddde690488729e33ec036254f6b8`, captured SHA256 `4b26d0c5168e0907ae9044b2587a37ba647f6b6651a94e02f1c12a2e3af58d45`: preserve source quantity in the applicable transform branch. Qualify the stack-to-nonstack holy-water branch separately; do not infer seventeen nonstack outputs or silently discard a remainder.

Release proof must drive selected native carried USE_WITH against real PostgreSQL: captured stack17 plus herb1, exact consumed/replaced quantities and selected identity/attributes, rollback on refusal, exact replay, and the separately source-qualified nonstack policy. Do not seed the expected transformed output. Earlier physical-custody proofs are not recipe proofs.

The old lane already prepared22008/22009 custody and source facts. Reuse that collector delivery; do not redo their admission or silently replace live-creature soul release with kills. Live-creature transformation/credit remains the separate QENGINE2 queue entry above.

## Beginning bridge boundary

ENGINE-REQUEST questpc3-n3: source-bounded read-only level-range StepIn with an authoritative refusal destination | owner: QENGINE2 admission plus World movement owner | needed by: The Beginning Quest | where: gameplay/quests/world_callbacks.rs and native movement admission

This is ENGITEM legacy request18/B1/B7, not part of its active assigned queue. Main exposes `minimum_level` but not a general maximum-level/refusal-destination contract. Reuse the scoped immutable callback admission owner; do not invent quest storage to stand in for current progression or teleport after an unrelated reward commit. The requesting lane retains the exact source bounds and bridge/log8 journey; generic engine fixtures may use explicitly admitted fixture bounds.

## Source decisions retained for delivery

These are references to the existing owned capture, not replacement reference-data dumps:

- Beginning: `data-global/npc/zirella.lua:109-143`, blob `1b9058cff0c8b29783daaae30038c26b7d731072`, captured SHA256 `d38ed72ad6088b832d09380ca10e79d0e3f8abd9c06bef34c9fa6308d1046380`. Preserve the selected five-branch physical delivery contract, including movable ground custody and selected cart USE_WITH.
- First Dragon season: `data-global/npc/tamoril.lua:45-63`, blob `2434aac117de37173927fad61dcf9b686cf97e02`, captured SHA256 `d5d55a74c2bb9ade413c23442924b46ee6e5b05c9daf73ed8259e67f04576969`. January14 through February12; month-only guards and NPC despawn are not equivalent dialogue behavior.
- First Dragon treasure: `data-global/scripts/quests/the_first_dragon/actions_treasure_chest.lua:94-109`, blob `edbcba3184fc3adfe3db6ef580a38eae9c84f9e1`, captured SHA256 `8481fd65e997b7422192f4a85b6ae0a7c012afebe5eb561cde80bfe891ec761f`. Twenty distinct UIDs14001-14020; conditional Treasure Hunter participant belongs in the same item-grant transaction, not a later inferred write.
- First Dragon entry: `data-global/scripts/quests/the_first_dragon/movements_entrance_teleport.lua:41-87`, blob `8543eff38c536b247ca605cdb1972fe603388863`, captured SHA256 `d7fd9593fb416160ed4b1d63e53890ef3d3ae8db8f559a15750c9bd062c094a0`. Reuse occupied-room checks and named personal two-hour cooldown; literal cleanup is30minutes despite ten-minute speech. The four source exits already have a qualified earlier lane slice; do not redo it.
- Threatened harvest: `data-global/scripts/quests/threatened_dreams_quest/event_raven_herb_bush.lua:46-53`, blob `84bb7ea71ddbd0457f714c31952a6822724276a1`, captured SHA256 `ac2c7fd19872d03ed0dd365963715ed2343ea4e27327f3ed73867215fdc2ea01`. Literal `60*30*1000` is1800000 seconds against `os.time`; do not silently reinterpret it as30minutes. QENGINE2's existing queue preserves this decision.

All four unfinished journeys remain WAITING-ENGINE. Pilgrimage/Stampede remain BLOCKED on prior collector delivery; this request file does not reassign those quests or expand scope to the sixteen excluded packet rows.
