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

## Retrieval

For an item represented by a Supply Stash balance, retrieve selects one unit or,
when retrieve-all is set, that deterministic definition revision's whole balance.
The depot-search route resolves only the server-issued row. The existing stash
owner revalidates and locks that row, checks the current session/item fence,
backpack slots and carrying capacity, and records its immutable receipt before
returning `retrieved`. A missing balance returns `not found`; backpack, slot and
weight refusals return `no room` without a partial mutation.

Depot and CharacterInbox result retrieval remains rejected. It requires an
accepted multi-custody writer and database proofs; a sequence of independent
withdrawals cannot satisfy all-or-nothing retrieve-all admission. Capability
153's client feature remains explicitly unsupported. No Crystal/Canary wire
identity or limit is asserted by this Oteryn-owned codec.
