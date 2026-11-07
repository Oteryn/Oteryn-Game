# ITEM_EFFECTS_V1 — shared item visual effects

Capability 385 requires 6. It enables only state domain 105: empty type-1 snapshot
at revision 0, and type-1 deltas containing the bounded `WorldPresentationBatchV1`
codec's `MagicEffect` subset. No commands. Any sound, ambient sound, missile,
spell words, value text or cooldown payload is malformed for this domain.

Limits remain 1,024 effects, 128 bytes per event and 131,136 bytes per batch.
IDs are authoritative donor effect IDs with server positions and the existing
visibility rule. Original committed settlement publishes; receipt replay does not.
Reconnect skips journal history and starts the connection-local revision at zero.
Failure to send never undoes an item transaction. A client without capability385
receives no domain105 frame.

Minimal codec/session/native foundation ported from SV8 commit
813a2d4b1a121e96f807ad2248f77415f37af389 (source capability350/domain104),
renumbered to next-free capability385/domain105 for SRV7 W18 integration.
No unrelated novelty-item behavior is imported.

Capability 23 remains unoffered. This capability enables no S17 casting surface.
