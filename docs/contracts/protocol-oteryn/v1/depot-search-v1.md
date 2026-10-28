# Depot search v1

Command `182`, gated by capability `153` (requiring depot `37` and stash `62`),
queries the admitted Character's depot boxes, Supply Stash and CharacterInbox.
It extends the custody owners in migrations 0151 and 0189; migration 0335 adds
read indexes only. Counts span towns and definition revisions in the Character's
World. No Channel filter applies. Search requires current native content, live
player control, the current item/session fence and a reachable authored locker.
A read waiting in PostgreSQL rechecks the session fence before publishing.

## Request

Bounded protobuf-compatible payload (at most 24 bytes): varint field 1 is action
(1 search, 2 detail, 3 retrieve); varint field 2 is a nonzero native source item id
(`oteryn:item.tibia.i<id>`); optional varint field 3 is retrieve-all (0 or 1).
Search and detail require field 3 to be absent. Unknown and duplicate fields are
rejected. Search and detail currently return the same aggregate.

## Reply

At most 8192 bytes. Varint field 1 is outcome (1 found, 2 retrieved, 3 not found,
4 no room, 5 rejected); optional varint field 2 is retrieved quantity. Repeated
message field 3 contains at most 64 result rows: varint fields 1 through 4 are
source item id, depot quantity, stash quantity and inbox quantity. Repeated
message field 5 contains tier (varint 1, 0..10) and quantity (varint 2, nonzero),
strictly ascending by tier. Tier zero includes untiered items and stash balances.
Counts cover live items only; item-instance identities never cross this wire.

## Staged retrieval

The retrieve action is reserved and currently returns rejected without mutation.
It does not close retrieval parity. Native depot withdrawal and chosen-count
stash withdrawal continue through their existing fenced owners and immutable
receipts. Atomic retrieval across depot, stash and inbox requires an accepted
multi-custody writer and database proofs; a sequence of independent withdrawals
cannot satisfy all-or-nothing slot/weight admission. Capability 153's client
feature remains explicitly unsupported. No Crystal/Canary wire identity or limit
is asserted by this Oteryn-owned codec.
