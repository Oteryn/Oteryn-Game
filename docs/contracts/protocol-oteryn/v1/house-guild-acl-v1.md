# House guild access lists

Capability 515 requires House capability 49. Command 6252 replaces the guild selectors for one House guest or subowner list under the current ACL revision.

## Request

The protobuf envelope uses field 8 for the House target (field 1 World UUID and field 2 House key), and field 4 for the replacement message.

| Replacement field | Type | Meaning |
|---|---|---|
| 1 | varint | Guest (1) or subowner (2) |
| 2 | varint | Expected House ACL revision |
| 3 | repeated UTF-8 string | Up to 100 selectors, each up to 100 bytes |

`@Guild` grants the selected role to current members of that guild in the House's World. `Rank@Guild` selects a current named rank. The server checks live guild membership at access time. The House owner can edit either list. A subowner can edit the guest list. An empty replacement clears the guild selectors for the selected list. Character entries remain independently editable through command 56.

## Result

The existing House result envelope carries the disposition in field 1 and the committed ACL revision in field 2. A stale revision returns Conflict. Invalid selectors and denied edits return Rejected.
