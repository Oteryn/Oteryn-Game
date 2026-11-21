# Shared boss phase owner

`NativePhaseOwner` extends the existing native boss encounter mechanic. It binds
one scope generation, the complete content pin, exact actor incarnations and the
admitted player sessions. Source adapters supply a bounded dependency plan and
use the existing map, spawn and settlement owners.

`cults::CultsEncounter` supplies the five independent pillar/guardian branches,
stop-form replacement and the final Essence join. Native deaths are proved from
sealed corpse projections. Name matches, zero health, duplicate deaths and
unrelated actors cannot advance the join. A failed physical replacement restores
the carrier and remains retryable.

The production World callback `engenc2-cults.json` selects this owner during
atomic party admission. `ChannelSpellStates` owns the bounded phase registry;
the native death queue observes sealed deaths before loot admission and retries
failed physical replacements on the existing monster cadence.

Terminal settlement uses the existing fenced native quest writer. Only admitted,
current-lease players with actual damage contribution receive Humans.Mission=2.
The queue preserves their frozen damage evidence even when the corpse cannot be
materialized. Native and PostgreSQL tests cover the phase join, source callback,
participant filtering and once-only durable settlement. This proves the shared
phase closure, not the remaining whole-quest route or reward qualification.

Source: zimbadev/crystalserver at
`9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d`,
`data-global/scripts/quests/cults_of_tibia/creaturescripts_destroyed_pillar.lua`
and `creaturescripts_essence_of_malice.lua`.

Court Warlock uses the same `NativePhaseOwner` and dispatch phase registry.
Its production USE callback selects the five source staging tiles; the existing
Character boss cooldown table admits the whole qualified party for 20 hours.
The adapter waits three seconds, then preserves the donor's whole-second clock:
seven Raubritters at each 45-second phase boundary and each intervening multiple
of `floor(45/2)`, followed by 22 Falcons at second 318 after admission. The
Warlock joins only once the room is clear, with health reduced by the fraction
of Raubritters killed. Its native death reaches the existing room-player quest
writer (Questline 11 -> 12); the shared tick transports survivors to the source
exit after the 60-second grace period, or at the 30-minute fight deadline.

Source: Crystal `201189f871d23597b82caf88fd611454a2ddefb8`,
`data-global/scripts/quests/the_order_of_the_stag/court_warlock_fight.lua`
(lines 22-40, 134-147, 204-278, 323-355, 361-414), read in full.
The donor explicitly labels phase duration, cooldown, deadline and health
scaling as unconfirmed. CWARLOCK retains those values, rather than claiming
independent Global verification. PG scenarios are mounted in dispatch's
`postgres_tests/cwarlock.rs` and enter through production lever USE and tick.

Focused CWARLOCK verification uses
`cargo test --locked -p oteryn-gameplay-dispatch --lib cwarlock_ -- --nocapture --test-threads=1`.
Supply `OTERYN_TEST_POSTGRES_ADMIN_URL`: each of the five PostgreSQL cases must
print `CWARLOCK_PG_EXECUTED` with a nontrivial duration. Without that variable,
those cases intentionally return successfully for route-less lanes and provide
no database evidence. The additional literal wave-clock assertion runs in
both lanes. `cargo test --locked -p oteryn-gameplay-quests --lib cults` also
checks that registering Court Warlock preserves the established phase schema.

CWARLOCK local content validation (2026-11-17):

| Check | Result |
| --- | --- |
| `validate_full_game_content_tree.py` | PASS: 19 domains, 39 systems, 100 nodes, none unassigned |
| `validate_materialized_game_tree.py` | PASS: 97/97 materialized files, 10 World markers, 1,261 successor files |
| `quest-authoring/run_checks.py --part checks` | PASS: complete offline checks runner |
| `quest-authoring/run_checks.py --part unit-tests --jobs 2` | PASS: 182 modules, 1,464 tests; 8 existing skips |
