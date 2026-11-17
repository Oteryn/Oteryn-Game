# QCX-37 Soul War Quest

Base `98d09d5ce17df2a621df3fac9c2ce341dccc073b`.

## Source access

The Fandom MediaWiki API returned HTTP 402, `{"message":"Please contact the site owner for access."}`, for both titles and both pinned revisions. The repository’s retained structured capture is in `tools/content-schema/quest-authoring/wiki_quest_facts.json`. It cites [quest curid 91798, oldid 1192477](https://tibia.fandom.com/index.php?curid=91798&oldid=1192477) and [spoiler curid 91800, oldid 1134584](https://tibia.fandom.com/index.php?curid=91800&oldid=1134584). The prior quest-pattern audit at rows 739–740 reports the historical cooldown and cage differences below. The live API response provided no current revision or page ID.

Crystal Lua was read at `201189f871d23597b82caf88fd611454a2ddefb8`, using the repository and pin identified in `docs/reference/QUESTPC8-CONTRACT-NOTES.md`. The existing [callback inventory](../evidence/OTV2-20261102-quests-2-20261102.md) documents the wider donor surface, including the leads requiring current-main readback.

## Entry and claims readback

| Surface | Evidence and finding |
| --- | --- |
| Definition | `quests-00200-00299.json` describes a script-only import and has an empty claims array. Native gameplay handles the hunt access and final chest independently of that record. |
| Global snapshot | The retained quest extract lists premium status, level 250, Soul Set, Soul Mender, an achievement reference and Revenant Outfits. The spoiler extract lists Feaster of Souls under Requirements and marks its execution semantics unknown. The native hunt callback checks level and acceptance locally. The higher-level premium and prerequisite gates require end-to-end readback. The native item pool is the 18 Crystal items and contains no separate Soul Mender grant. The extract does not supply reward counts or selection semantics. |
| Five hunt portals | Crystal `moveevent-soul_war_entrances.lua:1-50` requires level 250 and Flickering Soul acceptance. The five source/destination pairs and both requirements are present in native dispatch's `soul_war_world.rs`. |
| Claustrophobic Inferno exit | Crystal `moveevent-soul_war_entrances.lua:90-118` matches the active callback from (34022, 31091, 11) to (33685, 31599, 14). The position selector carries no action or unique ID. No quest-specific held callback file was found. |
| Boss gates | Crystal requires five distinct boss flags at the physical Megalomania step (33611, 31430, 10). Main has native final boss admission based on the boss flags and lacks a matching authored physical gate. The Crystal Spite step requires 20 Hazardous Phantoms. The earlier inventory also reports a Greed step after 25 of each of five apparitions. Physical access for the latter two requires further readback. |
| Chest | Crystal `action-reward_soul_war.lua:3-28` awards one random item per character after Megalomania at (33620, 31400, 10). All 18 Crystal IDs (34082–34099, `lib/quests/soul_war.lua:84-103`) match the native pool. The native dispatcher owns this claim. The authored reward-claim shard's Goshnar text belongs to a different claim. |
| Reward-room portal | Crystal requires Megalomania completion at (33621, 31416, 10), sends eligible players to (33621, 31411, 10), and returns other players to their previous tile. The authored callback list lacks this portal. Completion lives in native Soul War state and the generic world callback has no predicate for that state. |
| Flickering Soul | Crystal `npc/flickering_soul.lua:125-138,184-191` handles Revenant base outfit after final completion, hunt access through task acceptance, and taint cleansing. Main has dedicated native state and outfit grant paths. The prior inventory also records multipart dialogue differences requiring current-main verification. |
| Historical cooldown | Prior spoiler audit reports 68 hours. The imported Crystal lever candidate carries 72 hours (259200 seconds, `lib/quests/soul_war.lua:471`). The raw Crystal candidate records donor evidence. Native activity qualification loads a separate source-pinned recipe in `tools/content-schema/native-gameplay/srx3-soul-war-entry-recipes.json`. The existing activity-template regression asserts 72 hours. A change needs a current global citation and coordinated updates to that native recipe and assertion. |
| Historical Soul Cage | Prior spoiler audit reports 14 seconds to destroy it. Crystal gives 40 seconds before removal and empowerment (`lib/quests/soul_war.lua:979-1004`). The native 40-second spawn cadence is a separate timing measurement. Current global expiry and live runtime expiry require verification. |

## Blockers and remaining work

`BLOCKER reward-room portal authored StepIn cannot read native Megalomania completion data-global/scripts/quests/soul_war/moveevent-teleport_entrance_reward.lua:9-22`

`BLOCKER physical Megalomania gate authored StepIn cannot read native distinct miniboss flags data-global/scripts/quests/soul_war/moveevent-soul_war_entrances.lua:60-88`

OPEN-FACTS: current wiki page and spoiler revision IDs, premium and Feaster enforcement points, global access, Soul Mender delivery, item pool and counts, standalone quest XP, achievements, outfit addons, cooldown and Soul Cage expiry.

PARTIAL: current-global comparison, placement and unique/action readback, detailed zone and boss progression, NPC callback parity, and runtime qualification for the historical differences. This pass produced an audit report. There are zero gameplay corrections and zero new regression cases.
