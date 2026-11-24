# Visible player status v1

Capability 600 requires capability 6 (world spatial entities). The bundle-world
server offers command 6363 only through this capability. Existing logout and
server-message payloads keep their existing wire shape.

## Request

Exactly 24 bytes: the 16-byte spatial placement identity and the unsigned
64-bit big-endian incarnation from the observing session's world visibility
view. Both must be nonzero. The server rechecks current player type, exact
incarnation, view range and privacy rules for invisibility and staff ghosts.
Queries for one's own actor, unknown targets, or actors outside the observer's
view are rejected with an empty result.

## Reply

An accepted response has two bytes. The first is 1 (online) or 2 (away).
Away means the committed actor is in control-loss grace or has reached the
existing server-configured gameplay inactivity warning threshold. Reading
status does not refresh the idle timer. The second is 0 or 1, the target's
server-owned no-logout tile flag. The tile flag is independent of presence.
Unavailable current facts are rejected with an empty result.

This is a point-in-time view of a visible actor. Clients clear it when the
actor leaves the interest set or the connection is replaced. It exposes no
account identifier, remote presence, or exact idle duration.
