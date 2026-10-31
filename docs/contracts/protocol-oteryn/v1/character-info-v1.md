# Character info v1 (CHARINFO)

## Request

Command 113 is reserved for the server-side character information family. Every query is scoped
to the controlled actor, or to a character allowed by the native Friends visibility owner. History
requests contain an unsigned 16-bit offset and a page size from 1 through 50. Unlock and select
operations are writes and must pass the current Character session-generation fence.

Requests have a one-byte operation tag. Character IDs are 16 bytes; page offsets are
little-endian `u16`, limits and title IDs are `u8`. The registered maximum is 20 bytes.

| Tag | Operation | Bytes after tag | Total bytes |
| --- | --- | --- | ---: |
| 0 | QuerySelf | offset, limit | 4 |
| 1 | QueryVisible | character ID, offset, limit | 20 |
| 2 | Unlock | title ID | 2 |
| 3 | Select | title ID (0 clears) | 2 |
| 4 | LookVisible | character ID | 17 |

## Result

Replies are bounded to 32 KiB. They may contain base/progression statistics, native death and PvP
history from the preceding 30 days, item counts grouped by inventory/depot/stash/inboxes, appearance
and achievement summary counters, earned titles, and the currently qualified display title. The
query reads those native owners and does not persist derived aggregate data.

Permanent titles remain earned. Conditional titles are displayable only while their predicate is
true. Rotation titles carry the next-rotation expiry. A read or look requalifies the selected title;
the fenced durable owner clears a selection that is no longer qualified.

The stable Game title catalogue appends Pensive Wanderer as ID 5 (MONKQUEST3,
owner decision 2026-10-30). IDs 1–4 retain their meanings. Honouring the Shrine of
Eternity earns this permanent title in the same fenced quest transaction; the
Character title owner supplies its ordinary query, unlock and selection behavior.
The existing `u8` title fields and capability 97 are unchanged.

### Catalogue extension and automatic unlock (PAR-P12a)

The catalogue appends IDs 6–43 (`CharacterTitle::ALL` in
`crates/protocol-oteryn/src/character_info.rs`); IDs 1–5 keep their meanings and
migration 0572 widens both title checks to 1–43. Names, female names,
descriptions and permanence follow Global (TibiaWiki *Cyclopedia*, revision
1197515). Each admitted title has one rule over a native owner: bank balance,
level, rank 1 of the current World highscore snapshot, daily reward streak,
guild leadership, Bosstiary points or unlocked bosses per category, the kill
count of a named creature, or a ten-year-old Character. The 69 Global titles
whose predicate has no native owner yet are listed with their reason in
`NOT_ADMITTED_TITLES` and are never granted.

The fenced durable owner grants every newly qualified title, clears a selection
that no longer qualifies and revokes the failed conditional titles in one
transaction. It runs at login admission, on the actor cadence after a kill
settlement that changed the level, a tracked creature or a boss, and before the
self query. The wire format, the `u8` title fields and capability 97 are
unchanged.
