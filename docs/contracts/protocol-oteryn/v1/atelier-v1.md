# Gem Atelier v1

Commands 308 (`ATELIER_QUERY`), 309 (`ATELIER_SET_LOCK`) and 424 (`ATELIER_MUTATE`)
require selected capability 148 (`WHEEL_ALLOCATION_V1`). These commands retain its
existing production, Item-view and bundle-world offers. The separate balance query
is specified in [Atelier resources V1](atelier-resources-v1.md).
The native client opens Wheel and Atelier with Ctrl+W. Its controls send allocation, reset,
vessel, query, lock and paid mutation intents through capability 148. Displayed prices come
from the live Atelier query; operation receipts never replace the newer confirmed view.

The canonical codec is `crates/protocol-oteryn/src/atelier.rs`. Requests are at most
64 bytes; replies at most 16,384 bytes. Query remains an empty payload, and lock retains
its existing four fields. Paid mutations carry only intent:

| Field | Wire tag | Value |
| --- | --- | --- |
| occurrence | 1, bytes | Exactly 16 bytes; UUIDv7 immutable receipt identity |
| expected_revision | 2, varint | Expected Atelier revision |
| action | 3, varint | 0 reveal, 1 dismantle, 2 switch domain, 3 grade up |
| gem | 4, bytes | Exactly 16 bytes; present only for dismantle/switch domain |
| quality | 5, varint | 0 lesser, 1 regular, 2 greater; reveal only |
| kind | 6, varint | 0 basic, 1 supreme; grade up only |
| mod_id | 7, varint | Unsigned 16-bit catalogue modifier id; grade up only |

All fields required by an action must be present. Unknown or duplicate fields,
irrelevant fields, out-of-range values and malformed lengths are rejected.
The server derives prices, inventory and bank custody, native item revisions,
vocation-compatible random modifiers, random domain and fragment yield. The
client cannot choose a price, draw, resource balance or mint identity.

Mutations use the existing Character revision sequencer and the session-generation
fenced Atelier writer. Material burns/mints, coin and bank debits, Character revision,
Atelier state and the immutable occurrence receipt commit in one transaction.
The same occurrence and intent returns its recorded result without another debit;
binding an existing occurrence to a different revision or action is rejected.
Premium, promotion, level, lock, vessel, grade and resource refusals remain owned
by that writer. Missing native economic facts refuse a fresh paid action without
preventing reconciliation of an already committed occurrence.

A well-formed rule refusal returns an accepted command with `refused=true` and
the unchanged current Atelier projection. Malformed or stale admission input is
rejected; an unavailable or indeterminate write ends the connection so the same
occurrence can be replayed. The committed Wheel and Atelier projections are
reloaded before the stat consumer runs, and admission restores both after relog.
When a fresh durable read proves that admission has not initialized an Atelier
row, the query returns `refused=true`, revision zero and empty collections. This
lets an ineligible Character open the panel without ending play. It does not
create initial gems; initialization remains owned by the existing admission path.
Storage-read failure still returns unavailable. An existing empty Atelier row is
a valid normal projection and is distinct from an absent row.

## Paid resource scope

Reveal selects a matching live gem from the Character's carried inventory,
including nested containers. Supply Stash gems are not reveal inputs. Grade
upgrades consume matching carried fragments first, then debit only the missing
quantity from that Character's Supply Stash in the same World. Fragment kind and
native definition revision must match the server-owned catalogue. The server
selects these sources automatically; commands and client controls are unchanged.

The stash debit extends the existing Supply Stash balance and immutable ledger.
It does not withdraw a temporary item or create an Atelier fragment balance.
The stash entry, material custody evidence, gold fee, Character revision and
Atelier occurrence receipt commit in one transaction. A shortage or refused
transition leaves every quantity and revision unchanged. Replaying a committed
occurrence reads its recorded result without selecting or debiting resources
again, including after relog or when current native content is unavailable.

Paid actions select eligible carried coins recursively before using the bank
remainder. They do not debit Gold Pouch coins, even though the separate live
resource projection includes those coins in its carried-gold display. This is
the accepted native fee-source boundary in
`docs/architecture/reviews/OTERYN_GAME_CHARACTER_GOLD_FEE_BOUNDARY_DECISION_2026-09-30.md:104-110`
and `docs/architecture/reviews/OTERYN_GAME_BANK_FEE0_FEES_FROM_THE_BANK_DECISION_2026-09-30.md:168-173`.
The display is not payment authority. A stash-and-bank grade upgrade does not
require a backpack when it needs no physical output. Coin change retains the
existing main-backpack destination and capacity checks.

Dismantling applies the existing managed filter and category/unassigned map.
An allowed target and its descendants precede managed main-container fallback;
ordinary breadth-first insertion follows a missing, filtered or full target.
Outputs can top up compatible partial stacks before placing the remainder.
The native preference map is reused; separate loot and obtain profiles are not added.
Change and fragment outputs both refuse atomically when their required output
has no valid destination. Only items whose carried ancestry is owned by the
current Character in the current World qualify as carried payment sources;
disconnected, foreign, ordinary Inbox and depot items are excluded.

Crystal `201189f` `player_wheel.cpp:1224-1236,1548-1560` distinguishes reveal
(`useStash=false`) from fragment upgrades (`useStash=true`). The shared selection
rule is `player.cpp:5304-5356,5821-5844`: carried items, recursively, precede the
stash remainder. `game.cpp:2862-2969` supplies carried coin and bank payment.

Vessels use the existing Wheel allocation/save command 161 and Wheel receipts.
They are not a second Atelier storage owner.

Replies retain `refused` (tag 1), `revision` (tag 2) and repeated gems (tag 3).
Each gem retains id/quality/domain/locked/vessel (tags 1..5) and adds its catalogue
modifier ids as repeated unsigned 16-bit varints at tag 6. Repeated grade rows
at reply tag 4 carry kind (tag 1), modifier id (tag 2), and grade 0..3 (tag 3).
Missing new fields remain valid when decoding a legacy reply. The bounds are
225 gems and 140 modifier grades; the player can identify and grade the actual
revealed modifiers without supplying server-owned effect values.

Live query 308 supplies the full Fragment Workshop catalogue: 46 Basic
modifiers and 23 Supreme modifiers for the Character's base vocation. Each
compatible `(kind, mod_id)` has a grade row, including grade 0 (Grade I) when
the Character has never upgraded it. Owning or selecting a revealed gem with
that modifier is not a prerequisite for grade up. The existing server catalogue
defines these identities; the client does not infer a vocation or invent a list.

These zero-grade rows are a query projection only. Durable grade storage and
mutation, lock and replay receipts retain their sparse stored-grade projection;
the query does not insert grade-0 rows or rewrite historical receipt bytes.
This follows the accepted Gem Atelier decision at142-143 and270-273 and Crystal
`player_wheel.cpp:1489-1507,1514-1568`. It does not alter the slot-2 draw pool or
resolve the separate accepted pool conflict.

The existing 0148 modifier list uses the current query's grade identities,
combined with the selected gem's modifiers as a legacy sparse/empty-reply
fallback. Duplicate identities are removed without merging Basic and Supreme
modifiers that share a numeric id. A still-valid Workshop selection survives
changing the selected gem and a live refresh. The existing upgrade button sends
the same `{kind, mod_id}` intent; server eligibility, prices and grade limits
remain authoritative. No new control, wire tag, capability or grade store is added.

Live query replies also contain up to three quality quotes at tag 5 and six grade
quotes at tag 6. Quality fields (varints 1..6) are quality, reveal gold, domain-switch
gold, fragment kind, minimum fragments and maximum fragments. Grade fields
(varints 1..4) are kind, target grade 1..3, gold and fragments. Prices come from
the same server catalogue used by the durable writer; fragment ranges come from
the Crystal-priority runtime rule. Missing quotes in a legacy reply are accepted.
Clients display these costs before submitting a mutation; quotes are never
mutation inputs or authority to spend a different amount.

Successful mutation replies carry the recorded Atelier result, without live quote
fields. Vessel membership is a derived Wheel projection that is not stored in the
Atelier receipt. The client queries 308 after a mutation or receipt replay to show
current prices and current vessel membership; receipt revisions must not replace
a newer already-observed live projection.

## Shared mutation interval

Live resource balances use the separately negotiated
[Atelier resources V1](atelier-resources-v1.md) query (capability 272,
command 484). Its seven current balances are refreshed automatically in the
existing panel when selected. They do not extend this V1 reply or its mutation
receipts. The query changes no items, bank/stash balances, Character revision or
Atelier state and does not consume the mutation interval. Its carried-gold
number includes the native Inbox-rooted Gold Pouch for display only; that does
not widen the fee input set described above.

Sessions without capability 272 keep the original operation plus Wheel 161 and
Atelier 308 refresh sequence, with no command 484 or balance lines. A
well-formed refusal or malformed optional balance reply does not disable the
ordinary 148 controls; it leaves no current balance projection. Missing balances
are omitted rather than rendered as zero. The server still determines resource
eligibility and affordability for every fresh mutation.

Fresh Wheel saves, resets, vessel changes and Atelier locks, reveals, dismantles,
domain changes and grade upgrades share one 490 ms ExAction interval per native
session. The existing Channel owner uses its monotonic clock; the separate
200 ms world-action interval is unchanged. An exhausted attempt returns a normal
rule refusal without changing state, spending resources or extending the interval.
A fresh admitted attempt starts the interval even if a later gameplay rule refuses it.

The durable occurrence lock and immutable receipt lookup precede this admission
check. An exact replay returns its retained result during the interval, and an
internal Character revision retry keeps the first admission decision. Server-owned
Atelier initialization does not start an interval.

Read-only command 161 (action 0) and command 308 are exempt: together they provide
the native panel projection and remain usable immediately after a mutation. This is
an explicit native wire adaptation of Crystal `201189f` `game.cpp:11325-11396`,
whose single OpenWheel response includes the panel data. The 490 ms value is
`config.lua.dist:469` (500 ms) minus the source's 10 ms adjustment. Coordinator
[ANSWER 6544165094](https://github.com/Oteryn/Oteryn-Game/pull/2771#issuecomment-6544165094)
authorizes this read exemption.

## Reference

Crystal `201189f871d23597b82caf88fd611454a2ddefb8`:
`src/game/game.cpp:11366-11410` selects the actions;
`src/creatures/players/wheel/player_wheel.cpp:1218-1254` reveals,
`:1293-1350` destroys, `:1353-1371` rotates, `:1374-1381` locks and
`:1514-1601` upgrades grades. Native Oteryn command ids and receipts are
protocol-oteryn contracts, not Crystal packet compatibility.
