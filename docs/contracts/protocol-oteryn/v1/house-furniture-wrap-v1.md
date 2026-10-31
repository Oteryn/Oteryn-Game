# House furniture wrap V1

HOUSE-ITEM-WIRE-1: capability 207 requires House V1 (49). Command 402 is an
explicit wrap/unwrap operation, modelled on client packet 0x8B. It is offered
only in bundle Worlds, with an active gameplay generation. Ordinary USE is
unchanged. The native client input is currently unsupported.

## Request

Exactly 45 bytes: action (1 wrap, 2 unwrap), item UUID (16 bytes), expected
Item state revision (u64 big endian), expected house ACL revision (u64 big
endian), then x, y, floor (each i32 big endian, runtime cell coordinates).
The session supplies the character, named House, connection generation,
lease generation and scope generation; clients cannot select those authorities.

The physical House owner proves current custody. The durable writer checks
node incarnation and the current named-House session, property lock and exactly
one OWNER/SUBOWNER edit grant. An ACL revision mismatch refuses. Unwrap requires
a walkable, unoccupied tile belonging to that House. The same Item UUID,
mint lineage and quantity survive conversion; its revision advances once.
House placement/reclaim provenance and main backpack custody change atomically.
A receipt keyed by GameSessionId/CommandId binds the complete request and
replays its committed revision, including after owner reconstruction.

## Result

CommandResult is Accepted with byte 0 followed by the committed revision
(u64 big endian). Refusals are Rejected with one byte: 1 outside a house,
2 no edit right, 3 not wrapable, 4 stale generation/revision/ACL or conflicting
replay, 5 blocked/occupied/outside-house unwrap tile, 6 inventory full.
Unknown storage outcomes disconnect; they never report a successful change.
Committed commands refresh the selected inventory/container views.

## Content evidence and UNKNOWN

No Crystal/Canary `playerWrapableItem` implementation exists in the in-repo
reference sources searched for this slice. Deterministic conversion rules
come from `gameplay_transport/house_furnishing.rs`. The admitted pair is oven
`oteryn:item.tibia.i6355` ↔ oven kit `oteryn:item.tibia.i6371`:
`imports/canary/items-xml/items.xml:17549-17552` explicitly has
`wrapableto=6371`; repository item 6371's description says to use it in a
house to construct an oven. The active content must contain both definitions.
Only this single-cell oven variant is admitted, so unwrap preserves the
original furniture identity without guessing an orientation or kit recipe.
Unwrap requires a prior wrap receipt for that Item UUID; freshly minted kits
without that lineage remain UNKNOWN until their construction rules are qualified.
All other furniture, oven orientations, multipart furnishings, store-bound
furnishings and kit pairs remain UNKNOWN here. Contents, timed states,
imbuements and written text refuse conversion. This does not alter the
outdated Tibia manual statement or claim general furniture parity.

Migration 0595 keeps immutable before/after and custody evidence in the House
owner. Its SECURITY DEFINER entry is the only receipt writer; runtime/control
have read access, and only runtime can execute the fenced transition. Existing
item/custody guard chains receive narrow same-physical-transaction proof arms.
