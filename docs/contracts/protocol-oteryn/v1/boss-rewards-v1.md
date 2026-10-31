# Personal boss rewards v1

Capability 224 `BOSS_REWARDS_V1` requires Item definition-index capability 4 and existing boss capability 117. It is
offered only by the bundle-world capability set. Commands 428–430 use the
existing admitted command envelope; their bodies carry no Character, session,
lease, world, scope or arbitrary Item destination. The existing server owners
derive every authority and the receiving backpack from the current player.

## Discovering reward groups

Existing command 340 keeps its empty first-page request and its existing protobuf
reply. It lists at most 64 own, live, nonempty groups in expiry/death order. Expiry
is always mint plus seven days, so this remains oldest-first order.

When capability 224 is offered and selected, a full page can be continued by
encoding its last death id as protobuf field 1 (`0x0a 0x10`, then 16 nonzero bytes,
18 bytes total). A shorter page is currently exhausted. The server resolves that
cursor only within the admitted Character and World; an unknown or foreign
cursor is refused. Empty or expired historical groups may still anchor the
cursor, but never appear as live rewards. Duplicate fields, nonminimal encodings,
zero ids and trailing bytes are malformed. Clients without capability 224 retain
the unchanged empty-request behavior. No additional command number is used.

Pages are live keyset reads, not an historical snapshot. Refreshing the first
page observes rewards committed while another page was being read.

## Request

All integers below are unsigned little-endian; all requests begin with byte
`1`. Exact lengths are required. Unknown versions, zero required death ids,
invalid ordinals, truncation and trailing bytes are rejected.

| Command | Body after version | Total bytes |
| --- | --- | --- |
| 428 BOSS_REWARD_BROWSE | death id (16 bytes), after ordinal (u16, 0..1000) | 19 |
| 429 BOSS_REWARD_TAKE | death id (16 bytes), ordinal (u16, 1..1000) | 19 |
| 430 BOSS_REWARD_COLLECT_ALL | death id (16 bytes); all zero selects all own groups | 17 |

Browse returns up to 64 remaining rows after the cursor. The death identity is
the immutable server death occurrence, never a fresh loot roll seed supplied by
the player. The player may select only their own already-frozen entitlement.

## Reply

Every reply begins with version byte `1` and a status byte:

| Value | Status | Meaning |
| --- | --- | --- |
| 0 | Ok | Requested result committed or read successfully |
| 1 | Empty | No remaining live entitlement |
| 2 | Expired | Seven-day expiry reached |
| 3 | NoRoom | Capacity prevented delivery; this request moved no entitlement |
| 4 | OutOfReach | No current qualified reward chest interaction |
| 5 | Busy | Owner cannot accept the request now |
| 6 | Rejected | Identity, state, scope, replay intent or authority refused |
| 7 | Unavailable | Durable owner unavailable; no success is asserted |
| 8 | ContentUnavailable | Pinned Item definition/policy cannot be resolved |

A browse reply then contains death id (16 bytes), expiry in Unix milliseconds
(u64), row count (u8), each row `{ordinal:u16, definition_reference:u32,
quantity:u32}`, and `more` (byte 0 or 1). Rows have strictly increasing ordinals,
nonzero definition references and quantities. At most 64 rows are accepted and
the total result limit is 1024 bytes. Take and collect-all replies instead
contain moved entitlement count (u16) and remaining entitlement count (u32),
for 8 total bytes including version/status. Unknown statuses and trailing bytes
are malformed. A partial collect returns `Ok`, the actual committed prefix in
`moved`, and the undelivered entitlement count in `remaining`. Capacity refusal
returns `NoRoom` only when the request moved nothing. An entitlement is counted
as moved only after its Item custody and delivery receipt committed.

## Ownership and replay

The native combat owner captures damage dealt, damage received and non-self
healing and freezes the full participation census at the single committed death.
The existing boss owner stores that census, eligible private chests, frozen loot
and exact seven-day expiry in one transaction. Existing anti-leech eligibility
is preserved. Collection uses the existing reward mint/custody owner and its
typed op4 audit, with exact boss death and ordinal provenance. It never mints a
second corpse loot copy and never re-rolls on retry.

The physical reward chest is qualified by the native content/interaction owner.
Current recovery, node incarnation, assignment generation, Character lease and
admitted session fences are checked before a cached command result is returned.
Reusing a command identity with a different request is refused. Each immutable
entitlement is claimable once; no caller-controlled Item destination is accepted.
Channel and Instance identities retain their distinct typed namespaces.

## Source and deliberate extension

Primary source is Crystal `201189f871d23597b82caf88fd611454a2ddefb8`:
`data/scripts/systems/reward_chest.lua:39-108,141-171` (participation and personal
loot), `data/libs/functions/monster.lua:207-224` (top-level personal reward rolls),
`src/game/game.cpp:3502-3542,12069-12120` (private reward access/collection), and
`src/io/iologindata_save_player.cpp:625` (seven-day expiry). The bounded Oteryn
wire body, durable command receipts and current authority fences are native
integration requirements. The accepted Oteryn damage-and-fair-share anti-leech
rule remains authoritative; it is documented separately from donor loot factors.

The native client feature registration records unsupported presentation. This
contract exposes the production server dispatch path and does not claim a native
client screen is already implemented.
