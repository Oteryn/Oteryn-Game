# PS-R2-003 — House handoff sends an intervening ambient frame

Base: `4f0b6c62a789489fca6c37dd36d5917d096f328e`.

**CONFIRMED P2**, independently verified from the actual producer, native client caller, capability metadata, consumer and error handling. This is static integration proof; no native async/TLS/database reproduction was executed.

## Verified path and effect

`apps/client/src/native_entry.rs:158-174` obtains the actual supported capability list from `features/0179-hotkey-equip/preferences.rs:21-39`. That function filters only capability 180 when smart equip is disabled. It retains House transfer 86 and ambient sound 288, both included in `crates/session/src/capability_support.rs`. The corresponding protocol capability metadata offers both in all three current offer sets. Ambient requires capability 6, also part of the default support set. These facts describe the checked source; deployed server configuration is unknown.

The native House panel creates capability 86 command 102 in `apps/client/src/features/0086-house-transfer/protocol.rs:8-19`. The live server dispatcher requires an input permit and selected command ownership before invoking `connection/serve/house_transfer.rs`. A successful authority result follows the real durable commit and custody reconciliation in `gameplay_transport/house_transfer.rs`; the Channel selection is retained, including ambient sound.

In `crates/gameplay/dispatch/src/gameplay_transport/connection/serve/house_transfer.rs:18-65`, the server writes the accepted command result, then unconditionally sends an ambient silence update when capability 288 is selected, and only then returns `Transferred`. `refresh.rs:697-734` encodes that update as a StateDelta using the source generation and sequence. It does not check whether any sound was playing. The outer serve loop handles `Transferred` only after this write and subsequently emits the destination snapshot.

The real `crates/session/src/gated_command.rs` synchronously invokes `property_services/house_handoff.rs` for the accepted result. The latter adopts the acknowledgement's destination generation and requires the immediately following frame to be SnapshotBegin. The ambient StateDelta therefore fails generation validation or, when generations happen to be equal, the message-type check. `session_exchange.rs:13-24` marks the Session unusable for either error. A valid accepted House entry with both capabilities selected consequently cannot complete this handoff on the current native Session.

## Counterguards and limits

Rejected transfers and sessions without ambient capability do not take the offending branch. Existing input, capability, lease and custody checks still apply; this finding does not bypass authority. The transfer is already committed when delivery fails. The architecture decision explicitly requires recovery to follow the committed destination instead of rolling back to the source. No durable loss, permanent reconnect lockout or production incidence is claimed.

The ambient helper also writes the source sequence into the new admitted session's continuity. The independent follow-up checked the actual fresh continuity constructor (`gameplay_transport/house_transfer.rs:29-41,363-371` and `connection.rs:299-306`), outer transfer forwarding, House snapshot counter and `session/replacement_snapshot.rs:14-24`. For a normal fresh destination without retained command reconciliation, a source counter S becomes S+1 for the result, then S+2 for ambient, then S+3 for the House snapshot target. The client expects 1. Merely skipping the unexpected ambient frame would consequently leave a second sequence mismatch in the same transition. This is part of the same finding, not an additional count; it does not establish a persistent reconnect failure or durable loss.

## Recommendation

Make the handoff's presentation reset and acknowledgement/snapshot order agree on both peers, preserving generation fencing and committed destination recovery. A focused regression should use the actual producer and Session with capabilities 86, 288 and its prerequisites, covering a silent initial ambient state and a nonzero source sequence. This report does not modify game code.
