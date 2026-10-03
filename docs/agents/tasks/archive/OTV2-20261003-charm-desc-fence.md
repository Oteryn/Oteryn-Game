# OTV2-20261003-charm-desc-fence

```yaml
task_id: OTV2-20261003-charm-desc-fence
title: Current attacker lease fence at the Charm damage descendant write
mode: REPAIR
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/charm-native-damage-descendant-20261001
pr: 1625
issue: 1622
base_sha: d7377c1a82e21b73ed223e44078af2dece1328c8
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: CHARM-DESC-FENCE worker, sole writer under control plane session_013KJX6mv8LQveCKKXYgAX94
created_at: 2026-10-03
updated_at: 2026-10-03
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/ability/commit.rs
  - apps/game-server/src/foundation/channel_owner_ability_commit_tests.rs
  - apps/game-server/tests/ability_engine.rs
  - docs/agents/tasks/archive/OTV2-20261003-charm-desc-fence.md
public_contracts: []
depends_on: [OTV2-20261001-charm-native-damage-descendant]
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Repairs Codex P1 thread 4172944113 on PR 1625 (confirmed by the control plane). The
descendant write `commit_exact_owner_charm_damage` took a bare `(attacker, lease_generation)`
pair and only compared it with the frozen primary. A caller that passed the frozen tuple after
the attacker's lease advanced still passed, and the creature's per-attacker high-water mark
(still at the primary's generation) accepted sub-ordinal 1, so a superseded session could change
HP. The write now takes the attacker's current `foundation::CharacterLease` (the existing lease
type from `GameSession::character_lease()` / the current admission authority). A different
character or command is still `InvalidPlan`; a lease that does not accept the frozen generation
is `Owner(CarrierError::SupersededAttackerSession)`, the existing typed carrier error, returned
before the owner write. No new fence infrastructure.

`apps/game-server/tests/ability_engine.rs` compiles `ability/` against a stub `foundation`; its
stub gained `CharacterLease` and the `SupersededAttackerSession` variant so all targets build.

## High-risk authority/recovery qualification

```yaml
applicable: true
model: AuthorityInvariant_x_ConsumerBoundary_x_MutationOperator
authority_invariants:
  - identity/binding: lease character and command equal the frozen primary
  - current liveness/authority: current attacker lease generation equals the frozen generation
consumer_boundaries:
  - commit_exact_owner_charm_damage -> CurrentOwnerExactActorCommit::commit_damage_for_attacker
mutation_operators:
  applicable: [mismatched character, stale/superseded lease generation, mismatched command, owner supersession]
  considered_not_applicable:
    - durable restart/PG reload: ephemeral native receipt carrier
one_invariant_per_negative_case: true
independent_current_fact_sources: [CharacterLease from the current GameSession/admission authority, NamespaceContinuityGuard]
record_derived_matching_helper:
  allowed_for_positive_happy_path: true
  forbidden_for_negative_authority_or_provenance_cases: true
finding_family_sweep:
  sibling_apis: commit_exact_owner_damage/primary take the caller's current lease at the first write; the carrier high-water rejects lower generations; unchanged
  protocol_versions: NOT_APPLICABLE
  direct_and_reconciled_paths: descendant only path that reuses frozen attacker evidence
  fenced_durable_writes: NOT_APPLICABLE
  restart_retry_replay_concurrency_pg_reload: replay under the current lease still returns the original receipt
```

## Validation

RED: with the new guard disabled, the superseded-lease test committed HP 17 to 8. GREEN: new
test `charm_native_superseded_attacker_lease_after_primary_is_refused_without_hp_change`
(superseded lease refused, slots and HP unchanged; then the current lease commits exactly the
descendant) and the updated `charm_native_frozen_proof_never_supplies_current_owner_or_command_authority`
pass. Server library 1287 PASS, 2 existing ignored; `ability_engine` 31 PASS; `cargo fmt
--check` and strict all-target Clippy (`-D warnings`) clean; governance/lifecycle checks run
before freeze. Review: control plane runs the single re-review on the frozen head.
