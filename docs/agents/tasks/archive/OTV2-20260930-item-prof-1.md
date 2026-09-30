# OTV2-20260930-item-prof-1

```yaml
task_id: OTV2-20260930-item-prof-1
title: ITEM-PROF-1 - weapon proficiency profiles for Items
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/item-prof-1
issue: 162
lane_id: content-world
pr: 1322
base_sha: 82bae6fd
head_sha: null   # a commit cannot hold its own SHA; exact head is in the FREEZE_SHA packet
final_head_sha: null
final_head_frozen_at: null
owner: "ITEM-PROF-1 impl worker (claude-code-session-01EfiFA9LMuUuzoNkizLfR2R)"
created_at: 2026-09-30
updated_at: 2026-09-30
execution_policy: continuous_progress
owned_paths:
  - tools/content-schema/item-authoring/**
  - docs/agents/tasks/archive/OTV2-20260930-item-prof-1.md
public_contracts: []
depends_on: ["ITEM-ID-1b (#1305, merged)", "staging #1283"]
blocks: ["PROF-CONTENT-1", "PROF-2", "PROFICIENCY-1"]
external_repositories: []
jira: null   # sync pending (coordinator batch)
```

## Outcome

Request: #162 5906079988 (owner). Two of its three items already existed on `main` (staging #1283):
`tools/content-census/stage_proficiencies.py` imports the digest-pinned client file
`proficiencies-7fea90ec...json` (443 profiles) and extracts flag 61 `proficiency_id` from the 15.30 appearances
(666 objects) into `imports/cipsoft-staticdata/{proficiencies,weapon-proficiency-bindings}/`. The request text
("read by no tool", "not extracted today") was stale. The item schema also already had `proficiency`
(`profile_binding`/`levels`/`shaping`/`augments`, admitted-profile only). This task adds the missing join:

- `item_weapon_proficiency.py` verifies both manifests and all shard digests against the pins, then writes
  `samples/item-weapon-proficiency-15-30-7fea90ec.json` (`OTERYN_ITEM_WEAPON_PROFICIENCY_STAGING/v1`): 443 profiles
  (raw levels/perks unchanged, `threshold_class`, `top_level`, `mastery_level` = top + 2), 666 bindings keyed
  `oteryn:item.tibia.i<client id>`, the 3 threshold tables (TibiaWiki revid 1192598, owner-accepted #162
  5905899852), and a raw inventory of the perk enum values. `--check` diffs a regeneration.
- Schema: optional `proficiency.client_binding {client_proficiency_id >= 1, threshold_class standard|knight|crossbow}`
  (a valid `proficiency` alone); `validate_item.py` ignores it when comparing an admitted inline profile.
  4 new `verify_formal_schema.py` cases (1 positive, 3 negative); 246/246 pass.
- Threshold class is keyed per binding (owner decisions D197-D200, #162 2026-09-30), not per profile; see
  "Owner decisions" below. Bindings: 26 crossbow, 43 knight, 173 standard, 424 unknown.

## Cross-check (evidence only; TibiaPal, owner-verified #162 5905825574 / 5905851791)

`weapon-proficiencies.json?v=20260816-1` (sha256 34702ada...df7d, fetched 2026-09-30, digest matches): 435 weapons.
Every weapon's client id is also in our 666 bindings. Same `proficiencyId` for 430; 5 differ: client ids 49520,
49858, 49859, 49860, 27455 are 54/64/74/86/215 here and 497-501 there (ids absent from the 15.30 file, max 496:
TibiaPal is newer than 15.30). Raw perk fields equal for 425 of the 430 shared profiles (numeric equality; the other 5
are the 5 above's profiles 330, 64, 74, 86, 43). Name-derived threshold class equals TibiaPal `weaponType`
class (Crossbow / Sword-Axe-Club / rest) for all 430.

## Not done / blocked

- `item_authoring.proficiency` in `content/world/definitions/declarations.json` is not populated: it is outside the
  owned paths, and the Rust `ProjectV2WeaponProficiencyProfile` (`deny_unknown_fields`) has no field for the client id
  or threshold class, and its `levels` need `ProjectV2AugmentBinding` semantics that need the confirmed enum mapping
  below. Follow-up: Rust field for `client_binding` (or the mapping) plus the world-project generator; then populate
  from `bindings` in the staging artifact. 23 of the 666 bindings (client ids 53207-53228, 53855) have no Item
  definition yet (`item_defined: false`).

## Owner decisions (#162, 2026-09-30) and how they are applied

- D197: fist and every other melee weapon not covered by D200 is `standard`.
- D198: Arbalest, Chain Bolter, The Ironworker, The Devileye, Thorn Spitter are crossbows. Control-plane rule:
  the class is per binding; bolt ammunition (`ammotype=bolt`, Crystal ff7ede5 `items.xml` via the committed
  `docs/agents/evidence/OTV2-20260919-content-world-cw2-b1-item-identity-catalog.json`, sha256 pinned; matches
  TibiaWiki `secondarytype=Crossbows`) = crossbow. That covers crossbow 3349, crossbow of destruction, cobra, naga
  and inferniarch arbalests in shared bow/crossbow profiles. `arrow` = standard. A bow/crossbow-family binding
  with no ammotype (the replica bows and the 18 replica crossbows 26004-26067, and moonsilver 53225-53228 newer than the
  catalog) is `unknown`, not guessed.
- D200: Knight only for knight-restricted sword/axe/club weapons (Canary `getExperienceArray`: weapon vocation
  includes knight). No committed per-item vocation source covers the weapon list; the only in-repo evidence is the
  TibiaWiki wave-1 snapshot `vocrequired` (`OTV2-20260925-item-enrichment-wave1-source-snapshot.json`, sha256
  pinned), joined to bindings by an unambiguous item name. `Knights` = knight; another explicit value = standard;
  no value = `unknown`.
- D199: perk enum mapping accepted (TibiaWiki Weapon_Proficiency_Tables revid 1206177, 3671/3671 perks
  cross-matched; Canary `src/enums/weapon_proficiency.hpp`). Emitted as `perk_mapping` in the sample; raw values
  kept in profiles and `perk_raw_enums`. The Type -1 row is dropped (absent from the 15.30 file).

| Raw field | Value | Meaning (D199) | Unit |
|---|---|---|---|
| Type | 0 / 1 | attack / defence | flat |
| Type | 2 | weapon shield defence modifier | flat |
| Type | 3 | combat skill (SkillId) | flat |
| Type | 4 | specialised magic level (DamageType) | flat |
| Type | 5 | spell augmentation (AugmentType, SpellId) | fraction; AugmentType 6 = negative seconds |
| Type | 6 / 7 | damage vs bestiary family (BestiaryId) / vs bosses and Sinister Embraced | fraction |
| Type | 8 / 9 / 10 / 11 | critical hit chance: general / elemental (ElementId) / offensive runes / auto-attack | fraction |
| Type | 12 / 13 / 14 / 15 | critical extra damage: general / elemental / runes / auto-attack | fraction |
| Type | 16 / 17 | mana leech / life leech | fraction |
| Type | 18 / 19 | mana on hit / hit points on hit | flat |
| Type | 20 / 21 | mana on kill / hit points on kill | flat |
| Type | 22 | damage at range (distance in tiles) | flat, tiles |
| Type | 23 | ranged hit chance | fraction |
| Type | 24 | attack range | flat |
| Type | 25 / 26 / 27 | skill-scaled extra damage for auto-attacks / spells / healing (SkillId) | fraction |
| Type | 28 / 29 | Alpha Strike (+% damage vs targets above 95% HP) / Omega Strike (below 30% HP) | fraction |
| Type | 30 | armor penetration (1.0 = +100%) | fraction |
| Type | 31 | elemental pierce (ElementId) | fraction |
| Type | 32 | homing missile (ElementId, MissileId, Probability, Multiplier) | fractions |
| SkillId | 1, 6, 7, 8, 9, 10, 11, 13 | Magic Level, Shielding, Distance, Sword, Club, Axe, Fist Fighting, Fishing | |
| AugmentType | 2, 3, 6, 14, 15, 16, 17 | spell base damage, healing, cooldown, life leech, mana leech, critical extra damage, critical hit chance | see Type 5 |
| ElementId / DamageType | 1, 8, 16, 32, 64, 128, 256, 1048576 | physical, fire, earth, energy, ice, holy, death, healing (bitmask) | |

## Open question for the owner

Knight coverage: only 43 sword/axe/club bindings have vocation evidence in repo, so 424 bindings are `unknown`.
Options: (a) accept `unknown` until the Crystal/Canary `items.xml` vocation attribute is staged as evidence,
(b) treat a wiki-matched weapon without `vocrequired` as unrestricted (`standard`). Recommendation: (a), then stage
the `vocation` attribute from the pinned items.xml.

## Validation

- `verify_formal_schema.py` 246/246; `test_item_weapon_proficiency.py` 10/10; `item_weapon_proficiency.py --check` up to
  date; `build_formal_schema.py` regenerated; `ruff format --check` clean; governance validator pass; repair generation 1 applies D197-D200.
- Review: none requested by this worker (content tooling, no runtime or protocol change).
