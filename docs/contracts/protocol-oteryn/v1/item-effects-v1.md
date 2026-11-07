# ITEM_EFFECTS_V1 — NX-5

Capability 350 requires 6. It enables only state domain 104: empty type-1 snapshot
at revision 0, and type-1 deltas containing the bounded `WorldPresentationBatchV1`
codec's `MagicEffect` subset. No commands. Any sound, ambient sound, missile,
spell words, value text or cooldown payload is malformed for this domain.

Limits remain 1,024 effects, 128 bytes per event and 131,136 bytes per batch.
IDs are authoritative captured client effect IDs, with server positions and the
existing reference visibility rule. Voodoo effects are private to the using exact
actor; other item effects are visible to nearby observers. Only original committed
settlement publishes; receipt replay does not. Reconnect skips journal history and
starts this connection-local revision at zero. Failure to send never undoes an item
transaction. A client without capability 350 receives no domain-104 frame.

Capability 23 remains unoffered. This capability enables no S17 casting surface.
