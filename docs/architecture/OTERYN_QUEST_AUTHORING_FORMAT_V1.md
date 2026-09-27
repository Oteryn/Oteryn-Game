# Oteryn Quest Authoring Format v1: reward claims and door gates

- Date: 2026-09-27
- DecisionStatus: CANDIDATE (owner decisions D32-D33 recorded in §9; slice 1 covers reward chests,
  slice 2 quest, key and level doors)
- DeliveryStatus: OPEN (design draft and offline transcription only)
- ImplementationStatus: NOT_STARTED
- Programme: CW2 B6 quests/interactions (`docs/agents/programs/OTV2_CONTENT_WORLD_BULK_CATALOG_IMPORT_PLAN.md`)
- Open precision gap kept open: `CONTENT-QUEST-01` (`ARCHITECTURE_ANALYSIS_GAP_REGISTER.md` §13) for
  storyline quests (missions, objectives, branching, journal)
- Companion: `OTERYN_ENCOUNTER_AUTHORING_FORMAT_V1.md` (encounters emit outcomes the quest domain consumes)
- Machine artifacts: `tools/content-schema/quest-authoring/` (schema, semantic validator, focused
  checks, Canary + CrystalServer chest and door transcriptions, wiki coverage evidence)

## 1. Problem

TibiaWiki (Fandom) lists 373 quests with a quest infobox (`samples/quest-coverage-2026-09-27.json`).
Checked against the Lua sources of both reference servers:

| | Canary 47dfd51f | CrystalServer ff7ede59 | Either |
|---|---:|---:|---:|
| Implemented | 324 | 328 | 332 |
| Partial (a storage stub, an outfit item, one reward line) | 9 | 13 | 12 |
| Absent from the Lua sources | 40 | 32 | 29 |

Eight quests exist only in CrystalServer (25 Years of Tibia, A Piece of Cake, Between the Lines,
Bloody Tusks, Dream Warrior Outfits, Formal Dress Outfits, Newhaven, Targuna) and four only in
Canary (Decaying Defender Outfits, Hand of the Inquisition Outfits, Orcsoberfest, Too Hot to
Handle). Most absent quests are 13.x-15.x content; some old single-chest quests may exist only as a
map container in `world.otbm`, which the coverage check does not read.

Only 93 of the 373 quests appear in Tibia's in-game quest log. The largest group of the rest are
reward chests: a chest hands out its reward once per character. Both servers keep these chests as
data (`startup/tables/chest.lua`, table `ChestUnique`) served by one shared script
(`scripts/actions/system/quest_reward_common.lua`). Neither server has a script per chest.

The CW2 B6 map catalogue (`OTV2-20260920-content-world-cw2-b6-interaction-bindings.json`) finds 0
action and unique ids in `world.otbm`. The servers put chest unique ids and door action ids on the
map at startup from their tables (`chest.lua`, `door_quest.lua`, `door_key.lua`, `door_level.lua`),
so neither chests nor doors are in that catalogue.

## 2. Principles

1. A reward chest is a world interaction, not a storyline quest (D32). It has no missions and no
   quest-log entry. A lightweight `reward_only` quest record names it and links it to the wiki,
   achievements, prerequisites of other quests and analytics.
2. Legacy storage keys, KV quest names, unique ids and action ids are not Oteryn identities
   (CW import plan B6). They are kept as source evidence in the manifest for reimport.
3. Both reference servers are sources (D33). They number chests differently (unique id 6117 is the
   Combat Knife chest in Canary and Captain Iglue's chest in CrystalServer), so chests are joined by
   map position.
4. The source mechanics are the reference, including where the two servers differ. Source defects
   are recorded, not reproduced (as in D25). Where the source is unclear, the reference-date wiki
   decides.
5. The claim is character state (quest progress: Character persistence, strong durable, shared
   across channels, `MULTICHANNEL_SYSTEM_SCOPE_MATRIX.md`). Items change hands under the item
   transaction contract (DUR-03). Character writes stay session-generation fenced.

## 3. Mechanics taken from the servers

| Source behaviour (`quest_reward_common.lua`) | Oteryn rule |
|---|---|
| A storage key or KV quest name marks the chest as taken for the character; "The chest is empty." afterwards | One claim per character per `RewardClaim`. Its identity comes from the source marker as `…:reward-claim/<marker>`. |
| Several chests share one marker (the Annihilator's four chests, Barbarian Arena weapon sets, Pits of Inferno, Thieves Guild) | One `RewardClaim` with several placements; taking any placement consumes the claim (a choice). |
| Free backpack slot and capacity are checked; "too heavy" and "no room" messages | Checked before anything is handed out. The handout is all or nothing. |
| Without a container Canary checks and hands out items one by one and marks the chest after the first item; a later capacity failure loses the rest | Defect, not reproduced: one item transaction for the whole reward, idempotent under retry. |
| `container`: reward items go inside a container ("You have found a bag.") | `reward.container`. |
| `isKey`/`keyAction`: the key item gets the door's action id | `reward.key_binding` names the door key (`…:door-key/<n>`); key doors read it (§3.1). |
| `AttributeTable`: a written text on the reward, per unique id; Canary stamps every reward item, CrystalServer only the item it names | `reward.written_text` with the item that carries it. The CrystalServer fix is taken for chests present in both servers. |
| `achievementTable`: the Annihilator chests grant an achievement | `placement.achievement`. |
| `randomReward`: one entry drawn per use (Canary writes the draw into the shared table) | `reward.random_one_of`, drawn per claim with auditable randomness. |
| `time` (hours) with `storage`: meant as a cooldown, but the script only reads `timerStorage`, so both servers hand out the six Secret Library chests once | `repeat: {kind: cooldown, hours}` from the data; the wiki confirms it (Brass-Shod Chest: Falcon Bastion chests open once every 24h); `source_divergence` records the server defect. |
| `weight` field set by hand | Not kept; the runtime derives weight from item definitions. |

Messages are presented from item names at runtime ("You have found a …", "… is empty",
"… too heavy …", "… no room …"). They are not stored per chest.

### 3.1 Doors

The map loader puts each door table key on the listed positions as the door's action id. Three
shared scripts, identical in both servers (`data/scripts/actions/doors/`), decide:

| Source behaviour | Oteryn rule |
|---|---|
| Quest door (`door_quest.lua`, `QuestDoorAction`): passes a player whose storage named by the action id is set, and moves the player through; otherwise "The door seems to be sealed against unwanted intruders." | `Gate` with `quest_progress`: a read-only view of the character's quest progress, published by the quest domain (as the encounter `killer_progress` condition reads it). When a chest records the same marker, the gate names that `RewardClaim`. |
| Level door (`door_level.lua`): passes a player whose level is at least the action id minus 1000; otherwise "Only the worthy may pass." | `Gate` with `min_level`. |
| Key door (`door_key.lua`): a key whose action id matches locks or unlocks the door for everyone; action ids 101 and 1001 are always locked | `Gate` with `door_key` and state `shared_lock`; the gate lists the reward claims that hand out its key. |
| `QuestDoorUnique`: vocation doors, the Katana and Secret Service doors, each opened by a dedicated script | Not a gate: `unresolved_semantics` until that script is transcribed. |
| A quest door keyed by a bare number (CrystalServer restores such ids "since the world.otbm stopped storing ids") | Not a gate: no named quest progress; the door stays sealed unless a script handles it (`unresolved_semantics`). |

Quest and level gates are checked per character on each passage. A key door's lock is world-object
state of the channel or instance the door stands in (`ChannelRuntime`, local like NPC runtime state
in the scope matrix), not character state.

## 4. Shape

```
RewardClaim                          content/interactions/ (definition) + world placements
  identity                           key + revision (source-derived candidate key)
  label, section                     source comments, evidence only
  quest                              Quest ref when linked by KV name, storage key or own label
  quest_link_basis                   kv_quest_name | storage_key | label
  quest_candidate_from_section       Quest ref for review: the file's section header only
  claim
    per                              character
    repeat                           once | cooldown(hours)
  placements[]                       one per chest that shares the claim
    position                         source map position (bound by the map project)
    appearance                       chest item
    reward
      items[]                        item + count
      random_one_of[]                optional, at least two options
      container                      optional
      key_binding                    optional door key
      written_text                   optional, with the carrying item
    achievement                      optional
  source_divergence                  optional note where the source contradicts its own data

Quest (kind reward_only)             content/quests/definitions/
  identity, display_name
  shown_in_quest_log                 from the wiki category "Quests in In-Game Quest Log"
  wiki                               title, pageid, revid (facts only)
  requirements_from_wiki             premium, level (as recorded; not yet typed)
  claims[]                           RewardClaim refs

Gate                                 content/interactions/ (definition) + world placements
  identity                           key + revision (…:door-gate/progress|key|level/<rule>)
  label                              source comment, evidence only
  quest, quest_link_basis            Quest ref linked by storage key or own label
  condition                          one of
    quest_progress                   progress (…:quest-progress/<marker>), claim (RewardClaim ref or null)
    min_level                        level
    door_key                         key_binding (…:door-key/<n>), key_from_claims[] (RewardClaim refs)
  state                              per_character_pass | shared_lock (key doors only)
  placements[]                       position (source map position), appearance (door item or null)
```

Item references use the source namespace (`canary:item/<id>`, `crystalserver:item/<id>`), as the
monster and encounter samples do. Native `ItemType` binding follows when the item crosswalk
resolves source ids. The CW2 B3 loot binding evidence has every row still `UNRESOLVED`.

## 5. Joining the two servers

1. Chests and doors are joined by map position; an entry with several positions yields one
   placement each. A rule both servers have keeps one identity under the Canary namespace, also for
   doors only CrystalServer lists.
2. Identical in both servers: `mapped`.
3. Present in one server only: included under that server's namespace.
4. One server has an invalid claim marker and the other a valid one: the valid one is taken. Canary
   uid 6093 stores the undefined global `keyAction` and CrystalServer the key storage.
5. Otherwise: `conflict`. The manifest keeps both values for the wiki to decide (D25); until then the
   claim or gate carries the Canary value. A storage name that differs only in letter case or
   underscores (`GraveDanger.Questline` and `QuestLine`) is the same rule.
6. Empty containers without a reward are world objects, not claims: `approved_omission`, listed in
   `empty_containers.json`.
7. A quest keeps one identity across slices and servers: the Canary namespace when Canary
   implements it at all (the coverage sample), otherwise the CrystalServer namespace.

## 6. First transcription

`samples/chests/` (`ots_chests.py`, deterministic):

| | Count |
|---|---:|
| `ChestUnique` entries: Canary / CrystalServer | 389 / 490 |
| Positions: in both / Canary only / CrystalServer only | 386 / 1 / 104 |
| Reward claims / placements | 336 / 359 |
| Placements from both servers / CrystalServer only / Canary only | 344 / 14 / 1 |
| Choice groups (several placements, one claim) | 11 |
| Containers / door keys / written texts / random rewards / cooldowns | 68 / 27 / 14 / 6 / 6 |
| Empty containers and duplicate markers (approved omissions) | 134 |
| Conflicts: decided by the wiki / open | 1 / 1 |
| Claims linked to a wiki quest: KV name / storage key / own label | 92 / 151 / 37 |
| Claims with a section-only candidate (review) / without a link | 35 / 21 |
| Reward-only quests | 120 |

The Thieves Guild goblet chest hands out a golden goblet in Canary and a stolen golden goblet in
CrystalServer. The wiki decides for CrystalServer: the spoiler says the chest behind the quest door
holds the Stolen Golden Goblet. One corpse chest still shows a different corpse appearance in each
server; both items are called "dead human", so the wiki cannot decide it and the map will. The Canary-only
chest (Wrath of the Emperor, 33074/31170/8) holds the same reward as a CrystalServer chest a few
tiles away (33079/31173/8), probably one chest moved. The map decides.

Section-only matches are not links, because the file's section headers do not always cover the
entries below them. The four outlaw camp key chests, for example, sit under the Katana Quest header.

### 6.1 Doors

`samples/doors/` (`ots_doors.py`, deterministic, reads `samples/chests/claims.json` for claim and key
links):

| | Count |
|---|---:|
| Door positions: in both / Canary only / CrystalServer only | 336 / 5 / 104 |
| Gates: quest progress / key / level | 184 / 38 / 14 |
| Quest gates reading a reward claim (the Annihilator door) | 1 |
| Key gates whose key comes from a chest | 24 |
| Quest gates linked to a wiki quest | 176 |
| Conflicts | 10 |
| Unresolved: dedicated-script doors / bare-number quest doors | 7 / 9 |

The ten conflicts are storage names that the servers model differently. Canary gives each of the
six Kilmaresh sixth-mission mask doors a storage per mask, while CrystalServer gates all six on one
`Kilmaresh.Sixth.Favor`. The Kilmaresh access door, the two Order of the Lion eastern doors
(`AccessEastSide`, `AccessEasternSide`) and King Zelos's door (`KingZelos.Room`, `KingZelosDoor`)
are renamed. They are left for the quest domain, which defines the progress these gates read.

## 7. Ownership

- Static claim and placement: Content (`content/interactions/`, `content/world/placements/`),
  candidate-only until CW3 accepts a closed release.
- Claim state (taken, cooldown until): Character persistence, per character, shared across channels.
- Item handout: DUR-03 item transaction, all or nothing, idempotent by claim.
- Achievement grant: Achievement domain on the claim outcome.
- Gates: Content (definition and placements). Quest and level checks read character state; a key
  door's lock is channel or instance world-object state.
- Quest records: `content/quests/definitions/` (`reward_only`); storyline quests wait for
  `CONTENT-QUEST-01`.

## 8. Next slices

1. Quest-log quests (51 in Canary, 59 in CrystalServer) with missions and objectives: needs the
   `CONTENT-QUEST-01` decision on graph and state.
   The quest domain then defines the progress markers the door gates read, and settles the ten
   door conflicts.
2. NPC-driven outfit and addon quests (under the NPC service boundary).
3. The dedicated-script doors (vocation doors, Katana, Secret Service).
4. TibiaWiki BR is not captured: `www.tibiawiki.com.br` answers this capture host with a
   Cloudflare challenge.

## 9. Owner decisions

| # | Decision | Basis |
|---|---|---|
| D32 | A reward chest is modelled as a world interaction: a `RewardClaim` taken once per character (or on a cooldown), with a lightweight `reward_only` quest record for naming, wiki linkage, achievements and prerequisites. It gets no missions and no quest-log entry. A chest that is a step of a storyline quest becomes an objective or reward of that quest. | Owner accepted the proposal ("tak", 2026-09-27). Only 93 of 373 wiki quests appear in Tibia's quest log; both servers serve chests from one data table. |
| D33 | Canary and CrystalServer are both reference sources for quests, extending D30 (Crystal as a second donor for encounters): for quests CrystalServer is a full source, not only a donor. Their mechanics are used as implemented; where they differ, the union is taken, joined by map position, and conflicts go to the reference-date wiki (D25). | Owner request ("pamiętaj żeby używać i crystal i canary jako reference", "canary i crystal mają pewnie mechaniki wdrożone więc możemy się nimi posiłkować", 2026-09-27). |
