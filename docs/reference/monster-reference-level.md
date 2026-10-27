# Monster Reference Level evidence (NEWS-HIST-MRL-1)

Evidence classification: **UNKNOWN** for the official formula and official creature caps;
**DERIVED** for the observed approximation below. Generated creature caps remain explicitly null.

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

## Observed approximation (no official formula; Canary/Crystal UNKNOWN)

The 2024-04-16 change describes a softer first 100 levels below a cap followed by a faster
increase. The generic Oteryn fallback uses +0.2% damage per missing level through 100, then
+1% per missing level. These are approximation constants, not an official formula.
At or above the cap, or with a null cap, the multiplier is exactly `DAMAGE_MULTIPLIER_SCALE`.
Integer parts per million keep computation deterministic; damage rounds down once before
existing armor/resistance processing. Extremely large multipliers saturate at `u32::MAX`;
existing checked damage conversion refuses an unrepresentable hit.

Community observations do not establish one universal curve. In particular, Rotten Blood's
+42% at 59 levels below its proposed cap and +58% at 240 levels below cannot both fit a
curve with two constant slopes, the larger after 100. Reports may involve different dates, mitigation,
rounding, or approximate caps. We preserve that conflict rather than call any fit official.
Area calibration interpolates observed points from `(0, 0)`, with an additional derived knot
at 100 missing levels. The segment after that knot is over three times steeper than the segment
just before it. Those knot values are fitting choices, not additional community observations;
the Rotten Blood fit needs a nonuniform slope within the first 100. Beyond the last point
the curve adds +1% per missing level. These fits do not establish an official formula.
The checked [observation data](monster-reference-level-observations.json) has these examples:

| Area | Approximate configured cap | Player level | Extra damage |
|---|---:|---:|---:|
| Bulltaur Lair | 340 (within the reported ~300+) | 188 / 276 | 80% / 22% |
| Nimmersatt Breeding Ground | 454 | 220 / 454 | 60% / 0% |
| Rotten Blood | 790 | 550 / 731 / 790 | 58% / 42% / 0% |

These examples are test calibration data, not enabled gameplay content. No area name appears
in the Rust implementation and no observed cap is silently assigned to generated creatures.

## Authoring and dispatch

Every generated Creature's `authoring.profile.details` contains `reference_level: null` unless
explicitly authored otherwise. Null disables MRL. The creature binder and tree generator emit
that key in the generated shards and native profiles. Legacy canonical WorldProject serialization
keeps its historical omission to preserve qualified predecessor digests.

An optional `reference_level_overrides` list in those details applies to spatial spawn areas.
Each entry has `bounds` (`min_x`, `min_y`, `max_x_exclusive`, `max_y_exclusive`), `floor`, an
explicit nullable `reference_level`, optional `party_mode`, and optional `curve`. The issuer's
current position selects a disjoint area; outside the boxes the creature cap applies. Thus the
same creature can be enabled in one area and disabled in another. Areas use the current world's
coordinate frame and are half-open in x/y. Overlap, inverted bounds and zero caps are rejected.
A `curve` contains increasing positive `levels_below` with monotonic `extra_damage_ppm` values.
Omitting it uses the generic soft-100 approximation.

`party_mode` defaults to `per_player`: each hit uses that player's owned level, never a party
average. The optional `lowest_member` mode is an **Oteryn anti-carry rule, not a Tibia mechanic**.
It uses the lowest level of the hit player's party members currently inside the selected box
and floor. The production scheduled spell reader uses the existing strong World party read
under its scope transaction, joins current player positions and levels, and falls back to the
hit player's level when lookup is unavailable. Cached party projections cannot grant membership.
Other combat readers must supply a current same-area minimum or leave it unavailable; melee
uses the same scaling resolver and fallback. No override means `per_player`.

An area cast draws its base damage once; only target-specific scaling differs. Occurrence
receipts retain replay behavior. MRL affects incoming damage only, never experience or loot.

## Community sources

- [TibiaWiki: Monster Reference Level](https://tibia.fandom.com/wiki/Monster_Reference_Level)
  (community reference; direct retrieval unavailable during this task).
- [GuildStats discussion](https://guildstats.eu/forum/viewtopic.php?f=8&t=15719)
  (community reference; direct retrieval unavailable during this task).
- [TibiaQA: minimum levels for MRL areas](https://www.tibiaqa.com/35742/what-minimum-level-required-areas-where-monster-reference-level-active)
  (2024-04-16): confirms the named areas and hidden caps, but supplies no official numeric formula.

The numeric observations above were supplied in the MRL-3 owner task; they remain community
observations, not independently verified official constants. The pinned Canary and Crystal
search results above remain UNKNOWN and do not justify claiming source parity.
