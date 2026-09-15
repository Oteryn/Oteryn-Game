# Runtime actor-local identity reuse and generation retention

- Decision: `RUNTIME-ACTOR-LOCAL-GENERATION-V1`
- Status: **CANDIDATE; acceptance requires independent exact-head review and protected integration**
- Source escalation: Issue #539
- Resource gate: Issue #530
- Evidence prerequisite: PR #537 protected as `main@2d33d812e578087ac982afc03964fb1917b05b4d`
- `MERGE_AUTHORITY: ARCHITECTURE_COORDINATOR_ONLY`

## Resolution packet

```yaml
classification: ARCHITECTURE_RESOLUTION
repository: Oteryn/Oteryn-Game
main_sha: 2d33d812e578087ac982afc03964fb1917b05b4d
source_escalation: 539
blocking_question: >-
  For CHANNEL_RUNTIME_ACTOR_CARRIER_V1, how may actor-local identities be
  retired/reused, how long must actor-local generation state survive, and
  what happens when that finite generation namespace cannot safely advance?
facts:
  proven:
    - "#537 proves unique opaque identity retirement can grow retained history independently if identities are not drawn from a fixed reusable namespace."
    - "#537 proves 1,000 reuses of one identity can advance generation without retained-cardinality growth."
    - "FND-03 requires current scope ownership generation fencing and fail-closed stale-generation rejection."
    - "#508 requires an exact actor reference to bind current Channel scope/fence, actor-local semantic identity and actor-local generation."
    - "WorldId and ChannelId remain distinct and inseparable for Channel scope identity."
  derived:
    - "The already non-reused ScopeOwnershipGeneration can safely act as the outer incarnation fence for a finite actor-local namespace."
    - "A fixed local slot namespace with one retained generation cell per slot removes the independently growing tombstone/history dimension demonstrated by #537."
  unknown:
    - "The production Channel actor-slot maximum M; ADR-0009/PERF-01 owns that measured value."
    - "The final Rust container/index/free-slot implementation and exact ABI/allocator footprint."
    - "The final contract-owned numeric/internal error code representation."
  conflict: []
accepted_decision: RUNTIME-ACTOR-LOCAL-GENERATION-V1, conditional on protected integration
rejected_options:
  - ever_growing_tombstones_keyed_by_every_retired_opaque_identity
  - generation_wrap_or_saturating_reuse
  - raw_pointer_vector_index_or_client_handle_as_actor_authority
  - actor_namespace_reset_while_the_same_scope_ownership_generation_remains_authoritative
  - actor_local_id_remap_or_alias_to_a_different_generation_cell_within_one_scope_generation
  - separate_unbounded_generation_history_hidden_behind_active_actor_count
  - bump_scope_ownership_generation_only_to_reclaim_actor_ids
  - durable_actor_identity_history_as_a_first_carrier_prerequisite
affected_contracts:
  - FND-03 runtime ownership/generation fencing
  - Issue #530 RUNTIME-ACTOR-RL-03
  - Issue #508 ABILITY_EXACT_ACTOR_RESOLUTION_V1 prerequisite
affected_paths:
  - docs/architecture/reviews/OTERYN_GAME_RUNTIME_ACTOR_LOCAL_GENERATION_DECISION_2026-09-10.md
  - docs/agents/tasks/active/OTV2-20260910-runtime-actor-local-generation-539.md
implementation_owner: "future #162-allocated shared Channel runtime carrier worker"
implementation_scope: >-
  typed finite actor-local slot namespace plus inseparable local generation
  state inside the first Channel-owned carrier; no implementation authority
  is granted by this decision
resource_values_changed: false
production_authority_changed: false
cross_repository_authority_changed: false
supersedes: []
required_validation:
  - actor_local_id_has_immutable_one_to_one_logical_slot_binding_within_scope_generation
  - distinct_actor_local_ids_cannot_alias_one_logical_generation_cell
  - same_scope_same_slot_reuse_advances_generation
  - stale_pre_reuse_ref_rejects
  - removal_without_reuse_rejects_old_ref
  - scope_generation_change_fences_entire_prior_local_namespace
  - same_scope_namespace_reset_is_forbidden_or_fails_closed
  - checked_local_generation_exhaustion_never_wraps
  - exhaustion_records_terminal_slot_state_without_actor_or_index_publication
  - exhaustion_rejects_before_partial_actor_or_index_publication
  - no_retirement_path_allocates_unbounded_history
  - retained_generation_cardinality_equals_configured_slot_cardinality
required_independent_review: "YES — exact-head shared-identity/resource/fencing review"
next_action: >-
  Independently review this exact candidate and, if clean, return it to the
  unique #162 control plane for protected integration.
```

## Problem

The first shared Channel actor carrier needs stale-reference safety without an unbounded history of every actor identity ever observed. PR #537 demonstrated both sides of the problem: reuse of one local identity can keep generation retention constant, while retiring an unbounded sequence of unique opaque identities creates history growth even when the active actor count returns to zero.

The decision must close `RUNTIME-ACTOR-RL-03` without inventing a production actor count and without turning a vector position, pointer, client handle, AI fixture ID or string into authority.

## Constraints

The accepted outer authority remains:

```text
WorldId
+ ChannelId
+ current ScopeOwnershipGeneration
+ actor-local semantic identity
+ actor-local generation
```

The first carrier is Channel-only. `ScopeOwnershipGeneration` is the owning runtime fence, is never authority-reused, and rejects stale scope work before actor state can become authoritative. Actor-local identity remains runtime-local and is not a durable Character identity, protocol handle or public stable ID.

The carrier must stay finite. Removal, reuse and exhaustion cannot evict a live actor, expose a partially inserted actor/index entry, downgrade to a weaker identifier, or change Ability/Movement/AI semantics.

## Accepted decision

### 1. One finite local namespace per authoritative scope generation

`CHANNEL_RUNTIME_ACTOR_CARRIER_V1` owns one finite typed actor-local namespace for each exact:

```text
(WorldId, ChannelId, ScopeOwnershipGeneration)
```

The namespace contains exactly the configured actor-slot cardinality `M`. This decision does **not** choose `M`; the production value remains blocked on ADR-0009/PERF-01.

`ActorLocalId` is an opaque typed identity within that namespace. For the complete lifetime of one `ScopeOwnershipGeneration`, every valid `ActorLocalId` has an immutable one-to-one binding to exactly one **logical actor slot** and therefore to exactly one actor-local generation cell. An `ActorLocalId` cannot later be remapped to another logical slot, and two distinct `ActorLocalId` values cannot alias the same logical slot/generation cell within that scope generation.

An implementation may move physical storage or map the typed identity efficiently to its logical slot, but such physical relocation must preserve that immutable logical binding. A raw array/vector index, pointer or client-visible handle is not independently valid actor authority and cannot bypass the complete typed reference.

### 2. Generation belongs to the immutable logical slot binding, not to an ever-growing retirement map

Every configured logical slot owns one finite checked `ActorLocalGeneration` cell for the lifetime of the current scope ownership generation. That cell exists whether the slot is currently occupied or vacant after prior use. Because `ActorLocalId -> logical slot` is immutable within that scope generation, generation history cannot be lost or switched merely by remapping an ID to a different physical storage position.

Abstract state is sufficient for the contract:

```text
NEVER_USED
LIVE(g)
VACANT_REUSABLE(g)
EXHAUSTED(g_max)
```

Exact Rust enum/layout is not selected here.

A successful first use establishes a valid local generation according to the future typed implementation contract. Removing an actor changes `LIVE(g) -> VACANT_REUSABLE(g)` only after the actor is no longer current. Removal does not erase or decrement `g` and does not detach that retained generation from the logical slot/`ActorLocalId` binding.

### 3. Reuse requires checked generation advance before publication

Reusing the same `ActorLocalId` in the same `ScopeOwnershipGeneration` requires a checked strict successor of the retained local generation in that ID's immutable logical slot. The successor is established as part of the admission transition before the replacement actor can become externally resolvable/current.

Therefore:

```text
old ref:  (..., local_id=L, generation=g)
new actor: (..., local_id=L, generation=g+1)
```

and the old reference cannot resolve to the new actor. No saturating increment, wrap, reset, decrement or same-generation reuse is permitted.

For a representable successor, a failed admission must not publish the new actor, lookup/index entry or current reference; every existing actor and all externally authoritative actor-resolution state remain unchanged. A checked **no-successor** failure at `g_max` is the deliberate exception for internal slot-lifecycle bookkeeping: it atomically records `VACANT_REUSABLE(g_max) -> EXHAUSTED(g_max)` while publishing no actor/index/current reference and changing no existing actor. That terminal slot-state transition prevents the allocator from selecting the same unsafe slot again and is not partial actor publication.

### 4. The outer scope generation is the namespace incarnation fence

Actor-local generation state is required to survive for the full lifetime of the **same** authoritative `ScopeOwnershipGeneration`. The carrier must not destroy and recreate a fresh local namespace while that same scope generation remains authoritative after actor references could have escaped.

If the carrier state is lost while the same scope generation is still nominally current, the runtime fails closed. It must either restore the exact required local-generation state and immutable logical ID-to-slot bindings from an already-authorized source or cease authority so that the normal owning scope lifecycle establishes a new `ScopeOwnershipGeneration`. This V1 does not require durable actor history and does not authorize a persistence schema.

Once a legitimately authorized scope-owner transition establishes a **new** `ScopeOwnershipGeneration`, the new owner may initialize a fresh finite actor-local namespace. Every reference from the prior namespace contains the old outer generation and is rejected before local actor state is accepted. Local generation high-water state therefore does not need to survive across distinct scope ownership generations merely to prevent stale-reference revival.

A scope ownership generation must never be advanced merely as an actor-ID garbage-collection trick. Its accepted authority/recovery lifecycle remains independently owned.

### 5. Exhaustion is checked, terminal for the failed allocation, and never wraps

If a previously used slot has no safe representable local-generation successor, the slot atomically enters `EXHAUSTED(g_max)` for the remainder of that `ScopeOwnershipGeneration`. It is never reused under that outer generation. Recording this terminal slot-lifecycle state is required finite capacity bookkeeping; it does not publish a replacement actor or change the retained generation value.

The semantic internal result is:

```text
ACTOR_LOCAL_GENERATION_EXHAUSTED
-> Foundation category: CAPACITY_EXCEEDED
-> progression: TERMINAL for the rejected actor-allocation attempt
-> mutation: EXHAUSTED_SLOT_STATE_ONLY; NO_PARTIAL_ACTOR_OR_INDEX_PUBLICATION
```

This decision names the semantic result but does not allocate a public wire code. The implementation may continue to admit through other non-live, non-exhausted slots within the configured finite carrier. If no safe slot remains available, new actor admission fails closed; it does not recycle a live/exhausted slot and does not self-advance `ScopeOwnershipGeneration` to manufacture capacity.

The existing stale-reference path remains `STALE_GENERATION`; generation exhaustion is a capacity/lifetime failure, not a stale-reference classification.

### 6. RL-03 is the same bounded first-carrier resource as the configured slot set

For this V1 shape, retained actor-local generation cardinality is exactly:

```text
retained_generation_cells = configured_actor_slots = M
```

Retirement allocates no per-retirement tombstone, high-water map entry or identity-history node. Reusing or exhausting a slot changes only the finite state already owned by that immutable logical slot/`ActorLocalId` binding.

Accordingly, after this decision is protected-integrated:

```text
RUNTIME-ACTOR-RL-03 = SAME_RESOURCE_AS_RL01_FOR_CHANNEL_RUNTIME_ACTOR_CARRIER_V1
independent_RL03_numeric_maximum = NOT_REQUIRED
```

This classification does **not** choose the RL-01 value. `M` still requires the accepted representative ADR-0009/PERF-01 capacity cell before registry serialization or executable carrier acceptance.

A future implementation that instead introduces a separately growing retirement/tombstone/history structure, or remaps actor-local IDs across independent generation cells, does not comply with this V1 classification and must reopen RL-03 with measured finite evidence before acceptance.

## Direct lookup and physical representation boundary

This decision does not choose a Rust `Vec`, slab, hash table, arena, ECS or allocator. The carrier may use a separate bounded lookup/index representation only if #530's RL-02 evidence and later physical selection account for it honestly. Any index/free-slot bookkeeping must remain finite relative to configured `M`; it cannot smuggle an independently unbounded identity history back into the design or permit two IDs to alias/remap across logical generation cells.

Exact-target lookup remains direct by complete typed actor reference and does not gain geometry/range/LoS/visibility/pathfinding or dynamic-retarget authority.

## Concurrency, late work and owner fencing

The one logical owner for the current Channel scope serializes local actor admission/removal/reuse. A late callback, AI decision, Ability occurrence, Movement request or other completion carrying an old scope generation or actor-local generation cannot become authoritative merely because the same local identity is live again.

Validation order must preserve the outer fence: wrong `WorldId`/`ChannelId` or stale `ScopeOwnershipGeneration` rejects before current actor state is exposed. Matching scope then requires exact local identity, that identity's immutable logical slot binding, and exact current local generation.

A removal/reuse transition cannot publish the new generation before the actor record/index state needed for exact current lookup is coherently available, and cannot leave a new actor/current reference visible if admission fails. A no-successor exhaustion may only publish the internal `EXHAUSTED` slot state described above.

## Options and trade-offs

### Selected — fixed local slot namespace with retained per-slot generation

Benefits:

- eliminates independently growing retirement history for the first carrier;
- composes directly with the already accepted non-reused outer scope generation;
- makes stale local references fail deterministically after reuse;
- keeps retention cardinality tied to the still-to-be-measured actor-slot capacity;
- requires no durable actor-history schema for the first vertical slice.

Costs:

- an individual slot becomes unavailable after representational generation exhaustion until a legitimate new outer scope generation;
- allocator/index implementation still needs a bounded physical design and PERF-backed total capacity;
- same-generation carrier loss must fail closed rather than silently reset local identities.

### Rejected — ever-growing tombstone/high-water map

This preserves stale-reference safety but recreates the exact independent-growth problem demonstrated by #537. It would require a separate finite RL-03 maximum and exhaustion policy without improving the first exact-target/local-step proof.

### Rejected — never reuse local identities within a scope generation

A monotonic unique-ID allocator avoids per-ID generation reuse but moves the lifetime problem into a monotonically consumed identity namespace. It eventually exhausts independently of active actor count and therefore does not close RL-03 for the first carrier.

### Rejected — remap an actor-local ID to another generation slot

Allowing one escaped `ActorLocalId` to move between independent generation cells can make an old `(ActorLocalId, generation)` pair current again when the destination cell carries the same generation value. Preventing that resurrection would require extra per-ID history, defeating the selected RL-03 classification. The logical ID-to-slot/generation-cell binding is therefore immutable within one scope ownership generation.

### Rejected — reset local generations after removal/restart

This can make an old escaped reference equal a later current reference and violates stale-generation safety unless an independently changed outer scope generation already fences it. Reset under the same outer generation is forbidden.

### Rejected — durable actor-local generation history now

Durable retention could support a broader lifetime model, but it expands persistence/schema/recovery authority and delays the next proof without being necessary when the outer scope generation already fences owner replacement. Reopen only if later requirements need actor-local references to remain meaningful across scope-owner generations.

## Decision timing

**Must decide now: YES.** #530 cannot classify RL-03, select a compliant first physical carrier, serialize its exercised resources or allocate the shared carrier while actor-local reuse/retention remains ambiguous. #508 Phase A and #139's first local-step child are downstream of that carrier.

The irreversible risk of deciding incorrectly is stale-reference resurrection or an unbounded hidden history structure. The selected rule avoids both while leaving the production slot count and physical data structure to measured evidence.

Supersession is justified only by named evidence showing that the finite per-slot lifecycle cannot satisfy a required future actor lifetime, recovery, migration or performance property. A successor must preserve outer scope fencing, no stale-reference revival, checked no-wrap exhaustion and a finite accounted resource model, and must state any migration/compatibility consequences.

## DECISIONS_NOT_TAKEN

This decision deliberately does not choose:

- production `M` / maximum actors per Channel;
- reference hardware, latency objectives, CPU/memory/network headroom or other PERF-01 values;
- Rust container, ECS, slab, hash/index, allocator or free-slot algorithm;
- exact bit width or public encoding of `ActorLocalId` / `ActorLocalGeneration`;
- protocol/client actor handle mapping;
- `InstanceRuntime` actor identity/storage;
- durable actor-state persistence or restart schema;
- spawn/respawn identity policy beyond the runtime-local carrier rule;
- Character identity semantics;
- Ability geometry/range/LoS/floor legality;
- Movement pathfinding/dynamic occupancy/visibility/speed;
- AI memory/threat/chase/flee policy;
- Combat/death/loot semantics;
- production deployment or orchestration.

## CROSS_DOMAIN_FINDINGS

```yaml
- id: RUNTIME-ACTOR-LOCALGEN-CROSS-01
  observed_in_domain: FND03_RUNTIME_ACTOR_CARRIER
  target_owner: PERF-01 / #162
  severity: P1
  evidence: "#537 RUNTIME-ACTOR-RL-01 = PERF_REFERENCE_CELL_REQUIRED; ADR-0009/PERF-01 backlog"
  conflict_or_gap: "Production Channel actor-slot cardinality M is still unselected."
  required_before: "RESOURCE_LIMITS_REGISTRY serialization and executable shared carrier acceptance"
  worker_action: REPORT_ONLY

- id: RUNTIME-ACTOR-LOCALGEN-CROSS-02
  observed_in_domain: FND03_RUNTIME_ACTOR_CARRIER
  target_owner: GAME-ABILITY-01 / #508
  severity: P2
  evidence: "#508 Phase A exact actor-resolution contract"
  conflict_or_gap: "Ability must consume the shared complete typed actor reference and cannot create a second actor registry or scalar-ID bridge."
  required_before: "ABILITY_EXACT_ACTOR_RESOLUTION_V1 implementation allocation"
  worker_action: REPORT_ONLY

- id: RUNTIME-ACTOR-LOCALGEN-CROSS-03
  observed_in_domain: FND03_RUNTIME_ACTOR_CARRIER
  target_owner: VSL-MOVE-01 / #139
  severity: P2
  evidence: "#530 first-child relationship to REFERENCE_LOCAL_STEP_STATIC_KERNEL/v1"
  conflict_or_gap: "Movement may consume current actor position only after the shared carrier is protected; this decision grants no Movement implementation authority."
  required_before: "#139 re-evaluation"
  worker_action: REPORT_ONLY

- id: RUNTIME-ACTOR-LOCALGEN-CROSS-04
  observed_in_domain: FND03_RUNTIME_ACTOR_CARRIER
  target_owner: OPS scope ownership / FND-03
  severity: P2
  evidence: "accepted scope ownership generation fencing"
  conflict_or_gap: "A carrier reset under the same ownership generation would invalidate the outer-incarnation safety argument."
  required_before: "runtime carrier implementation/recovery design"
  worker_action: REPORT_ONLY
```

## Required validation for the later implementation

The implementation allocation must prove at minimum:

1. exact current scope/local-id/local-generation resolves;
2. missing/vacant local slot rejects;
3. every `ActorLocalId` remains bound to exactly one logical slot/generation cell for the complete scope generation and distinct IDs cannot alias that cell;
4. reuse of one slot advances generation and the old ref rejects;
5. repeated same-slot churn does not grow retained generation cardinality;
6. retirement across every configured slot never creates a history entry beyond `M`;
7. local-generation successor overflow never wraps, atomically marks only that vacant slot `EXHAUSTED`, and the exhausted slot cannot be reused;
8. failed generation advance/admission leaves every existing actor unchanged and publishes no partial actor/index/current-reference state; the only permitted exhaustion mutation is the terminal slot-state transition in item 7;
9. a legitimate new `ScopeOwnershipGeneration` fences every prior local reference even if local IDs/generation representations are initialized anew;
10. resetting/reconstructing the local namespace under the same scope generation is rejected/fail-closed unless exact generation state and immutable logical bindings are restored;
11. wrong World, wrong Channel and stale scope generation reject before actor state exposure;
12. untyped AI/client/protocol-like scalars cannot call the authoritative resolver as complete actor refs;
13. no variable geometry/target collection or unbounded retirement structure is introduced.

Documentation-only candidate validation remains governance/semantic CI, whole-diff self-review and genuinely independent exact-head architecture review. Runtime/E2E is `NOT_APPLICABLE` to this paper-only decision and remains mandatory for the later executable carrier according to its allocated risk surface.

## Handoff

If this exact candidate is independently clean and protected-integrated, #162 may close the RL-03 architecture blocker in #530 as `SAME_RESOURCE_AS_RL01_FOR_CHANNEL_RUNTIME_ACTOR_CARRIER_V1` and move to the still-separate ADR-0009/PERF-01 reference-cell prerequisite.

This decision does not activate #508, #139, runtime implementation, registry mutation or production behavior.
