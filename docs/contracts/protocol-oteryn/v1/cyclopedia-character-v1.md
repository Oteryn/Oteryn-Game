# CYCLOPEDIA_CHARACTER_V1

Capability 199 carries the Cyclopedia character window tabs that the earlier
`CHARACTER_INFO_V1` summary (capability 97) does not (PAR-P12, Crystal
`protocolgame.cpp` `sendCyclopediaCharacter*`). The server owns every fact; a
request only names the tab. The capability is offered only by a node serving a
World booted from a bundle (`BUNDLE_WORLD_OFFERED_CAPABILITIES`). Without the
capability selected, its commands are refused.

## Badges request

The legacy command-type-387 request has an empty payload. It names the
requesting session's account; this legacy request cannot read another account.

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
UTF-8 unless noted. Legacy tab replies are at most 16384 bytes.

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

### Rich Character achievement pages

Command 388 also accepts opt-in selector 128: `128:u8, page:u8`, optionally followed by the 16-byte target Character. Pages are zero-based, 64 rows each, with a maximum complete list of 16,384 earned facts. Legacy tab request/reply grammars and their bounds are unchanged.

The rich reply is `128:u8, character:[u8;16], name_bytes:u8, name:UTF-8`, followed by the existing AccountAchievementsResult codec. The name is 2–29 bytes. Metadata and earned timestamps come from the activated catalogue and immutable grant-request provenance. The same recovery/session-generation fence and foreign-target VIP consent apply. The total command result budget is 32,836 bytes; legacy replies remain bounded at 16,384 bytes. Unsupported, over-limit or unreadable target data is refused without an invented empty list.

## Account badge visibility extension (T027)

The empty command-387 request and its legacy reply remain unchanged. Prefix
`f0` selects the bounded visibility extension: `f0 00` queries self;
`f0 01 <target:16 bytes>` queries another character under the existing accepted
VIP consent rule; `f0 02 <earned badge:u8> <public:0|1>` sets the requesting
account's visibility. Extra bytes and unknown badge ids are rejected.

The reply is `f0 <earned-count:u8> <earned ids> <hidden-count:u8> <hidden ids>`.
Each list is ascending, unique and bounded by the 20 admitted badges. Hidden
ids must be a subset of earned ids. Other-character queries contain only public
badges and an empty hidden list. Mutations cannot target another account or an
unearned badge. Visibility never changes the permanent award rows. Mutations
are session-fenced and receipted by the command id; replay returns its original
reply without overwriting later visibility edits.

A new client retries only a refused extension self query once as the legacy empty
self query. A legacy reply leaves visibility editing unavailable; mutations and
other-character queries never downgrade. Command-387 bounds are 18 request bytes
and 43 reply bytes when this extension is supported.

## Appearance gallery

Command 388 additionally accepts selector 129 followed by `offset:u16`.
This is a self-only, session-fenced read. It returns available outfits (including
starter choices) and owned mounts from the same qualified cosmetics projection
used for appearance selection. Familiar entries describe the active content
catalogue; they do not assert an account unlock or summon eligibility.

Reply: `129:u8, offset:u16, total:u16, count:u8`, then `count` entries of
`kind:u8, look_type:u32, key:text`. Kinds are outfit=1, mount=2, familiar=3.
Entries are sorted by kind and key, with exactly `min(24, total-offset)` rows.
Look ids must be positive; keys are nonempty UTF-8 of at most 255 bytes without
NUL/newlines. Maximum reply: 6270 bytes. The client resolves these confirmed ids
through its pinned local sprite loader, without packaging additional art.

## Item summary

Selector 130 on command 388 accepts `offset:u16` and reads only the requesting
Character under its current item/session fence. The single statement includes
main backpack and equipment roots, legacy and nested bag edges, depot boxes,
Character inboxes, and World-scoped stash balances. Closed bags are included.

Reply: `130:u8, offset:u16, total:u16, count:u8`, then `count` rows of
`definition_len:u16, definition:UTF-8, inventory:u64, depot:u64, stash:u64, inbox:u64`.
There are exactly `min(24,total-offset)` rows, sorted by definition in byte
order; keys are nonempty, at most 512 bytes, and contain no NUL/newlines.
At least one quantity per row is positive. Maximum reply: 13110 bytes.
Filters on the client apply to the confirmed page; pagination retains the
complete server custody projection and does not require opening containers.

## Death loss forecast

Command 388 accepts the single-byte selector 131 for a self-only forecast under
the current Character/session fence. It is read-only: it neither samples a
future lost-item set nor consumes a blessing or creates a death receipt.

The reply is at most 51 bytes: `131:u8, observed_at_unix_ms:u64, level:u32,
experience:u64, regular_blessings:u8, flags:u8`, followed by a creature-death scenario and an optional
player-death scenario. The latter is present only when flag 16 is set; without
it the reply is 37 bytes and carries no player-death estimates. Each scenario is `experience_lost:u64,
equipment_per_mille:u16, container_per_mille:u16, consumption:u8,
amulet_consumed:u8`. Consumption is keep=0, regular=1, PvP safeguard=2.
Flags are owned Twist=1, current promotion benefit=2, effective red/black skull=4,
Adventurer PvP exemption=8, and published PvP available in this scope=16.
Reserved flags, invalid bool/enumeration tags, chances above 1000, XP loss above
current XP, trailing bytes and truncation are rejected.

The server uses the same XP formula, blessing disposition, item-loss ladders,
Amulet of Loss, vocation/level exemptions and Adventurer owner as normal death
settlement. Promotion includes the current Premium gate. A player-death scenario
is displayed only with a published compatible World policy. XP is the baseline
before attacker-specific Bless reductions; final death settlement re-reads its
facts and determines actual item loss. A lost backpack includes its contents.

## Coherent profile (opt-in selector 132, command 388)

Request: `132:u8` plus optional `target:16 bytes`; absent target selects self.
Reply: `132:u8, character:16 bytes, name_len:u8, name:utf8, badge_count:u8, badge_ids:u8[], equipment_count:u8`, followed by each `slot:u8, definition_len:u16le, definition:utf8, quantity:u32le`. Name is 2–29 bytes, definitions 1–512 bytes, equipment at most 32 unique slots, quantity positive; badges use the admitted catalogue in ascending order without duplicates. Reply is bounded to 17000 bytes.

Identity, permanent account badges and equipped items are read together under the current viewer item/session, node and recovery fences and existing accepted VIP consent rule. Self queries refresh eligible badges first; foreign reads never grant awards. Refusal discloses no profile. Legacy empty command 387 and tab 7 remain unchanged. Native Badges and Inspection use this opt-in snapshot and discard cached facts on target changes or refused refreshes.

## Named spell modifiers (selector 133, command 388)

Request: `133:u8, page:u8, filter:u8`, optionally followed by the target Character's 16 bytes. The low three filter bits select equipment (1), weapon proficiency (2) and Wheel (4); at least one source is required. Bit 3 selects source ordering; its absence selects spell ordering. Other bits are rejected. Filtering and ordering precede pagination; each page contains at most 64 of at most 1024 observed modifiers.

Reply: `133:u8, page:u8, filter:u8, character:16 bytes, total:u16le, stats_len:u16le, stats:bytes, available:u8`, then the modifier snapshot when available. `stats` is the unchanged Offence combat codec, including the observation time and Character/equipment revisions. The reply is bounded to 32000 bytes. The target is identified by `character`; the query target is not echoed separately.

The snapshot has `schema=1:u8, known_sources:u8, source_count:u8`, followed by sources (`kind:u8, equipment_slot:u8, key_len:u16le, key:utf8`), then `row_count:u16le` and rows (`source_index:u8, effect:u8, spell_len:u16le, spell:utf8, value:i64le`). There are at most 32 sources; keys are 1–512 bytes and spell names 1–128 bytes, without control characters. Values are non-negative. Source kinds are equipment (1), proficiency (2), Wheel (3); known-source bits use the request's source masks. An unavailable source is distinct from an observed source with no modifiers. An absent snapshot identifies an observation made without named modifier facts.

Effects 1–6 are base damage, base healing, critical chance, critical extra damage, life leech and mana leech, in hundredths of a percentage point. Effect 7 is an individual cooldown reduction in milliseconds, effect 8 a Wheel spell-augment stage (1–3), and effect 9 a secondary Focus-group cooldown reduction in milliseconds. Individual reductions remain subject to the cast owner's half-base floor. Modifiers describe the observed equipment/allocation; cast, vocation and target conditions still apply.

Production samples read native item declarations, the actual selected weapon-proficiency cast rewards, and the current Wheel projection/stages. Named facts and scalar combat facts are committed together after the existing current session/node/recovery, content and exact-equipment checks; named facts participate in the replay fingerprint. Legacy writers leave the optional column NULL. Reads preserve the existing accepted VIP consent and current viewer fence, including for offline targets. The read model grants no combat, item or cast authority.

Stored modifiers also carry their sample revision. Reads accept them only when that revision matches the scalar sample, so older writers that leave optional columns untouched make prior modifiers unavailable instead of attaching them to a newer observation.

## Vocation in General Stats (capability 487)

Capability 487 `CYCLOPEDIA_CHARACTER_VOCATION_V1` requires 199. When a session
selected it, a General Stats reply (tab 1) appends `vocation_id:u8` after the
unchanged V1 body. Ids follow the spellbook details vocation order: `none`=1,
`druid`=2, `elder_druid`=3, `sorcerer`=4, `master_sorcerer`=5, `knight`=6,
`elite_knight`=7, `paladin`=8, `royal_paladin`=9, `monk`=10, `exalted_monk`=11.
The value is the target's stored build-state vocation, read in the same fenced
transaction as the rest of the tab. Other tabs never carry it, and a session
without 487 receives byte-identical replies. Any reply with an extension is
bounded by `MAX_EXTENDED_TAB_RESULT_BYTES` (32768), within the registry bound
of command 388.
