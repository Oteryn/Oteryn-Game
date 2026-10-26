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
