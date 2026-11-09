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


Perfume custody follow-up
------------------------
The registered Rotworm callback exchanges canonical Items 17673 and 17674.
Their physical admission uses the existing source importer, exact Crystal
identity bindings and accepted-first semantic reconciliation. Pinned evidence:
https://github.com/opentibiabr/canary/blob/47dfd51f45280a59a1d3e50ba7edd573d7234446/data/items/items.xml
(retained locally as `imports/canary/items-xml/items.xml`, SHA-256 checked by the
importer), plus the retained client appearance bank
`appearances-2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2.dat`.
Both qualify as portable, nonstackable Items weighing 350; no fluid subtype,
charge, timer or custom use effect is inferred. The production callback remains
the live-use authority. The focused PG case is
`engnpc2_rotworm_perfume_postgres_production_exchange`; the earlier empty-bag
fixture remains a separate intent-admission regression. This personal receipt
does not complete the four-odour quest stage.

Local PostgreSQL execution on 2026-11-09: production exchange
`engnpc2_rotworm_perfume_postgres_production_exchange` passed in 43.266744682 s;
`engnpc2_live_creature_postgres_exact_intent_burn_and_replay` passed in
42.929173085 s. The production case retains registered bound state operations,
checks exact one-item output, same-transaction BURN/MINT/progress, then removes
the target and proves receipt replay does not remint.
