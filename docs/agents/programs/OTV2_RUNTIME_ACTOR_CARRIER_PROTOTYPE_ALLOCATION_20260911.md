# Runtime actor carrier prototype allocation

- Coordinator: #162
- Resource gate: #530
- Ability successor: #508
- Movement resource gate: #139
- Originally prepared against protected `main@663bd35a5196a925fc6eb0318381ad0b97f4cc2c`
- Reconciled by normal merge to protected `main@5ec6ca6369e98a6f66f679cfc7fc1248fb5992fb`

## Status and authority

```yaml
allocation_id: OTV2-RUNTIME-ACTOR-CARRIER-PROTOTYPE-530
repository: Oteryn/Oteryn-Game
coordinator_issue: 162
resource_issue: 530
ability_successor_issue: 508
movement_resource_issue: 139
allocation_state: NOT_ACTIVE
worker_launch: NOT_STARTED
allocation_branch: coord/runtime-actor-carrier-prototype-530-20260911
branch_after_activation: agent/runtime-actor-carrier-prototype-530
execution_target: isolated_workspace_or_repository_native_ci
validation_target: github_actions
lane_strategy: single_agent
runtime_implementation_authority: NON_PRODUCTION_EXAMPLE_ONLY
resource_registry_mutation_authority: NONE
production_authority: NONE
external_repository_write_authority: NONE
```

This is a **prospective non-production prototype/evidence allocation**. It becomes usable only after this exact allocation is protected-integrated, protected-main readback confirms it, and the unique #162 Work Delivery Coordinator performs a fresh ownership/custody check and explicitly activates the same worker lineage.

Publication, a green PR, Issue #530, or an alias invocation does not activate the worker. This allocation does not implement a production `ChannelRuntime`, select a production actor capacity, modify `RESOURCE_LIMITS_REGISTRY.json`, activate #508 Phase A, activate #139 Movement, or grant Merge Queue authority.

## Proven admission facts

At the current reconciled candidate:

- the allocation was originally prepared against protected `main@663bd35a5196a925fc6eb0318381ad0b97f4cc2c` and has since consumed protected `main@5ec6ca6369e98a6f66f679cfc7fc1248fb5992fb` by a normal two-parent merge with no rebase/reset/force-push;
- the protected delta consumed by that merge is path-disjoint control-plane/documentation work: #561 is the WP3 TLS1.3 final-flight allocation and #562 is UI/BUILD_TEST_MATRIX documentation; neither creates an actor-carrier implementation writer or changes the future prototype paths below;
- protected #537 already completed the synthetic resource-evidence pass; it must not be duplicated;
- protected #541 accepted `RUNTIME-ACTOR-LOCAL-GENERATION-V1`, including immutable `ActorLocalId` -> logical slot/generation-cell binding within one `ScopeOwnershipGeneration`, checked generation advance before reuse, fail-closed exhaustion, and fail-closed same-generation carrier reconstruction unless the exact generation/binding state is restored from an already-authorized source;
- protected #541 requires distinct `ActorLocalId` values never to alias one logical slot/generation cell within the same scope generation;
- protected #541 requires any failed reuse/admission with a representable generation successor to preserve the prior slot lifecycle/generation, indexes, externally resolvable references and all existing actors; the terminal `EXHAUSTED` transition is the sole deliberate failed-allocation state mutation when no successor exists;
- protected #541 requires local-generation exhaustion to atomically make the exhausted slot terminal for that outer generation, publish no replacement actor/index/current reference, leave all existing actors unchanged, and never select that exhausted slot again;
- protected #541 requires repeated same-slot churn plus churn/retirement across the configured namespace to prove there is no independently growing retirement/tombstone history and that retained generation-state cardinality remains exactly the configured slot cardinality `M`;
- protected #541 permits a fresh actor-local namespace only after a legitimately newer outer `ScopeOwnershipGeneration` fences every prior reference; a carrier restart/loss must never reset the namespace while the same outer generation remains authoritative;
- #530 independently requires checked count/size arithmetic before allocation, max/max+1 rejection before partial actor/index mutation, deterministic direct-lookup work evidence at tested occupancies, and honest physical accounting for slot/record plus lookup/index backing rather than hiding independently growing backing behind an actor-count label;
- #162 cleanup after protected #548 explicitly retired the premature PERF detour and set `NEXT_PATH: SHARED_CHANNEL_ACTOR_CARRIER_PROTOTYPE_EVIDENCE`;
- that cleanup explicitly permits a non-production carrier evidence/prototype to establish physical shape, current-owner lookup, and bounded correctness without selecting a production total-actor maximum;
- #508 Phase A still requires a direct current-owner exact actor lookup and forbids a second Ability-owned actor registry or geometry/spatial scans;
- #139 remains unactivated and must later be re-evaluated only for `REFERENCE_LOCAL_STEP_STATIC_KERNEL/v1` after the shared carrier boundary is protected/readable;
- protected server source exposes the real Foundation `RuntimeScopeRefV1` and `ScopeOwnershipGeneration` types; the prototype must consume those public types rather than clone their semantics into a competing authority;
- fresh open PR/Issue searches found no current `ChannelRuntime`/runtime-actor implementation writer and no existing actor-carrier prototype allocation;
- `apps/game-server/examples/` does not currently exist, so the single proposed example path is additive and isolated;
- active WP3 #351/#356, frozen WP4 #329/#335 and current client/UI work are path-disjoint from the prospective prototype paths below.

Changing any material fact requires fresh reconciliation before activation.

## Worker outcome

Build the smallest executable Rust prototype for `CHANNEL_RUNTIME_ACTOR_CARRIER_V1` that proves the accepted identity/generation/current-owner shape using the actual public Foundation scope primitives while remaining outside production runtime composition.

The prototype must answer only these questions:

1. Can one fixed-shape Channel-local carrier represent exact `WorldId + ChannelId + ScopeOwnershipGeneration + actor-local identity + actor-local generation` without omitting a fence component?
2. Can direct exact lookup accept only the current scope/generation and current actor generation, while deterministically rejecting missing, stale, recycled and cross-scope references?
3. Can the #541 finite local-slot/generation rule be represented with checked reuse and fail-closed exhaustion without independently growing tombstone/history state?
4. Can distinct actor-local IDs remain immutably one-to-one with distinct logical slot/generation cells so one ID can never resolve through another ID's cell?
5. Can a failed reuse/admission after selecting a reusable slot and deriving a valid generation successor roll back completely when any later admission step fails, without advancing the slot or exposing partial index/reference state?
6. Can same-generation carrier loss/reconstruction either restore the exact actor-local generation/binding state or fail closed, while allowing namespace reset only after a genuinely newer outer `ScopeOwnershipGeneration` fences all prior references?
7. Can repeated churn across the entire configured namespace keep retained generation/history cardinality exactly bounded by `M`, with no per-retirement growth?
8. Can all count/capacity/byte-size composition use checked arithmetic and reject overflow before allocation or authoritative state mutation?
9. What exact physical slot/record and lookup/index backing exists for the candidate, can either backing grow independently, and how is every exercised retained resource bounded for each tested `M`?
10. What deterministic direct-lookup work is performed at representative occupancy and every tested candidate `M`, with no scan/enumeration or occupancy-dependent candidate materialization?
11. Can a minimal direct lookup expose only the current existence/actionable fact and current authoritative position needed by the first exact-target/local-step proofs, without geometry, scanning or dynamic retargeting?
12. What is the concrete Rust candidate shape/size and deterministic operation behavior for the tested prototype points, explicitly without converting those points into a production capacity claim?

## Exact owned paths after activation

Only the following paths become writable after protected allocation integration plus explicit #162 activation:

```text
apps/game-server/examples/runtime_actor_carrier_prototype.rs
docs/agents/evidence/OTV2-20260911-runtime-actor-carrier-prototype.json
docs/agents/evidence/OTV2-20260911-runtime-actor-carrier-prototype.md
docs/agents/tasks/active/OTV2-20260911-runtime-actor-carrier-prototype-530.md
```

Everything else is read-only.

In particular, this allocation grants **no write authority** to:

```text
apps/game-server/src/**
apps/game-server/tests/**
apps/game-server/Cargo.toml
Cargo.toml
Cargo.lock
docs/contracts/RESOURCE_LIMITS_REGISTRY.json
docs/architecture/**
docs/agents/programs/** except this already-protected allocation itself
.github/**
vendor/**
Platform / Atlas / META / external repositories
```

If the prototype cannot be completed without any excluded path, return exactly:

```text
SHARED_LEASE_REQUIRED = <exact path> :: <exact symbol/resource> :: <reason>
```

and preserve the branch without seizing the path.

## Required prototype boundary

The one Rust example may define **prototype-local candidate** actor types, but it must import and use the protected public Foundation scope types rather than redefining them as authority:

```text
RuntimeScopeRefV1::Channel { WorldId, ChannelId }
ScopeOwnershipGeneration
```

The prototype-local actor reference must bind, inseparably:

```text
exact RuntimeScopeRefV1::Channel
+ exact ScopeOwnershipGeneration
+ opaque prototype ActorLocalId
+ prototype ActorLocalGeneration
```

The local ID/generation widths and container representation are candidate evidence only. They are not public API, wire format, stable IDs, resource-registry values or permanent storage decisions.

The candidate carrier must be Channel-only. Instance support is excluded.

## Current-owner rule

A lookup may resolve only when independently supplied **current** owner facts match the reference and carrier binding:

```text
current RuntimeScopeRefV1::Channel
+ current ScopeOwnershipGeneration
+ ActorTargetRefPrototype
-> direct exact slot lookup
-> exact actor-local identity-to-slot binding
-> exact actor-local generation/current existence check
-> minimal read-only actor facts
```

The actor reference itself is expected binding evidence; it is not allowed to self-prove that its scope generation is still current.

A mismatched/stale current `ScopeOwnershipGeneration` must reject even if actor-local identity/generation otherwise matches. A different `WorldId` with the same `ChannelId` must reject. A different `ChannelId` in the same world must reject. A distinct actor-local ID must never be accepted merely because it reaches a slot carrying a matching local generation.

Direct exact lookup must not enumerate actors, materialize a variable candidate collection, depend on hash/thread iteration order, or hide occupancy-dependent scan work. The worker must record deterministic lookup work units for representative occupancy and every tested candidate `M`.

## Actor-local slot/generation rule

Preserve protected `RUNTIME-ACTOR-LOCAL-GENERATION-V1`:

- finite logical slots for each tested prototype instance;
- immutable one-to-one `ActorLocalId` -> logical slot/generation cell for one scope ownership generation;
- two distinct `ActorLocalId` values cannot alias the same logical slot/generation cell within that scope generation;
- removal retains the slot generation;
- reuse requires checked generation successor before publication;
- after selecting a reusable slot and deriving a representable successor, **any later admission failure** must restore the exact prior slot lifecycle/generation and leave lookup/index/current-reference state plus every existing actor unchanged; no prospective generation/index/reference may become visible;
- stale references never resolve after successful reuse;
- when the generation has no successor, atomically transition `VACANT_REUSABLE(g_max) -> EXHAUSTED(g_max)`, publish no replacement actor/index/current reference, leave every existing actor unchanged, and never select that exhausted slot again for the remainder of the same `ScopeOwnershipGeneration`;
- actor-local generation state and immutable ID-to-slot/generation-cell bindings survive for the full lifetime of the same authoritative `ScopeOwnershipGeneration`;
- if carrier state is lost or reconstructed while that same outer generation remains authoritative, the prototype must either restore the exact required generation/binding state from an already-authorized source or fail closed; it must not initialize a fresh local namespace under the same generation;
- a fresh local namespace may be initialized only after an independently legitimate owner transition establishes a genuinely newer `ScopeOwnershipGeneration`, whose outer fence rejects every reference from the prior namespace before local actor lookup is accepted;
- repeated retirement/reuse must mutate only the finite lifecycle/generation state already owned by the configured logical slots; it may not allocate a per-retirement tombstone/high-water/history entry;
- retained actor-local generation-state cardinality must remain exactly `configured_actor_slots = M` for the tested carrier throughout churn, including after every configured slot has been used/retired/reused;
- no unbounded retired-ID/tombstone/history collection;
- no remapping of one `ActorLocalId` to another logical generation cell.

A slot/index implementation detail is never sufficient authority by itself: successful resolution still requires the complete scope/generation/local-generation reference and the immutable ID-to-slot binding.

This allocation does not authorize a persistence schema merely to survive carrier loss. If exact same-generation reconstruction cannot be proven from already-authorized state within the owned prototype paths, the safe prototype behavior is **fail closed until a legitimate newer outer scope generation exists**.

## Physical resource and arithmetic evidence

The prototype must report the actual candidate physical representation rather than only semantic cardinalities.

For every tested candidate `M`, record at minimum:

```text
configured_actor_slots = M
slot_or_record_size_bytes
retained_slot_or_record_bytes
lookup_or_index_entries
lookup_or_index_entry_size_bytes (if physically distinct)
lookup_or_index_retained_bytes (if physically distinct)
retained_generation_cells = M
independent_retirement_history_entries = 0
```

If actor storage and lookup/index are physically the same one-to-one bounded resource, prove that identity explicitly. If a map/table/vector/index backing can grow independently of active slots, record its exact capacity/backing growth and a finite tested bound; do not classify it silently as the same resource merely because logical entry count is one per actor.

Every multiplication/addition used to derive slot count, backing count, retained bytes, index bytes or combined candidate bytes must use checked arithmetic. Deliberately exercise count/byte overflow and prove rejection occurs before allocation, slot selection, generation change, index mutation or any authoritative actor-state mutation.

## Minimal carried facts

The prototype may retain only fixed-shape facts needed to prove the first shared boundary:

```text
actor kind discriminator: player-like | creature-like | npc/system-like
current existence / slot lifecycle
current actionable flag
current authoritative local position: minimal fixed x/y/z candidate representation
```

These fields are prototype evidence only. Do not add inventory, combat values, AI memory, behavior state, dialogue, loot, persistence, protocol handles, visibility state, timers or other variable domain payload.

## Capacity discipline

No tested `M` is a production maximum.

The worker may test the already-evidenced functional lower bounds and a few small deterministic candidate points, including:

```text
local-step minimum: 1 active actor
exact-target minimum: 2 active actors
mixed-kind evidence minimum: 3 active actors
```

Additional small test points are allowed only to exercise shape/boundary behavior. The result must carry:

```text
accepted_production_maximum_selected: false
resource_registry_mutated: false
production_capacity_claim: false
```

Do not use `AI01-ACTIVE-ACTORS=256`, synthetic #537 M values, map size, spawn count or arbitrary headroom as a production actor ceiling.

## Mandatory correctness matrix

The executable prototype and evidence must prove at least:

1. exact current actor resolves with matching world, channel, current scope generation, local ID and local generation;
2. missing/vacant actor rejects;
3. stale actor-local generation rejects after remove/reuse;
4. same local ID with newer generation cannot be targeted by an older ref;
5. two distinct `ActorLocalId` values cannot alias one logical slot/generation cell, and a reference for ID A cannot resolve an actor admitted under ID B even when other local fields/generation values would otherwise match;
6. same world + wrong channel rejects;
7. same channel + wrong world rejects;
8. stale/mismatched independently supplied current `ScopeOwnershipGeneration` rejects and leaves carrier state unchanged;
9. removal leaves the old reference stale and cannot expose a partial replacement;
10. after a reusable slot has been selected and a valid local-generation successor derived, inject a later admission failure and prove **complete rollback**: slot lifecycle/generation, free-slot state, lookup/index state, externally resolvable references and every existing actor are byte/semantically unchanged from the pre-attempt state;
11. checked local-generation exhaustion atomically records `VACANT_REUSABLE(g_max) -> EXHAUSTED(g_max)`, publishes no replacement actor/index/current reference, leaves every unrelated/existing actor unchanged, and proves that exhausted slot is never selected again under the same outer generation;
12. destroying/reconstructing carrier state while the same `ScopeOwnershipGeneration` remains authoritative cannot reset actor-local generations or ID-to-slot bindings: exact authorized state is restored or actor lookup/admission fails closed;
13. a fresh local namespace is accepted only after a genuinely newer independently supplied `ScopeOwnershipGeneration`, and every pre-transition reference rejects on the outer generation before local actor state can resolve;
14. repeated same-slot remove/reuse churn plus use/retirement/reuse across **every configured logical slot** leaves retained generation/history cardinality exactly `M`, creates no independently growing tombstone/history structure, and preserves immutable ID-to-slot bindings;
15. tested M+1 admission rejects before partial mutation and preserves the complete pre-existing actor/slot/index/generation/free-slot/current-reference state unchanged;
16. checked count/slot/index/byte arithmetic overflow rejects before allocation or any carrier/actor/index/generation mutation;
17. mixed player/creature/NPC-system actor occupancy follows the same carrier path;
18. fixed-size retained bytes per actual slot/record and, where physically distinct, per lookup/index entry are measured from the exact Rust candidate at every tested `M`; independently growable backing is measured and bounded rather than hidden behind `M`;
19. direct lookup work is recorded deterministically at representative occupancy and every tested candidate `M`, performs no actor/world scan or variable candidate materialization, and does not depend on hash/thread/enumeration order;
20. no geometry, range, LoS, nearest-N, visibility, pathfinding or dynamic retargeting path exists;
21. no `ai::ActorId`, Ability fixture `TargetId(String)`, client handle, pointer or string cast can substitute for the shared prototype reference;
22. prototype size/shape evidence is regenerated deterministically from the exact Rust candidate and records the retained generation-cell cardinality plus physical slot/index backing for each tested `M`;
23. after the complete negative/churn matrix, the evidence explicitly reports `retained_generation_cells == configured_actor_slots == M` and `independent_retirement_history_entries == 0` for the selected prototype shape.

## Relationship to #508 and #139

This worker does **not** implement either successor.

A successful prototype gives #162 concrete protected evidence for the shared physical/current-owner boundary. Only after protected integration/readback may #162 freshly reconcile whether #508 Phase A can consume that boundary without creating another actor registry.

Likewise #139 remains inactive. Its next re-evaluation is limited to `REFERENCE_LOCAL_STEP_STATIC_KERNEL/v1` and only resources exercised by that exact child.

Production capacity acceptance and any required registry serialization remain later gates before **production implementation acceptance**. They are not prerequisites to this bounded non-production prototype and cannot be satisfied by its test points.

## Validation

For this allocation PR:

- `python tools/agents/validate_governance.py`;
- `git diff --check`;
- full effective-diff adversarial self-review;
- canonical exact-head repository checks / `game-gate`;
- runtime E2E: `NOT_APPLICABLE` because this allocation changes documentation only.

For the later worker after activation:

- compile/run the isolated example test target through the existing `apps/game-server` manifest, without manifest edits;
- focused prototype matrix above;
- deterministic evidence regeneration/readback;
- `cargo fmt --check` for affected Rust;
- `cargo clippy --manifest-path apps/game-server/Cargo.toml --all-targets -- -D warnings` when selected by current repository policy;
- `cargo test --manifest-path apps/game-server/Cargo.toml --example runtime_actor_carrier_prototype` or the exact repository-supported equivalent after live readback;
- `python tools/agents/validate_governance.py`;
- `git diff --check`;
- all repository-native exact-head checks selected by the changed paths.

If the live build/test matrix selects a different exact command, the worker must follow the current protected matrix rather than this historical command spelling.

## Independent review / integration

This allocation grants future write authority to a high-risk fencing/multichannel prototype path after activation, so it requires a genuinely independent exact-head review before integration.

The currently active #162 standing bounded owner authorization may fund the applicable independent review and exact-candidate native Merge Queue transition only after fresh qualification. It does not authorize direct merge, generic auto-merge, protection bypass, force-push, skipped checks, production deployment or cross-repository writes.

If the connected surface does not expose the repository-required native `merge-async` Merge Queue primitive, preserve the exact candidate and report `BLOCKED_CAPABILITY_UNAVAILABLE`; do not substitute another merge primitive. Continue legal path-disjoint coordinator work.

## Expected worker return

```yaml
issue: 530
worker_task_id: OTV2-20260911-runtime-actor-carrier-prototype-530
admission_main_sha: <protected allocation merge SHA>
branch: agent/runtime-actor-carrier-prototype-530
head_sha: <exact head>
changed_paths: []
prototype_shape: <exact Rust candidate summary>
uses_protected_foundation_scope_types: true
current_owner_lookup: <PASS|BLOCKED>
immutable_actor_id_slot_binding: <PASS|BLOCKED>
local_generation_reuse_reconstruction_exhaustion: <PASS|BLOCKED>
post_selection_failure_rollback: <PASS|BLOCKED>
rl03_churn_cardinality: <PASS|BLOCKED; retained_generation_cells=M; independent_history=0>
lookup_index_shape: <same-resource proof OR exact distinct backing/cardinality/capacity>
direct_lookup_work: <deterministic work at representative occupancy and every tested M>
overflow_preallocation: <PASS|BLOCKED>
negative_matrix: <results>
shape_evidence: <exact regenerated evidence refs>
accepted_production_maximum_selected: false
resource_registry_mutated: false
production_capacity_claim: false
focused_validation: []
self_review: <result>
blocker: <one precise blocker or null>
recommended_next_action: <one next coordinator action>
```

## Completion

The allocation itself is complete only after protected integration/readback. The future worker is complete only when the one isolated prototype path and its evidence prove the bounded matrix without claiming production acceptance.

`IMPLEMENTATION_AUTHORITY: NONE_UNTIL_PROTECTED_INTEGRATION_AND_EXPLICIT_162_ACTIVATION`
`PRODUCTION_AUTHORITY: NONE`
`REGISTRY_MUTATION_AUTHORITY: NONE`
`MOVEMENT_139_AUTHORITY: NONE`
`ABILITY_508_PHASE_A_AUTHORITY: NONE`
