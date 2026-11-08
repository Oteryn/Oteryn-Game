# M03 live World eligibility

The receipt transaction reads permanent promotion through
`character_promotion::read_durable_promotion`, held blessings through
`game_character_blessings`, and Premium through the existing transaction-scoped
`GrizzlyPremiumGate`. The account comes from the locked Character root. No client
flag, quest storage or vocation spelling substitutes for these owners.

“Regular blessings” follows the death owner: seven blessings, including both
Mountain blessings, excluding Twist (`domain/death.rs::MAX_REGULAR_BLESSINGS`).
The five `price_family=regular` entries are a pricing/Pilgrimage group, not the
complete World-admission set. The QM03 collector implementation established this
same seven-blessing contract. The canonical catalogue supplies all keys.

Admission predicates are included in the World operation binding even when the
source omitted `requires_world_admission`. The generic context-free writer is
closed for these callbacks. Historical receipts replay before current eligibility
is checked; a fresh use after Premium expires must be admitted again.

The shared QM03 writer is reused; redundant questpc2 wrappers and checks are removed.
The first production slice adds the fenced capability and PG refusal/acceptance/
replay coverage. Quest entry bindings are a separate following slice; this file
does not qualify either whole quest as playable.
