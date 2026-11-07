# Player typing v1 (S19)

Capability **233 `PLAYER_TYPING_V1`** requires **6 `WORLD_SPATIAL_ENTITIES`**
and **7 `CHAT_V1`**. It adds command **447 `PLAYER_TYPING`** and negotiated
delta type **4** to the existing CHAT state domain **12**. The initial offer
is bundle-world only; neither the production fallback nor Item-view offer
includes this capability. Clients that do not select it receive no typing
updates. Native-client input and indicator rendering remain explicitly
unsupported in the feature registration.

## Intent

The command payload is a strict protobuf-shaped message with one required
field: field **1**, wire type **0**, Boolean `typing`. Both values are encoded
explicitly: `08 00` means off and `08 01` means on. Its bound is **2 bytes**.
Missing, repeated, unknown, non-Boolean or trailing fields are rejected before
the runtime owner runs. The actor and session come exclusively from admission;
the command cannot name another player.

## Reply

The ordinary command result has an empty payload. `Accepted` means the current
actor/session/connection binding was validated and the transient state was
queued for its selected player spectators; `Rejected` means malformed input,
missing capability, stale or foreign actor/session/connection, unavailable
transient owner, or the existing aggregate command rate limit refused it.
Typing does not change inventory, progression, party or any durable row.

The existing connection command ingress retains the terminal result.
Replaying a CommandId returns its recorded result without decoding replacement
payloads or executing another broadcast. The existing command-integrity limiter
also charges replays, before dispatch; typing adds no separate unsourced timer.

## Update

CHAT domain 12 delta type **4 `PLAYER_TYPING_DELTA_V1`** contains:

| Field | Wire type | Value |
| --- | --- | --- |
| 1 | 2 | Exact 16-byte spatial actor placement identity |
| 2 | 0 | Nonzero actor generation, uint64 |
| 3 | 0 | Explicit Boolean typing state |

All fields are required exactly once. Unknown fields, zero generations,
wrong identity lengths and non-Boolean states are malformed. Maximum encoded
payload size is **31 bytes**. This is the same actor identity/generation pair
used by the spatial domain, never a CharacterId or account identifier.

Updates advance the ordinary CHAT revision and server sequence. They share
the existing CHAT refresh cadence and per-refresh drain bound. At most one
unsent update per live selected speaker is retained for each recipient, with
the latest on/off state replacing an earlier unsent state. The maps are bounded
by the Channel's admitted session capacity. A new snapshot or chat retirement
drops queued updates; departed speakers are removed from every pending map.
Typing snapshot admission compares connection generations before clearing its
queues. Generic chat admission and refresh preserve that fence, so a delayed
older snapshot cannot replace the current typing selection.
An unselected session retains a disabled generation entry until terminal
retirement; it cannot broadcast, receive or drain typing updates. Admission
checks the still-current actor/session control under the Channel runtime lock
before selecting, so a delayed snapshot cannot recreate an entry after actor
retirement. These entries remain bounded by admitted actor capacity.
No typing state is loaded at login or replayed from durable storage. A client
clears an indicator when the corresponding spatial actor leaves or its
generation changes, and clears all indicators on a replacement snapshot.

## Recipient rule

The Channel owner validates the speaking actor against its current session
and obtains every recipient's current position under the same runtime lock.
The speaker receives its own event when it selected the capability. Selection
replaces Crystal's OTCR-only output gate. There is no party, room, vocation,
line-of-sight or invisibility filter added to the donor spectator rule.

For speaker floor `z`, allowed recipient floors are `0..=7` when `z < 6`,
`0..=8` at `z = 6`, `0..=9` at `z = 7`, and
`max(z - 2, 0)..=min(z + 2, 15)` underground. Set `offset = speaker.z - recipient.z`.
Both `recipient.x - offset - speaker.x` and
`recipient.y - offset - speaker.y` must lie in `-11..=11`. Positions on an
invalid floor are refused. All arithmetic is widened before subtraction.

## Source parity

Primary donor is Crystal
`201189f871d23597b82caf88fd611454a2ddefb8` (`OTS_HYPOTHESIS_ONLY`, selected by
this task as the implementation reference).

| Element | Donor source | Oteryn implementation/test |
| --- | --- | --- |
| Client typing on/off | `src/server/network/protocol/protocolgame.cpp:2818-2820` | `player_typing::decode_intent`; `player_typing_intent_requires_one_boolean` |
| Current player and spectator delivery including self | `src/game/game.cpp:13074-13084` | `TypingRuntime::broadcast`; `player_typing_broadcast_includes_speaker_and_selected_nearby_players_only` |
| Multifloor ranges and projection | `src/map/spectators.cpp:103-116,157-160,180-184`, `src/map/map_const.hpp:20-27`, `src/game/movement/position.hpp:57-59` | `observes`; `player_typing_geometry_uses_donor_multifloor_ranges_and_projected_coordinates` |
| Negotiated receiver and actor identity | `src/server/network/protocol/protocolgame.cpp:11314-11323`, `src/creatures/players/player.cpp:11366-11372` | Capability233 gate and `TypingUpdate` |
| Session-only state | Same donor functions; no database call or stored typing property | Generation-aware typing admission and CHAT retirement; retirement, reconnect and delayed snapshot tests |
| Native framing, replay, overload protection | Oteryn `session_ingress.rs` and `connection/serve/{command,integrity}.rs` | Existing ingress and limiter, reused by command447 |

The native protocol intentionally accepts only the task's Boolean on/off states;
Crystal reads an unrestricted uint8. The native transport coalesces
unsent typing states to bound transient work; this does not alter the final
on/off state observed by a consuming recipient.
