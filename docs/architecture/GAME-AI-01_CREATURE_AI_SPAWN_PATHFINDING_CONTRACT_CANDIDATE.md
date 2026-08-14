# GAME-AI-01 — Creature AI, Spawn and Pathfinding Contract Candidate

- DecisionStatus: `PROPOSED`
- DeliveryStatus: `OPEN`
- ImplementationStatus: `NOT_STARTED`
- Gate: `GAME-AI-01`
- Issue: `#261`
- Analysis source: `GAME-AI-01_CREATURE_AI_SPAWN_PATHFINDING_ANALYSIS.md`
- Scope: paper-only candidate contract; executable runtime/content authority **NONE**
- Canonicality: **NONCANONICAL WORKER PROPOSAL** until Architecture Coordinator acceptance/merge
- Merge authority: `ARCHITECTURE_COORDINATOR_ONLY`

## 1. Contract purpose

This contract freezes the candidate semantic boundary for authoritative creature AI, controlled-actor AI, NPC-local behavior, spawn/population control and pathfinding.

It composes with and does not replace:

- FND-03 runtime ownership/order/timer/auxiliary-work semantics;
- SIM-DETERMINISM-01 arithmetic/RNG/order/replay semantics;
- GAME-CHANNEL-01 simulation and value-source multiplicity/eligibility semantics;
- DUR-04 immutable content and proposal-only scripting semantics;
- accepted partial GAME-ABILITY targeting/cast/cooldown/effect boundaries;
- GAME-ITEM/DUR-03 value conservation and downstream reward authority;
- future GAME-INTERACTION/event/encounter contracts.

`MUST`, `MUST NOT`, `SHOULD` and `MAY` are normative inside this candidate. Unknown Reference behavior and absent numeric safety limits fail closed for implementation/parity claims.

## 2. Core authority invariant

For every local runtime AI actor:

```text
one current semantic ChannelRuntime or InstanceRuntime owner
+ one current scope ownership generation
+ one bounded deterministic AI state
+ one exact behavior-affecting revision/provenance set
-> one authoritative mutation boundary
```

No path worker, script worker, client, database callback, analytics system or foreign runtime may commit actor/spawn mutation directly.

Public-world AI/spawn state belongs to the current `ChannelRuntime`. Instance-local AI/spawn state belongs to the current `InstanceRuntime`. A world/event owner MAY coordinate a world-scoped occurrence, but local actor mutation still enters through the local runtime owner as normalized authoritative input.

## 3. Runtime AI versus investigation AI

The read-only Oteryn Game Intelligence / Investigation AI defined under ADR-0006 MUST remain outside runtime mutation authority.

Its output MAY inform human review, testing or future authored changes through separately governed workflows. It MUST NOT directly:

- pick a live creature target;
- alter live AI state;
- spawn/despawn actors;
- modify path results;
- grant/revoke loot or XP;
- change live behavior/template/population policy.

## 4. Immutable actor/template/spawn provenance

Every active actor MUST be bound to an exact immutable semantic provenance set sufficient to reproduce and attribute behavior.

The implementation may choose different physical type names, but the semantic set MUST include, as applicable:

- semantic runtime scope;
- current scope ownership generation;
- actor identity plus runtime-local generation/fence where local handles can recycle;
- exact World Bundle artifact digest;
- `content_revision`;
- `map_revision` or compatible navigation-data revision;
- `ruleset_revision`;
- `world_policy_revision`;
- `SimulationDeterminismProfileRevision`;
- stable spawn/source `ContentKey` plus immutable package/revision provenance;
- stable behavior-template `ContentKey` plus immutable package/revision provenance;
- script artifact/WIT/`script_execution_profile_revision` when a script leaf is used;
- stable source/event occurrence context when retry/recovery/reward behavior depends on it;
- validated controller/principal context for controlled actors when required.

Runtime index, file path, display name, NodeId, worker identity or mutable "latest" lookup MUST NOT substitute for semantic provenance.

A retry, delayed completion or recovery MUST NOT silently reinterpret an already accepted semantic occurrence under a newer incompatible behavior/content/ruleset/SIM/script revision.

## 5. Behavior execution model

### 5.1 Selected model

The v1 authoritative behavior execution model MUST be a **typed bounded finite state machine (FSM)**.

A behavior template MUST declare a finite validated graph containing:

- finite state keys;
- finite typed transition triggers;
- finite guards;
- finite typed actions/proposals;
- deterministic initial/fallback semantics;
- deterministic ordering/tie-break when more than one transition is eligible;
- hard bound on transitions/actions executed during one authoritative resolution.

The FSM MUST NOT recursively dispatch unlimited transitions. Exceeding a semantic execution bound MUST produce a deterministic failure/fallback before additional mutation.

Exact gameplay state names remain content/profile-owned; this contract does not assert Reference state names.

### 5.2 Behavior-tree decision

A runtime behavior tree is **not selected** for v1 and no behavior-tree framework/library receives architecture authority.

A later contract MAY accept behavior-tree authoring only with evidence of need and either deterministic lowering into the accepted bounded execution semantics or an equivalently explicit bounded node/depth/visit/continuation/abort contract.

Unbounded or scheduler-sensitive tree traversal is prohibited.

### 5.3 Script leaves

A DUR-04 authoritative component MAY act only as a bounded proposal leaf:

```text
snapshot-bound immutable inputs + explicit capabilities
-> bounded script/component
-> typed proposal
-> FSM/host/domain validation
-> owner commit or rejection
```

A script MUST NOT directly move actors, mutate arbitrary AI/server objects, write SQL, mint value or bypass downstream legality.

## 6. AI scheduling and time

GAME-AI MUST NOT introduce a universal fixed simulation tick.

Mutation-capable think/evaluation, spawn and respawn occurrences MUST enter as FND-03 owner-scoped normalized inputs/timers and receive owner-local `RuntimeExecutionOrdinal` when accepted.

Gameplay durations/deadlines MUST use FND-03/SIM normalized monotonic/semantic time, not direct wall-clock reads.

Every repeating AI/spawn timer family MUST declare a bounded FND-03 catch-up policy. A `SKIP_TO_LATEST`-like policy MAY be used only when skipped evaluations are semantically equivalent and cannot suppress required gameplay occurrences.

## 7. Perception and candidate enumeration

Perception/candidate enumeration MUST operate on a bounded snapshot compatible with the current authoritative actor state.

The pipeline MUST:

1. enumerate within a bounded spatial/query domain;
2. canonicalize ordering before any order-sensitive policy;
3. filter invalid scope/lifecycle/local-generation/perception candidates;
4. reject stale snapshot/revision evidence before commit;
5. return a typed capacity/failure outcome when required safe bounds cannot be honored.

Storage iteration order, pointer order, worker completion order and database default ordering MUST NOT become target priority.

Arbitrary truncation by unspecified backing-store order is prohibited. If a candidate set can exceed a hard bound, the policy MUST define a deterministic bounded selection rule or fail that decision safely.

## 8. Aggro, threat, memory and target selection

The target-selection pipeline MUST be:

```text
bounded eligible candidates
+ bounded current target/threat/stimulus memory
-> versioned policy score/priority
-> stable semantic tie-break
-> retain/switch/clear target decision
-> downstream action intent
```

Rules:

- threat/stimulus memory MUST be bounded in entries and semantic lifetime;
- score arithmetic MUST obey SIM numeric/formula semantics;
- current target MUST be revalidated before movement/action commit;
- equal-priority candidates MUST use a stable semantic comparator;
- stale local generations MUST fail closed;
- client input MAY cause a validated stimulus/controlled-actor request but MUST NOT directly author threat score/target authority;
- action legality remains downstream-owned;
- rejection from downstream legality MUST NOT trigger unbounded immediate retry.

Exact Reference aggro, threat, retarget and memory values remain evidence-gated.

## 9. Leash/home/reset semantics

A behavior policy MAY define bounded home/leash/reset behavior using validated spawn/home context.

The exact trigger radius, timer, path, target-clearing behavior, HP/condition restoration and reward-contribution effects MUST be versioned/profile-owned and Reference-evidenced where parity is claimed.

GAME-AI MUST NOT mutate foreign combat/reward state merely because the FSM enters a reset state. Required reset effects MUST pass through the owning typed domain boundary.

## 10. Pathfinding contract

### 10.1 Ownership

Potentially expensive pathfinding MUST execute as bounded FND-03 auxiliary work, never as an unbounded operation on the authoritative writer.

A path request MUST bind enough immutable/revalidatable context to detect stale work, including:

- semantic scope;
- scope ownership generation;
- actor local generation;
- source actor/state revision;
- start and normalized movement-capability profile;
- goal plus goal-revision evidence;
- map/navigation/content revision;
- ruleset/SIM/behavior revision context;
- logical work identity;
- applicable budget profile.

The worker output is a proposal only.

### 10.2 Revalidation

Before adopting a returned route, the current owner MUST revalidate:

- current scope and ownership generation;
- actor existence/local generation;
- source/goal compatibility;
- map/navigation revision compatibility;
- request still pending/current;
- cancellation/supersession state;
- current movement legality;
- declared resource bounds.

Stale/late/misrouted route proposals MUST be discarded without rollback.

### 10.3 Deterministic search profile

Any accepted path implementation MUST have a versioned deterministic search profile defining at least:

- neighbor enumeration;
- movement/cost semantics;
- equal-cost tie-break;
- bounded termination;
- route normalization/canonicalization;
- supported authoritative target compatibility.

No concrete algorithm/library is selected by this contract.

### 10.4 Repath and terminal results

Repath MUST require a typed deterministic trigger such as compatible target/goal change, route invalidation, accepted semantic deadline or movement failure.

Path work MUST terminate with a typed semantic class equivalent to:

- route found;
- no route;
- search budget exhausted;
- cancelled/superseded;
- stale generation/revision;
- capacity unavailable;
- invalid/unsupported request.

Failure MUST enter a finite deterministic AI fallback. Infinite retries are prohibited.

## 11. Spawn definition contract

Every spawn/population source MUST be immutable authored content identified by stable content/package provenance and compiled into the locked World Bundle.

A spawn definition MUST declare, as applicable:

- simulation scope;
- actor/template and behavior-template references;
- bounded population constraints;
- placement domain and deterministic candidate/selection semantics;
- respawn/timer semantics;
- occupancy handling;
- recovery class;
- required semantic revisions/capabilities;
- GAME-CHANNEL multiplicity/eligibility classification for value-producing sources.

Unresolved references, invalid bounds, incompatible revisions/capabilities or missing mandatory multiplicity/eligibility policy MUST block compilation/staging/activation as appropriate.

There MUST be no permissive runtime fallback multiplicity class for a value-producing source.

## 12. Spawn occurrence and idempotency

Every accepted spawn occurrence MUST carry enough stable semantic occurrence context to distinguish:

- idempotent retry/replay of the same occurrence;
- recovery continuation of the same occurrence;
- a genuinely later new occurrence.

This contract does not mandate a new global UUID. The owning source/event model MAY supply the stable occurrence identity.

NodeId, wall-clock timestamp or current ownership generation alone MUST NOT be used as a durable reward/source identity.

A fenced old owner MUST NOT publish a new spawn/death/reward-relevant occurrence after authority moved.

## 13. Spawn occupancy and finite placement

Spawn commit MUST revalidate current spatial legality/occupancy.

When the primary placement is unavailable, the authored policy MUST choose one finite deterministic behavior:

- bounded canonically ordered alternatives;
- one bounded postponed retry/deadline; or
- explicit skip/failure of that occurrence.

Unbounded random probing and recursive immediate spawn retries are prohibited.

## 14. Dynamic population/ecology policy

Dynamic population or ecology behavior MAY exist only through an explicit immutable/versioned authored policy. It MUST NOT be an implicit feedback loop hidden in runtime heuristics.

A dynamic population policy MUST define, as applicable:

- exact authoritative scope and owner;
- bounded population minima/maxima and mutation step/rate semantics;
- bounded evaluation cadence and FND-03 timer/catch-up policy;
- normalized authoritative input facts it may consume;
- deterministic arithmetic/tie-break/RNG semantics under SIM;
- GAME-CHANNEL multiplicity/eligibility consequences for value-producing sources;
- recovery/checkpoint semantics;
- hard resource limits and overload disposition.

Analytics/Game Intelligence output MUST NOT directly change population authority. A human-reviewed future policy revision MAY use analytics evidence as design input, but live runtime mutation still requires an accepted versioned policy and ordinary authoritative inputs.

For Reference, adaptive population/ecology behavior MUST remain disabled/unclaimed unless target evidence proves the exercised behavior. For Evolved, an adaptive policy MAY be an explicit declared difference, but it retains the same deterministic, bounded, provenance and anti-duplication requirements.

## 15. Spawn/AI recovery classes

Every source/encounter whose process-failure behavior can affect gameplay/value MUST choose an explicit recovery class:

### `EPHEMERAL_SCOPE_RESET`

State reconstructs from immutable content on scope activation. This is valid only when product/economy semantics explicitly allow reset and reset cannot create forbidden duplicate availability/value.

### `CHECKPOINTED_RUNTIME_CONTINUITY`

Future-determining AI/spawn state participates in deterministic checkpoint/replay evidence and resumes/reconciles under a valid new owner generation.

### `DURABLE_EVENT_OCCURRENCE`

A named event/world owner persists semantic occurrence/eligibility and local AI consumes typed projection facts.

A high-impact or value-producing source MUST NOT silently default to ephemeral reset when that can duplicate or erase semantic eligibility.

## 16. Deterministic state and recovery

AI/spawn state that can change a future authoritative result without a new external normalized fact MUST be represented in SIM deterministic state/checkpoint/replay evidence.

This includes, as applicable:

- current FSM state;
- bounded threat/target memory;
- current target evidence;
- accepted semantic timers;
- source population/occurrence state;
- behavior-affecting revision bindings;
- stateful RNG/substream cursor if used;
- pending logical path/continuation identity when it affects future behavior;
- controlled-actor principal/command state when behavior-affecting.

Derived route caches MAY be recomputed after recovery only when the deterministic search/profile plus retained semantic inputs guarantee equivalent normalized result. Old-generation worker results are always stale.

## 17. Gameplay RNG

AI/spawn gameplay randomness MUST comply with SIM:

- deterministic/replayable under the active SIM profile;
- purpose-isolated from unrelated mechanics;
- retry-stable for one logical occurrence;
- not process-global;
- not seeded from NodeId/thread/pointer/unordered iteration;
- exploit-sensitive seed/root/substream evidence protected from clients/ordinary telemetry where required.

Random placement/selection, when a profile actually defines it, MUST have stable decision identity and bounded candidate space. This contract does not claim any Reference AI/spawn decision is random without evidence.

## 18. Controlled actors: summons and pets

A summon/pet remains a server-authoritative runtime actor owned by its current Channel/Instance simulation.

Any player/controller command MUST be a validated normalized input and bind current control-right evidence. The client MUST NOT directly set authoritative position, target, damage, threat, lifetime or reward credit.

Controlled actors MUST retain enough controller/principal provenance for downstream attribution and stale-control rejection.

Exact ownership persistence, summon lifetime, command vocabulary, XP/loot attribution and despawn rules remain downstream/profile-owned.

## 19. NPC-local AI

NPC idle/movement/perception MAY reuse the bounded FSM/pathing kernel.

Dialogue, trade, quest, bank, economy or other durable business semantics MUST remain with their owning domains. Scripted interaction uses DUR-04 proposal semantics and cannot obtain ambient database/network/global-game authority.

## 20. Boss/encounter composition

Actor-local phase state MAY reside in the actor FSM only when its semantic scope is genuinely actor-local.

Multi-actor objectives, world events, durable encounter occurrence, shared eligibility and reward settlement require a named encounter/event owner. GAME-AI consumes typed phase/occurrence facts and emits bounded actor intents; it MUST NOT invent cross-owner atomicity.

## 21. Combat/action boundary

GAME-AI MAY select a typed action/ability intent. It MUST NOT directly:

- apply damage/healing;
- bypass target/range/line-of-sight/cast legality;
- change cooldown/charge/condition state;
- execute effect recursion;
- declare death independent of combat authority.

The downstream authoritative domain validates/commits/rejects the intent under its accepted semantics. AI consumes the typed outcome as a later/current owner input according to that domain contract.

## 22. Loot, XP and reward boundary

GAME-AI MUST NOT mint item instances, currency, XP or reward eligibility directly.

Downstream reward logic MAY consume exact AI/spawn/controller provenance together with authoritative combat/death/contribution facts.

Where one AI-controlled source can produce durable value, the composed system MUST prove:

- one semantic death/source occurrence settles at most once under retry/replay/recovery;
- source multiplicity/eligibility is explicit under GAME-CHANNEL;
- stale old-generation work cannot settle value;
- client-authored contribution is not trusted;
- controlled-actor contribution maps through a defined principal model without accidental duplicate credit;
- leash/reset/despawn does not erase/fabricate reward eligibility outside the reward owner.

Exact reward formulas/thresholds/transaction identities are foreign-domain decisions.

## 23. Overload semantics

GAME-AI MUST preserve FND-03 owner/control progress under overload.

At minimum:

- AI/path/spawn queues and pending sets are bounded;
- expensive search/planning uses bounded auxiliary capacity;
- actors cannot spawn unbounded worker tasks;
- candidate/memory/threat collections are bounded;
- capacity exhaustion returns a typed deterministic result/failure;
- ordinary AI traffic cannot consume the control/fencing reserve;
- semantic accepted timers/actions are not silently dropped;
- best-effort precomputation may drop/coalesce only when outcome-equivalent;
- overload policy cannot silently create an undocumented Reference/Evolved behavior change.

## 24. Mandatory resource-limit dimensions

Concrete numeric ceilings are intentionally not guessed here. Before implementation acceptance, the shared resource-limit registry or an accepted superseding machine-readable registry MUST contain hard maxima, units, failure categories, rationale and boundary tests for at least:

1. active AI actors per authoritative scope;
2. FSM states/transitions per template;
3. transitions/actions per owner resolution;
4. memory/threat/stimulus entries per actor;
5. perception/target candidates per decision;
6. pending AI timers/operations per actor/scope;
7. queued/in-flight path requests per actor/scope/executor;
8. path search work/nodes per request;
9. route length/result bytes;
10. repath/retry work over a bounded semantic window;
11. spawn sources/controllers per scope;
12. spawn population and placement candidates/attempts;
13. dynamic population-policy evaluations/changes per scope/window where enabled;
14. controlled-actor command backlog;
15. inherited script fuel/memory/host-call/query/proposal bounds;
16. replay/diagnostic volume where amplification-prone.

Absent required limits block executable `GAME-AI-01` implementation acceptance.

## 25. Fail-closed matrix

| Condition | Required result |
|---|---|
| malformed/unresolved AI or spawn definition | reject compile/staging/activation |
| incompatible behavior/content/ruleset/SIM/script revision | reject activation or explicit transition/reconciliation; never reinterpret silently |
| runtime actor/template inconsistency | quiesce/fail affected actor/source without default mutation |
| FSM bound exceeded | deterministic failure/fallback; no recursive extra work |
| candidate bound cannot be honored safely | deterministic query/decision failure unless explicit canonical bounded-selection policy exists |
| path capacity unavailable | reject/defer request through typed bounded AI policy |
| search budget exhausted | typed terminal path failure; no partial route authority |
| stale path result | discard proposal |
| actor recycled/despawned | local-generation mismatch rejects late work |
| script trap/fuel exhaustion/invalid proposal | zero proposal mutation committed |
| ownership generation changed | old work/timers cannot publish authority |
| missing future-determining recovery state | no claim of equivalent recovery until fail-closed reconciliation |
| missing value-source multiplicity/eligibility | block source activation |
| dynamic population policy missing bounds/provenance | block adaptive behavior; no implicit heuristic fallback |
| Reference behavior `UNKNOWN/CONFLICT/PENDING` | no `PARITY_CONFIRMED` claim and no guessed Reference enablement |

## 26. Reference/Evolved contract

One engine MUST support both profiles without forks.

### Reference

- uses the accepted immutable first Reference target cut;
- each exercised behavior requires sufficient evidence status before parity claim;
- exact aggro/threat/leash/path/spawn/controlled-actor/reward semantics remain `UNKNOWN` where this package lacks evidence;
- OTS similarity, current live behavior after the target cut or library defaults cannot fill evidence gaps.

### Evolved

- MAY intentionally use different versioned behavior/path/spawn/population policies;
- each intentional difference MUST be explicit, reviewable and revision-bound;
- Evolved MUST preserve the same authority, deterministic replay, bounded resource, provenance and anti-duplication invariants.

## 27. Required acceptance evidence for a future implementation

An implementation cannot claim this gate until tests/evidence prove at least:

1. deterministic FSM result under identical state/input/revisions;
2. deterministic target tie-break under shuffled backing order;
3. stale target/local-generation rejection;
4. no legality bypass after downstream action rejection;
5. hard FSM transition/action bound;
6. script failure commits zero authoritative mutation;
7. deterministic path result for identical search input/profile;
8. stale path result rejection after owner-generation change;
9. stale path result rejection/revalidation after goal/map revision change;
10. path budget exhaustion without writer stall;
11. actor despawn/recycled-slot safety with in-flight path work;
12. bounded deterministic spawn placement/occupancy fallback;
13. value-source activation failure when multiplicity/eligibility is absent;
14. dynamic population policy cannot exceed declared bounds or change from analytics feedback without a versioned policy revision;
15. recovery fences old owner and reconstructs one semantic occurrence;
16. deterministic replay/state-hash continuity for future-determining AI/spawn state;
17. controlled-actor command remains a request, not client authority;
18. controlled-actor attribution cannot create duplicate reward through duplicate representation;
19. overload preserves owner control/fencing serviceability and hard queue bounds;
20. Reference unknown/pending fixture cannot be promoted to parity by implementation convenience;
21. incompatible immutable revision activation fails closed.

## 28. Explicitly unresolved decisions

The candidate intentionally leaves unresolved until the owning evidence/implementation gate:

- exact Reference perception/aggro/threat/retarget/memory/leash semantics;
- exact path algorithm and library;
- exact Reference movement/path cost/tie/corner/obstacle behavior;
- exact spawn count/timer/occupancy/recovery values;
- exact dynamic ecology/population behavior, if any, for Reference;
- exact summon/pet command, persistence and reward rules;
- exact NPC interaction behavior;
- exact boss/world-event occurrence ownership APIs;
- exact loot/XP/contribution policy;
- physical content schema/serializer for AI/spawn definitions;
- all numeric AI/path/spawn limits;
- future need for behavior-tree authoring.

These unknowns do not authorize permissive defaults.

## 29. Cross-domain findings — report only

- `CROSS_DOMAIN_FINDING / REPORT_ONLY — GAME-ABILITY`: provide/confirm a stable typed AI action-intent and rejection boundary during whole-gate reconciliation.
- `CROSS_DOMAIN_FINDING / REPORT_ONLY — GAME-INTERACTION`: define normalized route invalidation/door/teleport/environment interaction facts without moving interaction authority into AI.
- `CROSS_DOMAIN_FINDING / REPORT_ONLY — GAME-ITEM/DUR-03/REWARD`: define exact controlled-actor contribution and idempotent reward-settlement semantics.
- `CROSS_DOMAIN_FINDING / REPORT_ONLY — EVENT/ENCOUNTER`: assign durable world-shared boss/event occurrence/eligibility ownership where needed.
- `CROSS_DOMAIN_FINDING / REPORT_ONLY — RESOURCE LIMITS`: register concrete AI/path/spawn hard maxima before implementation acceptance.
- `CROSS_DOMAIN_FINDING / REPORT_ONLY — ANL`: define any durable AI-decision evidence schema/retention through ANL rather than a parallel AI audit system.

## 30. Acceptance matrix for coordinator audit

| Requirement | Candidate disposition | Status |
|---|---|---|
| creature/summon/pet/NPC runtime ownership | current Channel/Instance owner; controlled/NPC foreign business state remains external | `PROPOSED` |
| spawn/template provenance | exact bundle/content/template/source/revision context mandatory | `PROPOSED` |
| behavior representation | typed bounded FSM selected; BT deferred; script proposal-only | `PROPOSED` |
| perception/aggro/targeting/memory | bounded deterministic pipeline; exact Reference tuning evidence-gated | `PROPOSED` |
| leash/reset | optional typed bounded policy; foreign mutations routed to owners | `PROPOSED` |
| pathing | auxiliary proposal, deterministic profile, cancellation/stale rejection | `PROPOSED` |
| spawn/population/respawn/occupancy | immutable content, finite placement, explicit recovery class | `PROPOSED` |
| dynamic population/ecology | versioned bounded policy only; no analytics-directed live mutation | `PROPOSED` |
| channel/world source scope | GAME-CHANNEL multiplicity/eligibility preserved | `PROPOSED` |
| boss/encounter extension | actor-local only unless named event/encounter owner | `PROPOSED` |
| controlled actor attribution | validated principal provenance; exact reward rules downstream | `PROPOSED` |
| loot/reward abuse boundary | AI cannot mint; one-occurrence/dedup invariants required downstream | `PROPOSED` |
| overload degradation | hard bounds; no writer/control starvation or unbounded retry | `PROPOSED` |
| Reference/Evolved mapping | common invariants + fail-closed Reference evidence gaps | `PROPOSED` |
| runtime implementation | none authorized | `NOT_STARTED` |

## 31. Candidate conclusion

This candidate chooses the smallest explicit execution model that satisfies current Oteryn authority, determinism, content, multichannel and safety contracts: bounded FSMs inside the current runtime owner, with pathfinding and optional scripts restricted to revalidated proposals.

Dynamic population/ecology is permitted only through explicit bounded versioned policy and can never become a direct analytics-to-runtime control loop.

No executable implementation, framework/library choice, numeric limit or unproven Reference mechanic is authorized by this document.

`MERGE_AUTHORITY: ARCHITECTURE_COORDINATOR_ONLY`
