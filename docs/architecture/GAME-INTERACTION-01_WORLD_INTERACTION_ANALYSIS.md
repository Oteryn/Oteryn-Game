# GAME-INTERACTION-01 — World Interaction Architecture Analysis

- Date: 2026-08-15
- Gate: `GAME-INTERACTION-01`
- Delivery task: `OTV2-20260815-game-interaction-architecture`
- Delivery PR: #269
- Worker: Domain Architecture Design Agent D
- DecisionStatus: **PROPOSED ANALYSIS**
- DeliveryStatus: **IN_REVIEW**
- ImplementationStatus: **NOT_STARTED**
- Runtime/client/protocol/DDL/Platform/production authority: **NONE**
- Merge authority: **ARCHITECTURE_COORDINATOR_ONLY**

## 1. Problem

Oteryn needs one safe architecture for world interactions without turning `use`, movement callbacks, scripts or map objects into alternate mutation engines.

The domain has to cover at least:

- explicit player interaction/use intent;
- doors, switches and levers;
- teleports/portals and other relocation affordances;
- fields, traps and hazards;
- readable and writable objects;
- item-assisted use and use-with routing;
- movement/contact/enter/exit triggers;
- timer-triggered behavior;
- script-authored interaction proposals;
- channel-local, instance-local and world-shared behavior;
- reset, persistence, crash recovery and content-revision migration;
- deterministic ordering, replay and anti-abuse/resource bounds.

The hard part is ownership. A single interaction can touch several already-owned domains. A locked door may consult an item; a teleporter changes spatial placement; a trap may invoke damage/conditions; a writable object may need durable moderated text; a world lever may affect every Channel. GAME-INTERACTION therefore cannot be designed as a generic API with permission to mutate arbitrary gameplay state.

## 2. Evidence classification

### PROVEN — accepted upstream constraints

The current repository establishes these constraints for this analysis:

1. FND-03 gives each active Channel/Instance one current logical authoritative mutation owner and requires every mutation-capable event, timer or asynchronous completion to re-enter that owner's normalized authoritative input boundary.
2. The multichannel scope matrix makes the public-map mutable overlay Channel-local by default and forbids implicit sharing across Channels.
3. GAME-CHANNEL-01 separates runtime multiplicity from durable eligibility/world policy and requires explicit scope for behavior whose meaning changes with Channel multiplicity.
4. GAME-ITEM-01 owns item-definition/instance legality and keeps item location outside item identity.
5. DUR-03 owns immediate value location, conservation, item/currency transaction identity, idempotency, fencing and recovery.
6. DUR-04 requires content/script execution to be immutable-revision-bound, deterministic and proposal-only; scripts do not receive arbitrary database, filesystem, network, wall-clock, RNG or mutable-world authority.
7. SIM-DETERMINISM-01 requires identical canonical state + ordered input evidence + semantic revision set to yield identical authoritative results, including retry/failover behavior.
8. GAME-ABILITY-01 has owner-accepted partial baselines for server-authoritative targeting/legality and explicit ability commit/channel semantics, while the overall GAME-ABILITY gate remains open.
9. `RESOURCE_LIMITS_REGISTRY.json` requires every externally controlled count, depth, length or byte size to have an accepted hard maximum before implementation acceptance. No missing limit is inferred at runtime.

### DERIVED — consequences for GAME-INTERACTION

From those constraints:

- interaction must be a bounded domain pipeline, not a generic arbitrary-action interpreter;
- client target identifiers/positions/verbs are requests only;
- movement/timer/script callbacks cannot mutate an object directly outside FND-03 ordering;
- Channel-local object state may be owned by the current `ChannelRuntime`; Instance-local state by the current `InstanceRuntime`;
- world-shared mutable interaction state requires a named world-domain owner and typed delegation rather than replicated mutable copies in each Channel;
- item consumption/move/transform cannot be hidden inside a door/switch script;
- trap/field/hazard damage and conditions cannot become a second combat/effect engine;
- teleport activation cannot grant GAME-INTERACTION implicit ownership of character movement, handoff, Channel switching or Instance admission;
- an asynchronous dependency result is stale unless scope ownership generation, relevant state revision and semantic revisions are still valid when the result re-enters authority;
- crash recovery cannot silently reset state merely because a new process/NodeId starts.

### UNKNOWN — evidence not sufficient to freeze

The current accepted sources do not establish:

- exact Reference Tibia target-priority/tie-breaking rules for `use`/`use with` across stacked objects;
- exact interaction distance, line-of-sight, floor or reachability rules;
- exact door/key consumption behavior for every Reference mechanic;
- exact teleport destination fallback rules for every mechanic;
- exact trap/field trigger precedence, cadence or damage semantics;
- exact readable/writable object text ownership, moderation, retention and persistence model;
- numeric spam/rate/cascade/timer/query/write limits;
- a canonical movement-domain contract for relocation acceptance;
- a generic cross-domain atomic workflow spanning interaction state + movement + ability + item/currency state.

These are not guessed here.

## 3. Decision timing

### Must decide now? YES

A bounded interaction authority model is needed before broad map/world gameplay implementation. Otherwise content could accidentally encode five incompatible mutation paths: explicit use handlers, movement hooks, timer callbacks, item scripts and general-purpose scripts.

### Concrete downstream work blocked

Without this boundary, the project cannot safely claim implementation of:

- stateful doors/switches/levers;
- interaction-triggered teleports;
- interaction-triggered traps/fields/hazards;
- item-to-world-object use;
- persistent/resettable map mechanisms;
- scriptable world interactions;
- deterministic interaction replay/recovery;
- a Studio/content schema for interaction definitions.

### What becomes harder if decided incorrectly

If arbitrary scripts or callbacks become direct mutation authority, correcting the design later would require rewriting content, replay evidence, recovery, anti-duplication and security boundaries. If all interaction state is accidentally process-global or world-global, multichannel semantics would become unsafe. If local state is committed before a required cross-domain value/movement/effect operation without an explicit workflow, recovery could produce impossible partial outcomes.

### Evidence that may supersede this analysis

A later accepted decision may change specific interaction policy with:

- provenance-cleared Reference mechanic evidence;
- representative playtest/product evidence;
- deterministic replay/fault-injection findings;
- measured performance evidence for target resolution/cascades;
- security findings showing the proposed capability surface is insufficient or too broad;
- accepted movement, ability or persistence contracts that provide a stronger cross-domain workflow.

## 4. Scope and ownership question

The central architectural question is:

> What does GAME-INTERACTION own, and what must it only validate and delegate?

### Recommended ownership

GAME-INTERACTION should own:

- semantic normalization of an interaction attempt/trigger;
- resolution of an **interactable world object** within the current authoritative runtime scope;
- affordance validation specific to that interaction definition;
- typed, versioned interaction-object state machines;
- deterministic transition planning and local interaction-state commit;
- trigger/reset/timer semantics for interaction-owned state;
- bounded routing to other domain owners;
- interaction-specific evidence, failure classification and anti-abuse policy requirements.

It should not own:

- arbitrary character movement/position legality;
- ItemInstance location/value mutation;
- currency/value conservation;
- ability targeting, damage, healing, combat condition formulas or cooldown semantics;
- world-shared mutable domains merely because an interaction initiated them;
- arbitrary durable free-form text without an accepted durable/moderation owner;
- AI decision policy;
- raw database state or script-level mutation authority.

This is the smallest useful boundary that can support rich mechanics while preserving existing ownership.

## 5. Options considered

### Option A — generic `Use()` callback with unrestricted script hooks

```text
client/system trigger
-> find object
-> object script mutates whatever it needs
```

Advantages:

- low short-term implementation cost;
- familiar to older MMO engines;
- very flexible for content authors.

Risks:

- violates DUR-04 proposal-only script authority;
- creates hidden cross-domain transactions;
- encourages process-global mutable state;
- makes replay and security analysis dependent on arbitrary script behavior;
- makes resource bounds and deterministic ordering difficult to prove;
- can bypass GAME-ITEM/DUR-03/GAME-ABILITY ownership.

Verdict: **REJECT**.

### Option B — one universal interaction transaction engine

```text
InteractionPlan
-> atomically mutate object + player + item + ability + world service state
```

Advantages:

- attractive surface simplicity;
- could appear to solve door+key, teleport+cost and trap+state coupling uniformly.

Risks:

- becomes a second owner for almost every gameplay domain;
- requires an impractically broad global transaction boundary;
- conflicts with current explicit one-owner and DUR-03 domain semantics;
- produces a generic cross-authority bypass surface;
- hides which service owns recovery when a remote dependency is ambiguous.

Verdict: **REJECT**.

### Option C — typed interaction state machine + bounded domain delegation

```text
normalized source occurrence
-> resolve current interactable object
-> validate interaction affordance/current state
-> build bounded typed transition/delegation plan
-> commit only state owned by current interaction authority
   OR invoke an explicitly owned cross-domain workflow
-> publish deterministic result/evidence
```

Advantages:

- matches FND-03, DUR-04 and SIM boundaries;
- supports data-first content and safe script extension;
- cleanly separates local object state from item/movement/ability/world-service authority;
- allows deterministic replay and bounded resource use;
- lets future Reference-specific rules be versioned without replacing the engine.

Costs:

- requires explicit typed routes instead of arbitrary scripting;
- cross-domain coupled mechanics need named workflow ownership;
- content compiler/validator must understand scope, reset and transition declarations.

Verdict: **RECOMMEND**.

## 6. Candidate interaction source taxonomy

A single authoritative interaction pipeline should accept multiple **origins**, while origin changes capability/context rather than mutation authority.

### 6.1 Explicit player interaction

Examples: use a door, pull lever, read object, activate portal.

The network command carries intent only. Server authority derives current actor/session/scope and resolves the requested object from current authoritative state.

### 6.2 Item-assisted interaction

Examples: use key/tool/item on a world object.

The item reference remains subject to GAME-ITEM/DUR-03 current legality/location/fencing. GAME-INTERACTION may route the attempt but cannot consume, move, split, transform or mint/burn the ItemInstance itself.

### 6.3 Movement/contact trigger

Examples: enter pressure plate, cross hazard boundary, leave trigger region.

Only a **committed authoritative movement occurrence** may generate the interaction trigger. A raw client movement request, physics callback or renderer collision observation is insufficient.

### 6.4 Timer trigger

A due semantic interaction timer becomes a new FND-03 normalized authoritative input. OS timer callbacks do not directly mutate object state.

### 6.5 Authorized system/world event

A typed event from a named authority may trigger an interaction when the content contract explicitly permits that origin. It carries explicit scope and correlation evidence.

### 6.6 Script proposal

A DUR-04 script may return a bounded proposed interaction action/transition. The host independently validates target, capability, state revision, scope and plan limits before authority is granted.

### 6.7 AI-originated intent

A future AI/NPC/creature owner may submit an authorized interaction intent through the same boundary. GAME-INTERACTION does not define when AI chooses to do so.

## 7. Candidate object/affordance taxonomy

The taxonomy should describe capabilities, not force a rigid inheritance hierarchy.

| Capability family | Examples | GAME-INTERACTION owns | Must delegate |
|---|---|---|---|
| Stateful gate | door, barrier | interaction state transition | movement/collision acceptance outside interaction domain; item costs if any |
| Activator | switch, lever, pressure plate | trigger/state transition and typed routes | downstream domain effects |
| Relocator affordance | teleporter, portal, transition pad | activation/route selection policy within content definition | authoritative position/handoff/admission/switch |
| Contact hazard | trap, trigger tile | trigger occurrence and trap-owned armed/cooldown state | damage/condition/effect semantics |
| Area/field interaction | environmental field/zone trigger | entry/exit/contact state only where interaction-owned | combat/ability field semantics when effect-bearing |
| Readable | sign/book-like world object | access + authoritative read result routing | item-owned text when target is an ItemInstance |
| Writable | board/book-like object | interaction access/intent validation only unless explicitly transient local state | durable text ownership/moderation/persistence |
| Item-routed use | key/tool/use-with | object-side affordance and routing | item legality/location/value/state mutation |
| Script-extended | special content mechanism | host validation and owned state transition | every foreign-domain action remains typed delegation |

A single definition may compose capabilities only if compiler validation proves the combination has unambiguous owner/scope/reset semantics.

## 8. Interaction target resolution versus GAME-ABILITY targeting

These are related but not identical responsibilities.

GAME-INTERACTION requires a bounded deterministic resolver for **interactable world objects**. It must not reuse GAME-ABILITY target resolution as a back door to combat semantics, nor create a second ability target engine.

Rules proposed:

- client-provided tile/object/entity reference is a hint, never final authority;
- resolution is constrained to the actor's current authoritative Channel/Instance scope unless a named cross-scope owner explicitly participates;
- candidate enumeration and result cardinality are bounded;
- ordering/tie-breaking is deterministic and content/ruleset-versioned where behavior matters;
- no dependence on hash-map iteration, memory address, worker completion order or unspecified database ordering;
- Reference-specific stacked-object priority remains `UNKNOWN` until evidence exists;
- if an interaction delegates an ability, ability target/legality resolution uses the accepted GAME-ABILITY boundary after delegation.

## 9. State and scope alternatives

Interaction state should be classified along two independent dimensions: **authority scope** and **lifetime/durability**.

### 9.1 Authority scope

Recommended classes:

- `CHANNEL_LOCAL` — current ChannelRuntime owns the mutable interaction-object instance;
- `INSTANCE_LOCAL` — current InstanceRuntime owns it;
- `WORLD_SHARED_DELEGATED` — the Channel/Instance does not own the mutable state; it invokes/queries a named world owner;
- `FOREIGN_DOMAIN_DELEGATED` — state belongs to Character/Item/Account/Guild/House/Reward/etc. owner and GAME-INTERACTION never stores a shadow authoritative copy.

There is deliberately no generic `GLOBAL` class.

### 9.2 Lifetime/durability

Recommended classes:

- `STATELESS` — result derives from immutable definition/current authoritative inputs only;
- `RUNTIME_EPHEMERAL` — state may reset only at a declared **fresh semantic initialization** boundary;
- `RUNTIME_RECOVERABLE` — state and pending semantic timers/operations must survive same-scope process recovery via checkpoint/replay/reconstruction evidence;
- `DURABLE_DELEGATED` — state survives runtime lifetime under a named durable owner/contract.

A new GameNode/NodeId is never by itself a fresh semantic initialization boundary.

## 10. Reset semantics

A reset is authoritative mutation, not an implementation detail.

Candidate reset policies should include at least:

- `NO_AUTOMATIC_RESET`;
- `AFTER_DURATION`;
- `ON_FRESH_SCOPE_INITIALIZATION`;
- `ON_NAMED_AUTHORITY_EVENT`;
- `DELEGATED_RESET_POLICY`.

For `AFTER_DURATION`, the definition must state whether process-down/offline time counts. If the timer can cross a process lifetime, opaque monotonic instants are insufficient; recoverable semantic timer state is required by FND-03.

A restart/recovery must never reactivate a switch, re-arm a trap or close/open a door merely because process-local memory was lost unless the accepted reset policy explicitly says the semantic scope itself was freshly initialized.

## 11. Cross-domain atomicity analysis

This is the highest-risk boundary.

### 11.1 Local-only interaction transition

If one current runtime owner owns all affected state, the transition can be resolved non-interleaved inside one FND-03 authoritative input and advance interaction state revision deterministically.

### 11.2 Delegated-only interaction

If GAME-INTERACTION owns no state change, it can validate the affordance and issue a typed command to the real owner. Success is whatever the owner commits and returns.

### 11.3 Coupled local + foreign mutation

Examples:

- consume a key **and** open a door;
- charge currency/item **and** teleport;
- disarm a trap **and** mutate an ItemInstance;
- world-shared lever **and** Channel-local overlay transition.

A generic interaction engine must not implement these as:

```text
commit local
-> call foreign owner
-> hope it succeeds
```

when product semantics require one coupled success/failure outcome.

Recommended rule:

- if the semantics require atomic/coupled success, use a **named owner-specific workflow/operation** whose contract defines prepare/commit/retry/reconciliation/fencing; absent such a workflow, fail closed rather than create a partial outcome;
- if product semantics explicitly permit independent outcomes, the definition may commit a local transition and emit a separately auditable delegated action, but this independence must be versioned content/ruleset policy rather than accidental failure behavior.

No generic cross-domain transaction escape hatch is introduced.

## 12. Concurrency and stale work

A mutable interaction object should expose an owner-local monotonically advancing interaction-state revision or equivalent current-state fence.

Examples:

- two players pull the same Channel-local lever: FND-03 `RuntimeExecutionOrdinal` decides order; the second sees the state committed by the first;
- a script/worker/dependency response prepared against object revision 41 returns after revision 42: it must be revalidated and may be rejected as stale/conflicting;
- a world-service completion returns after scope ownership generation changed: the old-generation completion cannot commit;
- a player retries the same `CommandRef`: FND-02/FND-03 duplicate semantics prevent a second interaction effect.

A state revision is not a globally unique entity identity and should not be used as one.

## 13. Trigger cascades and re-entrancy

World mechanisms often chain: switch -> door -> timer -> hazard -> another switch. Unbounded synchronous callback recursion is unsafe.

Recommended rules:

- authored trigger edges are typed and compiler-visible;
- the active resolution uses a bounded deterministic work list, not recursive script callbacks;
- each cascade has a root source occurrence/correlation reference, deterministic step ordering and hard depth/action/fan-out limits;
- duplicate edges/targets have explicit deterministic handling;
- an edge crossing authoritative owner/domain becomes typed asynchronous delegation and later re-enters the destination owner as a new normalized input;
- a script cannot create a private event bus or recursively call arbitrary interaction handlers;
- exceeding any cascade limit fails according to a declared atomicity policy, with no unbounded partial expansion.

Exact numeric limits remain unselected pending registered evidence.

## 14. Movement-triggered semantics

A movement trigger must derive from committed authoritative movement state, not from the original client command.

Candidate ordering:

```text
movement owner commits authoritative movement occurrence
-> emits normalized typed contact/enter/exit occurrence
-> current interaction owner resolves affected interaction definitions
-> deterministic interaction transition/delegation
```

The movement occurrence needs stable retry/replay correlation sufficient to ensure that recovery or duplicate delivery does not trigger the same one-shot semantic occurrence twice.

GAME-INTERACTION does not decide the movement collision/pathfinding algorithm, exact step order or player-position persistence contract.

## 15. Teleport/relocation semantics

A teleporter is an interaction affordance, not a hidden permission to rewrite placement.

The interaction layer may:

- resolve and validate the active portal/teleporter object;
- resolve a versioned destination descriptor from content;
- validate interaction-owned preconditions;
- issue a typed relocation request with exact semantic revision context.

It may not:

- silently choose another Channel when destination is unavailable;
- bypass Channel-switch locks/cooldown/admission;
- directly create an Instance/GameSession/CharacterLease;
- move an actor across authority boundaries without the owning handoff/admission contract;
- charge item/currency value without the owning workflow.

Exact movement-domain owner/API remains a cross-domain gap.

## 16. Traps, fields and hazards

GAME-INTERACTION can own environment-trigger semantics such as:

- armed/disarmed state;
- one-shot versus repeatable trigger state;
- local cooldown/reset state where accepted;
- contact/entry occurrence detection from authoritative movement output;
- typed proposal to apply an effect.

It must not own damage/healing/condition formulas, ability target legality, immunities or combat cost/cooldown semantics. Those remain GAME-ABILITY/combat-owned.

If a trap state transition and effect application must be one semantic all-or-nothing outcome, the final contract must require an explicit coupled workflow; it may not assume a generic transaction exists.

## 17. Readable and writable objects

### Readable

Read access can be represented as deterministic retrieval of immutable content text or current state owned by the target domain. The server supplies authoritative content/revision; the client does not provide the text being "read".

### Writable

Writable objects add untrusted user-controlled text and potential durable moderation/privacy requirements. The interaction layer can safely own:

- target/access/affordance validation;
- payload-format normalization;
- resource-limit checks;
- typed routing to the accepted owner.

It should not unilaterally freeze persistent free-form text ownership, retention, moderation, search/indexing or ItemInstance text mutation. Those require an explicit owner decision.

A purely Channel-local transient writable mechanic is possible only when the content definition explicitly declares that lifetime/scope and no durable/player-owned representation is implied.

## 18. Item-use routing

Item-assisted interactions need one normalized route but no new item authority.

Recommended flow:

```text
player CommandRef
-> normalize item-assisted interaction intent
-> validate actor/current runtime target
-> resolve interactable object
-> obtain typed current item eligibility/location evidence from GAME-ITEM/DUR-03 owner
-> build allowed object-side transition/delegation plan
-> commit through the correct owner/workflow
```

A script or interaction definition may not directly set item owner/location/count/durability/binding/currency value.

If an item invokes an ability, the downstream invocation enters GAME-ABILITY's authoritative pipeline; GAME-INTERACTION does not synthesize the final target/effect plan.

## 19. Script capability boundary

DUR-04 is sufficient as the outer security model if GAME-INTERACTION exposes only typed capabilities.

The interaction host surface should provide bounded operations such as conceptual:

- inspect allowed interaction snapshot fields;
- propose one of registered interaction state transitions;
- propose a typed domain route from an allowlisted capability;
- consume deterministic runtime-provided RNG only when an interaction definition explicitly requires randomness and SIM gives it a purpose stream.

The script must not receive:

- unrestricted world iteration;
- direct mutable object references;
- raw database handles;
- arbitrary command/event dispatch;
- ambient network/filesystem/process/env/secrets;
- wall clock or OS RNG;
- direct item/value/movement/ability mutation;
- ability to invent a world-shared owner;
- private unbounded timers or recursive event loops.

Script trap, invalid proposal or resource exhaustion commits no authoritative plan by default.

## 20. Content revision and migration

Interaction definitions are behavior-affecting content and must be immutable-revision-bound.

Each live interaction object/pending occurrence needs enough semantic revision evidence to identify the exact definition/ruleset/SIM inputs used for authoritative behavior.

Activation/migration should follow DUR-04 classes:

1. compatible/no migration;
2. read-compatible with explicit normalization;
3. explicit migration;
4. incompatible;
5. removed-policy/tombstone handling.

Rules derived:

- a pending timer/operation is not silently reinterpreted under a newer interaction definition;
- same logical retry uses the same behavior-affecting revisions unless an explicit migration/reconciliation contract says otherwise;
- activation of an incompatible revision blocks rather than resetting unknown state silently;
- removing an object with durable or externally coupled state requires explicit migration/retirement semantics;
- an inactive old content revision may remain available long enough to finish/reconcile already-bound work when accepted DUR-04 policy permits it.

## 21. Recovery analysis

### Crash before local commit

No authoritative interaction mutation exists. Retry/recovery resolves the same command/source occurrence using standard duplicate/replay semantics.

### Crash after local commit before client response

The committed state/revision is truth. The same `CommandRef` or source occurrence must reconcile to that result rather than apply the transition again.

### Crash while a delegated operation is pending

Recoverable state must retain enough evidence to know:

- owning semantic scope and generation/fence context;
- interaction object/state revision at prepare time;
- operation/source correlation identity owned by the involved contract;
- exact semantic revision set;
- whether local state was committed, reserved or still pending;
- how duplicate/timeout/cancel/late completion is classified.

If that evidence cannot be reconstructed, the system must fail closed rather than guess a transition.

### Scope owner replacement

A new NodeId does not reset the interaction. Stale old-owner completions/timers are rejected by generation/revision checks.

### World-shared owner unavailable

A Channel-local copy must not become temporary authority. The interaction fails with bounded dependency semantics unless the owning world contract explicitly defines a safe degraded mode.

## 22. Failure vocabulary mapping

GAME-INTERACTION should use the foundation categories rather than inventing incompatible public classes.

| Situation | Foundation category direction |
|---|---|
| malformed verb/payload/target encoding | `INVALID_INPUT` |
| incompatible interaction/content/ruleset revision | `UNSUPPORTED_REVISION` |
| stale runtime/object/generation/revision fence | `STALE_GENERATION` |
| current door/switch/trap state makes transition illegal | `CONFLICT` |
| candidate/cascade/payload/timer/pending-work hard limit hit | `CAPACITY_EXCEEDED` |
| required world/item/movement/ability owner unavailable | `DEPENDENCY_UNAVAILABLE` |
| named delegated workflow deadline expires | `TIMEOUT` |
| explicitly cancelled pending operation | `CANCELLED` |
| unexpected state that cannot be made safe | `INTERNAL_UNAVAILABLE` |

Narrow domain codes may be added later but must map to one category and state mutation/idempotency outcome.

## 23. Resource and anti-abuse analysis

The current shared resource registry has the correct rule — missing externally controlled limits block implementation acceptance — but does not yet provide GAME-INTERACTION-specific numeric maxima.

The final implementation contract will need registered hard maxima for at least:

- interaction command/domain payload bytes below the generic wire ceiling where applicable;
- player interaction attempts accepted per bounded scheduling/rate window;
- interactable candidate enumeration per resolution;
- resolved interaction targets per occurrence;
- writable payload bytes/codepoints and normalization expansion;
- transition actions per plan;
- cascade depth, total steps and fan-out;
- timers per interaction object and per runtime scope;
- pending delegated operations per object/actor/scope;
- script-proposed actions/host calls specific to interaction in addition to DUR-04 limits;
- retained recovery/evidence state per pending occurrence;
- world-service/query result count used by interaction;
- authoring-time state-machine states/transitions/edges per definition where pathological graphs could cause runtime/compiler exhaustion.

Security threats explicitly addressed by these bounds include interaction spam, forged cross-scope object references, stale-ref replay, trigger farming via duplicate movement delivery, recursive trigger bombs, script action amplification, oversized writable text, timer storms and world-shared hotspot amplification.

No numeric maximum is invented by this worker. Because `RESOURCE_LIMITS_REGISTRY.json` is outside Agent D's owned paths, registration is a coordinator/follow-up action before implementation authority.

## 24. Deterministic acceptance scenarios

The candidate contract should require at least the following architecture scenarios:

| Scenario | Required architecture result |
|---|---|
| Two players pull same Channel-local lever | FND-03 order produces one reproducible transition order; second sees first result. |
| Same player CommandRef is retried | no duplicate transition/effect; prior result or explicit duplicate outcome. |
| Movement occurrence crosses one-shot trap | stable source correlation prevents duplicate trigger after retry/recovery. |
| Timer and manual toggle become ready close together | owner assigns authoritative order; replay reproduces it. |
| Script traps/exhausts fuel | no authoritative transition/action plan commits. |
| Script proposes item-location mutation directly | proposal rejected; no local partial commit. |
| Key-assisted door interaction uses stale/moved item | item owner rejects stale eligibility/location; no coupled door success unless an accepted workflow proves it. |
| Same authored lever exists in two Channels as `CHANNEL_LOCAL` | states evolve independently; no process-global bleed. |
| World-shared lever used from two Channels | one named world owner orders state; Channels do not create independent authoritative copies. |
| World-service response returns to superseded runtime generation | stale completion cannot commit. |
| Crash after local commit before response | recovery reconciles committed revision and does not reapply. |
| Content revision changes while timer/operation is pending | pending occurrence remains bound to accepted old revision or follows explicit migration; no silent reinterpretation. |
| Writable payload exceeds hard bound | rejected before retained allocation/domain mutation. |
| Cascade exceeds depth/action/fan-out bound | deterministic bounded failure according to declared atomicity; no unbounded recursion. |
| Teleport target unavailable/incompatible | no silent alternate Channel/destination and no unrelated value consumption. |
| Split Channel ownership | only current generation may commit interaction state. |

Architecture analysis can mark these invariants as specified; executable proof remains `NOT_STARTED`.

## 25. Applicable foundation failure scenarios

Candidate interaction architecture should explicitly consume at least:

- `FS-STALE-GENERATION`;
- `FS-DUPLICATE-COMMAND`;
- `FS-CHANNEL-SPLIT-OWNER`;
- `FS-QUEUE-SATURATION`;
- `FS-CLOCK-SKEW` for timer interpretation;
- `FS-REVISION-MISMATCH`;
- `FS-WORLD-BUNDLE-CORRUPT` before activation;
- `FS-POSTGRES-UNAVAILABLE` when an accepted durable owner is required;
- `FS-DB-OUTBOX-BOUNDARY` when an owning durable workflow requires outbox evidence.

The final candidate should not claim executable PASS evidence; it can only freeze expected invariants.

## 26. Recommended architecture

Adopt Option C:

```text
SOURCE OCCURRENCE
(player CommandRef / committed movement occurrence / due semantic timer /
authorized system event / validated script proposal)
        |
        v
FND-03 NORMALIZED AUTHORITATIVE INPUT
        |
        v
INTERACTION INTENT NORMALIZATION
        |
        v
BOUNDED INTERACTABLE OBJECT RESOLUTION
        |
        v
AUTHORITY + SCOPE + CURRENT-STATE + AFFORDANCE VALIDATION
        |
        v
BOUNDED TYPED INTERACTION PLAN
        |
        +--> local interaction-owned transition
        |
        +--> typed delegated owner operation
        |
        `--> explicit named coupled workflow when atomicity crosses domains
        |
        v
COMMIT / PENDING / STRUCTURED FAILURE
        |
        v
STATE REVISION + RESULT/EVIDENCE
```

No raw callback or script receives direct mutation authority.

## 27. DECISIONS_NOT_TAKEN

This analysis deliberately does **not** decide:

- Reference-specific use/use-with target ordering or stacked-object priority;
- interaction range/LoS/floor/reachability algorithms and values;
- movement/pathfinding/collision implementation;
- teleport cross-scope handoff API or Channel/Instance admission details;
- ability/combat damage/condition/effect semantics;
- AI decision behavior;
- item/location/value transaction semantics already owned by GAME-ITEM/DUR-03;
- generic cross-domain ACID transaction machinery;
- persistent writable-text owner/schema/moderation/retention policy;
- physical content authoring format/WIT/Rust type names;
- scheduler/thread/async runtime implementation;
- exact numeric hard limits/rate windows/timeouts/reset durations;
- client UI/presentation/protocol payload layout;
- PostgreSQL schema/migrations;
- global architecture/status overlays;
- production activation.

## 28. CROSS_DOMAIN_FINDINGS

```yaml
cross_domain_finding:
  id: GAME-INTERACTION-01-XD-ABILITY-TRIGGER
  observed_in_domain: GAME-INTERACTION-01
  target_owner: GAME-ABILITY-01
  severity: P1
  evidence: GAME-ABILITY-01 targeting/legality and cast/channel/commit owner-accepted partial baselines; overall GAME-ABILITY-01 remains open
  conflict_or_gap: Interaction-triggered traps/fields/hazards require a stable typed non-player-origin ability/effect invocation and coupled-failure semantics without allowing GAME-INTERACTION to become a combat/effect owner.
  required_before: executable trap/field/hazard implementation claiming ability/combat effects
  worker_action: REPORT_ONLY
```

```yaml
cross_domain_finding:
  id: GAME-INTERACTION-01-XD-MOVEMENT-RELOCATION
  observed_in_domain: GAME-INTERACTION-01
  target_owner: ARCHITECTURE_COORDINATOR
  severity: P1
  evidence: FND-03 owns runtime execution scope and GAME-CHANNEL-01 owns Channel switching, but the allocated lane provides no canonical GAME-MOVEMENT relocation contract
  conflict_or_gap: Teleporter/portal interaction can validate activation and route descriptors, but exact authoritative position transition, local relocation legality and cross-scope handoff owner/API are not frozen here.
  required_before: executable teleport/portal implementation and movement-trigger integration
  worker_action: REPORT_ONLY
```

```yaml
cross_domain_finding:
  id: GAME-INTERACTION-01-XD-WRITABLE-TEXT
  observed_in_domain: GAME-INTERACTION-01
  target_owner: ARCHITECTURE_COORDINATOR
  severity: P2
  evidence: GAME-ITEM-01 and DUR-04 bound item/content/script authority but do not assign a complete durable player-authored world-text moderation/retention owner in the reviewed scope
  conflict_or_gap: Persistent writable boards/books/signs need explicit durable ownership, moderation/privacy/retention and migration semantics; GAME-INTERACTION can own access/routing but should not create that authority implicitly.
  required_before: persistent player-authored writable-object implementation
  worker_action: REPORT_ONLY
```

```yaml
cross_domain_finding:
  id: GAME-INTERACTION-01-XD-COUPLED-WORKFLOW
  observed_in_domain: GAME-INTERACTION-01
  target_owner: ARCHITECTURE_COORDINATOR
  severity: P1
  evidence: DUR-03 explicitly owns item/value transactional workflows and DUR-04 forbids generic script transaction escape hatches
  conflict_or_gap: Mechanics requiring one semantic outcome across interaction-local state plus foreign movement/item/value/ability/world authority need a named owner-specific operation/workflow; no generic cross-domain interaction transaction is accepted.
  required_before: implementation of consume-key-and-open, charge-and-teleport, or other coupled multi-owner mechanics
  worker_action: REPORT_ONLY
```

```yaml
cross_domain_finding:
  id: GAME-INTERACTION-01-XD-RESOURCE-LIMITS
  observed_in_domain: GAME-INTERACTION-01
  target_owner: ARCHITECTURE_COORDINATOR
  severity: P1
  evidence: docs/contracts/RESOURCE_LIMITS_REGISTRY.json requires registered hard maxima before implementation acceptance; Agent D does not own that shared registry
  conflict_or_gap: Interaction-specific candidate, cascade, writable-payload, timer, pending-operation and rate ceilings still require numeric evidence and registry entries.
  required_before: GAME-INTERACTION implementation acceptance
  worker_action: REPORT_ONLY
```

## 29. Final analysis recommendation

**RECOMMENDATION:** freeze a bounded server-authoritative world-interaction contract based on typed interaction state machines and explicit delegation. Permit rich content through immutable data and bounded DUR-04 script proposals, but never permit a world-object handler to become authority over arbitrary movement, items, value, abilities or world-shared state.

The contract can be made architecture-complete now while intentionally leaving Reference-specific behavior, movement-owner details, durable writable text, multi-owner workflow contracts and numeric limits unresolved/fail-closed for their proper owners.

MERGE_AUTHORITY: ARCHITECTURE_COORDINATOR_ONLY
