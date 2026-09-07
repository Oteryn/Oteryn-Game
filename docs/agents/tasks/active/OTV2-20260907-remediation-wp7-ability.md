# OTV2-20260907-remediation-wp7-ability

```yaml
task_id: OTV2-20260907-remediation-wp7-ability
title: Repair Ability effect magnitude boundary validation
mode: REPAIR
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: fix/remediation-wp7-ability-364
issue: 364
pr: null
admission_main_sha: b3e637dc43a0a31ff2caf24a6450f7df56b43777
base_sha: b3e637dc43a0a31ff2caf24a6450f7df56b43777
head_sha: null
final_head_sha: null
owner: WP7_ABILITY_REPAIR
created_at: 2026-09-07
updated_at: 2026-09-07
owned_paths:
  - apps/game-server/src/ability/effects.rs
  - apps/game-server/src/ability/plan.rs
  - apps/game-server/src/ability/commit.rs
  - apps/game-server/tests/ability_engine.rs
  - docs/agents/tasks/active/OTV2-20260907-remediation-wp7-ability.md
public_contracts: []
depends_on: []
blocks: [WP7_Ability_correctness]
external_repositories: []
```

## Outcome

Prevent directly constructed invalid Damage/Heal magnitudes from crossing public plan/commit boundaries or producing atomic/sequential partial mutation. Preserve valid deterministic ordering, helper behavior and replay semantics. Production reachability and G1 remain separate WP8 evidence.

## Architecture and source of truth

PROVEN on admission main: public `Effect::Damage` / `Effect::Heal` variants accept arbitrary `i64`; helper constructors reject `<= 0`, but `EffectPlan::new` validates target/capacity without magnitude validation and `AbilityEngine::commit`/`apply_fixture_effect` apply the direct value. `Damage { magnitude: -7 }` therefore reaches `checked_sub(-7)` and increases fixture health. Exact allocation is #162 comment `5567031784`; fresh open-PR search found no overlapping Ability runtime mutation lineage.

## High-risk authority/recovery qualification

NOT_APPLICABLE: no session authority, PREPARE/COMMIT, durable schema/recovery, production/live state or protected control-plane mutation.

## Acceptance criteria

- [ ] Direct zero/negative Damage and Heal variants are rejected at the public plan boundary before a plan is accepted.
- [ ] Commit/apply keeps a defense-in-depth fail-closed check before any fixture mutation, including ordered sequential paths.
- [ ] Atomic invalid plan/effect causes no partial health mutation; sequential invalid effect causes no newly applied invalid mutation and cannot bypass replay bookkeeping.
- [ ] Valid positive Damage/Heal retain existing signed health semantics, ordering and replay behavior.
- [ ] Focused integration tests, Rust 1.94 fmt, strict game-server Clippy/tests and applicable exact-head CI pass.

## Excluded scope

No broad Effect API redesign, gameplay activation/registration, AI, Foundation/Durability, Cargo/lock, lib/composition, workflow/protection, production/live data or external-repository changes.

## Validation

Focused/component/exact-head evidence: pending worker implementation. No historical green check validates future bytes.

## Context checkpoint

```yaml
last_progress: exact five-path WP7B allocation created from protected main
status: implementing
branch: fix/remediation-wp7-ability-364
head_sha: null
pr: null
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
owner_action_required: null
blocker: null
next_action: implement magnitude validation at plan and pre-mutation commit boundaries with direct-construction regressions
```
