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

## Runtime actor adapter

`read_for_actor` accepts the existing sealed Character item authority, issued
with the Character lock before the Ground owner lock. It checks that authority
against the acting fence, including Character, session, connection, lease and
scope generation. It also checks the actual runtime generation, the selected
actor's bound lease and the actor's current position. Reach is one native tile
in each horizontal axis, on the same signed native floor. The adapter reuses
the same physical transaction; it does not acquire a new Character lock after
Ground or issue a mutation permit. The runtime guard must remain held by the
calling quest participant.

The PG case includes synthetic runtime pins to isolate these runtime checks;
the corpse itself still comes from the real Root MINT and race-provenance path.
The actor adapter first failed by admitting a distant actor (PG run 5.32 s).
After implementation the expanded case passed in 7.404706899 s, including
adjacent diagonal use, distant/other-floor refusal, a foreign player actor,
another runtime generation and Character/session/connection/lease mismatches.
The tool transformation and personal-progress participant remain to be wired.

## Atomic quest participant

Reward callbacks can now explicitly declare `fresh_corpse: true` with an exact
`creature_target` and one carried input. The live constructor refuses that mode;
the corpse constructor requires an existing lane permit and a lifetime-bound
runtime actor snapshot. The ordinary Root quest reward transaction rereads the
selected top corpse using its current Character authority and the active native
Creature revision before consuming the tool or creating a reward. First-form
state and native expiry supply freshness; no quest-specific timeout is guessed.
The snapshot retains its runtime borrow through the asynchronous commit without
passing an unrelated detached position or a runtime reference to another owner.

`engnpc2_corpse_sample_postgres_atomic_burn_mint_progress_and_replay` executed on
2026-11-09 in 53.552703872 s. A genuine corpse MINT precedes the tested command;
changed tool intent refuses without a quest receipt/revision. Correct intent
burns the selected carried fixture input, mints one qualified native output and
advances private progress in the same transaction. Releasing the runtime then
replaying returns the existing receipt without another MINT. This is an engine
recipe fixture, not the canonical Slug stage. Client map-handle routing and the
canonical perfume input/output data still need wiring.

The callback mode/binding test was red before the field existed. All 27 reward
callback tests pass after the addition. This check also caught the stale Ice
Islands golden from the earlier seal/counter wiring: HEAD and the working code
produced the same current catalogue hash, which was used to refresh that golden.
The native Rotworm exchange and actor-bound corpse-reader regressions also ran
against the new binary (50.33251178 s and the duration in the execution log).
