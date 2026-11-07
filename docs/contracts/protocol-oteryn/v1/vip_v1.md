# VIP_V1 basic server wire (owner 2026-10-23)

Capability 43 is optional on bundle Worlds. Commands 48–50 use the existing ordered
command envelope and FND-02 protobuf binary. The canonical IDL is `vip_v1.proto`.
No CharacterId, AccountId, invitation_id, ChannelId or presence is exposed. VIP-2 owns
presence notifications. The basic surface uses query responses without allocating a
state domain or implying an online/offline state.

The operation field (1) is required and nonzero. The table specifies which other
fields are admitted; unrelated fields, duplicate singular fields, unknown operations,
malformed UTF-8, unknown fields and overlong payloads fail closed. Metadata bounds
are checked before durable mutation. Group numbers use repeated unpacked varints,
at most eight. Zero icon/notify and empty groups have their ordinary default values.

| Command | Operation | Fields |
|---|---|---|
| 48 | 1 add | text: character name, native naming grammar |
| 48 | 2 remove | number: entry |
| 48 | 3 edit | number: entry; text: description (512 bytes/128 characters); icon 0–10; notify; distinct groups 1–8 |
| 48 | 4 group add | text: name (100 bytes/25 characters) |
| 48 | 5 group rename | number: group; text: name |
| 48 | 6 group remove | number: group |
| 49 | 7 accept, 8 reject, 9 revoke | number: session-bound handle |
| 49 | 10 withdraw | number: entry |
| 49 | 11 status setting | number: 1 shown / 2 hidden |
| 50 | 12 query | optional number: after-entry cursor (absent means 0) |

Request bound: 640 bytes. Result bound: 65536 bytes, including protobuf overhead.
Result code 1 OK, 2 UNKNOWN_NAME, 3 ALREADY_LISTED, 4 LIST_FULL, 5 GROUP_LIMIT,
6 NAME_TAKEN, 7 NOT_FOUND, 8 EXHAUSTED (wait 1–3), 9 VIP_UNAVAILABLE, 10 REJECTED.
Malformed requests return code 10; transient or ambiguous database failure closes
before command sequencing, permitting retry against the immutable durable receipt.
Writes carry no view. Semantic identity is the canonical decoded request, never its
serialized protobuf bytes; field reordering does not change command equality.

Query view fields: `next_entry` (0 at end, otherwise the next request cursor),
`entries` (up to 25 per page; entry, name, description, icon, notify, groups,
confirmed), custom `groups` (group, name), target-side `consents` (handle, inviter
name, accepted, expires in Unix seconds), `shown`, `consent_revision`. Fixed groups
1 Enemies, 2 Friends, 3 Trading Partners need no stored row. `confirmed` is true
only for the querying character's accepted invitation; it reveals no presence.
Expired pending invitations are absent. A handle belongs to one exact invitation
and target GameSession; stale handles cannot select a replacement invitation.
Other sessions fetch committed changes with query 50; automatic relay/delta delivery
is deferred with VIP-2.

Entries and groups are Account + World; consent and the status setting belong to a
Character. VIP writes never advance CharacterRevision. Session-generation, lease,
scope, node incarnation and recovery fences are checked in the semantic transaction.
Receipts bind the acting Character and canonical request, including the immutable
session handle. Retained receipts replay before current session/Premium/rate checks,
with current recovery still required. Handle mappings and receipts are retained
without expiry in this basic implementation, covering the full command replay window.
The existing PARTY block writer ends consent in both directions atomically through
an installed trigger. Removing a block never restores an old invitation or consent.
