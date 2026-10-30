# Cyclopedia tracker v1

Capability 197 `CYCLOPEDIA_TRACKER_V1`. Command 383 owns the Bestiary tracker; command 384
owns the Bosstiary tracker. Both lists are durable per Character (migration 0570) and survive
logout and restart. Donor: Crystal `ProtocolGame::parseCyclopediaMonsterTracker` and
`refreshCyclopediaMonsterTracker` (`src/server/network/protocol/protocolgame.cpp:2782`,
`:3171`), Crystal `201189f871d23597b82caf88fd611454a2ddefb8`.

## Request

An empty payload reads the stored list without changing it. Otherwise byte 0 is the action
(`0` remove, `1` add) and the remaining 1..128 bytes are the canonical creature key
(`[A-Za-z0-9:._/-]`).

- Command 383 accepts only a Bestiary race of the loaded content.
- Command 384 accepts only a boss of the loaded content, and refuses an add or a remove
  until that boss has at least one credited kill (Crystal `protocolgame.cpp:2790`).
- Adding a tracked creature or removing an untracked one is accepted and changes nothing.
- An add that would take the list past the snapshot bound below is refused.

A refused command is `Rejected` with an empty payload and writes nothing.

## Snapshot

An accepted result carries the refreshed tracker, sorted by key:

| field | encoding |
|---|---|
| kind | `u8`: `0` Bestiary, `1` Bosstiary (Crystal `isBoss`) |
| count | big-endian `u16`, at most 1024 |
| per entry: key | `u8` byte length, then the key |
| per entry: kill count | big-endian `u32` credited kills (Bestiary or Bosstiary counter) |
| per entry: thresholds | three big-endian `u32`: the Bestiary race's stage thresholds, or the boss category's prowess, expertise and mastery kills |
| per entry: completed | `u8`: `1` when every stage is reached, else `0` |

The whole snapshot fits one command result (65 536 bytes); the durable owner refuses an add
that would break that. A stored creature the loaded content no longer knows is left out.

## Domain 98

State domain 98 `CYCLOPEDIA_TRACKER` (owner allocation, 2026-10-30) pushes the same refresh
without a command:

- Snapshot (type 1, revision 1 on every join, reconnect, resume and transfer): the Bestiary
  snapshot followed by the Bosstiary snapshot, as Crystal refreshes both trackers at login
  (`src/creatures/players/player.cpp:11560-11563`).
- Delta (type 1, revision +1): one tracker snapshot, sent after a credited kill of a creature
  on that tracker (Crystal `src/io/iobestiary.cpp:504-507`,
  `src/io/io_bosstiary.cpp:194`). A kill of an untracked creature, or a Bestiary kill past
  the final threshold, changes no tracked entry and sends nothing.
