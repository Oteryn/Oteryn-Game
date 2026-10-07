# OTV2-20261005-spell-lock-2a

```yaml
task_id: OTV2-20261005-spell-lock-2a
title: SPELL-LOCK-2a spell lane, commit window and complete reservation
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/spell-lock-2a-20261005
pr: 1907
base_sha: aecb02c5
head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
final_head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
final_head_frozen_at: null
owner: oteryn-hard-worker session_01ARgwFxy96wwU3MEiSbVPCd
created_at: 2026-10-06T00:00:00Z
updated_at: 2026-10-06T00:00:00Z
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_ARCH_SPELL_LOCK_2_2026-10-05.md §2.1 owned_paths
  - apps/game-server/src/durability/creature_source_items.rs  # CP amendment D848
  - apps/game-server/tests/support/type2_audit_activation_postgres_cases.rs  # CP amendment D848, lane permit before commit_item_mint only
  - apps/game-server/src/gameplay_transport/spell_access_facts.rs  # CP amendment D849 (owner-approved), pub(crate) async read / sync qualify split only
  - docs/agents/tasks/archive/OTV2-20261005-spell-lock-2a.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Implements `ARCH-SPELL-LOCK-2-V1` §1.1-§1.4 and §1.6 (packet §2.1).

## CP amendments

- D848: `creature_source_items.rs` and the type2 audit activation PostgreSQL case join
  owned_paths, because both reach key 33 (`commit_creature_source_items` through the scope
  assert, the test case through `commit_item_mint`) and the compiler requires them to take a
  permit.

- D849 (owner-approved for this file only): `spell_access_facts.rs` joins owned_paths for the
  minimal split of the owned-fact load into an async database read and a sync qualification, so
  the native post-commit transaction runs without Channel guards; 2b keeps the rest of the file.

## Fresh-lane test wrappers (D870 not needed)

The familiar and wild-spawn PostgreSQL cases outside owned_paths
(`tests/support/character_familiar_writer_postgres_cases.rs`, `gameplay_transport/qualification.rs`)
compile unchanged: `character_familiar.rs` and `qualification_wild_spawn.rs` keep their former
signatures as `#[cfg(test)]` wrappers that acquire a fresh `SpellLane` permit and call the new
`_in_window` / permit-taking production functions. CP widening D870 was therefore not adopted.

## §1.3 mutator disposition

Checks added (`assert_actor_spell_unreserved` / `assert_slot_spell_unreserved`, retryable
`PlanConflict` before any write):
- `commit_actor_condition`, `commit_actor_condition_with_life_result`,
  `commit_player_vitals_with_death_clear`, `commit_actor_condition_batch_with_vitals` (per plan);
- `rebind_player_session`, `commit_reserved_player`;
- `drain_auto_attacks`: a reserved attacker defers its swing (target already checked in
  `prepare_creature_damage_inner_bounded`).

Proven unreachable or already covered: `bind_attacker_lease`, `fence_player_writes`,
`lift_player_fence`, `bind_continuation` (attackers side table only); `mint_transition_fence`
(counter only); `reserve_fresh_session` (free slot); `rollback_reserved_player` (through
`remove()`, which checks); companion reserve/rollback/install, `bind_semantic_creation`,
`install_companion_policies` (`runtime_actor_companion.rs` already asserts; VacantReusable /
CreatureReserved slots or the policy table only); `realize_native_qualification_spawn`
(cfg(test), free slot); heal-and-cure paths through `commit_source_creature_heal_batch` and the
self-heal (already assert); `clear_respawn_player_conditions` (follows the checked position
commit).

## Tests

- `spell_owner_commit.rs` `lane_tests`: drop parks and only the resolution yields a permit;
  explicit park; install/release/uncommitted reclaim consume without parking; foreign-channel
  permit refused; one holder per lane and reload drops the parked attempt.
- `runtime_actor_spell_tests.rs`: a condition commit on a reserved slot refuses until the batch
  ends.
- `tests/spell_lane_key33_pin.rs`: pins the SQL key-33 lock functions and their triggers, every
  production Rust key-33 locker/writer, that each holds a lane-derived proof, and that the
  scope/item authorities are minted only behind a permit.

Known gaps: no PostgreSQL concurrency test of two concurrent spell commits racing the native
post-commit transaction; no end-to-end test of the attack deferral under a pending batch.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: no durable value, wire format, identity or authority changes; the change is an
in-process concurrency invariant of the Channel owner (decision §1.7).

## Fix round 1 (review of c0cb285f, checked against d29a8457)

- 4201415652: the world-item writer now reserves its physical batch in S
  (`reserve_spell_batch`), like the native, parameter and familiar writers.
- 4201415659: the world-item writer matches its verdict refusal after S. A rollback proves
  there was no COMMIT, then the physical reservation and the presentation hold are released and
  the attempt is dropped with `Rejected`. The parked resolver goes through the same path, so a
  rejection is kept instead of being re-parked as unavailable.
- 4201415665: the same for the parameter writer and `resolve_parked_parameter`.
- 4201415655: `commit_spell_owner_transaction` clears `commit_called` on a definite
  `CommitRejected` only, so the attempt is reclaimable. An unknown outcome and every other error
  after the call keep the ambiguity and park.
- Tests: `lane_tests::a_definite_commit_rejection_is_reclaimable_and_an_unknown_outcome_is_not`
  and the source pin `guarded_cast_writers_reserve_in_s_and_release_a_definite_rejection`.

## Fix round 2 (review of 612e1c91)

- 4201790135: `commit_familiar_spell_inner` marks the window already committed as soon as the
  familiar and cost receipts match as history, before training/join validation and the
  read-only COMMIT, so every later failure parks the attempt and keeps the lane blocked. Sibling
  sweep: native, world-item and parameter writers mark history on opening the window; their
  earlier fallible steps keep the attempt in the marker with its reservations, releasing nothing.
- Tests: `lane_tests::a_failure_after_history_is_established_parks_and_keeps_the_lane`
  (semantic and unavailable failure) and the source pin
  `familiar_history_marks_the_window_before_any_fallible_reconciliation`.

## Fix round 3

- 4205657485: a resolution cancelled at an await keeps the lane fenced on the marker
  (`ResolutionFence`).
- 4205974117: the native, world-item and parameter writers open and mark the window as soon as
  the item outcome is `AlreadyCommitted`, before the parameter-result and training follow-ups,
  so their failure parks the attempt; a new write's follow-up failure reclaims it into the
  marker. Familiar was already ordered so (round 2).
- The stacked -kr branch (D905) is merged so `kill_reward.rs` and `item_ref_admission.rs` take
  the permit and #1907 compiles against `main`.
- Tests: `lane_tests` cancellation/fence cases and the source pin
  `guarded_cast_writers_mark_history_before_any_fallible_follow_up`.

## Validation

- `cargo fmt --all -- --check`: pass
- `cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings`: pass
- `cargo test --locked -p oteryn-game-server`: pass
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass

## Closeout

- PR: 1907, no auto-merge; review state: awaiting review on the frozen head.
- merge commit/result: squash merge of #1907
