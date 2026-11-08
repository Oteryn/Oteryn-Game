# M03 — incremental live world-admission predicates

Reuses qmech b1d907db4's `Admission` schema. Structured Premium and the older
`requires_premium` flag now select the same transaction-bound Premium owner.
The weak generic transition API refuses callbacks carrying admission predicates.
Blessing/promotion predicates fail closed until their live-owner hook is added.
No new store or migration is introduced by this slice.

Feaster's first portal retains its existing Premium requirement using the shared
schema; its purification portal retains the historical spelling. This is a
production regression consumer, not completion of either M03 blocked quest.
The source and retained questpc7 witness are documented in QWORLD-M33.md.

The unit test `m03_structured_premium_uses_the_same_live_gate_as_legacy_callbacks`
failed with the old flag-only selection (0.02 s), then passed (0.02 s).
Real-dispatch PostgreSQL tests passed after the change:
- `m33_feaster_initial_phase_uses_first_portal_and_live_requirements`: 50.949 s.
- `m33_feaster_purification_phase_uses_second_portal_and_replays_once`: 50.903 s.
The tests assert the actual production schema used, live Premium refusal,
durable flags, destination and replay. Authoring checks, all 112 Python unit
modules and content-tree validation also passed.

M03 now reuses the complete admission implementation and both initial-entry
bindings merged from qmech at 0326dc2e5. The production reader checks durable
promotion and all seven non-Twist blessings; the account Premium port is read
live. No second admission implementation was added.

The focused `qworld_live_admission_postgres` target strengthens the existing
`qm03_world_admission_live_blessings_promotion_premium_fences_and_replay` case:
promotion and all blessing grants use their purchase owners with SQL guards
enabled, and lost blessings are consumed by the death owner before replay.
Its successful PostgreSQL execution took 42.506019042 seconds. Earlier runs
exposed and corrected the fixture's missing world policy, NPC selection,
Premium purchase port and guarded blessing-consumption setup.

The two `test_qm03_world_admission.py` tests passed in 0.692 seconds and verify
both production entry bindings and their selectors against the retained native
map. This qualifies the admission mechanic and initial entry bindings; it does
not assert completion of either quest's later storyline.

## Production-binding follow-up

The same focused PG case now selects the unmodified
`oteryn:quest-callback/qm03/anniversary` binding and calls
`commit_character_world_callback_with_items`, rather than the fixture-only
callback resolver. Its actual USE cause, level 20 requirement, live admission
predicates and chosen-source storage track remain intact. The complete case
passed in **41.632022735 seconds**, including refusals, guarded promotion and
blessing purchases, death consumption and receipt replay. This is durable-owner
coverage of the production entry binding, not a claim that the later quest
story or map teleport was dispatched by this case.
