# House guild access lists

Capability 515 requires House capability 49. Command 6252 replaces the selectors or ordered text rules for one House guest or subowner list under the current ACL revision.

## Request

The protobuf envelope uses field 8 for the House target (field 1 World UUID and field 2 House key), and field 4 for the replacement message.

| Replacement field | Type | Meaning |
|---|---|---|
| 1 | varint | Guest (1) or subowner (2) |
| 2 | varint | Expected House ACL revision |
| 3 | repeated UTF-8 string | Up to 100 lines and 1,999 characters including a newline between lines, within the existing command payload bound |
| 4 | optional bool | Send true to use ordered rules |

Replacement lines exclude NUL. Ordered lines also exclude carriage return and line feed, including inside comments. Without the ordered flag, the existing `@Guild` and exact `Rank@Guild` grammar and 100-byte per-selector bound apply. Stored selector lists that predate the 1,999-character write limit remain readable and in effect until replaced.

In ordered mode, each line can match a full character name case-insensitively. `*` matches zero or more characters and `?` matches one character. A leading `!` denies a match. Blank lines and lines beginning with `#` are ignored. The first matching line decides access to that list. Tibia Global documents this order as “it reads the list as it is formatted, top-to-bottom.” [TibiaWiki House list syntax](https://tibia.fandom.com/wiki/House#List_Syntax)

`*@Guild` and the existing `@Guild` form match current members of that guild in the House's World. `Rank@Guild` selects a current named rank. The server resolves it to the rank identity when the rule is saved. A rename preserves access and deleting the rank drops it. The server checks live guild membership at access time. The House owner can edit either list. A subowner can edit the guest list. An empty replacement clears the ordered rules for the selected list. Character UUID entries remain independently editable through command 56 and grant access when no text rule decides the list.

## Result

The existing House result envelope carries the disposition in field 1 and the committed ACL revision in field 2. A stale revision returns Conflict. Invalid rules and denied edits return Rejected.

### Door lists

A session that selects capability 515 can send the optional ordered text rule message
in field 4 of the existing command 56 door action. Set field 2 of that message to true to use ordered rules. It uses the same syntax and limits
within the command 56 payload bound. The server replaces the character entries and text rules in one
ACL revision. An omitted message preserves the rules. An empty message clears them.
Only the House owner may edit a door list. The House owner and characters granted access to
the specific door can open and close it, subject to the door's existing interaction checks.


### Reading rules

A selected capability 515 session can send true in command 57 field 6 with a list selector in field 5. The result adds field 6 with the saved lines and their grammar alongside the ACL revision and UUID entries. An omitted flag keeps the UUID-only result. A pre-upgrade list is rendered using its qualified guild and rank identities. A stale identity is shown as an inert comment.
