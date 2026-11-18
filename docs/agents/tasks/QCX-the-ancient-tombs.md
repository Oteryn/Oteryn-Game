# QCX42 · The Ancient Tombs Quest

Base `98d09d5ce17df2a621df3fac9c2ce341dccc073b`. Global evidence is the quest page and spoiler fetched on 2026-11-17. Crystal reference is `zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8`. Coordinates in the spoiler are sector and offset, as decoded by `tools/content-schema/world-authoring/convert_hunting_places.py`. [Quest, curid 3792 oldid 756365](https://tibia.fandom.com/?curid=3792&oldid=756365) · [Spoiler, curid 5073 oldid 1174796](https://tibia.fandom.com/?curid=5073&oldid=1174796)

## Requirements and progression

| Area | Global | Base and remaining differences |
| --- | --- | --- |
| Requirements | Premium, level 75, at least eight Scarab Coins, shovel or equivalent and a Dwarven Ring important for Dipthrah. Team sizes and level 120+ are recommendations. | The chosen definition already records Premium and level 75. The active room callbacks do not provide comprehensive Premium and level admission. Exact level-door placements are unconfirmed. Coin 3042 is a per-basin payment. No runtime owns all eight drop-to-basin routes. A hard Dwarven Ring inventory requirement would overstate the spoiler. |
| Quest order | Seven pharaohs in any order, then Ashmunrah. | The chosen journal and M24 native kill bindings require earlier stage markers. Main has kill credit, but the generated graph has a fixed prerequisite chain and no final Ashmunrah binding. The retained completion source and lowering tools must agree before replacing that graph. |
| Omruc | Peninsula hidden entrance, basin, invisible-wall maze and boss exit. | Room markers exist. The wrong-step maze return, original entrance placement and boss-room exit are not routed end to end. The native pass counterpart currently binds a treasure-room exit. |
| Thalas | Eight switches, about four minutes from the first, poison tile then Cobra greeting. | Main implements shared switch leases and the threshold gate. Its per-switch lease is 20 minutes. Cobra has an NPC definition but no quest greeting rule that checks live poison and teleports. The poison tile's native effect and transformation are unresolved. |
| Dipthrah | Switches may be solved in any order with no timer. Level 75 and the listed eight door choices follow. | Eight sign-progress callbacks and the switch/sign gate exist. Switches expire after 20 minutes. The map counterpart has eleven lever placements and the gate expects seven, which needs source reconciliation. The spoiler provides no requirement to read the book. |
| Mahrdis and Ashmunrah | Avoid cycling purple fire (300 damage on Mahrdis entry), reach the boss, then complete Ashmunrah's switches once per character. | The Mahrdis room callback listens on UID 40084 and lacks the donor effect. Crystal startup records the target at (33183,32757,15). B3 has an unconditional teleport there, which bypasses the callback's prior-basin guard. Enabling the guard before its basin works would close the usable route. Ashmunrah's gate reads shared lease count and lets later entrants benefit from the same switches. The cycling hazards and per-character switch turn are incomplete. |
| Vashresamun | Play instruments in order, enter boss room, use Blue Note for the treasure room, take Left Horn and exit to Ancient Ruins. | Five ordered instrument callbacks exist. Main's native UID 3956 counterpart is a treasure-room exit and the success landing is on floor 14 at (33186,33012). Global puts the treasure room on floor 15 around (33144,32665). B3 corroborates with a sarcophagus at (33145,32663), and an exit at (33146,32668) returning near the Ancient Ruins entrance. Main incorrectly assigns the latter exit to Omruc UID 3953. The actual Vashresamun boss-room forcefield at (33116,32656,15) has no UID or native teleport destination in B3. The old landing is frozen into the PostgreSQL recipe and recorded in item audit provenance. |
| Morguthis | All blue flames, any order, then the final teleporter. | Seven native flags and a threshold gate exist. The base gate also refuses a returning character whose imported treasure progress exceeds 2, although all seven flags are sufficient. |
| Rahemos | Level 75, every player finds the carrot, wrong guess costs 200 HP, then cross the maze and lava route. | Main provides the levers/room movement markers, but no carrot guess branch or door outcome. |
| Pass items | The boss-room forcefield takes a bearer to that pharaoh's sarcophagus. Without the pass, it returns the player to the start. | The carried-step owner and seven donor identities exist, but their native counterparts bind exits beyond the sarcophagi. The B3 exit destinations also disagree with the donor's reward-room assignment for Vashresamun, Thalas, Dipthrah and Mahrdis. Crystal's startup table corroborates all seven boss-portal identity placements independently of those destinations. The spoiler gives item names. Crystal `movements_tomb_teleport.lua:1-43,59-66` and item XML resolve the item identities and one-item consumption. |

## Rewards and map coverage

The quest definition lists a single Helmet of the Ancients (3229) and has no ready claim. No matching Ancient Tombs runtime claim is indexed under `content/interactions/reward_claims`, and the held callback folder contains no Ancient Tombs file. The Crystal keys and item XML identify the expected one-per-character pieces.

| Pharaoh | Pass | Sarcophagus reward |
| --- | --- | --- |
| Omruc | Crystal Arrow 3239 | Helmet Adornment 3228 × 1 |
| Thalas | Cobrafang Dagger 3238 | Gem Holder 3223 × 1 |
| Dipthrah | Ornamented Ankh 3241 | Damaged Helmet 3226 × 1 |
| Mahrdis | Burning Heart 3240 | Helmet Ornament 3222 × 1 |
| Vashresamun | Blue Note 3236 | Left Horn 3225 × 1 |
| Morguthis | Sword Hilt 3237 | Right Horn 3224 × 1 |
| Rahemos | Ancient Rune 3235 | Helmet Piece 3227 × 1 |
| Ashmunrah | — | Assemble one 3229 from one of each part |

B3 has seven two-tile sarcophagi. The usable half is appearance 1994, which is the half with container semantics. Crystal's container table confirms five usable containers and their contents without a competing Global assignment. The two conflicting Omruc and Vashresamun rewards require a coordinated routing correction. [Crystal container registrations, lines 615–700](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/container_quest.lua#L615-L700) The floor assembly and 30-minute Small Ruby (3030) transform to 3230 already exist. Main's altar counterpart is an authored bare tile beside the Morguthis endpoint. Global places the altar on the small stone table in Ashmunrah's reward room. The pinned startup table confirms AID 40040 on counter 2344 at (33198,32876,11), north of the retained Ashmunrah landing. B3 preserves the table at ordinal 1 without the action attribute. [Crystal table registration, lines 2667–2671](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/item.lua#L2667-L2671) The spoiler lists no quest XP or outfits. It mentions Poet Laureate in connection with the Dipthrah book, without specifying an award trigger. [Spoiler](https://tibia.fandom.com/?curid=5073&oldid=1174796) · [Crystal assembly, lines 1–45](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/the_ancient_tombs/movements_craft_helmet.lua#L1-L45)

## Corrections

- Bind the five corroborated sarcophagi to a distinct per-character reward marker and grant exactly one part through the existing item reward transaction.
- Route the five corroborated pass items at the corresponding boss-room forcefields. Native treasure-room exits retain their ordinary behavior.
- Bind floor assembly and the Small Ruby transform to the actual Ashmunrah north table.
- Allow the Morguthis teleporter after all seven flames regardless of a higher imported treasure value. A successful entry preserves that value. The older test's exact assumption of an upper gate is replaced with exact preservation assertions for the same values. [Spoiler, curid 5073 oldid 1174796](https://tibia.fandom.com/?curid=5073&oldid=1174796)

## Blockers

Donor paths below are relative to Crystal commit `201189f8`.

```text
BLOCKER eight coal basins atomic AddItem sacrifice and flame-occupant teleport data-global/scripts/quests/the_ancient_tombs/movements_all_teleports_tombs_coal_basin.lua:30-55
BLOCKER Thalas switches shared first-switch window and reset data-global/scripts/quests/the_ancient_tombs/actions_active_teleport_switches.lua:34-50
BLOCKER Dipthrah switches persistent shared switches without expiry data-global/scripts/quests/the_ancient_tombs/actions_active_teleport_switches.lua:10-15
BLOCKER Ashmunrah switches per-character puzzle completion and consume-on-entry data-global/scripts/quests/the_ancient_tombs/movements_enter_ashmunrah_teleport_switche_done.lua:9-16
BLOCKER Thalas poison/Cobra atomic live poison and greeting teleport data-global/npc/cobra.lua:48-69
BLOCKER Rahemos hat trial random carrot, wrong-guess damage and temporary door data-global/scripts/quests/the_ancient_tombs/actions_oasis_lever_door.lua:26-34
BLOCKER Vashresamun/Omruc pass destinations versioned durable recipe and audit binding data-global/scripts/quests/the_ancient_tombs/movements_tomb_teleport.lua:21-24,39-42
BLOCKER Omruc maze return-to-step-origin routing data-global/scripts/quests/the_ancient_tombs/movements_peninsula_tomb_maze.lua:3-15
```

## OPEN-FACTS

- Exact native entrances, level-door selectors, basin item targets, puzzle switch placements and hazard timing. The spoiler's image references do not expose tile identifiers or sequences.
- Exact deadline behind “about four minutes”, group timeout reset semantics, and the stable per-player Ashmunrah switch reset rules.
- Correct Omruc treasure location once Global Vashresamun geography overrides the conflicting donor assignment. Check the conflicting native exit destinations for Thalas, Dipthrah and Mahrdis.
- Exact Poet Laureate trigger, any permanent Soul Ruby behavior, and any rewards beyond the seven parts and helmet described by the spoiler.

## PARTIAL

Encounter and entrance parity, the unordered completion graph, Omruc and Vashresamun treasure claims and boss-room bindings, native return destination reconciliation, and achievement qualification.

## Validation

PG via shard (pgsplit build files applied on `2ed10172db54b0cb25d271eb66ac5c58dad015a9`). PostgreSQL 18.6 returned RC 0 with 9 passed, 0 failed and 0 ignored in 195.10 s.

| Case | Duration |
| --- | ---: |
| Ashmunrah north table assembly | 27.47 s |
| Ashmunrah north table enchantment | 26.02 s |
| Dipthrah forcefield | 21.43 s |
| Mahrdis forcefield | 24.63 s |
| Morguthis forcefield | 19.27 s |
| Morguthis seven-flame reentry | 17.50 s |
| Rahemos forcefield | 19.24 s |
| Five once-per-character sarcophagus claims | 18.67 s |
| Thalas forcefield | 20.85 s |

The baseline run returned RC 101 with nine failures. Assembly, enchantment, the five rewards, and the Mahrdis, Rahemos and Thalas forcefields reached their gameplay assertions and reproduced the differences. Dipthrah, Morguthis forcefield and Morguthis reentry needed fixture repairs before they reached the corresponding gameplay assertions. Those three have passing PostgreSQL evidence on the corrected head.

The route-less run returned RC 0 with 9 passed in 0.01 s and incurred no PostgreSQL case time. Quest authoring returned RC 0 across 44 check steps. World bundle pin-check returned RC 0 for one world. Formatting and dispatch library Clippy with `-D warnings` returned RC 0. The authoring hash refresh changed proof digests and generated provenance only in the other quest records. World pin inputs were unchanged. The temporary shard files were removed.
