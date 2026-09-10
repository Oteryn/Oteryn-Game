# Runtime actor carrier resource-evidence allocation

- Coordinator: #162
- Resource gate: #530
- Related: #139, #486, #508
- Prepared against protected `main@058cb8764e6d0323df12a79e6281fcdf5230b620`

## Status and authority

```yaml
allocation_id: OTV2-RUNTIME-ACTOR-CARRIER-RESOURCE-EVIDENCE-530
repository: Oteryn/Oteryn-Game
coordinator_issue: 162
resource_issue: 530
reference_programme: 486
ability_successor_issue: 508
movement_resource_issue: 139
allocation_state: NOT_ACTIVE
worker_launch: NOT_STARTED
allocation_branch: coord/runtime-actor-resource-evidence-530
branch_after_activation: agent/runtime-actor-resource-evidence-530
execution_target: isolated_workspace_or_repository_native_ci
validation_target: github_actions
lane_strategy: single_agent
runtime_implementation_authority: NONE
resource_registry_mutation_authority: NONE
production_authority: NONE
external_repository_write_authority: NONE
```

This is a **prospective evidence-only allocation**. It becomes usable only after this exact allocation is protected-integrated, protected-main readback confirms it, and the unique #162 Work Delivery Coordinator performs a fresh ownership/custody check and explicitly activates the same worker lineage.

Publication, a green PR, Issue #530, tool availability, or an alias invocation does not activate the worker. This allocation does not itself select a resource maximum, modify runtime code, activate Movement, implement #508, or grant Merge Queue authority.

## Proven admission facts

At preparation time:

- protected `main` is `058cb8764e6d0323df12a79e6281fcdf5230b620`;
- #508 has already closed the semantic identity/generation ingredients required by the first exact actor reference;
- Foundation physically contains `ScopeOwnershipGeneration`, `RuntimeExecutionOrdinal`, `RuntimeWorkStamp` and `ScopeRuntimeFence` fencing primitives;
- protected source still has no concrete shared Rust `ChannelRuntime` actor store/current-owner actor lookup;
- the prospective WP5 Game scope-assignment allocation remains `NOT_ACTIVE`, is Channel-only for its first slice, and supplies future scope ownership/fence authority rather than actor storage;
- fresh PR readback shows historical WP2 PR #361 is merged/closed, so its former Foundation paths are not an active writer collision;
- fresh open-PR search found no current Ability actor-resolution implementation writer;
- `AI01-ACTIVE-ACTORS=256` is owned by the AI bootstrap and is not authority for total player/NPC/creature Channel actor-store population;
- ADR-0009 explicitly defers `max_players_per_channel`, `max_players_per_game_node`, and `max_players_per_world` to representative `PERF-01` evidence rather than fixed architectural guesses.

Changing any of these material facts requires fresh reconciliation before activation. Historical descriptions are evidence only.

## Worker outcome

Produce a reproducible, non-production evidence package for `CHANNEL_RUNTIME_ACTOR_CARRIER_V1` that narrows #530's resource inventory far enough for the owner/control plane to make a truthful next decision.

The worker must answer, independently for each `RUNTIME-ACTOR-RL-*` row:

```text
SAME_RESOURCE_AS_ANOTHER_ROW
NOT_EXERCISED_BY_FIRST_CARRIER
MEASURED_CANDIDATE_EVIDENCE_AVAILABLE
PERF_REFERENCE_CELL_REQUIRED
ARCHITECTURE_ESCALATION_REQUIRED
```

The worker must **not** convert candidate measurements into an accepted hard maximum. If no accepted representative player/creature/NPC workload and named capacity cell exists, `RUNTIME-ACTOR-RL-01` remains `PERF_REFERENCE_CELL_REQUIRED` rather than receiving an arbitrary value.

## Exact owned paths after activation

Only the following paths become writable after protected allocation integration plus explicit #162 activation:

```text
tools/runtime-actor-resource-evidence/**
docs/agents/evidence/OTV2-20260910-runtime-actor-carrier-resource-evidence.json
docs/agents/evidence/OTV2-20260910-runtime-actor-carrier-resource-evidence.md
docs/agents/tasks/active/OTV2-20260910-runtime-actor-carrier-resource-evidence-530.md
```

Everything else is read-only.

In particular, this allocation grants **no write authority** to:

```text
apps/game-server/src/**
apps/game-server/tests/**
docs/contracts/RESOURCE_LIMITS_REGISTRY.json
docs/architecture/**
docs/agents/programs/** except this already-protected allocation itself
Cargo.toml
Cargo.lock
apps/game-server/Cargo.toml
.github/**
vendor/**
Platform / Atlas / META / external repositories
```

No existing production module may be edited merely to make the evidence harness convenient.

## Candidate carrier semantics the evidence may model

The evidence harness may model only the already accepted minimum semantics required by #508/#530. A synthetic carrier representation is an **evidence model**, not a permanent runtime API or storage decision.

At minimum one candidate record must be able to represent:

```text
Channel scope identity
current scope ownership generation/fence binding
actor semantic/local identity
actor local generation inseparable from the actor reference
current existence/actionable lifecycle fact
current authoritative local position only where needed by the first exact-target/local-step proofs
minimal actor-kind discriminator only if required to exercise mixed actor occupancy
```

The harness may compare multiple bounded physical candidate shapes when useful, but must not choose a generic ECS, permanent container, stable wire ID, protocol handle, persistence schema, or runtime module layout.

Do not import `ai::ActorId(u64)` as production authority. Do not stringify/cast AI/client handles into the shared reference. Do not model Instance support in this first evidence child.

## Required evidence by resource row

### `RUNTIME-ACTOR-RL-01` — active authoritative actor records per Channel

Required evidence:

- exact fixed/retained bytes per candidate actor record for each measured candidate shape;
- checked total retained-byte equations across tested occupancies;
- mixed actor populations including at least player-like, creature-like and NPC/system-like records so AI-only occupancy cannot define the store;
- direct lookup and insertion/removal work accounting expressed in deterministic operation/work units where possible;
- boundary behavior for every **tested candidate ceiling**, including max/max+1 and checked arithmetic overflow;
- the first Reference functional lower bound stated separately from any production safety ceiling.

The first functional lower bound may describe only the actors actually needed by the selected component fixtures. It is not a production capacity claim.

If no accepted representative capacity workload/hardware cell exists that can justify a total Channel actor ceiling without conflicting with ADR-0009/PERF-01, return:

```text
RUNTIME-ACTOR-RL-01 = PERF_REFERENCE_CELL_REQUIRED
```

with the smallest exact benchmark inputs still missing. Do not infer a maximum from `AI01-ACTIVE-ACTORS`, Content spawn cardinality, map cells, protocol outstanding commands, or arbitrary headroom.

### `RUNTIME-ACTOR-RL-02` — lookup/index entries

Measure the candidate physical representation.

If exactly one index entry is inseparably present for every active actor slot and cannot grow independently, prove the one-to-one invariant and classify it as the same bounded resource as RL-01. If it can grow independently, retain a separate row and report its exact candidate evidence; do not hide map/table backing behind an actor-count label.

### `RUNTIME-ACTOR-RL-03` — generation/tombstone/high-water state

Run repeated remove/reinsert/recycle churn and determine whether stale-reference protection is stored within the same bounded slot/record or creates independently growing retained history.

If generation is fixed in the reusable slot and no separate tombstone/history collection grows, prove same-resource identity. If retained history grows independently, report the exact resource and stop short of accepting a maximum without its own evidence.

### `RUNTIME-ACTOR-RL-04` — exact lookup work/candidate materialization

Preferred first-carrier shape is direct exact lookup, with no world scan, nearest-N search, spatial query, retained async lookup, or variable candidate materialization.

Prove either:

```text
NOT_EXERCISED_VARIABLE_COLLECTION
```

or identify and measure the exact amplified resource. Existing Ability `ABILITY01-TARGET-CANDIDATES` / `ABILITY01-RESOLVED-TARGETS` remain the consumer occurrence envelope; they do not automatically bound actor-store lookup internals.

### `RUNTIME-ACTOR-RL-05` — variable per-actor payload

Keep the evidence carrier fixed-shape. AI memory, combat state, inventory, loot, dialogue, behavior graphs and other variable domain payload remain owned elsewhere.

If the candidate requires variable actor-owned payload to satisfy even the first exact lookup/local-step boundary, inventory that exact field/resource and return it as a blocker rather than smuggling it into a fixed actor-size estimate.

## Required correctness/negative evidence

The harness/evidence package must include deterministic tests for at least:

1. exact current actor reference resolves;
2. missing actor rejects;
3. stale generation rejects after removal/recycle;
4. same local identity with a newer generation cannot be targeted by an old ref;
5. cross-Channel reference rejects;
6. an actor cannot be partially visible in the lookup index when admission exceeds a tested candidate ceiling;
7. capacity rejection leaves all pre-existing actor/index/generation state unchanged;
8. checked count/byte arithmetic rejects overflow before allocation;
9. lookup result does not depend on hash/thread/enumeration order;
10. repeated churn does not grow hidden tombstone/generation history when the candidate claims RL-03 is the same resource;
11. AI-local/client/protocol handles cannot be substituted for the shared semantic reference in the evidence model;
12. no geometry, range, LoS, pathfinding or visibility scan is reachable from the modeled exact lookup.

## Measurement discipline

Prefer deterministic structural measurements and explicit work-unit accounting over noisy microbenchmark timing.

Where wall-time/throughput measurements are included, record exact hardware/runner, OS, compiler, optimization mode, artifact revision, population/workload and repeated sample count. Such timings are diagnostic unless the accepted PERF-01 process names the cell as capacity evidence.

For every candidate ceiling M that is tested, record M and M+1 behavior independently. A tested candidate ceiling is not an accepted production hard maximum merely because its test passes.

The worker should reuse std-only/standalone measurement patterns where practical so no Cargo/workspace dependency is required. If a material dependency/Cargo/shared-path requirement is discovered, return `SHARED_LEASE_REQUIRED` and stop that path rather than seizing it.

## Relationship to first Reference slices

The evidence package must preserve these independent layers:

```text
shared Channel actor carrier resource safety (#530)
!= Ability occurrence target envelope (#508 / ABILITY01)
!= Movement work/state resources (#139)
!= AI bootstrap population/resource envelope
!= Content authored spawn definitions
!= production player-capacity claims (ADR-0009 / PERF-01)
```

The first movement child remains `REFERENCE_LOCAL_STEP_STATIC_KERNEL/v1`: one current actor, one cardinal adjacent static destination cell, deterministic walkable/blocked result, with diagonal/floor-change/relocation/pathfinding/dynamic occupancy/visibility/speed/Discovery excluded.

The first Ability Phase A remains `ABILITY_EXACT_ACTOR_RESOLUTION_V1`: one exact requested actor -> at most one current resolved target, no geometry/spatial scan and no dynamic retargeting.

The evidence worker does not implement either child.

## Expected return

Return one compact packet:

```yaml
issue: 530
worker_task_id: OTV2-20260910-runtime-actor-carrier-resource-evidence-530
admission_main_sha: <protected allocation merge SHA>
branch: agent/runtime-actor-resource-evidence-530
head_sha: <exact head>
changed_paths: []
resource_rows:
  RUNTIME-ACTOR-RL-01: <classification + measurements + missing benchmark discriminator>
  RUNTIME-ACTOR-RL-02: <classification + evidence>
  RUNTIME-ACTOR-RL-03: <classification + evidence>
  RUNTIME-ACTOR-RL-04: <classification + evidence>
  RUNTIME-ACTOR-RL-05: <classification + evidence>
first_reference_functional_lower_bound: <explicit non-production lower bound>
accepted_production_maximum_selected: false
perf_reference_cell_required: true|false
focused_validation: []
self_review: <result>
blocker: <one precise blocker or null>
recommended_530_disposition: <one next action>
```

## Validation

For the allocation PR itself:

- `python tools/agents/validate_governance.py`;
- `git diff --check`;
- full effective-diff self-review;
- canonical exact-head repository checks / `game-gate`;
- runtime E2E: `NOT_APPLICABLE` because this allocation changes no executable product code.

For the later evidence worker, run the standalone harness tests/measurements, checked JSON/evidence consistency validation, `git diff --check`, governance validation, and all exact-head checks selected by the changed paths.

## Independent review / integration

This allocation is a control-plane document that grants future autonomous write authority to four new evidence-only paths after activation. Under the bound META 3.1 AI review policy it is therefore treated conservatively as a material control-plane candidate:

```text
INDEPENDENT_DEEP_REVIEW = REQUIRED_BEFORE_INTEGRATION
OWNER_FUNDED_REVIEW_AUTHORIZATION = NOT_INFERRED
EXACT_CANDIDATE_MERGE_QUEUE_AUTHORIZATION = NOT_GRANTED_BY_THIS_DOCUMENT
```

Do not invoke an owner-funded reviewer without separate applicable authorization. Do not direct-merge, use generic auto-merge, bypass protection, or use a no-op/retrigger commit. Integration must follow the bound native exact-head Merge Queue policy when the authorized capability is available, then require real `merge_group` `game-gate` success and protected-main readback.

## Completion

The allocation itself is complete only after protected integration/readback. The evidence worker is complete when every `RUNTIME-ACTOR-RL-*` row has a reproducible classification and #530 receives the smallest truthful next action.

A result of `PERF_REFERENCE_CELL_REQUIRED` for RL-01 is a valid evidence outcome; it is not authorization to guess a number. #139 remains unactivated until the shared runtime-carrier prerequisite and the exact first Movement resource gate are legitimately ready.
