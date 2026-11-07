# Daily boosted creature and boss v1 (BOOSTED-1)

Game owns one durable selection per World and activated server-save reset epoch,
shared across Channels. The native `TIBIA_SERVER_SAVE_BERLIN_10` reset owner defines
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
An unavailable selection is rejected; storage/owner failures close the connection
through the existing unavailable disposition. Only bundle Worlds compose this
mechanic. A capability offer alone does not create a daily selection.

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
