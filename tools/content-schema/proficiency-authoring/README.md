# Weapon Proficiency authoring schema candidate v1

The static `Proficiency` definition catalogue from PROFICIENCY-0 §4.1
([`OTERYN_GAME_PROFICIENCY0_WEAPON_PROFICIENCY_DECISION_2026-09-29.md`](../../../docs/architecture/reviews/OTERYN_GAME_PROFICIENCY0_WEAPON_PROFICIENCY_DECISION_2026-09-29.md)).
This is only the content layer:

- Accrual, level-up, perk selection and clearing, shaping and Mastery rules belong to
  `rulesets/progression/weapon-proficiency/`.
- A Character's progress and selected perks are durable Character state (PROFICIENCY-0 §4.2).
- Which weapon uses which definition is the Item `proficiency.profile_binding` (item-authoring).

None of these is modelled here. Nothing is written to `content/`, `rulesets/` or runtime code; populating
`content/proficiencies/` and switching Items to `profile_binding` stay with PROF-CONTENT-1.

| File | Purpose |
|---|---|
| `proficiency.schema.json` | One catalogue (`OTERYN_PROFICIENCY_AUTHORING_CATALOGUE/v1`), JSON Schema 2020-12, closed shapes. A definition has `identity {family: Proficiency, key, revision}`, `name`, `source` (client id, raw `Version`, record digest) and 1-7 `levels`, each with 1-3 `perks`. A perk is a `oneOf` over 33 closed kinds, one per source `Type` code. |
| `proficiency_authoring.py` | `build` promotes the staged 15.30 definitions with the D199 code map and rejects any unmapped code or key set; `build --check` diffs a rebuild against the committed candidate. `validate` runs the schema and the semantic rules. |
| `test_proficiency_authoring.py` | No-network tests: perk encoding and its rejections, the committed build, one negative case per rule. |
| `samples/proficiencies-candidate.json` | The candidate catalogue: 443 definitions, 3,671 perks, all 33 kinds. One definition per line. |

## Sources

- **CipSoft 15.30 client file, primary.** `imports/cipsoft-staticdata/proficiencies/` (443 definitions),
  digest-checked against its manifest, which pins the client file SHA-256 `7fea90ec…`. PROFICIENCY-0 §4.1
  admits a definition from this source alone.
- **Perk code map, D199 (owner-accepted).** The meaning of `Type`, `SkillId`, `ElementId`/`DamageType` and
  `AugmentType`, from `item-authoring/samples/item-weapon-proficiency-15-30-7fea90ec.json` (`perk_mapping`;
  Canary `weapon_proficiency.hpp`, TibiaWiki `Weapon_Proficiency_Tables` 3671/3671). TibiaPal's planner
  (`tibiapal.com/weapon-proficiency`) uses the same 33 names and the same element codes (8 fire, 32 energy).
- **Thresholds.** Copied from the same ITEM-PROF-1 sample (TibiaWiki `Weapon_Proficiency` revid 1192598,
  owner decision #162 5905899852): standard, knight and crossbow tables of 9 cumulative values.

## Shape decisions

- **Key:** `oteryn:proficiency.tibia.p<ProficiencyId>`, revision `definition-r1` (PROFICIENCY-0 §4.1).
- **Perks** keep the source order within a level, so a selection is the array index, as in the
  `selections SMALLINT[]` of PROFICIENCY-0 §4.2 and TibiaPal's share token (`p`: one index per level,
  `-1` unassigned).
- **Values** stay as given. The unit is fixed by the kind (D199): flat for kinds from Types 0-4, 18-22 and
  24; a fraction (×100 = %) otherwise; a spell `cooldown` augment is negative seconds. `homing_missile`
  carries `probability` and `multiplier` and no `value`.
- **Numeric client ids** that name other families (`spell_client_id`, `bestiary_class_id`,
  `missile_client_id`) stay provenance. Their crosswalks to Spell and Bestiary keys are not part of this
  schema.
- **Thresholds** sit in `threshold_tables` at catalogue level, not in each definition: the class
  (standard, knight, crossbow) is chosen per weapon binding (D197, D198, D200), so one definition can
  have more than one. A tree of n levels uses entries 1..n; Mastery is entry n + `mastery_offset` (2).
  This refines PROFICIENCY-0 §4.1 ("each level's threshold").
- **Not modelled:** shaping (reshape, rank 0-10, refine, Lunar Ascension Orb) and its costs, deferred
  to PROFICIENCY-1; the point table; shared progress across weapons.

## Rules (`validate`)

- The schema passes.
- Keys and ids are unique, and each key follows its `proficiency_id`.
- Levels are 1..n in order.
- Every `value` is positive, except a `cooldown` augment, which is negative.
- Each threshold table strictly increases.
- Against staging: the same id set, the same record digests, and every perk decodes back to its
  staged source perk exactly.

```sh
pip install -r requirements.txt
python proficiency_authoring.py build --check
python proficiency_authoring.py validate samples/proficiencies-candidate.json
python test_proficiency_authoring.py
```
