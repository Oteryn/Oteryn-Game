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
The expanded real-PG regression passed in 62.744 seconds (execution marker
62.679689028 seconds), additionally checking both role grants, the absence of a
PUBLIC grant, all three stored absolute decay deadlines, and a second real hit's
durable NoOp while first-stage blood exists. Seven native Cloak units passed in
89.10 seconds. Workspace compilation passed in 5m40s; dispatch Clippy passed in
2m26s, alongside formatting, equipment-pin verification, and migration checks.

The follow-up ordering regression failed in 67.941 seconds: after a real failed
receipt INSERT, a second due auto-attack consumed its canonical clock and ordinal
before the original blood callback completed. Both auto-attacks and monster AI
now recheck the retained callback under the same physical lane before fresh work;
the latter also closes the gap after its map-bootstrap preflight. The regression
passed in 68.409 seconds (execution marker 68.337201316 seconds), preserving the
entire held attack state, HP, and the one pending event across the SQL failure.

The Player StepIn slice uses Crystal mechanics lines 830–855 and
[Soul War constants](https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data-global/lib/quests/soul_war.lua):14,20–24:
33854/34006/34007 deal 20/15/10% maximum HP. Crystal
`src/lua/functions/creatures/creature_functions.cpp:591–610` sends the energy
health change through `combatChangeHealth` with a null attacker. The existing
direct Lua HP/shield/death owner handles it without another mitigation pass.

Migration 2174 admits only retirement backed by the original scoped native Cloak
MINT, preserving the source creator, body, scope generation, content, map, frame,
and physical position. Retained delivery resolves before later lane work. The
source Item reader supplies the actual destination pools; unrelated Item origins
are not granted this cause. A sparse cell hint, registered only after a genuine
MINT, routes environmental entry through that reader even without optional field
capability 30. The hint grants no Item or HP authority and is removed on consumption
or an authoritative empty read.

The native Player STEP regression was red in 69.605 seconds (HP stayed 30065
instead of 24052), then passed in 80.015 seconds (execution marker
79.945571675 seconds), including physical retirement after the creator disappears.
Both role grants and a real SQL rejection of a MINT presented as consumption were
also exercised. No new protocol messages or capability numbers were allocated.

The focused rerun preserves that healthy case and adds
`srx3_cloak_blood_step_unknown_commit_delivers_hp_once_postgres`: a real failed
retirement keeps the actual committed movement contact queued; the production
writer then commits without HP installation, and ordinary lane recovery reuses
that receipt and installs damage exactly once. The healthy and recovery cases
passed in 88.061 and 86.716 seconds respectively (execution markers
87.998849662 and 86.630097938 seconds). Both original HP and physical-consumption
assertions remain in each case, with one retirement receipt after reconciliation.

W11 remains PARTIAL: Cloak healing, consumption by other creatures, unrelated
pool origins, and a native-client play test of this new StepIn slice remain.
