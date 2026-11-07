# Private channels across a World (S04b)

This extends the existing `CHAT_PRIVATE_CHANNEL_V1` owner (capability 80), `CHAT_INTENT`
command 13, intent tags 7–13, line kind 6, and CHAT domain 12 channel-list delta type 3.
There is no new client command, capability or parallel chat service. The Session SDK
consumes the existing channel-list delta and exposes typed private-channel state. The
production dispatcher selects this World owner for every private-channel intent; it does
not fall back to the former Channel-local state.

## Player rules and primary source

Primary source: Crystal `201189f871d23597b82caf88fd611454a2ddefb8`.

| Element | Rule | Crystal source |
| --- | --- | --- |
| Creation | One channel per Premium owner; first free number from 100 through 9999 | `src/creatures/interactions/chat.cpp:458-472` |
| Invitation | Owner resolves an online character by name in the same World; another Channel or node is not a boundary; self-invitation is refused | `src/game/game.cpp:3862-3882` |
| Repeated invitation | Existing invitation succeeds without adding another member | `src/creatures/interactions/chat.cpp:50-54` |
| Exclusion | Owner removes both the invitation and joined membership | `src/game/game.cpp:3885-3905`; `src/creatures/interactions/chat.cpp:73-92` |
| Joining | Ownership or an invitation grants access; invitation alone does not join | `src/creatures/interactions/chat.cpp:39-43,529-534,707-709`; `src/game/game.cpp:3917-3936` |
| Speech | Only joined members speak; every joined member receives the line, including the speaker | `src/creatures/interactions/chat.cpp:216-228` |
| Owner departure | Closing the owner's channel or ending the owning session closes the room | `src/creatures/interactions/chat.cpp:537-545,574-583` |

The existing Oteryn invitation bounds remain 16 invitees per channel and 16 invitations
per session (`crates/protocol-oteryn/src/chat/private_channel.rs`, `CHATPM-RL-01/02`).
These are accepted Oteryn resource bounds, not new Crystal gameplay limits. The source
wire accepts a nonzero u16 number; the server allocates only the Crystal range above.
The current Canary private-channel implementation has the same Premium, ownership,
invite, exclude, member-only speech and owner-close rules; no Canary override is used.

## Ownership, admission and lifecycle

Migration 0718 adds a negotiated participant registry, channel metadata, invitations/
memberships and immutable command-result receipts. Every key carries the World, and
membership belongs to a durable Game session. Active target resolution uses the existing
character root and account/character admission guards, active session state, non-null
transport and absence of control loss. A target must have selected capability 80 on its
current connection. It need not share the sender's Channel or node.

Fresh operations prove the recovery record, current node incarnation, Channel assignment,
character lease, game session and connection generation at prepare and again at commit.
Private metadata mutations and receive custody additionally serialize under the existing
admission guard locks and a World private-channel lock. No runtime mutex is held across
PostgreSQL I/O. Creation uses the existing Premium coordinator and its physical activation
transaction/savepoint; a separate boolean Premium read never authorizes a later write.

A reconnectable session retains its channel and invitations during grace. A terminal
session cannot authorize a new operation or receive a line. A terminal owner immediately
ceases to authorize its room; a later fresh operation removes the inactive metadata.
Disabling capability 80 removes the session's room, invitations and memberships in the
same transaction. A stale admission cannot delete a newer runtime binding.

The numeric channel identifier may be reused. Each creation also receives a never-reused,
server-only epoch. The epoch is carried in the authenticated relay destination and queued
metadata, never accepted from the client. A reused numeric identifier does not authorize
any queued line from the earlier channel.

## Publication, replay and privacy

The existing encrypted World relay carries private-channel text. No plaintext, ciphertext,
name-bearing request document or message history is written to the new tables. A command
receipt stores its session/World/character/connection binding, command number, SHA-256
request digest, disposition and wait seconds. The digest binds every intent field, including
the channel number, target name and text. Reusing a command number for a different request
is rejected. Retained exact results can be replayed after reconnect in the same session,
following the existing durable owner convention; replay never grants fresh mutation rights.

Successful publication commits its encrypted PostgreSQL notification and terminal receipt
in one transaction. Database logging settings are pinned before any sealed publication is
bound. Terminal text, mute, rate or readiness refusals also receive a durable result: a
previously refused command cannot later publish when conditions improve. An unknown
commit outcome follows the existing transport ambiguity/disconnect path, allowing the
original command to resolve from its record rather than reporting a false refusal.

Delivery is ephemeral and at most once. `OK` means that the existing World publication
committed, not that each recipient consumed a line. Listener loss can lose a committed
notification; the server does not replay chat history. Relay freshness, authenticated World
binding and listener readiness remain owned by the existing relay.

Private lines use the existing shared CHAT egress bound and delivery acknowledgement.
A bounded server-only cache of current joined channel/epoch/join-time interests filters
notifications before they occupy a local queue. Unrelated private rooms therefore cannot
consume that session's slots. Admission/list reads, successful joins/creation/departure and
exact successful replay refresh these interests from the same durable owner. The cache
never grants disclosure. Per-admission reservations prevent an old asynchronous result
from replacing a newer connection binding or reviving a forgotten session.
Immediately before queue custody, the receiver rechecks its complete admission fence,
capability, current joined membership, owner lifecycle, channel epoch and join timestamp.
A line predating the current join, an excluded member or an earlier channel epoch is
removed without disclosure. The exact queued line and its server-only destination metadata
must still be at the front when custody changes. As with Guild and Party, a line accepted
before a later exclusion/close can finish its already-authorized socket write afterward.

The channel list includes the existing public owner's current access-filtered room set.
Peers without the public-channel extension do not gain the added room bits. List changes
are discovered from the World owner, so an invitation or closure on another node reaches
the existing client list delta without a new protocol message.

## Session SDK and native UI boundary

`crates/session/src/private_channels.rs` decodes channel-list delta 3 only after capability
80 is selected. It uses the existing bounded codec, then verifies that added public-room
bits require capability 259 before changing state. The list uses the shared CHAT revision
and replaces the previous list whole. Available public rooms do not change the open-room
set, and metadata does not occupy the chat-line history.

The SDK re-exports `ChatChannelList`, `InvitedChannel`, `PrivateChannelIntent` and
`PrivateChannelLine`. `Session::private_channel_list()` and `ChatLog::private_channels()`
return the most recent validated list; they return `None` before the first list delta.
`SessionEvent::ChatChannelList(AppliedDelta<ChatChannelList>)` preserves server sequence
and base/new revisions. Malformed or oversized lists, stale revisions and missing
capabilities change neither the stored list nor the CHAT revision. The existing command,
line and refusal adapters also gate private variants on capability 80.

This is SDK support, not a completed native private-channel panel. At the pinned Oteryn
base `ae4188a9c8d461acb152044814ea23075de02ed4`,
`crates/client-ui/src/features/0080-chat-private-channel/registrations/handlers.json:4-13`
explicitly marks the native feature unsupported with empty commands and pending
state/protocol/view/input/settings; its fixture repeats that declaration. This slice
does not claim that an existing native private-channel UI is playable.

## Verification

`private_channel_postgres_world_lifecycle_fences_atomic_replay_and_epoch` uses two real
assigned nodes/Channel runtimes in one World, existing wire dispatch and two existing relay
listeners. It reads committed rows and checks creation, invite/join/speech/exclude/leave,
Premium refusal, malformed/unselected wire, repeated commands and request rebinding,
listener-down refusal replay, both invitation caps, other-World target isolation, stale
admission, prepare/commit invalidation, exclusion/rejoin custody, numeric ID reuse, owner
grace/terminal lifecycle, capability withdrawal and absence of chat storage columns.

The original private-channel rule tests remain strict. The separately mounted private
message case exercises actual ingress and relay delivery across the same two-node fixture.
The Session SDK tests cover real pushed list events, full replacement, interleaved private
lines, shared revision continuity, capability 80/259 gates and atomic refusal of malformed,
oversized, over-capacity and stale lists. Passing configured PostgreSQL cases print their
completion marker. They return `Ok(())` without running when the test database is absent;
a no-database run is not evidence that durable transitions passed.
