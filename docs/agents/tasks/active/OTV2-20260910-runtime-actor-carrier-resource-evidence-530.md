# OTV2-20260910-runtime-actor-carrier-resource-evidence-530

```yaml
task_id: OTV2-20260910-runtime-actor-carrier-resource-evidence-530
title: Measure first Channel actor-carrier resource dimensions
mode: AUDIT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/runtime-actor-resource-evidence-530
issue: 530
pr: null
base_sha: 951f98746e2fb8b93be0a4f04518b74e266df188
admission_main_sha: 951f98746e2fb8b93be0a4f04518b74e266df188
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: activated evidence-only worker under #162
created_at: 2026-09-10
updated_at: 2026-09-10
execution_policy: continuous_progress
owned_paths:
  - tools/runtime-actor-resource-evidence/**
  - docs/agents/evidence/OTV2-20260910-runtime-actor-carrier-resource-evidence.json
  - docs/agents/evidence/OTV2-20260910-runtime-actor-carrier-resource-evidence.md
  - docs/agents/tasks/active/OTV2-20260910-runtime-actor-carrier-resource-evidence-530.md
public_contracts: []
depends_on:
  - issue_530
  - issue_508_accepted_identity_semantics
  - protected_allocation_951f98746e2fb8b93be0a4f04518b74e266df188
blocks:
  - shared_channel_actor_carrier_resource_disposition
  - ability_exact_actor_resolution_v1
  - reference_local_step_static_kernel_v1
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Produce reproducible, public-safe, **non-production** resource evidence for `CHANNEL_RUNTIME_ACTOR_CARRIER_V1` without choosing a production actor store, ECS/container, actor identity allocator, runtime module, resource-registry maximum or production capacity.

The evidence compares bounded synthetic physical shapes, validates exact actor-reference negative cases, and classifies each `RUNTIME-ACTOR-RL-*` row separately.

## Architecture and source of truth

`PROVEN`: protected admission is `main@951f98746e2fb8b93be0a4f04518b74e266df188`; real #532 Merge Queue group run `34478991801` completed successfully; repaired protected allocation blob is `0b81a498f1553e61fca89eebd85a23df080e428b`; #162 activation is comment `5619094154`.

`PROVEN`: `WorldId` and `ChannelId` are distinct; current `ScopeOwnershipGeneration` and actor-local generation are required to reject stale authority. #508 already closed the semantic ingredient question and still lacks a physical production shared runtime carrier/current-owner resolver.

`PROVEN`: ADR-0009 does not accept fixed production `max_players_per_channel`, GameNode or world values. Production capacity requires representative `PERF-01` evidence on named hardware/artifacts and multiple realistic workload families.

`DERIVED`: direct exact-target lookup and the first local-step need only tiny functional lower bounds (2 actors and 1 actor respectively), but those correctness lower bounds are not production capacity.

`UNKNOWN`: final production actor container/index representation, actor-local identity retirement/reuse policy, generation-retention/exhaustion representation and total actor capacity remain unselected.

`CONFLICT`: none after keeping candidate measurements distinct from production limits.

## High-risk authority/recovery qualification

```yaml
applicable: false
reason: evidence-only synthetic tooling and documentation; no production mutation, authority acquisition, persistence, registry mutation, runtime implementation, public contract or recovery controller is changed
```

The harness still tests stale/mismatched scope and actor generations because those are the subject of the evidence, but it does not become an authority-bearing runtime consumer.

## Acceptance criteria

- [x] Every modeled positive reference contains `WorldId + ChannelId`, current scope ownership generation and actor-local generation.
- [x] Same ChannelId under another WorldId rejects.
- [x] Stale/mismatched `ScopeOwnershipGeneration` rejects and leaves carrier state unchanged.
- [x] Missing/stale/recycled/cross-Channel references reject.
- [x] M+1 active admission rejects before partial state for every tested synthetic ceiling.
- [x] Checked count/byte arithmetic rejects overflow.
- [x] Mixed player/creature/NPC-system population is exercised.
- [x] Exact lookup is deterministic and does not materialize variable target/geometry collections.
- [x] Fixed-shape candidate retains no variable AI/combat/inventory/loot/dialogue payload.
- [x] Repeated same-identity churn proves fixed retained storage where appropriate.
- [x] Unique opaque-identity churn determines whether retained generation/history can grow independently of active count.
- [x] Every RL row receives one explicit classification.
- [x] No production maximum is selected.
- [x] Machine-readable JSON is deterministically regenerable.
- [ ] Exact-head repository CI and whole-diff review on the published worker candidate.

## Excluded scope

No writes to `apps/**`, existing runtime/tests, Cargo/lockfiles, `docs/contracts/RESOURCE_LIMITS_REGISTRY.json`, architecture contracts, workflows, Platform, Atlas, META or external repositories. No Movement/#139 or Ability/#508 implementation. No InstanceRuntime, geometry/range/LoS, pathfinding, visibility, AI behavior, combat, persistence or production changes.

No tested synthetic value, including 256, may be called a production maximum.

## Implementation / findings

The worker implements a standalone stdlib-only Python model and deterministic generator under its allocated tool directory plus two public-safe evidence files.

Two synthetic candidate shapes are compared:

- fused fixed bucket: modeled actor state, lookup location and generation/retirement state share one fixed table;
- split record + fixed index: actor records and lookup index are physically separate, while generation history remains separately accountable if retained independently.

Logical fixed-width accounting is explicit and intentionally not Python RSS/Rust ABI evidence:

```text
ActorRef: 40 bytes
fused bucket: 64 bytes
split actor record: 56 bytes
split index entry: 24 bytes
split generation-history entry: 16 bytes
```

Synthetic tested active ceilings: `2, 3, 8, 64, 256`.

Measured retained logical bytes:

```text
M=2:   fused=256,    split record+index=208
M=3:   fused=512,    split record+index=360
M=8:   fused=1024,   split record+index=832
M=64:  fused=8192,   split record+index=6656
M=256: fused=32768,  split record+index=26624
```

Observed exact-lookup maximum probes in the deterministic fused fixture were respectively `1, 1, 2, 7, 10`, with structural upper bounds equal to fixed bucket counts `4, 8, 16, 128, 512`. No variable candidate collection is created.

### Resource dispositions

`RUNTIME-ACTOR-RL-01 = PERF_REFERENCE_CELL_REQUIRED`

Candidate bytes/work/M+1 behavior are measurable, but a production total-actor ceiling is not authorized by synthetic data. ADR-0009/PERF-01 remains the required capacity evidence source.

`RUNTIME-ACTOR-RL-02 = MEASURED_CANDIDATE_EVIDENCE_AVAILABLE`

Fused storage can make lookup/index state physically inseparable from the actor table; split storage retains an independently accountable fixed index. Therefore a universal same-resource claim cannot be made before physical carrier selection.

`RUNTIME-ACTOR-RL-03 = ARCHITECTURE_ESCALATION_REQUIRED`

This is the material finding. Reusing one opaque local identity 1,000 times advances generation in-place with fixed retained storage. Retiring unique opaque identities, however, can consume retained generation/history slots while active count returns to zero after every removal. For candidate active ceilings `1,2,3,8`, history exhaustion occurs after `2,4,8,16` unique retirements respectively in the deterministic fused table, with `active_count=0` at exhaustion.

Protected semantics require stale-reference rejection but do not yet select the actor-local identity retirement/reuse and generation-retention/exhaustion rule needed to prove a finite same-resource or separate bound. A vector index/pointer/client/AI handle cannot be substituted as semantic authority to evade this question.

`RUNTIME-ACTOR-RL-04 = NOT_EXERCISED_BY_FIRST_CARRIER`

Exact lookup uses fixed-table probing only; no variable candidate materialization or geometry/spatial scan is modeled.

`RUNTIME-ACTOR-RL-05 = NOT_EXERCISED_BY_FIRST_CARRIER`

The candidate actor record is fixed-shape; variable domain payloads remain outside the carrier.

## Validation

### Focused

- command/run: `python tools/runtime-actor-resource-evidence/self_test.py`
- result: `PASS 14 tests`
- command/run: `python tools/runtime-actor-resource-evidence/generate.py --check docs/agents/evidence/OTV2-20260910-runtime-actor-carrier-resource-evidence.json`
- result: `PASS evidence JSON matches deterministic generator`

### Component/integration

- command/run: standalone deterministic structural model only
- result: PASS; no runtime component was compiled or modified by this worker

### E2E

- scenario: `NOT_APPLICABLE` — no executable product/runtime/persistence mutation
- result: no production behavior claim

### Exact-head CI

- final head: recorded in immutable PR/check evidence after publication; not self-referentially embedded in this commit
- trigger source: ordinary worker PR
- workflow/run/job: pending publication
- runner assignment: pending publication
- classification: exact changed paths only
- result: pending publication

## Self-review

- exact head: recorded after the single worker commit exists
- method/reviewer: worker whole-diff adversarial review
- material findings: RL-03 independent-growth result is an evidence finding, not a defect in the candidate; no known out-of-scope write or production-authority leak in the prepared bytes
- verdict: prepared for exact-head repository qualification

## Independent review

- required: NO at worker freeze under current risk policy; this candidate is evidence-only tooling/documentation and grants no authority, runtime, registry, public-contract, persistence, production or merge mutation
- exact head: `NOT_APPLICABLE`
- method/auditor: `NOT_APPLICABLE`
- material findings: `NOT_APPLICABLE`
- verdict: repository-native semantic/governance/CI plus mandatory whole-diff self-review remain required; any automatically supplied external review remains advisory evidence

## PR and closeout

- changed-file review: pending publication; must remain exactly the seven worker-owned files
- unresolved review threads: pending publication
- related/superseded PRs: #531 allocation, #532 allocation repair
- protected auto-merge: not authorized
- merge commit/result: pending
- ownership release: pending protected integration/readback or explicit blocked terminal handoff

## Context checkpoint

```yaml
last_progress: deterministic resource evidence and 14-case negative harness prepared
status: validating
branch: agent/runtime-actor-resource-evidence-530
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
blocker: RUNTIME-ACTOR-RL-03 architecture disposition plus later ADR-0009/PERF-01 capacity evidence
next_action: publish one exact worker commit/PR and qualify it with repository-native checks; do not activate #139 or #508
```
