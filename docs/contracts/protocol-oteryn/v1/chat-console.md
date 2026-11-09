# Console sender metadata

Command 736 belongs to existing capability 7, CHAT_V1. It is a read-only
lookup of current durable sender levels in the admitted World, not a claim
about a character's historical level when a retained message was sent.
The client labels these values as current. Existing chat codecs are unchanged.

## Request

One u8 count (0..32), then each name as a u8 UTF-8 byte length (1..120)
and those bytes. NUL and trailing bytes are rejected.

## Reply

One u8 count equal to the request count, then one little-endian u32 per
requested name, in request order. Zero means unavailable; it must never be
rendered as a level. Replies have at most 129 bytes. Names are resolved
through the admitted World's character roots and durable progression owner,
under the requester's recovery fence. No cross-World lookup occurs.
Rejected commands have an empty payload. Without capability 7 selected the
command is refused. Older servers may reject the query; the client retains
chat and omits unavailable levels.
