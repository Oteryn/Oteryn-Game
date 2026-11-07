# ACTIVITY_SCOPE/v1

The existing activity handoff owner changes the runtime scope and issues a fresh
GameSessionId for every admitted participant. Capability117 owns state domain99,
allocated by PR2769 ANSWER6543120139. It adds no command and no capability.
The server uses the existing StateSnapshot envelope.

## Snapshot

Payload type1 is the strict protobuf-shaped record encoded by
`crates/protocol-oteryn/src/activity_scope.rs`, bounded to2048 bytes.

| Field | Wire type | Meaning |
| --- | --- | --- |
| 1 | bytes16 | Retained handoff occurrence UUIDv7 |
| 2 | bytes16 | Source GameSessionId |
| 3 | bytes16 | Fresh destination GameSessionId |
| 4 | varint | Scope tag:1 Channel,2 Instance |
| 5 | bytes16 | WorldId |
| 6 | bytes16 | ChannelId or InstanceId, according to field4 |
| 7 | varint | Current destination ownership generation |
| 8 | varint | Current destination connection generation |
| 9 | varint | Current destination Character lease generation |
| 10 | repeated varint | Exactly the retained selected capability IDs, strictly increasing |

UUIDs, nonzero generations, ordered unique bounded selection and capability117
membership are validated. Unknown or duplicate singular fields, malformed
identifiers, truncation, mixed scope identity and oversized payloads refuse.
The receiving client retains the tag: an Instance identifier is never a Channel.

## Ordering and authority

The complete barrier frame consumes the source connection's next server sequence.
After that frame the client switches to the stated fresh session and resets the
destination stream. Full destination snapshots begin at sequence1 before further
gameplay commands. Every participant receives this barrier, including those who
did not activate the lever.

The server rebinds the entire composed native owner, the exact actor, the durable
session fence and its retained selected capabilities together. A retained
notification is a receipt, never current assignment, node, lease or transport
authority. Retries cannot resurrect a delivered barrier or widen selection.
The old socket's loss handler follows only its exact controlled successor chain;
it cannot terminate a later resumed connection.

An unsuccessful or uncertain SQL handoff retains the native reservation and
immutable attempt for reconciliation. No successful scope notification is sent
for an uncommitted handoff. Initial transfer accepts only a fresh, current,
control-preallocated Instance held by the same node, with a source-qualified
activated template. Missing capacity and missing content are typed refusals.

## Native client adoption

The Session decoder checks the source session, World and retained selection before
reading the destination's full sequence1 snapshot. Its first domain99 must match
the committed barrier exactly and use the destination connection generation as
its revision. No identity, counter or domain cache changes until the complete
snapshot validates. A current-session domain99 in cold admission or requested
recovery is an idempotent typed projection; it preserves the admitted counters.

The native play task discards buffered source projections and unsent source
commands, clears source panels, targets and scene inputs, then renders the fresh
map. Channel and Instance remain distinct tagged identities. Destination actions
are enabled after the native view acknowledges that scope. An in-flight source
request interrupted by the barrier returns local `ScopeChanged`, which keeps the
play loop alive without inventing a server command result.

Every command after adoption carries the destination GameSession UUID in
ClientCommand field16, including the first command when both old and new streams
use connection generation1 and command1. Requested recovery retains that binding.
