# questpc shared capability requests

Owner: qengine2. These requests do not authorize approximations or whole-quest
qualification. Main47db34f09 and live qengine2 head279a9cff8 were checked.
Continue using existing owners; do not add a second persistence or timer path.
Source URLs and immutable hashes are in CONTRACT-NOTES.md and QUEST-STATUS.json.

| Capability | Needed by | Source behavior / regression boundary |
|---|---|---|
| Multiple fresh root item types in one NPC receipt | Bigfoot's Burden Quest; An Interest In Botany Quest | Xelvar gives four crystals; Rabaz consumes11699 and gives11700 plus3035x10 and3000XP. Atomic capacity/refusal/replay; no synthetic container or wallet substitution. |
| Live equipped-slot predicates in NPC item programmes | Blood Brothers Quest | Julius stage11 requires item3083 in necklace slot. Carried-only and wrong-slot instances must refuse without source progress. |
| Calendar year/month projection and current-year writes | Hot Cuisine Quest | Jean Pierre's annual August period, once-per-year first-menu bonus and per-dish bonus flags;365-day cooldown is not equivalent. |
| Revision-fenced timed ground transforms from USE/StepIn | An Uneasy Alliance Quest; Lion's Rock Quest | Uneasy reset10s; Lion pagodas/amphora15s and flower60s. Grant and object mutation share the fence; stale reset cannot overwrite a newer object. |
| Replay-bound random quest outcomes and live condition predicates | Barbarian Test Quest | Mead math.random(5)>1, failed sip resets success0; exact counter/early-return behavior. Mammoth requires actual CONDITION_DRUNK. Replay cannot reroll. |
| Item-arrival trigger and atomic world readset/corpse mutation/boss summon | Elemental Spheres Quest | Source onAddItem, all seven charged altars, exact corpse/object identity; failure grants no partial progress. |
| Player-driven shared world-state predicates and fenced refill/object reset | Krailos Quest | Salt Lake dryness, six refill uses and toxin removal; machinery sabotage/reset. Do not substitute character-only flags. |
| Retain actual historical NPC13 definition predecessor | Hidden Threats Quest / catalogue compatibility | Existing regression pins8c6494650b3d22ccd66d6893c77f8b0ad16b06dfcd69725316150c1be49ee084. Keep its real historical definition/defaults; never replace the expected hash. |

| Carried-tool callback composes counter increment, temporary outfit and replay-bound random spawn | In Service of Yalahar Quest | Original sewer UIDs3071–3074: crowbar3304/9598, four once-only flags, Mission01 increment; Skeleton3s/BogRaider5s outfits and rat spawn branch. Never publish four flag writes with missing coupled effects. |

| Source player public speech and fenced multi-object conditional toggles with delayed lever reset | Children of the Revolution Quest | Oil10189 success uses TALKTYPE_MONSTER_SAY; puzzle toggles four existing cells from five source levers and resets9126→9125 after5s. Private success text or fixed transforms would change the source. |

| Quest objective receipt after a successful native item-tool action, preserving its transformation and decay | Kissing a Pig Quest | Native machete3696→3695 calls decay; Hoggle garden completion must share the successful tool fence. Do not replace the native action with a permanent callback transform or count refused/replayed cuts. |

Resolved requests: main QNPCFX supplies actual UTC calendar month and delayed
conversation end. Citizen uses that existing programme planner through a small
local adapter; both mainland source stories pass on PostgreSQL18.6.
