# Oteryn Quest Authoring Format v1

- Date: 2026-09-27
- DecisionStatus: CANDIDATE (owner decisions D32-D36 recorded in §9; slice 1 covers reward chests,
  slice 2 quest, key and level doors, slice 3 the quest-log quests as staged missions with their
  transitions, slice 4 the quest scripts as interaction definitions)
- DeliveryStatus: OPEN (design draft and offline transcription only)
- ImplementationStatus: NOT_STARTED
- Programme: CW2 B6 quests/interactions (`docs/agents/programs/OTV2_CONTENT_WORLD_BULK_CATALOG_IMPORT_PLAN.md`)
- Precision gap `CONTENT-QUEST-01` (`ARCHITECTURE_ANALYSIS_GAP_REGISTER.md` §13): D34 settles the
  graph and state representation (staged missions); party/guild/world scope, schedules and resets,
  migration of active quests and authoring tooling stay open
- Companion: `OTERYN_ENCOUNTER_AUTHORING_FORMAT_V1.md` (encounters emit outcomes the quest domain consumes)
- Machine artifacts: `tools/content-schema/quest-authoring/` (schemas, semantic validator, focused
  checks, Canary + CrystalServer chest, door, quest-log and interaction transcriptions, wiki coverage
  evidence, the map check and the readiness map)

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
5. Narrative text (book texts, journal lines) is reserved content (`LICENSE-ASSETS.md`). As in the
   NPC bundles, the samples keep a text reference (SHA-256, length, placeholders), never the text,
   and the validator rejects committed text.
6. The claim is character state (quest progress: Character persistence, strong durable, shared
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
| `AttributeTable`: a written text on the reward, per unique id; Canary stamps every reward item, CrystalServer only the item it names | `reward.written_text`: the item that carries it and a text reference. The CrystalServer fix is taken for chests present in both servers. |
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

### 3.2 Quest log

Canary keeps one catalog file per quest (`lib/core/quests/catalog/*.lua`), CrystalServer one `Quests`
table (`lib/core/quests.lua`). Both describe a quest the same way:

| Source behaviour | Oteryn rule (D34) |
|---|---|
| A quest appears in the log once its start storage reaches the start value | `start`: a progress track and the value it must reach. |
| A mission is shown while one storage lies between `startValue` and `endValue` | A mission is one progress track (an integer in Character persistence, shared across channels) with a start and an end value. |
| `states[value]` gives the journal text for each storage value; `description` one text for all | `journal.per_stage` (a text reference per stage value) or `journal.fixed`. |
| A journal function computes the text, e.g. "You already hunted %d/300 badgers" from a kill counter | `journal.template`: text references for the string pieces and the progress tracks the function reads; typed counters follow when a quest needs them. |
| Scripts advance the storage directly: NPC dialogues, map movements, actions, creature events, chests (`setStorageValue`) | The quest domain alone writes quest progress (D35). Each mission declares its transitions (a new value, a step, or a computed value, with the stage it starts from); owners (NPC dialogue, movement, action, creature event, encounter outcome, claim) only request a named transition, and conditions read the stage. |

Transitions are read from every `setStorageValue` on a mission's storage in both servers
(`lua_writers.py`): the owner comes from the script object of the enclosing callback (`MoveEvent`,
`Action`, `CreatureEvent`, …) or from the file (`npc/`, `lib/`). The effect is the literal value,
a step on the same storage, or a computed value (a `timestamp` is a cooldown). The `from` stage is
read from the nearest comparison of the same storage through its if-block: kept in the `then`
branch, negated in `else` or after an early `return`, unknown otherwise, and not `exact` when the
condition is compound. It is a line-level reading, not an evaluation; the transcription of each
owner confirms it.

A transition written by an NPC script names its dialogue in `requested_by`:
- the NPC bundle key of the NPC authoring format (`canary:npc/<file>`, `crystal:npc/<file>`);
- the player keywords (`MsgContains`) of the if-blocks around the write;
- the dialogue topics those if-blocks test.

The NPC side binds this to its dialogue nodes once its promotion needs it. Until then the node keeps
its opaque `LUA_ACTION` and the quest side carries the reference, so no NPC-owned file changes (O4
is settled by D35).

Missions whose storage is a bare number (the 70 Killing in the Name of tasks) keep it as the track
`storage/<n>`. A mission graph with typed, branching objectives (option B) is added only for a quest
that staged missions cannot express (D34).

### 3.3 Interactions

The movement, action and creature-event scripts are what GAME-INTERACTION-01 plans: an edge on a
target, read-only conditions and children. D36 keeps them in that shape, in `content/interactions/`,
and hands every child to the domain that already owns the effect. No second effect engine is added
(ADR-0019), and the item schema's `use.interactions[]` and the monster schema's event bindings (D6)
point at these definitions.

| Source behaviour | Oteryn rule (D36) |
|---|---|
| `onStepIn`, `onStepOut`, `onAddItem`, `onUse`, `onDeath`/`onKill` on a registered action id, unique id, item or position | `source.edge`: `ON_ENTER`, `ON_LEAVE`, `ON_CONTACT`, `USE`, `ON_DEATH`, `ON_KILL`; the registrations stay as evidence. |
| `if` on a player storage, a global storage, whether the actor is a player, its level, how many of an item it carries, or the item type, unique id, action id or subtype of the edge source, the object in contact or the use target | Read-only conditions: `quest_stage`, `world_state` (D29), `actor_is_player`, `actor_level`, `actor_item_count`, `object`. `branch`/`otherwise` keep the if/elseif/else structure. |
| `player:setStorageValue` | Quest child: a request of the named mission transition (D35), or of a progress track the quest domain still has to declare. |
| `Game.setStorageValue` | Quest child: world state shared by all players (D29). |
| `Game.createMonster` | Ability child: a summon effect (GAME-ABILITY-01) at a named anchor. |
| `player:addItem` | Item child: a hand-out through the DUR-03 item transaction; items the script then puts into a handed-out container are its `contents`. |
| `player:removeItem`; `remove` on the item used (`onUse`) or dropped onto the edge (`onAddItem`) | Item child: consumption through DUR-03 (D38), never map state. |
| `addAchievement` | Achievement child: a grant by the Achievement domain. |
| `addOutfit`, `addOutfitAddon`, `addMount`, `addExperience` | Outfit, Mount and Experience children: grants requested from the owning Character domain. |
| `kv:set`, `addCondition`, `setBossCooldown`, creature removal | Unresolved with a reason that names the missing owner, so the readiness map (§6.7) counts it as a needed feature. |
| `sendMagicEffect`, `sendTextMessage`, `say`, `sendCancelMessage`, `addMapMark` | Presentation child, never authoritative; a message keeps its source line, never its text (LICENSE-ASSETS.md). |
| `teleportTo` | D37 relocation child (`request: relocate`, `scope: in_scope`) naming an anchor or the previous tile, owned by the current scope's `ChannelRuntime`/`InstanceRuntime` (`OTERYN_INTERACTION_RELOCATION_AND_WORLD_OBJECT_OWNERS_PROPOSAL_V1.md` §3); a computed target that cannot yet name an anchor stays blocked (GAME-INTERACTION-01 §19.3), as does relocation to another Channel or Instance until `SCOPE_HANDOFF` has a contract. |
| `transform`, `createItem`, `remove`, `revertItem`, `decay`, `setActionId` on map objects (walls, levers, flames) | D38 world-object overlay operation (`TRANSFORM`, `CREATE`, `REMOVE`, `RETAG`) at the same scope runtime's overlay (proposal §4), optionally carrying `revert_after_ms` for a `decay`/`revertItem`/`addEvent` revert; a call the converter cannot yet classify by kind stays blocked. |
| Encounter scripts | Stay encounters; they emit outcomes only (D27). |

Source positions become anchors (`p1`, `p2`, …) with the coordinates kept as evidence until world
placements bind them. Anything the data cannot express (loops with their own control flow, local
lookups, computed values, callbacks deferred with `addEvent`) stays `unresolved` with its source line
and reason; when an interaction needs it, it
becomes a DUR-04 component that proposes a plan, not a script with direct writes.

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
      written_text                   optional: carrying item and text_ref (sha256, length, placeholders)
    achievement                      optional
  source_divergence                  optional note where the source contradicts its own data

Quest (kind reward_only)             content/quests/definitions/
  identity, display_name
  shown_in_quest_log                 from the wiki category "Quests in In-Game Quest Log"
  wiki                               title, pageid, revid (facts only)
  requirements_from_wiki             premium, level (as recorded; not yet typed)
  claims[]                           RewardClaim refs

Quest (kind script_only)             content/quests/definitions/
  identity, display_name, wiki,      as reward-only quests; no missions, start or gates
  requirements_from_wiki, claims[]

Quest (kind storyline)               content/quests/definitions/ + missions/
  identity, display_name, wiki,      as reward-only quests; claims[] may be empty
  requirements_from_wiki, claims[]
  source_name                        the quest-log name in the source
  start                              progress, at_least (or null)
  missions[]
    key, name
    progress                         …:quest-progress/<track>
    start_value, end_value           the range in which the mission shows
    journal                          per_stage[{value, text_ref | template}] | fixed(text_ref) | template(parts, reads) | none
    transitions[]                    key (<owner>_<n>), owner, callback, from {op, value, exact} | null,
                                     to | increment | computed (timestamp | expression), servers
  gates[]                            Gate refs whose condition reads one of its tracks

Progress track                       samples/questlog/progress.json (evidence for Character persistence)
  key                                …:quest-progress/<track>
  missions[], start_of[]             who uses the track
  read_by_gates[]
  writes                             per server: number of writes found
  transitions[]                      key, script, per-server source path, line and registrations

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
   claim or gate carries the Canary value. A decision recorded in `conflict_decisions.json` (§6.4)
   maps the conflict to the chosen server and cites its basis. A storage name that differs only in letter case or
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
| Conflicts: decided by the wiki / as equivalent (§6.4) | 1 / 1 |
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
| Gates: quest progress / key / level / lever | 184 / 38 / 14 / 1 |
| Quest gates reading a reward claim (the Annihilator door) | 1 |
| Key gates whose key comes from a chest | 24 |
| Quest gates linked to a wiki quest | 176 |
| Conflicts (decided, §6.4) | 10 |
| Unresolved: Dawnport vocation doors (an interaction) / doors no script opens / bare-number quest doors | 4 / 2 / 9 |

The ten conflicts are storage names that the servers model differently. Canary gives each of the
six Kilmaresh sixth-mission mask doors a storage per mask, while CrystalServer gates all six on one
`Kilmaresh.Sixth.Favor`. The Kilmaresh access door, the two Order of the Lion eastern doors
(`AccessEastSide`, `AccessEasternSide`) and King Zelos's door (`KingZelos.Room`, `KingZelosDoor`)
are renamed. The wiki gives each mask its own catacomb door, so the Canary storages are kept; the
renamed doors are an implementation detail and keep Canary too (§6.4).

Seven doors are opened by their own script, identical in both servers:
- **Katana Quest door.** A `lever` gate: no player condition; a lever nearby opens and closes it
  for everyone (`shared_lock`).
- **Dawnport vocation doors (4).** They never open. A `USE` checks the vocation-choice track,
  removes mainland items and relocates the player, so they are an interaction, not a gate.
- **Secret Service door.** Its script only reads the door as the target of a mission item.
- **One CrystalServer door.** It only restores a lost id; no script sets or reads it.

### 6.2 Quest log

`samples/questlog/` (`ots_questlog.py`, deterministic, reads the chest and door samples):
`quests.json` is the whole quest catalogue (storyline quests, the reward-only quests no storyline
quest absorbs, and the script-only quests), `progress.json` the progress tracks, `manifest.json` the
mission mapping.

A **script-only** quest is one both servers implement in scripts (bosses, chests, world state) but
neither lists in its quest log, for example Soul War, Kilmaresh and Heart of Destruction.
`script_quests.json` names each one by its wiki title and gives the basis of the match. For 25 the
wiki title and the script directory match once normalised. For 10 an explicit key covers an
apostrophe, an article, a plural or a source spelling.

| | Count |
|---|---:|
| Quest-log entries: Canary / CrystalServer / CrystalServer only | 51 / 59 / 7 |
| Storyline quests / missions | 58 / 529 |
| Journals: per stage / fixed / template | 468 / 38 / 23 |
| Progress tracks (mission and start / auxiliary, §6.5) / set by both servers' Lua / with no literal writer found | 1,001 (556 / 445) / 704 / 109 |
| Storyline quests linked to a wiki quest | 52 |
| Reward-only quests absorbed by a storyline quest / script-only quests / catalogue quests | 21 / 35 / 192 |
| Gates / reward claims attached to storyline quests | 138 / 98 |
| Missions mapped / of which decided from a conflict (§6.4) | 529 / 35 |
| Transitions / missions with at least one | 1,832 / 422 |
| Transitions by owner: NPC / action / movement / creature event / library / other | 1,442 / 182 / 78 / 31 / 40 / 59 |
| Effects: new value / step / computed | 1,727 / 63 / 42 |
| Transitions with a known `from` stage (exact) / in both servers | 441 (343) / 1,371 |
| NPC transitions / with keywords / with topics / whose NPC is in the NPC census | 1,442 / 1,240 / 739 / 1,442 |

The 35 conflicts were mostly journal texts; the rest storage names or value ranges. They are decided
in §6.4. "No literal writer" means the index found no
`setStorageValue(<storage>, …)` call; such tracks are set through a variable, a loop or a KV store,
as the task counters are.

The Queen of the Banshees shows the whole chain. In both servers the seal-flame movement scripts
set seven of its eight seal missions, from an unset seal to 1, and the Queen's dialogue sets the
last. Eight quest gates
(the seal doors and the banshee door, each on its own door storage) belong to the quest, and six
reward claims hand out the final chests.

### 6.3 Interactions

`samples/interactions/` (`ots_interactions.py`, deterministic, reads the quest-log sample)
transcribes every script under `scripts/quests/` of both servers (130 quest directories), keyed
`interaction/<quest>/<script>` (plus the script object when a file holds several callbacks) and
joined by that key; `interaction.schema.json` and
`validate_quest_content.py --interactions` validate it with the quest catalogue.

| | Count |
|---|---:|
| Interactions | 1,221 |
| Edges: `USE` / `ON_ENTER` / `ON_DEATH` / `ON_LEAVE` / `ON_CONTACT` | 599 / 400 / 202 / 14 / 6 |
| Children: Quest / Ability / Item (hand-out, consumption) / Achievement / Outfit / Mount / Experience / Presentation / Movement / WorldObject | 947 / 208 / 343 (160, 183) / 36 / 14 / 5 / 9 / 2,679 / 800 / 891 |
| D37 relocation children: to a named anchor / blocked (computed target, incl. an unproven previous-tile expression) | 219 / 581 |
| D38 overlay operations: classified by kind (`TRANSFORM`/`CREATE`/`REMOVE`/`RETAG`) / still blocked pending call-type re-transcription from source | 0 / 891 |
| Quest children naming a mission transition | 215 |
| Unresolved statements / conditions | 1,908 / 1,916 |
| Of the statements, naming a missing owner: delayed callback / key-value write / creature removal / condition / boss cooldown | 304 / 56 / 43 / 26 / 25 |
| Interactions mapped / unresolved / of which decided from a conflict (§6.4) | 256 / 965 / 37 |

An independent spot check of 56 randomly sampled classified lines against their source found no
misclassification. Most interactions keep some unresolved part: local tables and lookups,
boss-room loops and delayed callbacks are the common ones. They stay with their source line
rather than being guessed. Every track a quest child writes is declared by the catalogue (§6.5).

The Queen of the Banshees shows the result for one quest: 18 interactions; its seven seal flames
request the seven movement transitions of slice 3 (one per mission, the last one opening the final
battle); the 9 tracks it writes outside missions (the seal doors and two helper counters) are
declared as auxiliary tracks (§6.5). Its two conflicts keep
Canary: the first seal lever's item ids (the wiki is silent) and the first seal's magic walls, which
CrystalServer triggers elsewhere and closes late, against the wiki.

The Queen of the Banshees is also the worked example for D37 and D38 (`ots_interactions.py`,
`interaction.schema.json`). Its 20 interactions carry 19 Movement and 27 WorldObject children. Only a
`teleportTo(fromPosition)` call with no further argument (allowing a trailing non-positional one such
as a `pushMove` flag) is a typed D37 relocation to the previous tile; an offset or lookup that merely
mentions `fromPosition` (e.g. `Position(fromPosition.x + 1, ...)`) is not a fully-delimited literal
match and stays blocked. The converter now names 9 of its Movement children as typed relocations to a
named anchor (`request: relocate`, `scope: in_scope`); the other 10 (the seven seal flames' step-back
teleports plus the two picked-at-runtime sacrifice/computed targets) stay blocked until each can name
an anchor. Its 27 WorldObject children — the seal levers and the magic walls the two conflicts (§6.4)
are about — are not yet classified into `TRANSFORM`/`CREATE`/`REMOVE`/`RETAG`: the committed
transcription never recorded which source call (`transform`, `createItem`, `remove`, `setActionId`,
`decay`, `revertItem`) produced each one, only its source line, so turning them into typed D38
operations needs a fresh run of `ots_interactions.py` against the pinned Canary/CrystalServer
checkouts (§5). That run would classify each WorldObject call by its exact argument structure (a
literal `Position(x,y,z)` names a pre-authored anchor; a `createItem` id is only literal when it is a
complete, delimited integer; a revert attaches only when its own receiver or literal position provably
names the same object as the operation it reverts), never by an identifier's name or its position in a
list. The same evidence gap holds for the rest of the corpus: the 891 WorldObject children and 581 of
the 800 Movement children (372 genuinely computed, 209 a `fromPosition`-referencing expression that is
not itself provably the bare previous-tile relocation) stay blocked; the other 219 anchor-bound
Movement children are typed today, since their target was already on record as a literal position.

### 6.4 Conflict decisions (D25)

`conflict_decisions.json` records one decision per conflict of the four transcriptions. Each gives
the chosen server, its basis and the difference in our own words; no wiki, journal or dialogue text
is copied. The converters apply it and stop when a recorded decision no longer matches a conflict.

| Basis | Chests | Doors | Missions | Interactions |
|---|---:|---:|---:|---:|
| Equivalent behaviour or implementation detail (Canary kept) | 1 | 4 | 6 | 10 |
| Wording: spelling or grammar (the correct version) | | | 6 | |
| Wiki decides for Canary | | 6 | | 6 |
| Wiki decides for CrystalServer | | | 8 | 9 |
| Wiki silent (Canary kept) | | | 15 | 12 |

CrystalServer wins where Canary is outdated or broken. Examples:
- Hot Cuisine: the 15th dish added with the Monk vocation.
- The Way of the Monk: the ten-shrine pilgrimage.
- The Thieves Guild: Percybald is in Carlin.
- Blood Brothers: Boreth's plant-burning encounter.
- Heart of Destruction: Canary summons a creature name that does not exist.
- Rottin Wood and the Married Men: five nets and merchants, where Canary stops at four.
- The Ancient Tombs: switch puzzles shared by the players.
- The Thornfire prepared bucket.

A first pass found 60 interaction conflicts. Twenty-three were callbacks paired by their position in
a file, so one server's added callback shifted the pairs; they are now joined by their script object.
Interactions decided for CrystalServer keep the quest's Canary identity (D33).

### 6.5 Declared auxiliary tracks

Quest scripts also write tracks outside any mission: seal doors, counters, cooldowns and puzzle
state. Under D35 the quest domain owns them too, so the catalogue declares each one (445) in
`progress.json` with `auxiliary_of`. The owning quest comes from three sources, in this order:

| Owner basis | Tracks |
|---|---:|
| The longest mission-track prefix that names one quest | 279 |
| The script directory whose scripts write the missions of one quest | 8 |
| `track_owners.json`: assigned from the track and script names and the script code, checked by sampling | 158 |

- The 81 tracks that belonged to a wiki quest missing from the catalogue now name their
  script-only quest.
- 8 belong to no quest; their `note` gives the reason, for example world changes, generic helpers and
  an example script.
- 33 auxiliary tracks are read by door gates.

The converter stops when a written track has no owner, and when `track_owners.json` names a track
that needs no record. No interaction writes an undeclared track.

Both converters expand `local X = Storage.…` aliases before reading writes. That added 216
transitions to slice 3. A file that shadows `Storage` itself keeps its full paths.

### 6.6 Map check

`samples/map-check/` (`ots_map_check.py`, deterministic) reads Canary's `otservbr.otbm` (release
v3.6.1, the `mapDownloadUrl` of the pinned revision, sha256 `a80de1dd…`) and CrystalServer's own
`world.otbm`. Neither map is committed.

| | Count |
|---|---:|
| Chest placements with the chest item on the tile (Canary map) | 343 / 343 |
| Door placements with the door item on the tile / tile present with no expected item | 33 / 337 |
| Interaction anchors whose tile exists | 313 / 313 |
| Trigger ids (`aid()`/`uid()` registrations) found on the raw map / stamped at startup / not found | 160 / 124 / 65 |

- Canary stamps chest, door and most trigger ids onto placed items at startup (the Map Attributes
  Loader tables), so the raw map holds the items but not those ids. The check therefore compares
  item ids and reads the startup tables, where every listed item is on its tile.
- The 65 ids not found belong to places loaded at runtime (quest overlay maps) or to scripts
  nothing on the map triggers; they are listed for review.
- The two open map questions of §6 are answered. The corpse chest is the same tile with a different
  corpse item in each map, which confirms the "equivalent" decision. The Wrath of the Emperor chest
  exists only at each server's own position on its own map: the chest moved between the servers.

### 6.7 Readiness map

`samples/readiness/readiness.json` (`ots_readiness.py`, from the committed samples) lists, per
quest, the engine features it needs and the data gaps it still has. It joins interactions to
quests through the tracks they read or write, then through the script directory.

Built in the order that completes the most quests first, the features give (192 quests):

| Built so far | Quests complete on the engine side (without data gaps) |
|---|---:|
| `USE` trigger and reward claim | 75 (75) |
| + world objects, teleports, step triggers | 88 (76) |
| + item consumption, kill triggers, quest state | 95 (78) |
| + NPC dialogue and progress doors | 121 (98) |
| + delayed callbacks, item hand-out, summons, achievements, conditions | 158 (99) |
| + creature removal, boss cooldowns, key-value state, outfits, mounts, experience | 192 (99) |

- The reward chest is the first target: with the `USE` trigger it completes 75 quests.
- The Queen of the Banshees needs ten features, including summons and delayed callbacks, and still
  has 18 data gaps in 9 interactions.
- 29 of 1,221 interactions join no catalogue quest. They are generic scripts (Rookgaard helpers,
  Candia bosses, Marapur, the Raging Mage tower, Soulpit and others), not quest content.
  Five script directories whose name differs from their quest key are joined explicitly
  (`DIRECTORY_QUESTS`).

## 7. Ownership

- Static claim and placement: Content (`content/interactions/`, `content/world/placements/`),
  candidate-only until CW3 accepts a closed release.
- Claim state (taken, cooldown until): Character persistence, per character, shared across channels.
- Item handout: DUR-03 item transaction, all or nothing, idempotent by claim.
- Achievement grant: Achievement domain on the claim outcome.
- Interactions: Content (`content/interactions/`), compiled to GAME-INTERACTION-01 plans; each
  child is executed by its owner (quest, ability, item transaction), never by the interaction.
- Gates: Content (definition and placements). Quest and level checks read character state; a key
  door's lock is channel or instance world-object state.
- Quest records: `content/quests/definitions/` (`reward_only`, `script_only`, `storyline`) and
  `content/quests/missions/`.
- Mission progress: Character persistence (one integer per track), shared across channels. Only the
  quest domain writes it, by validating a requested transition against the current stage (D35);
  a repeated request is idempotent, and writes are session-generation fenced. Rewards of a
  transition go through the reward and item transaction owners, once per transition.

## 8. Next slices

1. Owner transcription. The movement and action scripts are transcribed (§6.3), all conflicts are
   decided (§6.4), the tracks the scripts write are declared (§6.5) and NPC transitions name their
   dialogue (§3.2). Next, the NPC format binds `requested_by` to its dialogue nodes when it promotes
   dialogue; that work lives in the NPC-owned files.
2. NPC-driven outfit and addon quests (under the NPC service boundary).
3. Movement and world-object owners: D37 and D38 are decided
   (`OTERYN_INTERACTION_RELOCATION_AND_WORLD_OBJECT_OWNERS_PROPOSAL_V1.md`). After the independent
   review and the scope-runtime implementation, the blocked children become executable. The first
   target is The Queen of the Banshees, played from start to end.
4. The dedicated-script doors (vocation doors, Katana, Secret Service).

**Runtime readiness for a playable quest (audit 2026-09-27).** The server does not execute any quest
content yet. WorldProject/v2 admits Quest, Interaction, WorldObject, Npc and Dialogue records as
declarations only (`content/project/v2.rs`); only the Reference linker's definition families lower
to runtime, and none of them is a quest family.

A player completing The Queen of the Banshees needs:

| Need | Today | Blocked on |
|---|---|---|
| Step-on and use triggers | generic proposal dispatcher only (`interaction/`) | GAME-INTERACTION-01 successor (PROPOSED) |
| Quest progress store, fenced per character | only character XP persists | a quest-state store; D35 gives the rules, no contract yet |
| Teleport, walls and levers | one local step (`movement.rs`); no map-object state | VSL-MOVE-01 implementation; D37/D38 review |
| Summons | spawn rejected (`ai/mod.rs`) | GAME-AI-01 (PROPOSED) |
| Reward items | fixture-only | DUR-03 (CANDIDATE, no runtime authority) |
| NPC dialogue with quest hooks | none | an NPC dialogue runtime contract (none yet) |
| Quest lowering from content | none | a Quest/Interaction definition family in the Reference linker |

The smallest playable slice is a reward chest: a `USE` trigger, one DUR-03 hand-out and a per-character
claim. It needs only GAME-INTERACTION-01 and DUR-03 accepted and a claim store, and it completes 75
quests on the engine side (§6.7). It also needs the first durable inventory: today only XP persists,
and item weight and capacity limits are still `PARITY_PENDING_EVIDENCE` (GAME-ITEM-01). A full storyline quest
needs every row above.
5. TibiaWiki BR is not captured: `www.tibiawiki.com.br` answers this capture host with a
   Cloudflare challenge.

## 9. Owner decisions

| # | Decision | Basis |
|---|---|---|
| D32 | A reward chest is modelled as a world interaction: a `RewardClaim` taken once per character (or on a cooldown), with a lightweight `reward_only` quest record for naming, wiki linkage, achievements and prerequisites. It gets no missions and no quest-log entry. A chest that is a step of a storyline quest becomes an objective or reward of that quest. | Owner accepted the proposal ("tak", 2026-09-27). Only 93 of 373 wiki quests appear in Tibia's quest log; both servers serve chests from one data table. |
| D33 | Canary and CrystalServer are both reference sources for quests, extending D30 (Crystal as a second donor for encounters): for quests CrystalServer is a full source, not only a donor. Their mechanics are used as implemented; where they differ, the union is taken, joined by map position, and conflicts go to the reference-date wiki (D25). | Owner request ("pamiętaj żeby używać i crystal i canary jako reference", "canary i crystal mają pewnie mechaniki wdrożone więc możemy się nimi posiłkować", 2026-09-27). |
| D34 | Storyline quests use staged missions (option A of `CONTENT-QUEST-01`): a quest has missions, each mission one integer progress track with a start and an end value and a journal text per stage; transitions are named events of their owners (NPC, interaction, gate, claim, encounter). A graph of typed, branching objectives (option B) is added only for a quest that staged missions cannot express. | Owner decision 2026-09-27 ("zróbmy A a potem jeśli będzie potrzeba to B"). Both servers already describe all 529 missions this way. |
| D35 | Only the quest domain writes quest progress. Each mission declares named transitions (from a stage to a new value, a step or a computed value); NPC dialogue, movements, actions, creature events, encounters and claims request a transition, conditions read stages, and the quest domain validates the request against the current stage, applies it idempotently and session-generation fenced, and hands rewards to the reward and item owners. This settles the quest-state part of the NPC schema's open decision O4. | Owner accepted the proposal ("zgadzam się", 2026-09-27). Quest progress is character state shared across channels while NPC runtime is channel-local; both servers let any script write progress (1,616 transitions over 355 missions). |
| D36 | Movement, action and creature-event scripts become interaction definitions in `content/interactions/`, compiled to GAME-INTERACTION-01 plans: an edge (`ON_ENTER`, `ON_LEAVE`, `ON_CONTACT`, `USE`, `ON_DEATH`, `ON_KILL`), read-only conditions and children executed by their existing owners: quest transitions and world state (D35, D29), ability effects such as summons (GAME-ABILITY-01), item handouts (DUR-03), achievement grants. Teleports and map-object changes stay in the definition as blocked children until a movement and a world-object owner contract exist; encounters keep emitting outcomes only (D27); what data cannot express becomes a DUR-04 component. Items (`use.interactions[]`) and monsters (event bindings, D6) reference these definitions. | Owner continued after the consistency check against GAME-INTERACTION-01, GAME-ABILITY-01, DUR-04, ADR-0019 and the item and monster schemas ("kontynuuj", 2026-09-27), replacing the earlier proposal of a shared rule core. |
| D37 | Interaction relocation children (teleports) are owned by the current scope's `ChannelRuntime`/`InstanceRuntime`, as VSL-MOVE-01 accepts. They are identified by the GAME-INTERACTION child identity and fenced on World, scope, session generation, position revision and content generation. A request not committed in its tick is rejected, and relocation to another Channel or Instance stays blocked until `SCOPE_HANDOFF` has a contract. | Owner accepted proposals R1-R3 ("zgadzam się", 2026-09-27); the contract text awaits independent review (`OTERYN_INTERACTION_RELOCATION_AND_WORLD_OBJECT_OWNERS_PROPOSAL_V1.md`). |
| D38 | Map-object changes are a scope-ephemeral overlay owned by the same scope runtime, extending the local transition candidate. Operations are `TRANSFORM`, `CREATE`, `REMOVE` and `RETAG`, each optionally with `revert_after`. Anything durable is quest state (D35); pick-up-able objects and carried-item removal are always DUR-03. | Owner accepted proposals W1-W3 ("zgadzam się", 2026-09-27); the contract text awaits independent review. |
| D39-D42 | The reward chest slice: GAME-INTERACTION-01 governs the `USE` edge (D39); DUR-03 mints from an interaction child, idempotent per claim and character (D40); the reward goes only into the inventory: weight and backpack room are checked first, and a failure tells the player why and creates nothing, never dropping to the ground (D41); a per-character `RewardClaim` record commits in the same transaction as the item (D42). | Owner consent after a comparison with both servers (2026-09-27); details and the open composition point in `OTERYN_REWARD_CHEST_PLAYABLE_SLICE_DECISIONS_V1.md`. |
