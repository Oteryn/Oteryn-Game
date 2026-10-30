# Weapon Proficiency V1

P08 candidate. Capability 195 requires item views (4), whose domain 9 generation index
names the weapon. Command 378 selects, changes or clears one perk of one Weapon Proficiency
track through the existing Character proficiency writer (`perk_selection` receipts,
PROFICIENCY-0 §4.3). No value is spent. Commands 379, 380 and 392-395 modify a selected perk
through the `perk_modification` writer (PROFICIENCY-1B, PROFICIENCY-1C).

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

## Modification request

Commands 379 `PROFICIENCY_MODIFY`, 380 `PROFICIENCY_RANK_UP`, 392 `PROFICIENCY_ORB_RANK`,
393 `PROFICIENCY_RESHAPE_OFFER`, 394 `PROFICIENCY_RESHAPE_CHOOSE` and 395 `PROFICIENCY_CLEAR`
(PROFICIENCY-1B §11.1; the command type is the operation). The payload is the weapon's nonzero
item definition reference (u32, big endian), the slot (1 or 2), for 379 the zero-based level
(below 7) and for 394 the choice (0-2, or 3 to decline the offer), then the track's expected
committed revision (nonzero u64, big endian): 14 bytes for 379 and 394, 13 for the others,
within `PROF1B-RL-01` (24 B). Any other payload is a `REJECTED` command status with no result.

## Modification reply

Carried in an accepted command result: one result byte (PROFICIENCY-1B §11.2), then, after an
accepted command whose slot is still modified, six bytes: level, shaping pool entry index, rank
and the pending offer's three entry indexes (`255 255 255` = no offer). At most 7 bytes.

| Byte | Result | Byte | Result |
|---|---|---|---|
| 0 | `ACCEPTED` | 10 | `SLOT_OCCUPIED` |
| 1 | `NOT_ADMITTED` | 11 | `LEVEL_TAKEN` |
| 2 | `NOT_UNLOCKED` | 12 | `NOT_MODIFIED` |
| 3 | `NOT_IN_PROTECTION_ZONE` | 13 | `RANK_MAX` |
| 4 | `UNKNOWN_TRACK` | 14 | `OFFER_PENDING` |
| 5 | `STALE_REVISION` | 15 | `NO_OFFER` |
| 6 | `REVISION_CHANGED` | 16 | `POOL_TOO_SMALL` |
| 7 | `RATE_LIMITED` | 17 | `SHAPING_OUTDATED` |
| 8 | `CONFLICT` | 18 | `INSUFFICIENT_DUST` |
| 9 | `NO_SELECTION` | 19 | `NO_ORB` |

`NOT_ADMITTED` also answers an unknown definition, a weapon without shaping content and an
operation whose costs or values are unevidenced (PROFICIENCY-1C §3: `RANK_UP` while ranks 1-9 are
unpublished).

## Modification rules

- Every operation needs a protection zone. Checks run in PROFICIENCY-1B §5 order: the
  occurrence's receipt or terminal record answers first, then `PROF1B-RL-03` (two new
  occurrences per second) is charged, then the writer checks the bound revisions, admission,
  track, expected revision, zone, the operation and its dust under the Character cursor.
- Costs and pools are the Global values of PROFICIENCY-1C (`shaping.json`): slot 1 250 dust at
  proficiency level 3, slot 2 1,000 dust when mastered, reshape offer 250 dust, clear free,
  one Lunar Ascension Orb to reach rank 10.
- Draws use the durable `proficiency_shaping` server seed (SIM-DETERMINISM-01 §10) with the
  occurrence, so a retry draws the same entries and a restart reseeds nothing.
- The occurrence derives from the CommandId, actor and GameSession; a replay never charges or
  draws again.
