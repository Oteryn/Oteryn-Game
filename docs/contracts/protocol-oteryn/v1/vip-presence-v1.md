# VIP presence V1

Capability **258 `VIP_PRESENCE_V1`** requires capabilities 7 (`CHAT_V1`) and 43
(`VIP_V1`). It is offered only by `BUNDLE_WORLD_OFFERED_CAPABILITIES`.
The list and consent commands remain 48–50; this extension allocates no additional
client command, state domain or durable relationship owner.

## Wire

The existing CHAT domain 12 revision stream carries delta type **5**
`VIP_PRESENCE_DELTA_V1`. Its schema is `vip-presence-v1.proto#VipPresenceBatchV1`.
Types 1–3 remain existing chat/private-channel payloads; type 4 belongs to the
separately delivered player-typing capability 233.

Each batch replaces the complete presence view, with at most 100 entries sorted
by distinct positive list-local entry numbers. Each row contains the current
character name, coarse status, announcement bit and confirmation lifetime.
No AccountId, CharacterId, WorldId, ChannelId, node, position or instance is encoded.
The maximum encoded batch is 14,400 bytes (100 bounded names and worst-case
protobuf fields). Unknown fields, duplicated scalar fields, malformed names,
invalid statuses, out-of-order/duplicate entry numbers and excess rows are refused.

Status values are `UNCONFIRMED=0`, `OFFLINE=1` and `ONLINE=2`. ONLINE carries
`confirmed_for_ms` from 1 through 10,000; other statuses require zero. UNCONFIRMED
cannot announce. The client demotes ONLINE to UNCONFIRMED when this monotonic
lifetime expires; a failed or stalled connection cannot leave ONLINE indefinitely.

## Existing owners and privacy

The accepted VIP0 decision, section 8, governs privacy. The list remains shared
per (Account, World), but each current observing Character must have its own
accepted consent from the target. Without that consent the row is UNCONFIRMED.
A hidden target is OFFLINE. Otherwise the authoritative session row in states
1 or 2 on the same World is ONLINE, including another Channel/node and grace.
Channel transfer and death do not produce a logout transition.

Every observation runs under the existing recovery, exact session-generation,
lease, node and scope fences and reads consent, visibility and target session
truth together. An unavailable or failed read sends no ONLINE confirmation.
Queries use the existing VIP and session indexes with LIMIT 101 to reject overflow.

## Notifications and reconciliation

The existing sealed World chat relay carries kind 6 presence hints with the
server-only target identity and an online/offline bit. Hints allocate only for
targets already watched by local sessions, are coalesced within their bounded
list, and never substitute for an authoritative read.

Login and terminal-logout hints use the Character identity proven by committed
admission or the canonical terminal session row. Publication does not depend on
the target selecting CHAT or having a local chat entry. A missed advisory hint
is repaired by the existing bounded reconciliation.

A login/logout notice is sent only if an entry has notifications enabled, a
previous confirmed ONLINE/OFFLINE state actually changes, and the authoritative
read agrees with the hint. Initial snapshots, reconnects, missed-hint correction,
consent changes and visibility changes do not announce. Text follows Crystal
`player_vip.cpp:48-66`: `<name> has logged in.` / `<name> has logged out.`

The server refreshes ONLINE confirmation every 10 seconds and the full view at
least every 60 seconds, as VIP0-RL-16/-09 require. Local list/consent writes mark
the local view dirty; complete replacements correct remote edits and deletions.
Listener loss immediately invalidates local confirmations. Restoration forces
a fresh read without announcing transitions that might have happened while down.

Fresh VIP mutations on sessions selecting capability 258 return UNAVAILABLE
while the World relay is not ready. This refusal is retained by the existing VIP
command owner under the original session, CommandId and canonical request hash.
Exact replay precedes current readiness and remains unchanged after local runtime
loss or listener recovery; changed-request reuse is REJECTED. The denial path
cannot apply the requested mutation or spend a rate token. Migration 0716 extends
the existing writer for this outcome; its signature, authority fences, retained
request binding and grants remain unchanged.

This slice uses one bounded read per observing session, with at most 101 rows
including overflow detection. Node-wide SQL batching from VIP0 section 8.2 is an
approved performance follow-up. The per-session reader preserves the 10-second
ONLINE confirmation, 60-second reconciliation and immediate listener-reconnect
read with no announcement. It adds no second scheduler or shared authority cache.

## Client surface

The session client decodes this negotiated delta, emits the typed
`SessionEvent::VipPresence` event and exposes an expiring presence view. Reading
that persistent view clears announcement bits and demotes expired ONLINE entries
without requiring another network frame.

The native capability-258 feature registration and architecture baseline remain
explicitly unsupported pending the separate native VIP panel. This slice provides
the server and typed session API; it does not add native panel state, rendering,
input or settings.
