# CGAP-1 · Crystal content-gap audit

```yaml
task_id: CGAP-1
title: Crystal content-gap audit
mode: AUDIT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/happy-davinci-ijz3wy-cgap1
pr: null
base_sha: f067b43cfde9c2559454f4b78e1763ac6bfc11c0
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Codex Cloud
created_at: 2026-11-21T16:41:00Z
updated_at: 2026-11-21T16:56:00Z
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/CGAP-1-content-gap-audit.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories:
  - zimbadev/crystalserver
```

## Outcome

The census compares the requested main snapshot `f067b43cfde9c2559454f4b78e1763ac6bfc11c0` with Crystal `201189f871d23597b82caf88fd611454a2ddefb8`. It records both source coverage and specific Oteryn execution evidence. The largest physical gaps in the global map comparison are 984 NPC placement keys and 1,152 monster spawn-member occurrences. The claim probe bound all placements for 80 claim definitions on the pinned base bundle, with 257 other candidates missing at least one placement. Jaracal and Radiant Nimbus are the two missing runtime mount appearances.

## Architecture and source of truth

**Proven.** The pinned Crystal source archive matches all 166 retained raid files and both retained seasonal raid files byte-for-byte. The relevant imports are [raids-201189f.json](../../../imports/crystalserver/raids-201189f.json) and [seasonal-raids-201189f.json](../../../imports/crystalserver/seasonal-raids-201189f.json). For the remaining families, the archive supplies the Lua and XML at the same pin. The summer spawn hold inventory is historical import evidence from a different revision.

**Derived.** The comparison keys and their limitations are listed below. The main column gives donor units with a corresponding Oteryn witness. Main-only records are included in inventory notes. The missing column is the remainder of the stated denominator, including unresolved lexical entries where indicated. Each count comes from the commands in [Reproduction](#reproduction). Counts across rows use different units.

**Unknown.** Dynamic Lua arguments, runtime reachability beyond the checked registries and world bundle, and gameplay parity within an already-present row require targeted verification. The audit uses the base bundle and the materializable Item declarations for the claim binder's admission predicate. Node dispatch applies generation-specific Item policy and per-use guards.

## Findings

| Area and comparison unit | Donor / scope | Main match | Missing |
| --- | ---: | ---: | ---: |
| 1 · Normalized quest script families with a definition | 123 | 107 | 16 |
| 1 · Quest Lua files in those definition-matched families | 1,141 | 1,096 | 45 |
| 1 · Versioned quest storage blocks with a definition match | 223 | 204 | 19 |
| 1 · Donor script families with a runtime binding witness | 123 | 92 | 31 |
| 1 · Main quest definitions with a runtime binding witness | 352 | 242 | 110 |
| 1 · Main claim-bearing quests with a claim path witness | 135 | 60 | 75 |
| 1 · Donor reward-chest IDs with a served Crystal UID witness | 359 | 75 | 284 |
| 2 · NPC Lua constructor files with a definition match | 1,187 | 1,141 | 46 |
| 2 · Global NPC name-and-cell placement keys | 1,115 | 131 | 984 |
| 2 · Global placed NPC identities with any declared placement | 1,051 | 135 | 916 |
| 2 · NPC identities with shop offers | 330 | 301 | 29 |
| 2 · NPC identities with travel routes | 75 | 56 | 19 |
| 2 · NPC identities with ambient voices | 319 | 291 | 28 |
| 3 · Monster Lua constructor files with a definition match | 3,547 | 3,506 | 41 |
| 3 · Global monster spawn group centres | 55,112 | 54,668 | 444 |
| 3 · Global monster spawn-member occurrences | 88,944 | 87,792 | 1,152 |
| 4 · Materializable items with donor-backed facts and no screened UNKNOWN | 219 | 173 | 46 |
| 4 · Screened item/field pairs | 527 | 470 | 57 |
| 5 · Global monster loot rows by item identity | 17,360 | 16,971 | 389 |
| 5 · Alternate Crystal pack loot rows by item identity | 15,928 | 15,570 | 358 |
| 6 · Achievement call sites with a main output witness | 238 | 61 | 177 |
| 6 · Outfit call sites with a main output witness | 264 | 166 | 98 |
| 6 · Mount call sites with a main output witness | 29 | 0 | 29 |
| 6 · Configured global taming item/mount grant pairs | 42 | 42 | 0 |
| 6 · Configured direct-use mount script/target pairs | 8 | 2 | 6 |
| 6 · Crystal mount appearance IDs in main runtime | 252 | 250 | 2 |
| 7 · Global XML wave files with active raid catalogue evidence | 160 | 134 | 26 |
| 7 · Global Lua raid registration files with active catalogue evidence | 3 | 2 | 1 |
| 7 · Named raids with a donor timer-source witness | 6 | 0 | 6 |
| 7 · Global world-change scripts with a native owner | 7 | 7 | 0 |

### 1. Quests

The source inventory groups raw files in the two `scripts/quests` trees by normalized top-level name with explicit aliases. Shared folders such as `others` remain separate. The storage join covers named groups under the versioned `Storage.Quest` tables, with 2,126 distinct numeric IDs within those groups. Script-family counts measure association with a definition or runtime owner. A single witness can cover a family with several scripts.

The compiled quest loader and selector registries returned 24 authored quest binding rows, 1,025 world callbacks, 447 offered reward callbacks, 1,162 NPC rules and 61 death objectives. The mission catalogue contains 101 quest rows, 1,344 tracks and 3,270 transitions. Tracks by themselves do not establish a runtime trigger. The runtime join also checks the classic Rottin Wood and Troll Sabotage starts, Newhaven tutorial, arena dispatch and known event owners. Dawnport is explicitly excluded by `in_runtime_scope`. The readiness fields in `content/quests/definitions/index.json` do not enter the comparison.

The claim probe uses `bind_embedded_reward_claims` and `map::load` against the artifact digest committed in `content/world/pins/oteryn.json`. In the empty placement baseline it found 337 candidates and eight left out, including two owner exclusions and six variants. Against the base bundle, the 257 unbound claims produced 275 `NoEntry` placement rows and one `NoCrystalUniqueId` row. The quest-level claim join also credits offered NPC, world and carried-item value callbacks and Svargrond's own reward owner. The donor UID tally counts literal top-level chest action IDs with a reward table. Other implementations may use native selectors or replacement IDs, so the UID gap is a source-to-map reconciliation queue.

Biggest named script groups without a runtime witness by source file count are Explorer Society, Roshamuul, Secret Service, Soulpit, What a Foolish Quest, Dawnport (owner-excluded), No Rest for the Wicked, the alternate arena folder and Raging Mage Tower. The shared `others` folder is the largest unmatched bucket and should be triaged script by script. These joins establish a lower bound on observed runtime coverage. A quest can have authored stages or an alternate owner outside the queried registry surfaces.

<details>
<summary>Quest definition keys without an observed runtime binding or claim path</summary>

Keys below omit the `oteryn:quest.` prefix. The claim list covers definitions with explicit claims, unresolved claims or the reward-only kind. A quest with one claim witness may still have additional unsupported rewards.

**No binding witness**

```text
an_interest_in_botany_quest assassin_outfit authored.20_years_a_cook_quest
authored.cartography_101_quest authored.demon_s_lullaby authored.demon_wars_world_change
authored.devovorga_s_essence_mini_world_change authored.down_the_drain_mini_world_change
authored.draccoon_herald_outfits_quest authored.hive_born_world_change
authored.insectoid_outfits_quest authored.jungle_camp_mini_world_change
authored.kissing_a_pig_quest authored.lumberjack_mini_world_change authored.make_believe_quest
authored.measuring_tibia_quest authored.nimmersatt_s_lair authored.nomads_mini_world_change
authored.noodles_is_gone_mini_world_change authored.orcsoberfest
authored.poacher_caves_mini_world_change authored.podzilla_quest authored.rathleton_quest
authored.ravenous_madness_quest authored.rise_of_devovorga
authored.river_runs_deep_mini_world_change authored.rootwalker_outfits_quest
authored.royal_bounacean_advisor_outfits_quest authored.shards_of_a_broken_moon_quest
authored.small_ruby_quest authored.small_sapphire_quest authored.spider_nest_mini_world_change
authored.spirit_grounds_mini_world_change authored.spring_into_life
authored.the_fire_feathered_serpent_world_change authored.the_great_expedition
authored.the_lightbearer authored.the_repenters_quest authored.top_of_the_city_quest
authored.twisted_waters_world_change authored.war_against_the_hive_quest
authored.warpath_mini_world_change authored.waterskin_of_mead_quest awash_world_change
beggar_outfits_quest bloody_tusks_quest bored_mini_world_change chyllfroest_mini_world_change
dawnport_quest demon_outfits_quest devil_helmet_quest dream_warden_outfits_quest
druid_outfits_quest factions illuminator_outfit_quest knight_outfits_quest
newhaven_legion_helmet_quest no_rest_for_the_wicked_quest obsidian_knife_quest
opticording_sphere_quest oriental_outfits_quest ornamented_shield_quest overhunting_world_change
pick_quest poltergeist_outfits_quest power_ring_quest present_quest rapier_quest
rascoohan_outfits_quest ring_quest roshamuul_quest sanctuary_of_the_lizard_god scale_armor_quest
sea_of_light_quest secret_service_quest serpentine_tower_quest shaman_treasure_quest
short_sword_quest silver_amulet_quest silver_brooch_quest sinatuki_quest skeleton_decoration_quest
skull_of_ratha small_health_potion spirithunters_quest stealth_ring_quest steamship_world_change
steel_helmet_quest strong_potions_quest swamp_fever_world_change the_ape_city_quest
the_beginning_quest the_explorer_society_quest the_mummys_curse_world_change
the_roost_of_the_graveborn_quest the_shattered_isles_quest the_tainted_soul
the_ultimate_challenges thornfire_world_change time_ring_quest to_outfox_a_fox_quest
troll_cave_quest twenty_miles_beneath_the_sea_quest vampire_hunter_quest vampire_shield_quest
warmaster_outfits_quest wedding_ring_quest what_a_foolish_quest within_the_tides_quest
yalaharian_outfits_quest
```

**No claim path witness**

```text
alawar_s_vault_quest an_interest_in_botany_quest bear_room_quest beggar_outfits_quest
behemoth_quest berserk_potion_quest berserker_treasure_quest black_knight_quest
cults_of_tibia_quest dawnport_quest dead_archer_quest devil_helmet_quest dragon_corpse_quest
dragon_tower_quest dreamer_s_challenge_quest druid_outfits_quest edron_goblin_quest
emperor_s_cookies_quest fanfare_quest fire_axe_quest fishing_box_quest frost_dragon_quest
geomancer_quest ghoul_room_quest goblin_temple_quest hunter_outfits_quest inukaya_quest
iron_helmet_quest katana_quest knight_outfits_quest life_ring_quest lion_trophy_quest
longsword_quest mad_mage_room_quest medusa_shield_quest minotaur_hell_quest
newhaven_legion_helmet_quest oriental_outfits_quest ornamented_shield_quest power_ring_quest
present_quest ring_quest scale_armor_quest secret_service_quest serpentine_tower_quest
shadows_of_yalahar_quest shaman_treasure_quest short_sword_quest silver_amulet_quest
silver_brooch_quest skeleton_decoration_quest stealth_ring_quest steel_helmet_quest
strong_potions_quest targuna_quest thais_lighthouse_quest the_ape_city_quest
the_explorer_society_quest the_inquisition_quest the_outlaw_camp_quest the_pits_of_inferno_quest
the_queen_of_the_banshees_quest the_shattered_isles_quest the_thieves_guild_quest
threatened_dreams_quest time_ring_quest to_outfox_a_fox_quest tomes_of_knowledge_quest
triangle_tower_quest troll_cave_quest vampire_hunter_quest vampire_shield_quest waterfall_quest
wedding_ring_quest what_a_foolish_quest
```

</details>

### 2. NPCs

The donor has 1,189 Lua files including helpers. Main has 1,282 definitions and 151 distinct declared placement keys, including main-only cells. The explicit placement hold list is empty. The definition comparison uses Crystal file bindings with a normalized key fallback. The placement join also considers literal donor display names and absolute XML coordinates. The alternate pack has 118 placement keys with zero matches in the global target frame. Those need a map-owner decision before they enter a placement backlog.

Missing placements are the largest gap. The most repeated named groups are Lesser Messenger of Heaven and Lizard Tunnel Guard (six cells each), Repenter (five), Anaztassja Moroia and Gnome Trooper (four each), and Esrik (three). The other service gaps include Alesar shop, Buddel travel variants, and Dal the Huntress day/night voices. The shop and travel counts qualify an NPC with any admitted offer or route. They do not compare stock, prices, destinations, conditions or tickets. Ambient voices include the S10 overlay and use literal donor voice-table declarations. Check `content/services/{trade,travel}` and `content/npcs/s10-nonquest.json` alongside the placement shards.

### 3. Spawns and monsters

Main has 1,865 creature definitions and 54,691 published spawn declarations with 87,819 points. The donor has 3,550 monster Lua files across the global and alternate packs, with three outside the literal constructor scan. The point join is a multiset of group centre, creature key and absolute point. It tests incidence rather than timing, facing or group selection. The alternate map contributes 5,097 unmatched points in its own coordinate frame.

Largest global missing point groups by family are Rootthings (393 points for Amber Shaper, Nutshell and Bug Tracker combined), Raubritters (247 for Skirmisher, Marksman and Chastener), Quara raiders (185 for Raider, Looter and Plunderer) and Usurper Knight (37). Larger missing creature-definition clusters include No Rest for the Wicked, undeads and bosses. The largest explicit boss holds are Zugurosh (four) and Koshei the Deathless (two). Every one of the 35 points in [held-bosses.json](../../../content/world/spawns/held-bosses.json) has reason `encounter-bound` and remains outside ambient spawn shards by creature and cell.

[The summer import hold list](../../../imports/crystalserver/summer-update/map-content-linking/spawns/held-groups.json) covers 25 groups and 417 points. Twenty-one groups share `POINT_FIELDS_NOT_REPRESENTED_BY_ACCEPTED_CORE` and `SAME_CELL_SELECTION_REQUIRES_OWNING_CONTRACT`, including the 47-point group at `x33898_y32201_z6`. The other four group holds are creature binding failures involving Imp Minion (two groups), Crate and Enusat the Onyx Wing. Import holds can overlap the global point shortfall. Boss activation needs the encounter owner. Weighted same-cell groups need the spawn owner to preserve the source selection rule.

### 4. Items

The screen starts with 934 materializable Tibia item IDs and joins donor XML by ID or range. It checks the portable mappings in `tools/content-schema/item-authoring/crystal-field-dispositions.json`, with separate checks for explicit weapon and transform leaves and literal Lua use/equip witnesses. The Lua scan can identify candidate transformations in files with multiple action handlers. Review the action receiver and item ID before moving a candidate to portable data.

| Field family | Donor fact pairs | Main known | UNKNOWN |
| --- | ---: | ---: | ---: |
| Skill modifiers | 6 | 6 | 0 |
| Use/transform | 17 | 13 | 4 |
| Fluid source | 30 | 0 | 30 |
| Weapon | 451 | 428 | 23 |
| Equipment | 23 | 23 | 0 |

Biggest groups are world fluid sources (30), ammunition kind (12), max hit chance (11), literal Lua use transforms (three) and rotation (one). The fluid rows are owner-routed to placed WorldObject/Terrain by the field dispositions, so their Item `UNKNOWN` is expected pending a world-source coverage check. Ammunition kind and max hit chance are concentrated in projectile definitions such as flash, shiver and flaming arrows. Check launcher versus projectile ownership before promoting those values. The use group includes piggy bank and ectoplasm container. A `KNOWN` state shows authored presence and does not attest equality of values or completeness within an unmapped nested field.

### 5. Loot

Main has 1,292 loot tables, 1,271 nonempty and 19,541 entries. The global donor gap consists of 365 resolvable missing rows and 24 skipped item names or expressions. The alternate pack has 338 missing and 20 skipped. The two packs share source content and should be compared independently.

The largest global gaps are Enraged Arbaziloth (51), Death Dragon (24), Morbol (20), The Fire Empowered Duke (18), Poor Soul (16), Disgusting Ooze (15), Ayana the Crimson Curse (13), Gorga (12), Bulltaur Forgepriest (11) and Guard Captain Quaid (11). The scan parses literal Lua tables, attaches rows to the creature's actual loot reference where available, and consumes one main item occurrence per donor occurrence in each source file. It uses explicit IDs or unique `items.xml` names. It does not compare probabilities or quantities, assign ambiguous IDs, evaluate Lua, or grant nested-container parity based on a matched item ID.

### 6. Achievements, outfits and mounts

There are three distinct checks in this area. Donor grant call-site counts are lexical occurrences. A main output witness means an identified granter can produce that entitlement somewhere. Trigger parity needs its own comparison. For achievements, 148 calls have literal outputs without a witness and 29 are unresolved. For outfits those figures are 47 and 51. All five resolved direct literal mount calls lack a target witness in the audited granters, and 24 calls use unresolved arguments. The configured taming and direct-use rows are independent and recover executable paths hidden behind dynamic calls.

Achievement definitions cover 552 of 556 donor names. Runtime outfit appearances cover all 260 donor look IDs. The runtime mount catalogue has 250 rows and the separate declaration shard has 252. Appearance IDs establish the two specific runtime omissions, Jaracal (donor mount 250, look 1962) and Radiant Nimbus (donor mount 251, look 1975). Slug equality would miscount renamed mounts. Both donor mounts have quest acquisition types. Their XML rows establish identity, speed and premium facts. The acquisition route needs its own source-backed binding.

Highest-volume unresolved output groups are achievement grants from global NPCs (52 resolved call sites without output witness), common item actions (22) and global actions/other (14), plus global NPC outfits (25) and common item outfits (six). Mount follow-ups are the two appearance omissions and six direct-use script/target pairs without a selected policy and materializable-item witness. The latter are Haze, Neon Sparkid, Primal Demonosaur, Vortexion, Singeing Steed and Spirit of Purity. The source-policy screen follows `item_cosmetics::source_policy_allows` and joins the exact-pin native packet to currently materializable items and grant mappings. A rejected screen row needs the item generation and policy owner checked before concluding it has no acquisition path.

### 7. Raids and world events

The main catalogue has 240 records, 137 active. Every one of the 159 indexed donor wave files has a catalogue source reference. Of those, 134 have an active catalogue record. The additional, unindexed [Old Widow wave](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/raids/venore/the_old_widow.xml) brings the global inactive or uncatalogued wave total to 26. The alternate pack's two indexed waves use its own source paths and neither has an active exact-path match.

The biggest gap groups are Ankrahmun (four), Carlin (four), Ab'Dendriel (three), Venore (three), Farmine (two), Port Hope (two) and Svargrond (two). The remaining notable groups are Man in the Cave (step-in trigger, absent from the active Lua raid catalogue), the named `raids_schedule.lua` timers, and the alternate-pack indexed XML waves. The timer-source gap includes Midnight Panther, Draptor and Undead Cavebear, which also have XML wave evidence. Timer presence requires the specific scheduling witness, independent of wave presence. Inactive wiki-only rows often lack a resolved wave, native area and schedule. The seven donor world-change scripts tested all have a runtime owner through the world-change registry, daily boot or time-gated NPC placement. The separate seasonal scheduler contains Autumn Vintage, A Piece of Cake, Halloween and Christmas.

## Work packages

Each row is one proposed Codex Cloud lane, targeted at approximately 500 hand-written changed lines including focused validation. Generated data is sized by the stated slice. Shared shard or loader writes are dependencies on the relevant owner and should be integrated in order.

| Package | Slice and owned paths | Dependencies |
| --- | --- | --- |
| Q1 · Explorer Society progression | One start-to-report branch. Own new Explorer Society records under `content/quests/npc-callbacks/` and `content/quests/world-callbacks/` and the matching registration in `crates/gameplay/quests/src/quest/`. | Place the selected NPCs and map anchor first. Reuse existing callback admission. |
| Q2 · Classic chest placement | One regional cohort of `NoEntry` claims. Own that region's inputs in `content/world/placements/` and the matching entries in `content/quests/native-reward-placement-aliases.json`. | Verified appearance, position and UID, materializable reward and container children. Serialize bundle pin generation. |
| N1 · Repeatable NPC placement | One global region anchored by Lizard Tunnel Guard and adjacent missing NPCs. Own placement authoring for that region in `content/world/npc-placements/`. | Exact map floor and NPC definition; place seasonal and time-gated NPCs through their existing owner. |
| N2 · Merchant offers | Alesar and one local trader cohort. Own those records under `content/services/trade/` and their `content/npcs/definitions/` attachments. | Item IDs and shop unlock conditions; N1 or equivalent placement evidence before playability verification. |
| N3 · Buddel travel | Own the Buddel service cohort under `content/services/travel/` with the corresponding NPC links. | Source fares, endpoints, quest guards and a paid-trip receipt through the existing travel owner. |
| N4 · Dal day/night voices | Own the Dal rows in `content/npcs/s10-nonquest.json` and related NPC definitions. | Confirm time-gated identity and the approved voice fields. |
| S1 · Rootthing spawn cells | Own the Rootthing cohort in `content/world/spawns/` and the matching authored inputs. | Valid terrain and creature identities. Weighted same-cell groups depend on an approved selection contract. |
| S2 · Zugurosh encounter | Own the Zugurosh encounter slice in `content/encounters/` and `crates/gameplay/encounters/src/` with its `held-bosses.json` records. | Shared boss room entry, reset and kill-credit mechanic. Keep boss admission with the encounter owner. |
| I1 · Projectile semantics | Own the affected XML-to-weapon mappings in `tools/content-schema/item-authoring/` and regenerated `content/items/definitions/`. | Validate whether each donor fact belongs to launcher or ammunition. Follow with a separate placed-fluid or use-transform lane where warranted. |
| L1 · Arbaziloth loot | Own Arbaziloth tables in `content/loot/` and source binding evidence in `tools/content-schema/monster-authoring/`. | Resolve phase-specific corpse ownership and item admission before importing quantities and rates. |
| G1 · Item-action achievements | Own one common item action family in `crates/gameplay/items/src/` and its native item/achievement authoring inputs. | Existing atomic achievement-progress owner and materializable inputs. |
| G2 · Mount appearances | Own the Jaracal and Radiant Nimbus catalogue mappings in `content/cosmetics/{mounts,outfits}/` and `tools/content-schema/quest-authoring/qengine_mounts.py`. | Admitted appearances and alias resolution. Acquisition follow-up depends on quest/taming evidence for each mount. |
| R1 · Ankrahmun XML waves | Own the Ankrahmun cohort in `content/encounters/raids/` and its generator inputs in `tools/content-migration/raid_catalogue.py`. | Native area and creature bindings, plus wave execution support for the chosen XML shape. |
| R2 · Scheduled raid cohort | Own Midnight Panther and Draptor timer admission in `content/encounters/raids/` and `apps/game-server/src/node/raid_scheduler.rs`. | Same-head wave identity plus existing scheduler clock, duplicate suppression and reset policy. |

## Acceptance and scope

- [x] Compare the two requested immutable revisions and record counts, joins, exceptions and commands.
- [x] List quest binding and reward claim gaps under explicit witness rules, and propose lane-sized work.
- [x] Deliver one findings file as the sole tracked change.

High-risk authority/recovery qualification and mechanic reference-parity tests are **NOT_APPLICABLE** for this read-only audit. The world build and quest probes use local files and do not exercise a player session. The task record uses the requested branch-only delivery. Commit identity and remote verification are recorded externally to the commit.

## Validation

- Exact-base environment preflight passed. The retained `start.sh` passed when invoked with Bash.
- Offline quest, content, world and compiler builds passed. The quest loader, claim-binder probes and Python census completed. The rebuilt bundle passed `map::load` against the tracked pin.
- `python3 tools/agents/validate_governance.py` passed.
- `python3 -m unittest discover -s tools/agents/tests` passed (135 tests).
- `python3 tools/repository/reliability_checklist.py --paths docs/agents/tasks/CGAP-1-content-gap-audit.md` selected the governance checks above.
- `git diff --cached --check` passed. Self-review checked the sole changed path, table values against the census output and the scope of each claim. Exact-head PR CI and independent PR review are **NOT_APPLICABLE** to the branch-only findings file.

## Reproduction

All content reads are relative to the pinned Game checkout. Save the code blocks under the indicated temporary paths. The audit used the existing environment and wrote scratch output under `/tmp`.

```bash
set -eo pipefail
export OTERYN_AUTO_START=0
source /workspace/oteryn-env/env.sh
cd /workspace/Oteryn-Game
test "$(git rev-parse HEAD)" = f067b43cfde9c2559454f4b78e1763ac6bfc11c0
mkdir -p /tmp/cgap-donor
curl -fsSL https://codeload.github.com/zimbadev/crystalserver/tar.gz/201189f871d23597b82caf88fd611454a2ddefb8 |
  tar xz -C /tmp/cgap-donor --strip-components=1 --wildcards '*/data*'
cargo build --offline --locked -p oteryn-gameplay-quests --message-format=json > /tmp/cgap-cargo.jsonl
cargo build --offline --locked -p oteryn-gameplay-content --message-format=json > /tmp/cgap-content-cargo.jsonl
cargo build --offline --locked -p oteryn-gameplay-world -p oteryn-world-bundle-compiler --message-format=json > /tmp/cgap-world-cargo.jsonl
python3 /tmp/cgap-rust-link.py /tmp/cgap-cargo.jsonl /tmp/cgap-quests.rs /tmp/cgap-quests oteryn_gameplay_quests
/tmp/cgap-quests > /tmp/cgap-quests.tsv
cut -f1 /tmp/cgap-quests.tsv | sort | uniq -c
python3 /tmp/cgap.py /tmp/cgap-donor > /tmp/cgap-preliminary.out
python3 /tmp/cgap-rust-link.py /tmp/cgap-content-cargo.jsonl /tmp/cgap-claim-candidates.rs /tmp/cgap-claim-candidates oteryn_gameplay_content
/tmp/cgap-claim-candidates > /tmp/cgap-claim-candidates.tsv
/workspace/oteryn-env/target/debug/oteryn-world-bundle-compiler compile . content/world/pins/oteryn.identity.json /tmp/cgap-world.otwb non-production
python3 /tmp/cgap-rust-link.py /tmp/cgap-world-cargo.jsonl /tmp/cgap-claims.rs /tmp/cgap-claims oteryn_gameplay_content oteryn_gameplay_world serde_json
/tmp/cgap-claims /tmp/cgap-world.otwb > /tmp/cgap-claims.tsv
awk -F '\t' 'NF==3 {print $1,$3}' /tmp/cgap-claims.tsv | sort | uniq -c
python3 /tmp/cgap.py /tmp/cgap-donor > /tmp/cgap.out
cat /tmp/cgap.out
```

<details>
<summary>Scratch census and probe source</summary>

`/tmp/cgap-rust-link.py`

```python
import json
import subprocess
import sys
from pathlib import Path

log, source, output, *crates = sys.argv[1:]
artifacts = [r['filenames'] for line in open(log) for r in [json.loads(line)]
             if r.get('reason')=='compiler-artifact']
directories = sorted({Path(f).parent for fs in artifacts for f in fs})
flags = [f'-Ldependency={d}' for d in directories]
for crate in crates:
    libraries = [f for fs in artifacts for f in fs
                 if Path(f).name.startswith(f'lib{crate}') and f.endswith('.rlib')
                 and Path(f).stem.removeprefix('lib').split('-')[0]==crate]
    assert libraries, crate
    flags += ['--extern',f'{crate}={libraries[0]}']
subprocess.run(['rustc','--edition=2024',*flags,source,'-o',output],check=True)
```

`/tmp/cgap-quests.rs`

```rust
use oteryn_gameplay_quests::quest::{death_objectives, loader, npc_rules, reward_callbacks, world_callbacks};
use npc_rules::NpcRuleEffect;

fn main() {
    let lowered = loader::load_embedded_quest_state("content-1").unwrap();
    let catalogue = lowered.catalogue();
    if let Some(bindings) = catalogue.authored_bindings() {
        for q in &bindings.quests {
            println!("authored\t{}\t{}", q.quest, q.stages.len());
        }
    }
    for callback in world_callbacks::load_for(catalogue).unwrap() {
        println!("world\t{}\t{}", callback.quest, callback.key);
        if callback.item_reward.is_some() || callback.effects.outfit_reward.is_some()
            || !callback.achievements.is_empty() || callback.experience.is_some()
        {
            println!("value\t{}", callback.quest);
        }
        for t in callback.storage {
            if let Some(track) = catalogue.track(&t.track) {
                println!("track_owner\t{}", track.quest);
            }
        }
        for a in callback.achievements {
            println!("achievement\t{a}");
        }
        if let Some(o) = &callback.effects.outfit_reward {
            println!("outfit\t{}", o.outfit);
        }
    }
    for callback in reward_callbacks::offered_for(catalogue).unwrap() {
        println!("reward\t{}\t{}", callback.quest, callback.key);
        if callback.give.is_some() || callback.outfit.is_some() || !callback.outfits.is_empty()
            || callback.achievement.is_some() || !callback.achievement_progress.is_empty()
            || callback.experience.is_some()
        {
            println!("value\t{}", callback.quest);
        }
        for o in callback.cosmetic_targets() {
            println!("outfit\t{}", o.outfit);
        }
        if let Some(a) = callback.achievement {
            println!("achievement\t{a}");
        }
        for a in callback.achievement_progress {
            println!("achievement\t{}", a.key);
        }
    }
    for objective in death_objectives::load().unwrap() {
        println!("death\t{}\t{}", objective.quest, objective.key);
    }
    for program in npc_rules::NpcRuleCatalog::runtime().unwrap().programs() {
        for rule in &program.document.rules {
            println!("npc\t{}\t{}", rule.quest, rule.key);
            for effect in &rule.effects {
                match effect {
                    NpcRuleEffect::Achievement { name } => {
                        println!("achievement\t{name}");
                        println!("value\t{}", rule.quest);
                    }
                    NpcRuleEffect::Outfit { look_type, .. } | NpcRuleEffect::OutfitAddon { look_type, .. } => {
                        println!("outfit_id\t{look_type}");
                        println!("value\t{}", rule.quest);
                    }
                    NpcRuleEffect::GiveItem { .. } | NpcRuleEffect::GiveMoney { .. } | NpcRuleEffect::Experience { .. } => {
                        println!("value\t{}", rule.quest);
                    }
                    _ => {}
                }
            }
        }
    }
}
```

`/tmp/cgap-claim-candidates.rs`

```rust
use std::collections::BTreeSet;
use oteryn_gameplay_content::content::world_reward_claims::{
    bind_embedded_reward_claims, RewardClaimBase, RewardClaimTile, UniqueEntry,
};

struct Empty;
impl RewardClaimBase for Empty {
    type Tile<'a> = Empty;
    fn tile(&self, _: u16, _: u16, _: i8) -> Option<Empty> { None }
}
impl RewardClaimTile for Empty {
    fn depths(&self) -> &[u8] { &[] }
    fn unique(&self, _: u8) -> Option<UniqueEntry> { None }
}

fn emit(base: &impl RewardClaimBase, keys: &BTreeSet<&str>) {
    let report = bind_embedded_reward_claims(base, &|i| keys.contains(i.key().as_str())).unwrap();
    for row in report.served {
        println!("served\t{}", row.key.as_str());
    }
    for row in report.unbound {
        println!("unbound\t{}\t{:?}",row.claim,row.reason);
    }
    for (k,why) in report.left_out {
        println!("left_out\t{k}\t{why:?}");
    }
}
fn main() {
    let input = std::fs::read_to_string("/tmp/cgap-item-keys.txt").unwrap();
    let keys: BTreeSet<_> = input.lines().collect();
    emit(&Empty,&keys);
}
```

`/tmp/cgap-claims.rs`

```rust
use std::collections::BTreeSet;
use oteryn_gameplay_content::content::world_reward_claims::{
    bind_embedded_reward_claims, RewardClaimBase, RewardClaimTile, UniqueEntry,
};
use oteryn_gameplay_world::map::{self, BundlePins};

struct Empty;
impl RewardClaimBase for Empty {
    type Tile<'a> = Empty;
    fn tile(&self, _: u16, _: u16, _: i8) -> Option<Empty> { None }
}
impl RewardClaimTile for Empty {
    fn depths(&self) -> &[u8] { &[] }
    fn unique(&self, _: u8) -> Option<UniqueEntry> { None }
}

fn emit(base: &impl RewardClaimBase, keys: &BTreeSet<&str>) {
    let report = bind_embedded_reward_claims(base, &|i| keys.contains(i.key().as_str())).unwrap();
    for row in report.served {
        println!("served\t{}", row.key.as_str());
    }
    for row in report.unbound {
        println!("unbound\t{}\t{:?}",row.claim,row.reason);
    }
    for (k,why) in report.left_out {
        println!("left_out\t{k}\t{why:?}");
    }
}
fn main() {
    let input = std::fs::read_to_string("/tmp/cgap-item-keys.txt").unwrap();
    let keys: BTreeSet<_> = input.lines().collect();
    if let Some(filename) = std::env::args().nth(1) {
        let raw = std::fs::read(filename).unwrap();
        let pin: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string("content/world/pins/oteryn.json").unwrap()
        ).unwrap();
        let val = |s: &str| pin[s].as_str().unwrap().to_owned();
        let digest = val("digest");
        let bytes = std::array::from_fn(|i| u8::from_str_radix(&digest[i*2..i*2+2],16).unwrap());
        let map = map::load(&raw, &BundlePins {
            digest: bytes, project_format_version: val("project_format_version"),
            world_schema_version: val("world_schema_version"),
            content_revision: val("content_revision"), production: pin["production"].as_bool().unwrap(),
        }).unwrap();
        emit(&map,&keys);
    } else {
        emit(&Empty,&keys);
    }
}
```

`/tmp/cgap.py`

```python
from collections import Counter, defaultdict
from pathlib import Path
import json, re, sys, unicodedata
import xml.etree.ElementTree as ET

D = Path(sys.argv[1])
C = Path('content')
G = D/'data-global'
PIN = '201189f871d23597b82caf88fd611454a2ddefb8'

def read(p):
    return Path(p).read_text(encoding='utf-8', errors='replace')

def load(p):
    return json.loads(read(p))

def rows(pattern, wrapper=None):
    return [r[wrapper] if wrapper else r for p in sorted(C.glob(pattern))
            for r in load(p)['records']]

def slug(s):
    return re.sub(r'[^a-z0-9]+', '_', unicodedata.normalize('NFKD', s).encode(
        'ascii', 'ignore').decode().lower()).strip('_')

def tail(key):
    return key.rsplit('.', 1)[-1]

def key(r):
    return r['identity']['key']

def xml(p):
    return ET.parse(p).getroot()

def lua(p):
    return re.sub(r'--\[(=*)\[.*?\]\1\]|^[ \t]*--[^\n]*', '', read(p), flags=re.S|re.M)

def measure(label, donor, main):
    print(label, len(donor), len(donor & main), len(donor-main))
    return donor-main

def rank(label, seq):
    print(label, sorted(Counter(seq).items(), key=lambda p: (-p[1], p[0]))[:10])

def nodes(o):
    if isinstance(o, dict):
        yield o
        for v in o.values():
            yield from nodes(v)
    elif isinstance(o, list):
        for v in o:
            yield from nodes(v)

def loc(r):
    return tuple(r['cell'][k] for k in ('x','y','floor'))

def points(file, tag):
    for group in xml(file).findall(tag):
        x,y,z = (int(group.attrib['center'+a]) for a in 'xyz')
        for row in group.findall(tag):
            a = row.attrib
            yield ((x,y,z), slug(a['name']), x+int(a.get('x',0)),
                   y+int(a.get('y',0)), int(a.get('z',z)))

# 1. Script families and storage blocks, with explicit title aliases.
aliases = dict(line.split(':') for line in """
25_years_of_tibia:25_years_of_tibia
bigfoot_burden:bigfoot_s_burden
inqusition:inquisition
order_of_lion:order_of_the_lion
marapur:within_the_tides
great_dragon_hunt:great_dragon_hunt
spike_tasks:spike_task
djinn_war:djinn_war_efreet_faction
a_piece_of_cake:piece_of_cake
adventures_of_galthen:adventures_of_galthen
alawars_vault:alawar_s_vault
barbarian_test:barbarian_test
dangerous_depth:dangerous_depths
deeplings_worldchange:liquid_black
dreamers_challenge:dreamer_s_challenge
fathers_burden:a_father_s_burden
killing_in_the_name_of:killing_in_the_name_of
lions_rock:lion_s_rock
mintwallin:mintwallin
primal_ordeal:primal_ordeal
rookie_guard:rookie_guard
rottin_wood_and_married_men:rottin_wood_and_the_married_men
svargrond_arena:barbarian_arena
too_hot_to_handle:too_hot_to_handle
troll_sabotage:troll_sabotage
""".split())
def qnorm(s):
    s = slug(s)
    s = re.sub(r'_quest$', '', s)
    s = re.sub(r'^the_', '', s)
    return aliases.get(s,s).replace('_','')

roots = (G/'scripts/quests', D/'data-crystal/scripts/quests')
qdirs = defaultdict(list)
for root in roots:
    for p in sorted(root.rglob('*.lua')):
        rel = p.relative_to(root)
        if len(rel.parts)>1:
            qdirs[qnorm(rel.parts[0])].append(p)
        else:
            qdirs[qnorm(p.stem)].append(p)
qd = rows('quests/definitions/quests-*.json','definition')
byq = defaultdict(set)
for r in qd:
    for alias in [tail(key(r)), r['display_name']]:
        byq[qnorm(alias)].add(key(r))
print('quest main definitions',len(qd))
qmissing = measure('quest script families with definition',set(qdirs),set(byq))
print('quest Lua files',sum(map(len,qdirs.values())),
      sum(len(v) for k,v in qdirs.items() if k in byq),sum(len(qdirs[k]) for k in qmissing))
rank('quest missing definition by files',(p.parent.name for k in qmissing for p in qdirs[k]))
print('quest unmatched',sorted({p.parent.name if p.parent not in roots else p.stem for k in qmissing for p in qdirs[k]}))
storage_groups = set()
storage_leaves = set()
for p in sorted(D.glob('data-*/lib/core/storages.lua')):
    base = p.relative_to(D).parts[0]
    text = lua(p)
    for block in re.findall(r'^\tQuest = \{(.*?)^\t\},',text,re.S|re.M):
        version = ''
        family = ''
        for line in block.splitlines():
            if m:=re.match(r'^\t\t((?:Pre)?U\d[\w]*) = \{',line):
                version, family = m[1], ''
            elif line.startswith('\t\t},'):
                version, family = '', ''
            elif version and (m:=re.match(r'^\t\t\t(\w+) = \{',line)):
                family = m[1]
                storage_groups.add((base,version,family))
            elif line.startswith('\t\t\t},'):
                family = ''
            elif family and (m:=re.match(r'^\s+\w+ = (\d+)\s*,',line)):
                storage_leaves.add((base,version,family,int(m[1])))
sc = lambda s: qnorm(re.sub(r'(?<=[a-z0-9])(?=[A-Z])','_',s))
measure('versioned quest storage groups with matching definition',storage_groups,
        {r for r in storage_groups if sc(r[2]) in byq})
print('distinct numeric IDs within those groups',len(storage_leaves))
state = load(C/'quests/missions/quest-state.json')['quests']
print('mission-catalogue quests, tracks, transition rows',
      len(state),sum(len(r['tracks']) for r in state),sum(len(r['transitions']) for r in state))
runtime = defaultdict(list)
for line in read('/tmp/cgap-quests.tsv').splitlines():
    tag, *record = line.split('\t')
    runtime[tag].append(record)
print('runtime registry rows',sorted((k,len(v)) for k,v in runtime.items()))
ids = {key(r) for r in qd}
def owners(q):
    if q in ids:
        return {q}
    if q.startswith('oteryn:quest/'):
        return byq.get(qnorm(q.removeprefix('oteryn:quest/')),set())
    return set()
binding = {q for tag in ('authored','world','reward','death','npc','track_owner')
           for identity,*_ in runtime[tag] for q in owners(identity)}
rewards = {q for identity,*_ in runtime['value'] for q in owners(identity)}
claim_path = Path('/tmp/cgap-claims.tsv')
claim_status = defaultdict(set)
if not claim_path.exists():
    claim_path = Path('/tmp/cgap-claim-candidates.tsv')
if claim_path.exists():
    for line in read(claim_path).splitlines():
        tag, identity, *_ = line.split('\t')
        claim_status[tag].add(identity)
claim_results = claim_status['served'] if claim_path.name=='cgap-claims.tsv' else claim_status['unbound']
print('claim probe',claim_path.name,sorted((k,len(v)) for k,v in claim_status.items()))
claims = [r for p in sorted(C.glob('interactions/reward_claims/*.json'))
          if p.name!='index.json' for row in load(p)['records']
          for r in [row['definition']] if key(r) in claim_results]
claim_keys = {key(r) for r in claims}
rewards |= {key(r) for r in qd if any(c['key'] in claim_keys for c in r.get('claims',[]))}
uid_claims = set()
for r in claims:
    q = (r.get('quest') or {}).get('key','')
    rewards.update(byq.get(qnorm(q.rsplit('/',1)[-1]),set()))
    for place in r.get('placements',[]):
        uid_claims.update(b['unique_id'] for b in place.get('source_binding',{}).get('legacy_unique_ids',[])
                          if b['server']=='crystalserver')
    uid_claims.update(p['unique_id'] for p in r.get('native_contents',{}).get('placements',[])
                      if isinstance(p.get('unique_id'),int))
chest = lua(G/'startup/tables/chest.lua')
parts = re.split(r'^\t\[(\d+)\]\s*=\s*\{',chest,flags=re.M)
chest_uids = {int(i) for i,t in zip(parts[1::2],parts[2::2]) if re.search(r'\breward\s*=\s*\{',t)}
measure('donor literal reward-chest IDs with Crystal claim UID witness',chest_uids,uid_claims)
native_event_witness = {
    'nightmare_isles':('crates/gameplay/dispatch/src/gameplay_transport/daily_world_gates.rs','nightmare_isles.lua'),
    'oriental_trader':('apps/game-server/src/node/serve/daily_world_changes.rs','changes.oriental_trader'),
    'spawns_npc_by_time':('apps/game-server/src/node/serve/bundle_npcs.rs','spawns_npc_by_time.lua'),
    'thawing':('apps/game-server/src/node/serve/channel_boot.rs','daily.thawing'),
}
other_events = {e for e,(p,w) in native_event_witness.items() if w in read(p)}
runtime_world_changes = {r['key'] for r in load(C/'encounters/world-changes/runtime.json')['changes']}
binding.update(q for event in runtime_world_changes|other_events for alias in (event,event+'_world_change',event+'_mini_world_change')
               for q in byq.get(qnorm(alias),set()))
binding.update(byq.get(qnorm('piece_of_cake'),set()) if any(
    r.get('event')=='piece_of_cake' for r in load(C/'encounters/event-schedule/schedule.json')['seasonal']) else set())
classic = {'rottin-wood':'rottin_wood_and_the_married_men','troll-sabotage':'troll_sabotage'}
classic_loader = read('crates/gameplay/quests/src/quest/classic_dialogue.rs')
for file,alias in classic.items():
    if f'content/quests/{file}-starts.json' in classic_loader:
        binding.update(byq.get(qnorm(alias),set()))
if 'Newhaven tutorial rules' in read('crates/gameplay/quests/src/tutorial/mod.rs'):
    for alias in ('newhaven','tutorial'):
        binding.update(byq.get(qnorm(alias),set()))
arena = load(C/'quests/arena-completion.json')
if arena['trophy_claims'] and '.claim_arena_reward(' in read(
        'crates/gameplay/dispatch/src/gameplay_transport/arena_match.rs'):
    rewards.add(arena['quest'])
binding |= rewards
binding.discard('oteryn:quest.dawnport_quest')
rewards.discard('oteryn:quest.dawnport_quest')
ids = {key(r) for r in qd}
qm = measure('main quests with identified runtime binding',ids,binding)
qr = measure('donor script families with identified runtime binding',set(qdirs),
             {n for n,keys in byq.items() if keys & binding})
print('main no binding witness', ' '.join(sorted(q.removeprefix('oteryn:quest.') for q in qm)))
expected = {key(r) for r in qd if r.get('claims') or r.get('unresolved_claims') or r.get('kind')=='reward_only'}
no_claim = measure('main claim-bearing quests with identified claim path',expected,rewards)
print('main no claim witness',' '.join(sorted(q.removeprefix('oteryn:quest.') for q in no_claim)))
rank('quest missing runtime by files',(p.parent.name for k in qr for p in qdirs[k]))

# 2. NPC file registry and global map placement incidence.
npc_paths = sorted(p for base in (G,D/'data-crystal') for p in (base/'npc').rglob('*.lua'))
npc = rows('npcs/definitions/npcs-*.json')
npc_files = [p for p in npc_paths if 'Game.createNpcType' in lua(p)]
nf = {slug(p.stem) for p in npc_files}
npkeys = {tail(key(r['declaration'])) for r in npc}
main_npc = {slug(b['external_id']):tail(key(r['declaration'])) for r in npc
            for b in r.get('source_bindings',[]) if b['identity_namespace']=='crystalserver/npc-file'
            and b['disposition']=='EXACT'}
for n in nf & npkeys:
    main_npc.setdefault(n,n)
np_missing = measure('NPC files by stem with definition',nf,set(main_npc))
print('NPC raw Lua files, constructor files, main defs',len(npc_paths),len(npc_files),len(npc))
measure('NPC constructor files with definition',set(npc_files),{p for p in npc_files if slug(p.stem) in main_npc})
npc_names = {}
for p in npc_files:
    t = lua(p)
    m = re.search(r'(?:internalNpcName\s*=|createNpcType\s*\()\s*(["\'])(.*?)\1',t)
    if m and slug(p.stem) in main_npc:
        npc_names.setdefault(slug(m[2]),main_npc[slug(p.stem)])
db = list(points(G/'world/world-npc.xml','npc'))
mp = {(tail(r['npc']),*loc(r)) for r in rows('world/npc-placements/placements-*.json','declaration')}
dp = {(npc_names.get(n,main_npc.get(n,n)),x,y,z) for _,n,x,y,z in db}
npplace = measure('NPC global placement name+cell',dp,mp)
measure('NPC identities on global map with any placement', {p[0] for p in dp},{p[0] for p in mp})
print('placement rows donor/main/explicit holds',len(db),len(mp),len(load(C/'world/npc-placements/held.json')['held']))
rank('NPC placement gaps by repeated identity',(n for n,*_ in npplace))
print('NPC unbound file examples',sorted(n for n in np_missing if n)[:8])
for f in sorted((D/'data-crystal/world').glob('*-npc.xml')):
    alt_places = {(npc_names.get(n,main_npc.get(n,n)),x,y,z) for _,n,x,y,z in points(f,'npc')}
    measure('alternate NPC placement name+cell',alt_places,mp)
for label,pattern,main in [
        ('shops',r'\.shop\s*=\s*\{|:addShopItem\s*\(',{tail(key(r)) for r in rows('services/trade/trade-*.json','declaration') if r.get('offers')}),
        ('travel',r'addTravelKeyword\s*\(\s*["\']|StdModule\.travel|:addTravel\s*\(',{tail(key(r)) for r in rows('services/travel/travel-*.json','declaration') if r.get('routes')}),
        ('voices',r'\.voices\s*=\s*\{',{tail(key(r['declaration'])) for r in npc if any(
            f['field_path'].endswith('.voice_lines') and f['value']['value'] not in ('[]','') for f in r['declaration']['fields'])}
         | {tail(r['key']) for r in load(C/'npcs/s10-nonquest.json')['voices']})]:
    sources = {main_npc.get(slug(p.stem),slug(p.stem)) for p in npc_paths if re.search(pattern,lua(p))}
    absent = measure('NPC '+label,sources,main)
    print('NPC main '+label,len(main),'examples',sorted(absent)[:8])

# 3. Monster constructors and global spawn placement multiset.
monster_files = sorted(p for base in (G,D/'data-crystal') for p in (base/'monster').rglob('*.lua'))
ctor = r'Game\.createMonsterType\s*\(\s*(["\'])(.*?)\1'
monsters = {p:slug(m[2]) for p in monster_files if (m:=re.search(ctor,lua(p)))}
creature = rows('creatures/definitions/creatures-*.json')
creature_keys = {tail(key(r['definition'])) for r in creature}
paths = {b['external_id']:tail(key(r['definition'])) for r in creature for b in r.get('source_bindings',[])
         if b['identity_namespace']=='crystalserver/monster-file' and b['disposition']=='EXACT'}
covered = {p for p,n in monsters.items() if n in creature_keys or str(p.relative_to(D)) in paths}
missing_monsters = measure('monster files with literal constructor and definition',set(monsters),covered)
print('monster raw files, outside scanner, main definitions',len(monster_files),len(monster_files)-len(monsters),len(creature))
rank('monster missing definitions by source directory',(str(p.relative_to(D).parent) for p in missing_monsters))
mon_src = G/'world/world-monster.xml'
ms = rows('world/spawns/spawns-*.json','declaration')
dg = {tuple(int(r.attrib['center'+a]) for a in 'xyz') for r in xml(mon_src).findall('monster')}
measure('global spawn group centres',dg,{loc({'cell':r['centre']}) for r in ms})
dp = Counter(points(mon_src,'monster'))
mp = Counter((loc({'cell':r['centre']}),tail(p['creature']),*loc(p)) for r in ms for p in r['points'])
gap = dp-mp
print('global spawn member occurrences',dp.total(),(dp & mp).total(),gap.total(),
      'main totals',len(ms),mp.total())
held = load(C/'world/spawns/held-bosses.json')['records']
print('held boss points',len(held),'distinct',len({r['point']['creature'] for r in held}))
rank('boss holds by reason',(r['reason'] for r in held))
measure('held boss point key-and-cell still outside ambient spawns',
        {(*loc(r['point']),tail(r['point']['creature'])) for r in held},
        {(*loc(p),tail(p['creature'])) for r in ms for p in r['points']})
for f in sorted((D/'data-crystal/world').glob('*-monster.xml')):
    alt_points = Counter(points(f,'monster'))
    print('alternate spawn member occurrences',alt_points.total(),(alt_points & mp).total(),(alt_points-mp).total())
rank('spawn member gaps by creature',(p[1] for p in gap.elements()))
gap_species = Counter(p[1] for p in gap.elements())
for family,species in {
    'Rootthings':('rootthing_amber_shaper','rootthing_nutshell','rootthing_bug_tracker'),
    'Raubritters':('raubritter_skirmisher','raubritter_marksman','raubritter_chastener'),
    'Quara raider cohort':('quara_raider','quara_looter','quara_plunderer'),
}.items():
    print(family,sum(gap_species[n] for n in species))
hg = load('imports/crystalserver/summer-update/map-content-linking/spawns/held-groups.json')['records']
print('import held groups/points',len(hg),sum(r['point_count'] for r in hg))
rank('import group hold reasons',(reason for r in hg for reason in r['reasons']))
print('import held group examples',[(r['key'],r['point_count'],r['reasons']) for r in sorted(hg,key=lambda r:-r['point_count'])[:4]])

# 4. Explicit XML and literal Lua source facts for materializable items.
items = {int(tail(key(r))[1:]):r for r in rows('items/definitions/items-*.json','definition')
         if re.fullmatch(r'i\d+',tail(key(r)))}
fields = load('tools/content-schema/item-authoring/crystal-field-dispositions.json')['fields']
selected = {'skill_modifiers','use_transform','fluid','weapon','equipment'}
mapping = defaultdict(set)
for f in fields:
    if 'xml_item_attribute' in f['origins']:
        for dest in f['allowed_destinations']:
            parts = dest.strip('/').split('/')
            if dest == '/item/modifiers/skill_boost' or f['source_field'] in ('magicpoints','magicpointspercent'):
                mapping[f['source_field'].lower()].add('skill_modifiers')
            if len(parts)>1 and parts[1] in selected:
                mapping[f['source_field'].lower()].add(parts[1])
mapping['fluidsource'].add('fluid')
mapping['fluidcontainer'].add('fluid')
for attr in ('transformequipto','transformdeequipto','transformonuse','rotateTo'):
    mapping[attr.lower()].add('use_transform')
source = defaultdict(set)
name_ids = defaultdict(set)
materializable = {i for i,r in items.items() if r.get('materializable')}
for r in xml(D/'data/items/items.xml').findall('item'):
    a=r.attrib
    ids_r = [int(a['id'])] if 'id' in a else range(int(a['fromid']),1+int(a['toid']))
    attrs = [x.get('key','').lower() for x in r.findall('attribute') if x.get('value')]
    types = {x.get('value','').lower() for x in r.findall('attribute') if x.get('key','').lower()=='type'}
    groups = {group for attr in attrs for group in mapping[attr]}
    if 'fluidcontainer' in types:
        groups.add('fluid')
    for i in ids_r:
        if a.get('name'):
            name_ids[a['name'].casefold()].add(i)
        if i in materializable:
            for group in groups:
                source[(i,group)].update(attr for attr in attrs if group in mapping[attr])
                if group=='fluid' and 'fluidcontainer' in types:
                    source[(i,group)].add('type=fluidcontainer')
for p in sorted(D.glob('data*/scripts/**/*.lua')):
    s = lua(p)
    for i in re.findall(r':id\(\s*(\d+)\s*\)',s):
        i=int(i)
        if i not in materializable:
            continue
        if re.search(r'\.onUse\b',s) and re.search(r'\bitem:transform\(\s*\d+',s):
            source[i,'use_transform'].add('Lua item:transform')
        if re.search(r'\.on(?:DeEquip|Equip)\b|:slot\(["\']',s):
            source[i,'equipment'].add('Lua equip')
print('materializable tibia item IDs',len(materializable))
unknown = {(i,f) for i,f in source if items[i].get('semantics',{}).get(f,{}).get('state','UNKNOWN')=='UNKNOWN'}
known = {(i,f) for i,f in source if items[i].get('semantics',{}).get(f,{}).get('state')=='KNOWN'}
print('item groups SOURCE/KNOWN/UNKNOWN/OTHER',len(source),len(known),len(unknown),len(source)-len(known)-len(unknown))
weapon_leaf = {
    'attack':'attack','defense':'defense','extradef':'extra_defense','weapontype':'weapon_type',
    'ammotype':'ammunition','range':'range','hitchance':'hit_chance','maxhitchance':'max_hit_chance',
    **{f'element{x}':'elemental' for x in ('physical','fire','ice','energy','earth','holy','death')},
}
transforms = {'transformequipto':'EQUIP','transformdeequipto':'DEEQUIP','transformonuse':'USE',
              'Lua item:transform':'USE','rotateto':'ROTATE'}
leaves = defaultdict(set)
for (i,f),attrs in source.items():
    for a in attrs:
        if f=='fluid' and a=='fluidsource':
            leaves[i,'fluid.world_source'].add(a)
        elif f=='use_transform' and a in transforms:
            leaves[i,f+'.'+transforms[a]].add(a)
        elif f=='weapon' and a in weapon_leaf:
            leaves[i,f+'.'+weapon_leaf[a]].add(a)
        else:
            leaves[i,f].add(a)

def leaf_state(i,field):
    group,_,leaf = field.partition('.')
    row = items[i].get('semantics',{}).get(group,{})
    state = row.get('state','UNKNOWN')
    if state!='KNOWN' or not leaf:
        return state
    if group=='weapon':
        return row['value'].get(leaf,{}).get('state','UNKNOWN')
    if group=='use_transform':
        return next((r['target']['state'] for r in row['value']['targets'] if r['kind']==leaf),'UNKNOWN')
    return 'UNKNOWN'
unresolved = {k for k in leaves if leaf_state(*k)=='UNKNOWN'}
distinct = {i for i,_ in leaves}
gap_items = {i for i,_ in unresolved}
print('item donor-backed pairs SOURCE/KNOWN/UNKNOWN/OTHER',len(leaves),
      sum(leaf_state(*k)=='KNOWN' for k in leaves),len(unresolved),
      sum(leaf_state(*k) not in ('KNOWN','UNKNOWN') for k in leaves))
print('distinct item IDs source/without UNKNOWN/with UNKNOWN',len(distinct),len(distinct-gap_items),len(gap_items))
for f in sorted(selected):
    print('item '+f,*(sum(g.split('.')[0]==f for _,g in s) for s in
          (leaves,{k for k in leaves if leaf_state(*k)=='KNOWN'},unresolved)))
rank('items UNKNOWN donor source fields',(f for _,f in unresolved))
print('item UNKNOWN non-fluid examples',[(i,f,items[i].get('semantics',{}).get('presentation',{}).get('value',{}).get('name',{}).get('value'))
      for i,f in sorted(unresolved) if not f.startswith('fluid')][:16])
print('materializable donor-world-source IDs',','.join(str(i) for i,f in sorted(unresolved) if f=='fluid.world_source'))
Path('/tmp/cgap-item-keys.txt').write_text(''.join(key(items[i])+'\n' for i in sorted(materializable)))
# 5. Loot rows projected to monster and item identity (multiset).
loot = defaultdict(Counter)
for r in rows('loot/loot-*.json','definition'):
    for e in r['entries']:
        loot[tail(key(r))][e['item']['key']] += 1
main_loot_defs = rows('loot/loot-*.json','definition')
print('main loot tables/nonempty/entries',len(main_loot_defs),len(loot),sum(v.total() for v in loot.values()))
creature_to_loot = {tail(key(r['definition'])):tail(r['definition']['loot']['key'])
                    for r in creature if r['definition'].get('loot')}
def table_entries(text, start):
    # Ignore braces within strings. Track outer and child rows independently.
    stack = []
    for m in re.finditer(r'"(?:\\.|[^"\\])*"|\'(?:\\.|[^\'\\])*\'|[{}]',text[start:]):
        token = m[0]
        if token=='{':
            if stack:
                stack[-1][1].append(text[stack[-1][0]:start+m.start()])
            stack.append([start+m.end(),[]])
        elif token=='}' and stack:
            cursor,chunks = stack.pop()
            if not stack:
                break
            chunks.append(text[cursor:start+m.start()])
            if stack:
                stack[-1][0]=start+m.end()
            yield ''.join(chunks)

for base in (G,D/'data-crystal'):
    loot_gap = Counter()
    counts = Counter()
    for p,n in monsters.items():
        if base not in p.parents:
            continue
        text=lua(p)
        match=re.search(r'\b\w+\.loot\s*=\s*(\{)',text)
        if not match:
            if re.search(r'\.loot\s*=',text):
                counts['nonliteral_tables']+=1
            continue
        canon=paths.get(str(p.relative_to(D)),n)
        pool=loot[creature_to_loot.get(canon,canon)].copy()
        for entry in table_entries(text, match.start(1)):
            ident=re.search(r'\bid\s*=\s*(\d+)',entry)
            name=re.search(r'\bname\s*=\s*(["\'])(.*?)\1',entry)
            if not (ident or name or re.search(r'\bchance\s*=',entry)):
                continue
            counts['source_rows']+=1
            ids_loot={int(ident[1])} if ident else name_ids[name[2].casefold()] if name else set()
            if len(ids_loot)!=1:
                counts['skipped_unresolved_item']+=1
                continue
            item=f'oteryn:item.tibia.i{next(iter(ids_loot))}'
            if pool[item]:
                pool[item]-=1
                counts['present']+=1
            else:
                counts['absent']+=1
                loot_gap[n]+=1
    print(base.name,'loot source / present / absent-or-skipped',counts['source_rows'],counts['present'],
          counts['absent']+counts['skipped_unresolved_item'],dict(counts))
    rank(base.name+' loot absent rows by monster',loot_gap.elements())

# 6. Grant site census and an output-identity join against identifiable main granters.
cos = load(C/'cosmetics/outfits/authoring.json')['document']
mount_catalogue = rows('cosmetics/mounts/mounts-*.json','declaration')
mount_xml = {int(r.get('id')):slug(r.get('name')) for r in xml(D/'data/XML/mounts.xml').findall('mount')}
mount_look = {int(r.get('clientid')) for r in xml(D/'data/XML/mounts.xml').findall('mount')}
mount_keys = {tail(key(r)) for r in mount_catalogue}
mount_runtime = {tail(r['key']) for r in cos['mounts']}
measure('mount definitions to runtime catalogue key overlap',mount_keys,mount_runtime)
print('main mount definitions / runtime rows',len(mount_catalogue),len(cos['mounts']))
measure('Crystal XML mount appearance IDs to main runtime',mount_look,{r['look']['id'] for r in cos['mounts']})
ach_xml = {int(i):slug(n) for i,n in re.findall(r'^\s*\[(\d+)\]\s*=\s*\{\s*name\s*=\s*"([^"]+)"',
          lua(D/'data/scripts/lib/register_achievements.lua'),re.M)}
ach_defs = rows('achievements/achievements-*.json')
measure('achievement registry names',set(ach_xml.values()),{slug(r['name']) for r in ach_defs})
outfit_xml = {int(r.get('looktype')):slug(r.get('name')) for r in xml(D/'data/XML/outfits.xml').findall('outfit')}
outfit_ids = {r[sex]['id'] for r in cos['outfits'] for sex in ('male','female') if r.get(sex)}
measure('outfit XML appearance IDs to main runtime',set(outfit_xml),outfit_ids)
taming = load(C/'cosmetics/taming-native.json')
granters = {'achievement':set(), 'outfit':set(), 'mount':set()}
granters['achievement'].update(r['achievement_key'].rsplit('/',1)[-1] for r in taming if r.get('achievement_key'))
granters['mount'].update(tail(r['mount_key']) for r in taming)
grant_mapping = {m['grant_key']:m['target'] for m in cos['grant_mappings']}
granters['achievement'].update(slug(a.rsplit('/',1)[-1]) for a,*_ in runtime['achievement'])
runtime_mount_look = {r['key']:r['look']['id'] for r in cos['mounts']}
donor_mount_look = {int(r.get('clientid')):slug(r.get('name'))
                    for r in xml(D/'data/XML/mounts.xml').findall('mount')}
granters['mount'].update(donor_mount_look[runtime_mount_look[r['mount_key']]] for r in taming
                        if r['mount_key'] in runtime_mount_look and runtime_mount_look[r['mount_key']] in donor_mount_look)
granters['achievement'].add('natural_born_cowboy')  # mount_taming.rs successful taming
for entry in runtime['outfit']:
    identity = entry[0]
    identity = grant_mapping.get(identity,{}).get('outfit_key',identity)
    granters['outfit'].add(tail(identity.split('/',1)[0]))
granters['outfit'].update(outfit_xml[int(i)] for i,*_ in runtime['outfit_id'] if int(i) in outfit_xml)
taming_pairs = {(int(tail(r['item_key'])[1:]),donor_mount_look[runtime_mount_look[r['mount_key']]])
                for r in taming if r['mount_key'] in runtime_mount_look
                and runtime_mount_look[r['mount_key']] in donor_mount_look}
taming_lua = lua(G/'scripts/actions/mounts/mounts.lua')
donor_taming = {(int(i),mount_xml[int(m)]) for i,m in re.findall(
    r'^\t\[(\d+)\]\s*=\s*\{\s*NAME = "(?:[^"\n]*)",\s*ID = (\d+)',taming_lua,re.M)
    if int(m) in mount_xml}
miss_taming = measure('global taming config item/mount grant pairs',donor_taming,taming_pairs)
print('unmatched taming pairs',sorted(miss_taming))
direct_items = D/'data/scripts/actions/items'
configured_item_mounts = set()
for p in sorted(direct_items.glob('usable*items.lua')):
    t = lua(p)
    if ':addMount(' in t:
        configured_item_mounts.update((p.stem,mount_xml[int(m)]) for m in re.findall(
            r'\bmountId\s*=\s*(\d+)',t) if int(m) in mount_xml)
print('explicit donor direct-use mount pack targets',sorted(configured_item_mounts))
print('missing Crystal mount appearances',sorted((i,n) for i,n in donor_mount_look.items()
      if i not in set(runtime_mount_look.values())))
item_packet = load('tools/content-schema/native-gameplay/item-cosmetic-actions-native.json')
assert item_packet['crystal_commit']==PIN
item_rules = []
item_grants = {r['key']:r for r in item_packet['grants']}
acquisitions = {(r['item_key'],r['donor']) for r in cos['item_acquisitions']}
direct_item_witness = set()
for rule in item_packet['rules']:
    i = rule['item_id']
    donor = rule['donor']
    if i not in materializable:
        continue
    if (rule['mode'] in ('random_outfit','random_mount') or donor.startswith('data-crystal/scripts/special/')) and (
            f'oteryn:item.tibia.i{i}',donor) not in acquisitions:
        continue
    item_rules.append(rule)
    for choice in rule['choices']:
        granters['achievement'].update(slug(a.rsplit('/',1)[-1]) for a in choice.get('achievements',[]))
        for g in choice.get('grants',[]):
            if g not in grant_mapping:
                continue
            row = item_grants[g]
            if row['kind']=='mount' and row['look_type'] in donor_mount_look:
                n = donor_mount_look[row['look_type']]
                granters['mount'].add(n)
                direct_item_witness.add((Path(donor.split(':',1)[0]).stem,n))
            else:
                granters['outfit'].update(outfit_xml[i] for i in row.get('look_types',[]) if i in outfit_xml)
print('item cosmetic rules authored / source-policy and materializable screen',len(item_packet['rules']),len(item_rules))
direct_gap = measure('configured direct-use mount script/target pairs',configured_item_mounts,direct_item_witness)
print('direct-use mount pairs without selected policy/identity witness',sorted(direct_gap))
sites = defaultdict(list)
patterns = {'achievement':r':(addAchievement(?:Progress)?)\s*\(\s*',
            'outfit':r':(addOutfit(?:Addon)?)\s*\(\s*',
            'mount':r':(addMount)\s*\(\s*'}
for p in sorted(D.glob('data*/**/*.lua')):
    if p.name == 'register_achievements.lua':
        continue
    text = lua(p)
    for family,pattern in patterns.items():
        for m in re.finditer(pattern,text):
            arg = re.match(r'(?:"([^"]+)"|\'(.*?)\'|(\d+)\b)',text[m.end():])
            n = None
            if arg:
                if arg[3]:
                    n = {'achievement':ach_xml,'outfit':outfit_xml,'mount':mount_xml}[family].get(int(arg[3]))
                elif family=='achievement':
                    n = slug(arg[1] or arg[2])
            sites[family].append((p,n))
for family,a in sites.items():
    counts = Counter('output_witness' if n in granters[family] else
                     'resolved_without_witness' if n else 'unresolved' for _,n in a)
    print(family,'grant call sites donor/output-witness/gap-or-unresolved',len(a),counts['output_witness'],
          len(a)-counts['output_witness'],dict(counts),'main named targets',len(granters[family]))
    targets = {n for _,n in a if n}
    absent = measure(family+' literal grant targets',targets,granters[family])
    rank(family+' unmatched resolved calls by module',
         (str(p.relative_to(D).parent) for p,n in a if n in absent))
    print(family+' literal target examples',sorted(absent)[:8])

# 7. Raid XML wave paths, Lua registrations and seasonal events.
cap = load('imports/crystalserver/raids-201189f.json')
season = load('imports/crystalserver/seasonal-raids-201189f.json')
for p in (cap,season):
    assert p['revision']==PIN
    if isinstance(p['files'],list):
        from hashlib import sha256
        print('capture exact matching raw files',sum(
            (D/r['path']).is_file() and sha256((D/r['path']).read_bytes()).hexdigest()==r.get('sha256')
            for r in p['files']),len(p['files']))
print('pinned raid/seasonal capture files',len(cap['files']),len(season['files']))
raids = load(C/'encounters/raids/catalogue.json')['records']
all_raid_path = {s['path'] for r in raids for s in r['sources'] if s['revision']==PIN}
active_raid_path = {s['path'] for r in raids if r['active'] for s in r['sources'] if s['revision']==PIN}
raid_index = G/'raids/raids.xml'
registered = {(G/'raids'/r.get('file')).relative_to(D).as_posix()
              for r in xml(raid_index).findall('raid') if r.get('file')}
wave_files = {p.relative_to(D).as_posix() for p in (G/'raids').rglob('*.xml') if p!=raid_index}
measure('global indexed raid wave files',registered,all_raid_path)
measure('global indexed raid wave files with active catalogue',registered,active_raid_path)
gap_raid = measure('global XML wave files with active catalogue',wave_files,active_raid_path)
rank('inactive or uncatalogued global XML waves by area',(Path(p).parent.name for p in gap_raid))
print('unindexed global wave files',sorted(wave_files-registered))
print('indexed paths without file',sorted(registered-wave_files))
lua_raid = {p.relative_to(D).as_posix() for p in (G/'scripts/raids').rglob('*.lua')
            if re.search(r':register\s*\(',lua(p))}
measure('global registered raid-script files with active catalogue',lua_raid,active_raid_path)
print('raid Lua without active path',sorted(lua_raid-active_raid_path))
schedule = lua(G/'scripts/globalevents/others/raids_schedule.lua')
names = {slug(n) for n in re.findall(r'raidName\s*=\s*"([^"]+)"',schedule)}
schedule_paths = {s['path'] for r in raids if r['active'] for s in nodes(r['schedule'])
                  if 'path' in s and s.get('revision')==PIN}
measure('scheduled named raids with donor timer-source witness',names,
        {slug(r['name']) for r in raids if r['active'] and any(
         s.get('path','').endswith('/raids_schedule.lua') for s in nodes(r['schedule']))})
print('scheduled names',sorted(names),'active timer-source path count',len(
    {p for p in schedule_paths if p.endswith('/raids_schedule.lua')}))
alt = D/'data-crystal/raids'
alt_index = {str((alt/r.get('file')).relative_to(D)) for r in xml(alt/'raids.xml').findall('raid')}
measure('alternate data-crystal indexed XML raid wave files',alt_index,active_raid_path)
donor_events = {p.stem.removesuffix('_respawn_event') for p in (G/'scripts/world_changes').glob('*.lua')}
event_gap = measure('global world-change script names with native owner',donor_events,runtime_world_changes|other_events)
print('world-change gaps',sorted(event_gap))
print('main raid catalogue records/active',len(raids),sum(r['active'] for r in raids))
rank('inactive catalogue reasons',(s for r in raids if not r['active'] for s in r['inactive_reasons']))
print('world changes main runtime', [r['key'] for r in load(C/'encounters/world-changes/runtime.json')['changes']])
print('main seasonal scheduler', [r['event'] for r in load(C/'encounters/event-schedule/schedule.json')['seasonal']])
```

</details>
