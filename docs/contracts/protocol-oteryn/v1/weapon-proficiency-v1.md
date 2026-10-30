# Weapon Proficiency V1

P08 candidate. Capability 195 requires item views (4), whose domain 9 generation index
names the weapon. Command 378 selects, changes or clears one perk of one Weapon Proficiency
track through the existing Character proficiency writer (`perk_selection` receipts,
PROFICIENCY-0 §4.3). No value is spent.

## Request

Exactly six bytes: the weapon's nonzero item definition reference (unsigned big endian, the
domain 9 generation index), the zero-based proficiency level and the zero-based perk index at
that level, or `255` to clear the level. Other lengths and a zero reference are `REJECTED`
before any read.

## Reply

Carried in an accepted command result: one result byte, a level count `n` (at most 7) and `n`
bytes of the track's committed selections after the command (`255` = none). A refusal before
the track is read carries `n = 0`.

| Byte | Result | Meaning |
|---|---|---|
| 0 | `ACCEPTED` | committed, replayed, or the level already held that perk |
| 1 | `REJECTED` | unknown definition, or an item without a promoted proficiency definition |
| 2 | `UNKNOWN_TRACK` | the Character has no proficiency progress on this weapon |
| 3 | `NOT_UNLOCKED` | the level is above the track's unlocked levels, or out of range |
| 4 | `INVALID_PERK` | the perk index does not exist at that level |
| 5 | `NOT_IN_PROTECTION_ZONE` | a change or clear of a selected perk outside a protection zone |
| 6 | `MODIFIED_LEVEL` | the level's perk is replaced by an active modification (PROFICIENCY-1B §10) |
| 7 | `BUSY` | the track moved or the writer was unavailable; nothing was written |
| 8 | `CONFLICT` | the command's occurrence already committed another selection |

## Rules

- Selecting a perk on an empty level works anywhere; changing or clearing a selected perk
  only in a protection zone (English TibiaWiki `Weapon_Proficiency`, *General*, read
  2026-10-30; `plan_proficiency_selection`). Crystal `zimbadev/crystalserver` `main`
  `src/server/network/protocol/protocolgame.cpp:11873`-`:11888` replaces the whole active
  list without a zone check; Global wins.
- Uncheckpointed kill progress that crosses a level is checkpointed first, so a level reached
  by a kill can be selected at once.
- The occurrence derives from the CommandId, actor and GameSession. A command id replayed in
  the session is answered by the foundation cache; after a reconnect the committed receipt of
  the occurrence is found before any write, so a selection never commits twice.
- After a commit the session's proficiency projection is refreshed from the committed track,
  so the selected perk applies to the next attack.

Perk modification (modify, refine, maximise, reshape, clear) is not part of this capability:
PROFICIENCY-1B §3.3 keeps every operation `NOT_ADMITTED` until shaping content with evidenced
pools, rank values and costs exists.
