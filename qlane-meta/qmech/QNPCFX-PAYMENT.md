# QNPCFX — Koshei payment binding

The existing QENGINE reward callback owner already composes a bank/carried fee,
physical item inputs, quest state and an authoritative timestamp in one fenced
Type-2 transaction. Reuse it; do not introduce a second wallet, mint, transaction
or payment table in the source NPC executor.

`content/quests/npc-callbacks/qnpcfx-koshei.json` binds the source step at A Sweaty
Cyclops (retained Lua 283–338). Four exact amulet pieces and 5,000 gold start the
24-hour work. A later `amulet` conversation grants item 7532 once. This assembly
is not final quest completion: Koshei's fight remains a separate stage.

The source stores a finish timestamp (`now + 86400`). The binding stores the
transaction start timestamp and uses the existing `ElapsedAtLeast(86400)`
condition for collection; those deadlines are equivalent. A separate phase
track distinguishes not started, working and collected. Collection preserves
the timestamp (`Add(0)`); repeat commands cannot restart the timer or mint again.

The original script charges before checking all pieces. The owner's required
atomic-exchange semantics take precedence: incomplete inputs, insufficient funds
and unavailable output capacity must leave payment and progress unchanged.
Existing bank policy, carried coin/change custody and replay authority remain
with the fee owner. No outfit/addon path or migration is introduced.
