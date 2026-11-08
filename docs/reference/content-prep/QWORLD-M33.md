# M33 — Feaster STEP phase selection

Reused the two Feaster of Souls STEP callback records from questpc7 commit
`2caf66704f735bde02cb68370a5b5e00dd2d87a6`, rather than recreating its quest.
The retained authoring witness is recorded in each callback's sources.

Sources retained from that checked branch:
- https://www.tibiawiki.com.br/index.php?title=Feaster_of_Souls_Quest&oldid=421868
- https://www.tibiawiki.com.br/index.php?title=Para_Norbert&oldid=419674

Both records use the same entrance tile and appearance. Programme phase 0 leads
to the initial destination, and phase 5 leads to purification. Level 250 and an
active premium account remain required. Production callback selection uses the
personal quest copy, with all guards checked again by the fenced durable writer.
Overlapping phase ranges remain invalid registry entries. A pending replay keeps
its originally selected callback.

The two `qworld_phase_steps` PG cases execute real STEP dispatch against the
production callback registry and premium latch. Their fenced setup transitions
arrange programme phases; they do not claim to qualify the preceding NPC story
or the whole quest. They check phase refusal, destinations, level/premium refusal,
flag writes and command replay. No new database migration is needed.

## Local execution proof

After merging qmech `4bed43770`, both real-dispatch PostgreSQL cases passed:
- `m33_feaster_initial_phase_uses_first_portal_and_live_requirements`: 53.679 s.
- `m33_feaster_purification_phase_uses_second_portal_and_replays_once`: 53.244 s.

The focused world-callback unit set passed 36 tests (8.54 s). Before the engine
changes, the disjoint-selector test failed with duplicate-selector rejection and
the authored programme-domain test failed because phase 5 exceeded the derived
track bound. Both are now green. The first PG run also exposed the fixture's
wall-at-destination error; only fixture geometry was corrected for that failure.
