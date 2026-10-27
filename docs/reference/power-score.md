# Player Might and creature Threat

`power_score.py` builds `content/creatures/power-score.json`; this is offline guide data, not a wire or persistence contract. Might and Threat deliberately share the dimensionless scale
`round(sqrt(offence × effective-HP) / 10)`.

## Player model

The input is vocation, level, representative skill (or the typical curve), gear (`normal` or `bis`), and optional fractional imbuement/wheel offence modifiers. The typical trained curve is
`round(base + slope × log2(max(level, 8) / 8))`: knight 55/18, paladin 50/17, sorcerer and druid 30/13 (base/slope). It represents consistent offline training, not a requirement. Monk is accepted by the calculator when present, but is omitted from calibration because the pinned places have no monk recommendation.

The weapon term follows the skill offset/divisor and two-second base cadence in [`content/combat/attack_constants_v1.json`](../../content/combat/attack_constants_v1.json). Player spell tables are available under `content/abilities/{definitions,formulas}`; the guide combines attacks into a representative rotation rather than claiming maximum burst. Assumptions not supplied as a single content record are: representative weapon attacks (knight 50, paladin 48, mage 36, monk 46); spell level coefficients (0.34, 0.48, 0.82, 0.76, 0.44); mage 2.4 s and monk 1.8 s effective rotation cadence; BiS offence ×1.22; HP/mana gains from the vocation progression convention; mage mana-shield availability at 35% of mana; mitigation `min(48%, skill/650 + 6% for BiS)`; and sustained healing per second `4 + level × {0.14, 0.22, 0.30, 0.34, 0.20}`. Healing is capped naturally by the 30-second comparison window. These are versioned in the generated constants/model and must be recalibrated when runtime content gains exact loadout rotations.

Effective HP is `(HP + usable mana shield + 30 × healing/s) / (1 − mitigation)`. Imbuement and wheel values default to zero and multiply offence by `1 + imbuement + wheel`; no bonus is silently assumed.

### Might examples

Each cell is **normal/BiS**.

| level | knight | paladin | sorcerer | druid |
|---:|---:|---:|---:|---:|
| 8 | 14/16 | 14/16 | 9/11 | 10/11 |
| 50 | 38/43 | 35/40 | 29/33 | 29/34 |
| 100 | 58/66 | 53/61 | 48/55 | 49/56 |
| 200 | 91/105 | 85/97 | 84/96 | 84/96 |
| 400 | 148/171 | 141/162 | 152/174 | 151/174 |
| 700 | 226/260 | 220/254 | 252/289 | 251/288 |
| 1000 | 299/345 | 296/342 | 352/404 | 349/401 |

## Creature and spawn model

For every creature the output preserves HP, armour, defence, resistances, speed, maximum melee, every authored attack's element/range/interval/chance, total DPS, and per-element DPS. Attack cadence and ranges come from the pinned `monster-attack-authoring.jsonl`; defence/armour and creature profiles come from canonical content. Effective HP uses `min(65%, (armour + defence)/500)` mitigation (an explicit guide assumption). An absent/unavailable attack produces `data_quality: "missing_attacks"`, a `null` Threat, and exclusion from calibration—never zero damage. Thus 1st Mate Ratticus remains unknown, while Bulltaur Brute uses its available 170–300 melee range. Reward bosses, plus zero-XP/100,000+ HP encounter bosses, are flagged and excluded.

| creature | DPS | Threat | normal green level (K/P/S/D) |
|---|---:|---:|---|
| Rat | 2.00 | 1 | 8/8/8/8 |
| Dragon | 44.25 | 22 | 48/55/69/68 |
| Hydra | 91.78 | unknown | excluded: missing attack data |
| Juggernaut | 396.75 | 311 | 1920/1882/1502/1517 |

An area's creatures are matched from the pinned hunting-place facts. Until explicit spawn counts exist, each listed kind has weight one. Pack pressure is `1 + 0.18 × (known kinds − 1)` and area Threat is the weighted Threat sum times that pressure. This preserves meaningful ranges rather than collapsing every area to level 8.

| area | Threat | normal green level (K/P/S/D) |
|---|---:|---|
| Ab'Dendriel Elf Cave | 20 | 42/48/61/60 |
| Ab'Dendriel Slimes Cave | 45 | 149/167/173/172 |
| Abandoned Sewers | 145 | 765/785/668/673 |
| Alatar Lake | 9 | 9/10/17/17 |
| Alchemist Quarter | 414 | >2000/>2000/>2000/>2000 |

## Verdict, parties, and calibration

For a pack, time-to-kill is effective pack HP divided by player DPS; additional targets are discounted by vocation AoE share (knight .65, paladin .72, sorcerer .90, druid .88). Time-to-die is player EHP divided by pack DPS. Threat/Might ratios are: `≤.25 trivial` (grey), `≤.60 easy` (green), `≤1 fair` (yellow), `≤1.60 hard` (orange), otherwise `deadly` (red). The green level is the first level at or below the easy boundary, for each vocation and gear tier.

`party_mode=per_player` (default) yields `sum(Might)/sqrt(1 + .12 × (members − 1))`; `lowest_member` uses the lowest member. The same verdict calculation then uses party Might. The pure Rust domain helper applies these boundaries, with `Boss` overriding colour.

Calibration compares non-boss, complete-data area predictions with the pinned TibiaWiki recommended levels already in `content/world/areas/hunting-places`; no live site is scraped. The current generated fit contains 1067 observations and mean absolute error **366.26 levels**. This large error is reported rather than hidden: equal kind weights are not spawn counts, recommendations encode profit/route constraints, and incomplete attacks remove members. The dataset is useful as a deterministic first guide, but not a claim of parity; explicit spawn counts and runtime loadouts are the next calibration inputs.

## Reproduction

```console
python3 tools/content-migration/power_score.py
python3 -m unittest tools/content-migration/test_power_score.py
```
