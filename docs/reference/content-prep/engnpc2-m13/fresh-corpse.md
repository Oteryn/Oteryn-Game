# Exact fresh-corpse read for quest tools

The Goblin Merchant perfume task calls for a recently dead Slug, not any Item
with the same corpse appearance. Public description:
https://www.tibiawiki.com.br/wiki/Task:Rafzan (perfume task, first ingredient).
This slice supplies the read primitive; it does not claim the Slug stage is wired.

`spell_item_transaction::corpse_target::read_current_fresh` reuses the existing
scope owner's physical transaction, tile ordering and Item row locks. It checks
the selected top UUID against the original `CORPSE_MATERIALIZATION` receipt and
`game_corpse_source_races`. Race key and definition revision must both match.
A changed Item definition/state, natural retirement deadline, or overdue first
native decay stage prevents admission. Age is measured using database time.
The nonzero age limit is a caller's server/source policy; this reader does not
invent a quest-specific numeric freshness threshold.

The returned Item definition is a read result, not a quest mutation permit. A
quest participant must retain its scope/runtime guard and the same transaction
through tool changes and progress. It must separately validate the actor range,
exact carried tool and active content generation.

The focused PG case mints a real provenance-bearing corpse through the existing
Root, then checks selection, race/revision, cell binding, retained row lock,
server age limit, and refusal to reuse authority in another physical transaction.
It creates no quest progress or consumption receipt. No migration is required.

Execution on 2026-11-09: the new PG case failed at the expected missing positive
admission in 5.09 s, then passed after implementation with
`PG_EXECUTED engnpc2_fresh_corpse_postgres_exact_provenance_and_clock duration=7.107905774s`.
