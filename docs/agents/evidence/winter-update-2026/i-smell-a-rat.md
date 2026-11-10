# Winter Update 2026: I Smell a Rat (quest, hunting grounds, bosses, gear)

Units: WU26-24 to WU26-28 (and the shared WU26-29, WU26-30 in the README).

## 1. Official rules

### Quest (thread 563, Game Content Designer #60914, replies #61011, #61102)

- Investigation of a rat conspiracy; exploration, puzzles, NPCs. Start: **Hushwell
  Asylum** on the road from Thais to Kazordoon (talk to a patient).
- #61011: gathering **musk** from summoned rats is allowed (tool: musk catcher 54769).
- #61102: **Greed Marks** are earned in the quest, not handed out; **30 Greed Marks** buy a
  shortcut; players should progress in Rattica before leaving.

### Hunting grounds (thread 562, #60911)

- **Rattica Outskirts**: mostly giant rats; introduction area, also huntable.
- **Rattica Warrens**: spellcasting hoarder rats and ranged greedcarrier rats; comparable to
  the Hyaena Lairs.
- **Rattica Underbelly**: hoarder and greedcarrier rat swarms; comparable to the Hyaena
  Lairs.
- **Greed Dimension** (entered after defeating the boss): amass greed for the **Greed
  Monger**; difficulty roughly Asura Palace.
- **Frenzy** (new monster mechanic): a frenzied monster or boss deals more damage the fewer
  hit points it has; shown by a new icon on the creature; timing and strength differ per
  creature; the giant rat is the most common user.

### Bosses (thread 564, #60916)

- Temple of Greed encounters for **teams of two**.
- **Lever cooldown 20 h applies only to the player who pulls the lever.** A player on lever
  cooldown may still join another player's attempt as the second participant, without
  limit.
- **Reward cooldown 20 h applies to all participants**, separate from the lever cooldown;
  extra fights within it give no repeated rewards.

### Gear (thread 561, #60909)

Level 150, upgrade classification 3.

| Id | Item | Voc | Values |
|---:|---|---|---|
| 55504 | ratguard (shield) | Knight | defence 50; 1 imbuement slot; imbuement exclusion: Powerful Lich Shroud; physical +3%, death +3%; augments Shield Bash critical hit chance +10% and critical hit damage +20% |
| 55494-55498 | black / soil / verdant / dusk / pearl rat quiver | Paladin | capacity 8; energy protection 0%-3% (range as published) |
| 55506 | arcane ratbook | Sorcerer | defence 30; 1 imbuement slot; physical +2%; magic level +3, death magic level +1; augment Death Echo critical extra damage +8% |
| 55505 | verdant ratbook | Druid | defence 30; 1 imbuement slot; physical +2%; magic level +3, earth magic level +1; augment Forked Thorns critical extra damage +8% |
| 55433 | rat claws | Monk | attack 40; defence 16; elemental bond energy; 2 imbuement slots |

The thread names the paladin quiver "Dark"; the client object is "black rat quiver".
`rodent quiver` 55750 exists in the client but is not in the thread.

## 2. Client evidence

- Quest line 192 "I smell a rat".
- Creatures (f1): giant rat 2896 (looktype 1993), greedcarrier rat 2897 (1994), hoarder rat
  2898 (1995), greed elemental 2899 (1998), greedweaver 2900 (1999), greater greed 2901 (2000).
- Bosses (f5): Rathilda 2867, Ratasha 2868, Rathena 2869 (looktype 1997), Rattila 2870 (1996),
  The Greedking 2871 (2001).
- Achievements (f3): 619 Ratvocat, 620 Feeling Lucky ("tried your luck 100 times"), 621 Just
  One More, 630 Riding in Style, 631 Riding in Luxury, 632 Riding in Prestige, 633 Rat Pack.
- Mount keys 55626-55628 (cheesy luxury / prestigious / stylish ratmobile key); collectible
  rats 55655, 55898-55901; boss bags 55652-55654, 56022-56024 (Ratasha, Rathilda, Rathena),
  55855/55856/55858, 56025-56027 (Greedking bag, equipment bag, jewel bag); labyrinth
  voucher 55825; piece of ratwear 55749.
- Valuables Yasir buys: rat teeth 150, rat ears 350, rat tail 250, hoarder claw 600, greed
  gland 800, miasmic pustule 400, virulent pustule 1,500, almost a treasure 1,200, rat family
  painting 3,500, pact of greed 8,000, condensed greed 4,000, eye of greed 600, cheese of
  rot / pestilence / plague 15,000 each, wheel of cheese 75,000, Greedkings crown 300,000,
  Greedkings claw 150,000.
- Soul cores 55792-55797.
- No frenzy icon id is visible in this diff (creature icons are not appearances); three new
  effects 350/351/353 are unattributed.

## 3. What Oteryn has today

- Rats: Rat and Cave Rat exist (`content/creatures/definitions/creatures-01000-01499.json`,
  `creatures-00000-00499.json`); **no giant rat**, no greed creatures, no Hushwell Asylum,
  Rattica, Temple of Greed or Greed Monger anywhere in `content/`, `crates/` or `tools/`.
- Frenzy: **absent**. Monster damage to a player is scaled only by the Elite bounty x2 and
  the prey reduction before it is applied
  (`crates/gameplay/dispatch/src/gameplay_transport/attack/player_block.rs:332`-`:345`), with
  no factor from the attacker's own HP; the only HP threshold
  primitive is `below_health_threshold`
  (`crates/gameplay/encounter-rules/src/thresholds.rs:4`-`:6`), used for boss heals.
  Creature icons on the wire: only party shield / skull
  (`crates/protocol-oteryn/src/party_skull.rs:15`-`:19`).
- Boss rooms: one cooldown for every participant, checked at admission and set for all
  (`crates/gameplay/encounter-rules/src/boss_room.rs:43`-`:92`: every participant must be
  past `cooldown_until_ms`; the plan returns one `cooldown_until_ms`). No separate lever and
  reward cooldowns.
- Gear model: all fields supported (see `banners-return.md` section 3), including elemental
  magic level points (`crates/gameplay/content/src/content/reference_playable.rs:355`-`:369`,
  `DeathMagicLevelPoints`, `EarthMagicLevelPoints`, `ElementalBond`) and augment kinds
  `CriticalHitChance` and `CriticalExtraDamage`
  (`crates/gameplay/content/src/content/item_spell_augments.rs:9`-`:17`). There is no
  separate "critical hit damage" augment kind. Shield Bash has source-import conflict rows
  (`content/abilities/source-imports/unavailable-spells.jsonl:223`, `:396`); whether it is
  castable at runtime was not established by this read, and the ratguard augments need a
  live Shield Bash consumer.

## 4. Gap as implementation units

### WU26-24 Rat gear (class C, S)

- Owned files: item definition shards for 55433, 55494-55498, 55504-55506 under
  `content/items/definitions/`, augment bindings; pin refresh.
- Data: section 1 table; level 150; classification 3; ratguard imbuement exclusion.
- Prerequisite: WU26-30 asset import.
- Acceptance: `rat_gear_records_match_thread_561`, `pin-check` green.
- Open: whether "critical hit damage +20%" maps to `CriticalExtraDamage`; quiver protection
  per variant.

### WU26-25 Frenzy monster mechanic and icon (class A, M)

- Owned files: monster damage path (`crates/gameplay/dispatch/src/gameplay_transport/attack/player_block.rs`
  and the monster spell damage paths, `crates/gameplay/monster-ai/`),
  creature profile field in `crates/gameplay/content/src/content/project/v2/creature.rs`,
  wire for a creature status icon in `crates/protocol-oteryn/` (claim a capability), client
  overlay in `crates/client-ui` / `apps/client`.
- Rule: a creature with a frenzy profile deals extra damage scaled by missing HP; the
  profile sets when it starts and how strong it is; the creature shows a frenzy icon while
  active.
- Acceptance: `frenzy_damage_rises_as_hp_falls`, `non_frenzy_creature_damage_unchanged`,
  `frenzy_icon_sent_when_active` (protocol golden).
- Open: curve and thresholds (per creature, not published).

### WU26-26 Rattica and Greed creatures (class C, M; blocked on stats)

- Owned files: creature, behavior, loot, presentation and bestiary records; soul cores
  55792-55797.
- Known: names, race ids, looktypes; giant rat uses frenzy (WU26-25).
- Blocked: HP, XP, abilities, loot.

### WU26-27 Lever-operator cooldown and separate reward cooldown (class B, M)

- Owned files: `crates/gameplay/encounter-rules/src/boss_room.rs`, the lever admission
  writer and its catalogue (`crates/gameplay/durability/src/durability/activity_handoff/catalogue.rs`,
  `content/encounters/bosses/`), boss reward path, migration if a reward-cooldown record is
  new (claim the number).
- Rule: a room policy flag `lever_only_cooldown`: only the lever puller must be off and
  receives the 20 h lever cooldown; other participants may be on lever cooldown; every
  participant has a 20 h reward cooldown and gets no reward while it runs.
- Acceptance: `lever_only_cooldown_lets_cooled_down_partner_join`,
  `reward_cooldown_blocks_second_reward_within_twenty_hours`,
  `default_rooms_keep_all_participant_cooldown`.

### WU26-28 Quest line 192, Temple of Greed bosses, Greed Dimension (class C, L; blocked on map)

- Owned files: quest definitions and missions, Hushwell Asylum and Rattica areas, NPCs
  (patients, Greed Monger), encounters for Rathilda, Ratasha, Rathena, Rattila, The
  Greedking (two players, WU26-27 policy), Greed Marks (30 for the shortcut), mount keys and
  ratmobile mounts, collectible rats, achievements 619-621 and 630-633.
- Blocked: map, mission texts, boss mechanics and stats, Greed Mark sources and the Greed
  Dimension rules.

## 5. Open questions

1. Frenzy curve per creature and the icon's wire value.
2. Greed Mark sources and spends beyond the 30-mark shortcut; Greed Monger exchange.
3. What "tried your luck" (achievements 620 / 621) refers to (likely a greed gamble).
4. Ratguard "critical hit damage" augment kind; quiver energy protection per variant.
