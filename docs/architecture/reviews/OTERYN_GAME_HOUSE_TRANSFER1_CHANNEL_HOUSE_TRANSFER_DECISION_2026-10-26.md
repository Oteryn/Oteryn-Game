# HOUSE-TRANSFER-1 Channel / House transfer

Date: 2026-10-26. Owner instruction: HOUSE-TRANSFER-1, NIGHT-W1.
Status: implementation decision; independent protocol, persistence and security
review and protected integration remain required before release.

Must decide now: YES. HOUSE-RUNTIME-0 §4 and ADMIT-0 §3.2 explicitly
delegate the transfer wire and lost-acknowledgement boundary to this contract.

## Identity and custody

A House is `(WorldId, HouseId)`, with the tagged assignment key and derived
runtime scope of migration 0074. It is never a Channel tile or an arbitrary
Instance UUID. The origin Channel is routing history, not House authority.
The native carrier retains the actor's state and identity while custody moves;
there must be exactly one writer and no parallel player actor store. Channel
actor references may identify the originating allocation; they cannot grant
Channel mutation authority while the House owns the actor.

The first implementation admits same-node transfers only, preserving 0074's
same-node entry restriction. Cross-node handoff, voluntary Channel change and
generic Instance transfer stay refused. A missing destination owner, assignment,
valid tile, capability support or recovery proof is a refusal, never permission
to synthesize authority.

## States and messages

Capability 86 `HOUSE_TRANSFER_V1` (`house-transfer-v1`) gates command 102 and
state domain 51. Existing clients keep the closed boundary. The transfer intent
names a direction and a House, never an assignment, lease, destination Channel,
transport or selected capability set. Those are server decisions.

`SOURCE -> PREPARED -> COMMITTED/AWAITING_ACK -> ACKNOWLEDGED` is the successful
path. PREPARED freezes actor input and reserves destination space while source
authority remains live. Abort before commit releases that reservation and
restores the source. COMMITTED terminalizes the source and creates a fresh
destination GameSession at a strictly newer Character lease in one transaction.
There is no compensating transfer back after commit.

The command result's successful acknowledgement names transfer id, destination
GameSession, World, explicit Channel or House identity, destination ownership
generation, connection generation, Character lease, actual position and complete
selected capabilities. It is emitted only after durable commit and native custody
reconciliation. A refusal contains no destination grant.

Destination required capabilities are checked before commit against authenticated
client support. The fixed selection never widens beyond that support. The
destination Channel's generation-bound ADMIT-0 effective set is mandatory on
exit even if the client entered with a smaller House selection.

No destination snapshot or capability-gated message may precede this
acknowledgement or its authenticated recovery. Domain 51 is a replacement scope
projection, not a second authority or an alternate map. Delivery failure closes
the connection without announcing refusal of a possibly committed operation.

## Durable fences

Entry uses the existing prepare/commit implementation, including its property,
ACL, guild revisions, reservation, origin assignment and House assignment locks.
The transport additionally proves the exact live source transport, connection
generation, Character lease, current node incarnation and admission claims.

Exit has a separate direction record in migration 0215. Migration 0074's
`direction = 1` CHECK and session/handoff proofs remain intact. Exit requires the
live named House session, its derived instance identity, origin Channel, exact
connection generation and lease, House assignment and destination Channel
assignment, transport and current node incarnation. Destination admission uses
fresh session-use membership. Last-position publication and clearing House
placement commit with exit. Migration 0149's bed fences are unchanged.

Every transfer is idempotent by source GameSession and command identity. An
identity reused with different direction, House or selection conflicts. A stale
source may observe a historical result only through authenticated recovery; a
receipt never grants current authority by itself.

## Recovery and failures

The durable acknowledgement and routing record commit with the destination
session. A lost acknowledgement, including a crash after commit and before any
write to the socket, is recovered by authenticating recovery credentials against
the existing current security/trust floors, following the committed source to
its current destination, and independently rechecking the live destination
session, lease, transport generation, assignments, node and current admission
claims. A new transport requires a strictly newer connection generation.
Only then is a scope-identifying recovery acknowledgement sent, before any
snapshot. Replaying immutable evidence alone never reinstalls an actor owner.

Before commit, crash recovery aborts PREPARED and retains the source. After
commit, recovery installs only the destination. Unknown commit outcome blocks
source input until readback resolves it. Replaced assignments, stale node,
revoked access, mismatched content, unsupported capabilities, malformed input,
concurrent transport replacement and exhausted generations fail closed.

## Exit placement and parity choice

Exit targets the recorded origin Channel's catalogue entrance, with the existing
CHAR-POSITION-0 fallback when blocked or occupied. The player cannot select a
different Channel. An unavailable origin requires ordinary server admission
fallback; until that route is independently available the transfer refuses
before changing source authority.

Canary/Crystal use physical House tiles, invitation checks at admission and
the House entrance for kick/return. Oteryn retains those observable rules while
using its accepted multichannel House ownership. No upstream code/assets are
redistributed. Beds, item custody, ACL management, kick/revocation, idle unload
and cross-node routing retain their own owning contracts; this decision grants
none of their mutations.
