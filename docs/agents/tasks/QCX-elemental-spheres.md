# QCX66 — Elemental Spheres Quest

Quest key `oteryn:quest.elemental_spheres_quest`. Base `871c1dec7d9963420d455210e4cddc0c9df80d26`.

## Evidence

The supplied capture, fetched on 2026-11-17, is the Global authority for the comparison (TIBIAWIKI_STRUCTURED). [Quest, curid 25542, oldid 1178813](https://tibia.fandom.com/?curid=25542&oldid=1178813) · [Spoiler, curid 25641, oldid 1179410](https://tibia.fandom.com/?curid=25641&oldid=1179410)

Crystal `201189f871d23597b82caf88fd611454a2ddefb8` is OTS_HYPOTHESIS_ONLY. The retained NPC source is in `content/dialogues/quest-sources/elemental_spheres.json`, with the full Lua and its checksum. The donor step inventory is in `tools/content-schema/quest-authoring/batches/quests4/elemental_spheres.json`. The checkout has a historical Arkulius comparison in `docs/reference/content-prep/questpc2-m08-live-eligibility.md`.

## Comparison

| Part | Global and current repository |
| --- | --- |
| Level, Premium and team | Level 80 and Premium already agree in the definition and four admitted mission offers. The wiki lists a quest log. The definition is script-only with native lowering still held, so this does not prove a live end-to-end log. Level 100+ is recommended. Global lists one character of each of five vocations across the quest, while the final chamber paragraph describes four shrines. The chosen definition substitutes a personal mixed-gem route and omits the team and Monk branch. |
| Quest opening | The wiki places Arkulius beside Alverus in Edron. The retained promotion candidate is north-facing at 33280,31845,3, and the placement scope omitted him. The Global Alverus, shrines, sphere and below-level mission replies were missing from the live quest program. Global refuses to start if the player carries any of the five soils or five Overlord drops. The donor and the native program accept based on level, vocation and stage, with Premium added in Oteryn. |
| Gem machine and spheral access | Global requires twenty matching gems per vocation (Sorcerer ruby 676, Druid emerald 677, Knight amethyst 678, Paladin sapphire 675) and five of each of the four gems for Monk. It accepts gems singly while off, caps the count at twenty, and charges a fresh set for re-entry. The chosen recipe has a single aggregate of twenty mixed gems. Crystal provides four single-element machine branches at 33268/33269,31830,10 and can retain excess gems. The placements contain the two inactive machine objects 842/843. There is no live quest machine callback. |
| Soils and Overlords | Global requires eight charged sites, then a matching elemental corpse, one active Overlord at a time and a repeatable 100-second window starting with the first soil. The four legacy soil conversions and corpse-arrival/latch owner already exist on main. The charge profile has per-object 120-second decays for the three terrain transforms. There is no first-soil shared 100-second window or Spirit altar binding. Global's five drops are Eternal Flames, Mother Soil, Pure Energy, Flawless Ice Crystal and Reality Splinter. The live Arkulius report requires the corresponding first-four items (946, 947, 948, 942) and advances from stage 1 to 2 on possession alone. Global additionally requires highest damage. Main has a shared top-damage source death owner for registered creatures, with no Elemental Spheres binding. |
| Sphere exits and chamber admission | The donor has a return selector at action 24545, plus a four-character keyed entrance at unique 1010. The live callback catalogue has neither. The quest exit callback is already in the live registry and selects action 9029. The checked Edron placement regions contain the untagged entrance lever (2772 at 33269,31835,10) and untagged exit teleports (1949 at seven cells on floor 12, with destination 33265,31838,10). |
| Lord encounter | Global has separate charge machines, four central switches, walls, a ten-minute encounter and re-entry after ten minutes. The four central 2772 placements at (33262/33273,31831/31840,12) have no unique tags. The donor selectors 1011–1014 and machine/room state are recorded in the step inventory. The live callbacks have no charge, coordinated fight or timeout for this encounter. The existing party-transfer owner requires ground offerings and cannot enforce each participant’s carried boss item or quest progress. |
| Ashes | Global states a ten-second loot window, followed by Obsidian Knife skinning, with Neutral Matter for all players. The donor skin rule targets ashes 8181/8182 and produces 954 for a player using a personal skinning storage. There is no qualifying live quest callback for the full encounter-to-ash-to-team route. |
| Final reward | Crystal NPC turn-in consumes one Neutral Matter (954) and declares one of Dragon Robe (8039), Greenwood Coat (8041), The Ironworker (8025) or Windborn Colossus Armor (8055). These four selections already exist in the live NPC rules, which set stage 3 without a canonical completion flag. The canonical Item tree marks their records and Neutral Matter nonmaterializable. The effective spell-native profile shard also lacks their carried item policies. No separate record in `content/interactions/reward_claims` is linked to the quest. The chosen definition lists only Dragon Robe. Global adds Spirit Bind for Monk (client identity 51294 in the Item tree, currently nonmaterializable) and Lord of the Elements for everyone (canonical achievement already exists). Global lists no quest XP or outfit. |
| Arkulius shop | Global transcript limits Arkulius to buying Neutral Matter at 5,000 gold. The repository listed that plus nine older or Spirit offers. Crystal lists the classic soils and boss drops in addition to Neutral Matter. Global controls the projected offer list. |

## BLOCKER and OPEN-FACTS

`BLOCKER Arkulius start exclusion shared inventory planner cap of eight distinct items versus eleven required including the turn-in item data-global/npc/arkulius.lua:104-145`

`BLOCKER sphere entry atomic gem count with split-machine toggle and vocation-specific teleport data-global/scripts/quests/elemental_spheres/actions_machine1.lua:2-5`

`BLOCKER Overlord window first-soil timed group activation and expiry data-global/scripts/quests/elemental_spheres/movements_summon_overlord.lua:2-79`

`BLOCKER Arkulius Overlord report Overlord binding and NPC guard for the closed shared top-damage credit owner data-global/scripts/quests/elemental_spheres/creaturescripts_overlord_kill.lua:1`

`BLOCKER Lord chamber per-character carried checks, coupled switch and machine state, wall closure and timeout data-global/scripts/quests/elemental_spheres/actions_lord_of_elements_lever.lua:63-129`

`BLOCKER Neutral Matter all-participant ashes entitlement and bounded loot window data-global/scripts/actions/tools/skinning.lua:259-264`

`BLOCKER Arkulius payout Neutral Matter and the vocation rewards require native Item admission data-global/npc/arkulius.lua:162-165`

OPEN-FACTS. Global does not give numeric selectors or map cells for the Monk route, the selector-to-cell mapping of the exits, the Monk role in the four-shrine final sequence, spirit altar/corpse bindings, the exact achievement grant event, the precise re-entry clock anchor, loot probabilities or tie-breaking for equal highest damage. The captured wiki pages name quest items and rewards without giving client IDs. Numeric identities in the table are from the retained donor and canonical Item tree.

PARTIAL. Chosen quest definition, start inventory exclusion, machine and realm admission/return, Spirit branch, highest-damage credit, exact altar timing, chamber encounter and placement tags, ashes distribution, native reward/Neutral Matter custody and achievement trigger.
