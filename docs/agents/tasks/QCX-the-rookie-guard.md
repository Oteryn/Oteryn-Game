# QCX77 The Rookie Guard Quest

Base `871c1dec7d9963420d455210e4cddc0c9df80d26`. The cached TibiaWiki export was
fetched on 2026-11-17 and carried on qfacts commit
`460e1d8c2c0e88c438ae5e54910db00822dc0129`. The quest page defines availability and the aggregate
reward list. The spoiler's walkthrough defines stage order and reward timing.
The archived transcript still says all four catapults, while the walkthrough and
Crystal both say two distinct catapults out of four. The walkthrough controls.
[Quest, curid 53931, oldid 891816](https://tibia.fandom.com/?curid=53931&oldid=891816)
[Quest/Spoiler, curid 53932, oldid 891815](https://tibia.fandom.com/?curid=53932&oldid=891815)

Crystal `201189f871d23597b82caf88fd611454a2ddefb8` corroborates item and script
identities. The donor NPC and quest script paths below are relative to that
revision's `data-global/` directory. The M09 import evidence links the older
pin used for the already admitted acceptance. [Crystal donor tree](https://github.com/zimbadev/crystalserver/tree/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/the_rookie_guard)
[M09 import evidence](../../reference/content-prep/qmech-m09/reward-sources.md)

## Differences at the base

The canonical definition has empty claims and no native lowering. Its 48
source interactions, 32 source progress records, and three door gates
(academy, library vault, crypt) are retained as import evidence. Its
non-runtime recipe is a six-mission approximation with aggregate intents for
one brass shield and seven potions. Its chosen completion comes after Tomb
Raiding and does not model Mission 12. Mission 11 acceptance is the sole
Rookie Guard entry in the active NPC callback and offer directories. It
correctly grants one silver amulet (3054) and one empty flask (12785), requiring
Mission10=3 and Mission11=-1. The imported NPC definition and neutral greeting
support that transaction. Vascalir is placed at (32104, 32196, 7) in
`content/world/npc-placements/placements-00000-00055.json`. The active and held world callback directories and
reward claim catalogue have no Rookie Guard binding to move or correct.

| Part | Global facts and identity corroboration | Difference at base |
| --- | --- | --- |
| Entry | Free account, Rookgaard, starting level 0–5, recommended level 2, rope and shovel for the route. New characters from Dawnport start at Mission 2. Mission 1's chest and Wooden Sword are historical. The transcript warns that continuation ends at level 9. Crystal `npc/vascalir.lua:64` rejects initial greetings above 5. | Premium and minimum match. The level range is retained as raw wiki data. No native Mission 2 start, level ceiling, route equipment rule, or level-9 continuation rule is bound. No Mission 1 grant is admitted. |
| 2 Defence | Load two distinct catapults from four choices with Heavy Stone (12724), one trip per stone, 5 XP per load, then one Studded Shield (3426). Crystal `mission02_defence.lua:90`, `npc/vascalir.lua:265`. | Recipe count is two. Barn pickup, per-target use, XP and NPC reward are unbound. |
| 3 A Rational Request | Five rat kills and one Sabre (3273). Crystal `mission03_rational_request.lua:5`, `npc/vascalir.lua:357`. | Five appears only in the recipe. Kill credit and Sabre grant are unbound. Crystal also grants 30 XP, which the cached wiki does not specify. |
| 4 Home-Brewed | Deliver Lily's herbs (12671) to Hyacinth. Two potions (7876), then one Star Ring (12669), and five Brown Mushrooms (3725) in a separate box. Crystal `npc/lily.lua:115`, `npc/hyacinth.lua:74`, `:113`, `:130`. | Recipe folds the NPC path into a generic delivery and aggregates the potions. No Lily or Hyacinth exchange, separate ring payout, or box claim. |
| 5 Web of Terror | Blue grease gives a two-minute window. Silk is progress only. Escape before expiry or retry after being webbed. Vascalir grants one Studded Armor (3378). Crystal `mission05_web_terror.lua:89`, `:116`, `:148`, `npc/vascalir.lua:430`. | Recipe treats silk as a collect target. Timed escape, armor, and separate confirmation are unbound. |
| 6 Run Like a Wolf | Poacher's knife (12672), skin (12740), hasted escape with retry, one Leather Boots (3552) and one potion (7876) from the box. Crystal `mission06_run_like_wolf.lua:151`, `:174`, `:198`, `:219`, `npc/tom.lua:150`. | Mission omitted from the recipe. No timed run or distinct chest and boots grants. |
| 7 Attack! | Academy library access, reusable Destroy Field rune, Book of Orc Language (12675), 100 XP and 100 gold (Crystal uses one platinum coin, 3035). Crystal `mission07_attack.lua:104`, `:137`, `npc/vascalir.lua:513`. | Mission, field removal, chest and hand-in omitted. |
| 8 Less Risk – More Fun | First successful deposit with Paulie credits 50 additional gold to the bank, then return to Vascalir. Crystal `npc/paulie.lua:161`. | Mission omitted. No quest-bound bonus or completion hook on the banking transaction. |
| 9 Rock 'n Troll | Pick (3456) and Leather Legs (3559) from separate chests, five beams, collapse on ascent, then 50 XP and one Brass Helmet (3354). Crystal `mission09_rock_troll.lua:94`, `:137`, `npc/vascalir.lua:595`. | Recipe counts five without beam identity, collapse or rewards. The donor awards 100 XP during the collapse rather than the wiki's 50 XP, so the donor payout must not be copied. |
| 10 Tomb Raiding | One Garlic Necklace (3083) on acceptance. Recover Fleshy Bone (12674), and separately claim one potion (7876) and five mushrooms (3725). Hand-in awards 150 XP and one Sword (3264). Crystal `npc/vascalir.lua:622`, `:664`, `:690`, `mission10_tomb_raiding.lua:83`, `:111`. | Recipe ends after a generic bone collection. Crypt gating, three separate claims, amulet, XP and sword are unbound. |
| 11 Sweet Poison | Empty flask on a fresh wasp corpse gives filled flask (12784). Hand-in consumes it and grants one Antidote Potion (7644) and 150 XP. A second yes grants one Brass Shield (3411). Crystal `mission11_sweet_poison.lua:54`, `npc/vascalir.lua:787`, `:835`. | Acceptance matches. Wasp extraction and hand-in are unbound. The shield appears only in the aggregate recipe. |
| 12 Into the Fortress | Academy chest returns tools in a bag (2853), one each of trap (12788), rolling pin (12789), filled flask (12784) and bone (12674). Disguise, distract guard, poison cauldron, paralyze guard, traverse barriers and defeat Kraknaknork to reach reward room. Separate chests grant 30 eggs (3606) and two plus one potions (7876). Crystal `mission12_into_fortress.lua:152`, `:197`, `:275`, `:307`, `:323`, `:457`, `:540`, `:646`, `:653`, `:741`. | Entire mission omitted. No kit or loot claims, fight, completion or reward-room access. Seven potions across the quest are 2 + 1 + 1 + 2 + 1. The summary lists Small Ruby (3030), 350 gold and Your Student Book. See OPEN-FACTS for allocation. |
| Cosmetics and access | No achievement, outfit or durable new area entitlement in the wiki. Local access is staged within the chain. Donor warns to leave for the Isle of Destiny at level 8. | No achievement or outfit difference. Local access gates remain unbound. |

In the binary placement data, decoding the Rookgaard windows
`x=31870..32190, y=32100..32260` in the y125 regions examined 208,027 tiles and found no matching donor
quest actions or uniques among the script's `25xxx`, `40xxx` and `50xxx`
literals. In particular the barn/catapults (40005–40009), webs (40010),
catapult quest routes, wolf items (40044–40045, 40076), library chest
(40047), tunnel chests (40048–40049), sarcophagus (40055), Academy kit
(40056), boss and fortress chests (40073–40074, 40079–40081), and wasp tower
trigger (50353) lack those selectors. A native placement binding would need
verified object coordinates and ordinals or a qualified placement import.

## BLOCKER

- BLOCKER Mission 2 entry and level-9 continuation upper-level NPC quest admission predicate npc/vascalir.lua:64
- BLOCKER Mission 3 five rats Rookie Guard scoped combat quest kill credit scripts/quests/the_rookie_guard/mission03_rational_request.lua:5
- BLOCKER Mission 5 spider lair timed condition-dependent capture and escape scripts/quests/the_rookie_guard/mission05_web_terror.lua:116
- BLOCKER Mission 6 wolf escape quest-scoped timed return and retry scripts/quests/the_rookie_guard/mission06_run_like_wolf.lua:151
- BLOCKER Mission 8 Paulie first deposit quest-progress participant and bonus in the bank transaction npc/paulie.lua:161
- BLOCKER Mission 12 guard distraction quest-triggered guard and pack spawn with restoration scripts/quests/the_rookie_guard/mission12_into_fortress.lua:275
- BLOCKER Mission 12 boss room Rookie Guard-specific timed encounter and kill-gated treasure route scripts/quests/the_rookie_guard/mission12_into_fortress.lua:558

## OPEN-FACTS

The cached Global pages do not specify exact placement coordinates, source
selectors, internal state values, a numerical failure duration for the wolf
run, or precise disguise and chest reset timings. They do not allocate the
final gold total among all sources or identify the exact chest for Your Student
Book. The 100 gold for Mission 7 and 50 gold bank bonus for Mission 8 are
explicit. A donor pair of platinum coins at the finale accounts for the
remaining 200, but the cached Global spoiler does not give that chest payout
directly. The donor-only 30 XP for the rats, 50 XP for skinning the wolf,
80 XP on the extra Tomb Raiding dialogue, and 50 XP beats in the fortress
are unconfirmed. The spoiler gives five mushrooms in each of two places, while
the summary lists a 5–10 range. The exact timing of the Global level-9 cutoff
is also unconfirmed outside the historical transcript.

## PARTIAL

The entry and remaining mission chain, including the filled-flask hand-in,
chest placement bindings, independent loot claims, and finale access remain
open. The brass shield confirmation is independently bound to the imported
post-hand-in state. Its dispatch case supplies that state with the existing
fixture transition.

## Dispatch evidence

PG via shard (pgsplit build files applied on
`a8380fd07843ed5bf4aab74e44aaaa364a87b4ed`). The pre-fix case reached the
real Vascalir NPC dispatch path with Mission11=4. The confirmation returned
success without advancing to 5, so the regression assertion failed. One
failed case in 123.13 s.
