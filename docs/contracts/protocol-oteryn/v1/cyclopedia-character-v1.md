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
(`consent=2`) for it on the same World. Tabs project native owners. A self query
for tabs 8–10 first refreshes the durable combat observation under the current
session fence; a foreign query reads its most recent permitted observation.

| Tab | Name | Native owner |
|---|---|---|
| 1 | General stats | progression state, build state, last persisted vitals |
| 2 | Recent deaths | death receipts; Character killers from unjustified points and guild war frags |
| 3 | Recent PvP kills | unjustified points (status 1) and guild war frags (status 2) |
| 4 | Achievements | account achievements, paged by key |
| 5 | Outfits and mounts | account appearance unlocks, paged by key |
| 6 | Store summary | blessings, permanent prey slots, prey wildcards, hirelings |
| 7 | Inspection | equipment slots and their item instances |
| 8 | Offence | persisted observation of current combat and equipment owners |
| 9 | Defence | the same observation, with live defence and protection facts |
| 10 | Misc | the same observation, including blessings and active Wealth Duplex |

Tabs 2 and 3 page the last 30 days (`HISTORY_WINDOW_MS`), newest first; tabs 4
and 5 page by key. Tabs 1, 6, 7, 8, 9 and 10 ignore the page.

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
| 8–10 | `observed_at_unix_ms:u64, character_revision:u64, equipment_revision:u64, count:u8`, then the ordered stat entries described below |

The death cause follows Crystal `player_cyclopedia.cpp:72-80`: `Died at Level
{level}`, and ` by {killer}` with the known killers joined by ` and ` (Crystal
names at most the last-hit and the most-damage killer; Oteryn names every
Character its PvP owners recorded for the death, in name order). The PvP kill
description follows Crystal `player_cyclopedia.cpp:134`: `Killed {victim}.`;
the status values are Crystal's `game_definitions.hpp:107-111`. Crystal reads
70 days of kills (`player_cyclopedia.cpp:94`); Oteryn keeps the 30-day window
`CHARACTER_INFO_V1` already uses for both histories.

## Combat observation (tabs 8–10)

These are current effective attributes, following Crystal 201189f
`protocolgame.cpp:5016-5325`, rather than lifetime damage counters. The native
wire schema has no dependency on Crystal's client packet layout. Each stat
entry is `key:u8, known:u8`, followed by `value:i64` only when `known=1`.
`known=0` means unavailable; it must not be displayed as zero. Every tab sends
its complete key list in ascending order, including unavailable fields. A
decoder rejects missing, reordered, duplicate or foreign keys, a knowledge
tag other than 0/1, truncated values and trailing bytes. Revisions are
positive. The complete reply remains bounded by `MAX_TAB_RESULT_BYTES`.

Ratios use parts per million: `1_000_000 = 100%`, `50_000 = 5%`. Points and
seconds are integers. The signed fields are the imbuement residuals, attack
skill/fight-mode contributions and elemental protection; the other fields
reject negative values. Element ids are Crystal's physical=0, fire=1,
earth=2, energy=3, ice=4, holy=5 and death=6 (`utils_definitions.hpp:36-50`).
Skill ids use the donor's display ids: shielding=6, distance=7, sword=8,
club=9, axe=10 and fist=11.

| Tab | Keys in order | Unit |
|---|---|---|
| 8 | 1 critical chance; 2 intrinsic; 3 equipment; 4 imbuement; 5 wheel; 6 concoction | ratio |
| 8 | 7 critical extra damage; 8 intrinsic; 9 equipment; 10 imbuement; 11 wheel | ratio |
| 8 | 12 life leech; 13 equipment; 14 imbuement; 15 wheel | ratio |
| 8 | 16 mana leech; 17 equipment; 18 imbuement; 19 wheel | ratio |
| 8 | 20 Onslaught; 21 equipment; 22 amplification contribution; 23 cleave | ratio |
| 8 | 24–30 perfect-shot damage at ranges 1–7; 31 flat damage/healing; 32 attack total; 33 equipment attack | points |
| 8 | 34 attack skill id | id |
| 8 | 35 attack skill contribution; 36 fight-mode contribution | signed points |
| 8 | 37 attack element | id |
| 8 | 38 converted damage | ratio |
| 8 | 39 converted element | id |
| 8 | 40–46 ranged accuracy at distances 1–7 | ratio |
| 9 | 64 dodge; 65 equipment; 66 amplification contribution; 67 wheel | ratio |
| 9 | 68 magic-shield capacity; 69 direct capacity bonus | points |
| 9 | 70 capacity percentage bonus | ratio |
| 9 | 71 physical reflection; 72 armor; 73 mantra; 74 defence; 75 equipment defence | points |
| 9 | 76 defence skill id | id |
| 9 | 77 shielding skill; 78 wheel defence | points |
| 9 | 79 mitigation; 80 equipment; 81 skill; 82 wheel | ratio |
| 9 | 83 physical; 84 fire; 85 earth; 86 energy; 87 ice; 88 holy; 89 death protection | signed ratio |
| 10 | 100 Momentum; 101 equipment; 102 amplification contribution; 103 wheel | ratio |
| 10 | 104 Transcendence; 105 equipment; 106 amplification contribution; 107 Amplification | ratio |
| 10 | 108 held regular blessings; 109 maximum regular blessings | count |
| 10 | 110 Wealth Duplex time remaining at the observation | seconds |
| 10 | 111 other active concoctions | count; currently unavailable |

The server captures an observation during the existing production weapon
refresh and on a self query. It reads the current equipment and imbuements
through their existing owners, then checks the actor/session lease, pinned
content, and the installed combat effects. The durable writer re-proves
recovery, node, session/connection/lease and scope generations, independent
content activation, and exact equipment and Character revisions in one
transaction. It updates one bounded row per Character; it does not change
combat values or advance the Character revision.

The row stores the input digest and a monotonic sample revision as its
receipt. An identical retry at the same owner time, session, lease and
connection returns that stored revision without updating the row. A changed
retry, an older observation, stale authority, changed equipment or malformed
stat metadata is refused. A new session generation can replace the sample
only after the current fences pass. Blessing count and Wealth Duplex expiry
are read in the same snapshot transaction. Framed command replay returns the
original response without performing a new query. Offline inspection includes
the original timestamp and revisions so the client can identify stale data.

### Current owner coverage and approved differences

[Coordinator answer to Q5 in PR #2782](https://github.com/Oteryn/Oteryn-Game/pull/2782#issuecomment-6547429543)
approves showing the current native combat owners and representing missing
ones explicitly. This change does not introduce the missing combat mechanics.

| Facts | Native observation and Crystal source |
|---|---|
| Critical chance/damage, leech | Installed `EquippedOffence` with its existing intrinsic 5%/10% and `u16` saturation. Wheel life/mana contributions are included when the live wheel owner supplies them. Crystal `protocolgame.cpp:336-399`. |
| Source breakdown | Equipment-only projection and the installed imbuement residual are reported separately. The native equipment owner sums contributing worn items; Crystal's equipment column uses the left-hand item plus proficiency. This existing ownership difference is retained. |
| Onslaught, Amplification | The configured tier curves and amplification already consumed by `EquippedOffence::fatal`; zero for a known zero tier. Crystal `item.cpp:194-204,230-240`, `config.lua.dist:219-221,232-234`, `protocolgame.cpp:10441-10474`. |
| Cleave, perfect shot, reflection | Existing installed offence values, retaining their native units and range slots. Crystal `protocolgame.cpp:5036-5041,5220`. |
| Flat damage/healing | Existing `level_base_damage_healing` combat formula. Crystal's display delegates to `player.cpp:513-540`; the native accepted level curve remains authoritative for native combat. |
| Weapon, element and accuracy | Current native melee/ranged loadout and the same live attack skill and hit-chance function used by attacks. Distance-only observations are unavailable for melee; absent or unreadable skills remain unavailable. Crystal `protocolgame.cpp:5048-5171`. |
| Armor, mantra, defence | Current installed armor and hand defence, current native mantra projection, and live defending skills. Defence uses the same native bound at the donor display's post-swing fight-mode factor (`player.cpp:722-740`). Attack-only proficiency skill bonuses do not inflate incoming defence. Crystal `protocolgame.cpp:5222-5233`, `player.cpp:638-711,12772-12787`. |
| Elemental protection | The protection installed on the real combat owner, including its existing additive composition. Crystal `protocolgame.cpp:218-301` composes absorption multiplicatively; that separate mechanic is unchanged. |
| Blessings, concoctions | Existing durable regular blessings, excluding Twist of Fate, and active Wealth Duplex expiry rounded up to seconds. Crystal `protocolgame.cpp:5291-5320`; only Wealth Duplex has a native concoction effect owner. |

The following remain explicitly unavailable: critical wheel/concoction
contributions without a native consumer; donor attack-total/skill/fight-mode
display decomposition and wand display attack; dodge/Ruse, Momentum and
Transcendence; item magic-shield capacity modifiers; mitigation and its
breakdown; wheel defence; and non-Wealth-Duplex concoctions. Native combat
owners also differ from Crystal in shield/spellbook equipment bonuses
(Crystal +30%/+60%, `player.cpp:690-704`), mantra slot inclusion (native Extra
contributes), and the source breakdown noted above. These are recorded in
the PR's Left out section rather than implemented by the display layer.

## Other fields not admitted

No native owner currently records these facts:

- Monster killers in a death cause and the "most damage" killer: the death
  receipt records no killer.
- Justified (0), assisted (3) and arena (4) kill statuses: only unjustified
  kills and guild war frags are recorded.
- General stats speed, capacity, maximum health and mana, soul, food and
  skill percentages: live state.
- The outfit/mount split of appearance unlocks: the keys are opaque.
- Store summary XP boosts, house items and purchase history: no owner.
