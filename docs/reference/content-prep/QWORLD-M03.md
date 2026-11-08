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

M03 remains PARTIAL: live blessing/promotion enforcement and an entry binding for
25 Years of Tibia Quest or The Pits of Inferno Quest are the next slices.
