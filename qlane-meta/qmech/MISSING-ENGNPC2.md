# ENGNPC2 checked missing mechanics

**Scoped audit completed; M11 resolved in `eb9aae7e3828e112982be054de300827085ae039`.** The table records the checked request/base gaps at the original audit; it is not a claim that other lanes’ current QMECH work is still missing. The coordinator’s `ASSIGN.md` assigns only M11 to ENGNPC2; all other rows below are owned elsewhere. The finished equipped-head capability `88d39c9d9` is already in QMECH.

Scope: the ENGNPC-tagged engine requests assigned to this lane, not an audit of
every mechanic in each quest. No client deficiency is asserted here.
Quest recipe stages were read from the following immutable request branches:

- questpc2-n2-n3 `f7cd80b802d2275752ab6be8509d6ad9d95da175`.
- questpc7-n3 `2caf66704f735bde02cb68370a5b5e00dd2d87a6`.

In the table, `D1` is `content/quests/definitions/quests-00100-00199.json` and
`D2` is `content/quests/definitions/quests-00200-00299.json`. Each record pointer
is followed by the checked `oteryn_recipe.payload.recipe.stages` key.
"Partly" identifies the specific blocked stage; it does not certify other stages.

| quest | missing mechanic (server or client) | evidence (the quest file and step, plus the missing hook/table/command) | blocks the quest fully or partly |
| --- | --- | --- | --- |
| Formorgar Mines / The Ice Islands | Server: three distinct live Baby Seal painting receipts | questpc7 `D2#/records/27/definition`, s4 paints three seals. `dispatch/.../cosmetic_item_use.rs::use_carried_quest_item` refuses actor `use_with`; the carried quest owner has no exact creature-occurrence participant or distinct painting receipt. The request also names capture/release, but this report does not infer that every release requires a new primitive: s3's mast is an object target. | Partly: s4 |
| Goblin Merchant | Server: carried-tool USE on the exact fresh native Ground Slug corpse | questpc7 `D1#/records/51/definition`, s3, Slug odour. The same carried route refuses all targets; `dispatch/.../quest_callbacks.rs` handles map-object `WorldObjectTarget`, without a native Ground-corpse Instance-to-quest USE adapter. The native death producer proof in questpc7 is not such an adapter. | Partly: s3 |
| Goblin Merchant | Server: live Rotworm sampling with exact tool transformation and progress | questpc7 `D1#/records/51/definition`, s3, Rotworm odour. `cosmetic_item_use.rs::use_carried_quest_item` explicitly rejects an actor `use_with` target. The actor-target dispatch route calls mount taming, not a quest sampling participant. | Partly: s3 |
| Goblin Merchant | Server: exact live Adventurer capture, once per distinct occurrence | questpc7 `D1#/records/51/definition`, s4, three Guardcatcher captures. The same rejected carried-target route has no quest receipt binding the selected live actor generation to tool consumption/transform and personal progress. | Partly: s4 |
| Tower Defence | Server: source NPC four-grant direct-backpack MINT bridge | questpc2 `D2#/records/55/definition`, s5/s6, reporting the password to Bunny; the engine request specifies four distinct fifty-arrow outputs. `quests/.../npc_rules/item_exchange.rs::item_exchange_keys` rejects `grants > 1`; `durability/.../reward_claim_mint/npc_program.rs::claim` destructures exactly one gift. L2's `backpack_outputs.rs` exists on `be6626b49508c33b062c097480ed173472a8bdba` but is absent from the inspected engine base. | Partly: final report/reward |
| The Dream Courts | RESOLVED M11: source dialogue predicate over the current Character name | questpc2 `D2#/records/21/definition`, s3 haunted-house/cellar/temple/tomb passages; `qlane-meta/questpc2/ENGINE-REQUESTS.md` identifies Stricken Soul's confirmation/name branch. `quests/.../npc_rules.rs::NpcRuleInput` has no current name; `npc_rules/evaluate.rs::words_match` reads literal source keywords only. Original gap is now closed by `npc_rules/player_name.rs`, the transactional current-name read in `durability/.../quest_state/npc_program.rs`, and `content/dialogues/quest-rules/quest-batch3-engnpc2-dream-courts.json`. | Unblocked: Stricken Soul entry branch; later quest mechanics remain separate |
| The Demon Oak | Server: one source NPC receipt for Axe, exact 1000 gold including bank, Hallowed Axe and committed visual | questpc2 `D1#/records/31/definition`, s3, Oldrak's Hallowed Axe preparation; the engine request specifies the exact price. The plain source planner rejects `Money`/`TakeMoney`; `item_fee_burn/purchase_cause.rs` and its audit validator accept quest-payment keys only under `oteryn:quest-reward/`. Migration 1000's fee proof is coupled to `game_quest_reward_sources`, not the source NPC program/item receipt. The new visual projection alone does not compose these owners. | Partly: s3 |
| Elemental Spheres | Server: live level/vocation predicates in the source NPC item transaction | questpc2 `D1#/records/37/definition`, s1, Arkulius assignment. `durability/.../quest_state/npc_program.rs::evaluate_source` supplies `level: 0` and `vocation: ""`; the plain item planner rejects `Level` and `Vocation` guards. | Partly: s1/source item admission |
| Dangerous Depths | Server: source NPC item/points/SetNowPlus receipt (ENGNPC owns this row) | questpc2 `D1#/records/27/definition`, s2, Gnomal Warming measurements and Uncharted Territory reports. `npc_rules/item_exchange.rs::item_exchange_keys` has no `StorageTime` guard or `SetNowPlus` effect arm. Sixteen-track capacity exists; it is not this remaining composition. ENGNPC was explicitly assigned the row and remains its owner. | Partly: s2 reports |

Engine paths abbreviate `crates/gameplay/<crate>/src/...`; all named hooks and
closed allowlists were inspected. The equipped-head source predicate is delivered. The earlier local NPC visual
candidate was not imported because the coordinator assigned its composition to
QNPCFX/M10. That candidate is preserved locally, outside M11 delivery.
This report does not claim complete quest playability.

Queued reuse, not another implementation: QENGINE2's checked queue at
`925d8480ef1cb88b16154a1dbf018a57ac22152a` lists
`live_creature_item_use_transform_and_credit`; its scoped authored-binding APIs
are also absent from the inspected base. Recheck QMECH and CLAIMS before taking
any of the live-target rows. Do not replace real creatures/corpses with props,
fabricate a reward bag, or turn equipped items/live player facts into quest tracks.
