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
