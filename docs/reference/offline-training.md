# TRAINING-CONTENT-1 evidence

Exercise rates retain exact donor and community candidates, including the7versus7.2melee disagreement and Bow unit differences.600base mana is corroborated but final gains and offline formulas remain unqualified. Owner-deferred measurements are pending;32item/charge facts are preserved.

melee: Donor7base tries versuscommunity7.2regular hits equivalent.

bow: Donor7distance tries versuscommunity2.16blood/4.32missandolder3.6prose; units not interchangeable.

mana: 600base spent mana matches donor/wiki; final gain also depends on multipliers and callbacks.

offline: Donor promoted-vocation timing and rate configuration retained; no Global per-vocation equivalence claimed.

stamina: No exercise-local drain call does not prove absence of every downstream drain/regen path.

deferred_test: Owner postponed Global exercise measurement; do not spend weapons or launch a test.

The inspectable source candidates and unresolved fields are in [gameplay-source-reconciliation.json](content-prep/training-content-1/gameplay-source-reconciliation.json); all source URLs, revisions and digests are in [sources.json](content-prep/training-content-1/sources.json). Historical project selections are retained only as superseded entries in the proposal audit. They are not current runtime defaults.

Active classifications: 58 PROVEN, 3 UNKNOWN, 4 DERIVED, 2 CONFLICT. No Rust, Cargo, migration, generated pins, runtime activation or merge.

## T-OFFTRAIN: initial ten minutes (2026-10-29)

The pinned Canary (`47dfd51f45280a59a1d3e50ba7edd573d7234446`) and Crystal
(`201189f871d23597b82caf88fd611454a2ddefb8`) login handlers agree:
`data/scripts/creaturescripts/player/offline_training.lua:3-5` computes the
full logout interval; lines 14-17 return without training for intervals below
600 seconds. Lines 19-24 train from that full interval, debit the pool, and
refill from the full interval minus training. Neither subtracts the initial
600 seconds; exactly 600 seconds can train. Lines 7-10 refill the full
interval when no training skill is selected. These are `OTS_HYPOTHESIS_ONLY`
sources, inspected before changing the planner.

Official Get Help entry 178 takes precedence: the character must be logged
out for **more than** ten minutes; the first ten minutes neither train nor
consume the pool, and are excluded from the subsequent refill calculation.
The source is [Get Help entry 178](https://www.tibia.com/support/?subtopic=gethelp&entryid=178),
retrieved 2026-10-29; its four worked examples are reproduced here. For 15h10m offline, starting pools of 12h,
6h30m, 3h and 2h train those amounts and finish with 3h, 8h30m, 12h and 12h,
respectively. The 43200-second pool cap stays in effect. This correction
applies to selected offline training; ordinary refill without a selection
retains the full elapsed interval.
