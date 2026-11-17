# questpc3 — PARTIAL checked mechanics report

Checked against qmech `aca848557`; this first report covers verified API gaps, not a claim that every quest step has been audited. No gameplay changes accompany this report.

Quest evidence below refers to `content/quests/definitions/quests-00200-00299.json`, JSON pointer `/records/N/definition/oteryn_recipe/payload/recipe/stages/I` (zero-based I). Source-complete mechanics must preserve the selected recipe counts; donor counts are not substituted here.

| quest | missing mechanic (server or client) | evidence (quest file and step, missing hook/table/command) | blocks the quest fully or partly |
| --- | --- | --- | --- |
| The First Dragon Quest | Server: annual NPC dialogue date-window predicate | Quest file record 23, stage s1 (Tamoril). `crates/gameplay/quests/src/quest/npc_rules.rs:149` exposes CalendarMonth only; input has epoch seconds and month at lines 329–330. `npc_rules/evaluate.rs:122` tests month equality, with no day-window predicate. A source Jan 14–Feb 12 window cannot be expressed by that predicate. | Partly: date-qualified start dialogue. |
| The First Dragon Quest | Server: player-admitting timed encounter with personal cooldown and cleanup | Quest file record 23, stage s5 (final confrontation). `crates/gameplay/quests/src/quest/world_callbacks/effects.rs:300–303` rejects timed encounters with players. Existing timing therefore cannot represent timed admission of the player into the boss encounter. | Partly: final encounter admission/lifecycle. |
| The Tainted Souls Quest | Server: captured target-stack quantity in carried transformations | Quest file record 43, stage s2 (bless/combine talisman). `crates/gameplay/items/src/item_craft.rs:157–160` requires operand counts of one, and transformed outputs at lines 181–186 use quantity one. No operand represents the captured whole target stack. Stack-to-nonstack output policy still needs separate source qualification. | Partly: source-faithful talisman crafting. |
| Threatened Dreams Quest | Server: transactional world-callback character clocks | Quest file record 51, stage s5 (moon mirror/fairy progression), including timed source acquisition prerequisites. `crates/gameplay/quests/src/quest/world_callbacks.rs:158` Callback exposes static storage and effects, but no state_operations field for an atomic elapsed-time predicate plus SetNow write. NPC epoch-time evaluation is a separate ingress and does not supply this world-callback operation. | Partly: timed world-item prerequisites; other steps remain playable independently. |

## Remaining audit

Ground cart hand-ins, live-creature item use, conditional treasure achievements, and complete native bindings remain to be checked in this package. Their omission from this PARTIAL report does not mean they are implemented.
