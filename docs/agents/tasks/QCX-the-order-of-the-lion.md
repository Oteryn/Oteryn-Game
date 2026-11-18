# QCX75 The Order of the Lion Quest

Base `871c1dec7d9963420d455210e4cddc0c9df80d26`. Global evidence is the fetched wikitext from the quest page and spoiler, with the Crystal 201189f8 quest-authoring packet and retained NPC extracts as donor evidence. [Quest, curid 91781 oldid 847762](https://tibia.fandom.com/?curid=91781&oldid=847762) · [Spoiler, curid 91784 oldid 1097856](https://tibia.fandom.com/?curid=91784&oldid=1097856)

| Part | Global and imported behavior |
| --- | --- |
| Entry | Global requires premium and level 250. The donor boat action 59602 checks level 250. The quest callback inventory has a return action 59603 but lacks a playable outbound action 59602. The batch-four level guard requires a personal state effect, and this boat has none in the retained donor. |
| Evrard | Global starts the conversation by day and grants southern access after inspecting the ledger and map. The transcript says the player keeps them, while donor `evrard.lua:79` debits one each (635 and 2822). The document unique IDs 47404 and 47405 and NPC dialogue are source evidence without admitted quest-specific document or NPC handlers. |
| Residents | The donor matches the Global quantities: 20 Broken Longbows (34161), 10 Bread (3600), 20 Ham (3582), 20 Crypt Warriors and 20 Deer. Both NPC schedules need playable hand-ins and both kill quotas need the highest-damage player as owner. The retained source dialogue carries these rules. The NPC callback and offer banks have no active records for these hand-ins. |
| Castle | The two STEP entries checked AccessEasternSide, which donor Evrard sets before any of the resident tasks. Both now require five trust credits, using the same read-only predicate as elevator ascent. Global places castle admission after all five residents. The guard conversation and the two floor-resolver-held stairs still need qualification. |
| Siege | Global requires a day conversation, a ten-minute wait that restarts on early conversation, a one-Tibian-day wait, then night Fugue and five warlocks. Fugue credit already has a native owner. The earlier Kesar NPC timing and subsequent negotiation remain unbound. Falcon and Cobra access belong at the post-Fugue siege stage. |
| Drume | Global allows up to five players, requires three Usurper Commanders, fails when the three Lion Commanders die, and then spawns Kesar and Drume with possible Falcon aid. The retained encounter source describes room admission and cooldown. The full battle and mission-five producer are pending. |
| Finale | The quest definition has no generic reward claims and the reward-claim bank has no quest-specific chest. The existing Kesar finale already writes Lionheart, Hero of Bounac and AugustinDoor through the fenced quest owner. Global also grants all-area access including the depot and Bounac NPC trading. The full depot and trader access path still needs qualification. The wiki describes a chance at Lion Set loot from bosses. |

## Blockers and remaining work

```text
BLOCKER Bounac outbound entry read-only level and Premium admission for the boat data-global/scripts/quests/the_order_of_lion_quest/action-bounac_entrance.lua:2
BLOCKER Bounac stairs floor resolver can bypass the down selector and trigger the opposite direction data-global/scripts/quests/the_order_of_lion_quest/action-bounac_entrance.lua:24
BLOCKER Drume battlefield commander-owned squad teardown, defeat ejection and phase spawns data-global/scripts/quests/the_order_of_lion_quest/creaturescripts_commander_kills.lua:8,30
```

OPEN-FACTS. Exact numeric reward IDs and loot rates, quest XP or outfit awards, Drume cooldown and deadline, and the engine coordinates and imported placements for the Wiki map links. The wiki names no fixed item delivery. Nearby B3 slices decoded during this pass did not expose the donor action and unique IDs, so placement binding remains open.

PARTIAL. Outbound boat, held stairs, Evrard and documents, resident hand-ins and kill credits, guard, Kesar timing and negotiation, Drume encounter and credit, depot and trading access. Treat the castle correction as a callback-level fix. Its PostgreSQL fixture uses local geometry and seeded resident history.
