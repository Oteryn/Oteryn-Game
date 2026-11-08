# House podium display V1

CLIENTGAP-SYSTEMS L180. Capability 449 requires 150; command 733 is independently negotiated. Existing 150 bytes and housing mode restrictions are unchanged.

## request

One byte operation: 0 query followed by exactly one 16-byte podium item UUID; 1 set or 2 preview followed by one visibility bit field and the exact legacy SetRequest. Operations 3/4/5 are equivalent Query/Set/Preview using a connection-local nonzero u64 BE handle (after the flag byte for 4/5), with the legacy SetRequest item field required to be all zero. Only the current SessionItemView can resolve that handle to a durable Instance; map-base/entity/stale handles are refused, and actual UUIDs are not returned to the client. Bit 0 shows the display and bit 1 shows the platform; other bits are rejected. Set preserves the same session, holder, named-house, item, catalogue, ownership and revision fences. Preview validates those same current facts but performs no write.

## reply

Refusal: 0 followed by the existing podium refusal ordinal. State: 1, committed boolean, podium kind, visibility bits, appearance namespace (0 unavailable, 1 outfit, 2 object), platform appearance u32 BE, display appearance u32 BE, exact placed item definition key (one-byte length, 1–160 UTF-8 bytes), revision (one-byte length, 1–128 UTF-8 bytes), and the exact legacy optional Projection. Appearance ids come from the active immutable catalogue. Unavailable metadata is zero and is presented as unavailable, never a fabricated skin. Old stored rows default both visibility flags to true. Outfit preview uses the catalogue male base variant because legacy podium state has no sex/color variant.

Every request/reply is bounded by 512 bytes. No world/character/session id is trusted from the payload; the exact admitted owner supplies those bindings.
