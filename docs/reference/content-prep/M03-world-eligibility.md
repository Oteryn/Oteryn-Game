# M03 live World eligibility

The receipt transaction reads permanent promotion through
`character_promotion::read_durable_promotion`, held blessings through
`game_character_blessings`, and Premium through the existing transaction-scoped
`GrizzlyPremiumGate`. The account comes from the locked Character root. No client
flag, quest storage or vocation spelling substitutes for these owners.

“Regular blessings” means the five entries whose `price_family` is `regular` in
`rulesets/character/blessings/catalogue.json` (owner ruling 2026-10-18). Enhanced
Mountain blessings and Twist do not substitute for a missing regular blessing.
The catalogue supplies the canonical storage keys; there is no second key list.

Admission predicates are included in the World operation binding even when the
source omitted `requires_world_admission`. The generic context-free writer is
closed for these callbacks. Historical receipts replay before current eligibility
is checked; a fresh use after Premium expires must be admitted again.

The first production slice adds the fenced capability and PG refusal/acceptance/
replay coverage. Quest entry bindings are a separate following slice; this file
does not qualify either whole quest as playable.
