# SKILL-FORMULAS-CONTENT-1 evidence

Pinned skill and leech formulas remain inspectable candidates. Actual source conflicts are Prey damage base, Monk constants and None magic multiplier; index conventions are reconciled. Exact Global formulas/rounding are unqualified. Official critical base5%chance and10%extra damage are preserved.

damage_prey: Community exclusion versus donor adjusted-damage call path; Global test unresolved.

monk: Wiki/calculator magic1.25,melee1.4,shield1.15versusdonor1.3/1.2/1.1.

none: Wiki mana multiplier3.0versusdonor4.0.

offset: Current skill exponent s-10 equals target level exponent L-11 when L=s+1; not an off-by-one conflict.

cumulative: Sum individually truncated donor level costs; do not substitute an unrounded geometric sum.

leech: Shared helper does not include every additional charm/wheel/talisman effect; realDamage is health-capped in consumer path.

The inspectable source candidates and unresolved fields are in [gameplay-source-reconciliation.json](content-prep/skill-formulas-content-1/gameplay-source-reconciliation.json); all source URLs, revisions and digests are in [sources.json](content-prep/skill-formulas-content-1/sources.json). Historical project selections are retained only as superseded entries in the proposal audit. They are not current runtime defaults.

Active classifications: 4 PROVEN, 15 UNKNOWN, 4 CONFLICT. No Rust, Cargo, migration, generated pins, runtime activation or merge.
