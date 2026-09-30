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
pr: null   # recorded in the FREEZE_SHA packet on #162
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
- Threshold class is derived from the profile name: word `Crossbow` = crossbow; `Sword|Axe|Club` = knight;
  else standard. 260 knight, 18 crossbow, 165 standard profiles (bindings 445/15/206).

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

## Owner questions

1. Threshold class of 5 ranged weapons TibiaPal labels `Other`: Distance 2H Arbalest, Chain Bolter, The Ironworker,
   The Devileye, Thorn Spitter. By name they are `standard`. Options: (a) keep standard, (b) crossbow (they are
   crossbows in game; needs an in-game or wiki confirmation). Recommendation: (b) once confirmed.
2. Fist weapons (28 in TibiaPal) are `standard` (the wiki table lists only Knight and Crossbows). Confirm.

## PROPOSAL: perk enum mapping (marked for owner confirmation; nothing applies it)

Evidence: TibiaPal per-perk labels and descriptions for the same raw fields (435 weapons), our client file. The
client file does not define these values.

| Raw field | Value | Proposed meaning (evidence: TibiaPal name/description) |
|---|---|---|
| Type | 0 / 1 | +attack / +defence (flat) |
| Type | 2 | weapon shield defence modifier |
| Type | 3 | combat skill (SkillId) |
| Type | 4 | specialised magic level (DamageType) |
| Type | 5 | spell augmentation (AugmentType, SpellId) |
| Type | 6 / 7 | % damage against bestiary family (BestiaryId) / against bosses and Sinister Embraced |
| Type | 8 / 9 / 10 | critical hit chance: general / elemental (ElementId) / offensive runes |
| Type | 11 | auto-attack critical hit chance |
| Type | 12 / 13 / 14 / 15 | critical extra damage: general / elemental / runes / auto-attack |
| Type | 16 / 17 | mana leech / life leech (TibiaPal labels 17 "Armor penetration" but its description reads life leech; trust description) |
| Type | 18 / 19 | mana on hit / hit points on hit (TibiaPal names swapped vs descriptions; trust descriptions; confirm in game) |
| Type | 20 / 21 | mana on kill / hit points on kill (same swap; confirm) |
| Type | 22 / 23 / 24 | damage at Range / ranged hit chance / attack range |
| Type | 25 / 26 / 27 | skill-scaled extra damage for auto-attacks / spells / healing (SkillId) |
| Type | 28 / 29 / 30 | conditional: +% damage above 95% target HP / below 30% target HP / armor penetration (TibiaPal names these "Highest combat skill scaling", descriptions differ; trust descriptions) |
| Type | 31 / 32 | elemental pierce (ElementId) / homing missile (ElementId, MissileId, Probability, Multiplier) |
| Type | -1 | +50% life leech for Annihilation (2 perks) |
| SkillId | 1, 6, 7, 8, 9, 10, 11, 13 | Magic Level, Shielding, Distance, Sword, Club, Axe, Fist Fighting; 13 unresolved (appears only with rods, wands, bows) |
| AugmentType | 2, 3, 6, 14, 15, 16, 17 | spell base damage %, healing %, cooldown, life leech, mana leech, critical extra damage, critical hit chance (all "for <SpellId>") |
| ElementId / DamageType | 1, 8, 16, 32, 64, 128, 256, 1048576 | combat-type bitmask: physical, fire, earth, energy, ice, holy, death, healing |

Gating: Types 18-21, 28-30 and SkillId 13 stay unconfirmed until in-game or official evidence; `Value` units
(fraction vs flat) per Type are not yet proven.

## Validation

- `verify_formal_schema.py` 246/246; `test_item_weapon_proficiency.py` 5/5; `item_weapon_proficiency.py --check` up to
  date; `build_formal_schema.py` regenerated; `ruff format --check` clean for the new files.
- Review: none requested by this worker (content tooling, no runtime or protocol change).
