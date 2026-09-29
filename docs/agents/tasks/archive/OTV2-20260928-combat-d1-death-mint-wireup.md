# OTV2-20260928-combat-d1-death-mint-wireup

```yaml
task_id: OTV2-20260928-combat-d1-death-mint-wireup
title: Combat D1, test-only committed creature death -> one ItemMint wire-up
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: d1 (local; the control plane publishes to claude/oteryn-work-coordinator-jv59l0)
issue: 162
pr: 1128
allocation: "#162 comment 5873329796 ('Allocation: Combat D1', owner split exception)"
base_sha: e1ccfd40c08c39e968d7010589e3e31cbb5e3d71
head_sha: ce1d3ab6f6b0021a2c6de2996f43d96e5e84c859
final_head_sha: ce1d3ab6f6b0021a2c6de2996f43d96e5e84c859
final_head_frozen_at: 2026-09-28
owner: "Oteryn: sol combat lead (claude-code-session-01U1WRHgL9X8RbuiG1pwczrF)"
created_at: 2026-09-28
updated_at: 2026-09-28
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/foundation/runtime_actor_carrier.rs        # exclusive
  - apps/game-server/src/foundation/mod.rs                          # exclusive (serialized edits)
  - apps/game-server/src/durability/item_mint.rs                    # constructor only
  - apps/game-server/src/foundation/channel_owner_combat_death_tests.rs
  - apps/game-server/tests/item_mint_postgres.rs                    # allocated; unchanged
  - apps/game-server/tests/support/item_mint_postgres_cases.rs
  - docs/agents/tasks/active/OTV2-20260928-combat-d1-death-mint-wireup.md
  - docs/agents/evidence/OTV2-20260928-combat-d1-death-mint-wireup.md
  - docs/agents/evidence/OTV2-20260928-combat-d1-death-mint-wireup.json
public_contracts:
  - DUR-03
  - VSL-COMBAT-01
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

A committed lethal occurrence of the Channel owner exposes its durable death key, and a
runtime `ItemMintCause` constructor builds the full loot cause from that typed key. A test-only
wire-up proves death -> one fixture loot entry (draw 0) -> `freeze_item_mint` /
`commit_item_mint` under the live fence on real PostgreSQL. D1 is **not** D admission, activates
nothing at runtime and makes no playable-Combat claim.

## Architecture and source of truth

- PROVEN: DUR-03 death-identity decision
  (`docs/architecture/reviews/OTERYN_GAME_DUR03_RESOURCE_MAXIMA_AND_CREATURE_DEATH_IDENTITY_DECISION_2026-09-28.md`)
  §4.1 key = `ActorRef` fields; §4.2 MINT cause = full typed tuple, no hash-only equality, no
  caller bytes; §4.3 / D52 descendants commit only under the live generation.
- PROVEN: VSL-COMBAT-01 §7 (despawn manufactures no death), §8, §9.2-9.3, §13
  (`VSL_COMBAT_FIXTURE_PROFILE`, test/evidence only).
- DERIVED: the existing `CreatureDeathOccurrenceRef` is the ref; its key is the new
  `CreatureDeathOccurrenceKey`. Commit binding, damage and HP stay bound attributes.

## High-risk authority/recovery qualification

```yaml
applicable: true   # a fenced durable MINT is driven from a runtime death
model: AuthorityInvariant_x_ConsumerBoundary_x_MutationOperator
authority_invariants:
  - "a death key exists only from the owner's committed, projected lethal occurrence"
  - "the MINT cause's death component is only that typed key (no caller bytes)"
  - "D52: freeze/commit only while the death generation is the live assignment"
consumer_boundaries: [CreatureDeathOccurrenceRef::death_key, ItemMintCause::from_creature_death]
mutation_operators:
  applicable: [replay, re-freeze, stale generation (scope move), despawn, nonlethal, stale owner]
  considered_not_applicable:
    - "raw key forgery: no constructor exists outside the carrier"
    - "time: no wall time in the key"
one_invariant_per_negative_case: true
independent_current_fact_sources: [carrier continuity guard, PostgreSQL runtime-scope assignment]
record_derived_matching_helper:
  allowed_for_positive_happy_path: true
  forbidden_for_negative_authority_or_provenance_cases: true
finding_family_sweep:
  sibling_apis: "freeze, commit, reconcile exercised"
  protocol_versions: NOT_APPLICABLE
  direct_and_reconciled_paths: "commit + reconcile after re-freeze"
  fenced_durable_writes: "D52 refusal on former and new holder"
  restart_retry_replay_concurrency_pg_reload: "replay + re-freeze; restart/concurrency already in stage C cases"
  evidence: [docs/agents/evidence/OTV2-20260928-combat-d1-death-mint-wireup.md]
finding_dispositions:
  p0_p1_accepted_and_repaired: []
  p0_p1_rejected_with_exact_evidence: []
  p2_fixed_accepted_or_deferred: []
```

## Acceptance criteria

- [x] (a) one creature death produces exactly one MINT (PG).
- [x] (b) replayed / re-frozen death returns the same item and IDs, no duplicate (PG).
- [x] (c) a stale-generation death after the scope moves is refused, nothing minted (PG, D52).
- [x] (d) administrative despawn produces no death and no MINT (PG + unit).
- [x] (e) two fixture creatures have distinct death keys and items (PG + unit).
- [ ] Exact-head CI and required independent review (control plane, after freeze).

## Excluded scope

XP / `character_progression.rs`, `RESOURCE_LIMITS_REGISTRY.json`, migrations,
`gameplay_transport/**`, protocol/client, content, loot tables/probabilities, `lib.rs`, Cargo,
runtime activation. No production caller of the wire-up.

## Implementation / findings

- `CreatureDeathOccurrenceKey` (carrier): private `ActorRef` newtype, typed getters, sole
  constructor `CreatureDeathOccurrenceRef::death_key`.
- `ItemMintCause::from_creature_death(key, TypedDefinitionRef, String, u32)` (`pub(crate)`,
  `allow(dead_code)` until activation). `for_test` kept.
- `CombatDeathFixture` (`cfg(test)`): one-creature carrier; death only via owner commit + Combat
  projection.
- Wire-up (`d1_*` helpers) lives only in `tests/support/item_mint_postgres_cases.rs`.
- Finding: the fixture carrier holds one creature per Channel generation, so (e) uses two
  assigned Channels of one World.

## Validation

See `docs/agents/evidence/OTV2-20260928-combat-d1-death-mint-wireup.md`.

## Self-review

- exact head: local candidate before freeze
- method/reviewer: implementing agent
- material findings: none open
- verdict: READY_FOR_FREEZE

## Independent review

- required: YES (allocation `review_requirement: required`; the control plane triggers it after freeze)
- exact head: pending
- method/auditor: pending
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: 8 owned paths; `item_mint.rs` constructor-only
- unresolved review threads: none; Codex found no major issues on `ce1d3ab`
- related/superseded PRs: none
- protected integration: Merge Queue (auto-merge at the owner's direction)
- merge commit/result: `b58155c` on protected `main`; readback of all owned files byte-identical to `ce1d3ab`
- ownership release: exclusive leases on `foundation/runtime_actor_carrier.rs` and `foundation/mod.rs` released at merge

## Context checkpoint

```yaml
last_progress: PR #1128 merged via Merge Queue as b58155c; protected-main readback matched ce1d3ab; record archived
status: completed
branch: d1
head_sha: ce1d3ab6f6b0021a2c6de2996f43d96e5e84c859
pr: 1128
final_head_sha: ce1d3ab6f6b0021a2c6de2996f43d96e5e84c859
final_head_frozen_at: 2026-09-28
ci_trigger_source: null
ci_check_generation: null
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: completed
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: none for D1; full D admission waits on the VSL §19 rows and the Character progression-readiness proof
```
