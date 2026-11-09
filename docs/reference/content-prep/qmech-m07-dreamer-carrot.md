# M07 Dreamer's Challenge: Carrot entrance

Reference: Crystal `201189f871d23597b82caf88fd611454a2ddefb8`,
`data-global/scripts/quests/dreamers_challenge_quest/movement_carrot.lua:9-32`.
UID2241 checks for a carried carrot3595 before checking Premium. Success consumes
one carrot and teleports to `(32861,32235,9)`. Missing carrots return the actor to
the origin and apply a 10–20 fire roll through the existing combat owner. Free
accounts carrying a carrot receive Hugo's source speech before the same refusal.
Teleport cues use source constant11; fire impact uses16. Native protection applies.

This callback uses the fenced Quest writer and carried consumption owner without
inventing a progress flag. Migration2602 preserves existing receipt predicates and
requires the exact same-transaction carried carrot debit for the empty STEP receipt.
The registered callback is closed over its UID, entitlement, input and destination.
Known source refusals apply damage only after the return movement commits. Replayed
commands do not debit another carrot or repeat damage and speech.

The native generator adds only carrot3595, using identical pinned Crystal and
Canary XML node SHA256
`6c756e98369225f43afdda2044254cd3004d32c841ce83cebd78897e36704180`.
The existing canonical materializable StackCapable classification remains required.
Base profiles5783→5784 and compiled cohort5799→5800; no existing profile is changed.

The mounted PostgreSQL cases use literal UID2241 on synthetic source geometry.
They qualify the real dispatch STEP and persistence owners, not a recovered UID
position in the production map. Generic route authoring remains M01's boundary.
The retained donor world and custom maps did not provide this UID correspondence;
no native position alias is inferred from a teleport destination.

Validation: both mounted PostgreSQL18.6 cases pass through real STEP ingress,
including carried custody, exact debit, Premium refusal, origin fire, missing-item
branch order, empty progress and replay. Three-crate check and all-target/all-feature
Clippy pass. Five focused Dreamer Quest tests pass. Provider suites have9 and13
passes, generated native pins are fresh, and the exact production cohort case
passes. The route-less filtered run has7 passes. Local node boot is skipped as
instructed. The unrelated Ice Islands hash-baseline mismatch remains unchanged.
