# OTV2-20261006 Crystal Summer quest audit

Status: retained evidence and work queue  
Repository baseline: `Oteryn/Oteryn-Game@e953f1ef67ed5a9c66cf8ce6cac37da66f999dcb`  
Crystal comparison baseline: `zimbadev/crystalserver@00ce02a57ca5a12e48f32a3476e37471167e4c3f` (`summer-update`)  
Audit date: 2026-10-06

## Purpose

Retain the read-only comparison between the Crystal `summer-update` quest-script corpus and the canonical Oteryn quest catalogue so follow-up workers can continue from the same evidence instead of re-counting from scratch.

This document is evidence and a work queue only. It does not certify runtime completeness, gameplay parity, or merge readiness.

## Evidence classification

- **PROVEN**: directly observed in the pinned repositories/files named here.
- **DERIVED**: deterministic comparison or mapping derived from PROVEN repository facts.
- **UNKNOWN**: not established by this audit.
- **CONFLICT**: source evidence disagrees or the mapping needs manual resolution.

## Baseline facts

### Oteryn catalogue

**PROVEN**

At the pinned Oteryn baseline:

- canonical quest definitions: **352**
- `definition_ready`: **42**
- `waiting_data`: **242**
- `waiting_native_bindings`: **68**
- quest source crosswalk entries: **284**

Primary Oteryn paths:

- `content/quests/definitions/`
- `tools/content-schema/quest-authoring/samples/binding_packets/source/crosswalk.json`
- `docs/agents/evidence/OTV2-20261004-quest-component-status.md`

### Crystal summer branch corpus

**PROVEN**

`zimbadev/crystalserver@00ce02a57ca5a12e48f32a3476e37471167e4c3f` contains **122** top-level directories under:

`data-global/scripts/quests/`

These 122 directories are not equivalent to 122 newly introduced Summer Update 2026 quests. The branch contains inherited quest-script corpus plus new/update-specific content.

## High-level result

**DERIVED**

Classification of all 122 Crystal quest-script directories:

| Class | Count | Meaning |
| --- | ---: | --- |
| SOURCE_BOUND | 101 | Exact Crystal script-directory evidence is represented in the Oteryn quest source crosswalk. |
| DEFINITION_ONLY | 15 | A canonical Oteryn quest definition exists, but this exact Crystal directory is not bound into the canonical crosswalk. |
| INDIRECT | 1 | The directory is explicitly mapped to a differently named canonical quest. |
| AUX | 5 | The directory is a helper/system/world feature rather than a missing canonical quest. |
| **Total** | **122** | Full Crystal `data-global/scripts/quests/` top-level directory set. |

After excluding the five AUX directories, all **117/117** quest-bearing or quest-adjacent modules have at least a canonical Oteryn representation or explicit mapping.

This does **not** mean 117 quests are playable.

## Critical runtime caveat

**PROVEN**

The canonical source crosswalk explicitly states that it is evidence-only and does not assert executable or complete-stage admission.

For the source-bound quest set observed in this audit:

- `runtime_readiness` was not established by the crosswalk;
- `stage_full_coverage` was not established by the crosswalk;
- most canonical definitions remain `waiting_data`;
- the audit did not execute end-to-end quest gameplay.

**UNKNOWN**

The number of quests that are currently playable from start through final reward without stubs, missing bindings, missing encounters, broken world interactions, or incomplete reward delivery.

That is the next meaningful audit.

## Priority gap set: 15 DEFINITION_ONLY modules

These are the clearest follow-up targets because the canonical quest already exists but the exact Crystal directory is not admitted to the source crosswalk.

| Crystal directory | Canonical Oteryn quest | Current known gap |
| --- | --- | --- |
| `alawars_vault` | Alawar's Vault Quest | source-directory binding; reported source gaps; reward/native lowering |
| `behemoth` | Behemoth Quest | source-directory binding; reported source gaps; reward/native lowering |
| `dark_trails` | Dark Trails Quest | source binding; source-kind/log conflict; reported gaps; unknown requirement |
| `deeper_fibula` | Deeper Fibula Quest | source binding; item semantics; reward/native lowering |
| `demon_helmet` | Demon Helmet Quest | source binding; reported source gap |
| `devil_helmet` | Devil Helmet Quest | source binding; reported source gap; reward/native lowering |
| `draconia` | Draconia Quest | source binding; reported gap; item semantics; reward/native lowering |
| `hunter_outfits_quest` | Hunter Outfits Quest | source binding; reported source gap |
| `koshei_the_deathless_quest` | Koshei the Deathless Quest | source binding; reported source gap |
| `parchment_room` | Parchment Room Quest | source binding; reward/native lowering |
| `the_annihilator` | The Annihilator Quest | source binding; reported source gap; reward/native lowering |
| `the_pits_of_inferno_quest` | The Pits of Inferno Quest | source binding; source-kind/log conflict; reported gaps; reward/native lowering |
| `to_blind_the_enemy_quest` | To Blind the Enemy Quest | source binding; reported source gap; reward/native lowering |
| `triangle_tower_quest` | Triangle Tower Quest | source binding; reported gap; item semantics; reward/native lowering |
| `waterfall` | Waterfall Quest | source binding; reported source gap; reward/native lowering |

### Recommended order

**RECOMMENDATION**

1. `the_pits_of_inferno_quest`
2. `the_annihilator`
3. `dark_trails`
4. `draconia`
5. `koshei_the_deathless_quest`
6. remaining ten definition-only modules in bounded batches

Reason: close high-value, well-known quest chains first, then use the same binding/lowering pattern on the smaller set.

## INDIRECT module

### `mintwallin_quest`

**PROVEN**

The repository already contains an explicit directory-to-quest mapping:

`mintwallin_quest -> canary:quest/mintwallin_cyclops_quest`

in:

`tools/content-schema/quest-authoring/ots_readiness.py`

Canonical quest: **Mintwallin Cyclops Quest**.

**DERIVED**

This should not be counted as a missing quest. Follow-up work should determine why the relevant source interaction has not been admitted into the canonical crosswalk and whether the existing mapping is sufficient for native lowering.

## AUX modules: not missing quests

### `candia_bosses`

**PROVEN**

Contains Sugar Daddy boss helper scripts such as:

- `sugar_daddy_lever.lua`
- `sugar_daddy_teleports.lua`

The quest-authoring evidence lists the associated interaction as unlinked/generic rather than a canonical quest identity.

Classification: **AUX boss encounter/helper**.

### `edron_rope`

**PROVEN**

Contains `movements_rope.lua` and uses `Storage.EdronRopeQuest`.

The repository track-owner data identifies it as an Edron rope world feature rather than a canonical quest.

Classification: **AUX world feature**.

### `extension_mota`

**PROVEN**

Contains movement/teleport helper logic and appears in the unlinked-interaction set.

Classification: **AUX movement helper**.

### `rookgaard_quests`

**PROVEN**

The observed Crystal directory contains a reward helper (`quiver.lua`) and is present in the generic/unlinked interaction set.

Classification: **AUX Rookgaard helper**, not a standalone missing canonical quest.

### `soulpit`

**PROVEN**

Soulpit is represented as a separate system/encounter owner in Oteryn:

`rulesets/encounters/soulpit/index.json`

The quest-authoring documentation also identifies Soulpit interactions among generic/unlinked components rather than canonical quest content.

Classification: **AUX system/encounter**.

## Summer Update 2026 headline quests

The two Summer Update 2026 headline quest lines already have canonical authored definitions in Oteryn.

### Make Believe Quest

**PROVEN**

Canonical identity:

`oteryn:quest.authored.make_believe_quest`

Canonical location:

`content/quests/definitions/quests-00300-00351.json`

Observed state at the audit baseline:

- stages: **20**
- classification: `OTERYN_AUTHORED_APPROXIMATION`
- readiness: `waiting_native_bindings`
- `runtime_enabled: false`
- missing: `quest_native_lowering_missing`
- missing: `authored_trigger_and_delivery_bindings_missing`

Existing qualification evidence also records terminal-complete authored stage structure, but that does not make the quest runtime-ready.

### Shards of a Broken Moon Quest

**PROVEN**

Canonical identity:

`oteryn:quest.authored.shards_of_a_broken_moon_quest`

Canonical location:

`content/quests/definitions/quests-00300-00351.json`

Observed state at the audit baseline:

- stages: **16**
- classification: `OTERYN_AUTHORED_APPROXIMATION`
- readiness: `waiting_native_bindings`
- `runtime_enabled: false`
- missing: `quest_native_lowering_missing`
- missing: `authored_trigger_and_delivery_bindings_missing`

### Summer Update follow-up

**RECOMMENDATION**

Treat both headline quests as native-binding/runtime implementation work, not as missing-definition work.

Minimum closure for each should include:

1. exact native progress/state lowering;
2. trigger bindings;
3. NPC/dialogue bindings;
4. encounter/world-object bindings;
5. reward delivery bindings;
6. end-to-end replay from acceptance through terminal completion;
7. regression proof for repeatability/cooldowns/party semantics where applicable.

## SOURCE_BOUND directory inventory

**DERIVED**

The following 101 Crystal directories have exact source-path evidence represented by the canonical Oteryn crosswalk. Multi-quest mappings are expected for shared script directories and are not automatically defects.

- `25_years_of _tibia_quest` -> 25 Years of Tibia Quest
- `a_piece_of_cake` -> A Piece of Cake
- `a_pirates_tail_quest` -> A Pirate's Tail Quest; Grave Danger Quest
- `adventures_of_galthen` -> Adventures of Galthen Quest
- `an_uneasy_alliance` -> An Uneasy Alliance Quest
- `assassin_outfit` -> Assassin Outfits Quest
- `barbarian_test` -> Barbarian Test Quest; The Ice Islands Quest
- `between_the_lines` -> Between the Lines Quest
- `bigfoot_burden` -> Bigfoot's Burden Quest
- `blood_brothers_quest` -> Blood Brothers Quest
- `chayenne_realm` -> Realm of Dreams Quest
- `children_of_the_revolution` -> Children of the Revolution Quest; The New Frontier Quest; Wrath of the Emperor Quest
- `cradle_of_monsters` -> The Cradle of Monsters Quest
- `cults_of_tibia` -> Cults of Tibia Quest
- `dangerous_depth` -> Dangerous Depths Quest
- `dawnport` -> Dawnport Quest
- `deeplings_worldchange` -> Deeplings World Change
- `demon_oak` -> The Demon Oak Quest
- `desert_dungeon_quest` -> The Desert Dungeon Quest
- `dreamers_challenge_quest` -> Brotherhood Outfits Quest; Dreamer's Challenge Quest; Nightmare Outfits Quest
- `druid_outfits_quest` -> Outfit and Addon Quests
- `elemental_spheres` -> Elemental Spheres Quest
- `fathers_burden` -> A Father's Burden Quest
- `feaster_of_souls` -> Feaster of Souls Quest; Poltergeist Outfits Quest
- `ferumbras_ascension` -> Hero of Rathleton Quest
- `forgotten_knowledge` -> Forgotten Knowledge Quest
- `formogar_mine_hoist` -> Formorgar Mines Hoist Quest
- `giant_smithhammer` -> Giant Smithhammer Quest
- `grave_danger` -> Grave Danger Quest
- `grimvale` -> Grimvale Mini World Change; Grimvale Quest
- `heart_of_destruction` -> Heart of Destruction Quest
- `hero_of_rathleton` -> Hero of Rathleton Quest
- `hidden_threats` -> Tibia Tales
- `hot_cuisine` -> Hot Cuisine Quest
- `in_service_of_yalahar` -> In Service of Yalahar Quest
- `killing_in_the_name_of` -> Killing in the Name of... Quest
- `kilmaresh` -> Kilmaresh Quest
- `krailos` -> Krailos Quest
- `lions_rock` -> Lion's Rock Quest; Tibia Tales
- `liquid_black` -> Liquid Black Quest
- `marapur` -> Within the Tides Quest
- `mysterious_ornate` -> Opticording Sphere Quest
- `newhaven` -> Newhaven Quest
- `no_rest_for_the_wicked` -> No Rest for the Wicked Quest
- `oramond` -> Oramond Quest
- `others` -> Blood Brothers Quest; Oriental Outfits Quest; Secret Service Quest; Serpentine Tower Quest; Steal From Thieves Quest; The Explorer Society Quest; The Inquisition Quest; The Shattered Isles Quest; Tibia Tales
- `primal_ordeal_quest` -> Primal Ordeal Quest
- `raging_mage_tower` -> The Mage's Tower World Change
- `roshamuul_quest` -> Roshamuul Quest
- `rotten_blood_quest` -> Rotten Blood Quest
- `rottin_wood_and_married_men` -> Rottin Wood and the Married Men Quest
- `sea_of_light` -> Sea of Light Quest
- `secret_service` -> Secret Service Quest
- `shadows_of_yalahar_quest` -> Shadows of Yalahar Quest
- `soul_war` -> Soul War Quest
- `spike_tasks` -> Spike Task
- `spirit_hunters` -> Spirithunters Quest
- `svargrond_arena` -> Barbarian Arena Quest; The Ultimate Challenges
- `targuna` -> Targuna Quest
- `thais_lighthouse` -> Thais Lighthouse Quest
- `thais_quest` -> Life Ring Quest
- `the_ancient_tombs` -> The Ancient Tombs Quest
- `the_ape_city` -> The Ape City Quest
- `the_cursed_crystal` -> The Cursed Crystal Quest; Tibia Tales
- `the_djinn_war_quest` -> The Djinn War - Efreet Faction
- `the_dream_courts` -> The Dream Courts Quest
- `the_explorer_society` -> Outfit and Addon Quests; The Explorer Society Quest
- `the_first_dragon` -> The First Dragon Quest
- `the_gravedigger_of_drefia` -> The Gravedigger of Drefia Quest
- `the_great_dragon_hunt_quest` -> Adventurers Guild
- `the_hidden_city_of_beregar` -> The Hidden City of Beregar Quest
- `the_hunt_for_the_sea_serpent` -> The Hunt for the Sea Serpent Quest; Tibia Tales
- `the_ice_islands_quest` -> The Ice Islands Quest; The Shattered Isles Quest
- `the_inquisition_quest` -> The Inquisition Quest
- `the_isle_of_evil_quest` -> The Isle of Evil Quest
- `the_lost_brother` -> Adventurers Guild
- `the_new_frontier` -> The New Frontier Quest
- `the_order_of_lion_quest` -> The Order of the Lion Quest
- `the_outlaw_camp` -> The Outlaw Camp Quest
- `the_paradox_tower` -> The Paradox Tower Quest
- `the_postman_missions_quest` -> The Postman Missions Quest
- `the_primal_ordeal` -> Primal Ordeal Quest
- `the_queen_of_the_banshees` -> The Queen of the Banshees Quest
- `the_rookie_guard` -> The Rookie Guard Quest
- `the_secret_library_quest` -> The Secret Library Quest
- `the_shattered_isles_quest` -> The Shattered Isles Quest
- `the_spike_tasks` -> Spike Task
- `the_tainted_soul` -> The Tainted Souls Quest
- `the_thieves_guild_quest` -> The Thieves Guild Quest
- `the_travelling_trader` -> The Travelling Trader Quest
- `the_way_of_the_monk` -> The Way of the Monk Quest
- `their_masters_voice` -> Their Master's Voice World Change
- `threatened_dreams_quest` -> Threatened Dreams Quest
- `tibia_tales` -> Tibia Tales
- `tinder_box_quest_chyllfroest` -> Tinder Box Quest
- `tower_defence_quest` -> Tower Defence Quest
- `troll_sabotage` -> Troll Sabotage Quest
- `unnatural_selection` -> Unnatural Selection Quest
- `what_a_foolish_quest` -> What a Foolish Quest
- `white_pearl` -> Serpentine Tower Quest
- `wrath_of_the_emperor` -> Wrath of the Emperor Quest

## Suspicious/shared mappings that deserve manual review

**DERIVED**

The source-path comparison produced several mappings that are valid crosswalk associations but are surprising enough that a follow-up worker should not silently treat them as one-to-one quest ownership:

- `ferumbras_ascension` -> Hero of Rathleton Quest
- `a_pirates_tail_quest` -> A Pirate's Tail Quest + Grave Danger Quest
- `barbarian_test` -> Barbarian Test Quest + The Ice Islands Quest
- `children_of_the_revolution` -> three canonical quests
- `others` -> nine canonical quests
- `the_explorer_society` -> Outfit and Addon Quests + Explorer Society
- shared `Tibia Tales` links
- duplicate `spike_tasks` / `the_spike_tasks`

These associations may be legitimate shared-source relationships. The audit does not certify ownership granularity.

## Work queue

### Q1 - close the 15 exact-directory binding gaps

Goal: move each DEFINITION_ONLY module to an evidence-backed source association without inventing semantics.

For each module:

1. inspect the pinned Crystal files;
2. locate the canonical Oteryn quest identity;
3. map trigger/progress/reward/world-object evidence;
4. preserve every unresolved source fact;
5. update source crosswalk/binding packets through the existing quest-authoring pipeline;
6. do not infer runtime readiness from source association alone.

### Q2 - close Make Believe native bindings

Goal: convert the existing 20-stage authored quest into native executable content.

Acceptance evidence should include exact trigger/progress/dialogue/encounter/reward bindings and an end-to-end terminal-completion replay.

### Q3 - close Shards of a Broken Moon native bindings

Same shape as Q2 for the existing 16-stage authored quest.

### Q4 - full playable-path audit

This is the next major audit and should answer:

> Which canonical quests can a player actually start and complete end to end on the current Oteryn runtime?

Suggested classification:

- `PLAYABLE_VERIFIED`
- `START_BLOCKED`
- `PROGRESS_BLOCKED`
- `ENCOUNTER_BLOCKED`
- `WORLD_BINDING_BLOCKED`
- `REWARD_BLOCKED`
- `DATA_ONLY`
- `NOT_ASSESSED`

For every quest marked playable, require runtime evidence rather than catalogue presence.

### Q5 - review shared directory ownership

Manually inspect the suspicious/shared mappings above and distinguish:

- legitimate shared component;
- quest-chain dependency;
- donor-directory naming artifact;
- crosswalk over-association;
- generic interaction that should move to AUX/system ownership.

## Do not regress these conclusions

- Do not equate the 122 Crystal directories with 122 new Summer Update 2026 quests.
- Do not count the five AUX directories as missing quests.
- Do not count canonical definition presence as runtime completion.
- Do not mark SOURCE_BOUND as playable without runtime proof.
- Do not duplicate Make Believe or Shards identities; continue from their existing canonical authored definitions.
- Preserve pinned donor provenance and unknown fields during follow-up.

## Reproduction pointers

Crystal source root:

`zimbadev/crystalserver@00ce02a57ca5a12e48f32a3476e37471167e4c3f:data-global/scripts/quests/`

Oteryn canonical quest shards:

- `content/quests/definitions/quests-00000-00099.json`
- `content/quests/definitions/quests-00100-00199.json`
- `content/quests/definitions/quests-00200-00299.json`
- `content/quests/definitions/quests-00300-00351.json`

Oteryn source/binding evidence:

- `tools/content-schema/quest-authoring/samples/binding_packets/source/crosswalk.json`
- `tools/content-schema/quest-authoring/samples/readiness/readiness.json`
- `tools/content-schema/quest-authoring/samples/gap-triage/triage.json`
- `tools/content-schema/quest-authoring/samples/source-repair-r13/remaining-source-work.json`
- `tools/content-schema/quest-authoring/ots_readiness.py`
- `docs/agents/evidence/OTV2-20261004-quest-component-status.md`
- `docs/agents/evidence/OTV2-20261004-quest-completion-native-qualification.json`

System ownership evidence:

- `rulesets/encounters/soulpit/index.json`
- `docs/architecture/OTERYN_QUEST_AUTHORING_FORMAT_V1.md`

## Audit boundary

No runtime was started and no quest was executed during this audit. No claim of gameplay parity is made.

The useful conclusion is narrower: the canonical quest catalogue is already broad; the next work should focus on exact source binding, native lowering, and verified playable-path closure rather than importing more quest names.
