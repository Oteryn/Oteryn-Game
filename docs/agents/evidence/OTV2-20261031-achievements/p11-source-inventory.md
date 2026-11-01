# P11 Crystal achievement granter source inventory

- Donor: Crystal `201189f871d23597b82caf88fd611454a2ddefb8`.
- Source evidence class: `OTS_HYPOTHESIS_ONLY`. Crystal citations describe observed reference-server code; repository and coordinator evidence establish the selected Oteryn implementation and decisions. They do not establish official Tibia behavior. This task explicitly requests compatibility with the pinned Crystal source.
- Authoritative Game base: `ae4188a9c8d461acb152044814ea23075de02ed4`, branch `chatgpt/achievements`. The earlier scratch checkout `ec760bb…` was not the selected base; its arena owner is not evidence for this PR.
- Sweep: 224 textual `:addAchievement` / `:addAchievementProgress` matches in Lua under `data/` and `data-global/`; 223 actual call statements after stripping Lua comments. `153` Lua files mention achievement. Catalogue declarations, read gates, text, migration and administrative helpers are separated below.
- Classification counts are call sites, not unique achievement names: Added=13, Existing=12, Infrastructure=4, Left out=194, Question=0. The one potion call site covers four named achievements under the coordinator-approved source correction.
- A supplementary whole-tree sweep found 17 further calls in the alternative `data-crystal/` map pack. They are recorded separately below and are not included in the requested `data/` + `data-global/` counts. Crystal's pinned default selects `data-global` (`config.lua.dist:4-6`, `src/config/configmanager.cpp:446`).
- No new achievement store, wire command or capability. Existing character counter and account fact/request owners are reused. Migration 0707 adds only the condition potion application's immutable receipt beside the existing item-use receipt.

## Added production granters

| Achievement | Donor threshold | Durable event |
|---|---:|---|
| Here, Fishy Fishy! |250|Successful ordinary-rod water catch 3578 and either admitted rod's ice-hole common catch.|
| Exquisite Taste |250|Ice-hole rare fish 7158/3580/7159, ordinary or mechanical rod.|
| Skin-Deep |500|Successful admitted simple skin yields; failed corpse attempts excluded.|
| Ashes to Dust |500|Successful demon/vampire dust 5906/5905.|
| Ship's Kobold |1250|Consumed donor-route arrival after actual durable placement; includes carpets and legitimate zero-fare routes.|
| Polisher |1000|Successful rust removal; broken items and lottery outcomes excluded.|
| Berserker |100|Berserk potion 7439 applies its real skill condition through the existing item-use and player-condition owners.|
| Mastermind |100|Mastermind potion 7440 applies its real magic-level condition.|
| Sharpshooter |100|Bullseye potion 7443 applies its real distance/shielding condition.|
| Inner Peace |100|Transcendence potion 49271 applies its real fist condition.|

Counter and first-grant provenance commit with the existing fenced owning transaction. Receipt replay returns before the counter hook. Counter saturation and account uniqueness retain the first event/time. The common Achievement connection observation below now covers these grants and the previously wired granters; see the final PR evidence for executed validation.

### Existing production routes reused

| Input | Existing registration | Owning write and result |
|---|---|---|
| USE command 2, item-use capability 15 | `0030-use_item.mechanic` → item craft dispatcher | `durability/item_craft.rs` commits the yield, receipt, counter and account fact; ordinary USE disposition and refreshed inventory return through the existing route. |
| USE / USE-with command 2 | `0030-use_item.mechanic` → consume-transform dispatcher | `durability/consume_transform.rs` commits successful rust output and Polisher with the original receipt. |
| NPC_TALK command 7, NPC capability 3 | `0180-npc.mechanic` → NPC travel service | Existing donor-route service stores departure; `npc_travel::consume_in_transaction` checks current authority, consumes pending arrival, persists actual position and calls the new counter hook in that transaction. A positive fare is not required. |
| USE command 2, item-use capability 15 | Existing carried-item dispatcher → `gameplay_transport/condition_potions.rs` | The existing player condition store prepares and reserves the actual buff; `durability/item_use/condition_potions.rs` commits burn, immutable application receipt, counter and account fact in one fenced transaction. The spell lane retains uncertain applications for receipt reconciliation. |

## Common achievement notices

The existing account owner remains the durable source. `composed_admission/achievements.rs` reads at most catalogue size plus one fact, derives names and the watermark from that same read, and checks the current character account, exact item/session fence, connection generation, transport and runtime actor control before and after the read. Unknown or duplicate keys fail closed. No timestamp is used as a cursor.

`connection/serve/achievement_notices.rs` keeps only the bounded set of already observed keys for the connection. Initial join and every resync/reconnect seed that set without historical notifications. Committed native action/trade/travel commands refresh it after their normal results/effects; passive kill/event facts are observed on the existing five-second idle liveness cadence. The shared immediate chest path uses the same set so a later observation or command replay does not repeat its notice. Capability 8 receives domain 13; capability 31 receives the existing donor status text. A client selecting neither receives no account observation or unsolicited message.

The immediate chest route also rechecks the current bounded account observation before delivery, so control loss after the source commit cannot leak an earned notice. The compatibility default for injected transport fixture authorities retains watermark-only behavior; the production composed authority always returns the complete bounded fact set or fails closed. No new wire command, domain, capability, timer, durable owner, receipt or migration is introduced by this delivery work.

## Current-owner limits and source differences

The 194 excluded mutating call sites are all listed individually below. This path-based grouping accounts for each exactly once; counts refer to call sites, including repeated thresholds, not unique achievement names.

| Missing producer group | Call sites |
|---|---:|
| Donor NPC quest/dialogue transitions |84|
| Specific item/action/quest outcomes |59|
| Quest event/movement outcomes, including the unavailable arena completion |34|
| Gnome rank helper transitions |10|
| Creature/player event outcomes outside the quest subtree |2|
| Complete-area/world discovery |2|
| Shared digging actions needing forced teleport or item mint |2|
| Snow movement outcome |1|
| **Total** |**194**|

- Arena: `dispatch/.../arena_catalogue.rs:1-69` and mechanic0430 are read-only. No `durability/arena.rs` exists on the selected base. Source `data-global/scripts/quests/svargrond_arena/movements_arena_pit.lua:47` requires completing the 10th pit in the corresponding rank; an arena catalogue read cannot grant it.
- Annihilator: `content/.../reward-claim-variants-00000-00099.json` has one four-choice quest variant, with the fourth reward a bag containing an item. It remains `WAITING_IMPLEMENTATION`, including `CONTAINER_REWARD_NATIVE_LOWERING_NOT_IMPLEMENTED`. `world_reward_claims.rs:322-329` admits key-only variants without achievement grants; `reward_claim_mint.rs` rejects container outputs. `quests/.../world_callbacks.rs:515` records missing lever 30015. This PR does not enable three choices while silently dropping the fourth.
- Ice sculpture completion, marble, and mutated-pumpkin branches are not present in `items/src/fishing.rs` gathering admission. Only 7441→7442 is admitted for ice; it must not grant final 7446 achievements.
- Current mount owner has 47 authored pairs (38 ordinary creature taming and 9 music-box pairs). It already grants the authored achievement and Natural Born Cowboy. Existing horse policy deliberately uses Global chance 15 (`native_gameplay/mount_taming.rs:253`); Crystal special branch uses 3. This P11 change does not rewrite prerequisite taming policy. Donkey lookType 399 fallback, item-target mounts and Glooth Glider construction are not admitted.
- Existing Yasir granter uses successful durable trade; donor grants on opening trade. Existing Hope of the Merudri granter uses final native shrine. Those pre-existing trigger choices are disclosed rather than counted as new parity work.
- Generic NPC services, Bestiary counts, quest track tables and the 18 admitted world callbacks do not authorize unrelated donor NPC, event or quest completions. All excluded sites below identify the missing producer.
- Q4=B and Q6=B ([coordinator answer](https://github.com/Oteryn/Oteryn-Game/pull/2782#issuecomment-6547429543), [scope answer](https://github.com/Oteryn/Oteryn-Game/pull/2782#issuecomment-6547466349)) explicitly correct the donor's unreachable potion achievement branch: declarations carry `condition` at `potions.lua:52-54,63`, while the achievement hook is incorrectly inside `potion.func` at `:115-122`. Exactly 100 actually applied uses grant each corresponding achievement; replay, rejected use and KeptCurrent do not advance. The existing HP/MP Potion Addict branch remains separate.
- All four condition potions retain the donor's executable sub-ID zero. `potions.lua:3,10,16,29` supplies undefined bare JeanPierre globals; `lua_enums.cpp:479-483` exports only prefixed names, `condition_functions.cpp:188-196` calls the unchecked numeric loader, and `lua_functions_loader.hpp:74-88` converts Lua nil to zero. The existing `SpellSkills(0)` slot therefore replaces the previous potion, matching the donor rather than inventing stacking. Duration is 600000 ms (`potions.lua:2,9,15,28`); Transcendence's fist +4 and buff flag are at `:30-31`.
- Q8=A ([coordinator answer](https://github.com/Oteryn/Oteryn-Game/pull/2782#issuecomment-6547749872)): fresh sessions keep the current native condition policy. Cross-session temporary-condition persistence is explicitly **Left out**, owned by the parallel **CLOSE-2** task with migrations **0690-0699**. Crystal serializes/restores those conditions (`iologindata_save_player.cpp:264-275`, `iologindata_load_player.cpp:287-304`); this PR preserves the original expiry during uncertain-commit reconciliation within the same session and adds no parallel condition persistence.
- Q9=A ([coordinator answer](https://github.com/Oteryn/Oteryn-Game/pull/2782#issuecomment-6547886810)) authorizes the minimal shared slot in the existing condition owner: `Attributes` and `SpellSkills(0)` conflict, including dispel/immunity identity, while typed reads and nonzero sub-IDs remain unchanged. This follows Swift Foot's default sub-ID zero (`data/scripts/spells/support/swift_foot.lua:1,35-43`) and Crystal's identity/duration/replacement rules (`src/creatures/creature.cpp:1409-1416`, `src/creatures/combat/condition.cpp:436-447,552-592`). The three regression cases are defined in `foundation/condition_attribute_slot_tests.rs`; the same three tests were observed RED (0 passed, 3 failed) and GREEN (3 passed, 0 failed), as recorded in the adjacent `validation.md`. CLOSE-2 still owns serialization.
- Mechanical rod 9306 already has the ice-hole rule in `items/src/fishing.rs:98,185-190`, but its materializable profile was missing. The existing gathering importer now admits that exact existing identity. It preserves the repository-qualified weight 920: `imports/tibiawiki/facts/items-stats.json:272519-272536`, page 36707/revision 1114794, and canonical `content/items/definitions/items-33000-33499.json:1`; `lower_wiki_stats_packet.py:4-8,22-24,584` defines the existing precedence and hundredths-of-ounce unit. Crystal `items.xml:25156-25159` and Canary `items.xml:25017-25020` instead say 919. Capture and admission guard this explicit source difference; the other 28 gathering profiles retain their original semantics. Ordinary-water mechanical fishing remains outside the admitted native rule.

## Every actual mutating call site

| Donor file:line | Grant argument / count | State | Owner or missing producer |
|---|---|---|---|
| `data/libs/functions/player.lua:130` | `"Gnome Little Helper"` | Left out | Gnome rank mutation and quest-point thresholds have no admitted native rank producer. |
| `data/libs/functions/player.lua:136` | `"Gnome Little Helper"` | Left out | Gnome rank mutation and quest-point thresholds have no admitted native rank producer. |
| `data/libs/functions/player.lua:137` | `"Gnome Friend"` | Left out | Gnome rank mutation and quest-point thresholds have no admitted native rank producer. |
| `data/libs/functions/player.lua:143` | `"Gnome Little Helper"` | Left out | Gnome rank mutation and quest-point thresholds have no admitted native rank producer. |
| `data/libs/functions/player.lua:144` | `"Gnome Friend"` | Left out | Gnome rank mutation and quest-point thresholds have no admitted native rank producer. |
| `data/libs/functions/player.lua:145` | `"Gnomelike"` | Left out | Gnome rank mutation and quest-point thresholds have no admitted native rank producer. |
| `data/libs/functions/player.lua:151` | `"Gnome Little Helper"` | Left out | Gnome rank mutation and quest-point thresholds have no admitted native rank producer. |
| `data/libs/functions/player.lua:152` | `"Gnome Friend"` | Left out | Gnome rank mutation and quest-point thresholds have no admitted native rank producer. |
| `data/libs/functions/player.lua:153` | `"Gnomelike"` | Left out | Gnome rank mutation and quest-point thresholds have no admitted native rank producer. |
| `data/libs/functions/player.lua:154` | `"Honorary Gnome"` | Left out | Gnome rank mutation and quest-point thresholds have no admitted native rank producer. |
| `data/npclib/npc_system/modules.lua:245` | `"Ship's Kobold", 1250` | Added | Donor-route arrival receipt → existing achievement counter; only after actual placement, including carpets and legitimate zero-fare routes; tutorial departures excluded. |
| `data/scripts/actions/items/blueberry_bush.lua:7` | `"Bluebarian", 500` | Left out | Specific donor item/action outcome has no admitted durable use producer; existing general item-use owner supports food/potions/taming/catalysts/concoctions/refining, not this action. |
| `data/scripts/actions/items/clay_lump.lua:31` | `"Clay Fighter"` | Left out | Specific donor item/action outcome has no admitted durable use producer; existing general item-use owner supports food/potions/taming/catalysts/concoctions/refining, not this action. |
| `data/scripts/actions/items/clay_lump.lua:32` | `"Clay to Fame", 5` | Left out | Specific donor item/action outcome has no admitted durable use producer; existing general item-use owner supports food/potions/taming/catalysts/concoctions/refining, not this action. |
| `data/scripts/actions/items/costume_bags.lua:17` | `"Masquerader", 100` | Left out | Specific donor item/action outcome has no admitted durable use producer; existing general item-use owner supports food/potions/taming/catalysts/concoctions/refining, not this action. |
| `data/scripts/actions/items/die.lua:22` | `"Number of the Beast"` | Left out | Specific donor item/action outcome has no admitted durable use producer; existing general item-use owner supports food/potions/taming/catalysts/concoctions/refining, not this action. |
| `data/scripts/actions/items/explosive_present.lua:6` | `"Joke's on You"` | Left out | Specific donor item/action outcome has no admitted durable use producer; existing general item-use owner supports food/potions/taming/catalysts/concoctions/refining, not this action. |
| `data/scripts/actions/items/fireworks_rocket.lua:10` | `"Rocket in Pocket", 3` | Left out | Specific donor item/action outcome has no admitted durable use producer; existing general item-use owner supports food/potions/taming/catalysts/concoctions/refining, not this action. |
| `data/scripts/actions/items/fireworks_rocket.lua:14` | `"Fireworks in the Sky", 250` | Left out | Specific donor item/action outcome has no admitted durable use producer; existing general item-use owner supports food/potions/taming/catalysts/concoctions/refining, not this action. |
| `data/scripts/actions/items/ice_flower.lua:10` | `"Ice Harvester", 10` | Left out | Specific donor item/action outcome has no admitted durable use producer; existing general item-use owner supports food/potions/taming/catalysts/concoctions/refining, not this action. |
| `data/scripts/actions/items/ice_flower_seeds.lua:8` | `"Preservationist"` | Left out | Specific donor item/action outcome has no admitted durable use producer; existing general item-use owner supports food/potions/taming/catalysts/concoctions/refining, not this action. |
| `data/scripts/actions/items/muck_remover.lua:30` | `"Goo Goo Dancer", 100` | Left out | Specific donor item/action outcome has no admitted durable use producer; existing general item-use owner supports food/potions/taming/catalysts/concoctions/refining, not this action. |
| `data/scripts/actions/items/music_box.lua:30` | `"Natural Born Cowboy"` | Existing | All 9 music-box creature pairs exist in cosmetics/taming.json; existing mount_taming grants authored achievement and Natural Born Cowboy atomically. |
| `data/scripts/actions/items/music_box.lua:31` | `monsterConfig.achievement` | Existing | All 9 music-box creature pairs exist in cosmetics/taming.json; existing mount_taming grants authored achievement and Natural Born Cowboy atomically. |
| `data/scripts/actions/items/party_hat.lua:10` | `"Party Animal", 200` | Left out | Specific donor item/action outcome has no admitted durable use producer; existing general item-use owner supports food/potions/taming/catalysts/concoctions/refining, not this action. |
| `data/scripts/actions/items/piggy_bank.lua:9` | `"Allowance Collector", 50` | Left out | Specific donor item/action outcome has no admitted durable use producer; existing general item-use owner supports food/potions/taming/catalysts/concoctions/refining, not this action. |
| `data/scripts/actions/items/potions.lua:100` | `"Potion Addict", 100000` | Existing | item_use Potion owner already advances Potion Addict. |
| `data/scripts/actions/items/potions.lua:121` | `potion.achievement, 100` | Added | Coordinator-approved Q4/Q6 correction: four condition potions apply real buffs, then count through the existing fenced item-use receipt/application path; first grant is atomic, and replay/refusal does not advance. |
| `data/scripts/actions/items/potions.lua:136` | `"Demonic Barkeeper", 250` | Left out | Demonic essence 6558 random potion transform is not admitted by the current potion/consume-transform owner. |
| `data/scripts/actions/items/rust_remover.lua:107` | `"Polisher", 1000` | Added | consume_transform successful rust output; failed cleanup and both lottery outcomes excluded. |
| `data/scripts/actions/items/usable_afflicted_outfit_items.lua:19` | `"Beak Doctor", 2` | Left out | Specific donor item/action outcome has no admitted durable use producer; existing general item-use owner supports food/potions/taming/catalysts/concoctions/refining, not this action. |
| `data/scripts/actions/items/usable_afflicted_outfit_items.lua:30` | `"Beak Doctor", 2` | Left out | Specific donor item/action outcome has no admitted durable use producer; existing general item-use owner supports food/potions/taming/catalysts/concoctions/refining, not this action. |
| `data/scripts/actions/items/usable_mount_items.lua:23` | `useItem.achievement` | Left out | Specific donor item/action outcome has no admitted durable use producer; existing general item-use owner supports food/potions/taming/catalysts/concoctions/refining, not this action. |
| `data/scripts/actions/items/usable_mount_items.lua:27` | `"Vortex Tamer", 3` | Left out | Specific donor item/action outcome has no admitted durable use producer; existing general item-use owner supports food/potions/taming/catalysts/concoctions/refining, not this action. |
| `data/scripts/actions/items/usable_mount_items.lua:31` | `"Natural Born Cowboy"` | Left out | Specific donor item/action outcome has no admitted durable use producer; existing general item-use owner supports food/potions/taming/catalysts/concoctions/refining, not this action. |
| `data/scripts/actions/items/usable_outfit_items.lua:98` | `outfitInfo.achievement` | Left out | Specific donor item/action outcome has no admitted durable use producer; existing general item-use owner supports food/potions/taming/catalysts/concoctions/refining, not this action. |
| `data/scripts/actions/items/usable_phantasmal_jade_items.lua:35` | `"Natural Born Cowboy"` | Left out | Specific donor item/action outcome has no admitted durable use producer; existing general item-use owner supports food/potions/taming/catalysts/concoctions/refining, not this action. |
| `data/scripts/actions/items/usable_phantasmal_jade_items.lua:36` | `"You got Horse Power"` | Left out | Specific donor item/action outcome has no admitted durable use producer; existing general item-use owner supports food/potions/taming/catalysts/concoctions/refining, not this action. |
| `data/scripts/actions/items/usable_singeing_steed_items.lua:33` | `"Hot on the Trail"` | Left out | Specific donor item/action outcome has no admitted durable use producer; existing general item-use owner supports food/potions/taming/catalysts/concoctions/refining, not this action. |
| `data/scripts/actions/items/usable_spirit_of_purity_items.lua:33` | `"The Spirit of Purity"` | Left out | Specific donor item/action outcome has no admitted durable use producer; existing general item-use owner supports food/potions/taming/catalysts/concoctions/refining, not this action. |
| `data/scripts/actions/items/voodoo_doll.lua:11` | `"Dark Voodoo Priest"` | Left out | Specific donor item/action outcome has no admitted durable use producer; existing general item-use owner supports food/potions/taming/catalysts/concoctions/refining, not this action. |
| `data/scripts/actions/objects/large_seashell.lua:18` | `"Shell Seeker", 100` | Left out | Specific donor item/action outcome has no admitted durable use producer; existing general item-use owner supports food/potions/taming/catalysts/concoctions/refining, not this action. |
| `data/scripts/actions/tools/claw_of_the_noxious_spawn.lua:19` | `"Cursed!"` | Left out | Specific donor item/action outcome has no admitted durable use producer; existing general item-use owner supports food/potions/taming/catalysts/concoctions/refining, not this action. |
| `data/scripts/actions/tools/prepared_bucket.lua:59` | `"Firefighter"` | Left out | Specific donor item/action outcome has no admitted durable use producer; existing general item-use owner supports food/potions/taming/catalysts/concoctions/refining, not this action. |
| `data/scripts/actions/tools/sneaky_stabber_of_eliteness.lua:25` | `"Bad Timing", 10` | Left out | Specific donor item/action outcome has no admitted durable use producer; existing general item-use owner supports food/potions/taming/catalysts/concoctions/refining, not this action. |
| `data/scripts/actions/tools/squeezing_gear_of_girlpower.lua:35` | `"Bad Timing", 10` | Left out | Specific donor item/action outcome has no admitted durable use producer; existing general item-use owner supports food/potions/taming/catalysts/concoctions/refining, not this action. |
| `data/scripts/actions/tools/watering_can.lua:107` | `"Green Thumb", 100` | Left out | Specific donor item/action outcome has no admitted durable use producer; existing general item-use owner supports food/potions/taming/catalysts/concoctions/refining, not this action. |
| `data/scripts/actions/tools/watering_can.lua:120` | `"Green Thumb", 100` | Left out | Specific donor item/action outcome has no admitted durable use producer; existing general item-use owner supports food/potions/taming/catalysts/concoctions/refining, not this action. |
| `data/scripts/actions/tools/whacking_driller_of_fate.lua:25` | `"Bad Timing", 10` | Left out | Specific donor item/action outcome has no admitted durable use producer; existing general item-use owner supports food/potions/taming/catalysts/concoctions/refining, not this action. |
| `data/scripts/creaturescripts/monster/giant_spider_wyda_death.lua:7` | `"Someone's Bored"` | Left out | Named creature-event/quest storage, credited participant or encounter-completion owner is missing; generic kill credit does not prove donor event eligibility. |
| `data/scripts/creaturescripts/monster/terrified_elephant_death.lua:24` | `"Trail of the Ape God"` | Existing | bestiary_progress credited kill counter and bestiary_achievement rule already grant Trail of the Ape God. |
| `data/scripts/creaturescripts/player/castlemania.lua:48` | `"Castlemania"` | Left out | Named creature-event/quest storage, credited participant or encounter-completion owner is missing; generic kill credit does not prove donor event eligibility. |
| `data/scripts/discovery_system.lua:543` | `ACHIEVEMENT_WIDELY_TRAVELLED` | Left out | All-subareas-complete main-area discovery and complete-world denominator are not owned; current BasicArea fixture is not donor discovery state. |
| `data/scripts/discovery_system.lua:555` | `ACHIEVEMENT_MEASURING_THE_WORLD` | Left out | All-subareas-complete main-area discovery and complete-world denominator are not owned; current BasicArea fixture is not donor discovery state. |
| `data/scripts/lib/register_achievements.lua:662` | `achievIdentifier, denyMsg` | Infrastructure | Existing account owner; helper expands/dispatches other granters, not an independent game event. |
| `data/scripts/lib/register_achievements.lua:712` | `foundAchievement.id` | Infrastructure | Existing account owner; helper expands/dispatches other granters, not an independent game event. |
| `data/scripts/movements/snow.lua:19` | `"Snowbunny", 10000` | Left out | Snow step-out transform/decay event is not durably owned; generic movement cannot count donor snow events. |
| `data/scripts/talkactions/god/achievement_functions.lua:26` | `achievementInfo.id, true` | Infrastructure | God-only administrative mutation; not an ordinary player event or permission to add an admin route. |
| `data-global/npc/a_prisoner.lua:128` | `"Mathemagician"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/a_sleeping_dragon.lua:195` | `"Wayfarer"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/a_wandering_soul.lua:92` | `"Ghostwhisperer"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/ajax.lua:141` | `"Brutal Politeness"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/alyxo.lua:197` | `"Sculptor Apprentice", notice` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/appaloosa.lua:219` | `"Natural Born Cowboy"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/ariella.lua:85` | `"Allow Cookies?"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/armenius.lua:99` | `"His True Face"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/avar_tar.lua:85` | `"Allow Cookies?"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/bozo.lua:752` | `"Perfect Fool"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/bozo.lua:753` | `"Fool at Heart"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/chondur.lua:99` | `"Way of the Shaman"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/chondur.lua:115` | `"Way of the Shaman"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/chondur.lua:242` | `"Blessed!"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/cledwyn.lua:183` | `"Rift Warrior"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/cledwyn.lua:198` | `"Rift Warrior"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/commander_stone.lua:89` | `"Crystal Keeper"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/commander_stone.lua:126` | `"Call Me Sparky"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/commander_stone.lua:161` | `"One Foot Vs. Many"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/commander_stone.lua:203` | `"The Picky Pig"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/cornelia.lua:82` | `"Wild Warrior", 2` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/dedoras.lua:190` | `"Battle Mage"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/duncan.lua:166` | `"Animal Activist"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/eldoran_the_ambitious.lua:134` | `"Bastard"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/emma.lua:63` | `"Secret Agent"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/emma.lua:120` | `"Top CGB Agent"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/enpa_deia_pema.lua:90` | `"Hope of the Merudri"` | Existing | quest_monk_rewards grants Hope of the Merudri at final Eternity shrine; current native quest trigger retained. |
| `data-global/npc/gabel.lua:136` | `"Marid Ally"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/gerimor.lua:441` | `"Corruption Contained"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/gnomadness.lua:89` | `"Ripp-Ripp Hooray!"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/gnomelvis.lua:90` | `"Becoming a Bigfoot"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/gnomeral.lua:98` | `"Crystals in Love"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/gnomeral.lua:140` | `"Substitute Tinker"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/gnomeral.lua:182` | `"Spore Hunter"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/gnomeral.lua:222` | `"Grinding Again"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/grizzly_adams.lua:392` | `tasks.GrizzlyAdams[k].rewards[o].value[1]` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/hairycles.lua:190` | `"Friend of the Apes"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/hairycles.lua:471` | `"Allow Cookies?"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/henricus.lua:252` | `"High Inquisitor"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/henricus.lua:314` | `"Demonbane"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/hjaern.lua:217` | `"Allow Cookies?"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/irmana.lua:118` | `226` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/irmana.lua:192` | `226` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/jack.lua:119` | `"Truth Be Told"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/jack.lua:147` | `"You Don't Know Jack"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/jack_springer.lua:91` | `"Inquisition's Hand"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/jean_pierre.lua:135` | `"Culinary Master"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/kesar_the_younger_day.lua:123` | `"King's Council"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/kesar_the_younger_day.lua:253` | `"Lionheart"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/kevin.lua:295` | `"Archpostman"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/king_tibianus.lua:229` | `"King Tibianus Fan"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/lorbas.lua:76` | `"Allow Cookies?"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/malor.lua:147` | `"Efreet Ally"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/marina.lua:85` | `"Matchmaker"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/markwin.lua:115` | `"Allow Cookies?"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/morgan.lua:106` | `"Wild Warrior", 2` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/nah_bob.lua:105` | `"Allow Cookies?"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/one_eyed_joe.lua:69` | `"Wail of the Banshee"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/ottokar.lua:72` | `"Doctor! Doctor!", 100` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/palimuth.lua:315` | `"Follower of Palimuth"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/palimuth.lua:317` | `"Turncoat"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/palomino.lua:219` | `"Natural Born Cowboy"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/rashid.lua:114` | `"Recognised Trader"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/rashid_custom.lua:116` | `"Recognised Trader"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/rottin_wood.lua:123` | `"Merry Adventures"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/rottin_wood.lua:252` | `"Bunny Slipped"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/sam.lua:117` | `"Backpack Tourist"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/simon_the_beggar.lua:250` | `"Allow Cookies?"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/sven.lua:114` | `"Honorary Barbarian"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/telas.lua:520` | `"Golem in the Gears"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/the_bone_master.lua:96` | `"Bone Brother"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/the_bone_master.lua:116` | `"Skull and Bones"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/the_bone_master.lua:127` | `"Dread Lord"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/the_dream_master.lua:93` | `"Nightmare Knight"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/the_dream_master.lua:112` | `"Nightmare Walker"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/the_dream_master.lua:123` | `"Lord Protector"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/the_orc_king.lua:120` | `"Allow Cookies?"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/the_queen_of_the_banshees.lua:163` | `"Warlock"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/tooth_fairy.lua:125` | `"Toothfairy Assistant"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/uncle.lua:63` | `"Secret Agent"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/uncle.lua:108` | `"Top AVIN Agent"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/wyda.lua:97` | `"Allow Cookies?"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/yalahari.lua:283` | `"Follower of Azerus"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/yalahari.lua:285` | `"Turncoat"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/yaman.lua:134` | `"Allow Cookies?"` | Left out | Specific donor NPC quest dialogue/track transition has no admitted native producer; generic NPC trade/bank/travel or authored quest text is not its completion event. |
| `data-global/npc/yasir.lua:820` | `"Si, Ariki!"` | Existing | npc_trade already grants Si, Ariki! on committed trade; donor opens trade here. Existing trigger difference retained and disclosed. |
| `data-global/npc/yasir.lua:826` | `"Si, Ariki!"` | Existing | npc_trade already grants Si, Ariki! on committed trade; donor opens trade here. Existing trigger difference retained and disclosed. |
| `data-global/scripts/actions/kazordoon/ore_wagons.lua:63` | `"Rollercoaster", 100` | Left out | Specific donor item/action outcome has no admitted durable use producer; existing general item-use owner supports food/potions/taming/catalysts/concoctions/refining, not this action. |
| `data-global/scripts/actions/mounts/mounts.lua:131` | `mount.ACHIEV` | Left out | Glooth Glider multi-item construction has no admitted consuming cosmetic-grant owner. |
| `data-global/scripts/actions/mounts/mounts.lua:133` | `"Natural Born Cowboy"` | Left out | Glooth Glider multi-item construction has no admitted consuming cosmetic-grant owner. |
| `data-global/scripts/actions/mounts/mounts.lua:151` | `mount.ACHIEV` | Existing | Wild-horse successful taming already grants Lucky Horseshoe and Natural Born Cowboy; existing Global chance 15 differs from Crystal special chance 3. |
| `data-global/scripts/actions/mounts/mounts.lua:153` | `"Natural Born Cowboy"` | Existing | Wild-horse successful taming already grants Lucky Horseshoe and Natural Born Cowboy; existing Global chance 15 differs from Crystal special chance 3. |
| `data-global/scripts/actions/mounts/mounts.lua:191` | `mount.ACHIEV` | Existing | All 38 current creature rows already use fenced mount_taming grant owner; see authored pair inventory below. |
| `data-global/scripts/actions/mounts/mounts.lua:193` | `"Natural Born Cowboy"` | Existing | All 38 current creature rows already use fenced mount_taming grant owner; see authored pair inventory below. |
| `data-global/scripts/actions/mounts/mounts.lua:206` | `mount.ACHIEV` | Left out | Donkey fallback based on target lookType 399 is not an admitted taming identity; exact creature-key owner does not expose this event. |
| `data-global/scripts/actions/mounts/mounts.lua:208` | `"Natural Born Cowboy"` | Left out | Donkey fallback based on target lookType 399 is not an admitted taming identity; exact creature-key owner does not expose this event. |
| `data-global/scripts/actions/mounts/mounts.lua:224` | `mount.ACHIEV` | Left out | Generic NPC/action/unique taming branch has no matching configured donor TYPE row or native target owner. |
| `data-global/scripts/actions/mounts/mounts.lua:226` | `"Natural Born Cowboy"` | Left out | Generic NPC/action/unique taming branch has no matching configured donor TYPE row or native target owner. |
| `data-global/scripts/actions/mounts/mounts.lua:244` | `mount.ACHIEV` | Left out | Item-target mount activation (tin lizard/uniwheel/loco) has no matching owner; creature taming cannot stand in for it. |
| `data-global/scripts/actions/mounts/mounts.lua:246` | `"Natural Born Cowboy"` | Left out | Item-target mount activation (tin lizard/uniwheel/loco) has no matching owner; creature taming cannot stand in for it. |
| `data-global/scripts/actions/mounts/mounts.lua:261` | `mount.ACHIEV` | Left out | Generic NPC/action/unique taming branch has no matching configured donor TYPE row or native target owner. |
| `data-global/scripts/actions/mounts/mounts.lua:263` | `"Natural Born Cowboy"` | Left out | Generic NPC/action/unique taming branch has no matching configured donor TYPE row or native target owner. |
| `data-global/scripts/actions/mounts/mounts.lua:278` | `mount.ACHIEV` | Left out | Generic NPC/action/unique taming branch has no matching configured donor TYPE row or native target owner. |
| `data-global/scripts/actions/mounts/mounts.lua:280` | `"Natural Born Cowboy"` | Left out | Generic NPC/action/unique taming branch has no matching configured donor TYPE row or native target owner. |
| `data-global/scripts/actions/other/construction_kits.lua:131` | `"Interior Decorator", 1000` | Left out | Specific donor item/action outcome has no admitted durable use producer; existing general item-use owner supports food/potions/taming/catalysts/concoctions/refining, not this action. |
| `data-global/scripts/actions/other/fishing.lua:136` | `"Biodegradable"` | Left out | Special shimmer/desert fishing branch requires donor catch storage/cooldown or special target owner; ordinary fishing does not produce that event. |
| `data-global/scripts/actions/other/fishing.lua:170` | `"Desert Fisher"` | Left out | Special shimmer/desert fishing branch requires donor catch storage/cooldown or special target owner; ordinary fishing does not produce that event. |
| `data-global/scripts/actions/other/fishing.lua:292` | `"Exquisite Taste", 250` | Added | Committed ordinary/ice fish yield; current admitted rod pairs; item_craft receipt gates replay. |
| `data-global/scripts/actions/other/fishing.lua:296` | `"Exquisite Taste", 250` | Added | Committed ordinary/ice fish yield; current admitted rod pairs; item_craft receipt gates replay. |
| `data-global/scripts/actions/other/fishing.lua:300` | `"Exquisite Taste", 250` | Added | Committed ordinary/ice fish yield; current admitted rod pairs; item_craft receipt gates replay. |
| `data-global/scripts/actions/other/fishing.lua:314` | `"Here, Fishy Fishy!", 250` | Added | Committed ordinary/ice fish yield; current admitted rod pairs; item_craft receipt gates replay. |
| `data-global/scripts/actions/other/mechanical_fishing.lua:162` | `"Biodegradable"` | Left out | Special shimmer/desert fishing branch requires donor catch storage/cooldown or special target owner; ordinary fishing does not produce that event. |
| `data-global/scripts/actions/other/mechanical_fishing.lua:196` | `"Desert Fisher"` | Left out | Special shimmer/desert fishing branch requires donor catch storage/cooldown or special target owner; ordinary fishing does not produce that event. |
| `data-global/scripts/actions/other/mechanical_fishing.lua:350` | `"Exquisite Taste", 250` | Added | Committed ordinary/ice fish yield; current admitted rod pairs; item_craft receipt gates replay. |
| `data-global/scripts/actions/other/mechanical_fishing.lua:354` | `"Exquisite Taste", 250` | Added | Committed ordinary/ice fish yield; current admitted rod pairs; item_craft receipt gates replay. |
| `data-global/scripts/actions/other/mechanical_fishing.lua:358` | `"Exquisite Taste", 250` | Added | Committed ordinary/ice fish yield; current admitted rod pairs; item_craft receipt gates replay. |
| `data-global/scripts/actions/other/mechanical_fishing.lua:372` | `"Here, Fishy Fishy!", 250` | Added | Ice-hole common catch (9306→7236) shares the atomic craft hook. Ordinary-water mechanical fishing has no admitted conversion; that producer remains outside this granter wiring. |
| `data-global/scripts/actions/other/music.lua:65` | `"Rockstar", 10000` | Left out | Specific donor item/action outcome has no admitted durable use producer; existing general item-use owner supports food/potions/taming/catalysts/concoctions/refining, not this action. |
| `data-global/scripts/actions/system/quest_reward_common.lua:213` | `achievement` | Left out | Annihilator reward variant remains WAITING_IMPLEMENTATION; complete four-choice container-reward lowering and lever 30015 owner missing. |
| `data-global/scripts/actions/system/quest_reward_common.lua:252` | `achievement` | Left out | Annihilator reward variant remains WAITING_IMPLEMENTATION; complete four-choice container-reward lowering and lever 30015 owner missing. |
| `data-global/scripts/actions/tools/skinning.lua:162` | `"Ice Sculptor"` | Left out | Final sculpture item 7446 required; existing gathering owner admits only first stage 7441→7442. |
| `data-global/scripts/actions/tools/skinning.lua:163` | `"Cold as Ice", 10` | Left out | Final sculpture item 7446 required; existing gathering owner admits only first stage 7441→7442. |
| `data-global/scripts/actions/tools/skinning.lua:187` | `"Marblelous"` | Left out | Marble 10426 final sculpture/random described goblet owner is not admitted. |
| `data-global/scripts/actions/tools/skinning.lua:188` | `"Marble Madness", 5` | Left out | Marble 10426 final sculpture/random described goblet owner is not admitted. |
| `data-global/scripts/actions/tools/skinning.lua:280` | `"Mutated Presents"` | Left out | Mutated pumpkin target and its storage/reward branch are not admitted by current gathering rules. |
| `data-global/scripts/actions/tools/skinning.lua:319` | `"Ashes to Dust", 500` | Added | Successful admitted simple skin/dust inventory yield; every existing simple gathering row shares the atomic craft hook. |
| `data-global/scripts/actions/tools/skinning.lua:321` | `"Skin-Deep", 500` | Added | Successful admitted simple skin/dust inventory yield; every existing simple gathering row shares the atomic craft hook. |
| `data-global/scripts/game_migrations/20241708000535_move_achievement_to_kv.lua:15` | `achievement.name` | Infrastructure | Legacy storage migration; no donor KV migration is run in Oteryn. |
| `data-global/scripts/lib/register_actions.lua:422` | `"The Undertaker", 500` | Left out | Donor grants after hole transform AND player teleport. Current terrain owner transforms/reverts only; no matching forced-movement completion. |
| `data-global/scripts/lib/register_actions.lua:432` | `"Gold Digger", 100` | Left out | Sand digging gold yield needs item mint; current terrain route refuses yield-bearing plans. |
| `data-global/scripts/lib/register_actions.lua:628` | `"Petrologist", 100` | Existing | item_use PickRefine owner already advances Petrologist in the receipt transaction. |
| `data-global/scripts/quests/a_piece_of_cake/movements_cake_base_isle.lua:12` | `"Cake Conqueror"` | Left out | Specific donor quest action/movement, prerequisites and completion/reward transition are absent from current native world callbacks; generic quest storage is insufficient. |
| `data-global/scripts/quests/a_piece_of_cake/movements_cake_base_isle.lua:22` | `"I Did My Part"` | Left out | Specific donor quest action/movement, prerequisites and completion/reward transition are absent from current native world callbacks; generic quest storage is insufficient. |
| `data-global/scripts/quests/a_pirates_tail_quest/creaturescripts_tentugly_death.lua:17` | `"Release the Kraken"` | Left out | Specific donor quest action/movement, prerequisites and completion/reward transition are absent from current native world callbacks; generic quest storage is insufficient. |
| `data-global/scripts/quests/barbarian_test/action_horn.lua:24` | `"Bearhugger"` | Left out | Specific donor quest action/movement, prerequisites and completion/reward transition are absent from current native world callbacks; generic quest storage is insufficient. |
| `data-global/scripts/quests/bigfoot_burden/actions_rewards.lua:89` | `reward.achievement[1]` | Left out | Specific donor quest action/movement, prerequisites and completion/reward transition are absent from current native world callbacks; generic quest storage is insufficient. |
| `data-global/scripts/quests/bigfoot_burden/actions_rewards.lua:90` | `reward.achievement[2], 50` | Left out | Specific donor quest action/movement, prerequisites and completion/reward transition are absent from current native world callbacks; generic quest storage is insufficient. |
| `data-global/scripts/quests/dangerous_depth/actions_crude_lava_pump_achievements.lua:36` | `"Buried the Baron"` | Left out | Specific donor quest action/movement, prerequisites and completion/reward transition are absent from current native world callbacks; generic quest storage is insufficient. |
| `data-global/scripts/quests/dangerous_depth/actions_crude_lava_pump_achievements.lua:43` | `"His Days are Counted"` | Left out | Specific donor quest action/movement, prerequisites and completion/reward transition are absent from current native world callbacks; generic quest storage is insufficient. |
| `data-global/scripts/quests/dangerous_depth/actions_crude_lava_pump_achievements.lua:50` | `"Duked It Out"` | Left out | Specific donor quest action/movement, prerequisites and completion/reward transition are absent from current native world callbacks; generic quest storage is insufficient. |
| `data-global/scripts/quests/dangerous_depth/actions_crude_lava_pump_achievements.lua:57` | `"Death in the Depths"` | Left out | Specific donor quest action/movement, prerequisites and completion/reward transition are absent from current native world callbacks; generic quest storage is insufficient. |
| `data-global/scripts/quests/feaster_of_souls/creaturescripts_pale_worm_death.lua:19` | `"Beyonder"` | Left out | Specific donor quest action/movement, prerequisites and completion/reward transition are absent from current native world callbacks; generic quest storage is insufficient. |
| `data-global/scripts/quests/ferumbras_ascension/actions_reward.lua:21` | `"Hat Hunter"` | Left out | Specific donor quest action/movement, prerequisites and completion/reward transition are absent from current native world callbacks; generic quest storage is insufficient. |
| `data-global/scripts/quests/forgotten_knowledge/actions_forbidden_fruit.lua:13` | `"Forbidden Fruit"` | Left out | Specific donor quest action/movement, prerequisites and completion/reward transition are absent from current native world callbacks; generic quest storage is insufficient. |
| `data-global/scripts/quests/heart_of_destruction/actions_reward.lua:13` | `"Ender of the End"` | Left out | Specific donor quest action/movement, prerequisites and completion/reward transition are absent from current native world callbacks; generic quest storage is insufficient. |
| `data-global/scripts/quests/hero_of_rathleton/actions_reward.lua:9` | `"The Professors Nut"` | Left out | Specific donor quest action/movement, prerequisites and completion/reward transition are absent from current native world callbacks; generic quest storage is insufficient. |
| `data-global/scripts/quests/hero_of_rathleton/movements_lava.lua:20` | `"Go with da Lava Flow"` | Left out | Specific donor quest action/movement, prerequisites and completion/reward transition are absent from current native world callbacks; generic quest storage is insufficient. |
| `data-global/scripts/quests/lions_rock/actions_lions_rock.lua:132` | `"Lion's Den Explorer"` | Left out | Specific donor quest action/movement, prerequisites and completion/reward transition are absent from current native world callbacks; generic quest storage is insufficient. |
| `data-global/scripts/quests/raging_mage_tower/creaturescripts_raging_mage_1.lua:11` | `"Mageslayer"` | Left out | Specific donor quest action/movement, prerequisites and completion/reward transition are absent from current native world callbacks; generic quest storage is insufficient. |
| `data-global/scripts/quests/svargrond_arena/movements_arena_pit.lua:47` | `ARENA[arenaId].achievement` | Left out | Authoritative base ae4188a has only read-only arena_catalogue command240; no durable completed-rank/pit completion owner. |
| `data-global/scripts/quests/targuna/actions_reliable_ram.lua:46` | `ACHIEVEMENT_NAME` | Left out | Specific donor quest action/movement, prerequisites and completion/reward transition are absent from current native world callbacks; generic quest storage is insufficient. |
| `data-global/scripts/quests/targuna/movements_sandcastles.lua:68` | `ACHIEVEMENT_NAME` | Left out | Specific donor quest action/movement, prerequisites and completion/reward transition are absent from current native world callbacks; generic quest storage is insufficient. |
| `data-global/scripts/quests/the_dream_courts/actions_container_rewards.lua:134` | `"Tied the Knot"` | Left out | Specific donor quest action/movement, prerequisites and completion/reward transition are absent from current native world callbacks; generic quest storage is insufficient. |
| `data-global/scripts/quests/the_dream_courts/actions_old_lock.lua:16` | `"Keeper of the 7 Keys"` | Left out | Specific donor quest action/movement, prerequisites and completion/reward transition are absent from current native world callbacks; generic quest storage is insufficient. |
| `data-global/scripts/quests/the_first_dragon/actions_treasure_chest.lua:101` | `"Treasure Hunter"` | Left out | Specific donor quest action/movement, prerequisites and completion/reward transition are absent from current native world callbacks; generic quest storage is insufficient. |
| `data-global/scripts/quests/the_great_dragon_hunt_quest/actions_treasure.lua:37` | `"Hoard of the Dragon"` | Left out | Specific donor quest action/movement, prerequisites and completion/reward transition are absent from current native world callbacks; generic quest storage is insufficient. |
| `data-global/scripts/quests/the_ice_islands_quest/actions_paint.lua:23` | `"Animal Activist"` | Left out | Specific donor quest action/movement, prerequisites and completion/reward transition are absent from current native world callbacks; generic quest storage is insufficient. |
| `data-global/scripts/quests/the_inquisition_quest/actions_rewards.lua:22` | `"Master of the Nexus"` | Left out | Specific donor quest action/movement, prerequisites and completion/reward transition are absent from current native world callbacks; generic quest storage is insufficient. |
| `data-global/scripts/quests/the_isle_of_evil_quest/actions_heated_worm_punisher.lua:42` | `"Worm Whacker"` | Left out | Specific donor quest action/movement, prerequisites and completion/reward transition are absent from current native world callbacks; generic quest storage is insufficient. |
| `data-global/scripts/quests/the_new_frontier/creaturescripts_tirecz_kill.lua:26` | `"Champion of Chazorai"` | Left out | Specific donor quest action/movement, prerequisites and completion/reward transition are absent from current native world callbacks; generic quest storage is insufficient. |
| `data-global/scripts/quests/the_pits_of_inferno_quest/actions_fountain.lua:9` | `"Fountain of Life"` | Left out | Specific donor quest action/movement, prerequisites and completion/reward transition are absent from current native world callbacks; generic quest storage is insufficient. |
| `data-global/scripts/quests/the_secret_library_quest/creaturescripts_kill.lua:33` | `monsterStorage.achievements[i]` | Left out | Specific donor quest action/movement, prerequisites and completion/reward transition are absent from current native world callbacks; generic quest storage is insufficient. |
| `data-global/scripts/quests/the_secret_library_quest/liquid_death/movements_teleportTo.lua:31` | `tab[i].achievementName` | Left out | Specific donor quest action/movement, prerequisites and completion/reward transition are absent from current native world callbacks; generic quest storage is insufficient. |
| `data-global/scripts/quests/unnatural_selection/movements_mission3_dance_dance_evolution.lua:60` | `"Talented Dancer"` | Left out | Specific donor quest action/movement, prerequisites and completion/reward transition are absent from current native world callbacks; generic quest storage is insufficient. |
| `data-global/scripts/quests/wrath_of_the_emperor/actions_mission10_a_message_of_freedom_sceptre.lua:42` | `"Godslayer"` | Left out | Specific donor quest action/movement, prerequisites and completion/reward transition are absent from current native world callbacks; generic quest storage is insufficient. |

## Supplementary alternate-map-pack call sites

The prompt names `data/` and `data-global/` as the granter source scope. The pinned server loads one selected map pack; it does not combine `data-global` and `data-crystal`. This additional sweep accounts for all 17 mutating calls in `data-crystal/` without importing different map rules into the selected pack. In particular, the default machete path at `data-global/scripts/lib/register_actions.lua:864-869`, large-hole transform at `:649-652`, and crop paths at `:1035-1049` do not contain the achievement hooks present in the alternative pack. The existing native terrain owner is not evidence to silently select those other pack rules.

| Alternative donor file:line | Achievement expression | Default-pack mapping or reason left out |
|---|---|---|
| `data-crystal/npc/cledwyn.lua:183` | `"Rift Warrior"` | Same achievement as the default NPC; the specific quest/dialogue completion owner is absent. |
| `data-crystal/npc/cledwyn.lua:198` | `"Rift Warrior"` | Second dialogue branch; same missing quest completion producer. |
| `data-crystal/npc/henricus.lua:98` | `"Demonbane"` | Alternative NPC quest flow; the corresponding default NPC granter is already listed as missing its producer. |
| `data-crystal/npc/henricus.lua:114` | `"High Inquisitor"` | Alternative NPC quest flow; no admitted native completion event. |
| `data-crystal/scripts/actions/other/construction_kits.lua:77` | `"Interior Decorator", 1000` | Construction-kit placement has no admitted native action producer; the default pack site is listed above. |
| `data-crystal/scripts/actions/other/quest_reward_common.lua:40` | `achievement` (`Annihilator`, chest IDs 6085-6088 at `:12-18`) | Ordinary reward branch of the same blocked Annihilator choice set; no partial activation of the set. |
| `data-crystal/scripts/actions/other/quest_reward_common.lua:76` | `achievement` (`Annihilator`) | Container reward branch; native container-reward lowering remains missing. |
| `data-crystal/scripts/lib/register_actions.lua:249` | `"Nothing Can Stop Me", 100` | Unselected map-pack rule. The selected machete transformation has no such hook at `data-global/.../register_actions.lua:864-869`. |
| `data-crystal/scripts/lib/register_actions.lua:283` | `"Petrologist", 100` | The corresponding default shiny-stone refinement is already granted by the existing fenced item-use owner. No second granter. |
| `data-crystal/scripts/lib/register_actions.lua:378` | `"The Undertaker", 500` | Hole transformation plus forced movement; same missing movement-completion producer as the default pack. |
| `data-crystal/scripts/lib/register_actions.lua:382` | `"The Undertaker", 500` | Additional alternative-pack large-hole hook. Selected `data-global/.../register_actions.lua:649-652` transforms 867 without this hook. |
| `data-crystal/scripts/lib/register_actions.lua:407` | `"Gold Digger", 100` | Scarab-coin digging outcome has no admitted atomic mint producer; default site is already listed. |
| `data-crystal/scripts/lib/register_actions.lua:427` | `"Happy Farmer", 200` | Alternative-pack crop hook; selected wheat transformation and yield at `data-global/.../register_actions.lua:1045-1049` do not grant it. |
| `data-crystal/scripts/lib/register_actions.lua:442` | `"Natural Sweetener", 50` | Alternative-pack crop hook; selected sugar-cane transformation and yield at `data-global/.../register_actions.lua:1040-1044` do not grant it. |
| `data-crystal/scripts/quests/arena/movements_arena_pit.lua:47` | `ARENA[arenaId].achievement` | Same rank-completion requirement; no durable arena completion owner on the selected Game base. |
| `data-crystal/scripts/quests/inqusition_quest/reward.lua:21` | `"Master of the Nexus"` | Alternative map quest reward; corresponding default quest completion/reward owner is absent. |
| `data-crystal/scripts/quests/pits_of_inferno/actions/fountain_of_life.lua:9` | `"Fountain of Life"` | Alternative map fountain action; corresponding default quest completion owner is absent. |

## Dynamic achievement declarations used by the call sites

These declarations expand dynamic arguments above; they are not additional grant events. Each row records the exact pinned declaration line.

| Donor file:line | Declared achievement(s) |
|---|---|
| `data-global/lib/quests/killing_in_the_name_of.lua:226` | Blood-Red Snapper |
| `data-global/lib/quests/killing_in_the_name_of.lua:256` | No More Hiding |
| `data-global/lib/quests/killing_in_the_name_of.lua:273` | Rootless Behaviour |
| `data-global/lib/quests/killing_in_the_name_of.lua:303` | Meat Skewer |
| `data-global/lib/quests/killing_in_the_name_of.lua:386` | Breaking The Ice |
| `data-global/lib/quests/killing_in_the_name_of.lua:420` | Twisted Mutation |
| `data-global/lib/quests/killing_in_the_name_of.lua:437` | Crawling Death |
| `data-global/lib/quests/killing_in_the_name_of.lua:506` | Spareribs for Dinner |
| `data-global/lib/quests/killing_in_the_name_of.lua:523` | Arachnoise |
| `data-global/lib/quests/killing_in_the_name_of.lua:557` | Back into the Abyss |
| `data-global/lib/quests/killing_in_the_name_of.lua:574` | Choking on Her Venom |
| `data-global/lib/quests/killing_in_the_name_of.lua:591` | Howly Silence |
| `data-global/lib/quests/killing_in_the_name_of.lua:609` | Dream's Over |
| `data-global/lib/quests/killing_in_the_name_of.lua:626` | Scorched Flames |
| `data-global/lib/quests/killing_in_the_name_of.lua:646` | Zzztill Zzztanding! |
| `data-global/lib/quests/killing_in_the_name_of.lua:663` | Stepped on a Big Toe |
| `data-global/lib/quests/killing_in_the_name_of.lua:680` | Something Smells |
| `data-global/lib/quests/killing_in_the_name_of.lua:697` | Kapow! |
| `data-global/lib/quests/killing_in_the_name_of.lua:715` | One Less |
| `data-global/lib/quests/killing_in_the_name_of.lua:733` | Hissing Downfall |
| `data-global/lib/quests/killing_in_the_name_of.lua:751` | The Serpent's Bride |
| `data-global/lib/quests/killing_in_the_name_of.lua:769` | Just Cracked Me Up! |
| `data-global/lib/quests/killing_in_the_name_of.lua:788` | The Drowned Sea God |
| `data-global/lib/quests/killing_in_the_name_of.lua:806` | The Gates of Hell |
| `data-global/lib/quests/killing_in_the_name_of.lua:824` | Beautiful Agony |
| `data-global/lib/quests/killing_in_the_name_of.lua:845` | Enter zze Draken! |
| `data-global/lib/quests/killing_in_the_name_of.lua:863` | King of the Ring |
| `data-global/lib/quests/killing_in_the_name_of.lua:881` | Back from the Dead |
| `data-global/lib/quests/svargrond_arena.lua:123` | Greenhorn |
| `data-global/lib/quests/svargrond_arena.lua:141` | Scrapper |
| `data-global/lib/quests/svargrond_arena.lua:159` | Warlord of Svargrond |
| `data-global/scripts/actions/mounts/mounts.lua:7` | Bearbaiting |
| `data-global/scripts/actions/mounts/mounts.lua:9` | Little Ball of Wool |
| `data-global/scripts/actions/mounts/mounts.lua:10` | Pig-Headed |
| `data-global/scripts/actions/mounts/mounts.lua:11` | Fried Shrimp |
| `data-global/scripts/actions/mounts/mounts.lua:12` | The Right Tone |
| `data-global/scripts/actions/mounts/mounts.lua:13` | Loyal Lad |
| `data-global/scripts/actions/mounts/mounts.lua:14` | Dragon Mimicry |
| `data-global/scripts/actions/mounts/mounts.lua:15` | Scales and Tail |
| `data-global/scripts/actions/mounts/mounts.lua:16` | Fata Morgana |
| `data-global/scripts/actions/mounts/mounts.lua:17` | Friend of Elves |
| `data-global/scripts/actions/mounts/mounts.lua:18` | Bibliomaniac |
| `data-global/scripts/actions/mounts/mounts.lua:19` | Beaver Away |
| `data-global/scripts/actions/mounts/mounts.lua:35` | Fabled Construction |
| `data-global/scripts/actions/mounts/mounts.lua:37` | Blacknailed |
| `data-global/scripts/actions/mounts/mounts.lua:38` | Gryphon Rider |
| `data-global/scripts/actions/mounts/mounts.lua:39` | Moth Whisperer |
| `data-global/scripts/actions/mounts/mounts.lua:40` | Knock on Wood |
| `data-global/scripts/actions/mounts/mounts.lua:41` | Stuntman |
| `data-global/scripts/actions/mounts/mounts.lua:42` | Magnetised |
| `data-global/scripts/actions/mounts/mounts.lua:43` | Lacewing Catcher |
| `data-global/scripts/actions/mounts/mounts.lua:44` | Lovely Dots |
| `data-global/scripts/actions/mounts/mounts.lua:53` | Way to Hell |
| `data-global/scripts/actions/mounts/mounts.lua:55` | Beneath the Sea |
| `data-global/scripts/actions/mounts/mounts.lua:56` | Starless Night |
| `data-global/scripts/actions/mounts/mounts.lua:57` | Mind the Dog! |
| `data-global/scripts/actions/mounts/mounts.lua:58` | Up the Molehill |
| `data-global/scripts/actions/mounts/mounts.lua:59` | Lion King |
| `data-global/scripts/actions/mounts/mounts.lua:60` | Chequered Teddy |
| `data-global/scripts/actions/mounts/mounts.lua:61` | Golden Sands |
| `data-global/scripts/actions/mounts/mounts.lua:62` | Personal Nightmare |
| `data-global/scripts/actions/mounts/mounts.lua:63` | Rhino Rider |
| `data-global/scripts/actions/mounts/mounts.lua:64` | Slugging Around |
| `data-global/scripts/actions/mounts/mounts.lua:65` | Pecking Order |
| `data-global/scripts/actions/mounts/mounts.lua:66` | Out of the Stone Age |
| `data-global/scripts/actions/mounts/mounts.lua:67` | Spin-Off |
| `data-global/scripts/actions/mounts/mounts.lua:68` | Gear Up |
| `data-global/scripts/actions/mounts/mounts.lua:69` | Swamp Beast |
| `data-global/scripts/actions/mounts/mounts.lua:70` | Engine Driver |
| `data-global/scripts/actions/mounts/mounts.lua:71` | Lucky Horseshoe |
| `data-global/scripts/actions/mounts/mounts.lua:72` | Well Roared, Lion! |
| `data-global/scripts/actions/mounts/mounts.lua:73` | Like Fox and Mouse |
| `data-global/scripts/actions/system/quest_reward_common.lua:181` | Annihilator |
| `data-global/scripts/actions/system/quest_reward_common.lua:182` | Annihilator |
| `data-global/scripts/actions/system/quest_reward_common.lua:183` | Annihilator |
| `data-global/scripts/actions/system/quest_reward_common.lua:184` | Annihilator |
| `data-global/scripts/quests/bigfoot_burden/actions_rewards.lua:13` | Final Strike, Death on Strike |
| `data-global/scripts/quests/bigfoot_burden/actions_rewards.lua:27` | Gnomebane's Bane, Fall of the Fallen |
| `data-global/scripts/quests/bigfoot_burden/actions_rewards.lua:40` | Death from Below, Diplomatic Immunity |
| `data-global/scripts/quests/targuna/actions_reliable_ram.lua:10` | A reliable Friend |
| `data-global/scripts/quests/targuna/movements_sandcastles.lua:5` | Castle Crasher |
| `data-global/scripts/quests/the_secret_library_quest/creaturescripts_kill.lua:16` | Millennial Falcon, Master Debater |
| `data-global/scripts/quests/the_secret_library_quest/liquid_death/movements_teleportTo.lua:3` | Spectulation |
| `data/scripts/actions/items/music_box.lua:2` | Dragon Mimicry |
| `data/scripts/actions/items/music_box.lua:3` | Scales and Tail |
| `data/scripts/actions/items/music_box.lua:4` | Friend of Elves |
| `data/scripts/actions/items/music_box.lua:5` | Magnetised |
| `data/scripts/actions/items/music_box.lua:6` | Way to Hell |
| `data/scripts/actions/items/music_box.lua:7` | Starless Night |
| `data/scripts/actions/items/music_box.lua:8` | Spin-Off |
| `data/scripts/actions/items/music_box.lua:9` | Lucky Horseshoe |
| `data/scripts/actions/items/music_box.lua:10` | Chequered Teddy |
| `data/scripts/actions/items/potions.lua:52` | Berserker |
| `data/scripts/actions/items/potions.lua:53` | Mastermind |
| `data/scripts/actions/items/potions.lua:54` | Sharpshooter |
| `data/scripts/actions/items/potions.lua:63` | Inner Peace |
| `data/scripts/actions/items/usable_mount_items.lua:5` | Nothing but Hot Air |
| `data/scripts/actions/items/usable_outfit_items.lua:4` | Funghitastic |
| `data/scripts/actions/items/usable_outfit_items.lua:5` | Funghitastic |
| `data/scripts/actions/items/usable_outfit_items.lua:9` | Crystal Clear |
| `data/scripts/actions/items/usable_outfit_items.lua:10` | Crystal Clear |
| `data/scripts/actions/items/usable_outfit_items.lua:14` | Cobbled and Patched |
| `data/scripts/actions/items/usable_outfit_items.lua:15` | Cobbled and Patched |
| `data/scripts/actions/items/usable_outfit_items.lua:18` | Inquisition's Arm |
| `data/scripts/actions/items/usable_outfit_items.lua:19` | Inquisition's Arm |
| `data/scripts/actions/items/usable_outfit_items.lua:22` | Mainstreet Nightmare |
| `data/scripts/actions/items/usable_outfit_items.lua:23` | Mainstreet Nightmare |
| `data/scripts/actions/items/usable_outfit_items.lua:26` | Unleash the Beast |
| `data/scripts/actions/items/usable_outfit_items.lua:27` | Unleash the Beast |
| `data/scripts/actions/items/usable_outfit_items.lua:30` | Fully Decayed |
| `data/scripts/actions/items/usable_outfit_items.lua:31` | Fully Decayed |
| `data/scripts/actions/items/usable_outfit_items.lua:34` | Honorary Rascoohan |
| `data/scripts/actions/items/usable_outfit_items.lua:35` | Honorary Rascoohan |
| `data/scripts/actions/items/usable_outfit_items.lua:38` | Friendly Fire |
| `data/scripts/actions/items/usable_outfit_items.lua:39` | Friendly Fire |
| `data/scripts/actions/items/usable_outfit_items.lua:43` | Fiend Slayer |
| `data/scripts/actions/items/usable_outfit_items.lua:44` | Fiend Slayer |
| `data/scripts/actions/items/usable_outfit_items.lua:47` | Illuminator |
| `data/scripts/actions/items/usable_outfit_items.lua:48` | Illuminator |
| `data/scripts/discovery_system.lua:26-27` | Numeric 470=Widely Travelled; 539=Measuring the World (catalogue lookup). |
| `data-global/npc/irmana.lua:118,192` | Numeric 226=Aristocrat (catalogue lookup). |

## Already-authored mount pairs

All listed pairs already pass their success through `durability/item_use/mount_taming.rs:255-274`. The source annotations in existing content are preserved; shared Crystal data is cross-checked by the existing native mount tests.

| Item | Creature | Existing achievement key |
|---|---|---|
| `oteryn:item.tibia.i12260` | `oteryn:creature.boar` | `oteryn:achievement/pig_headed` |
| `oteryn:item.tibia.i12304` | `oteryn:creature.undead_cavebear` | `oteryn:achievement/out_of_the_stone_age` |
| `oteryn:item.tibia.i12306` | `oteryn:creature.midnight_panther` | `oteryn:achievement/starless_night` |
| `oteryn:item.tibia.i12307` | `oteryn:creature.draptor` | `oteryn:achievement/scales_and_tail` |
| `oteryn:item.tibia.i12308` | `oteryn:creature.black_sheep` | `oteryn:achievement/little_ball_of_wool` |
| `oteryn:item.tibia.i12311` | `oteryn:creature.terror_bird` | `oteryn:achievement/pecking_order` |
| `oteryn:item.tibia.i12318` | `oteryn:creature.crustacea_gigantica` | `oteryn:achievement/fried_shrimp` |
| `oteryn:item.tibia.i12320` | `oteryn:creature.wailing_widow` | `oteryn:achievement/spin_off` |
| `oteryn:item.tibia.i12509` | `oteryn:creature.sandstone_scorpion` | `oteryn:achievement/golden_sands` |
| `oteryn:item.tibia.i12519` | `oteryn:creature.slug` | `oteryn:achievement/slugging_around` |
| `oteryn:item.tibia.i12546` | `oteryn:creature.dromedary` | `oteryn:achievement/fata_morgana` |
| `oteryn:item.tibia.i12547` | `oteryn:creature.crystal_wolf` | `oteryn:achievement/the_right_tone` |
| `oteryn:item.tibia.i12548` | `oteryn:creature.donkey` | `oteryn:achievement/loyal_lad` |
| `oteryn:item.tibia.i12549` | `oteryn:creature.panda` | `oteryn:achievement/chequered_teddy` |
| `oteryn:item.tibia.i12550` | `oteryn:creature.enraged_white_deer` | `oteryn:achievement/friend_of_elves` |
| `oteryn:item.tibia.i12802` | `oteryn:creature.wild_horse` | `oteryn:achievement/lucky_horseshoe` |
| `oteryn:item.tibia.i14142` | `oteryn:creature.manta_ray` | `oteryn:achievement/beneath_the_sea` |
| `oteryn:item.tibia.i14143` | `oteryn:creature.ladybug` | `oteryn:achievement/lovely_dots` |
| `oteryn:item.tibia.i16153` | `oteryn:creature.ironblight` | `oteryn:achievement/magnetised` |
| `oteryn:item.tibia.i16154` | `oteryn:creature.magma_crawler` | `oteryn:achievement/way_to_hell` |
| `oteryn:item.tibia.i16155` | `oteryn:creature.dragonling` | `oteryn:achievement/dragon_mimicry` |
| `oteryn:item.tibia.i16244` | `oteryn:creature.dragonling` | `oteryn:achievement/dragon_mimicry` |
| `oteryn:item.tibia.i16244` | `oteryn:creature.draptor` | `oteryn:achievement/scales_and_tail` |
| `oteryn:item.tibia.i16244` | `oteryn:creature.enraged_white_deer` | `oteryn:achievement/friend_of_elves` |
| `oteryn:item.tibia.i16244` | `oteryn:creature.ironblight` | `oteryn:achievement/magnetised` |
| `oteryn:item.tibia.i16244` | `oteryn:creature.magma_crawler` | `oteryn:achievement/way_to_hell` |
| `oteryn:item.tibia.i16244` | `oteryn:creature.midnight_panther` | `oteryn:achievement/starless_night` |
| `oteryn:item.tibia.i16244` | `oteryn:creature.panda` | `oteryn:achievement/chequered_teddy` |
| `oteryn:item.tibia.i16244` | `oteryn:creature.wailing_widow` | `oteryn:achievement/spin_off` |
| `oteryn:item.tibia.i16244` | `oteryn:creature.wild_horse` | `oteryn:achievement/lucky_horseshoe` |
| `oteryn:item.tibia.i16251` | `oteryn:creature.modified_gnarlhound` | `oteryn:achievement/mind_the_dog` |
| `oteryn:item.tibia.i17858` | `oteryn:creature.water_buffalo` | `oteryn:achievement/swamp_beast` |
| `oteryn:item.tibia.i19136` | `oteryn:creature.gravedigger` | `oteryn:achievement/blacknailed` |
| `oteryn:item.tibia.i20274` | `oteryn:creature.shock_head` | `oteryn:achievement/personal_nightmare` |
| `oteryn:item.tibia.i21186` | `oteryn:creature.walker` | `oteryn:achievement/gear_up` |
| `oteryn:item.tibia.i21439` | `oteryn:creature.noble_lion` | `oteryn:achievement/lion_king` |
| `oteryn:item.tibia.i24960` | `oteryn:creature.stone_rhino` | `oteryn:achievement/rhino_rider` |
| `oteryn:item.tibia.i27605` | `oteryn:creature.mole` | `oteryn:achievement/up_the_molehill` |
| `oteryn:item.tibia.i28791` | `oteryn:creature.flying_book` | `oteryn:achievement/bibliomaniac` |
| `oteryn:item.tibia.i30170` | `oteryn:creature.lacewing_moth` | `oteryn:achievement/lacewing_catcher` |
| `oteryn:item.tibia.i30171` | `oteryn:creature.hibernal_moth` | `oteryn:achievement/moth_whisperer` |
| `oteryn:item.tibia.i31576` | `oteryn:creature.gryphon` | `oteryn:achievement/gryphon_rider` |
| `oteryn:item.tibia.i34258` | `oteryn:creature.white_lion` | `oteryn:achievement/well_roared_lion` |
| `oteryn:item.tibia.i39548` | `oteryn:creature.giant_beaver` | `oteryn:achievement/beaver_away` |
| `oteryn:item.tibia.i43901` | `oteryn:creature.foxmouse` | `oteryn:achievement/like_fox_and_mouse` |
| `oteryn:item.tibia.i52638` | `oteryn:creature.battlefrazzle` | `(only Natural Born Cowboy)` |
| `oteryn:item.tibia.i5907` | `oteryn:creature.bear` | `oteryn:achievement/bearbaiting` |

## Other achievement mentions, without a mutating call

Each file below was included in the broad case-insensitive sweep. Catalogue definitions, UI/store descriptions, read-only checks, quest journal text and storage constants are not missing event producers. Dynamic declaration-only files are expanded above.

- `data/modules/scripts/charbazaar/charbazaar.lua`
- `data/modules/scripts/gamestore/gamestore.lua`
- `data-global/lib/core/quests.lua`
- `data-global/lib/core/storages.lua`
- `data-global/lib/quests/killing_in_the_name_of.lua`
- `data-global/lib/quests/svargrond_arena.lua`
- `data-global/lib/quests/their_masters_voice.lua`
- `data-global/npc/bron.lua`
- `data-global/npc/iskan.lua`
- `data-global/npc/oblivis.lua`
- `data-global/scripts/actions/other/others/quest_system2.lua`
- `data-global/scripts/quests/grave_danger/creaturescripts_zelosDeath.lua`
- `data-global/startup/tables/container_books_2.lua`
- `data-global/startup/tables/container_books_3.lua`
- `data-global/startup/tables/container_quest.lua`

## Validation targets

- `oteryn-gameplay-progression` unit filter `achievement_granters`: both rods/common/all rare fish, all existing simple skin yield kinds, dust, misses, unrelated tools, six active catalogue keys and exact donor thresholds.
- `oteryn-gameplay-dispatch --all-features` filter `standard_fishing`: real USE wire→owner→PostgreSQL on ordinary fishing, skin success/failure, dust, early ice, ice rare/common, mechanical rod; all counters seeded one below threshold; exact replay preserves first event/timestamp.
- Dispatch filter `npc_paid_travel`: real NPC route, rejected/stale/departure paths do not advance; actual first arrival advances once; carpet crosses threshold; later trips/replays preserve saturated counter and first grant provenance.
- Dispatch filter `rust_lottery`: real USE/USE-with success, break, lottery win/loss; only successful rust advances; receipt and wire replay preserve provenance.
- Existing type2 receipt, custody, session-generation, capacity and transaction guards remain the authorities. New mounted tests return `Ok(())` without PostgreSQL configuration and print `POSTGRES_CASE_COMPLETED P11` only after actual assertions.
- Dispatch unit filters `p11_account_observation` and `p11_notice_facts`: actual live transport action/replay/chest dedup, capability 8/31 gating, resync silence, existing periodic cadence, control loss, catalogue metadata, same-millisecond and older timestamps, duplicate/unknown keys.
- Mounted PG `p11_account_notice_postgres_observes_commits_replay_and_session_fences`: real grant owner and composed observation; committed/replayed account facts, catalogue points/names, mismatched account/transport/generation/lease, missing actor and actual durable control loss.
- `condition_potions` unit and mounted PG filters: each real buff and live combat consumer, exact 600000 ms expiry, shared donor slot replacement, actual-use threshold, replay, vocation/stale refusal, pre-commit absence and post-commit lost-ack reconciliation retaining the original expiry. Cross-session persistence is the Q8 dependency above.
- Foundation filter `condition_attributes_zero_slot`: the three Q9 regressions cover Swift Foot then potion, potion then shorter Swift Foot, and independent nonzero sub-IDs; the third also exercises shared dispel, suppression and Cleanse immunity. Their actual three-case RED and GREEN execution is recorded in the adjacent `validation.md`.
- Python `test_condition_potion_profiles.py` verifies four exact identities, source/canonical provenance, physical policy, no HP/MP consumption semantics and refusal before any output mutation. `test_gathering_item_profiles.py` verifies mechanical-rod admission at qualified weight 920 and rejects weight, binding, canonical or pinned Wiki revision drift.
- Exact test commands, executed results and remaining validation status belong to the adjacent `validation.md` and the PR. Test target descriptions above are not substitutes for those results.
