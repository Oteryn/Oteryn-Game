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
updated_at: 2026-09-10T14:18:54Z
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
- [x] Define checked exhaustion behavior with no wrap, no partial insertion and no live-actor eviction.
- [x] Preserve `WorldId + ChannelId + ScopeOwnershipGeneration + actor-local identity + actor-local generation` as the minimum exact-reference authority shape.
- [x] Do not choose a production actor ceiling; keep `RUNTIME-ACTOR-RL-01` blocked on ADR-0009/PERF-01.
- [x] Record realistic alternatives, trade-offs, decision timing, supersession evidence, `DECISIONS_NOT_TAKEN` and `CROSS_DOMAIN_FINDINGS`.
- [ ] Exact-head whole-diff self-review and applicable repository CI pass.
- [ ] Genuinely independent exact-head review reports no unresolved material finding before integration.

## Excluded scope

No runtime/server/client code; no `RESOURCE_LIMITS_REGISTRY.json`; no Cargo/workflow/ruleset/protection change; no persistence/schema; no protocol/wire actor handle; no `InstanceRuntime`; no Movement, Ability, AI, Combat or spawn implementation; no geometry/range/LoS/pathfinding/visibility; no production deployment; no external-repository write.

## Implementation / findings

Candidate decision `RUNTIME-ACTOR-LOCAL-GENERATION-V1` is published in PR #541. It selects a finite typed actor-local slot namespace scoped by the already-mandatory `ScopeOwnershipGeneration`. Each configured slot retains its actor-local generation while inactive; reuse requires checked generation advance before publication. A legitimately new scope ownership generation may initialize a fresh local namespace because every older exact actor reference is already rejected by the non-reused outer scope generation. No unbounded tombstone set is required by this shape.

The candidate classifies `RUNTIME-ACTOR-RL-03 = SAME_RESOURCE_AS_RL01_FOR_CHANNEL_RUNTIME_ACTOR_CARRIER_V1` while leaving the numeric RL-01 capacity `M` unresolved for ADR-0009/PERF-01.

## Validation

### Focused

- command/run: GitHub compare of protected base to candidate branch
- result: pre-PR publication compare `ahead_by=2`, `behind_by=0`, exactly two added documentation paths; final-head compare pending after this task metadata update

### Component/integration

- command/run: `NOT_APPLICABLE` — documentation-only architecture candidate
- result: `NOT_APPLICABLE`

### E2E

- scenario: `NOT_APPLICABLE` — no executable runtime path changes
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: pending
- trigger source: pull_request #541
- workflow/run/job: pending
- runner assignment: pending
- classification: pending
- result: pending

## Self-review

- exact head: pending after this final task-metadata mutation
- method/reviewer: authoring supervising architect, whole two-file diff
- material findings: none accepted before final-head re-read; final-head verification pending
- verdict: pending

## Independent review

- required: YES — new actor identity/generation lifetime and resource-exhaustion semantics affect shared runtime authority
- exact head: pending
- method/auditor: genuinely independent exact-head reviewer
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: PR #541 contains exactly the two declared owned paths before final-head re-read
- unresolved review threads: pending
- related/superseded PRs: #537 evidence prerequisite
- protected auto-merge: FORBIDDEN
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: Candidate decision published in draft PR #541; task moved to exact-head validation/review.
status: validating
branch: arch/539-runtime-actor-local-generation
head_sha: null
pr: 541
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request
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
next_action: Re-read the final PR #541 head, exact diff, repository CI and independent-review requirement.
```
