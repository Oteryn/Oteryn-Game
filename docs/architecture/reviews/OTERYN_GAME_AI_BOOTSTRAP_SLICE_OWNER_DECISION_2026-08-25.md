# Oteryn Game AI Bootstrap Slice Owner Decision

- Status: `OWNER_ACCEPTED ARCHITECTURE DECISION` after merge to protected `main`
- Date: 2026-08-25
- Repository: `Oteryn/Oteryn-Game`
- Governing escalation: Issue #164
- Parent programme: Issue #162 (`Oteryn: work coordinator`)
- Authoring base: `main@7ac06bd84a1a31fc9a3ea2560de8ae20cea96741`
- Owner/Supervising Architect durable disposition: Issue #164 comment `5417284319`
- Parent architecture gate: `GAME-AI-01`
- New global gate ID: `NONE`
- Implementation authority granted by this decision: `NONE`
- Production authority: `NONE`

## 1. Decision timing

**Must decide now?** `YES`, but only for the first executable structural AI slice.

**Concrete downstream work blocked without this decision:** the Work coordinator cannot lawfully allocate `OTV2-IMPL-AI` from Issue #164 because the exact executable slice was not separated from later AI capabilities and cross-domain integration obligations.

**What becomes harder later if decided too broadly now:** accepting concrete AI representation, production pathfinding, spawn policy, cross-domain Ability/Interaction APIs or Reference behavior before executable evidence exists would create avoidable schema/API migration, test coupling and ownership ambiguity.

**Evidence that may justify superseding or extending this decision:** merged Interaction/Ability implementations, a compatible Movement/runtime owner seam, real dynamic-route-invalidating Interaction requirements, representative creature behavior/content evidence, pathfinding benchmarks, production load evidence, or a later named AI/spawn product slice whose resource dimensions differ from this bootstrap.

**Deliberately not decided:** behavior tree/FSM/statechart/framework/library, production pathfinding algorithm/library, production AI scheduler, spawn/population semantics, permanent AI/content schema, dynamic Interaction route invalidation API, Ability action/result API integration, Reference aggro/path/spawn behavior, controlled actors, scripts, durable encounter ownership, reward/value semantics and production wiring.

## 2. Reconciliation of existing authority

The historical file `docs/architecture/GAME-AI-01_CREATURE_AI_SPAWN_PATHFINDING_CONTRACT_CANDIDATE.md` remains a proposal artifact and MUST retain its historical `PROPOSED` / `NONCANONICAL WORKER PROPOSAL` status.

That historical header does **not** mean the `GAME-AI-01` gate is unaccepted. The later owner-acceptance baseline `docs/architecture/OTERYN_V2_REMAINING_FIRST_WAVE_OWNER_ACCEPTANCE_BASELINE_20260816.md` accepted the semantic scope delivered through PR #276 and records `GAME-AI-01: DecisionStatus: ACCEPTED`. Binding accepted semantics include current ChannelRuntime/InstanceRuntime ownership, representation-neutral deterministic bounded resolution, staged all-or-nothing AI-local commit/rejection, bounded path proposal work with owner revalidation, deterministic target/perception selection, foreign-domain proposal boundaries, stable provenance/recovery and fail-closed Reference uncertainty.

This decision therefore does not accept the historical candidate wholesale and does not reopen the global `GAME-AI-01` gate. It selects one executable subset of already accepted semantics and binds that subset to already accepted/registered first-slice resource ceilings.

## 3. Selected first executable slice

The selected slice is the **pure-local GAME-AI-01 bootstrap v1**.

Its purpose is to produce executable evidence for AI determinism, bounded work, provenance/fencing and proposal safety without prematurely integrating AI with production gameplay domains.

One admitted semantic resolution is equivalent to:

```text
exact owner/generation/revision-bound immutable AI snapshot
+ bounded canonical perception candidates
+ one bounded local AI occurrence
-> finite deterministic evaluation
-> IDLE or ACQUIRE_CANDIDATE(target)
-> optionally at most one bounded PATH_PROPOSAL
-> no foreign-domain mutation
```

`ACQUIRE_CANDIDATE(target)` is an AI-local normalized decision result only. It is **not** an Ability/Combat intent, Movement command, durable target lock, threat entry or reward/contribution fact.

`PATH_PROPOSAL` is auxiliary proposal evidence only. It has no authority to move an actor, mutate occupancy, change Interaction state or publish gameplay state. A proposal is usable only by a later owning integration boundary after fresh owner/generation/revision and domain legality revalidation.

The bootstrap MAY use a private reversible deterministic search implementation or fixture representation to produce bounded proposal evidence. That implementation detail is not a public/canonical algorithm choice, must not leak into protocol/content/persistence schemas, and may be replaced later without semantic migration.

No authoritative AI-local mutation is required merely to produce these normalized results. If an implementation introduces any AI-local mutable bookkeeping, it MUST remain bounded, owner-local, non-product-semantic, and obey the already accepted staged/preflight all-or-nothing commit-or-reject rule. This decision does not authorize a new retained threat/target/memory collection.

## 4. Admission inputs and provenance

Every bootstrap resolution must bind enough exact immutable/revalidatable context to reject stale or semantically incompatible work. As applicable to the local fixture, this includes:

- authoritative scope identity and current ownership generation;
- actor semantic identity and actor-local generation/fence;
- exact behavior/template revision;
- exact content/world/map/navigation revision used by the fixture;
- ruleset/SIM determinism profile revision;
- stable logical occurrence/work identity for deterministic ordering/RNG isolation where RNG is used;
- bounded perception candidates canonicalized by stable semantic identity before order-sensitive evaluation.

Pointer address, hash-map iteration order, thread/worker identity, wall-clock order or mutable `latest` lookup must not influence the semantic result.

Dynamic environmental facts owned by GAME-INTERACTION are not admitted into this bootstrap slice. If route validity would depend on a door, teleport, mutable environmental interaction or other dynamic Interaction-owned fact, that route request is outside this slice and must fail closed rather than inventing route-invalidation semantics.

## 5. Resource profile

No new resource value is selected by this decision. The canonical `docs/contracts/RESOURCE_LIMITS_REGISTRY.json` remains the numeric authority.

The bootstrap may exercise only the already accepted first-slice AI resources below:

| Resource ID | Hard maximum | Bootstrap rule |
|---|---:|---|
| `AI01-ACTIVE-ACTORS` | 256 actors/scope | no wider scope envelope |
| `AI01-AUTHORED-UNITS` | 4 authored units | fixed acquire-or-idle representation only |
| `AI01-EVALUATION-WORK` | 8 work units/resolution | complete rejection on max+1 |
| `AI01-PERCEPTION-CANDIDATES` | 64 candidates/decision | canonicalize before order-sensitive evaluation |
| `AI01-PATH-REQUESTS-PER-ACTOR` | 2 requests/actor | bootstrap configuration is narrower: at most one proposal request per actor occurrence |
| `AI01-PATH-SEARCH-WORK` | 1024 work units/request | bounded termination required |
| `AI01-ROUTE-STEPS` | 128 steps/proposal | max+1 rejected before proposal publication |
| `AI01-ROUTE-BYTES` | 4096 bytes/proposal | checked accounting before retain/publish |

Configuration may be narrower than the hard maximum and must never exceed it. Existing registry failure semantics apply; max+1 and arithmetic overflow reject before unchecked allocation, accepted proposal publication or partial authoritative mutation.

The following dimensions remain explicitly unreachable/fail-closed in this slice and therefore require no new numeric decision now:

- AI memory/threat/stimulus collections (`AI-RL-04`);
- AI timers/delayed operations (`AI-RL-06`);
- repath/retry windows (`AI-RL-10`);
- spawn sources/population mutation (`AI-RL-11`);
- spawn placement search (`AI-RL-12`);
- postponed occupancy retry (`AI-RL-13`);
- controlled-actor command backlog (`AI-RL-14`);
- script-backed AI (`AI-RL-15`);
- variable/amplification-prone replay or diagnostic payloads (`AI-RL-16`; fixed counters only).

If any implementation makes one of these dimensions reachable, that candidate is outside this decision and must stop for a fresh resource/architecture decision before merge.

## 6. Cross-domain boundaries

### Ability / Combat

`GAME-AI-XD-01` remains deferred for this bootstrap. The slice emits no Ability action intent and consumes no Ability result. `ACQUIRE_CANDIDATE` cannot damage, heal, cast, consume cooldown/charge state, create effects or establish combat legality.

After a compatible Ability implementation is merged, a separate AI Action Integration slice may define/consume the real typed Ability-owned intent/result boundary. This bootstrap must not predict that API.

### Interaction

`GAME-AI-XD-02` remains deferred for this bootstrap. No door/teleport/environmental mutation or dynamic route-invalidation API is consumed or invented. Path proposal evidence is limited to immutable fixture/navigation facts with exact revision binding; stale revision/generation evidence is rejected.

A later dynamic path-adoption slice requires the accepted Interaction-owned normalized invalidation/environment facts before it can become executable.

### Movement

AI never adopts its own path proposal. No actor position, occupancy or visibility mutation is authorized. Movement remains with its own owner and Issue #139/resource/readiness lifecycle.

### Durability / reward / value

No persistence, item/value custody, loot, XP, currency, reward eligibility, contribution attribution or durable encounter occurrence is in scope.

## 7. Required failure behavior

The bootstrap must fail closed on at least:

- owner or actor generation mismatch;
- incompatible behavior/content/map/navigation/ruleset/SIM revisions;
- malformed or non-canonical candidate input;
- any exercised resource max+1 or checked-arithmetic overflow;
- evaluation budget exhaustion before a complete result;
- path capacity/search/route bound exhaustion;
- stale or superseded path proposal work;
- any attempted foreign-domain direct mutation;
- any attempt to enable an excluded resource dimension.

A rejected or over-budget resolution publishes no accepted action/path proposal from the rejected staged result and commits zero product-semantic AI-local mutation from that resolution.

## 8. Implementation evidence required before merge

A future `OTV2-IMPL-AI` allocation implementing this decision must prove on its exact final head:

1. identical accepted snapshot/input/revisions produce identical result under shuffled backing enumeration order;
2. stable tie-break behavior for equivalent candidate scores/priority;
3. every exercised hard maximum accepts at max and rejects max+1 before partial mutation/proposal publication;
4. checked arithmetic overflow rejects deterministically;
5. evaluation/search budget exhaustion leaves zero product-semantic AI-local partial mutation;
6. path proposal output is bounded and deterministic for identical fixture input/profile;
7. owner/actor generation or relevant revision change rejects stale proposal work;
8. no code path in the allocated slice directly mutates Movement position/occupancy, Ability effects, Interaction state, persistence or value/reward state;
9. excluded memory/timers/repath/spawn/controlled/script/variable-diagnostic dimensions remain unreachable;
10. focused tests, full applicable workspace checks, whole-diff review and exact-head `game-gate` pass.

This architecture decision itself changes no runtime, Cargo/workspace, registry, workflow, protocol, stable IDs, production configuration or external repository.

## 9. Work coordinator handoff

After this decision is merged to protected `main`, Issue #164 may be closed as architecture-resolved. That closeout does **not** start implementation.

The Work coordinator may then create one fresh exact AI implementation allocation only if live preflight still proves:

- this decision is canonical on current `main`;
- the selected worker paths are disjoint from active #165/#166/#167 ownership;
- any shared composition/Cargo/workspace surface remains under the coordinator's serialized lease;
- no excluded behavior has been added to the proposed AI child slice;
- the allocation names exact base SHA, owned paths, tests, exclusions and the resource IDs above.

The worker must not consume sibling branch output before it is merged and explicitly admitted as a dependency.

## 10. Decision result

```yaml
gate: GAME-AI-01
global_gate_state: ACCEPTED
slice: pure-local bootstrap v1
slice_decision: ACCEPTED
implementation_status: NOT_STARTED
implementation_authority: NONE_UNTIL_SEPARATE_WORK_ALLOCATION
production_authority: NONE
new_resource_limits: NONE
registry_mutation: NONE
ability_integration: DEFERRED
interaction_dynamic_route_invalidation: DEFERRED
movement_adoption: FORBIDDEN_IN_BOOTSTRAP
spawn: EXCLUDED
persistent_threat_memory: EXCLUDED
timers_repath_retry: EXCLUDED
controlled_actors: EXCLUDED
scripts: EXCLUDED
reference_parity_claim: FORBIDDEN
```

This resolves the architectural ambiguity raised by Issue #164 while preserving all broader `GAME-AI-01` extension points.