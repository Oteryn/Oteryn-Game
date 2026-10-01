# Weapon Proficiency authoring schema candidate v1

The static `Proficiency` definition catalogue from PROFICIENCY-0 §4.1
([`OTERYN_GAME_PROFICIENCY0_WEAPON_PROFICIENCY_DECISION_2026-09-29.md`](../../../docs/architecture/reviews/OTERYN_GAME_PROFICIENCY0_WEAPON_PROFICIENCY_DECISION_2026-09-29.md)).
This is only the content layer:

- Accrual, level-up, perk selection and clearing, shaping and Mastery rules belong to
  `rulesets/progression/weapon-proficiency/`.
- A Character's progress and selected perks are durable Character state (PROFICIENCY-0 §4.2).
- Which weapon uses which definition is the Item `proficiency.profile_binding` (item-authoring).

None of these is modelled here. `build`, `validate` and the tests write only under this directory;
`content` writes `content/proficiencies/` and its registration (PROF-CONTENT-1a). Switching Items to
`profile_binding` and the Rust family stay with PROF-CONTENT-1b.

| File | Purpose |
|---|---|
| `proficiency.schema.json` | One catalogue (`OTERYN_PROFICIENCY_AUTHORING_CATALOGUE/v1`), JSON Schema 2020-12, closed shapes. A definition has `identity {family: Proficiency, key, revision}`, `name`, `source` (client id, raw `Version`, record digest) and 1-7 `levels`, each with 1-3 `perks`. A perk is a `oneOf` over 33 closed kinds, one per source `Type` code. |
| `proficiency_authoring.py` | `build` promotes the staged 15.30 definitions with the D199 code map and rejects any unmapped code or key set; `build --check` diffs a rebuild against the committed candidate. `validate` runs the schema and the semantic rules. |
| `proficiency_authoring.py content` | Writes (`--check` verifies) `content/proficiencies/index.json` (`OTERYN_FAMILY_INDEX/v1`) and three shards of 150 (`OTERYN_PROFICIENCY_SHARD/v1`, one `definition` per record, identity `{key, revision}`), and registers `Proficiency` in `content/project.json` (`migrated_families`), `content/manifest.json` (`families`, `managed_files`) and `content/content.lock.json` (`family_counts`). Threshold tables stay in the catalogue: they are progression rules and go to `rulesets/progression/weapon-proficiency/` with PROF-2. It also writes `content/proficiencies/bindings.json` (PROF-CONTENT-1c, below). |
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
- **Threshold class of a weapon (owner, 2026-09-30):** one rule, applied when content is built, not
  hand-written per Item: bolt ammunition gives crossbow; a sword, axe or club that knights may use
  gives knight; anything else gives standard (D197, D198, D200, as Canary does). The result is written
  to the weapon's row in `content/proficiencies/bindings.json` (`threshold_class`). CI checks that the
  written class matches the rule. Missing Item facts give `unknown`, and an `unknown` weapon gets no proficiency. An
  exception needs an explicit override with a cited source. PROF-CONTENT-1 implements this.
- **Not modelled:** shaping (reshape, rank 0-10, refine, Lunar Ascension Orb) and its costs, deferred
  to PROFICIENCY-1; the point table; shared progress across weapons.

## Canary and Crystal comparison (2026-09-30, `OtsHypothesisOnly`)

Canary `04b83b5` (`src/creatures/players/components/weapon_proficiency.*`) and Crystal `96d13ef`
(`src/creatures/players/proficiencies/*`, `player.cpp`) were read for cross-checking only:

- **Definitions.** Both load the CipSoft JSON as it is, with the same file (SHA-256 `1a915dff…`,
  420 definitions). The 15.30 client file has 443: 23 newer ids (474+), one changed definition (43),
  and the Type 32 homing missile (22 perks), which neither engine has. Canary quietly cuts levels and
  perks above its config caps; this schema rejects a definition that does not fit instead.
- **Weapon binding.** Both take the proficiency id from the appearances flag. Canary also lets
  `items.xml` override it. This matches the Item `profile_binding`.
- **Thresholds.** Canary picks the table per weapon: bolt ammunition gets crossbow; a sword, axe or
  club whose vocations include knight gets knight; everything else gets standard. Mastery is
  `maxLevel + 2`, capped at 9. It uses the same three tables. This matches the catalogue-level
  tables and D197/D198/D200.
- **Character state.** Both keep one track per weapon item id. Crystal stores experience and
  `(level, perk position)` pairs; Canary stores copies of the selected perks in KV. Storing the index
  (PROFICIENCY-0 §4.2, and the source order kept here) matches Crystal. Canary's perk copies would go
  stale when a definition changes.
- **Point table** (ruleset, not content). TibiaWiki `Weapon_Proficiency` revid 1192598 (the revision
  whose thresholds the owner accepted) gives, by Bestiary difficulty and influence stacks 0-5 / fiend:
  harmless 1 (fiend 2), trivial 30, easy 70, medium 100, hard 165, challenging 240, each +10% per stack
  (rounded down) and ×2.5 when fiendish. Bosses: bane 500, archfoe 5,000, nemesis 15,000, with no
  influence bonus. Canary and Crystal have the same base values. Canary has no influence bonus. Crystal
  gives a flat ×1.1 for any number of influence stacks and 1,500 for soulpit bosses, which the wiki does
  not list. The manual's 1-175 per kill and 1,000 per boss predate that revision.
  **Owner decision (2026-09-30):** the wiki table is admitted as Reference evidence for the point table,
  which lifts that PROFICIENCY-0 §4.5 parity gate; the Canary and Crystal values are not used. Soulpit
  bosses give nothing until evidenced. Multi-player credit follows the wiki (every damage contributor, as
  Bestiary). The ruleset lane (PROF-2) encodes it in `rulesets/progression/weapon-proficiency/`.
- **Shaping** is in neither engine.

## Rules (`validate`)

- The schema passes.
- Keys and ids are unique, and each key follows its `proficiency_id`.
- Levels are 1..n in order.
- Every `value` is positive, except a `cooldown` augment, which is negative.
- Each threshold table strictly increases.
- Against staging: the same id set, the same record digests, and every perk decodes back to its
  staged source perk exactly.

## Weapon bindings (PROF-CONTENT-1c)

`content/proficiencies/bindings.json` (`OTERYN_PROFICIENCY_ITEM_BINDINGS/v1`) gives each weapon its
Proficiency definition and threshold class: `{item: ItemRef, profile_binding: ProficiencyRef,
threshold_class}`, sorted by Item key. It is the content form of the Item `proficiency` binding of
WorldProject v2 (PROF-CONTENT-1b). `content/world` stays unmodified.

- **Source:** the ITEM-PROF-1 bindings (`item-authoring/samples/item-weapon-proficiency-15-30-7fea90ec.json`,
  pinned by SHA-256), from the 15.30 client (flags field 61, inferred).
- **Threshold class:** `item_weapon_proficiency.py` applies the rule (D197, D198, D200) to each binding
  when content is built and writes it to `content/proficiencies/bindings.json`; the Item Authoring
  Schema CI drift-checks it (`--check`).
- **Result:** 665 weapons bound: 470 standard, 158 knight, 37 crossbow (642 before ITEM-ADD-1 added
  the donor epoch-2 Items; Snowball with Ice Shards 53855 is an admitted appearance-only Item
  bound to p474 Standard without materialization or weapon-mechanics inference).
- **Not bound:**
  - 1 weapon whose class is `unknown` (ink sword 51666);

  Ink Sword is only counted (`excluded`), and it gets no proficiency until its threshold evidence is admitted.

## Content population

`tools/content-migration/world_project_v2_to_tree.py` registers the committed Proficiency family (no legacy
source), as it does Charm, so `content --check` and the generator agree byte for byte. Nothing loads
`content/proficiencies/` at runtime; `runtime_source` stays `legacy_until_separately_qualified`.

```sh
pip install -r requirements.txt
python proficiency_authoring.py build --check
python proficiency_authoring.py validate samples/proficiencies-candidate.json
python proficiency_authoring.py content --check
python test_proficiency_authoring.py
```
