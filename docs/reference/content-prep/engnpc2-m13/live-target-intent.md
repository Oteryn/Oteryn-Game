# Exact live-target tool intent

`QuestCreatureUseProof::read_current` borrows the actual Channel runtime owner,
checks the selected live Creature generation, current Character lease and range,
and now retains the entire decoded `UseIntent`. The durable participant compares
that complete intent with the carried-use bytes, including the source subject.
A matching Creature alone cannot authorize a different carried-use intent.

Real PostgreSQL regression:
`engnpc2_live_creature_postgres_exact_intent_burn_and_replay` (49.871719889s).
The red case changed the item-definition subject while retaining the same live
Rotworm, selected inventory UUID and command; it incorrectly committed progress.
Green refuses that change, commits the original use, consumes the selected stock
once, then replays the saved receipt after the Creature has been removed.

The case uses the real runtime, lease, Creature, Root transaction and active SQL
custody guards. Historical fixture stock is an empty carried bag; no quest state
or quest receipt is seeded by SQL. Its private track uses a plain fixture effect
so it cannot register or weaken a production bound-operation owner.

This qualifies the reusable live-target boundary, not perfume production.
The existing Rotworm callback consumes item17673 and grants17674; their current
materialized records have known weight350 but unknown stack semantics and remain
non-materializable. Those profiles and the full perfume-stage wiring are separate
unfinished work. Evidence: the two canonical Item records under
`content/items/definitions/items-06500-06999.json` and
`content/quests/npc-callbacks/m13-rotworm-sample.json`.
