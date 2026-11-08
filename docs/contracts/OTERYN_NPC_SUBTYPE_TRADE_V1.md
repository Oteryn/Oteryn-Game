# NPC-TRADE-3: native subtype trade owner

SRV9B establishes the server-side extension of NPC-TRADE-1/2. Command 8 and
command 97 continue to bind a server catalogue offer index and catalogue revision.
No capability, command, message or client-side subtype authority is introduced.

Item owns the durable value. Gold payment, bank debit, mint, burn, custody,
outbox and replay use the existing single physical NPC trade transaction under
the complete current Character item fence. A refusal rolls the transaction back.
Replay precedes live conversation and session checks and cannot mint again.

The admitted offer's `count` is always native transaction units; retained source
facts keep their original fields. Source fluid `count`/`subType` is
projected to `sub_type`; quantity is one nonstackable container per purchase.
The admitted fluid container identities are 2874,2875,2877,2880,2901,5552,9232.
The source enum 0..20 is the existing I10a Item subtype namespace: empty, water,
wine, beer, mud, blood, slime, oil, urine, milk, mana, life, lemonade, rum,
fruit juice, coconut milk, mead, tea, ink, candy, chocolate.

The immutable trade stores and hashes the selected subtype. Its audit payload
adds optional fluid subtype tag19 and full native charges tag20. Old requests
without subtype preserve their binding and audit encoding. Fluid mint writes
`game_item_fluid_states` in the same transaction, proven by the fresh trade,
mint line and Item identity. Empty means no row. Moves preserve the Item identity
and its existing state. Sales and SELL_ALL select only matching fluid state;
ordinary default sales cannot burn filled containers. Retired fluid instances
retain their state as history, never transfer it to proceeds or another item.
The deferred database guard verifies every bought/sold Item line's subtype.
A filled-fluid mint counts one additional effect work unit within the existing
trade ceiling; exceeding that ceiling refuses the entire plan, without relaxing
any existing resource or audit-byte limit.

Stackable rune counts remain piece quantities. Charge subtypes are admitted
only for nonstackable items and when exactly equal to the pinned Item
definition's native full charges (including an implicit source default). The native missing-timed-row representation
already means this full default (as in Hireling purchases). No alternative
charge store or charge-as-quantity encoding is introduced. Existing nondefault
timed-state sale prohibitions remain. Unknown/partial-charge variants stay held.
The 57 exercise-wrap holds are exactly 19 copies of the three proven Crystal
tuples, with native full charges500/1800/14400. Their prices may exceed the
three-stack sale-proceeds ceiling because they are purchases using the existing
coin/bank payment owner, not gold payouts.

Source facts, exact pin/line evidence, tests and the separate unresolved Lynda
product question are recorded in `content/npcs/S10-AUDIT.md`. Quest access
callbacks and parity-pending price disputes remain held.
