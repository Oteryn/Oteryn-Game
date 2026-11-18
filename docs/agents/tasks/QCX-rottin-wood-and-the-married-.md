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
| Requirements | Level 0, free account and quest log already match the definition. Level 20 is recommended. The spoiler calls for either knife and a hammer. Kitchen Knife 3469 is retained near the NPC at (32654, 32200, z7). The static placement gives no evidence of the wiki’s daily replenishment. |
| Entry placement | The wiki sends the player to Rottin Wood’s house in Outlaw Camp. The NPC definition and existing promotion candidate were present, but the scope omitted Rottin Wood. The placement generator admits the matching donor cell (32652, 32201, z7), which already has walkable ground in the retained map. [Crystal NPC spawn, lines 1912–1914](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/world/world-npc.xml#L1912-L1914) |
| Lucky Charms | The global transcript opens with `quest`. The native callback accepted only `mission`. Added the opening alias before Mission 1 begins. Accepting and replacing seven ribbons, delayed corpse skinning, the seven-success cap, crafting seven charms, and the hand-in for 100 XP and 500 gp are absent from the playable route. Crystal records one foot at 25%, two at 10%, and a failed attempt at 65%, transforming corpse 4301 to 4302. Its NPC promises more ribbons but has no refill branch. The imported ribbon, foot and charm definitions are 12171/12172/12173, respectively, all with `materializable: false` in the canonical item shard. |
| In a State of Disrepair | Global uses two confirmations. Base combined both replies and committed after one. The route now splits the existing source reply and commits after the second confirmation. Six hammer repairs, 120-second resets including reuse of the same wall, and 500 XP with 500 gp at hand-in lack active bindings. Crystal records six action IDs 42501–42506. Decoding the six donor cells in the B3 map finds all six corresponding broken-wall appearances and a repaired appearance at the adjacent cell (32659, 32213, z7). The six donor cells carry no corresponding action or unique metadata. The only overlay counter write on base is the initial reset to zero. Main has a generic timed transform. The original action selectors are absent and Hammer 3460 is marked non-materializable in the canonical Item definition. |
| Bushwhacking | Global marks the trap spot on the automap, gives five nets, kicks the player out on the fifth placement, blocks re-entry with a tree for five minutes and asks for five Captured Merchants. The donor spawns seven Travelling Merchants and converts remaining traps during cleanup. The base has no active automap mark, net, trap, scene, loot or replenishment binding for this quest. Crystal grants five 12186 nets and 100 gp per merchant loot, with an 1800-second initial refill deadline. The imported net is non-materializable. The donor's completion gives 1000 XP, 500 gp and Merry Adventures, matching the global completion reward. No NPC completion binding pays it on base. The achievement catalogue identifies `oteryn:achievement/merry_adventures`. |
| Lucky Loop | Global awards Bunny Slipped for seven fake feet, crafted with ribbons, wooden spoons and wool, and pays 300 gp in that case. Crystal awards Bunny Slipped on ordinary charm turn-in and lacks the fake-foot route. Global lists 100 XP and genuine rewards of 500 gp (90%), or 300 gp plus a Time Ring (9%), or a Hunting Horn (listed as 1%?). Donor uses ring 3053 and horn 12260 and a 20-hour cooldown. The canonical horn and fake feet are non-materializable. The catalogue identifies Fake Rabbit’s Foot 12174, Spoon with a Ribbon 12175, Wooden Spoon 3470 and Wool 10319. The achievement catalogue identifies `oteryn:achievement/bunny_slipped`. No active turn-in or craft bindings for the task were found. |
| Handyman | Global requires six repairs and pays 500 XP and 500 gp. The native `task` menu exists but the repair offer, repairs and hand-in are deferred. Crystal uses a 20-hour task cooldown. |
| Reward claims and placement | The definition has empty claim/gate lists. Rewards are described as NPC hand-ins, with five merchant loots in Mission 3. No dedicated quest records were found in reward claims, active or held world callback directories, or NPC offer/callback directories. The B3 map audit used the existing region codec for the six wall cells. The source captures direct use of wall action IDs and item types. The facts list no outfit or permanent quest-granted access. The temporary Bushwhacking tree is the documented access change. |

## BLOCKER

- BLOCKER Lucky Charms acceptance and ribbon refill non-materializable ribbon 12171 and absent global refill source branch data-global/npc/rottin_wood.lua:151–161
- BLOCKER Lucky Charms skinning corpse age and capped probabilistic quest progress dispatch data-global/scripts/actions/tools/skinning.lua:123–146
- BLOCKER Lucky Charms and Lucky Loop recipes non-materializable quest recipe inputs and outputs data-global/scripts/actions/tools/lucky_charm.lua:3–22
- BLOCKER Disrepair and Handyman repairs missing source action metadata and canonical Hammer admission for the existing timed transform data-global/scripts/actions/tools/hammer.lua:64–101
- BLOCKER Bushwhacking scene non-materializable net and shared merchant spawn/capture/cleanup sequence data-global/scripts/quests/rottin_wood_and_married_men/actions_trap.lua:1–123
- BLOCKER Bushwhacking merchant loot atomic scene-object removal, counter and gold transaction data-global/scripts/quests/rottin_wood_and_married_men/actions_corpse.lua:3–36
- BLOCKER mission and task turn-ins blocked item and objective inputs for the existing inventory/XP/achievement owner data-global/npc/rottin_wood.lua:162–194,117–138,238–291
- BLOCKER Lucky Loop and Handyman task lifecycle Rottin task deadline and shared native random reward integration data-global/npc/rottin_wood.lua:222–291,327–362

## OPEN-FACTS

Global facts omit numerical item/achievement IDs, exact rabbit-drop chances, the exact ribbon refill interval, exact daily cooldowns, the horn's confirmed probability, the daily knife respawn timing, placement action IDs and exact server positions for the NPC, structures and trap scene. The NPC cell uses corroborating donor placement evidence. The headline lists 2500 gp and 1600 XP. The spoiler describes individual mission and task payouts and leaves the inclusion of repeatable rewards and merchant loot in the headline unspecified. It also leaves the total spawn population beyond five captures unspecified.

## PARTIAL

The NPC placement and dialogue corrections cover contact and the existing Disrepair start. Ribbon/net grants and refills, kitchen knife respawn, corpse skinning and crafting, timed repairs, the merchant scene and loot, all reward payouts and both daily task implementations remain open. The report records Crystal-only quantities as donor evidence.

## Verification

The mounted case reuses the classic fixture and drives actual NPC dispatch with PostgreSQL quest state. It asserts the opening reply, the split Disrepair offer, unchanged progress after the first confirmation, the three writes after the second confirmation, the single receipt and the existing incomplete repair rejection. The dialogue changes reuse the table's literal replies in Rust. The placement scope is the only generator input change.

The placement generator preserves all 123 pre-existing rows and adds one. The World compiler regenerated the pin and preproduction manifest, and `pin-check` accepted digest `e4b4a0400ffb0518c1f710134f853e48787aa693b16d3e541c69a1a2539c320c`.
