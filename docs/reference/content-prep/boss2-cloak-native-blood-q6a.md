# Cloak native health callback: accepted Q6a physical blood

The accepted rule is authoring format section 13.7 Q6a in
`docs/architecture/OTERYN_ENCOUNTER_AUTHORING_FORMAT_V1.md`: a real Player hit
leaves a blood pool. This supersedes the literal donor's signed-damage predicate.

Source: [Crystal pinned Soul War mechanics](https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/soul_war/soul_war_mechanics.lua),
lines 807–824. The source captures the creature's position, checks specifically
for first-stage blood 33854, and creates it only when that stage is absent.
Mature blood 34006 or 34007 does not prohibit a new first-stage pool. Existing
qualified Item profiles retain the source decay chain: 33854 for 600 seconds,
34006 for 300 seconds, 34007 for 300 seconds, then retirement. Wiki: wiki-pending.

The production consumer accepts only a sealed, committed native health event,
retains it across a failed physical transaction, and acknowledges it after known
COMMIT. It uses the existing scoped Item writer and reconciles the original
receipt before deciding whether a first-stage pool already exists. Migration
2173 admits this closed Encounter cause without changing the Primal cause's
physical proof. No new protocol message or capability bit is allocated.

Regression: `srx3_cloak_accepted_q6a_actual_normal_player_hit_mints_blood_once_postgres`.
The pre-fix real-PG run reached the canonical HP event and failed with zero blood
Items instead of one (54.91 seconds). The fixture uses the real XP writer at level
2000 because its former level-500 fists completed sixteen swings without
penetrating the literal Cloak's defense and armor (107 each). Target statistics,
combat formulas, and all assertions remain intact. The regression also injects
a real receipt-insert failure and requires the event to survive until retry.
The post-fix run passed in 62.907 seconds; its explicit execution marker measured
62.759105367 seconds and recorded the injected SQL failure before successful retry.

This slice implements pool creation. The donor's separate pool StepIn behavior
(lines 830–855: Player damage, Cloak healing, and pool consumption) still requires
its own production composition; the wider W11 row remains PARTIAL.
