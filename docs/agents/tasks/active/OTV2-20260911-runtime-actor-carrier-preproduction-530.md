# OTV2-20260911-runtime-actor-carrier-preproduction-530

```yaml
task_id: OTV2-20260911-runtime-actor-carrier-preproduction-530
title: Implement bounded pre-production Channel actor carrier
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/runtime-actor-carrier-preproduction-530
pr: null
base_sha: 1a9cb71f424a821633fd42f8a1a19920ffeff2c3
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: OTV2_IMPL_FOUNDATION_RUNTIME
created_at: 2026-09-11T17:08:57Z
updated_at: 2026-09-11T17:08:57Z
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/foundation/runtime_actor_carrier.rs
  - apps/game-server/src/foundation/mod.rs
  - docs/agents/evidence/OTV2-20260911-runtime-actor-carrier-preproduction.md
  - docs/agents/tasks/active/OTV2-20260911-runtime-actor-carrier-preproduction-530.md
public_contracts: []
depends_on:
  - PR-572 protected allocation/readback
  - Issue-530
  - Issue-540 / PR-570 capacity deferral
  - protected actor-generation decisions including Issue-541
a blocks:
  - future bounded Ability exact-target use under Issue-508
  - future Movement re-evaluation under Issue-139
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Implement one Foundation-owned, fixed-bound, PRE_PRODUCTION Channel actor carrier with direct exact lookup and fail-closed actor/scope generation handling. The implementation exists to make later representative measurement possible; it does not select or expose a production capacity.

## Architecture and source of truth

- PROVEN: protected activation base is `main@1a9cb71f424a821633fd42f8a1a19920ffeff2c3`, the native Merge Queue integration of allocation PR #572.
- PROVEN: protected allocation `docs/agents/programs/OTV2_RUNTIME_ACTOR_CARRIER_PREPRODUCTION_ALLOCATION_20260911.md` is the exact write authority.
- PROVEN: #570 defers production actor capacity/VPS selection until representative measurement; `RUNTIME-ACTOR-RL-01` remains evidence-gated.
- PROVEN: `ScopeRuntimeFence` consumes supplied ownership facts and is not durable scope-assignment producer authority.
- PROVEN: #335 is READ-ONLY SERIAL HOLD; #356 is the only other active mutating lead at activation.
- PROVEN: #335/#356 do not own `apps/game-server/src/foundation/mod.rs` or the new carrier path.
- UNKNOWN until implementation proof: exact internal Rust carrier layout satisfying all allocation invariants without widening authority.

## High-risk authority/recovery qualification

`NOT_APPLICABLE` to production mutation/recovery qualification: this task has no production authority, durable assignment-producer authority, session/lease mutation, persistence/recovery mutation, or deployment authority. Multichannel fencing semantics are nevertheless high risk and require a genuinely independent exact-head review before integration.

## Acceptance criteria

- [ ] Add exactly one bounded Channel carrier and direct exact slot/generation lookup; no second growing index/history.
- [ ] Every construction requires explicit finite non-production capacity; zero/missing/overflow/allocation failure fails closed before partial publication.
- [ ] M succeeds when resources allow; M+1 rejects without retained-state mutation across multiple fixture bounds, without promoting a fixture value to product policy.
- [ ] No API exports/serializes/reports a development bound as production capacity and no production-labeled constructor/default exists.
- [ ] Exact reference binds WorldId + ChannelId + ScopeOwnershipGeneration + actor-local identity + actor-local generation.
- [ ] Wrong world/channel, stale scope generation, vacant/out-of-range identity, stale actor generation and cross-scope cases reject deterministically.
- [ ] Same-generation carrier reconstruction from raw scope/generation facts is impossible; pre-production continuity authority is move-only/non-replayable and does not mint grants.
- [ ] Removal/reuse advances retained actor-local generation; wrap is forbidden.
- [ ] Checked no-successor at `g_max` performs only `VACANT_REUSABLE(g_max) -> EXHAUSTED(g_max)`, publishes no replacement, preserves unrelated state and never reselects that slot in the same scope generation.
- [ ] `ScopeRuntimeFence` is not made Clone/Copy and its private raw-grant constructor is not widened.
- [ ] `foundation/mod.rs` receives only minimum private/crate-visible wiring.
- [ ] No Cargo/workspace/lib.rs/protocol/admission/Durability/transport/registry/workflow/production/#508/#139 mutation.
- [ ] Focused RED->GREEN, fmt, focused server tests, strict Clippy and adversarial whole-diff self-review pass on one exact head.
- [ ] Exact-head repository CI passes.
- [ ] Genuinely independent exact-head review is clean; unresolved material threads = 0.

## Excluded scope

No production capacity/default/readiness, `RESOURCE_LIMITS_REGISTRY`, VPS/deployment, durable scope-assignment producer/consumer composition, Ability #508, Movement #139, geometry/range/LoS/pathfinding/retargeting, gameplay formulas/state, persistence/schema/migrations, transport/TLS/listeners, public wire IDs, Cargo/workspace, external repositories or protected environment mutation.

## Implementation / findings

Coordinator bootstrap created this task record after explicit owner execution authorization and fresh protected readback. Implementation has not yet been published. The canonical worker must reuse this branch/task/PR lineage only.

## Validation

### Focused

- command/run: pending
- result: pending

### Component/integration

- command/run: pending
- result: pending

### E2E

- scenario: `NOT_APPLICABLE` for physical production-capacity qualification; this is deliberately a pre-production bounded component.
- result: pending focused runtime evidence only

### Exact-head CI

- final head: pending
- trigger source: pending
- workflow/run/job: pending
- runner assignment: pending
- classification: pending
- result: pending

## Self-review

- exact head: pending
- method/reviewer: implementing/coordinating agent
- material findings: pending
- verdict: pending

## Independent review

- required: YES — multichannel runtime/fencing semantics are high risk
- exact head: pending
- method/auditor: pending
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: #568 prototype, #570 architecture, #572 allocation
- protected auto-merge: forbidden substitute; native Merge Queue only
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: coordinator task bootstrap after protected allocation activation
status: implementing
branch: agent/runtime-actor-carrier-preproduction-530
head_sha: null
pr: null
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: null
ci_check_generation: null
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: implement focused RED->GREEN carrier module and minimal foundation wiring on this branch
```
