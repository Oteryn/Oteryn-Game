# Daily boosted creature and boss v1 (BOOSTED-1)

Game owns one durable selection per World and activated server-save reset epoch,
shared across Channels. The native `TIBIA_SERVER_SAVE_BERLIN_06` reset owner defines
the boundary; process startup and client clocks never choose the day. The selector
canonicalizes eligible active-generation definition keys, hashes World/epoch/slot,
and persists the first choice under the active reset pointer lock. Reload returns
that row before considering pools. With multiple eligible races, yesterday's race
is excluded independently for both slots (Crystal parity). A singleton pool keeps
its sole race. Empty pools leave the optional mechanic unavailable.

The creature pool contains Bestiary creatures, excluding Bosstiary, familiars and
reward bosses. The boss pool contains Archfoe Bosstiary creatures. These predicates
are projected from Game's admitted native profiles, never from client input.

## Query

Capability 151 `DAILY_BOOST_V1`, command 177 `DAILY_BOOST_QUERY`.
The payload is empty. The admitted route requires the negotiated capability,
current actor control and matching connection/scope fence. It revalidates the
World's active epoch and Channel owner in the bounded durability transaction.
Only bundle Worlds compose the mechanic. A capability offer alone does not
create a daily selection.

## Refusal

With the capability selected, a rejected command result carries a one-byte
`DailyBoostRefusal`. The server logs the reason with the command ID and result
sequence. A rejected query has no post-command presentation refresh.

| Code | Reason |
|---:|---|
| 1 | `MissingFence` |
| 2 | `SessionMismatch` |
| 3 | `ConnectionGenerationMismatch` |
| 4 | `InvalidCommandId` |
| 5 | `WrongCommand` |
| 6 | `CapabilityNotSelected` |
| 7 | `MalformedQuery` |
| 8 | `RuntimeBusy` |
| 9 | `OwnerUnavailable` |
| 10 | `ScopeMismatch` |
| 11 | `ScopeGenerationMismatch` |
| 12 | `WorldMismatch` |
| 13 | `ChannelMismatch` |
| 14 | `MissingActor` |
| 15 | `ActorNotCurrent` |
| 16 | `ControlLost` |
| 17 | `HolderBusy` |
| 18 | `DeadlineExceeded` |
| 19 | `SelectionUnavailable` |
| 20 | `SelectionMissing` |
| 21 | `ViewEncoding` |
| 22 | `Unavailable` |
| 23 | `InputUnavailable` |
| 24 | `InputGuardDeferred` |
| 25 | `RuntimeBusyBeforeSelection` |
| 26 | `RuntimeBusyAfterSelection` |

The two phase-specific runtime refusals mean the command exceeded its runtime
wait budget and can be retried as a new query. Code 8 is retained for decoding
replies from earlier servers.

## View

An accepted command result contains:

| Field | Encoding |
|---|---|
| reset_epoch | u64 big endian |
| creature key length, key | u8 length, 1–255 UTF-8 bytes |
| boss key length, key | u8 length, 1–255 UTF-8 bytes |

Maximum encoded view: 520 bytes. Extra query bytes and invalid or empty keys are
rejected. There is no unsolicited frame or state domain.

## Consumers and recovery

The committed selection is installed on the Channel runtime before gameplay owner
initialization and the session-independent home-spawn loop. Home-spawn timing is
computed from its authored delay on every schedule operation: selected creature
half-delay composes with Cyclopedia half-delay, with the Reference one-second minimum and without compounding on reload.
The boss slot does not alter XP or respawn in the pinned references.

Kill capture freezes the creature's doubled base XP and additional independent
loot roll into the existing pending kill row before party splitting and durable
receipt settlement. The boss slot adds two rolls plus a deterministic third on an inclusive 0–100 draw below 50
(+250% configured default loot bonus). Extra boss rolls include only admitted equipment and the three Reference equipment bags. Unique-drop metadata is not represented in the current native loot section; that Reference exclusion remains unqualified. Appended Forge resource entries roll once. Extra rolls
use distinct hash draw kinds. All rolls share the existing per-death 32-draw,
16-Item and 30,720-byte plan ceilings; requests exceeding any bound fail closed
before settlement. Each successful extra roll mints a distinct Item with its own
draw ordinal, preserving non-stackable item identity. No resource limit is raised.


Reference semantics are provisional OTS evidence, not verified against Global:
Canary `47dfd51f45280a59a1d3e50ba7edd573d7234446`, Crystal
`201189f871d23597b82caf88fd611454a2ddefb8`. Boss slots/progression and contribution-based
reward-chest distribution remain their separate owning mechanic. This change
applies the daily bonus through the existing Game kill/corpse settlement path.

## Selected World login probe

Use the selected World's game endpoint over the existing framed TLS transport.
The standalone, opt-in bootstrap command 6326 `WORLD_LOGIN_INFO` has a
one-request connection. The usual `ClientCommand` and `CommandResult`
envelopes contain only fields 1 (type) and 4 (bytes), in that order. The
request body contains fields 1 (command ID 1), 2 (6326) and 4 (16-byte nonzero
WorldId), in that order. The reply carries command ID 1 and status Accepted,
with a UTF-8 JSON payload of at most 4096 bytes containing `world_id` (sixteen
byte integers) and `info`. There is no generation or sequence for this
standalone exchange. Command 6326 is available only on the first frame.
The listener replies once and closes. The probe carries no credentials,
consumes no admission grant, creates no session and needs no capability.
Normal player bootstrap, the authenticated operator bulletin and command 177
keep their existing wire layouts.

`info` is null for an unavailable or mismatched World. Otherwise it contains
`players_online` (u32 count of active, eligible, currently connected
characters in that World across Channels) and `boost`. A non-null boost
contains `reset_epoch` (u64), `effective_from` and `effective_until` (i64 Unix
milliseconds), and `creature` and `boss`. Each race publishes `key`, `name`
and `artwork`, with artwork fields `kind` (`outfit` or `item`) and
`look_type` (positive u32 source appearance reference). The interval starts
at active reset activation and ends at the next scheduled Europe/Berlin 06:00
server save. Game reads the World owner's committed choice and resolves
presentation using the endpoint's loaded native catalogue. During retirement
or if the selection or presentation is unavailable, `boost` is null.

The Tibia Fandom archive documents login-screen placement and the duration
through server save. [Boosted Creature](https://tibia.fandom.com/wiki/Boosted_Creature)
[Daily Boosted Boss](https://tibia.fandom.com/wiki/Cyclopedia#Daily_Boosted_Boss)
