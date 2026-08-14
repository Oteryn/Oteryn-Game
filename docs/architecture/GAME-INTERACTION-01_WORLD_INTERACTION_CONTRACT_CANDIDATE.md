# GAME-INTERACTION-01 — World Interaction Contract Candidate

- Date: 2026-08-15
- Gate: `GAME-INTERACTION-01`
- Delivery task: `OTV2-20260815-game-interaction-architecture`
- Delivery PR: #269
- DecisionStatus: **CANDIDATE**
- DeliveryStatus: **IN_REVIEW**
- ImplementationStatus: **NOT_STARTED**
- Canonical semantic effect: **NONE until Architecture Coordinator audit, accepted integration and merge**
- Runtime/client/protocol/DDL/Platform/production authority: **NONE**
- Merge authority: **ARCHITECTURE_COORDINATOR_ONLY**
- Analysis source: `GAME-INTERACTION-01_WORLD_INTERACTION_ANALYSIS.md`

## 1. Purpose

This candidate freezes the minimum architecture for deterministic, server-authoritative world interaction without granting world objects, callbacks or scripts generic gameplay mutation authority.

It covers:

- explicit player use/interact intent;
- item-assisted interaction routing;
- doors, switches, levers and stateful mechanisms;
- teleport/portal/relocation affordances;
- fields, traps and hazards as interaction triggers;
- readable and writable world objects;
- movement/contact/enter/exit triggers;
- semantic timer-triggered interactions;
- bounded script extensions;
- interaction-object scope/state/reset/persistence/recovery;
- content-revision migration;
- deterministic ordering/replay;
- cross-domain delegation and atomicity boundaries;
- anti-abuse and resource-limit requirements.

This candidate is a **semantic contract**, not a Rust API, wire schema, database schema, Studio format or production configuration.

## 2. Authority chain

```text
foundation identities / protocol command ordering -> FND-ID-01 / FND-02
runtime scope ownership/order/timers/recovery       -> FND-03
World/Channel/Instance product scope               -> GAME-CHANNEL-01
item definition/instance/equipment legality        -> GAME-ITEM-01
item/currency/value location + transactions        -> DUR-03
content revisions + script capability boundary     -> DUR-04
deterministic arithmetic/RNG/order/replay           -> SIM-DETERMINISM-01
ability targeting/effect lifecycle                 -> GAME-ABILITY-01 owner baselines / later gate closure
world interaction state/trigger/routing             -> GAME-INTERACTION-01 candidate
movement/relocation exact domain contract           -> unresolved owner; coordinator action required
```

No downstream convenience layer may redefine another owner's semantic authority.

## 3. Binding candidate invariants

If accepted, GAME-INTERACTION-01 requires all of the following:

1. **Server authority:** client interaction fields are intent only; current authoritative state decides target, legality, transition and result.
2. **One runtime owner:** Channel-local interaction state mutates only through the current `ChannelRuntime`; Instance-local interaction state only through the current `InstanceRuntime`.
3. **No implicit world global:** world-shared mutable interaction state has a named world/domain owner and typed delegation. There is no process-global mutable interaction singleton or generic `GLOBAL` scope.
4. **One ordered ingress:** every player, movement, timer, system, script or asynchronous interaction occurrence enters the current owner through FND-03 normalized authoritative input before mutation.
5. **Deterministic resolution:** target selection, transition evaluation, cascades and relevant tie-breaking are bounded and deterministic under SIM-DETERMINISM-01.
6. **Typed state:** interaction mutable state is versioned/typed. Arbitrary JSON/EAV/script-owned mutable blobs are not the authoritative model.
7. **Explicit delegation:** movement, ItemInstance/value, ability/combat and other foreign-domain mutation occurs only through the typed owner boundary.
8. **No generic multi-owner transaction:** GAME-INTERACTION does not acquire atomic authority across unrelated domains. Coupled multi-owner mechanics require a named owner-specific workflow or fail closed.
9. **Proposal-only scripting:** DUR-04 scripts return bounded proposals/results; host/domain validation remains mandatory.
10. **Explicit lifetime:** every stateful interaction definition declares scope, lifetime, reset policy and recovery requirements.
11. **Revision binding:** in-flight/pending/retryable behavior is bound to exact behavior-affecting semantic revisions and is not silently reinterpreted after content activation.
12. **Stale-work rejection:** scope ownership generation, interaction state revision and applicable owner fences are revalidated when asynchronous/delegated work returns.
13. **Bounded resources:** every externally controllable count/depth/length/bytes/rate/pending-work dimension has a registered hard maximum before implementation acceptance.
14. **Fail closed:** missing scope/owner/limit/migration/coupled-workflow semantics block the affected mechanic; no permissive runtime fallback is inferred.

## 4. Candidate-local terminology

The names below define semantics only and are not frozen Rust/wire type names.

### 4.1 `InteractionDefinition`

An immutable, content-revision-bound definition describing one interactable world mechanism or one interaction capability attached to a world object.

It declares as applicable:

- stable content-local definition/object key under the World Bundle semantic graph;
- supported origin/verb/capability classes;
- authoritative scope class;
- state schema/state-machine transitions;
- trigger policies;
- reset/lifetime policy;
- typed delegation routes;
- deterministic ordering/tie policy identifiers where required;
- semantic timer policies;
- script capability profile where permitted;
- migration compatibility policy;
- resource-limit profile/registered limit requirements.

A content key is not automatically a globally unique durable gameplay identifier.

### 4.2 `InteractionIntent`

A normalized attempt to interact. It may include:

- authoritative actor/context derived by the server;
- source occurrence reference;
- requested interaction verb/capability;
- requested target hint;
- optional ItemInstance reference as a request input;
- optional bounded typed write/interaction payload;
- exact semantic revision context required for behavior.

The client cannot self-author authoritative actor, scope, ownership generation, target result or success.

### 4.3 Source occurrence reference

GAME-INTERACTION introduces no mandatory global `InteractionId` in this candidate.

Every occurrence reuses a stable identity/correlation shape owned by its source, for example:

- explicit player action -> FND-02 `CommandRef = (GameSessionId, CommandId)`;
- movement/contact trigger -> stable committed movement occurrence/effect reference supplied by the movement owner;
- timer -> stable semantic timer occurrence reference owned by the interaction/runtime timer contract;
- delegated workflow -> the owning OperationId/TransactionId/correlation identity;
- system/world event -> EventId/occurrence identity from the accepted owner where applicable.

The source reference must be sufficient for duplicate/replay reconciliation. A future dedicated interaction lifecycle identifier requires evidence and separate acceptance.

### 4.4 `InteractionObjectRef`

Conceptual reference used during one authoritative resolution to identify the current interactable object in an explicit `RuntimeScope` plus enough object/content/runtime identity to reject stale/mismatched references.

It is server-resolved. A client-supplied tile/object index is only a target hint.

### 4.5 `InteractionStateRevision`

An owner-local monotonically advancing revision, or semantically equivalent current-state fence, for mutable interaction-owned state.

It:

- advances when authoritative interaction state commits;
- detects stale prepared/script/dependency work;
- participates in recovery/replay evidence where needed;
- is not an entity identity, credential or replacement for FND-03 ownership generation.

Physical width/encoding is deliberately not frozen.

### 4.6 `InteractionTransitionPlan`

A bounded typed plan produced after target/current-state/affordance validation. It may contain:

- interaction-owned local state transitions;
- deterministic timer schedule/cancel requests owned by interaction runtime semantics;
- typed delegated owner requests;
- typed presentation/result events;
- no arbitrary mutation closure or general-purpose script callback.

The host validates the complete plan before any commit permitted by its declared atomicity class.

## 5. Origin model

All origins use the same authority boundary. Origin controls allowed capabilities, not whether server validation can be bypassed.

### 5.1 `PLAYER_EXPLICIT`

Explicit player `use`/`interact`/read/write activation carried by a current valid GameSession command.

Requirements:

- FND-02 duplicate/sequence semantics apply;
- authoritative actor/current runtime scope are derived server-side;
- target and payload are bounded before expensive work;
- interaction result cannot be committed twice for the same CommandRef.

### 5.2 `PLAYER_ITEM_ASSISTED`

Explicit player attempt involving one or more item references.

Requirements:

- item references are requests only;
- current item existence/location/ownership/eligibility comes from GAME-ITEM/DUR-03 authority;
- GAME-INTERACTION cannot consume/move/transform/mint/burn the item directly;
- coupled object+item success requires an accepted owner-specific workflow when both mutations must be atomic.

### 5.3 `MOVEMENT_CONTACT`

Generated only from a **committed authoritative movement occurrence**, never a raw movement request or renderer/physics callback.

Supported semantic categories may include enter, exit, contact or other explicitly versioned movement-derived triggers. Exact movement ordering/geometry remains movement-owned.

### 5.4 `SEMANTIC_TIMER`

A due interaction timer becomes a new FND-03 normalized authoritative input. The timer occurrence carries enough definition/state/revision evidence to reject stale firing.

### 5.5 `AUTHORIZED_SYSTEM_EVENT`

A named world/runtime/domain owner may invoke a capability explicitly granted by the interaction definition. Unscoped generic event-bus mutation is forbidden.

### 5.6 `SCRIPT_PROPOSAL`

A DUR-04 script returns a bounded proposed plan/input. It never becomes the authoritative origin simply by running; host validation and current-owner ordering remain mandatory.

### 5.7 Future AI/NPC/creature origin

A future AI owner may submit a typed interaction intent when an accepted contract permits it. GAME-INTERACTION defines no AI decision policy and grants no autonomous arbitrary mutation surface.

## 6. Interaction capability taxonomy

Capability composition is permitted only when compiler validation proves one unambiguous scope, ownership and transition model.

### 6.1 Stateful gates — doors/barriers

GAME-INTERACTION may own:

- open/closed/locked-like interaction state where the state belongs to the runtime object;
- deterministic state transition guards;
- interaction-specific activation/reset timer;
- publication of a typed topology/affordance change.

GAME-INTERACTION does not thereby own:

- movement/pathfinding/collision algorithm;
- ItemInstance key ownership/consumption;
- durable value changes;
- cross-Channel movement.

A locked-door mechanic may **query/validate** item eligibility through the item owner. If the key must be consumed atomically with opening, a named coupled workflow is required.

### 6.2 Activators — switches/levers/pressure mechanisms

GAME-INTERACTION owns the activator's declared interaction state and deterministic trigger routing.

A route to another object/domain is typed, bounded and explicit. It cannot dispatch arbitrary method names, script strings, database writes or unbounded event topics.

### 6.3 Relocator affordances — teleports/portals

GAME-INTERACTION owns activation affordance and versioned route/destination descriptor selection from content.

It does **not** own final authoritative character relocation/handoff/admission.

Requirements:

- no silent alternate destination or Channel retarget;
- no bypass of Channel switch hard locks/cooldown/fresh admission;
- no creation of Instance/GameSession/CharacterLease by interaction code;
- destination semantic revision and scope are explicit;
- if destination/cross-scope owner is unavailable or incompatible, the interaction fails or enters an accepted pending workflow; it does not guess;
- charge/cost + relocation coupling requires a named owner-specific workflow.

### 6.4 Traps/contact hazards

GAME-INTERACTION may own:

- armed/disarmed/one-shot/repeatable interaction state;
- trigger eligibility;
- reset/cooldown state specifically belonging to the trap mechanism;
- conversion of committed movement/contact occurrence into one bounded typed downstream effect request.

Damage/healing/conditions/immunity/combat legality belong to GAME-ABILITY/combat authority.

If trap-state transition and effect must be one all-or-nothing semantic outcome, execution is blocked until a coupled workflow contract exists.

### 6.5 Fields/area triggers

An authored area may expose interaction-owned enter/exit/contact semantics. An effect-bearing combat field remains subordinate to GAME-ABILITY/combat semantics for effects.

The term `field` does not grant GAME-INTERACTION a second duration/tick/condition engine.

### 6.6 Readable objects

A read interaction returns server-authoritative content/state under the target's exact revision.

- immutable authored text comes from the active content revision;
- mutable/domain-owned text is queried from its owner;
- the client never supplies authoritative text for a read result;
- reads remain bounded in bytes/codepoints/result count before implementation acceptance.

### 6.7 Writable objects

GAME-INTERACTION owns only the interaction-side boundary unless a later accepted contract assigns more:

- target/access/affordance validation;
- input normalization and safety limits;
- typed write request routing;
- structured success/failure presentation.

Persistent player-authored text ownership, moderation, privacy, retention, search/indexing and storage schema are **not assigned by this candidate**.

A `CHANNEL_LOCAL` transient writable mechanic is allowed only when the definition explicitly states that it is ephemeral/non-durable and the semantics do not imply an ItemInstance, Character or world-global persistent record.

### 6.8 Item-routed world use

GAME-INTERACTION may decide which world-object affordance the attempt addresses and may request item-owner validation.

It may not write item location/count/durability/binding/ownership/value state itself.

An item that activates an ability delegates into GAME-ABILITY rather than receiving a private interaction effect path.

### 6.9 Script-extended mechanisms

Scripts can refine allowed behavior only through DUR-04 typed capabilities and this candidate's plan validator. They cannot create a new capability family by invoking arbitrary host functions.

## 7. Interaction resolution pipeline

Every authoritative occurrence follows this semantic order. A later implementation may optimize internally only if observable authority/order/failure semantics are preserved.

```text
1. source occurrence accepted by its source contract
2. FND-03 normalized authoritative input reaches current scope owner
3. normalize InteractionIntent
4. resolve bounded interactable object candidate(s)
5. bind exact definition/content/ruleset/SIM revisions
6. validate actor/origin capability + current scope + target/current state
7. validate interaction-specific affordance/prerequisites
8. build bounded typed InteractionTransitionPlan
9. validate plan ownership, limits, atomicity class and stale fences
10a. commit interaction-owned local state atomically
 OR
10b. submit typed delegated operation
 OR
10c. enter named coupled owner-specific workflow
11. publish result/state revision/evidence
```

There is no hidden post-commit target re-resolution or script mutation pass.

## 8. Interactable-object target resolution

GAME-INTERACTION needs a deterministic resolver for **world interaction objects**. This is not a replacement for GAME-ABILITY's Target Resolver.

Requirements:

- current authoritative scope bounds the search;
- client reference/position/stack index is untrusted intent;
- candidate enumeration has a registered hard maximum;
- result cardinality has a registered hard maximum;
- stable deterministic ordering/tie-breaking is required when multiple candidates are eligible;
- ordering may not depend on memory address, hash/container iteration, thread/worker completion or unspecified database ordering;
- invalid or stale client hints do not become arbitrary world-query capability;
- target discovery cannot cross Channel/Instance authority implicitly;
- script extensions receive only a bounded immutable candidate/snapshot surface allowed by capabilities.

**Reference-sensitive target priority is deliberately unresolved.** Missing evidence fails the affected parity claim closed; it does not create an arbitrary default disguised as Reference behavior.

## 9. Affordance/legality boundary

Interaction legality is separate from target discovery.

After an interactable object is resolved, the current owner evaluates only interaction-owned predicates and typed facts from other owners, including as applicable:

- origin/verb supported by definition;
- current interaction state and revision;
- current runtime scope;
- actor capability/session facts supplied by existing authority;
- item eligibility fact from item authority;
- movement/spatial reachability fact from movement/visibility authority when required;
- world policy fact from named world service;
- rate/resource/cascade budget availability;
- content/ruleset revision compatibility.

GAME-INTERACTION does not freeze exact range/LoS/floor/reachability algorithms here. A later movement/interaction rule contract must supply those facts or policies explicitly.

## 10. Interaction-owned state machine

A stateful definition uses a bounded typed state machine.

Each transition declares:

- source state(s);
- allowed origin/capability;
- typed guards;
- deterministic destination state;
- local side effects owned by interaction;
- typed delegated actions;
- timer schedule/cancel semantics if applicable;
- reset implications;
- failure/atomicity class;
- behavior-affecting revision provenance.

Forbidden authoritative state representations include:

- arbitrary untyped script dictionaries as the primary model;
- mutable process-global variables;
- client-owned state;
- object state inferred only from visual sprite/renderer state;
- mutable state duplicated independently across Channels when semantics declare world-shared ownership.

## 11. Scope model

Every stateful interaction definition declares exactly one authoritative scope class for each mutable state component.

### `CHANNEL_LOCAL`

- semantic identity includes the current World/Channel context needed to resolve authority;
- current `ChannelRuntime` is the only runtime mutation owner;
- another Channel receives an independent state copy only because content policy explicitly defines Channel-local semantics, not by implementation accident;
- no state leaks through process-global cache/singleton.

### `INSTANCE_LOCAL`

- current `InstanceRuntime` owns the state;
- origin Channel is not an authority shortcut;
- instance handoff/recovery semantics remain FND-owned.

### `WORLD_SHARED_DELEGATED`

- Channel/Instance runtime is **not** the authoritative mutable owner;
- one named world/domain owner orders the state;
- local runtimes issue typed commands/queries and consume versioned results;
- caching may be a projection only and cannot accept writes as authority;
- owner unavailability fails according to explicit dependency policy; no Channel elects itself temporary world owner.

### `FOREIGN_DOMAIN_DELEGATED`

Character/Account/Item/Guild/House/Reward/etc. state remains with its existing owner. Interaction definitions may reference it only through typed capabilities.

### Forbidden scope

There is no generic unowned `GLOBAL` mutable interaction scope.

## 12. Lifetime and recovery class

Scope and lifetime are orthogonal. Every mutable component selects one explicit lifetime class.

### `STATELESS`

No interaction-owned mutable history. Result derives only from immutable definition + current authoritative inputs.

### `RUNTIME_EPHEMERAL`

State is allowed to reset only at a declared **fresh semantic initialization** boundary. Starting a new process/NodeId or recovering the same ChannelId/InstanceId is not sufficient by itself.

### `RUNTIME_RECOVERABLE`

The state can affect authoritative outcomes across process loss and therefore must be reconstructable from checkpoint/replay/semantic recovery evidence, including pending timers/operations where applicable.

### `DURABLE_DELEGATED`

A named durable owner stores/reconciles the state. GAME-INTERACTION does not create a private persistence bypass.

If a definition lacks an explicit lifetime class, stateful activation fails closed.

## 13. Reset policy

Each mutable interaction state component declares one versioned reset policy:

- `NO_AUTOMATIC_RESET`;
- `AFTER_DURATION`;
- `ON_FRESH_SCOPE_INITIALIZATION`;
- `ON_NAMED_AUTHORITY_EVENT`;
- `DELEGATED_RESET_POLICY`.

Rules:

- reset is an authoritative transition and participates in ordering/replay;
- a timer-driven reset is a semantic timer occurrence, not direct OS callback mutation;
- process-local monotonic time may schedule local elapsed work only within one process incarnation;
- a reset whose semantics survive process failure must retain reconstructable semantic due/remaining-time policy under FND-03;
- the definition states whether downtime/offline time counts when the policy spans process lifetime;
- reset cannot erase a foreign-domain committed effect/value transaction;
- recovery cannot manufacture a reset because a checkpoint was missing; ambiguous state fails closed/reconciles under owning recovery policy.

Exact reset durations are deliberately not selected here.

## 14. Semantic timer contract

An interaction timer that may influence authoritative state records enough semantic evidence to determine:

- which definition/object/state revision scheduled it;
- occurrence/purpose identity sufficient for duplicate suppression;
- due/elapsed policy;
- whether downtime counts;
- bound behavior-affecting revisions;
- cancellation/supersession condition;
- owning runtime scope/generation validation needed at fire time.

At due time:

```text
timer scheduler marks due
-> normalized authoritative input
-> current owner validates timer is still current
-> transition resolution
```

A stale timer from an older object state, definition revision or scope generation commits nothing.

## 15. Movement-trigger contract

Movement-triggered interaction is downstream of a committed authoritative movement occurrence.

```text
committed movement occurrence
-> typed bounded contact/enter/exit occurrence
-> FND-03 interaction input
-> resolve current interaction objects
-> deterministic transition/delegation
```

Requirements:

- no raw client move request can trigger an authoritative trap/plate before movement commit;
- duplicate/replayed delivery of one movement occurrence cannot duplicate a one-shot interaction occurrence;
- actor's current scope after the movement commit determines the interaction owner;
- candidate objects are resolved deterministically with hard enumeration bounds;
- exact order between multiple contacted objects requires deterministic policy/evidence and cannot depend on container iteration;
- cross-owner movement handoff is not invented here.

## 16. Cross-object trigger/cascade contract

Interaction definitions may form bounded authored trigger graphs.

Every cascade has:

- one root source/correlation reference;
- explicit typed edges;
- deterministic edge/target order;
- hard maximum depth;
- hard maximum total steps/actions;
- hard maximum fan-out/result count;
- explicit duplicate/visited-edge behavior;
- explicit atomicity class.

Within one current runtime owner, a bounded validated plan may apply multiple interaction-owned state transitions during one non-interleaved resolution when the plan's semantics require it.

Crossing an owner/domain boundary ends synchronous mutation authority. The foreign action becomes a typed command/workflow and any completion returns later as a new normalized authoritative input.

Forbidden:

- recursive immediate script callbacks;
- unbounded object-to-object event dispatch;
- dynamic arbitrary event topic names as mutation authority;
- depth/fan-out inferred from available memory;
- partial unlimited cascade followed by best-effort cleanup.

## 17. Plan atomicity classes

Every plan declares one of the following semantic classes or a later accepted equivalent.

### `INTERACTION_LOCAL_ATOMIC`

All mutation in the plan is owned by one current runtime interaction authority. Validation completes before commit; the transition commits atomically at the interaction-domain semantic boundary.

### `DELEGATED_OWNER_OPERATION`

GAME-INTERACTION commits no coupled local state requiring remote success. It issues a typed request to the actual owner and the owner's operation defines authoritative success/failure.

### `EXPLICIT_INDEPENDENT_EMISSION`

A local interaction transition may commit and a foreign action may subsequently fail **only when the versioned definition/ruleset explicitly states the outcomes are independent**. This is not a default fallback for dependency failure.

### `NAMED_COUPLED_WORKFLOW`

Used when semantics require one coupled outcome across more than one owner. The named workflow contract must define:

- owner/coordinator of the operation;
- operation identity/idempotency;
- participant/fence/revision validation;
- prepare/commit or equivalent boundary;
- retry/timeout/ambiguity reconciliation;
- crash recovery;
- compensation only where explicitly modeled.

If no such workflow exists, the mechanic cannot claim coupled atomic success.

There is no `GENERIC_CROSS_DOMAIN_TRANSACTION` class.

## 18. Item-assisted interaction contract

GAME-ITEM/DUR-03 remain authoritative.

For an item-assisted interaction:

1. player CommandRef is reserved/ordered by FND-02/FND-03;
2. world object is server-resolved;
3. ItemInstance reference is resolved/validated by its owner against current authoritative location/state/fence;
4. interaction affordance consumes typed eligibility facts only;
5. any item mutation occurs through DUR-03 transaction semantics;
6. if object transition and item mutation require one coupled outcome, `NAMED_COUPLED_WORKFLOW` is mandatory;
7. duplicate/retry cannot consume or transform the item twice;
8. stale/moved/foreign-World item evidence rejects the attempt without making the object transition appear successful.

Scripts cannot bypass these steps.

## 19. Teleport/portal contract

A relocator definition may include a versioned destination descriptor but not direct movement authority.

GAME-INTERACTION may commit an interaction-owned activation state only under a declared atomicity policy. Authoritative relocation is delegated to the movement/handoff/admission owner.

Mandatory safety rules:

- destination descriptor has explicit World/Channel/Instance semantics as applicable;
- World/Channel/Instance identities are never conflated;
- changing ChannelId follows GAME-CHANNEL/FND fresh-admission/switch policy and cannot be disguised as local teleport;
- entering/leaving Instance uses accepted Instance handoff authority;
- unavailable/incompatible/full/draining/unsafe destination cannot silently retarget;
- stale destination/content revision cannot be guessed compatible;
- item/currency cost is never consumed by interaction code;
- if cost and relocation must commit together, a named coupled workflow is required;
- retry/recovery uses stable operation/source correlation and does not teleport twice.

Exact local position legality and relocation API remain outside this candidate.

## 20. Trap/field/hazard effect contract

Interaction-owned trigger logic and ability/combat effects are separate.

```text
committed movement/timer/use occurrence
-> GAME-INTERACTION trigger/state validation
-> typed non-player-origin ability/effect request
-> GAME-ABILITY authoritative target/legality/effect lifecycle
```

GAME-INTERACTION cannot directly:

- compute/apply damage/healing;
- add/remove combat conditions;
- bypass immunity/PvP/PZ/target legality;
- invent ability cooldown/charge/cost semantics;
- re-target after GAME-ABILITY stabilizes targets.

Because overall GAME-ABILITY-01 remains open, the executable non-player-origin invocation/coupled-failure surface is a recorded cross-domain dependency rather than invented here.

## 21. Readable/writable contract

### Read

- server resolves target and authorization;
- authored immutable text is read from the exact active/bound content revision;
- mutable text comes from its named owner;
- response size/result count is bounded;
- read does not create hidden write/audit/value mutation unless the content explicitly routes a typed action.

### Write

- client text is untrusted input;
- validate encoding/normalization and registered byte/codepoint limits before retained allocation/persistence;
- validate target/current revision/access before submitting write;
- persistent write goes to a named owner; GAME-INTERACTION is not a generic text database;
- retry uses owner idempotency semantics;
- moderation/privacy/retention are owner policy, not script choice;
- an ItemInstance-backed writable object remains item/durability-owned for its persistent state.

Until a durable owner is accepted, persistent player-authored world text is architecture-blocked; transient explicitly Channel-local content may be supported separately under its declared lifetime.

## 22. Script contract

DUR-04's capability model is binding.

A GAME-INTERACTION script invocation receives only:

```text
explicit InvocationContext
+ bounded immutable interaction snapshot
+ explicit capability set
+ deterministic runtime-provided RNG capability when authorized
```

and returns only:

```text
bounded ProposedActionPlan / typed result
```

The host then revalidates:

- current scope owner/generation;
- target identity/current state revision;
- exact semantic revisions;
- origin/actor capability;
- every action's owning domain;
- plan cardinality/cascade limits;
- required resource budgets;
- atomicity class.

Forbidden script capabilities:

- direct mutable world references;
- arbitrary DB/SQL;
- filesystem/network/process/env/secrets;
- wall-clock or OS RNG;
- unrestricted world iteration;
- direct ItemInstance/value mutation;
- direct player relocation/handoff;
- direct ability/combat effect mutation;
- arbitrary world-shared state;
- arbitrary event bus/topic publication with mutation authority;
- private persistent VM globals as canonical object state;
- private unbounded timers;
- recursive arbitrary interaction invocation.

Trap, fuel exhaustion, invalid action, unauthorized route, stale snapshot or resource-limit breach commits no plan by default.

## 23. Deterministic randomness

Most interaction transitions should be deterministic without RNG. If a versioned interaction mechanic genuinely requires randomness:

- use SIM-DETERMINISM-01 runtime-provided deterministic RNG;
- purpose stream is explicit and isolated from unrelated systems;
- same logical occurrence/retry does not reroll;
- script cannot choose seed or access OS/process-global RNG;
- result/evidence retains enough purpose/occurrence/revision context for replay.

Reference-specific random behavior is not inferred without evidence.

## 24. Concurrency and ordering

### Same object, same owner

Concurrent-ready interactions are serialized by FND-03 authoritative input order. Each resolves against current state after preceding commits.

### Multiple objects in one owner

When a single validated plan owns several local interaction transitions, deterministic internal object/action ordering is required and included in replay evidence.

### Cross-session races

No network-arrival or thread wake-up timestamp creates global gameplay precedence. FND-03 owner order is authoritative unless a later domain rule defines a deterministic simultaneous/conflict rule.

### Async/delegated completion

Completion is a **new normalized input** and must carry/correlate enough evidence to validate:

- source CommandRef/occurrence/operation;
- semantic runtime scope;
- current scope ownership generation;
- expected interaction state revision;
- exact relevant content/ruleset/SIM revisions;
- foreign owner fence/revision as required.

Stale completion commits nothing and cannot revive an earlier owner/object state.

## 25. Persistence and recovery

GAME-INTERACTION does not define physical PostgreSQL schema. It defines semantic recovery obligations.

### 25.1 Interaction-local recoverable state

If loss of state could change a future authoritative outcome after same-scope recovery, the state is `RUNTIME_RECOVERABLE` or delegated durable state, not `RUNTIME_EPHEMERAL`.

Recovery evidence includes as applicable:

- interaction definition/object key and bound revision;
- current typed state and `InteractionStateRevision`;
- pending semantic timer states;
- pending delegated/coupled operation references;
- source/correlation identities;
- exact behavior-affecting semantic revisions;
- committed/uncommitted/reserved status needed for deterministic reconciliation.

### 25.2 Crash before commit

No interaction-owned mutation is committed. Same source occurrence may retry under existing duplicate semantics.

### 25.3 Crash after commit before response

Committed state/revision remains authoritative. Retried `CommandRef`/source occurrence reconciles to the committed outcome rather than applying again.

### 25.4 Crash with pending delegated work

Recovery must determine one of: not submitted, submitted/pending, committed foreign outcome awaiting observation, cancelled/terminal. Ambiguity is reconciled using the owning operation contract; it is never guessed from absence of a response.

### 25.5 New process / same semantic scope

New `NodeId` does not imply fresh interaction state. Same ChannelId/InstanceId recovery restores/reconstructs the accepted semantic state and newer FND-03 ownership generation; stale old-owner timers/completions fail closed.

### 25.6 Unrecoverable ambiguity

If required authoritative evidence is unavailable/corrupt/incompatible, risky mutation is unavailable until a named recovery path resolves it. Default-state reset is not a safety fallback.

## 26. Content revision activation and migration

Interaction behavior is immutable-revision-bound under DUR-04.

Every affected definition/state migration is classified as one of:

- compatible/no migration;
- read-compatible with explicit normalization;
- explicit migration;
- incompatible;
- removed/tombstoned policy.

Rules:

1. Active definitions are never edited in place.
2. An in-flight/pending occurrence keeps its bound behavior-affecting revisions across retry unless an explicit accepted migration/reconciliation policy says otherwise.
3. A semantic timer cannot fire under a newer definition merely because activation occurred after it was scheduled.
4. Mutable state is not mapped to a new state variant by ordinal/name coincidence; migration is typed/versioned.
5. Incompatible state blocks activation/use rather than silently resetting.
6. Removal of an object with durable/pending/foreign-coupled state requires explicit retirement/migration/tombstone handling.
7. World Bundle corruption/incompatibility is rejected before partial interaction activation.
8. Coexisting revisions may serve already-bound work only when DUR-04 permits exact-scope revision coexistence.

## 27. Failure semantics

Every public/cross-component interaction failure maps to `FOUNDATION_ERROR_VOCABULARY.md` and states retry/idempotency/partial-mutation outcome.

| Category | GAME-INTERACTION examples | Mutation rule |
|---|---|---|
| `INVALID_INPUT` | malformed verb, non-canonical payload, impossible target encoding | no mutation |
| `UNSUPPORTED_REVISION` | incompatible content/interaction/ruleset revision | no reinterpretation/downgrade; no unsafe mutation |
| `SESSION_REJECTED` | source command no longer belongs to accepted session authority | no interaction mutation |
| `STALE_GENERATION` | stale runtime generation/object revision/fence used as authority | no stale commit |
| `CONFLICT` | current state/guard/access no longer permits requested transition | no unrequested fallback transition |
| `CAPACITY_EXCEEDED` | candidate/cascade/write/timer/pending/rate bound hit | bounded rejection/backpressure, no unbounded allocation |
| `DEPENDENCY_UNAVAILABLE` | required item/movement/ability/world owner unavailable | fail/pending only under declared policy; no local authority takeover |
| `TIMEOUT` | named delegated workflow deadline expires | owning workflow reconciles; no guessed rollback |
| `CANCELLED` | explicit pending interaction operation cancelled | cleanup/result per owner; committed history remains history |
| `INTERNAL_UNAVAILABLE` | unexpected state cannot be handled safely | fail closed with redacted diagnostics |

Narrow codes can be introduced later but must map to one category.

## 28. Foundation failure-scenario obligations

Architecture expectation for applicable catalogue scenarios:

| Scenario | Candidate invariant | Runtime evidence status |
|---|---|---|
| `FS-STALE-GENERATION` | old runtime/object/dependency work cannot commit | `NOT_STARTED` |
| `FS-DUPLICATE-COMMAND` | one CommandRef cannot duplicate interaction effect | `NOT_STARTED` |
| `FS-CHANNEL-SPLIT-OWNER` | only current scope generation mutates local object state | `NOT_STARTED` |
| `FS-QUEUE-SATURATION` | bounded backpressure/rejection, no unbounded interaction growth | `NOT_STARTED` |
| `FS-CLOCK-SKEW` | wall clock is not local timer-duration authority; recoverable timer policy is explicit | `NOT_STARTED` |
| `FS-REVISION-MISMATCH` | no silent content/ruleset/interaction reinterpretation | `NOT_STARTED` |
| `FS-WORLD-BUNDLE-CORRUPT` | reject before partial interaction activation | `NOT_STARTED` |
| `FS-POSTGRES-UNAVAILABLE` | durable-owner mechanic cannot create unfenced local fallback authority | `NOT_STARTED` |
| `FS-DB-OUTBOX-BOUNDARY` | applicable durable coupled workflow follows its owner transaction/outbox contract | `NOT_STARTED` |

`NOT_STARTED` is implementation/proof truth; this candidate specifies the invariant only.

## 29. Resource-limit contract

Implementation acceptance is **blocked** until every externally controllable interaction dimension has a numeric hard maximum registered in `docs/contracts/RESOURCE_LIMITS_REGISTRY.json` or its accepted successor.

At minimum the owning follow-up must register/evidence limits for:

1. domain-specific interaction payload bytes where a stricter bound than FND-02 generic command payload is needed;
2. player/session/actor accepted interaction work rate or bounded scheduling budget;
3. interactable candidate enumeration per resolution;
4. resolved interaction target count;
5. writable input bytes/codepoints and normalization expansion;
6. transition actions per plan;
7. cascade depth;
8. total cascade steps/actions;
9. cascade fan-out;
10. interaction timers per object and per authoritative runtime scope;
11. pending delegated/coupled operations per object/actor/scope;
12. interaction-specific script proposal/host-call/result sizes in addition to DUR-04 outer limits;
13. retained recovery/evidence size per pending occurrence;
14. world/dependency query/result count used by interaction;
15. content state-machine states/transitions/edges where compiler/runtime complexity can be externally amplified.

Every entry must define unit, hard maximum, configurable range, failure category, allocation impact and boundary tests as required by the shared registry.

No value is guessed in this candidate because Agent D does not own the shared registry and no measured evidence supplies safe numbers.

## 30. Anti-abuse/security contract

The accepted implementation must defend at least against:

### Forged target/scope references

- derive actor/current scope server-side;
- server resolves target;
- cross-Channel/Instance target references fail closed;
- stale object/state revisions are rejected.

### Interaction spam/resource exhaustion

- bounded ingress/scheduling work;
- candidate enumeration caps;
- per-plan/cascade caps;
- no unbounded queued script/dependency/timer work;
- rate/budget exhaustion maps to bounded failure/backpressure.

### Duplicate trigger farming

- player duplicates use CommandRef semantics;
- movement/timer/system trigger sources carry stable occurrence/correlation evidence;
- retry/recovery cannot reroll or reapply one logical one-shot occurrence.

### Script capability escalation

- proposal-only host;
- explicit allowlisted capabilities;
- no arbitrary database/network/world iteration/mutable refs;
- plan revalidation after execution;
- deterministic fuel/memory/host-call/action limits.

### Cascade amplification

- compiler-visible typed edges;
- hard depth/action/fan-out caps;
- deterministic work list;
- no recursive callback/event bus.

### Writable-text abuse

- strict encoding/normalization/size bounds before allocation/storage;
- explicit authorization;
- persistent writes routed to moderation/privacy/retention owner;
- no unsanitized text used as commands/script/event identifiers.

### Teleport/placement abuse

- interaction cannot bypass movement/handoff/admission/Channel switch policy;
- no silent alternate destination;
- stale/invalid destination fails closed.

### Item/value abuse

- interaction never edits item/value state directly;
- DUR-03 transaction/idempotency/fencing remain authoritative;
- missing coupled workflow blocks mechanics that need coupled success.

## 31. Observability/evidence boundary

Interaction runtime should produce typed evidence sufficient to explain authoritative decisions without giving analytics enforcement authority.

As applicable evidence includes:

- source CommandRef/occurrence/correlation reference;
- actor/domain-safe identity references under ANL/privacy policy;
- WorldId + ChannelId/InstanceId scope;
- FND-03 ownership generation and RuntimeExecutionOrdinal where retained by owning evidence contract;
- interaction definition/object key and exact content/ruleset/SIM revisions;
- prior/result `InteractionStateRevision`;
- normalized trigger/verb class;
- deterministic target/plan summary appropriate for audit privacy;
- delegated operation/transaction refs;
- terminal failure category/code;
- resource-limit/cascade rejection evidence.

Analytics/AI remain read-only investigators and cannot use interaction telemetry to mutate gameplay, ban players or rewrite object state.

Physical ANL event-family schema remains ANL-owned.

## 32. Deterministic acceptance scenarios

A future implementation/proof task must exercise at least these cases.

### GI-01 — same lever race

Given two eligible player commands for one `CHANNEL_LOCAL` lever, FND-03 assigns authoritative order. Command B evaluates state after A. Replay with identical order/revisions produces identical result/state revisions.

### GI-02 — duplicate explicit use

Retry one already-accepted CommandRef. The object transition/delegated effect occurs at most once; prior result or explicit duplicate outcome is returned.

### GI-03 — movement one-shot trap

One committed movement occurrence enters a one-shot trap. Duplicate delivery/recovery of that movement occurrence does not trigger a second semantic trap occurrence.

### GI-04 — timer/manual transition race

A reset timer and player interaction become ready near the same time. FND-03 authoritative order determines the outcome; wall-clock/thread scheduling does not.

### GI-05 — script trap/resource exhaustion

Interaction script traps or exhausts its resource budget. No proposed transition/action commits; structured failure evidence is produced.

### GI-06 — script foreign-mutation escape attempt

Script proposes a direct ItemInstance/value/movement/ability mutation outside typed capabilities. Host rejects entire unauthorized proposal; no local partial transition is committed by default.

### GI-07 — stale key/item use

Player attempts locked door with item evidence that becomes stale/moved before commit. Item owner/fence validation rejects. Door does not claim coupled success without an accepted coupled workflow.

### GI-08 — Channel-local isolation

Same authored lever exists in Channels A and B with `CHANNEL_LOCAL` scope. Changing A cannot change B. Restarting one process does not create a shared/global state path.

### GI-09 — world-shared ordering

Two Channels activate one `WORLD_SHARED_DELEGATED` mechanism. One named world owner orders current state; Channels do not each commit an authoritative local copy.

### GI-10 — stale cross-owner completion

A dependency result prepared under scope generation G returns after G+1 owns the scope. The stale completion cannot commit or revive old object state.

### GI-11 — crash after local commit before response

Object transition commits, process fails before response. Recovery reconstructs committed revision; retry does not apply again.

### GI-12 — content activation with pending timer

Timer scheduled under definition revision R remains bound to R or follows explicit migration/cancellation. Activation R+1 cannot silently reinterpret the pending occurrence.

### GI-13 — writable hard bound

Payload at hard maximum is handled according to content/owner policy; one byte/codepoint beyond accepted bound is rejected before retained allocation/persistence and produces no write.

### GI-14 — cascade hard bound

A trigger graph exceeds depth/action/fan-out maximum. Resolution fails deterministically under declared atomicity; no unbounded recursion/work growth occurs.

### GI-15 — unavailable teleport destination

Target destination is unavailable/incompatible/full/draining or requires a forbidden Channel transition. No silent retarget and no unrelated item/currency consumption occurs.

### GI-16 — split owner

Two processes believe they own one Channel. Only current FND-03 ownership generation can commit interaction state/timers/completions.

### GI-17 — world-owner outage

World-shared mechanism owner is unavailable. ChannelRuntime does not become temporary owner; interaction fails/pends only according to explicit dependency policy.

### GI-18 — restart does not imply reset

A `RUNTIME_RECOVERABLE` door/trap/switch survives same-scope process replacement without resetting solely because NodeId changed.

## 33. Decision timing

### Must decide now? YES

This authority/state/delegation boundary is required before interaction-heavy content and runtime implementation. Delaying it would allow content/script/runtime APIs to harden unsafe ownership semantics.

### Downstream work unblocked by acceptance

Acceptance would unblock **later bounded implementation design/tasks**, not implementation authority itself, for:

- world-object interaction runtime model;
- content validator/compiler interaction schema;
- basic local doors/switches/levers;
- movement-trigger integration once movement owner contract exists;
- safe script host capabilities;
- recovery/replay fixture design;
- resource-limit measurement/registration work.

### Work still blocked after acceptance

- Reference parity claims needing exact interaction behavior evidence;
- teleport/portal execution needing movement/relocation owner contract;
- combat-effect traps/fields needing GAME-ABILITY executable boundary closure;
- persistent player-authored writable text needing explicit durable/moderation owner;
- coupled multi-owner mechanics needing owner-specific workflow contracts;
- implementation acceptance until numeric interaction resource limits are registered.

### Evidence required to supersede

Supersession requires representative Reference/product mechanics, deterministic replay/fault evidence, performance/resource measurements, security findings or accepted downstream owner contracts demonstrating that this boundary cannot safely express required behavior.

Any superseding design must preserve or explicitly replace server authority, one-writer runtime ownership, typed domain delegation, deterministic replay, stale-work fencing, bounded resources and proposal-only scripting.

## 34. DECISIONS_NOT_TAKEN

This candidate deliberately does not decide:

- exact Reference `use`/`use with` stacked-object target priority;
- exact interaction distance/LoS/floor/reachability rules;
- exact Reference key/door, switch, lever, trap, field, teleport or reset formulas/order;
- movement/pathfinding/collision implementation or canonical relocation API;
- cross-Channel/Instance movement/handoff implementation beyond preserving existing owners;
- GAME-ABILITY damage/healing/condition/immunity/PvP/effect semantics;
- AI decision policy;
- item/location/value semantics already owned by GAME-ITEM/DUR-03;
- a generic multi-owner transaction system;
- persistent writable-text owner/schema/moderation/privacy/retention/search policy;
- exact physical interaction content/WIT/Rust/wire schema;
- scheduler/thread/async implementation;
- exact timer/rate/cascade/query/payload numeric values;
- client UX/presentation;
- PostgreSQL DDL/migrations;
- global programme/status/register/horizon edits;
- runtime implementation, deployment or production activation.

Unresolved Reference-sensitive behavior remains fail-closed.

## 35. CROSS_DOMAIN_FINDINGS

```yaml
cross_domain_finding:
  id: GAME-INTERACTION-01-XD-ABILITY-TRIGGER
  observed_in_domain: GAME-INTERACTION-01
  target_owner: GAME-ABILITY-01
  severity: P1
  evidence: GAME-ABILITY-01_TARGETING_AND_LEGALITY_BOUNDARY_OWNER_BASELINE.md and GAME-ABILITY-01_CAST_CHANNEL_COMMIT_OWNER_BASELINE.md; overall GAME-ABILITY-01 remains open
  conflict_or_gap: Interaction-triggered traps/fields/hazards need a stable typed non-player-origin ability/effect invocation and explicit coupled-failure semantics without giving GAME-INTERACTION combat/effect authority.
  required_before: executable trap/field/hazard implementation claiming ability/combat effects
  worker_action: REPORT_ONLY
```

```yaml
cross_domain_finding:
  id: GAME-INTERACTION-01-XD-MOVEMENT-RELOCATION
  observed_in_domain: GAME-INTERACTION-01
  target_owner: ARCHITECTURE_COORDINATOR
  severity: P1
  evidence: FND-03 runtime ownership and GAME-CHANNEL-01 Channel switch policy exist; no movement/relocation contract is owned by this worker allocation
  conflict_or_gap: Teleporter/portal activation and movement-derived triggers require an exact authoritative movement occurrence/relocation/handoff owner surface that GAME-INTERACTION cannot invent.
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
  conflict_or_gap: Durable writable boards/books/signs require explicit owner, moderation/privacy/retention and migration semantics; interaction owns only access/validation/routing in this candidate.
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
  conflict_or_gap: Mechanics needing one semantic outcome across interaction state plus item/value/movement/ability/world owners require a named owner-specific operation/workflow; none is accepted generically.
  required_before: consume-key-and-open, charge-and-teleport, disarm-and-item-mutate or equivalent coupled implementation
  worker_action: REPORT_ONLY
```

```yaml
cross_domain_finding:
  id: GAME-INTERACTION-01-XD-RESOURCE-LIMITS
  observed_in_domain: GAME-INTERACTION-01
  target_owner: ARCHITECTURE_COORDINATOR
  severity: P1
  evidence: docs/contracts/RESOURCE_LIMITS_REGISTRY.json requires registered hard maxima for externally controlled dimensions; this worker does not own the shared registry
  conflict_or_gap: Interaction-specific target enumeration, cascade, writable payload, timer, pending-operation and rate maxima need measured numeric values and registry entries.
  required_before: GAME-INTERACTION implementation acceptance
  worker_action: REPORT_ONLY
```

## 36. Candidate acceptance statement

If accepted by the Architecture Coordinator and canonically integrated, GAME-INTERACTION-01 would establish this rule:

> **A world interaction is a bounded, revision-bound, server-authoritative state-machine occurrence executed by the current runtime owner. It may commit only interaction-owned state directly. Every item/value, movement/handoff, ability/combat or world-shared foreign mutation crosses an explicit typed owner boundary, and coupled multi-owner success requires a named recovery-safe workflow rather than a generic script/action bypass.**

This rule supports data-first rich mechanics while preserving FND-03 ownership, multichannel isolation, DUR-03 conservation, DUR-04 script safety and SIM determinism.

IMPLEMENTATION_AUTHORITY: NONE

MERGE_AUTHORITY: ARCHITECTURE_COORDINATOR_ONLY
