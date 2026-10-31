# CYCLOPEDIA_CHARACTER_V1

Capability 199 carries the Cyclopedia character window tabs that the earlier
`CHARACTER_INFO_V1` summary (capability 97) does not (PAR-P12, Crystal
`protocolgame.cpp` `sendCyclopediaCharacter*`). The server owns every fact; a
request only names the tab. The capability is offered only by a node serving a
World booted from a bundle (`BUNDLE_WORLD_OFFERED_CAPABILITIES`). Without the
capability selected, its commands are refused.

## Badges request

Command type 387 has an empty payload. It names the requesting session's
account; no other account can be read.

## Badges reply

`count:u8`, then `count` badge ids (`u8`) in ascending order. At most 20
badges, 21 bytes.

Badges belong to the account and are permanent, as in Global (TibiaWiki
*Social*, revision 1111449). Before replying, the fenced account badge owner
grants every badge whose requirement the native facts meet; it never revokes a
badge. The same refresh runs at login admission and after a kill settlement
that raised a level. Facts:

| Rule | Native owner |
|---|---|
| Account older than N years | the time in the account's UUIDv7 id |
| More than N loyalty points | `game_account_loyalty_projections` |
| Sum of all Character levels more than N | Character roots and progression state |
| Level N with all four / all five vocations; with the monk | highest level per base vocation over the account's Characters |

| Id | Badge | Requirement |
|---|---|---|
| 1-5 | Fledgeling, Veteran, Senior, Ancient, Exalted Hero | account older than 1/5/10/15/20 years |
| 6-8 | Tibia Loyalist (Grade 1-3) | more than 100/1000/5000 loyalty points |
| 9-11 | Global Player (Grade 1-3) | level sum more than 500/1000/2000 |
| 12-14 | Master Class (Grade 1-3) | level 100/250/500 with Druid, Knight, Paladin and Sorcerer |
| 22-24 | Extended Master Class (Grade 1-3) | level 100/250/500 with all five vocations |
| 25-27 | Enlightened Student (Grade 1-3) | a Monk at level 100/250/500 |

Ids 1-14 are Crystal's (`game.cpp:293-306`). Ids 15-21 are Crystal's
tournament badges; Tournaments were sunset in Global and Oteryn has no
tournament owner, so they are not admitted and never granted.

## Character tab request

Command type 388 carries `tab:u8`, `offset:u16`, `limit:u8` (1-50) and an
optional 16-byte target Character id: 4 or 20 bytes. Without a target it reads
the requesting session's Character. A foreign target is disclosed only under the
`CHARACTER_INFO_V1` rule: the viewer's account recorded accepted VIP consent
(`consent=2`) for it on the same World. The read is a projection of the native
owners; it writes nothing.

| Tab | Name | Native owner |
|---|---|---|
| 1 | General stats | progression state, build state, last persisted vitals |
| 2 | Recent deaths | death receipts; Character killers from unjustified points and guild war frags |
| 3 | Recent PvP kills | unjustified points (status 1) and guild war frags (status 2) |
| 4 | Achievements | account achievements, paged by key |
| 5 | Outfits and mounts | account appearance unlocks, paged by key |
| 6 | Store summary | blessings, permanent prey slots, prey wildcards, hirelings |
| 7 | Inspection | equipment slots and their item instances |

Tabs 2 and 3 page the last 30 days (`HISTORY_WINDOW_MS`), newest first; tabs 4
and 5 page by key. Tabs 1, 6 and 7 ignore the page.

## Character tab reply

`tab:u8`, then the tab body. Integers are little-endian; text is `len:u8` and
UTF-8 unless noted. At most 16384 bytes.

| Tab | Body |
|---|---|
| 1 | `experience:u64, level:u32, magic_level:u16`, seven `u16` skills (fist, club, sword, axe, distance, shielding, fishing), `stamina_minutes:u16, offline_training_minutes:u16, vitals_known:u8, health:u32, mana:u32` |
| 2 | `count:u16`, then `at_unix_ms:i64, cause:text` |
| 3 | `count:u16`, then `at_unix_ms:i64, status:u8, description:text` |
| 4, 5 | `total:u16, count:u16`, then `key:text` |
| 6 | `count:u8` blessing keys as text, then `permanent_prey_slots:u8, prey_wildcards:u8, hirelings:u8` |
| 7 | `count:u8`, then `slot:u8, definition_len:u16, definition:UTF-8, quantity:u32` |

The death cause follows Crystal `player_cyclopedia.cpp:72-80`: `Died at Level
{level}`, and ` by {killer}` with the known killers joined by ` and ` (Crystal
names at most the last-hit and the most-damage killer; Oteryn names every
Character its PvP owners recorded for the death, in name order). The PvP kill
description follows Crystal `player_cyclopedia.cpp:134`: `Killed {victim}.`;
the status values are Crystal's `game_definitions.hpp:107-111`. Crystal reads
70 days of kills (`player_cyclopedia.cpp:94`); Oteryn keeps the 30-day window
`CHARACTER_INFO_V1` already uses for both histories.

Not admitted, because no native owner records them:

- Offence, Defence and Misc tabs (Crystal `protocolgame.cpp:5016`, `:5197`,
  `:5265`): live combat aggregates of equipment, imbuements and the wheel.
- Monster killers in a death cause and the "most damage" killer: the death
  receipt records no killer.
- Justified (0), assisted (3) and arena (4) kill statuses: only unjustified
  kills and guild war frags are recorded.
- General stats speed, capacity, maximum health and mana, soul, food and
  skill percentages: live state.
- The outfit/mount split of appearance unlocks: the keys are opaque.
- Store summary XP boosts, house items and purchase history: no owner.
