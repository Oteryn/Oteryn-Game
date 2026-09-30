# Condition authoring schema candidate v1

The static `ConditionDefinition` catalogue of COND-CONTENT-1
([CONDITIONS-0](../../../docs/architecture/reviews/OTERYN_GAME_CONDITIONS0_ACTOR_CONDITIONS_DECISION_2026-09-30.md)):
the values for the six §3 families (`SPEED`, `DAMAGE_OVER_TIME`, `FOOD_REGENERATION`, `RECOVERY`,
`MANA_SHIELD`, `LIGHT`) used by spells, runes, potions, food, fields, monster attacks and charms.
Content owns the values. The conflict policies, ticks, dispel, lifecycle and the speed and
capacity arithmetic belong to the COND-1 runtime. Nothing here loads at runtime.

`capture`, `build`, `validate` and the tests write only under this directory. `content` writes
`content/conditions/` and registers `Condition` in `content/project.json`, `content/manifest.json` and
`content/content.lock.json`. Only the control plane's content train runs it; this package does not
commit any content.

| File | Purpose |
|---|---|
| `condition.schema.json` | One catalogue (`OTERYN_CONDITION_AUTHORING_CATALOGUE/v1`), JSON Schema 2020-12, closed shapes. A definition has `identity {family: Condition, key, revision}`, `family`, `conflict_key`, `negative` (the §5 dispel tag), `used_by`, `evidence` and `status`. An `admitted` definition carries exactly one value block, named after its family. A `blocked` definition carries a `blocked_reason` and no values. |
| `authored-conditions.json` | The hand-authored rows (spells, runes, potions, food, charms). Each row has its values and the Canary `path:lines` with the literal text (`needles`) those lines must contain. |
| `condition_authoring.py` | `capture --canary <checkout>` checks every needle in its cited lines and parses the `field` items of `items.xml`. `build` derives the catalogue (`build --check` diffs it). `validate` runs the schema and the rules. `content [--check]` writes or verifies the content family. |
| `test_condition_authoring.py` | No-network tests: the field parse order, the committed build, the authored values, the monster mirror, one negative case per rule. |
| `samples/canary-condition-sources-04b83b51.json` | The captured Canary facts: the git blob of every cited file, and the field items. |
| `samples/conditions-candidate.json` | The candidate catalogue, one definition per line: 842 definitions, 828 admitted, 14 blocked. |

## Sources

- **Canary `04b83b51` (the CONDITIONS-0 pin), `OtsHypothesisOnly`.** Values for:
  - spells and runes: Haste, Strong Haste, the three Light spells, Recovery, Intense Recovery, Magic
    Shield, Curse, Electrify, Envenom, Ignite, Inflict Wound, Soulfire Rune, Paralyse Rune;
  - potions and food: the Magic Shield Potion (35563), Filled Jalapeño Peppers (9085), Demonic Candy Ball (11587);
  - charms: Cripple, Numb, Adrenaline Burst (`iobestiary.cpp`), under the keys CONDITIONS-0 names;
  - fields: `items.xml` read in `ItemParse::parseFieldCombatDamage` order. A `start` after `damage` is
    ignored, as in Canary (energy field 2126). A field without damage applies no condition, so it has no definition.
- **Committed monster Effects**, as converted from Canary `47dfd51f` (monster schema §8, D12): every
  inline `Condition` Effect in `content/abilities/definitions/` whose type is a §3 family. The key is
  `oteryn:condition.monster.<Effect key tail>`. The damage schedule is copied unchanged
  (`ProjectV2DamageOverTime`). The speed coefficients come from the `SpeedModifier` Formula
  (`monsters.cpp:153-154`: `(m/2, 40, m, 40)`), in thousandths.
- **Food regeneration**: ITEM-USE-0 §6.1 (the 1,200 s cap; `foods.lua:145-146`). The rates come from the
  vocation data, and the time per food from the Item.

## Shape decisions

- **Speed.** `a_min`, `b_min`, `a_max`, `b_max` as in CONDITIONS-0 §3 (`a` in thousandths). Canary's fixed
  `CONDITION_PARAM_SPEED` delta `d` (special foods) is encoded exactly as `a = 1000`, `b = d + 40`.
- **Damage over time.** `damage_over_time.schedule` is the WorldProject/v2 shape (`Fixed`, `Decreasing`,
  `Geometric`). `first_tick: AfterInterval` is the §3.1 `delayed`. A field has `field: true` and ticks at
  once (Canary does not delay field conditions). A monster `Decreasing` total may start at 0: Canary
  draws the total, and a zero draw starts nothing.
- **Mana shield.** `capacity = min(max mana, trunc((constant + per_level × level + per_magic_level ×
  magic level) / 1000))`. The Wheel ×1.25 grade belongs to the Wheel and is not modelled.

## Blocked (fail-closed: not in `content/`)

- **10 monster speed conditions** (for example Bazir, Orshabaal, Gladiator): the Canary multiplier is
  not a whole number of thousandths, for example 2.901 / 2.
- **Searing fire** (2137, 2138, 7465-7473): one 300-damage tick with interval 0, below `COND0-RL-02`.
- **Holy Flash**: the tick count is `math.random(7, 11)`, drawn once when the script loads.

## Rules (`validate`)

- The schema passes, and keys are unique.
- The conflict key belongs to the family (`speed`; the element; one key each for the others).
- A record is admitted exactly when it carries its family's value block.
- `negative` marks exactly damage over time and paralysis.
- Every tick and regeneration interval is at least 1,000 ms (`COND0-RL-02`).
- Only fields are `field` and tick at once.
- `a_min ≤ a_max`, and a damage total's minimum is at most its maximum.
- The food cap is 1,200 s.
- At most 16 conflict keys are in use (`COND0-RL-01`).

```sh
pip install -r requirements.txt
python condition_authoring.py build --check
python condition_authoring.py validate samples/conditions-candidate.json
python test_condition_authoring.py
python condition_authoring.py content --check   # after the content train has populated content/conditions/

# evidence refresh (Canary checkout at 04b83b51; local only)
python condition_authoring.py capture --canary <canary checkout>
python condition_authoring.py build
```
