# qcx48 · The First Dragon Quest

Base `871c1dec7d9963420d455210e4cddc0c9df80d26` from `origin/main`. The
audit covers `oteryn:quest.the_first_dragon_quest`.

## Source order

1. TibiaWiki structured wikitext, captured on 2026-11-17, from facts commit
   `460e1d8c2c0e88c438ae5e54910db00822dc0129`.
   [Quest, curid 80244, oldid 1048348](https://tibia.fandom.com/?curid=80244&oldid=1048348)
   · [Spoiler, curid 80254, oldid 1054086](https://tibia.fandom.com/?curid=80254&oldid=1054086)
2. Crystal `201189f871d23597b82caf88fd611454a2ddefb8` as donor evidence.
   Retained quest evidence includes the NPC scripts and the per-action audit.
   The selected final encounter also cites the older Crystal capture described in
   the M18 notes.
   [Crystal quest scripts](https://github.com/zimbadev/crystalserver/tree/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/the_first_dragon)
   · [Retained action audit](../evidence/quests-4-20261102/the_first_dragon.json)
   · [M18 encounter notes](../../reference/content-prep/qmech-season/M18.md)

## Comparison against the base

| Part | Global facts | Repository and donor comparison |
| --- | --- | --- |
| Requirements and opening | Premium and level 40. Level 200 is recommended. Rope and shovel equivalents are route tools. In Service of Yalahar grants Magician Quarter access for Tamoril. Barbarian Test/Buddel gates the Gelidrazah route and New Frontier mission 1 gates the Kalyassa route. Vigintius and the Dead Explorer's Blurred Transcript are optional. | The definition agrees on level and Premium. The selected recipe lists the three access prerequisites together and has no optional inscription/transcript branch or route tool requirements. Its repeat mode is once. The canonical NPC program already uses January 14–February 12 and the two confirmations. It accepts `find` without the earlier topics and resets dragon progress to zero on every acceptance, as does Crystal `tamoril.lua:81,99–107`. The Global carryover of previous-event kills conflicts with unconditional reset. |
| Tazhadur | 200 common Dragons, with previous-event kills retained. The emerald leads to Tazhadur and the Tooth of Tazhadur (donor ID 24940). | The imported mission describes 200 and the chosen recipe describes 20. Crystal's death handler caps the counter at 200. The native First Dragon death counter serves the selected finale and supplies no credit to this trial. UID 35001 entry and the boss-product route are absent at runtime. |
| Kalyassa | Open five distinct Dragon Chests out of 20 for ruby access. Chest rewards are available before talking to Tamoril. All 20 grant Treasure Hunter (`oteryn:achievement/treasure_hunter`). Kalyassa supplies the Horn (24941). | The chosen recipe describes one treasure and the source mission describes 20. There are no runtime chest claims. Crystal checks a threshold of four on an initialized counter, and requires the questline for the portal. The Global five-chest threshold is authoritative. Crystal lists the same treasure names and counts except that the two pearl entries reuse one name, without Global's brown/green distinction. |
| Gelidrazah | Answer Tahmehe, Ishara, Svir in order. The sapphire leads to Gelidrazah and the Scale (24939). | The chosen recipe reduces knowledge to a visit. The retained Crystal NPC script has the quiz and writes `gelidrazah_access = 1`. The base has the NPC definition and quest track but omits the live quiz and Okolnir placement. UID 35004 sapphire admission remains separate from the quiz. |
| Zorvorax | Visit the Kha'labal oasis, Carlin stone circle, and Suntower. The amethyst leads to Zorvorax and the Bones (24942). | The chosen recipe describes one life visit, while the source mission describes three. Crystal binds AIDs 25002–25004 and entrance UID 35003. The three visits, source messages, entrance and boss-product route have no live binding. |
| Permanent lair and final entry | Defeat the four descendants, use the four products on the tile in front of the door, then enter. The Mystic Flame leads through Kizar to the lever. Up to 15 can enter the fight. | The selected runtime uses two synthetic s4 exploration occurrences, level 40 and Premium, and disallows admission after an s5 defeat. Crystal has four item sacrifices, a permanent access track and seasonal UID 24889. The selected encounter shares the 15-slot maximum and uses separate donor-derived timing. The four products and permanent entrance are not connected. |
| Finale | Five groups of three fight Fallen Challengers and use the resulting flames. Then Spirit of Fertility, Dry Flower, Angry Plant and Flower Bowl enable five Somewhat Beatable Dragons. Six linked-HP Dragon Essences precede the boss. Dragon Wardens respawn and the boss relocates. Victory sends the party to the reward room. | The selected runtime sends everyone directly to the final room and spawns the First Dragon. It writes a selected completion/access receipt and returns the group through Boss Activity. Crystal's Essence script removes the group on death rather than modeling the Global linked HP rule. The canonical reward-room transition and Feathers eligibility write are missing. |
| Outfit | Ask The First Dragon for Festive Outfits in the reward room. | The reward callback and NPC placement exist. Crystal grants looks 929/931 with addon mask 0 and uses Feathers 1→2. Oteryn's callback has that gate, but its selected victory supplies neither the reward room nor Feathers 1. Muriel's two add-ons belong to the separate Festive Outfits quest. |
| Final claims and XP | Mask ×1 every five days, Colourful Feathers ×3 every day, backpack ×1 per event/year. No quest XP amount is listed. | The chosen reward is an access receipt. No corresponding item reward claims exist. Crystal has the same contents and quantities, a five-day mask delay and a one-day feather delay. The donor's rolling 365 days for the backpack differs from once per event/year. The retained action audit's summary of one each conflicts with the actual Crystal item array. |
| World callback and placement coverage | The three life sites, twenty treasure chests, four boss entrances, the lair sequence and reward room use distinct world anchors. | The four exit callbacks are active records, selecting UIDs 35005–35008. There is no First Dragon file in `world-callbacks-held`. A scan of all 1,209 B3 regions found none of UIDs 14001–14023, 24889, 35001–35008 or 1066 and none of AIDs 25002–25004. The NPC placement family contains the reward NPC. It omits the Okolnir quiz NPC, Tamoril, Vigintius and Kizar. |

The 20 treasure chest rewards, in Crystal UID order, are shown below. Names and
counts are compared with the Global table. Numeric IDs come from Crystal
`actions_treasure_chest.lua:1–83` and `data/items/items.xml`.

| UID | Reward | Donor item ID × quantity |
| --- | --- | --- |
| 14001 | Giant Shimmering Pearl | 281/282 ×1, see OPEN-FACTS |
| 14002 | Gold Nugget | 3040 ×2 |
| 14003 | Blue Crystal Shard | 16119 ×1 |
| 14004 | Violet Crystal Shard | 16120 ×1 |
| 14005 | Green Crystal Splinter | 16122 ×2 |
| 14006 | Red Gem | 3039 ×1 |
| 14007 | Onyx Chip | 22193 ×3 |
| 14008 | Platinum Coin | 3035 ×3 |
| 14009 | Red Crystal Fragment | 16126 ×2 |
| 14010 | Yellow Gem | 3037 ×1 |
| 14011 | Talon | 3034 ×3 |
| 14012 | White Pearl | 3026 ×2 |
| 14013 | Gold Ingot | 9058 ×1 |
| 14014 | Opal | 22194 ×3 |
| 14015 | Small Diamond | 3028 ×2 |
| 14016 | Green Crystal Shard | 16121 ×1 |
| 14017 | Black Pearl | 3027 ×3 |
| 14018 | Emerald Bangle | 3010 ×1 |
| 14019 | Green Gem | 3038 ×1 |
| 14020 | Giant Shimmering Pearl | 281/282 ×1, see OPEN-FACTS |

Final backpack (2854 ×1) contains Ancient Coin (24390 ×1), Draken Sulphur
(11658 ×1), Seacrest Hair (21801 ×2), Mystical Hourglass (9660 ×2), Gold Token
(22721 ×3), Blue Gem (3041 ×1), Red Gem (×1), Yellow Gem (3037 ×1), Demon
Horn (5954 ×2), Slime Heart (21194 ×2), Energy Vein (×2), Petrified Scream
(10420 ×2), Brimstone Shell (11703 ×2), Deepling Warts (14012 ×2), Wyrm
Scale (9665 ×2) and Hellspawn Tail (10304 ×2). The other two chests are
Porcelain Mask (25088 ×1) and Colourful Feathers (25089 ×3).
[Crystal reward script](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/the_first_dragon/actions_rewards.lua#L1)
· [Crystal item registry](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data/items/items.xml)

## Changes

- The retained Crystal quiz now runs through the existing NPC program dispatcher.
  The questions and answer order follow spoiler revision 1054086. Only the last
  correct answer commits `gelidrazah_access = 1`. An out-of-order answer leaves
  the topic and durable track intact.
- The placement generator now admits Gelidrazah's Thirst at `32275, 31366, z4`,
  the existing donor candidate and the exact Okolnir cell in the spoiler.
- The quest recognizes the complete pre-quiz definition for existing saves,
  including its Festive reward and selected final encounter. The predecessor
  hash was read from the compiled base catalogue at `871c1dec7`. The
  PostgreSQL quiz case seeds a saved quest under that exact hash.

## Blockers

- BLOCKER Descendant portals atomic StepIn admission, room occupation, summon and personal boss cooldown data-global/scripts/quests/the_first_dragon/movements_entrance_teleport.lua:41
- BLOCKER Finale phased party rooms, creature-driven tile morphs and linked Essence HP data-global/scripts/quests/the_first_dragon/creaturescripts_death_dragon_essence.lua:3
- BLOCKER Finale reward-room relocation and claim-entitlement settlement data-global/scripts/quests/the_first_dragon/creaturescripts_death_first_dragon.lua:21
- BLOCKER Annual backpack claim per-event repeat epoch data-global/scripts/quests/the_first_dragon/actions_rewards.lua:52

The existing world callback clock supports elapsed seconds for daily and five-day
reward claims. The Boss Activity path supports the selected finale's separate
cooldown. The blockers above describe the unbound Global route and the missing
shared integrations for it.

## OPEN-FACTS

- The facts specify reward names and quantities. Numeric Global IDs, the
  brown/green pearl mapping onto Crystal UIDs 14001/14020, Red Gem 3039 vs
  36706 for the backpack, and Energy Vein 23508 vs 39547 for the backpack
  remain unverified. Dragon chest Red Gem has a literal donor ID 3039.
- Global cooldowns for the descendant fights and finale, encounter expiry,
  exact reward-chest cells, item appearance IDs and map actions, exact
  non-quiz NPC placements and reward-room coordinates, and a quest XP amount
  are unspecified. The donor advertises ten minutes for descendant rooms and
  schedules a 30-minute cleanup. The Global facts supply neither duration.
- Global party credit rules and event-boundary time zone are unspecified.
  The NPC storage layout and interim state values are donor implementation
  details. The supplied pages do not specify Festive add-on exchange terms.

## PARTIAL

Permanent lair, full descendant routes and product turn-ins, optional transcript,
Tamoril route admission and cross-event kill retention, all twenty treasure
chests and their achievement, life-site placements, full final encounter, and
the reward-room route and claims remain open. The Okolnir quiz unlock is only
the knowledge track. The sapphire portal requires its own admission path.

## Verification

- Baseline PostgreSQL shard, base quest data with the new cases locally mounted,
  0 passed and 2 failed. The quiz failed at the first `yes` with no response
  (35.409 s). The placement case failed on the missing generated Okolnir
  record (25.928 s). Both cases reached the local PostgreSQL fixture.
- Quest authoring `--part checks` passed all 44 commands.
- NPC placement conversion `--check` and Rust formatting passed.
- PG via shard (pgsplit build files applied on 4ce0b5582cdb4a902779927ead162ac980af2f8a). Both dispatch cases passed. The quiz took 31.963 s and the placement took 23.605 s. The saved-state fixture includes the prior receipt required by the character revision chain.
- The same filter without the PostgreSQL admin URL returned Ok for both cases in 0.00 s.
- World pin check passed for one bundle. Its compiled NPC frame contains Gelidrazah's Thirst at the expected Okolnir cell.
- Dispatch library Clippy passed with `-D warnings`.
