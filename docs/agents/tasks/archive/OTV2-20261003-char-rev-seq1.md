# OTV2-20261003-char-rev-seq1

```yaml
task_id: OTV2-20261003-char-rev-seq1
title: "CHAR-REV-SEQ-1: one revision-advancing write in flight per Character"
mode: IMPLEMENTATION
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/char-rev-seq1-20261003
pr: null
base_sha: 828e6afc
head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
final_head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
final_head_frozen_at: null
owner: claude-code-session-019GGMfKu22dMF6cWKQugmfM (oteryn-hard-worker)
control_plane: claude-code-session-013KJX6mv8LQveCKKXYgAX94
created_at: 2026-10-03
updated_at: 2026-10-03
execution_policy: continuous_progress
packet: "docs/architecture/reviews/OTERYN_GAME_ARCH_BATCH_D327_PACKETS_AND_HELD_P1S_2026-10-03.md §1.3 (PR #1655)"
decisions: [D336, D338, D340, D356]
owned_paths:
  - apps/game-server/src/durability/character_revision_sequencer.rs
  - apps/game-server/src/durability/mod.rs
  - apps/game-server/src/durability/character_progression.rs
  - apps/game-server/src/durability/character_death.rs
  - apps/game-server/src/durability/bestiary_progress.rs
  - apps/game-server/src/durability/charm_state.rs
  - apps/game-server/src/durability/item_fee_burn.rs
  - apps/game-server/src/durability/monk_state.rs
  - apps/game-server/src/gameplay_transport/charm.rs
  - apps/game-server/src/gameplay_transport/charm_native.rs
  - apps/game-server/src/gameplay_transport/charm_native_tests.rs
  - apps/game-server/src/gameplay_transport/monk_save.rs
  - docs/agents/tasks/archive/OTV2-20261003-char-rev-seq1.md
  # Extended by the control plane (D336, D338, D340):
  - apps/game-server/src/combat/death_reward.rs
  - apps/game-server/src/gameplay_transport/mod.rs   # only the revision_sequencer field and its init (D340)
  - apps/game-server/tests/support/charm_state_postgres_cases.rs   # only the new bind argument
  - apps/game-server/tests/support/combat_death_reward_postgres_cases.rs   # only the slot argument and one constructor line per case
  - apps/game-server/tests/support/combat_bestiary_postgres_cases.rs   # only the slot argument and one constructor line per case
  # Extended by the control plane (D356): the build and proficiency writers.
  - apps/game-server/src/durability/character_build.rs
  - apps/game-server/src/durability/character_proficiency.rs
  - apps/game-server/tests/support/character_build_postgres_cases.rs
  - apps/game-server/tests/support/character_proficiency_postgres_cases.rs
  - apps/game-server/tests/support/character_revision_sequencer_postgres_cases.rs   # new
  - apps/game-server/tests/durability_postgres.rs   # one mod line
leases: none (runtime cursor; no table, no migration, no capability, command or domain)
public_contracts: []
depends_on:
  - "QUEST-STATE-0 decision §5.2 (accepted)"
blocks:
  - QUEST-STATE-1
  - STANCE-1
  - PREY and Task Board writers
  - BOSSTIARY-1
  - CYC-DISCOVERY-1
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

QUEST-STATE-0 §5.2, "One write in flight per Character", with nothing added.

- `durability/character_revision_sequencer.rs` adds `CharacterRevisionSequencer`, owned by the
  Channel runtime (`ComposedFreshAdmission::revision_sequencer`), and its `RevisionSlot`.
  - The slot is an asynchronous FIFO queue per Character. It is not the runtime lock.
  - The slot holds the revision cursor. The cursor is loaded from the Character root when the
    slot has none. It moves only to a receipt's committed revision, never back. It is dropped
    after an unknown outcome or a mismatch.
  - Idle slots are dropped, so the map holds only Characters with a write queued or in flight.
- Every revision-advancing writer runs through the slot:
  - XP (`commit_experience`);
  - death (`commit_death`);
  - Bestiary (`commit_bestiary`);
  - charm (`commit_charm`), whose in-transaction fee burn is sequenced with it;
  - monk state save (`commit_monk_state_save`);
  - build (`commit_build`) and proficiency (`commit_proficiency`), added by D356. Neither has a
    non-test caller on `main` yet.

  A grep for root revision updates over `src/` finds exactly these seven writers. XP, death,
  charm, build and proficiency also replay a retained receipt exactly, at its original revision: their
  binding includes the revision, so a lost-response retry must not use the advanced cursor.
- Compositions hold the slot for their whole chain. `settle_creature_death_rewards[_with_bestiary]`
  take the principal's held slot, acquired by the caller before it takes the runtime lock. XP
  commits at the cursor, and Bestiary takes the revision XP committed. This replaces
  `bestiary_expected_revision`. The charm port takes the slot before its stage check and holds it
  through the commit.
- Mismatch handling is unchanged from §5.2:
  - Bestiary, whose binding excludes the revision, reloads the cursor and retries once.
  - XP, death, charm, monk, build and proficiency fail closed with no retry. The slot reports the mismatch as a
    defect.
- **Monk save behaviour change (D336, confirmed by the control plane).** On
  `CharacterRevisionMismatch`, `monk_save` used to re-read the fence and retry. It now fails
  closed: `MonkSave::Unknown`, with the defect reported by the slot. The actor-end release loop
  in `gameplay_transport/mod.rs` still re-attempts with backoff, as for any `Unknown`. Each
  attempt is a new occurrence at the reloaded cursor.
- Locks (D324):
  - The runtime lock is never held while waiting for the slot or across durable I/O.
  - The monk save takes the runtime lock only briefly, inside the slot, to read the actor's
    values.
  - `CurrentOwnerCombatDeath` borrows the runtime carrier, so the death composition takes an
    already-acquired slot and never waits for one.
- `gameplay_transport/mod.rs` integration came forward (D340): A2, which owns the path in the
  packet, has not started, so the two lines (the `revision_sequencer` field and its init) land
  here. A2 starts from `main` after this merges.
- The session-generation fence is unchanged. Every write is still refused at the existing gameplay
  fence in its own transaction, and the sequencer adds no second fence.
- A structural test (`no_production_writer_advances_a_revision_outside_the_sequencer`) fails when:
  - any non-test source file calls one of the seven durable writers outside the sequencer; or
  - any file other than `charm_state.rs` calls `burn_fee_in_transaction`.

  A fixture test proves that the gate catches a bypass writer.

## Tests

- Unit tests in `character_revision_sequencer.rs`:
  - concurrent writers commit in sequence with no mismatch, and the unsequenced race fails;
  - a composition holds the slot;
  - the Bestiary retry-once path;
  - a revision-bound mismatch fails closed;
  - cursor monotonicity, replay at the original revision, a foreign fence is refused, idle slots
    are dropped;
  - the structural gate and its bypass fixture.
- PostgreSQL 17.6 tests in `tests/support/character_revision_sequencer_postgres_cases.rs`, run by
  `durability_postgres`:
  - concurrent XP and charm on one Character commit in sequence with no mismatch;
  - a death chain, XP then Bestiary, holds the slot while a waiting monk save runs after it;
  - the Bestiary retry-once path after a bypass writer;
  - XP, death, charm and monk each fail closed after a bypass writer, with no retry.
- PostgreSQL 17.6 cases for build and proficiency, in their own cases files (run by
  `character_authority_postgres`): each commits through the slot from a stale caller fence, fails
  closed with nothing written after a bypass writer, and then commits at the reloaded cursor.

## Validation

Recorded in the FREEZE_SHA report to the control plane and in the PR body.

## Review

Persistence and concurrency review (Codex) on the final frozen head. The control plane requests
it.
