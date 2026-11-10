# M25 Ahau movement exit: implementation and validation boundary

This is not a completion report. Actual PostgreSQL dispatch, PUSH and native replay
remain pending. The source evidence is recorded separately in
`docs/reference/content-prep/M25-boss2-sources.md`.

Crystal 201189f `data-global/scripts/quests/adventures_of_galthen/actions_ahau.lua`
configures combat bounds `(33999,31692,10)..(34018,31705,10)`, a STEP trigger at
`(34002,31706,10)`, and return destination `(34036,31717,10)`.
`data/libs/functions/boss_lever.lua:337-338` installs `SimpleTeleport`; its Player
check and destination effect 11 do not require a quest storage or input controller.

The qualified physical footprint is the original rectangle union that one
independently map-proved exit cell. It does not admit the rest of row 31706 or
change the original combat rectangle. World collision, immutable cells, spell
tiles, floor facts and independently allocated Instance facts retain that exact
footprint. The existing shared activity owner performs custody and session return.

The movement owner already retains `CommittedMovementOccurrence`. The shared
observer reads that existing immutable receipt after the caller has recorded its
pace deadline and dropped its movement guards. Normal and buffered STEP,
target-walk, chase and Player PUSH reach the same observer. There is no new
movement queue. The callback checks current physical Player/session/lease,
original after-snapshot, full pins and the selected sealed template. Reconnectable
physical avatars do not gain input authority. Typed native-step return uses the
same durable writer and retained attempt, with migration 2171's dedicated proof;
it neither invents a client CommandId nor weakens migration 1140. A real death
still takes the existing death/temple path. Destination effect failure is cosmetic
and cannot strand committed custody.

## Chase reachability

The Ahau trigger is outside maxY 31705. Under the exact footprint its only possible
cardinal neighbor is `(34002,31705,10)`; the other three are absent, even if the full
World has tiles there. `combat/attack/chase.rs::measured_search` stops once
Chebyshev distance is at most one. `attack/attack_chase_search.rs` searches cardinal
paths and excludes the target's occupied cell. A legitimate target remains inside
the combat rectangle. Whenever the trigger could satisfy the adjacency goal, its
sole entrance already satisfies it. Otherwise entering this dead end cannot
advance a route. A positive Ahau chase-to-exit case would therefore require a
fabricated outside target or a widened footprint. Neither is used. The common
observer still handles the genuine chase path; a Player PUSH south from the sole
entrance can genuinely enter the trigger and needs direct PostgreSQL coverage.

## Evidence obtained so far

The initial targeted World build failed with E0599 for the missing Content exit
footprint API. This is API RED, not an executed runtime-behavior RED. After the
implementation, the exact World test
`map::boot::tests::boss_activity_exit_footprint_keeps_exact_source_cell_without_widening_rectangle`
passed in 0.18 seconds (`/tmp/srx3-m25-world-footprint-green3.log`). It uses a real
booted fixture Bundle and checks the source exit, independent Instance clone,
absent intervening cells, absent/blocked exit refusal and child-widening refusal.

The actual-source PostgreSQL filter is
`m25_ahau_native_step_exit_returns_exact_current_instance_participant_postgres`.
It exports untouched server frames through `SRX3_M25_CAPTURE_PATH`, with actual
decoded domain-1 positions and separate source-coordinate trigger metadata.
Neither its success nor the native-client replay is claimed here.
