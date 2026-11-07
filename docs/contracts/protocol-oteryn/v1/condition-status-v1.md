# C08 condition status V1

Rebased onto `c97858c3f440fd26676d3bc9be6d6a35b2c711ff`. Existing native
routes and capabilities 286/287 are retained; the five new shortcuts use
Ctrl+Alt+Shift plus C/R/B/K/P respectively to avoid the base bindings.
C08 mechanic ordinal is 636: base ordinal 631 belongs to arena_match;
package ordinals 632–635 remain unchanged. Capability 276 and command 492 are free.

GAP-SV7; original audit base `c8b3890e601e6073713dcac2ea9ee271f4a257e8`.
The gap was reverified in
`crates/protocol-oteryn/src/actor_spell.rs:127-136` (vitals have no conditions),
`crates/gameplay/dispatch/src/gameplay_transport/actor_spell.rs:1339-1347`
(the owner observes only vitals), and
`crates/client-ui/src/features/local-spell-action-bar/view.rs:22-40`
(the native flow draws health and mana only).

## Request

Capability 276, command 492; empty payload only. Selected bundle-world sessions
with native gameplay can read their own exact actor. The production composed
caller independently checks the runtime-bound Character lease against the
admitted session fence, then reads both condition owners while
holding the runtime and spell-state locks. No client-supplied actor or clock.
No persistence or game-rule change; no migration.

## Reply

A byte count (0..64), then count entries, each: icon `u8`, sub-id `u8`, remaining
milliseconds `i64` little endian. The count is bounded by the sum of the two
existing 32-instance stores. Entries are strictly sorted by `(icon, sub-id)`;
duplicates, trailing bytes, zero duration, values below -1, unknown icons and
sub-ids on non-subtyped families are rejected. `-1` means permanent; finite
DOT schedules never use this sentinel. Finite timers use ceiling milliseconds;
seconds in the native panel also round up. The empty catalogue clears icons.
Two owners of the same visible family collapse to the longest remaining timer;
a permanent instance wins. Refresh reads current authoritative state rather than
advancing or re-applying it. The existing native command queue owns refresh,
capability negotiation, decoding, panel state and logout reset. An open native
status window refreshes once per second, with the same pending-request guard;
the client never invents a remaining timer.

Catalogue (icon/sub-id): 1 Poison, 2 Burning, 3 Energy, 4 Bleeding, 5 Drowning,
6 Freezing, 7 Dazzled, 8 Cursed, 9 Invisible, 10 Drunk, 11 Rooted, 12 Feared,
13 Haste, 14 Paralysed, 15 Food, 16 Recovery, 17 Mana shield, 18 Light,
19 Attributes, 20 Skill bonus/sub-id, 21 Regeneration/sub-id, 22 Outfit,
23 Intense Hex, 24 Muted, 25 Pacified, 26 Powerless, 27 Agony.
All other families require sub-id zero.

## Donor parity and accepted projection differences

Reference classification: owner-requested Crystal donor; `OTS_HYPOTHESIS_ONLY`.
Crystal `zimbadev/crystalserver`, pinned
`201189f871d23597b82caf88fd611454a2ddefb8`, branch `main` audit pin;
source paths below are under that revision. No donor source/assets are copied.

| Rule | Donor file:line / accepted difference |
| --- | --- |
| Own active, unsuppressed condition icons; deduplicate families | `src/creatures/players/player.cpp:778-791` |
| Timed instances use remaining ticks; -1 denotes permanence | `src/creatures/combat/condition.cpp:193-204,420-422` |
| Finite DOT lifetime comes from its unspent damage schedule and next tick | `src/creatures/combat/condition.cpp:1712-1740,1793-1828` |
| Element icons | `src/creatures/combat/condition.cpp:1941-1984` |
| Drunk, Rooted, Intense Hex, Powerless | `src/creatures/combat/condition.cpp:485-526` |
| Mana shield and speed icons | `src/creatures/combat/condition.cpp:1519-1527,2487-2502` |
| Feared icon | `src/creatures/combat/condition.cpp:2289-2294` |
| Duration payload and native labels/glyphs for the complete existing Oteryn catalogue | Owner explicitly requests wire additions and active condition durations; Crystal `src/server/network/protocol/protocolgame.cpp:5712-5733` sends icon flags only. Oteryn uses protocol-owned IDs and native text glyphs, rather than donor numeric masks or proprietary assets. Families without a dedicated donor icon remain visible with their existing condition name; no new condition is granted. |
| No nine-icon truncation | Accepted presentation difference under C08's request to send active conditions; the server sends all bounded active entries instead of Crystal's nine-icon early return (`player.cpp:785-787`). |
| DOT estimate between authoritative refreshes | Tick-schedule duration excludes future renewal/field retention. The next server refresh corrects the schedule if field standing retained ticks (`condition.cpp:1799-1828`). Retained overdue ticks report one millisecond until the bounded tick owner drains them; the status read never deals damage or advances the owner. |

The owner excludes OT auras. This catalogue neither creates nor renders an aura,
OT VIP, autoloot, `!commands`, Dawnport or Prey Hunting Tasks.
