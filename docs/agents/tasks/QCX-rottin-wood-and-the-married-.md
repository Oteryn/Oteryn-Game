# QCX72 Rottin Wood and the Married Men Quest

Base `871c1dec7d9963420d455210e4cddc0c9df80d26`. Facts from
`460e1d8c2c0e88c438ae5e54910db00822dc0129:qcx-facts/rottin-wood-and-the-married-.md`.
[TibiaWiki quest, curid 50684, oldid 844603](https://tibia.fandom.com/?curid=50684&oldid=844603)
and [TibiaWiki spoiler, curid 50782, oldid 928967](https://tibia.fandom.com/?curid=50782&oldid=928967).
The Crystal 201189f8 comparison uses the repository's frozen 24-row
[donor inventory](../evidence/quests-2-20261102/rottin_wood_and_married_men.json)
and the matching pinned [Rottin Wood script](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/rottin_wood.lua).

| Part | Global facts and comparison with base |
| --- | --- |
| Requirements | Level 0, free account and quest log already match the definition. Level 20 is recommended. The spoiler calls for either knife and a hammer, and describes the kitchen knife's daily table spawn. |
| Lucky Charms | The global transcript opens with `quest`. The native callback accepted only `mission`. Added the opening alias before Mission 1 begins. Accepting and replacing seven ribbons, delayed corpse skinning, the seven-success cap, crafting seven charms, and the hand-in for 100 XP and 500 gp are absent from the playable route. Crystal's captured script lists 12171/12172/12173 for ribbon/foot/charm. |
| In a State of Disrepair | Global uses two confirmations. Base combined both replies and committed after one. The route now splits the existing source reply and commits after the second confirmation. Six hammer repairs, 120-second resets including reuse of the same wall, and 500 XP with 500 gp at hand-in lack active bindings. Crystal records six action IDs 42501–42506. The only overlay counter write on base is the initial reset to zero. |
| Bushwhacking | Global gives five nets, kicks the player out on the fifth placement, blocks re-entry with a tree for five minutes and asks for five Captured Merchants. The donor spawns seven Travelling Merchants and converts remaining traps during cleanup. There is no active net, trap, scene, loot or replenishment binding for this quest. The donor's completion gives 1000 XP, 500 gp and Merry Adventures, matching the global completion reward. No NPC completion binding pays it on base. |
| Lucky Loop | Global awards Bunny Slipped for seven fake feet, crafted with ribbons, wooden spoons and wool, and pays 300 gp in that case. Crystal awards the achievement on ordinary charm turn-in and lacks the fake-foot route. Global lists 100 XP and genuine rewards of 500 gp (90%), or 300 gp plus a Time Ring (9%), or a Hunting Horn (listed as 1%?). Donor uses ring 3053 and horn 12260. No active turn-in or craft bindings for the task were found. |
| Handyman | Global requires six repairs and pays 500 XP and 500 gp. The native `task` menu exists but the repair offer, repairs and hand-in are deferred. |
| Reward claims and placement | The definition has empty claim/gate lists. Rewards are described as NPC hand-ins, with five merchant loots in Mission 3. No dedicated quest records were found in reward claims, active or held world callback directories, NPC offer/callback directories, or structured placement records by quest/name. The source captures direct use of wall action IDs and item types. The facts list no outfit or permanent quest-granted access. The temporary Bushwhacking tree is the documented access change. |

## BLOCKER

- BLOCKER Lucky Charms acceptance and ribbon refill Rottin-specific item/state transaction and refill clock integration outside the active classic adapter data-global/npc/rottin_wood.lua:146–172,313–325
- BLOCKER Lucky Charms skinning corpse age and capped probabilistic quest progress dispatch data-global/scripts/actions/tools/skinning.lua:123–146
- BLOCKER Lucky Charms and Lucky Loop recipes quest-gated item-use recipes and state binding data-global/scripts/actions/tools/lucky_charm.lua:3–22
- BLOCKER Disrepair and Handyman repairs atomic wall transform and timer reset with character counter data-global/scripts/actions/tools/hammer.lua:64–101
- BLOCKER Bushwhacking scene net placement, merchant spawn/capture and timed area cleanup integration data-global/scripts/quests/rottin_wood_and_married_men/actions_trap.lua:1–123
- BLOCKER Bushwhacking merchant loot atomic scene-object removal, counter and gold transaction data-global/scripts/quests/rottin_wood_and_married_men/actions_corpse.lua:3–36
- BLOCKER mission and task turn-ins Rottin inventory, XP, achievement and progress transaction dispatch outside the active classic adapter data-global/npc/rottin_wood.lua:162–194,117–138,238–291
- BLOCKER Lucky Loop and Handyman task lifecycle Rottin task deadline and native random reward integration data-global/npc/rottin_wood.lua:222–291,327–362

## OPEN-FACTS

Global facts omit numerical item/achievement IDs, exact rabbit-drop chances, the exact ribbon refill interval, exact daily cooldowns, the horn's confirmed probability, the daily knife respawn timing, placement action IDs and exact server positions for the structures/trap scene. The facts give a 2500 gp headline and separate payout descriptions. They do not explicitly reconcile the headline with repeatable and variable rewards.

## PARTIAL

The NPC dialogue corrections cover the currently playable conversation and Disrepair start. Ribbon/net grants and refills, kitchen knife respawn, corpse skinning and crafting, timed repairs, the merchant scene and loot, all reward payouts and both daily task implementations remain open. The report records Crystal-only quantities as donor evidence.

## Verification

The mounted case reuses the classic fixture and drives actual NPC dispatch with PostgreSQL quest state. It asserts the opening reply, the split Disrepair offer, unchanged progress after the first confirmation, the three writes after the second confirmation, the single receipt and the existing incomplete repair rejection. The dialogue changes reuse the table's literal replies in Rust, with zero changes to generator inputs or pins.
