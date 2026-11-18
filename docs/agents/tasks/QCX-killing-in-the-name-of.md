# qcx68 — Killing in the Name of... Quest

Base `871c1dec7d9963420d455210e4cddc0c9df80d26`.

The 2026-11-17 TibiaWiki wikitext was fetched from the supplied facts branch at `460e1d8c2c0e88c438ae5e54910db00822dc0129`. It takes precedence for Global gameplay. [Quest, curid 36695, oldid 843864](https://tibia.fandom.com/?curid=36695&oldid=843864) · [Spoiler, curid 36696, oldid 1206121](https://tibia.fandom.com/?curid=36696&oldid=1206121)

Crystal `201189f871d23597b82caf88fd611454a2ddefb8` is the donor. Its eight blobs reproduce the existing catalogue through `grizzly_source.py --check`.

| Task | Imported | Global | Crystal line |
| --- | --- | --- | --- |
| Mammoths | 4,000 XP | 5,000 XP | 291 |
| Terramites | 1,000 XP | 2,500 XP | 321 |
| Gargoyles | 300 kills, no points | 200 kills, 2 points | 362 |
| Mutated Tigers | 300 kills, 2 points | 250 kills, 3 points | 528 |
| Hellspawns | No points | 4 points | 614 |
| Demons | Demon only, no XP | Demon and Infernal Demon, 100,000 XP, 0 points | 886 |
| Tiquandas Revenge / Demodras | Level 80 / 80 | Level 90 / 100 with Trophy Hunter standing | 917 |
| Grizzly promotions | Rank only | 5,000 / 8,000 / 10,000 / 15,000 / 30,000 XP, Huntsman and Elite Hunter achievements, Trophy Hunter backpack | 741–769 |
| Elite Hunter task points | Can continue increasing from 100 | No points from completion at 100 or above | 484–504 |

The table cites `data-global/lib/quests/killing_in_the_name_of.lua` for ordinary tasks and `data-global/npc/grizzly_adams.lua` for the special-task gate.

The native Grizzly handler already supports Premium and a three-task limit, persistent kill credit, task rewards, boss admissions, and the Holy Icon. The quest definition's empty claims and waiting marker describe the generic lowering; the native path has its own durable reward owner. The Holy Icon (9388, one) connects to Demon Oak. Herbicide belongs to Demon Oak completion. The Global capture lists no outfit grant.

The ranked catalogue generator applies the Global task values. The existing task XP, kill-credit, rank achievement, and Holy Icon owners settle the effects. The special-task thresholds apply on assignment. A hunter can still report an accepted assignment after losing a level. Trophy Hunter standing continues to use the native point rank.

PARTIAL — Trophy Hunter backpack and Lorek travel, boss-point rules and special retries, unranked NPC routes, the Oteryn campaign, and shared-XP hit eligibility.

## Remaining differences

- The ranked task tables agree on the other named task kills, XP, points and boss pairings. The generic definition preserves a separate, authored three-hunt Oteryn campaign with its own 10,000 XP and 5,000 gp. That campaign has no counterpart in the Global capture. Its callback remains live and is listed as PARTIAL.
- The rank thresholds (10/20/40/70/100 points and levels 6/6/50/80/130) agree. Promotion now grants the five Global XP amounts and both named achievements. The Global table also lists a Demon Backpack at Trophy Hunter. The catalogued shop sells Demon Backpack (9601) for 1,000 gp. The Holy Icon entitlement guard admits only item 9388 from the Demon completion receipt, so it cannot carry the promotion item. The Trophy Hunter item grant remains open.
- The listed trophy prices and point thresholds match the native shop. The donor also offers a Werehyaena Trophy at 12,000 gp. Lorek has four service routes in the repository. His Ranger-gated Banuta/Chor routes and mountain-pass wording are absent, while the existing Darama route has no rank condition. Crystal documents the conditional routes in `data-global/npc/lorek.lua:54-69`.
- The corrected rules stop awards once the player reaches 100 points. The Global method also describes a one-time catch-up at exactly 70, which remains open. The current boss menu tracks a single balance, reopens completed lower-tier bosses, and treats any first outgrown-tier selection as free. Global gives the free lower-tier choice only with a qualifying 130+ boss point, allows each outgrown boss once, and awards choices on capped completions even for tasks without an associated boss. Crystal NPC lines 346–361, 462–504, and 776–819 explain the inherited behavior. The current special-task rearm can repeat after multiple failures. Global permits one repeat.
- Main's Grizzly death path freezes the most-damage principal and the live shared-XP roster. Global requires a hit for every shared-XP recipient, attributes summon damage to the owner and excludes kills of summoned targets. The native reward path already excludes summoned targets. The current Grizzly branch does not filter that roster by hits, although the carrier exposes contributors. Shared-XP hit filtering remains PARTIAL. Crystal death helper `data/libs/functions/functions.lua:1064-1074` explains the imported roster behavior.
- The Global table has eleven unranked tasks. The native Grizzly catalogue contains only the 42 ranked tasks, and there are no equivalent full NPC offer, death-credit, payout and boss routes for the unranked set. Budrik's existing source storage fragment is evidence of a partial binding. The missing payouts are Daniel's four tasks (200/300/1,000/3,000 XP and 200/250/400/800 gp), Gabel and Malor (each 10,000 XP and 5,000 gp), both Raymond pirate tasks (each 10,000 XP and 5,000 gp), and Lugri's second hunt (40,000 XP). The Global unlocks include Goblins after Trolls, Cyclopes at 40–59 versus donor 30–59, faction prerequisites for both Djinn hunts, Meriana and three first-task finishes for Raymond's repeat route, To Outfox a Fox for Budrik, and Lugri's first hunt before the second. The missing boss paths are Merikh, Fahim, the random pirate boss, The Horned Fox and Necropharus. Crystal library lines 72–211 and monster death lines 59–77 cover the unranked objectives. This is a separate PARTIAL slice.
- The live ranked and special entrances are native positional callbacks. Decoding the map yielded item 1949 at all 30 donor entrance tiles, with no unique or action attributes. Bloodweb (32122,31185,4), Fazzrah (33310,31184,7) and Paiz (33066,31103,2) have different Wiki markers. Those markers are nearby map locations without item 1949. The verified trigger tiles are (32122,31188,4), (33310,31183,7) and (33066,31104,2). The unranked Global markers also differ from Crystal for Fahim, pirates and Necropharus. Exact unranked trigger tiles remain open.
- The quest definition pins source evidence and has no generic claims. The native Holy Icon claim and boss achievements already have dedicated runtime owners. The active and held callback trees have no matching Grizzly quest-key records to promote. No outfit is specified. The Global aggregate of 31 achievements exceeds the 28 ranked boss achievements and two named promotions. The capture does not explicitly identify the remaining achievement.

`BLOCKER Trophy Hunter Demon Backpack promotion item entitlement game_character_grizzly_reward_entitlements accepts only the Holy Icon data-global/npc/grizzly_adams.lua:760`

`BLOCKER summon-owner task credit shared lethal attribution for summoned attackers data-global/scripts/quests/killing_in_the_name_of/creaturescripts_monster_kill.lua:18-25`

OPEN-FACTS — exact unranked trigger tiles, duration/cooldown from Global, unspecified creature-product shop detail and Werehyaena offer status, Lorek fares and exact landings, remaining achievement identity, exact repeatability where the Global method prose gives general ranges.
