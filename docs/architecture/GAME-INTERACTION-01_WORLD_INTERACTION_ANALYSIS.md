# GAME-INTERACTION-01 — World Interaction Architecture Analysis

- Date: 2026-08-15
- Gate: `GAME-INTERACTION-01`
- Delivery task: `OTV2-20260815-game-interaction-architecture`
- Delivery PR: #269
- Worker: Domain Architecture Design Agent D
- Document role: architecture analysis supporting the candidate contract
- DecisionStatus: **PROPOSED**
- DeliveryStatus: **IN_REVIEW**
- ImplementationStatus: **NOT_STARTED**
- Runtime/client/protocol/DDL/Platform/production authority: **NONE**
- Merge authority: **ARCHITECTURE_COORDINATOR_ONLY**

## 1. Problem

Oteryn needs one safe architecture for world interactions without turning `use`, movement callbacks, scripts, map objects or asynchronous completions into alternate mutation engines.

The domain must cover at least:

- explicit player interaction/use intent;
- doors, switches and levers;
- teleports/portals and other relocation affordances;
- fields, traps and hazards;
- readable and writable objects;
- item-assisted use and use-with routing;
- movement/contact/enter/exit triggers;
- timer-triggered behavior;
- script-authored interaction proposals;
- Channel-local, Instance-local and world-shared behavior;
- reset, persistence, crash recovery and content-revision migration;
- deterministic ordering/replay and anti-abuse/resource bounds.

The central risk is authority overlap. One interaction may touch several already-owned domains: a locked door may inspect/consume an item, a teleporter changes placement, a trap may invoke damage/conditions, a writable object may require durable moderated text, and a world-shared lever may affect multiple Channels. GAME-INTERACTION therefore cannot be a generic API with authority to mutate arbitrary gameplay state.

## 2. Evidence classification

### PROVEN — accepted upstream constraints

The repository already establishes these constraints:

1. FND-03 gives each active Channel/Instance one current logical authoritative mutation owner and requires mutation-capable events, timers and asynchronous completions to re-enter that owner's normalized authoritative input boundary.
2. The multichannel scope model makes public-map mutable overlay state Channel-local by default and forbids implicit sharing between Channels.
3. GAME-CHANNEL-01 separates runtime multiplicity from world/durable policy and requires explicit scope whenever Channel multiplicity changes product/economy semantics.
4. GAME-ITEM-01 owns item definition/instance legality; item location is not item identity.
5. DUR-03 owns value location, conservation, item/currency transaction identity, idempotency, fencing and recovery.
6. DUR-04 requires immutable revision-bound deterministic content execution and proposal-only scripting; scripts receive no arbitrary database, filesystem, network, wall-clock, OS-RNG or mutable-world authority.
7. SIM-DETERMINISM-01 requires identical canonical state, ordered inputs and semantic revision set to yield identical authoritative results, including retry/failover behavior.
8. GAME-ABILITY-01 has owner-accepted partial baselines for server-authoritative targeting/legality and cast/channel/commit semantics; the overall GAME-ABILITY gate is still open.
9. `docs/contracts/RESOURCE_LIMITS_REGISTRY.json` requires an accepted numeric hard maximum for every externally controlled count, depth, length or byte-size dimension before implementation acceptance.

### DERIVED — consequences for GAME-INTERACTION

From those accepted constraints:

- interaction must be a bounded domain pipeline, not a generic arbitrary-action interpreter;
- client target identifiers, positions, verbs and item references are requests only;
- movement/timer/script callbacks cannot mutate authoritative object state outside FND-03 ordering;
- Channel-local object state is owned by the current ChannelRuntime; Instance-local state by the current InstanceRuntime;
- world-shared mutable interaction state requires a named world/domain owner rather than replicated mutable copies in each Channel;
- item consumption/move/transform cannot be hidden inside a door/switch handler;
- trap/field/hazard damage and conditions cannot become a second combat/effect engine;
- teleport activation cannot grant GAME-INTERACTION implicit authority over movement, handoff, Channel switching or Instance admission;
- asynchronous/delegated work is stale unless scope ownership generation, object/state revision and applicable semantic revisions remain valid when it re-enters authority;
- process restart or NodeId change does not itself reset semantic interaction state.

### UNKNOWN — evidence not sufficient to freeze

Current accepted sources do not establish:

- exact Reference Tibia `use`/`use with` target-priority rules for stacked objects;
- exact interaction distance, line-of-sight, floor or reachability rules;
- exact door/key consumption behavior for all Reference mechanics;
- exact teleport destination/fallback behavior;
- exact trap/field trigger precedence, cadence or effect semantics;
- the durable ownership/moderation/retention model for player-authored writable world text;
- numeric interaction rate, cascade, timer, target-query or write-payload limits;
- a canonical movement/relocation contract;
- a generic cross-domain atomic workflow spanning interaction state plus movement/item/value/ability state.

These remain `UNKNOWN`; this worker does not guess them.

## 3. Decision timing

### Must decide now? YES

A bounded interaction authority model is required before broad map/world gameplay implementation. Without it, content can accidentally harden incompatible mutation paths through explicit use handlers, movement hooks, timer callbacks, item scripts and general-purpose scripts.

### Downstream work blocked without a decision

- stateful doors/switches/levers;
- interaction-triggered teleports;
- interaction-triggered traps/fields/hazards;
- item-to-world-object use;
- persistent/resettable map mechanisms;
- safe scriptable world interactions;
- deterministic interaction replay/recovery;
- interaction authoring/validation in the content toolchain.

### Risks of deciding incorrectly

- unrestricted scripts/callbacks would create hidden cross-domain transactions and security authority;
- process-global mutable object state would violate multichannel isolation;
- committing local state before a required foreign mutation could produce unrecoverable partial outcomes;
- restart-as-reset semantics could duplicate/re-arm value-producing or player-impacting mechanics;
- unbounded target/cascade/timer work could turn ordinary interaction input into a resource-exhaustion surface.

### Evidence that may supersede this decision

- provenance-cleared Reference mechanic evidence;
- representative product/playtest evidence;
- deterministic replay/fault-injection findings;
- measured target/cascade performance evidence;
- security review findings;
- later accepted movement, GAME-ABILITY or owner-specific transaction/workflow contracts.

## 4. Ownership boundary

### RECOMMENDATION — GAME-INTERACTION owns

- normalization of an interaction attempt/trigger;
- bounded resolution of an **interactable world object** in the current authoritative runtime scope;
- interaction-specific affordance/current-state validation;
- typed, versioned interaction-object state machines;
- deterministic transition planning and local interaction-owned state commit;
- trigger/reset/semantic-timer semantics for interaction-owned state;
- bounded routing/delegation to other owners;
- interaction-specific failure classification, evidence requirements and anti-abuse/resource-limit obligations.

### GAME-INTERACTION must not own

- arbitrary character movement/position legality;
- ItemInstance location/value mutation;
- currency/value conservation;
- ability targeting, damage, healing, combat-condition formulas, immunity or cooldown semantics;
- foreign durable/account/character/guild/house/reward state;
- world-shared mutable state merely because a local interaction initiated it;
- persistent free-form player text without an accepted durable/moderation owner;
- AI decision behavior;
- raw database state or direct script mutation authority.

This is the smallest useful boundary that supports rich mechanics while preserving existing owners.

## 5. Options considered

### Option A — unrestricted generic `Use()` callback/script

```text
trigger -> find object -> object script mutates arbitrary state
```

**Tradeoff:** minimal short-term implementation effort and high authoring flexibility.

**Risks:** violates DUR-04, hides cross-domain transactions, weakens replay/recovery, encourages process-global state, makes limits hard to prove and can bypass GAME-ITEM/DUR-03/GAME-ABILITY.

**Decision:** REJECT.

### Option B — universal interaction transaction engine

```text
InteractionPlan -> atomically mutate object + player + item + ability + world-service state
```

**Tradeoff:** superficially simple common API.

**Risks:** becomes a second owner for nearly every gameplay domain, requires a broad global transaction surface, obscures recovery ownership and conflicts with accepted one-owner/value semantics.

**Decision:** REJECT.

### Option C — typed interaction state machine + bounded domain delegation

```text
normalized source occurrence
-> resolve current interactable object
-> validate affordance/current state
-> build bounded typed transition/delegation plan
-> commit interaction-owned state OR enter an explicit owner boundary/workflow
-> publish deterministic result/evidence
```

**Tradeoff:** more explicit contracts and content validation are required; coupled cross-owner mechanics need named workflows.

**Benefits:** aligns with FND-03/DUR-04/SIM, supports data-first authoring, preserves domain ownership and gives deterministic replay/recovery/security boundaries.

**RECOMMENDATION:** adopt Option C.

## 6. Source/origin taxonomy

All origins use the same authoritative pipeline; an origin changes allowed context/capabilities, not mutation authority.

### Explicit player interaction

Examples: use door, pull lever, read object, activate portal. Network input is intent only; authoritative actor/session/scope and target resolution are server-derived.

### Item-assisted interaction

Examples: key/tool/item used on a world object. Item current existence/location/eligibility remains GAME-ITEM/DUR-03-owned; GAME-INTERACTION may route the request but cannot directly consume/move/split/transform/mint/burn the item.

### Movement/contact trigger

Examples: pressure plate, hazard entry/exit/contact. Only a **committed authoritative movement occurrence** may create the interaction trigger. Raw client movement requests, renderer state and physics callbacks are insufficient.

### Semantic timer trigger

A due interaction timer becomes a new FND-03 normalized authoritative input. An OS timer callback cannot directly mutate state.

### Authorized system/world event

A named owner may invoke an explicitly allowed typed interaction capability with explicit scope/correlation evidence. There is no generic mutation event bus.

### Script proposal

A DUR-04 script may return a bounded proposal. The host independently revalidates target, capability, state revision, scope, semantic revisions and resource limits before any authoritative action.

### Future AI/NPC/creature intent

A future AI owner may submit a typed interaction intent when an accepted contract permits it. GAME-INTERACTION does not define AI behavior or decision policy.

## 7. Object/affordance taxonomy

| Capability family | Examples | Interaction-owned responsibility | Delegated responsibility |
|---|---|---|---|
| Stateful gate | door, barrier | typed local state transition/reset | movement/collision policy; item/value changes |
| Activator | switch, lever, pressure plate | trigger/state transition, typed routes | foreign-domain effects |
| Relocator affordance | teleporter, portal | activation and versioned destination descriptor | authoritative relocation/handoff/admission/switch |
| Contact hazard | trap, trigger tile | trigger + armed/reset/cooldown state | damage/condition/effect semantics |
| Area/field interaction | environmental area | interaction-owned entry/exit/contact state | combat/ability effects |
| Readable | sign/book-like world object | access + authoritative read routing | owner-managed mutable/item text |
| Writable | board/book-like object | access/input validation + typed write routing | persistent text/moderation/retention/storage |
| Item-routed use | key/tool/use-with | object-side affordance/routing | item legality/location/value/state mutation |
| Script-extended | special mechanism | host validation + owned state transition | every foreign-domain action through typed delegation |

Capability composition is permitted only when compiler validation can prove an unambiguous scope, owner, reset and transition model.

## 8. Target resolution versus GAME-ABILITY targeting

GAME-INTERACTION needs a bounded deterministic resolver for **interactable world objects**, not a second GAME-ABILITY target resolver.

Required properties:

- client tile/object/entity references are hints only;
- current authoritative Channel/Instance scope bounds target discovery unless a named cross-scope owner participates;
- candidate enumeration and result cardinality have hard limits before implementation acceptance;
- stable deterministic ordering/tie-breaking is required where multiple candidates are eligible;
- ordering cannot depend on hash iteration, memory address, thread completion or unspecified database order;
- Reference stacked-object priority remains `UNKNOWN` pending evidence;
- if an interaction delegates an ability, GAME-ABILITY performs its own authoritative targeting/legality flow.

## 9. State, scope and lifetime

### Authority scope

Recommended classes:

- `CHANNEL_LOCAL` — current ChannelRuntime owns mutable state;
- `INSTANCE_LOCAL` — current InstanceRuntime owns mutable state;
- `WORLD_SHARED_DELEGATED` — a named world/domain owner owns state; Channels consume typed versioned results;
- `FOREIGN_DOMAIN_DELEGATED` — Character/Account/Item/Guild/House/Reward/etc. remains with its existing owner.

There is deliberately no generic unowned `GLOBAL` mutable interaction class.

### Lifetime/durability

Recommended classes:

- `STATELESS` — no interaction-owned mutable history;
- `RUNTIME_EPHEMERAL` — may reset only at a declared fresh semantic initialization boundary;
- `RUNTIME_RECOVERABLE` — state/timers/pending operations affecting future outcomes survive same-scope process recovery through replay/checkpoint/reconstruction evidence;
- `DURABLE_DELEGATED` — a named durable owner persists/reconciles state.

A new process or NodeId is not by itself a fresh semantic initialization boundary.

## 10. Reset and timer semantics

Reset is authoritative mutation, not an implementation detail.

Candidate reset classes:

- `NO_AUTOMATIC_RESET`;
- `AFTER_DURATION`;
- `ON_FRESH_SCOPE_INITIALIZATION`;
- `ON_NAMED_AUTHORITY_EVENT`;
- `DELEGATED_RESET_POLICY`.

For duration-based reset, content/policy must state whether downtime counts. If semantics survive process lifetime, opaque process-local monotonic instants are insufficient; semantic timing evidence must be recoverable under FND-03.

A restart/recovery must not open/close a door, re-arm a trap or reset a switch merely because process-local memory was lost.

A due timer re-enters as normalized authoritative input and is revalidated against current object state, semantic revision and ownership generation. A stale timer commits nothing.

## 11. Cross-domain atomicity

### Local-only transition

When one current runtime owner owns every mutated interaction state component, a validated transition can commit non-interleaved inside one FND-03 authoritative input.

### Delegated-only operation

If GAME-INTERACTION owns no mutation requiring local commit, it validates/routs a typed request to the real owner and consumes that owner's authoritative result.

### Coupled local + foreign mutation

Examples:

- consume a key **and** open a door;
- charge value **and** teleport;
- disarm a trap **and** mutate an ItemInstance;
- local mechanism **and** world-shared state transition.

If product semantics require one coupled outcome, the mechanic requires a **named owner-specific workflow** that defines operation identity, idempotency, fencing/revisions, prepare/commit or equivalent boundary, timeout/retry/ambiguity reconciliation and crash recovery. Without such a workflow, fail closed rather than commit a partial outcome.

If semantics intentionally permit independent outcomes, that independence must be explicit, versioned policy — never accidental dependency-failure behavior.

A generic cross-domain interaction transaction escape hatch is rejected.

## 12. Concurrency and stale work

Mutable interaction state needs an owner-local monotonically advancing state revision or equivalent current-state fence.

Consequences:

- two players using the same Channel-local lever are ordered by FND-03; the second observes the state committed by the first;
- a script/dependency result prepared against an old object revision is revalidated and rejected/conflicted when stale;
- completion from an older scope ownership generation cannot commit;
- retry of the same CommandRef cannot apply the effect twice;
- state revision is a fence, not global identity or credential.

## 13. Trigger cascades and re-entrancy

World mechanisms may chain. Unbounded synchronous recursion is not acceptable.

Required design:

- typed compiler-visible trigger edges;
- bounded deterministic work list rather than recursive callbacks;
- one root source/correlation reference;
- deterministic step/edge ordering;
- hard maxima for depth, total actions/steps and fan-out;
- explicit duplicate/visited-edge behavior;
- a cross-owner/domain edge ends synchronous local mutation authority and becomes typed delegation;
- scripts cannot create private event buses or recursive arbitrary interaction calls.

Exact numeric maxima are not selected here.

## 14. Movement-triggered interaction

Required ordering:

```text
movement owner commits authoritative movement occurrence
-> typed contact/enter/exit occurrence
-> FND-03 interaction input
-> deterministic interaction resolution/transition/delegation
```

The committed movement occurrence needs stable replay/retry correlation so duplicate delivery does not trigger one semantic occurrence twice.

GAME-INTERACTION does not decide pathfinding, collision, exact step order or the persistence model for actor position.

## 15. Teleport/relocation

A teleporter is an interaction affordance, not permission to rewrite character placement.

GAME-INTERACTION may:

- validate the active portal/teleporter object;
- resolve a versioned destination descriptor from content;
- evaluate interaction-owned preconditions;
- issue a typed relocation request with exact semantic revision context.

It may not:

- silently choose another Channel/destination;
- bypass Channel-switch locks/cooldown/admission;
- create an Instance/GameSession/CharacterLease;
- relocate across authority boundaries without the owning handoff/admission contract;
- charge item/currency value directly.

Exact movement/relocation owner/API remains a cross-domain gap.

## 16. Traps, fields and hazards

GAME-INTERACTION may own:

- armed/disarmed/one-shot/repeatable trigger state;
- trap-local cooldown/reset state;
- contact/entry occurrence handling from committed movement;
- typed downstream effect request.

Damage, healing, conditions, immunity, PvP/PZ legality and effect formulas remain GAME-ABILITY/combat-owned.

When trap-state transition and effect application must be one semantic all-or-nothing outcome, a named coupled workflow is required; this analysis does not invent one.

## 17. Readable/writable objects

### Readable

Authored immutable text comes from the exact content revision; mutable/domain-owned text comes from its owner. The server produces the authoritative read result. Result size/count remains bounded.

### Writable

GAME-INTERACTION can own only the interaction-side boundary unless a later accepted contract assigns more:

- target/access/affordance validation;
- encoding/normalization and resource-limit validation;
- typed write request routing;
- structured result.

Persistent free-form text ownership, moderation, privacy, retention, search/indexing and storage schema require a named durable owner. A Channel-local transient writable mechanic is possible only when explicitly declared ephemeral/non-durable.

## 18. Item-assisted routing

Recommended flow:

```text
player CommandRef
-> normalize item-assisted interaction intent
-> resolve/validate current world target
-> obtain current item eligibility/location evidence from GAME-ITEM/DUR-03 authority
-> build bounded object-side transition/delegation plan
-> commit through the correct owner/workflow
```

Interaction definitions/scripts cannot directly set item owner/location/count/durability/binding/currency value. If an item invokes an ability, the downstream invocation enters GAME-ABILITY.

## 19. Script capability boundary

DUR-04 remains the outer security model. GAME-INTERACTION should expose only typed capabilities that let a script:

- inspect explicitly allowed immutable interaction snapshot fields;
- propose registered interaction state transitions;
- propose an allowlisted typed domain route;
- use deterministic runtime-provided RNG only when explicitly authorized by content and SIM purpose-stream semantics.

Scripts must not receive unrestricted world iteration, mutable references, DB handles, arbitrary command/event dispatch, network/filesystem/process/env/secrets, wall clock, OS RNG, direct movement/item/value/ability mutation, implicit world-shared ownership, private persistent VM globals, private unbounded timers or recursive event loops.

Trap, invalid proposal or resource exhaustion commits no authoritative plan by default.

## 20. Content revision and migration

Interaction behavior is behavior-affecting content and must be immutable-revision-bound.

Pending timers/operations/retries retain enough evidence to identify the exact definition/ruleset/SIM inputs used for behavior. Activation follows DUR-04 compatibility classes:

1. compatible/no migration;
2. read-compatible with explicit normalization;
3. explicit migration;
4. incompatible;
5. removed/tombstoned policy.

A pending occurrence is not silently reinterpreted under a new definition. Incompatible activation blocks rather than resetting unknown state. Removing an object with recoverable/durable/coupled state requires explicit retirement/migration semantics.

## 21. Recovery

### Crash before local commit

No interaction mutation exists; retry/recovery follows the source occurrence's duplicate semantics.

### Crash after local commit before response

Committed state/revision is truth; same CommandRef/source occurrence reconciles instead of applying again.

### Crash while delegated work is pending

Recoverable evidence must distinguish at least whether an operation was not submitted, submitted/pending, committed at the foreign owner but not yet observed, or terminal/cancelled. Absence of a response is never interpreted as proof of non-commit.

Required evidence includes as applicable:

- semantic scope and owner-generation context;
- interaction object/state revision;
- operation/source correlation identity;
- exact behavior-affecting semantic revisions;
- committed/reserved/pending state needed by the owning workflow.

### Scope-owner replacement

New NodeId does not reset interaction state. Old-generation timers/completions are stale and cannot commit.

### World-owner unavailable

A Channel-local cache cannot become temporary world authority. The interaction follows explicit dependency failure/pending policy or fails closed.

## 22. Failure categories

GAME-INTERACTION should reuse foundation categories:

| Situation | Foundation category direction |
|---|---|
| malformed verb/payload/target encoding | `INVALID_INPUT` |
| incompatible interaction/content/ruleset revision | `UNSUPPORTED_REVISION` |
| source session no longer valid | `SESSION_REJECTED` |
| stale runtime/object/generation/revision fence | `STALE_GENERATION` |
| current state prevents requested transition | `CONFLICT` |
| candidate/cascade/payload/timer/pending/rate bound hit | `CAPACITY_EXCEEDED` |
| required world/item/movement/ability owner unavailable | `DEPENDENCY_UNAVAILABLE` |
| named delegated workflow deadline expires | `TIMEOUT` |
| explicit pending operation cancelled | `CANCELLED` |
| unexpected state cannot be handled safely | `INTERNAL_UNAVAILABLE` |

Narrow domain codes may be added later but must map to a foundation category and state retry/idempotency/partial-mutation outcome.

## 23. Resource and anti-abuse analysis

Before executable implementation acceptance, measured numeric limits must be registered for at least:

- interaction-domain payload bytes where a stricter bound than generic FND-02 is required;
- player/session/actor interaction work rate or scheduling budget;
- target candidate enumeration and target-result count;
- writable input bytes/codepoints and normalization expansion;
- transition actions per plan;
- cascade depth, total steps/actions and fan-out;
- timers per object/runtime scope;
- pending delegated/coupled operations per object/actor/scope;
- interaction-specific script proposals/host calls in addition to DUR-04 outer bounds;
- retained recovery/evidence size per pending occurrence;
- dependency/world query result count;
- state-machine states/transitions/edges where pathological content can amplify compiler/runtime work.

These bounds defend against interaction spam, forged cross-scope references, stale-ref replay, duplicate movement-trigger farming, recursive trigger bombs, script amplification, oversized writable text, timer storms and world-shared hotspots.

No numeric maximum is invented by this worker; the shared resource registry is outside Agent D's owned paths.

## 24. Deterministic acceptance scenarios

A future implementation/proof package should exercise at least:

1. two players use one Channel-local lever -> FND-03 produces one reproducible transition order;
2. retry same player CommandRef -> no duplicate transition/effect;
3. duplicate delivery of one movement occurrence to one-shot trap -> no duplicate semantic trigger;
4. reset timer and manual interaction race -> FND-03 owner order is authoritative, not wall-clock/thread order;
5. script traps/exhausts resource -> no authoritative plan commits;
6. script proposes direct item/value/movement/ability mutation -> rejected, no default partial local commit;
7. stale/moved key/item evidence -> item owner rejects; no coupled door success without a named workflow;
8. same `CHANNEL_LOCAL` lever in two Channels -> state evolves independently;
9. `WORLD_SHARED_DELEGATED` lever used from two Channels -> one named world owner orders state;
10. dependency response returns to superseded runtime generation -> stale completion cannot commit;
11. crash after local commit before response -> recovery reconciles committed revision and does not apply again;
12. content revision changes while timer/operation is pending -> no silent reinterpretation;
13. writable payload exceeds hard bound -> reject before retained allocation/persistence;
14. cascade exceeds hard bound -> deterministic bounded failure, no unbounded recursion;
15. teleport destination unavailable/incompatible -> no silent alternate Channel/destination and no unrelated value consumption;
16. split Channel ownership -> only current generation can commit interaction state;
17. world-shared owner unavailable -> Channel does not become temporary authority;
18. same-scope process replacement -> `RUNTIME_RECOVERABLE` door/trap/switch does not reset merely because NodeId changed.

Architecture specifies these invariants; executable evidence remains `NOT_STARTED`.

## 25. Applicable foundation failure scenarios

At minimum:

- `FS-STALE-GENERATION`;
- `FS-DUPLICATE-COMMAND`;
- `FS-CHANNEL-SPLIT-OWNER`;
- `FS-QUEUE-SATURATION`;
- `FS-CLOCK-SKEW` for timer interpretation;
- `FS-REVISION-MISMATCH`;
- `FS-WORLD-BUNDLE-CORRUPT` before activation;
- `FS-POSTGRES-UNAVAILABLE` when a durable owner is required;
- `FS-DB-OUTBOX-BOUNDARY` when an owning durable workflow requires outbox evidence.

This paper-only package does not claim runtime PASS evidence for those scenarios.

## 26. Recommended architecture

```text
SOURCE OCCURRENCE
(player CommandRef / committed movement occurrence / semantic timer /
authorized system event / validated script proposal)
        |
        v
FND-03 NORMALIZED AUTHORITATIVE INPUT
        |
        v
INTERACTION INTENT NORMALIZATION
        |
        v
BOUNDED WORLD-OBJECT RESOLUTION
        |
        v
SCOPE + CURRENT STATE + AFFORDANCE VALIDATION
        |
        v
BOUNDED TYPED INTERACTION PLAN
        |
        +--> interaction-owned local transition
        |
        +--> typed delegated owner operation
        |
        `--> named coupled workflow when semantics cross owners atomically
        |
        v
COMMIT / PENDING / STRUCTURED FAILURE
        |
        v
STATE REVISION + RESULT/EVIDENCE
```

No raw callback or script receives direct mutation authority.

## 27. Future impact

If accepted, this design provides a stable semantic foundation for:

- a shared headless interaction validator/compiler/runtime model;
- World Project authoring of typed interaction definitions;
- deterministic local door/switch/lever behavior;
- bounded script-host capabilities;
- interaction replay/fault fixtures;
- later movement/ability/item/value owner integrations without re-owning them.

It intentionally does **not** authorize runtime implementation. Movement relocation, effect-bearing traps, persistent writable text, coupled multi-owner workflows and numeric limits remain follow-up owner work.

## 28. DECISIONS_NOT_TAKEN

This analysis deliberately does not decide:

- Reference-specific `use`/`use with` target ordering or stacked-object priority;
- interaction range/LoS/floor/reachability algorithms/values;
- exact Reference key/door, switch, lever, trap, field, teleport or reset behavior;
- movement/pathfinding/collision implementation;
- teleport cross-scope handoff API/admission details;
- GAME-ABILITY damage/condition/effect semantics;
- AI behavior;
- item/location/value transaction semantics already owned by GAME-ITEM/DUR-03;
- generic cross-domain ACID transaction machinery;
- persistent writable-text owner/schema/moderation/privacy/retention policy;
- physical content authoring format, WIT or Rust types;
- scheduler/thread/async implementation;
- exact numeric hard limits, rate windows, timeouts or reset durations;
- client UI/presentation/protocol payload layout;
- PostgreSQL DDL/migrations;
- global architecture/status overlays;
- runtime implementation or production activation.

## 29. CROSS_DOMAIN_FINDINGS

```yaml
cross_domain_finding:
  id: GAME-INTERACTION-01-XD-ABILITY-TRIGGER
  observed_in_domain: GAME-INTERACTION-01
  target_owner: GAME-ABILITY-01
  severity: P1
  evidence: GAME-ABILITY-01 targeting/legality and cast/channel/commit owner-accepted partial baselines; overall GAME-ABILITY-01 remains open
  conflict_or_gap: Interaction-triggered traps/fields/hazards require a stable typed non-player-origin ability/effect invocation and coupled-failure semantics without allowing GAME-INTERACTION to become combat/effect authority.
  required_before: executable trap/field/hazard implementation claiming ability/combat effects
  worker_action: REPORT_ONLY
```

```yaml
cross_domain_finding:
  id: GAME-INTERACTION-01-XD-MOVEMENT-RELOCATION
  observed_in_domain: GAME-INTERACTION-01
  target_owner: ARCHITECTURE_COORDINATOR
  severity: P1
  evidence: FND-03 owns runtime execution scope and GAME-CHANNEL-01 owns Channel switching, but this worker allocation has no canonical movement/relocation contract
  conflict_or_gap: Teleport/portal activation and movement-derived triggers need an exact authoritative movement occurrence/relocation/handoff owner surface that GAME-INTERACTION cannot invent.
  required_before: executable teleport/portal implementation and movement-trigger integration
  worker_action: REPORT_ONLY
```

```yaml
cross_domain_finding:
  id: GAME-INTERACTION-01-XD-WRITABLE-TEXT
  observed_in_domain: GAME-INTERACTION-01
  target_owner: ARCHITECTURE_COORDINATOR
  severity: P2
  evidence: reviewed GAME-ITEM-01 and DUR-04 contracts do not assign a complete persistent player-authored world-text moderation/retention owner
  conflict_or_gap: Durable writable boards/books/signs need explicit durable ownership, moderation/privacy/retention and migration semantics; GAME-INTERACTION can own access/routing but should not create that authority implicitly.
  required_before: persistent player-authored writable-world-object implementation
  worker_action: REPORT_ONLY
```

```yaml
cross_domain_finding:
  id: GAME-INTERACTION-01-XD-COUPLED-WORKFLOW
  observed_in_domain: GAME-INTERACTION-01
  target_owner: ARCHITECTURE_COORDINATOR
  severity: P1
  evidence: DUR-03 owns item/value transaction semantics and DUR-04 forbids generic cross-authority script transaction escape hatches
  conflict_or_gap: Mechanics requiring one semantic outcome across interaction-local state plus movement/item/value/ability/world authority require a named owner-specific operation/workflow; no generic cross-domain interaction transaction is accepted.
  required_before: consume-key-and-open, charge-and-teleport or equivalent coupled multi-owner implementation
  worker_action: REPORT_ONLY
```

```yaml
cross_domain_finding:
  id: GAME-INTERACTION-01-XD-RESOURCE-LIMITS
  observed_in_domain: GAME-INTERACTION-01
  target_owner: ARCHITECTURE_COORDINATOR
  severity: P1
  evidence: docs/contracts/RESOURCE_LIMITS_REGISTRY.json requires registered hard maxima before implementation acceptance; Agent D does not own that shared registry
  conflict_or_gap: Interaction-specific target enumeration, cascade, writable-payload, timer, pending-operation and rate ceilings still require measured numeric values and registry entries.
  required_before: GAME-INTERACTION implementation acceptance
  worker_action: REPORT_ONLY
```

## 30. Final analysis recommendation

**RECOMMENDATION:** freeze a bounded server-authoritative world-interaction contract based on typed interaction state machines and explicit owner delegation. Permit rich content through immutable data and bounded DUR-04 script proposals, but never allow a world-object handler to become authority over arbitrary movement, item/value, ability/combat or world-shared state.

The accompanying candidate contract can be architecture-complete while intentionally keeping Reference-specific behavior, movement-owner details, persistent writable text, multi-owner workflows and numeric limits unresolved/fail-closed for their proper owners.

MERGE_AUTHORITY: ARCHITECTURE_COORDINATOR_ONLY
