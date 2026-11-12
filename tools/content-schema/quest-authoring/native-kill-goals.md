# Native personal death goals

The existing retained death queue owns these goals. This is a quest progress
mechanism; a goal cohort alone does not qualify its quest's route or rewards.
The reusable implementation originates in questpc6 commit
`390e024503a7c02b9e15d67e680cec0ac711fd5b`.

`native_kill_goals.py` assembles source-checked cohort fragments. Schema 1 allows
1–16 goals, each with a unique transition key and counter, count 1–1000,
1–256 unique native origins and 1–8 bounded source references. Each source
pointer must name the exact captured creature and cell in a hash-bound spawn
shard. No region rectangle or encounter copy substitutes for an origin.

Each run has an inactive phase -1, active phase 0 and terminal phase 1.
The named activation is the only transition allowed to set -1 to 0; a run
cannot reopen. Counters start at 0 and cap at the selected count. Their only
increment writer is their registered native-kill transition with a
`CreatureDeath` cause. NPC/client command causes cannot manufacture a kill.
Other transitions may read a counter with an Add(0) guard.

Optional prerequisites name complete counters of the same quest. The bounded
prerequisite graph must be acyclic, refer to existing goals and use their exact
completion counts. Capture checks prerequisites in the death-time snapshot;
the durable transition checks them again. Killing a later objective before
its predecessor cannot gain credit through delayed settlement.

The lethal turn freezes the present most-damage principal's exact actor,
Character, session, lease generation and active goals. A retained dead actor
must prove its original native spawn cell; moving AI cannot change this origin.
Ordinary encounter actors have no such static origin. Settlement revalidates
current authority and lease; unavailable writes retry the same frozen occurrence
on the existing queue. Cancellation and repeated drains do not create a second
occurrence or increment. There is no separately drained quest-kill queue.

Selected native defenders retain ordinary reward admission. Their source corpse
closures preserve capacities, decay edges, explicit weights and WorldObject
movement/corpse identity. Missing weights reuse the existing Game non-pickupable
corpse weight-zero policy. Portable loot qualification preserves canonical
physical/combat facts and requires exact source/client/binding custody facts.
Charges and temporal facts must agree; source loot subtypes are never rewritten.
A fresh nonstackable loot instance uses the already accepted native definition
charge default (`crates/gameplay/domain/src/domain/loot_data.rs:71`).

Elvenbane selects six exact source origins, each capped at one. All five upper
objectives require the Dragon counter first. Its activation and return phase
callbacks are explicit Oteryn personal-route choices, separate from donor
storage and from whole-quest completion. Source geometry and native creature
stats remain unchanged. Full route, combat, treasure and exit qualification
requires the corresponding continuous PostgreSQL case.
