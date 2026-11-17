# ACTIVITY_SCOPE/v1

The existing activity handoff owner changes the runtime scope and issues a fresh
GameSessionId for every admitted participant. Capability117 owns state domain99,
allocated by PR2769 ANSWER6543120139. Soulpit adds command710 (next free670
plus lane offset40), without adding a capability.
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
| 11 | optional bool | Live Soulpit barrier requires acknowledgement; absent means false |

UUIDs, nonzero generations, ordered unique bounded selection and capability117
membership are validated. Unknown or duplicate singular fields, malformed
identifiers, truncation, mixed scope identity and oversized payloads refuse.
The receiving client retains the tag: an Instance identifier is never a Channel.

## Ordering and authority

The complete barrier frame consumes the source connection's next server sequence.
After that frame the client validates the stated fresh session and resets the
destination stream. The complete destination snapshot has target sequence0.
The destination projection carries field11=false; a live Soulpit source barrier
carries field11=true. Other activity barriers retain their previous behavior. Every participant receives this barrier, including those who
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


## Acknowledgement

Command710 `ACTIVITY_SCOPE_ACK` under capability117 has exactly32 payload bytes:
bytes0..16 are the barrier's handoff UUIDv7, bytes16..32 its destination
GameSessionId UUIDv7. It is the destination's command1, under the destination
connection generation, sent only after its entire fresh snapshot validates.
The native client does not send an ordinary gameplay command before this ACK.

The server preserves partially read frame bytes across a live Soulpit switch and
quarantines source input until this exact handoff/session acknowledgement arrives.
At most64 source frames may be discarded under the existing frame/deadline bounds.
No quarantined command executes or obtains a destination command receipt. The ACK
reserves and terminalizes command1 through the normal destination ingress owner.
It is not a scope grant, a user action or permission to change selection.

## Acknowledgement result

The server replies with exactly an empty `Accepted` CommandResult for command1 at
destination server sequence1. Only after validating its generation, sequence,
command id, status and empty payload does the client install the fresh destination
and enable ordinary commands beginning at command2. A missing, malformed, mismatched
or rejected ACK result fails closed without installing partial destination state.
Initial admission/recovery projections have field11=false and require no ACK.

This ordering prevents source command1 from being executed in a fresh destination
whose connection generation is also1. An interrupted source request is canceled
without automatic retry; the destination remains playable after a successful ACK.
Crystal 201189f `data-global/scripts/quests/soulpit/soulpit_fight.lua:79-106` and
`data-global/scripts/actions/soulpit/soulpit_arena_exit.lua:8-16` supply the group
entry and exit behavior; command710 is the native shared-owner transport fence.

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
