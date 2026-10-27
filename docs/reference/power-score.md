# Player Might and creature Threat

`tools/content-migration/power_score.py` builds `content/creatures/power-score.json` from pinned repository data only. This offline guide is not a wire or persistence contract. Might and creature Threat share the dimensionless scale `round(sqrt(offence_or_DPS × effective_HP) / 10)`; coefficients are empirical guide parameters, not runtime damage or verified loadout rotations.

## Player model

Inputs are vocation, level, representative skill (or the typical curve), normal/BiS gear, and optional fractional imbuement/wheel offence modifiers. Typical skill is `round(base + slope × log2(max(level, 8) / 8))`: knight 55/18, paladin 50/17, sorcerer/druid 30/13. Monk is supported by the calculator with its original coefficients, but excluded from fitting because places have no monk recommendations.

Offence is `(weapon × (skill + 4)/28 + level × spell_coefficient)/cadence`, times the gear multiplier and `1 + imbuement + wheel`. Weapon values are K/P/S/D/Monk = 50/48/36/36/46; cadence is 2/2/2.4/2.4/1.8 seconds. The weapon term follows the pinned `content/combat/attack_constants_v1.json` convention. Effective HP is `(HP + usable_mana_shield + 30 × healing_per_second)/(1 − mitigation)`. HP/mana start at 185/90 at level 8; HP gains are 15/10/5/5/10 and mana gains 5/15/30/30/10. Mages use 35% of mana as shield. Healing is `4 + level × {0.14, 0.22, 0.30, 0.34, 0.20}` per second. Mitigation is `min(player_mitigation_cap, skill/650 + (0.06 for BiS))`.

The generated `constants.model` is authoritative for the fitted parameters:

| parameter | value |
|---|---:|
| aoe_druid | 0.88 |
| aoe_knight | 0.5 |
| aoe_paladin | 0.65 |
| aoe_sorcerer | 0.9 |
| bis_multiplier | 1.22 |
| creature_mitigation_cap | 0.4 |
| pack_pressure | 0.18 |
| party_constant | 0.12 |
| player_mitigation_cap | 0.48 |
| spell_druid | 32.0 |
| spell_knight | 64.0 |
| spell_paladin | 64.0 |
| spell_sorcerer | 32.0 |

### Might examples

Each cell is **normal/BiS**. High fitted spell coefficients compensate for the representative rotation/model; they must not be copied into runtime spell formulas.

| level | knight | paladin | sorcerer | druid |
|---:|---:|---:|---:|---:|
| 8 | 34/39 | 34/39 | 23/26 | 23/27 |
| 50 | 152/174 | 144/165 | 106/121 | 108/124 |
| 100 | 293/336 | 274/314 | 204/233 | 208/239 |
| 200 | 578/664 | 537/616 | 402/460 | 411/470 |
| 400 | 1159/1333 | 1070/1230 | 801/918 | 820/940 |
| 700 | 2042/2353 | 1882/2167 | 1407/1613 | 1440/1652 |
| 1000 | 2936/3387 | 2701/3112 | 2017/2315 | 2066/2371 |

## Attack data and completeness

The generator follows each creature's bound behavior, its `attacks[]` schedules, abilities, executable effects, and referenced formulas under `content/abilities/{definitions,effects,formulas}`. It also checks creature `attack-*` references so an absent schedule cannot silently drop a damaging attack. Each output attack records source file paths, ability/effect keys, formula key, classification, and numeric schedule/range where available.

`Range` uses authored minimum/maximum. `CasterMagnitude` uses the behavior's magnitude. `MeleeAttackSkill` follows `crates/gameplay/combat-rules/src/attack/formulas.rs`: `[0, ceil(skill × attack/20 + attack/2)]`. Only effects with `effect_family: Damage` contribute DPS: `mean_damage × chance_ppm/1,000,000 ÷ interval_seconds`. Field/item creation, summons, healing and condition-only attacks are `non_damage`, not missing damage. This is direct-damage Threat; delayed conditions, summons and fields can still create danger absent from the score.

Damage without resolvable numbers or scheduling, and broken graph references, are `unresolved`. Any unresolved attack keeps the creature `missing_attacks` and its Threat **null**, even if other attacks have known DPS. Nothing is filled from outside the repository or with guessed damage. Zero direct-DPS creatures also retain null Threat. Creature effective HP is `HP/(1 − min(creature_mitigation_cap, (armour + defence)/500))`. Resistances are preserved, but the representative model does not choose an optimal attack element.

Across 1,863 creatures, missing attacks fell from **547 to 0**. There are **5,231 damage** effect rows and **790 non_damage** effect rows, with **0 unresolved**. Hydra now resolves from the graph. Reward bosses and zero-XP creatures with at least 100,000 HP remain excluded from green levels and calibration.

The previous import has 661 unavailable entries. These counts are reason incidences: three entries have multiple reasons, so the before column sums to 664. The after columns classify those same entries by their use in canonical creature attacks. Twelve legacy import identifiers are unreferenced, rather than claimed as recovered canonical attacks.

| import reason | before unavailable | after damage | after non_damage | after unresolved | unreferenced |
|---|---:|---:|---:|---:|---:|
| attack_count_differs_from_donor | 24 | 9 | 3 | 0 | 12 |
| conflicting_creature_bindings | 33 | 33 | 0 | 0 | 0 |
| donor_ability_has_no_effects | 36 | 0 | 36 | 0 | 0 |
| donor_health_conflict | 298 | 256 | 42 | 0 | 0 |
| no_unique_donor_bundle | 269 | 243 | 26 | 0 | 0 |
| source_provenance_ambiguous | 4 | 4 | 0 | 0 | 0 |

| creature | DPS | Threat | normal green level (K/P/S/D) |
|---|---:|---:|---|
| Rat | 2.00 | 1 | 8/8/8/8 |
| Dragon | 44.25 | 22 | 9/9/15/15 |
| Hydra | 91.78 | 50 | 26/27/39/38 |
| Juggernaut | 396.75 | 311 | 180/194/259/253 |

## Spawn mapping and area encounters

`content/world/spawns/index.json` enumerates the Spawn.Source shards and coordinate frame. Each declaration has a centre and `points[]`; count **points**, not groups/centres. Each point has a creature key and `cell {x,y,floor}`. Hunting places under `content/world/areas` have point positions, not polygons/bounds. Both use `global-target-2026-09-27`.

Assign each spawn point to the nearest hunting-place position within **128 tiles Euclidean distance**, on the **same floor**, whose listed creatures include that creature key. Ties use ascending area key. Each point belongs to at most one place; respawn time is not a population multiplier. This is a bounded anchor approximation, not a claim of exact cave boundaries. Entrance anchors do not establish underground floors, and nearby overlapping hunts may divide a population imperfectly. No floors, regions or coordinates are inferred from names.

An area with assigned points records their counts in `spawn_weights` and `spawn_weight_source: "spawns"` (**253 areas**). An area with no usable anchor or no matching points falls back to one per listed, matched kind and `"uniform"` (**192 areas**). Within a spawn-weighted area, unobserved species get no fabricated point. Names are case-insensitively matched to creature definitions; unresolved listed names remain outside the model.

For non-boss, complete creatures with non-null Threat, the area uses the **weighted mean** creature Threat. Counts describe a population, not a simultaneous whole-map pack. Let `N` be the usable population count, `pressure = 1 + pack_pressure × log2(max(1,N))`, and effective `pack_size = min(N,8)`. Base area `threat` is weighted mean × pressure. `threat_by_vocation` additionally multiplies this by `sqrt(1 + (pack_size − 1) × (1 − aoe_share))`; green levels compare that encounter Threat to player Might. `spawn_count`, `pack_size` and pressure are emitted separately. The eight-target cap and 128-tile mapping limit are fixed modeling assumptions, not fitted against holdout recommendations. Areas without any usable creature have null Threat and no calibration observations.

| area | base Threat | normal green level (K/P/S/D) |
|---|---:|---|
| Ab'Dendriel Elf Cave | 1.0 | 8/8/8/8 |
| Ab'Dendriel Slimes Cave | 3.85 | 8/8/8/8 |
| Abandoned Sewers | 110.96 | 89/88/100/99 |
| Al Dee Rat Cave | 1.29 | 8/8/8/8 |
| Alatar Lake | 4.72 | 8/8/8/8 |

## Verdict, parties and fitting

The ratio boundaries stay **≤.25 trivial (grey), ≤.60 easy (green), ≤1 fair (yellow), ≤1.60 hard (orange), otherwise deadly (red)**, matching the unchanged Rust `threat_tier` helper. The green level is the first integer level from 8 through 2000 at or below .60. Above-range predictions remain null in the guide and count as **2001** in metrics; difficult observations are never discarded for exceeding the search range.

The `encounter` helper reports pack time-to-kill with additional HP discounted by fitted vocation AoE share, time-to-die against summed pack DPS, and a conservative summed-pack Threat verdict. `party_might(per_player)` is `sum(Might)/sqrt(1 + party_constant × (members − 1))`; `lowest_member` uses the minimum. BiS and party constants are **unidentifiable** from solo, normal-gear recommended levels, so retain 1.22 and .12. Reporting them as fitted would be unsupported. All other free constants requested are searched; a cap may remain at its initial value when the data do not activate it.

Fit deterministically with stdlib only: start from POWER-SCORE-1 constants and run **four coordinate sweeps** in this order: K/P/S/D spell coefficients, player mitigation cap, creature mitigation cap, pack pressure, K/P/S/D AoE share. Grids are spell `{.1,.25,.5,1,2,4,8,16,32,64,128}`, player cap `{.1,.25,.48,.65,.8}`, creature cap `{.1,.25,.4,.65,.8}`, pressure `{0,.05,.1,.18,.3,.5}`, AoE `{0,.25,.5,.65,.72,.88,.9,.95}`. Retain the current value on ties. Minimize training MAE rounded to two decimals. This bounded coordinate search is not a proof of a global optimum.

Sort all 445 areas by key **before** exclusions and hold out every fifth (one-based positions 5,10,…). Neither model selection nor coefficient fitting reads holdout recommended levels. Mage recommendations supply separate sorcerer and druid observations; those are correlated, not independent hunts. Recommendations encode safety, routes and profit beyond direct combat. Remaining errors, incomplete geometry and omitted indirect damage limit using this as a calibrated guide to actual player risk.

Calibration has **1500 observations**, overall MAE **45.14 levels**. Train and holdout metrics below exactly match the generated `calibration.metrics` block.

| split | vocation | observations | MAE levels | median absolute error levels |
|---|---|---:|---:|---:|
| train | all | 1191 | 45.44 | 26 |
| train | druid | 297 | 43.9 | 25 |
| train | knight | 299 | 48.83 | 30 |
| train | paladin | 298 | 45.28 | 27.0 |
| train | sorcerer | 297 | 43.75 | 25 |
| holdout | all | 309 | 43.97 | 33 |
| holdout | druid | 78 | 42.72 | 33.5 |
| holdout | knight | 77 | 47.51 | 37 |
| holdout | paladin | 76 | 43.09 | 31.0 |
| holdout | sorcerer | 78 | 42.6 | 31.5 |
| overall | all | 1500 | 45.14 | 27.0 |
| overall | druid | 375 | 43.65 | 26 |
| overall | knight | 376 | 48.56 | 31.0 |
| overall | paladin | 374 | 44.84 | 27.0 |
| overall | sorcerer | 375 | 43.51 | 26 |

POWER-SCORE-1 reported 1,067 observations and overall MAE 366.26. Applying the same fifth-area split to its original output gives train MAE 369.43 (838 observations), holdout MAE 354.64 and median 92 (229 observations). It discarded predictions above 2000 and had incomplete creature coverage, so the observation populations differ; this is not a paired statistical comparison.

## Reproduction

Output uses sorted keys and sorted area/creature records, with one compact JSON record per line inside a pretty top level. There are no timestamps, random samples or network reads. Repeating generation in separate processes must match the checked-in bytes.

```console
python3 tools/content-migration/power_score.py --output power-score-run1.json
python3 tools/content-migration/power_score.py --output power-score-run2.json
cmp power-score-run1.json power-score-run2.json
python3 -m unittest tools/content-migration/test_power_score.py
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-domain power_score
/home/mole/oteryn-cache/codex-bin/cargo fmt --check
```
