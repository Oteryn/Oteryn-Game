# OTV2-20260910-runtime-actor-local-generation-539

```yaml
task_id: OTV2-20260910-runtime-actor-local-generation-539
title: Resolve runtime actor-local identity reuse and generation retention
mode: CONTRACT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: arch/539-runtime-actor-local-generation
pr: 541
base_sha: 2d33d812e578087ac982afc03964fb1917b05b4d
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: OTV2_SOL_SUPERVISING_ARCHITECT
created_at: 2026-09-10T14:07:43Z
updated_at: 2026-09-10
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_RUNTIME_ACTOR_LOCAL_GENERATION_DECISION_2026-09-10.md
  - docs/agents/tasks/active/OTV2-20260910-runtime-actor-local-generation-539.md
public_contracts: []
depends_on:
  - issue: 530
  - pr: 537
  - docs/agents/evidence/OTV2-20260910-runtime-actor-carrier-resource-evidence.md
blocks:
  - issue: 530
  - RUNTIME-ACTOR-RL-03
cross_repository_coordination_id: null
external_repositories: []
merge_authority: ARCHITECTURE_COORDINATOR_ONLY
implementation_authority: NONE
registry_mutation_authority: NONE
production_authority: NONE
```

## Outcome

Produce one bounded architecture resolution for Issue #539 that freezes only the actor-local identity retirement/reuse and generation-retention/exhaustion rule required by `CHANNEL_RUNTIME_ACTOR_CARRIER_V1`. The result must let #530 classify `RUNTIME-ACTOR-RL-03` without selecting a production Channel actor capacity, a generic ECS/container, or executable runtime implementation.

## Architecture and source of truth

- **PROVEN:** PR #537 is protected as `main@2d33d812e578087ac982afc03964fb1917b05b4d` after successful Merge Queue `merge_group` run `34485415384`.
- **PROVEN:** #537 evidence keeps `RUNTIME-ACTOR-RL-01 = PERF_REFERENCE_CELL_REQUIRED` and demonstrates independent retained-history growth when unique opaque local identities are retired.
- **PROVEN:** accepted FND-03 requires current scope ownership generation fencing and fail-closed stale-generation behavior; scope ownership generations are not reusable authority.
- **PROVEN:** #508 requires one production-shaped exact actor reference binding current Channel scope/fence, actor-local semantic identity and actor-local generation; client/protocol/AI fixture scalars cannot become server authority.
- **UNKNOWN:** the future production Channel total-actor maximum; ADR-0009/PERF-01 still owns that measured value.
- **UNKNOWN:** final Rust carrier/container/index representation and allocator details.

## High-risk authority/recovery qualification

`NOT_APPLICABLE` to this task's mutation surface: it writes architecture/task documentation only and performs no runtime, durable, production, protected-environment or authority-bearing state mutation. The architecture itself must nevertheless preserve stale-reference and scope-fence safety and therefore requires independent exact-head review before protected integration.

## Acceptance criteria

- [x] Define when an actor-local identity may be reused and how its generation advances.
- [x] Define the retention lifetime needed to make stale actor references impossible to revive.
- [x] State exactly whether `RUNTIME-ACTOR-RL-03` is the same finite resource as first-carrier slots or requires an independent bounded resource.
- [x] Define checked exhaustion behavior with no wrap, no partial actor/index publication and no live-actor eviction.
- [x] Preserve `WorldId + ChannelId + ScopeOwnershipGeneration + actor-local identity + actor-local generation` as the minimum exact-reference authority shape.
- [x] Require immutable one-to-one `ActorLocalId` -> logical slot/generation-cell binding within one scope ownership generation; prohibit remap/alias resurrection.
- [x] Permit only the finite `VACANT_REUSABLE(g_max) -> EXHAUSTED(g_max)` bookkeeping transition when checked generation successor is unavailable, while publishing no actor/index/current ref.
- [x] Require failed admission to leave every existing actor unchanged, including when the only permitted mutation is terminal exhaustion of the vacant candidate slot.
- [x] Do not choose a production actor ceiling; keep `RUNTIME-ACTOR-RL-01` blocked on ADR-0009/PERF-01.
- [x] Record realistic alternatives, trade-offs, decision timing, supersession evidence, `DECISIONS_NOT_TAKEN` and `CROSS_DOMAIN_FINDINGS`.
- [ ] Exact-head whole-diff self-review and applicable repository CI pass after the material review repair.
- [ ] Fresh genuinely independent exact-head review reports no unresolved material finding after the repair.

## Excluded scope

No runtime/server/client code; no `RESOURCE_LIMITS_REGISTRY.json`; no Cargo/workflow/ruleset/protection change; no persistence/schema; no protocol/wire actor handle; no `InstanceRuntime`; no Movement, Ability, AI, Combat or spawn implementation; no geometry/range/LoS/pathfinding/visibility; no production deployment; no external-repository write.

## Implementation / findings

Candidate decision `RUNTIME-ACTOR-LOCAL-GENERATION-V1` is published in PR #541. It selects a finite typed actor-local slot namespace scoped by the already-mandatory `ScopeOwnershipGeneration`. Each configured logical slot retains its actor-local generation while inactive; reuse requires checked generation advance before publication. Every `ActorLocalId` is immutably bound one-to-one to one logical slot/generation cell for the lifetime of the current scope ownership generation, so a stale escaped ID cannot be remapped onto a different cell carrying an old generation value. A legitimately new scope ownership generation may initialize a fresh local namespace because every older exact actor reference is already rejected by the non-reused outer scope generation. No unbounded tombstone set is required by this shape.

The candidate classifies `RUNTIME-ACTOR-RL-03 = SAME_RESOURCE_AS_RL01_FOR_CHANNEL_RUNTIME_ACTOR_CARRIER_V1` while leaving the numeric RL-01 capacity `M` unresolved for ADR-0009/PERF-01.

### Independent-review findings accepted and repaired

First independent Codex review of exact pre-repair head `4a2cdd6cae3a3457f4ce6feec11254bcc7d4dd01` completed and reported two material findings:

1. **P1 — immutable ID/slot binding missing.** The old wording allowed a future allocator to remap one `ActorLocalId` between physical/logical generation slots; a destination cell with the old generation could resurrect an escaped `(ActorLocalId, generation)` reference. Repair: the architecture now requires an immutable one-to-one `ActorLocalId` -> logical slot -> generation-cell binding for the complete `ScopeOwnershipGeneration`, prohibits alias/remap, and permits physical storage relocation only when the logical binding is preserved.
2. **P2 — exhaustion mutation ambiguity.** The old rollback sentence said every failed successor/admission left all carrier state unchanged, conflicting with the required terminal `EXHAUSTED` slot transition. Repair: ordinary failed admission publishes no actor/index/current-reference state, while checked no-successor at `g_max` atomically records only `VACANT_REUSABLE(g_max) -> EXHAUSTED(g_max)` and then rejects the allocation.

Both findings are accepted. The first independent review is no longer sufficient after these material repairs; a fresh review of the final repaired exact head is required.

## Validation

### Focused

- command/run: GitHub exact changed-file/readback and protected-contract reconciliation
- result: two-path scope preserved; material repair is limited to the architecture decision plus this task record

### Component/integration

- command/run: `NOT_APPLICABLE` — documentation-only architecture candidate
- result: `NOT_APPLICABLE`

### E2E

- scenario: `NOT_APPLICABLE` — no executable runtime path changes
- result: `NOT_APPLICABLE`

### Exact-head CI

- architecture repair head: `5f7d6fa60c838f35fa5874f4a1b6afedb8e6f386`
- repair task-record head: `2cc1a07ab701e860a868be8daf738eb8de9084ad`
- final head: pending this self-review checkpoint commit/readback
- trigger source: pull_request #541
- workflow/run/job: pending new exact-head generation after repair
- runner assignment: pending
- classification: documentation/architecture
- result: pending

## Self-review

- architecture repair head: `5f7d6fa60c838f35fa5874f4a1b6afedb8e6f386`
- repair task-record head: `2cc1a07ab701e860a868be8daf738eb8de9084ad`
- final metadata head: pending this checkpoint commit/readback
- method/reviewer: authoring supervising architect, complete two-file diff
- material findings: the architecture text addresses P1 with immutable one-to-one logical ID/slot binding and P2 with an exhaustion-only terminal slot-state mutation; no remaining P0/P1/P2 author-side finding
- verdict: `PASS_FOR_FRESH_INDEPENDENT_EXACT_HEAD_REVIEW`; this author-role result does not satisfy independent review

## Independent review

- required: YES — new actor identity/generation lifetime and resource-exhaustion semantics affect shared runtime authority
- reviewed pre-repair head: `4a2cdd6cae3a3457f4ce6feec11254bcc7d4dd01`
- pre-repair findings: P1 immutable logical ID/slot binding; P2 exhaustion state-transition ambiguity
- disposition: ACCEPTED_AND_REPAIRED
- final exact head: pending metadata commit/readback
- final verdict: pending fresh independent review

## PR and closeout

- changed-file review: PR #541 remains exactly the two declared owned paths
- unresolved review threads: two accepted pre-repair Codex threads; resolve only after repaired exact-head readback confirms both dispositions
- related/superseded PRs: #537 evidence prerequisite
- protected auto-merge: FORBIDDEN
- merge commit/result: pending control-plane integration after clean exact-head qualification
- ownership release: pending

## Context checkpoint

```yaml
last_progress: Accepted and repaired the independent exact-head P1/P2 findings on the existing PR #541 lineage.
status: validating
branch: arch/539-runtime-actor-local-generation
head_sha: null
pr: 541
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request
ci_check_generation: pending_after_repair
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 1
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: fresh exact-head CI and genuinely independent review after material P1/P2 repair
next_action: Freeze/read back the new PR #541 head, resolve only the two content-addressed old threads, complete exact-head CI, then obtain a fresh genuinely independent review.
```
