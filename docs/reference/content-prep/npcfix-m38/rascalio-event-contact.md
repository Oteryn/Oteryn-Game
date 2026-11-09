# Rascalio event contact

Retained source: `content/npcs/definitions/npcs-00500-00999.json`, record 460,
`declaration.fields` / `oteryn:source.npc.bulk.source_metadata`;
<https://tibiopedia.pl/npcs/Rascalio>. The source distinguishes the Nimmersatt's
Lair contacts from Rascalio's other quest roles.

The Carlin reference anchor is (32259, 31643, 7). A real-map compiler probe
admitted it with ordinary collision checks. The other event anchor at
(32378, 31592, 7) was held as NotWalkable and is not enabled by this slice.
These reference coordinates do not establish retail placement equivalence.

The server projects the existing durable world-change key `nimmersatts_lair`.
This is an explicit Oteryn binding, not a source-derived activation schedule.
A missing/inactive committed state produces no event actor. Event activation
policy and its producer remain separate unfinished work. The PG test's forced
0/1000 probabilities exercise both states; they are not production policy.

Each projection owns its exact actor occurrence, so retirement preserves
Rascalio at Meriana and other quest locations. Cell occupancy defers appearance
through the shared conditional actor owner; ordinary map admission still gates
construction. Database read failures retain the last successful projection and
retry on the next world tick, matching the existing timed NPC loop policy.

## Basic dialogue adaptation

The retained Oteryn Dialogue is
`content/dialogues/definitions/dialogues-00500-00835.json`, identity
`oteryn:dialogue.npc.rascalio`, revision `definition-r1`. Its two keywords are
`name` ("I am known as Rascalio.") and `job` ("My role here is Ferryman.").
These are existing project replies, not newly asserted canonical source quotes.

Basic contact admits only those two literal leaves, with no service, fee,
exchange, mission or travel authority. The existing full greeting assumes a
mission for the Draccoon; it cannot establish that state for an event visitor.
The explicitly adapted Oteryn greeting therefore keeps only its neutral prefix,
"Be greeted, |PLAYERNAME|!", rendered with the admitted customer's name.
The talk-book revision includes this policy and greeting. Cook-quest fidelity,
ferrying and event activation remain outside this basic contact slice.
