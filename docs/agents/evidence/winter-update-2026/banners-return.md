# Winter Update 2026: Banner's Return (quest, hunting grounds, boss, gear)

Units: WU26-18 to WU26-23 (and the shared WU26-29, WU26-30 in the README). Ledger rows:
W15a/W15d (quest log and per-quest population), W10 (hazard-style area rules), boss rows.

## 1. Official rules

### Quest (thread 566, Game Content Designer #60919, reply #61093)

- Captain (Marie-Denise) Banner hunts whoever destroyed her unit, the Green Lancers. The
  player fights the Omega threat on the **Onyx Massif**, a volcanic wasteland around the
  volcano **Red Gullet**, and finally faces Banner's nemesis.
- Start: Bounac, ask **Onfroi** about "banner". Requires the **Order of the Stag** quest and
  having met Captain Banner (who names Onfroi); revisiting her on the Isle of Ada is not
  needed if the warlock is already defeated. TibiaWiki changelist (from the teaser): quest and
  areas need **Premium**; non-linear progression with routes and shortcuts unlocked as you
  progress; new outfit and achievements.
- Reply #61093: a ladder will lead north in a later patch; Builder kill counting inside
  parties is being checked (so Builder kills are counted per player for progress).

### Hunting grounds (thread 568, #60922)

- **Onyx Massif**: entry area, moderate density, omega knights; slightly easier than
  Bloodfire Gorge (Isle of Ada).
- **Darkfire Gorge**: most of the island, higher density, open; slightly harder than
  Bloodfire Gorge.
- **Red Gullet**: stronger omega knights called **Builders**; lower density, strongest
  offence and defence. Difficulty depends on the player's **scorch level**, raised by
  challenging the **onyx beast** after killing the required number of Builders in the
  Gullet. Scorch level sets how many **blood cinders** a kill gives in the Gullet.
  Creatures in the Red Gullet have **no regular loot**; blood cinders are exchanged at the
  **Ash Sentry** NPC for valuables, monster traps, decorations, Gullet-exclusive rings and
  more.

### Boss (thread 569, #60924)

- **Banner's Nemesis**, an **Archfoe** boss, challengeable every **20 hours**, with
  selectable **difficulty** (higher difficulties add mechanics). The arena is hit by the
  Gullet's fire spit. **Captain Banner fights alongside** and must survive to the end for the
  quest to conclude.
- New gear drops with a **pity tracker that applies only until the first drop**; after that
  only the base chance plus the usual boss bonus and boss-scaling loot apply.
- Achievement 623 *Enemy Mine* names the nemesis **Boson** and his citadel, the Red Gullet.

### Gear (thread 570, #60928, fix #60964)

Level 700, upgrade classification 4. Boots for knight, paladin, monk; legs for sorcerer,
druid. Boots grant speed (#60964 corrected the description from "go strength" to "speed").

| Id | Item | Voc | Arm | Imbue | Protection | Skill boost | Augment |
|---:|---|---|---:|---:|---|---|---|
| 55229 | corrupted moonsilver flamestrider (legs) | Sorcerer | 12 | - | holy +2%, ice +9% | magic level +4 | Death Echo critical extra damage +8% |
| 55230 | corrupted moonsilver ashstrider (legs) | Druid | 12 | - | holy +2%, earth +9% | magic level +4 | Forked Glacier critical extra damage +8% |
| 55131 | corrupted moonsilver sabatons (boots) | Knight | 5 | 1 | holy +2%, physical +7%, energy +5% | speed +20, sword/axe/club +2 | Groundshaker critical extra damage +12% |
| 55132 | corrupted moonsilver scorchsteps (boots) | Paladin | 4 | 1 | holy +2%, physical +5% | speed +20, distance +2 | Divine Barrage critical extra damage +8% |
| 55228 | corrupted moonsilver footwraps (boots) | Monk | 3 | 1 | holy +2%, physical +4% | speed +20, fist +4 | Thousand Fist Blow(s) critical extra damage +12% |

## 2. Client evidence

- Quest line 195 "Banner's Return" (staticdata f6).
- Creatures (f1): omega bulwark 2883, omega bastion 2884, omega battler 2885, omega
  moonbreaker 2886, omega cinderflayer 2887, omega rampart 2888 (looktypes 1988-1990;
  the last three are coloured variants, likely the Builders).
- Boss (f5): Banner's Nemesis 2902, looktype 1992.
- Achievements (f3): 618 Forged in Conflict (highest scorch), 623 Enemy Mine (2 points),
  624 Omega Knight (studied Omega plans; blueprint 55776).
- Ash Sentry sale data (currency "Blood Cinder", location Darkfire Gorge): four rings of
  the Gullet (draining 55236, siphoning 55242, rending 55244, warding 55555) 5,000 each;
  volcanic charge 55246 500; dark shackles 55251 750; omega seal 55338 50; blood onyx 55339
  50; blood onyx core 55340 250; piece of corrupted moonstone 55411 15; onyx turmaline 55412
  60; six unnamed decorations 55549-55554 at 25,000-100,000; omega knight arms seal 55775
  120,000.
- Valuables Yasir buys: red gullet metal 1,600; broken gullet lenses 3,750; gullet moonstone
  splinter 3,100; omega seal 12,700; blood onyx 12,200; blood onyx core 64,000; broken blood
  onyx amulet 214,000. Corrupted moonstone (15,000) and onyx turmaline (68,000) are bought
  by 18 jewel traders (Augustin ... Yonan).
- Quest items: orbs 55777-55780, empty orb 55824, blueprint 55776, ancient key 55557.
- Soul cores 55786-55791 for the six omega creatures.
- Full id list: `client-delta.md`.

## 3. What Oteryn has today

- Prerequisite quest: `oteryn:quest.the_order_of_the_stag_quest` is `script_only` /
  waiting data, not in the quest log (`content/quests/definitions/quests-00200-00299.json`);
  talk rules `content/dialogues/quest-rules/quest-batch3-c-the_order_of_the_stag.json`
  (Kesar the Younger names Captain Banner); the Court Warlock death source is disabled
  (`content/quests/order-of-the-stag-death.json`, `runtime_enabled: false`). So the
  prerequisite itself is not completable today.
- NPCs: Onfroi exists (`oteryn:npc.onfroi_day` / `_night`,
  `content/npcs/definitions/npcs-00500-00999.json`), Yasir exists
  (`content/npcs/definitions/npcs-01000-01281.json`). **No** Captain Banner, Ash Sentry or
  Boson records.
- Areas: Isle of Ada and Bloodfire Gorge exist only as hunting-place records without anchor
  or footprint (`content/world/areas/hunting-places/hunting-places-00000-00444.json`). No
  Onyx Massif, Darkfire Gorge or Red Gullet anywhere in `content/`, `crates/` or `tools/`.
- World map: B3 regions converted from the pinned CrystalServer `world.otbm`
  (`tools/content-schema/world-authoring/convert_world_base.py`); new land can be authored
  by the world-authoring tools (pattern `newhaven_tutorial_land.py`) but the item-level map
  of the new island is in no available source. The test client ships only minimap
  colour tiles: chunks around x 31872 / y 32000 and x 33920 / y 32512 changed on several
  floors (see README).
- Item model supports every gear field: level and vocation
  (`crates/gameplay/content/src/content/reference_playable.rs:470`-`:480`), protections
  (`:517`-`:521`), skill boosts incl. speed (`:355`-`:369`), imbuement slots (`:145`),
  upgrade classification (`crates/gameplay/domain/src/domain/item_definition.rs:7`-`:10`),
  spell augments `CriticalExtraDamage` in hundredths of a percent
  (`crates/gameplay/content/src/content/item_spell_augments.rs:13`).
- Bosses: lever rooms with a 20 h cooldown applied to every participant
  (`crates/gameplay/encounter-rules/src/boss_room.rs:43`-`:92`;
  `content/encounters/bosses/crystal-lever-candidates.json`); difficulty selection wire
  (`crates/protocol-oteryn/src/boss_difficulty.rs:6`-`:24`) with per-boss maxima only and
  **no scaling formula**; Archfoe exists as a Bosstiary category
  (`rulesets/progression/bosstiary/categories.json:3`). **No pity tracker** (no drop-pity
  code in `crates/`). No allied NPC fighting in an encounter was found by this read
  (UNVERIFIED; check `crates/gameplay/encounters` before WU26-22).
- NPC services use item-referenced currencies (`ProjectV2ServiceOffer.currency`,
  `crates/gameplay/content/src/content/project/v2.rs:660`); "Blood Cinder" has no item id in
  the new objects, so it is probably a counter currency, which Oteryn's service model does
  not express.

## 4. Gap as implementation units

### WU26-18 Corrupted moonsilver gear (class C, S)

- Owned files: the item definition shards holding ids 55131, 55132, 55228, 55229, 55230
  under `content/items/definitions/`, augment bindings, `content/items/aliases.json` if names
  are aliased; pin refresh.
- Data: table in section 1; level 700; classification 4; vocations as listed.
- Prerequisite: the asset import of client 15.33.c68982 (WU26-30) so the ids exist.
- Acceptance: `corrupted_moonsilver_records_match_thread_570` (one row per item: armor,
  protections, skills, imbuement slots, augment spell and value 800 / 1200),
  `pin-check` green.

### WU26-19 Omega creatures and soul cores (class C, M; blocked on stats)

- Owned files: `content/creatures/definitions/` (new shard or the last shard), behaviors,
  loot, presentations, bestiary entries, soul core items 55786-55791.
- Known: names, race ids 2883-2888, looktypes and colours (client). Unknown: HP, XP, armour,
  abilities, resistances, loot (TibiaWiki has no pages yet; Red Gullet creatures have no
  regular loot).
- Acceptance: `omega_creatures_have_client_race_ids_and_looktypes`; balance numbers only
  from a cited source.

### WU26-20 Red Gullet economy: scorch level, blood cinders, Ash Sentry (class C, M)

- Owned files: a new gameplay module for per-character scorch level and cinder balance
  (crate chosen by the owner of hazard-like area rules), migration (claim the number), Ash
  Sentry NPC and service records, kill-reward hook in
  `crates/gameplay/dispatch/src/gameplay_transport/kill_reward.rs`.
- Rules (thread 568): kills in the Red Gullet give cinders scaled by scorch level and no
  regular loot; after the required Builder kills the player can challenge the onyx beast
  to raise the scorch level; Ash Sentry sells the listed items for cinders (prices from
  the client, section 2).
- Persistence: new per-character tables for scorch level and cinder ledger
  (session-generation fenced like the other character balances).
- Acceptance: `red_gullet_kill_grants_cinders_by_scorch_level_and_no_loot`,
  `onyx_beast_raises_scorch_after_required_builder_kills`,
  `ash_sentry_sells_ring_for_five_thousand_cinders` (PG).
- Blocked in part: Builder kill threshold, number of scorch levels and cinders per kill are
  not published; area membership needs the map (WU26-23).

### WU26-21 Boss drop pity tracker (class C, M; generic)

- Owned files: boss loot roll (find the boss reward owner in
  `crates/gameplay/encounters/` / `kill_reward.rs`), a new per-character, per-boss pity
  record, migration.
- Rule: until a character's first drop of the tracked gear from that boss, each kill
  raises its chance; after the first drop only the base chance (plus boss bonus and
  boss-scaling loot) applies.
- Acceptance: `pity_raises_chance_until_first_drop_then_stops`,
  `pity_state_survives_restart` (PG).
- Open: increment and cap of the pity chance (not published).

### WU26-22 Banner's Nemesis encounter (class C, L; blocked on map and stats)

- Owned files: encounter definition in `content/encounters/definitions/`, lever-room row,
  boss creature record (2902), Captain Banner ally, Gullet fire-spit hazard, difficulty
  entry in `crates/protocol-oteryn/src/boss_difficulty.rs` (`boss_maximum`), Archfoe
  Bosstiary binding.
- Rules: 20 h cooldown; difficulty selection with added mechanics; Banner must survive for
  quest completion; drops WU26-18 gear with WU26-21 pity.
- Blocked: arena map, boss and ally stats, difficulty maximum and mechanics per level.

### WU26-23 Quest line 195, areas and achievements (class C, L; blocked on map)

- Owned files: quest definition and missions in `content/quests/`, Onfroi dialogue, Captain
  Banner NPC, area records for Onyx Massif / Darkfire Gorge / Red Gullet, achievements 618,
  623, 624 in `content/achievements/`, the new outfit.
- Rules: start at Onfroi ("banner") with Order of the Stag done and Premium; non-linear
  routes and shortcuts; Builder kill counting per player in parties.
- Blocked: the island map and mission texts; the Order of the Stag prerequisite is itself
  not completable in Oteryn today.

## 5. Open questions

1. Number of Builders per scorch challenge, scorch levels, cinders per kill per level.
2. Whether blood cinders are an item or a character counter.
3. Pity tracker growth and cap; which items are tracked.
4. Banner's Nemesis difficulty maximum and per-level mechanics.
5. Which looktype is the new player outfit (see `client-delta.md`).
6. Effects of the four rings of the Gullet and of volcanic charge / dark shackles (names
   only in the client).
