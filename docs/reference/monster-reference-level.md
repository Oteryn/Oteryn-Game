# Monster Reference Level evidence (NEWS-HIST-MRL-1)

Evidence classification: **UNKNOWN** for every numeric curve constant and every creature
reference-level value.

## Sources checked

- Tibia.com news 7852 (2024-04-16) states only that damage increases below an unnamed level cap,
  is reduced for the first 100 levels below that cap, and then increases faster. It names Rotten
  Blood, bulltaurs, and Nimmersatt, but gives neither the formula nor any cap value.
- Tibia.com news 8472 (2025-07-29) states only that Gnomprona monster damage was decreased at lower
  Hazard Levels, is unchanged at Hazard Level 12, and is slightly increased above Hazard Level 12.
  This is hazard scaling, not a numeric Monster Reference Level curve or creature cap.
- Canary `47dfd51f45280a59a1d3e50ba7edd573d7234446` and Crystal
  `201189f871d23597b82caf88fd611454a2ddefb8` were searched for `reference level`,
  `reference_level`, `referenceLevel`, and `monster reference`; neither pinned reference supplies
  the mechanic, formula constants, or creature cap values.

## Implementation disposition

The multiplier therefore remains neutral (`1_000_000` parts per million). `reference_level` is not
populated for Gnomprona or Rotten Blood creatures because their values are UNKNOWN. Both creature
melee and creature damage-spell dispatch carry the real target player's owned level through the
same multiplier seam, so sourced values can later be added without inventing another damage owner.
