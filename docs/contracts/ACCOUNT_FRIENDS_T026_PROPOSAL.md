# T026 Game-native account Friends contract

Account Friends is an explicit Account + World relationship, separate from
character VIP consent. It does not change Platform identity or commercial data.
The existing VIP operations and character visibility grants remain unchanged.

Commands reuse capability 43 and the existing bounded VIP envelope. New operations:

| Command | Operation | Input |
|---|---|---|
| 48 | 13 select main | No fields; use the authenticated acting character |
| 48 | 14 invite account | Existing character name in this World |
| 49 | 15 accept account invitation | Session-bound friend handle |
| 48 | 16 withdraw / revoke | Session-bound friend handle |
| 50 | 17 query account Friends | Optional after-handle cursor |
| 49 | 18 reject account invitation | Session-bound friend handle |

The receiver must explicitly accept; sending a reciprocal invitation does not
accept a pending relationship. An account cannot accept its own outgoing request.
Only either participant can revoke. Rejected or revoked relationships disappear;
old session handles cannot select a replacement invitation.

Query views contain `account_friends` rows (`handle`, `name`, `state`,
`main_character`), the owner's selected `main_character`, and `next_friend`.
Pages contain at most 25 rows. There are at most 200 participating relationships;
invites fail conservatively when the combined pair reaches that bound. Main names
are returned only for accepted account friendships, and only while the selected
character still belongs to the same Account and World and remains active. No main
character is inferred when none was chosen. No AccountId or raw relation id is
exposed. Revocation removes disclosure on the next query.

Writes reprove session generation, lease, scope, node and recovery fences; current
ownership is read from the fenced actor, never from prepared account metadata.
Participant account guards are locked in stable order. New account writes use a
20-token bucket refilling one token per three seconds and durable immutable command
receipts. Replay returns the original result without repeating an edit. Query
handles are tied to the actor and GameSession and allocated monotonically.

The native client opens this independent consent surface with Ctrl+Alt+F. Selecting
main uses the current character; selecting another owned character requires entering
that character first. Successful changes automatically refresh the authoritative
account Friends query. No provider keys or external account mutation is involved.
