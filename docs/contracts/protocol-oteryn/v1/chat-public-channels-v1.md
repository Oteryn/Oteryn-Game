# CHAT_PUBLIC_CHANNELS_V1 (capability 259)

S04c extends the existing CHAT_V1 owner, command 13 and state domain 12.
It requires capabilities 7 and 80 and is offered only in the bundle World.
Both fresh and resumed acceptance messages enforce that dependency closure.
There is no new command, persistent message history or independent relay.

The existing room ids 1 World, 2 English, 3 Help and 4 Advertising remain unchanged.
Selected peers additionally accept 5 Advertising-Rookgaard, 6 Tutor, 7 Gamemaster
and 8 Guild Leaders in existing room intents, room-set bits and room lines.
Peers without 259 neither send these ids nor receive their bits or lines.
The maximum selected room set is eight bits; the legacy set remains four.
The maximum private-channel list payload is 2055 bytes for selected peers
(the room bitmap takes one additional varint byte). Existing legacy payloads
remain at most 2054 bytes.

## Authority and access

The source is Crystal `201189f871d23597b82caf88fd611454a2ddefb8`,
`data/chatchannels/chatchannels.xml` and its scripts. Game group, account type
and group access are independent Control-issued facts. Missing current role
projection means ordinary group/account 1, without access. No game command
can grant a role. PR #2780 comment 6547489522 authorizes the bounded projection;
the external Control issuance interface remains separate.

| Room | Admission | Speaking |
|---|---|---|
| World | Everybody | Level other than 1, or group at least Gamemaster |
| English | Everybody | Level at least 20 or current Premium |
| Help | Everybody | Level other than 1, or nonordinary group; not Help-muted |
| Advertising | Vocation, or group at least Senior Tutor | Level at least 20 or Premium; Gamemaster group bypasses; 120-second interval otherwise |
| Advertising-Rookgaard | No vocation, or group at least Senior Tutor | Level other than 1; Gamemaster group bypasses; independent 120-second interval otherwise |
| Tutor | Group at least Tutor | Admitted members |
| Gamemaster | Account type at least Gamemaster | Admitted members |
| Guild Leaders | Current Guild leader or group access | Admitted members |

Role revocation is observed on admission and again on public-room commands.
The local room view is bound to its original actor/session/connection/lease/scope
and capability selection. An older SQL completion cannot replace a newer binding.
Replacement admission drops queued chat; room dequeue checks the same binding
and capability selection before accepting a line for socket output.
The native Guild owner uses rank 1 for Leader; Crystal uses guild level 3.
The common protocol represents room lines with their room, validated sender name
and unchanged text. PR #2780 answer 6548400568 explicitly defers the following
donor presentation to a native-client follow-up:

| Deferred presentation | Crystal `201189f` source under `data/chatchannels/scripts/` |
|---|---|
| Guild Leaders `[Guild]`, `[Staff]` and `[Staff][Guild]` labels, and its yellow/orange TALKTYPE selection | `guild_leaders.lua:5-20` |
| Tutor yellow/orange/red TALKTYPE conversions | `tutor.lua:5-21` |
| Gamemaster yellow/orange/red TALKTYPE conversions | `gamemaster.lua:5-21` |
| Advertising yellow/orange/red TALKTYPE conversions | `advertising.lua:11-17,30-40` |
| Advertising-Rookgaard yellow/orange/red TALKTYPE conversions | `advertising-rook.lua:11-17,30-40` |
| Help yellow/orange/red TALKTYPE conversions, including orange/red permission flags | `help.lua:63-81` |
| World Chat yellow/orange/red TALKTYPE conversions | `world_chat.lua:8-21` |
| English Chat yellow/orange/red TALKTYPE conversions | `english_chat.lua:7-21` |

No Guild/Staff label is inserted into the sender name or message text. The
CharacterName grammar and full 255-scalar/1,020-byte ChatText limit are preserved.
Native capability-259 UI remains explicitly unsupported in its feature
registration and architecture baseline. This decision adds no command or
capability and leaves channel access, moderation and negotiated SDK behavior
within the implemented scope.

## Help commands

Tutor group and above recognize exact `!mute <online name>` and `!unmute <online name>`
prefixes in Help. The moderator's group must exceed the target's account type.
The target must have an eligible active session in the same World, on any node
or Channel. Unprivileged speakers send those strings as ordinary Help text.

Crystal `help.lua:3-5` sets a 3,600,000 ms condition; `:28` independently sets
180-second speaking exhaustion. `Chat::talkToChannel` and `ChatChannel::talk`
perform no extra channel-condition mute check. Thus speaking resumes at 180 s,
the condition blocks a repeated mute until one hour, and unmute succeeds only
while the speaking exhaustion remains active (`help.lua:46-56`). Both deadlines
are retained separately, not collapsed into a guessed duration.

The original admitted connection/session/lease/scope generation is re-proved in
the same durable transaction as role checks, target resolution, deadline change
and receipt. The same command and request digest replays its stored result with
no second mutation or announcement. This original-fenced receipt lookup precedes
current role, room and mute gates, so a subsequent role revocation cannot rewrite
the result of an uncertain commit. Reusing a command for a different request
is rejected. A possibly committed storage error preserves unknown command state.
Relay announcements are best effort and at most once, like existing room chat.

A recognized moderator command records terminal readiness, room, mute/rate and
malformed-target refusals in the same Help journal and under the same command lock.
Clearing the refusal cannot turn that command into a mutation on replay. Ordinary
group1 speakers still send command-looking strings as regular text; if current
role classification cannot be read, the command remains pending instead of
receiving an invented terminal result. Valid-name request hashes remain unchanged;
malformed-name refusals use a separate digest namespace and retain no target text.

## Session SDK

The session consumer checks capability 259 before accepting extended room ids or
bits in either CHAT snapshot format, room lines, room replacement deltas and
the capability-80 channel list. It also refuses unselected outgoing room intents
before allocating a command id. Invalid pushed room data changes no chat state
and makes that connection unusable. Public-room NOT_ALLOWED outcomes remain
valid for the four legacy rooms, independently of Guild or Party selection.
Capability 80 gates private-channel intents, lines, lists and NOT_INVITED replies;
the typed channel-list consumer reuses the existing session event bound.

All refusal results reuse the existing ChatDisposition values. Role or admission
failure is NOT_ALLOWED; a missing online target is NOT_ONLINE; duplicate mute or
unmute without active exhaustion is REJECTED. Help mute uses MUTED with its
remaining 1..180 seconds. No control-plane role values are accepted on the wire.
