# Observer presentation v1 (PAR-M10)

Capability **262 `OBSERVER_PRESENTATION_V1`** requires **6 `WORLD_SPATIAL_ENTITIES`**
and **7 `CHAT_V1`**. It adds no command. It adds negotiated delta type **6** to the
existing CHAT state domain **12**. The initial offer is bundle-world only; neither the
production fallback nor the Item-view offer includes it. A client that does not select it
receives no presentation. `oteryn-session` decodes the delta into
`SessionEvent::ObserverPresentation`; the native effect and speech views remain
explicitly unsupported in the feature registration.

The capability carries two transient world presentations that Crystal scripts send to
the players who see them: a magic effect at a position, and a line a creature says with
`TALKTYPE_MONSTER_SAY`. Neither changes game state, and neither is stored or replayed.

## Update

CHAT domain 12 delta type **6 `OBSERVER_PRESENTATION_DELTA_V1`** holds exactly one of two
length-delimited fields:

| Field | Wire type | Value |
| --- | --- | --- |
| 1 | 2 | `MagicEffect` |
| 2 | 2 | `MonsterSay` |

`MagicEffect`:

| Field | Wire type | Value |
| --- | --- | --- |
| 1 | 2 | Position: the `WORLD_SPATIAL` position message (sint32 x, sint32 y, sint32 floor) |
| 2 | 0 | Effect id, `1..=65535` (Crystal `MagicEffectClasses`; 0 `CONST_ME_NONE` is never sent) |

`MonsterSay`:

| Field | Wire type | Value |
| --- | --- | --- |
| 1 | 2 | Exact 16-byte spatial actor placement identity of the speaker |
| 2 | 0 | Nonzero speaker generation, uint64 |
| 3 | 2 | Position of the speaker, as above |
| 4 | 2 | UTF-8 text, 1 to 255 bytes |

Every field is required exactly once. Unknown, repeated or trailing fields, a zero
generation, a wrong identity length, a floor outside `0..=15`, a coordinate outside
`0..=65535`, an empty text or invalid UTF-8 are malformed; a text over 255 bytes or a
payload over **302 bytes** exceeds the limit. The speaker identity is the spatial
identity and generation pair, never a CharacterId or account identifier.

The effect ids the M10 scripts send are `CONST_ME_POFF` 3, `CONST_ME_STUN` 32,
`CONST_ME_ICEATTACK` 44 and `CONST_ME_WATERSPLASH` 54.

Updates advance the ordinary CHAT revision and server sequence and share the CHAT refresh
cadence. The Channel owner keeps at most 32 unsent presentations per live session and
drops the oldest beyond that, so a session that never drains its queue cannot grow it. A
session that leaves the Channel loses its queue. Nothing is loaded at login or replayed.

## Recipient rule

The Channel owner reads the source position and every recipient's current position under
one runtime lock.

- A magic effect uses the multifloor spectator rule of player typing
  (`player-typing-v1.md`, *Recipient rule*) with range 11 on both axes.
- A monster-say line reaches only players on the speaker's floor within 8 tiles on x and
  6 tiles on y.

## Source parity

Primary donor is Crystal `201189f871d23597b82caf88fd611454a2ddefb8`.

| Element | Donor source | Oteryn implementation/test |
| --- | --- | --- |
| Magic effect to player spectators, multifloor | `src/game/game.cpp:8950-8961`, `src/map/spectators.cpp:98-190`, `src/map/map_const.hpp:20-23` | `ChannelSpellStates::present_magic_effect`; `m10_effects_reach_multifloor_spectators_and_say_only_the_floor_viewport` |
| Say line to spectators on the speaker's floor in the client viewport | `src/game/game.cpp:6941-6960`, `src/map/map_const.hpp:20-21` | `ChannelSpellStates::present_monster_say` |
| Effect and talk type ids | `src/utils/utils_definitions.hpp:58,87,99,109,390` | `observer_presentation::MAGIC_EFFECT_*` |
| Yellow pillow 2397 | `data/scripts/movements/yellow_pillow.lua:3-12` | special movement step; Oteryn has no ghost mode, so the `isInGhostMode` check never returns early |
| Bath tub water splash | `data/scripts/movements/bath_tub.lua:13` | special movement step |
| Magic wolf trap and Thornfire wolf | `data/scripts/movements/trap.lua:40`, `data/scripts/movements/thornfire_wolf.lua:10` | creature step hooks |
