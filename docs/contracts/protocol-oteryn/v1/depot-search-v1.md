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
(1 search, 2 detail, 3 retrieve, 4 open/list); varint field 2 is a nonzero native source item id
(`oteryn:item.tibia.i<id>`); optional varint field 3 is retrieve-all (0 or 1).
Search and detail require field 3 to be absent. Detail accepts varint field 4
(tier 0..10, default 0); other actions reject field 4. Detail returns depot,
Inbox and stash counts for that tier only, matching Crystal's detail selection.
Search retains its aggregate across tiers. Unknown and duplicate fields are rejected.

Open omits field 2 (or sends zero) to request the first page. Its field 2 may
instead name the last item id of the preceding page. The list includes every
live native item id in depot chests, nested containers, Inbox and Supply Stash,
combining stack quantities and retaining tier counters. Pages contain at most
64 ids in increasing order, with no truncation of an item's tier counters.
Outcome `more` requires continuing after the final returned id until `listed`;
an empty list returns `listed` with no rows. Each page is one current snapshot,
not a retained snapshot across concurrent changes. This uses the existing wire
budget rather than introducing a second command or changing capability offers.

## Reply

At most 65536 bytes (the existing command-result payload ceiling). Varint field 1 is outcome (1 found, 2 retrieved, 3 not found,
4 no room, 5 rejected, 6 listed, 7 more, 8 detailed); optional varint field 2 is retrieved quantity. Repeated
message field 3 contains at most 64 result rows: varint fields 1 through 4 are
source item id, depot quantity, stash quantity and inbox quantity. Repeated
message field 5 contains tier (varint 1, 0..10) and quantity (varint 2, nonzero except an empty detail selection),
strictly ascending by tier. Tier zero includes untiered items and stash balances.
Counts cover live items only; item-instance identities never cross this wire.
`detailed` contains one row for the selected id and tier; an absent selection
returns that row with all counts zero, as Crystal does. Open, Search and Detail share the same live depot access
and session-generation disclosure checks. They do not mutate durable value;
repeating a read observes current holdings without a second effect.

## Retrieval

For an item represented by a Supply Stash balance, retrieve selects one unit or,
when retrieve-all is set, that deterministic definition revision's whole balance.
The depot-search route resolves only the server-issued row. The existing stash
owner revalidates and locks that row, checks the current session/item fence,
backpack slots and carrying capacity, and records its immutable receipt before
returning `retrieved`. A missing balance returns `not found`; backpack, slot and
weight refusals return `no room` without a partial mutation.

Depot and CharacterInbox retrieval delegates to the existing item-loop batch
owner, including nested items and its immutable receipt. Capability 153's client
feature remains explicitly unsupported. No Crystal/Canary wire identity or limit
is asserted by this Oteryn-owned codec.
