# Architect batch D327: amendment for D340 (CHAR-REV-SEQ-1 before A2)

```yaml
decision_id: ARCH-BATCH-D327-PACKETS-V1-AMENDMENT-D340
status: CANDIDATE
date: 2026-10-03
amends: OTERYN_GAME_ARCH_BATCH_D327_PACKETS_AND_HELD_P1S_2026-10-03.md (#1655), §0 and §1.3
answers: control-plane decision D340
```

D340 reverses the order that §0 set for CHAR-REV-SEQ-1 and A2. A2 has not started, so the CHAR-REV-SEQ-1 integration phase moves forward.

1. **Order.**
   - CHAR-REV-SEQ-1 adds the `revision_sequencer` field to `ComposedFreshAdmission` and its initialisation in `gameplay_transport/mod.rs` in its own authoring. It does not wait for A2. The §1.3 integration phase therefore no longer waits.
   - A2 starts from `main` only after CHAR-REV-SEQ-1 has merged, which adds a dependency to A2's packet. In addition, #1652, #1651 and ACH-NOTIFY-2 must have merged, as §0 already requires.
   - `foundation/runtime_actor_carrier.rs` stays exclusive to A2. CHAR-REV-SEQ-1 does not touch it.
2. **Paths added to CHAR-REV-SEQ-1** (granted by D340):
   - `apps/game-server/tests/support/combat_death_reward_postgres_cases.rs`
   - `apps/game-server/tests/support/combat_bestiary_postgres_cases.rs`

   Both changes are limited to building `DurabilitySession` with the sequencer.
3. **Serialisation on `gameplay_transport/mod.rs`.**
   - ACH-NOTIFY-2 and CHAR-REV-SEQ-1 both edit the file. Whichever merges second merges `main` before it freezes, and it re-runs its validation on the merged head.
   - The CHAR-REV-SEQ-1 edit is limited to the field and its initialisation, about two lines.
4. Nothing else in #1655 changes. There are no leases.
