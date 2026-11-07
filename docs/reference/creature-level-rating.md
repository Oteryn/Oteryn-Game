# Creature level rating

**Oteryn rating (not an official Tibia formula).** This is an offline design and
guide aid. It neither changes gameplay nor writes `reference_level` (or any other
runtime value). The generator reads committed creature, spell-profile, spawn,
and hunting-place content and emits `content/creatures/level-rating.json`.

## Formula

All output numbers are integers; probability calculations use parts per million
and DPS uses milli-damage per second.

* **Burst** is the greatest maximum damage among attacks that can occur in one
  attack turn. An attack carrying an area/shape marker receives a 1.5 multiplier.
* **DPS** sums `chance × (minimum + maximum) / 2 / interval`, plus periodic
  condition damage. Unknown attacks contribute zero rather than preventing a
  creature from being rated.
* **EHP** is `health / (1 - mean resistance)`, over physical, fire, ice, earth,
  energy, death, and holy damage, multiplied by the committed armor and
  mitigation adjustments.
* **Survival level** is the first level whose vocation HP can retain at least
  50% after the exposed burst of the default three-creature pack. Party values
  divide that burst among four members. Mage growth has the required five HP per
  level floor (both mage vocations use five).
* **Pace level** is the first level at which the linear representative player-DPS
  curve defeats EHP inside the reasonable-kill window. **EXP level** is
  `1.1 × experience^0.55`. Suggested level is the larger of weakest-vocation
  survival and pace. `review` is true when suggested and EXP levels differ by
  more than 30% of EXP level.
* An area's guide minimum is the nearest-rank 75th percentile of its creature
  survival levels, for each vocation in solo and party play. The upper endpoint
  is a calibrated triviality curve (burst below 10% HP and kill time below eight
  seconds), fitted against the observed caps below from mean creature experience
  and 75th-percentile burst. Named hunting places retain their names; otherwise
  spawns are grouped into deterministic 256×256×floor region IDs.

## Constants

| Constant | Value |
|---|---:|
| Pack size / party split | 3 / 4 |
| Area damage weight | 1.5 |
| Incoming exposure | 0.350 |
| Base HP at level 8 (druid, sorcerer, paladin, knight) | 150, 150, 155, 185 |
| HP/level (druid, sorcerer, paladin, knight) | 5, 5, 10, 15 |
| Representative DPS | 20 + 0.8 × level |
| Reasonable / trivial kill time | 35 s / 8 s |
| EXP fit | 1.1 × experience^0.55 |
| Triviality cap fit | −327.413 + 13.797 × floor(√mean EXP) − 0.563 × P75 burst |

The negative burst coefficient is a calibration correction, not a claim that
more burst is safer: survival supplies the range floor independently. Constants
are centralized in the generator so reviews can identify formula changes.

## Calibration and parity notes

The observed comparison values come from the Tibia community's
[Monster Reference Level description](https://tibia.fandom.com/wiki/Monster_Reference_Level)
and [MRL discussion](https://guildstats.eu/forum/viewtopic.php?f=8&t=15719).
The unittest permits ±10%; the current integer fit is exact for all three anchors.

| Area | Oteryn | Tibia observed | Canary | Crystal |
|---|---:|---:|---|---|
| Bulltaur Lair | 300 | about 300 | UNKNOWN | UNKNOWN |
| Nimmersatt's Breeding Ground | 454 | 454 | UNKNOWN | UNKNOWN |
| Rotten Blood (represented by Rotten Wasteland content) | 790 | 790 | UNKNOWN | UNKNOWN |

## Twenty highest-rated areas (example)

| Area | Recommended levels | Mean EXP/creature |
|---|---:|---:|
| Upper Roshamuul | 188–3476 | 87908 |
| Warzone 1 | 205–3328 | 82690 |
| Roshamuul Prison | 398–3262 | 90906 |
| Norcferatu Dungeons | 188–3061 | 70836 |
| Podzilla | 650–3021 | 95255 |
| Warzone 2 | 243–2974 | 70658 |
| Warzone 5 | 168–2841 | 61860 |
| Warzone 3 | 272–2797 | 65314 |
| Isle of Ada | 188–2316 | 45007 |
| Warzone 4 | 130–2188 | 38925 |
| Warzone 6 | 252–2011 | 38539 |
| Radiant Skyhold | 398–1938 | 42157 |
| Radiant Ascendancy | 398–1910 | 41608 |
| Stag Bastion | 188–1792 | 30342 |
| z04-x132-y127 | 420–1634 | 34427 |
| z05-x132-y127 | 420–1634 | 34427 |
| z02-x132-y127 | 420–1620 | 34067 |
| z03-x132-y127 | 420–1620 | 34067 |
| Secret Library | 272–1348 | 22601 |
| z10-x132-y122 | 524–1203 | 27000 |

Regenerate with `python3 tools/content-migration/creature_level_rating.py` and
verify freshness with the same command plus `--check`.
