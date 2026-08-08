# FND-03 — Authoritative Runtime Execution Contract

- Status: Candidate architecture contract; canonical when merged to `main`
- Date: 2026-08-08
- Gate: `FND-03`
- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Repository: `blakinio/Oteryn-v2`
- Consumes:
  - `FND-03_AUTHORITATIVE_RUNTIME_EXECUTION_ANALYSIS_BASELINE.md`
  - `FND-03_RUNTIME_LIFECYCLE_FAILURE_AND_REPLAY_ANALYSIS_BASELINE.md`
  - ADR-0001, ADR-0009, FND-ID-01 and FND-02
  - accepted instance ownership and disconnect/re-entry baselines
  - `RESOURCE_LIMITS_REGISTRY.json`
  - `FOUNDATION_ERROR_VOCABULARY.md`
  - `FOUNDATION_FAILURE_SCENARIOS.md`
- Does not authorize: Rust runtime implementation, production traffic, FND-04 admission/lease implementation, persistence schema, production orchestration, production telemetry, client diagnostics, Launcher/Guardian implementation or external-repository writes

## 1. Purpose

`FND-03` freezes the minimum authoritative execution semantics required before the Rust GameNode runtime can be implemented without guessing ownership, ordering, clock, timer, overload, stale-work or recovery behavior.

The runtime is deliberately multithreaded, but authoritative mutation remains single-owner by semantic scope:

```text
GameNode process / NodeRuntime
    |
    +-- ChannelRuntime A ------ one current logical mutation owner
    +-- ChannelRuntime B ------ one current logical mutation owner
    +-- InstanceRuntime X ----- one current logical mutation owner
    +-- InstanceRuntime Y ----- one current logical mutation owner
    |
    +-- bounded async I/O
    +-- bounded auxiliary CPU work
    `-- typed access to WorldServices owners
```

Different authoritative scopes may progress concurrently. One scope may use many execution resources for non-authoritative work. At no point may two tasks/threads/processes independently commit authoritative mutation for the same current channel or concrete instance.

This contract defines correctness semantics, not a library or thread topology.

## 2. Decision timing

### Must decide now — YES

The following must be frozen before authoritative runtime implementation:

- `NodeRuntime`, `WorldServices`, `ChannelRuntime` and `InstanceRuntime` responsibility boundaries;
- semantic identity versus ownership generation versus process placement;
- one authoritative input linearization boundary per scope;
- relationship between FND-02 command order and runtime execution order;
- wall-clock versus monotonic elapsed time versus authoritative execution order;
- timer authority, stale-timer rejection and catch-up policy requirements;
- auxiliary work proposal/revalidation rules;
- bounded queue/work classes and overload behavior;
- lifecycle activation, drain, checkpoint cut, fencing and recovery boundaries;
- deterministic gameplay randomness/replay evidence requirements;
- FND-03/FND-04 liveness interface;
- foundation failure/error dispositions;
- downstream ownership boundaries.

### Concrete work blocked

This contract blocks:

- authoritative GameNode/ChannelRuntime/InstanceRuntime implementation claims;
- runtime integration of FND-02 command ingress;
- runtime-dependent movement, combat, AI and instance vertical-slice implementation;
- the FND-04 contract where reconnect/session semantics depend on runtime liveness/fencing interfaces;
- production-oriented recovery/orchestration contracts that require an exact runtime fence boundary.

### What becomes harder if wrong

If operating-system scheduling, process-global mutable state, unbounded queues, wall-clock timers or direct asynchronous callbacks become implicit gameplay authority, later correction would require rewriting gameplay ordering, reconnect/recovery, anti-duplication evidence and test/replay infrastructure.

If benchmark-sensitive capacities or implementation technology are frozen without evidence, the project would create unnecessary migration cost and false performance assumptions.

### Evidence that may supersede this contract

A later ADR/contract may change a specific FND-03 choice only with named evidence such as:

- representative latency/capacity benchmarks;
- deterministic replay failures;
- split-owner or stale-work security findings;
- recovery/fault-injection evidence;
- profiling proving a current boundary causes unacceptable contention;
- changed product requirements that cannot be represented through the accepted extension points.

### Deliberately not decided

FND-03 does not select:

- Tokio or another async runtime;
- worker-pool implementation;
- OS thread count, worker count or CPU affinity;
- a fixed global simulation tick/quantum;
- benchmark-sensitive numeric internal capacities;
- PostgreSQL schema, isolation level or checkpoint encoding;
- journal technology, RPO or RTO;
- orchestrator/deployment product;
- heartbeat cadence/hysteresis or reconnect credentials;
- gameplay-specific simultaneous-action formulas;
- analytics/event transport backend.

## 3. Canonical runtime roles

### 3.1 `NodeRuntime`

`NodeRuntime` is the process-incarnation supervisor and bounded execution host for one canonical `NodeId`.

It owns process-scoped concerns such as:

- bootstrap/shutdown coordination;
- hosted-scope registry;
- current scope-assignment/ownership-grant consumption;
- immutable revision/configuration availability checks;
- bounded scheduling/executor capacity;
- bounded auxiliary CPU capacity;
- asynchronous transport/service/persistence adapters;
- process-level health, readiness and resource-pressure observations;
- orderly scope drain and process shutdown coordination.

`NodeRuntime` does **not**:

- directly mutate channel/instance gameplay state;
- become authority for all world-shared domains;
- infer gameplay scope from a mutable process-global `current_world` or equivalent singleton;
- obtain channel/instance authority merely because the process has a valid `NodeId`;
- expose unrestricted mutable references between authoritative scopes.

A fresh GameNode process receives a fresh `NodeId`. `NodeId` identifies one process incarnation; it is not a scope ownership fence.

### 3.2 `WorldServices`

`WorldServices` is a typed access boundary to explicitly owned world-scoped domains, not one mutable process-global service object.

Each world-shared mutable domain retains its own semantic owner and consistency contract. Examples may include communication/presence, party/guild state, market, rewards, house/shared-event state or other domains already classified by the multichannel scope matrix.

Rules:

- every operation carries explicit semantic scope such as `WorldId` and any required character/account/domain identity;
- channel/instance code receives typed commands/queries and bounded asynchronous outcomes rather than mutable world-state references;
- an in-process implementation must preserve the same semantic ownership/failure boundary required if later extracted to another process;
- immutable ruleset/content/World Bundle context is explicit revisioned input, not mutable `WorldServices` state;
- FND-03 does not absorb FND-04 session/lease authority, DUR persistence authority, ANL event/audit authority or later social/economy gameplay ownership.

### 3.3 `ChannelRuntime`

Each active `WorldId + ChannelId` has at most one current logical authoritative mutation owner.

`ChannelRuntime` owns channel-local public-world simulation classified by accepted architecture, including channel-local positions/visibility, creatures/spawns/AI, combat/transient effects, public-map mutable overlay, ground/corpse/transient loot state, local NPC runtime, proximity speech and channel-local timers/events/encounters where applicable.

All channel-local authoritative mutation crosses the scope execution boundary defined below.

### 3.4 `InstanceRuntime`

Each active `WorldId + InstanceId` has at most one current logical authoritative mutation owner.

After a committed Channel↔Instance handoff, `InstanceRuntime` owns the participating characters' instance-local position/combat/encounter state and all instance-local creatures/hazards/objectives/timers/transient items. Source channels no longer mutate that state.

An instance is not a hidden channel. Its identity does not include origin `ChannelId` or current `NodeId`.

### 3.5 Shared correctness kernel

`ChannelRuntime` and `InstanceRuntime` use one runtime correctness model:

```text
semantic scope identity
+ current ownership generation
+ bounded normalized inputs
+ one non-interleaved authoritative resolution boundary
+ monotonic timers
+ stale-work validation
+ lifecycle/fence/recovery hooks
```

Their domain state differs; their authority semantics do not.

## 4. Identity, ownership generation and placement

Runtime correctness uses three separate concepts:

```text
semantic scope identity
+ current scope ownership generation
+ current NodeId/process placement
```

### 4.1 Semantic scope identity

Canonical runtime scope references are conceptually:

```text
ChannelScopeRef  = WorldId + ChannelId
InstanceScopeRef = WorldId + InstanceId
```

These identities may survive eligible recovery or relocation.

### 4.2 Scope ownership generation

Every authoritative ChannelRuntime/InstanceRuntime activation is bound to a current non-reused **scope ownership generation** for that semantic scope.

Binding semantics:

- a generation is a fencing/ordering value, not an entity identity;
- only the currently accepted generation may publish authoritative mutation/results/readiness or submit generation-fenced durable work for that scope;
- recovery/replacement establishes a strictly newer generation;
- a generation is never reused after authority has moved beyond it;
- stale generations fail closed;
- wrap/exhaustion may never reuse an older value;
- if an implementation representation could exhaust, the scope must fail safe and require an explicit lifecycle/ownership recovery path rather than wrapping.

A GameNode/NodeRuntime **cannot self-grant or self-advance authoritative scope ownership** simply by starting or incrementing a local counter. A current ownership grant/fence must be established by the accepted scope-assignment/ownership authority. `OPS-CHANNEL-01` and durability/lease contracts finalize its physical storage/distribution/atomic fencing mechanism.

### 4.3 Placement

`NodeId` records the process incarnation currently hosting a scope. A scope may preserve ChannelId/InstanceId while NodeId and ownership generation change.

Process placement never changes the semantic scope of data or services.

### 4.4 Runtime-local handles

Runtime-local entity/task/timer handles may be compact, but recycled slots require a local generation/version so stale handles cannot target a replacement object.

Runtime-local handles never replace canonical identities in cross-runtime communication, durable state, recovery, public protocol or evidence that outlives the allocation context.

## 5. Authoritative execution boundary

Every mutation-capable event becomes one **normalized authoritative input** addressed to exactly one current scope owner before it can affect gameplay state.

Input classes include, as applicable:

- FND-02 reserved player commands;
- due gameplay timers;
- control/fencing/lifecycle transitions;
- accepted handoff transitions;
- typed WorldServices/dependency completions;
- auxiliary compute results;
- recovery/reconciliation inputs;
- bounded administrative/system commands under their own authorization contract.

Raw socket callbacks, OS timers, database callbacks and worker-thread completions never mutate authoritative gameplay directly.

### 5.1 `RuntimeExecutionOrdinal`

FND-03 freezes one owner-local semantic ordering value:

```text
RuntimeExecutionOrdinal
```

It is scoped to:

```text
(semantic runtime scope, scope ownership generation)
```

Semantics:

- starts at a defined non-zero first value for a new ownership generation;
- increases monotonically by one for each normalized authoritative input accepted for non-interleaved owner resolution;
- never reuses a value within that ownership generation;
- zero/no-op/rejected inputs that reached authoritative resolution still consume their accepted ordinal;
- is an execution/evidence value, not a credential or entity identity;
- does not replace FND-02 `CommandId`, `server_sequence`, state-domain revisions, EventId, OperationId or TransactionId;
- wrap/reuse is forbidden; representational exhaustion is scope-terminal until safe ownership lifecycle recovery establishes a new generation.

Exact physical width/serialization is not a public FND-03 contract requirement. Any implementation must choose a representation whose hard exhaustion behavior is explicit and testable. If later retained in a durable/public event/replay format, the owning ANL/DUR/test contract freezes its encoding.

### 5.2 Resolution is non-interleaved

One normalized authoritative input is resolved without interleaving another mutation-capable input into the middle of its owner-local state transition.

The input may:

- commit no domain mutation;
- commit one domain mutation;
- commit multiple deterministic internal domain mutations/effects as one non-interleaved resolution;
- transition an operation into explicit asynchronous `PENDING` state and return without waiting.

Resulting domain mutation order is represented by existing domain/state revisions and deterministic internal sub-order. FND-03 therefore does **not** require a second runtime-wide `RuntimeCommitOrdinal` counter.

The analysis-only terms `RuntimeInputOrdinal` and `RuntimeCommitOrdinal` are superseded by:

```text
one RuntimeExecutionOrdinal for owner input linearization
+ domain/state revisions for committed state
```

If an asynchronous operation later completes, that completion is a **new normalized authoritative input** with a new RuntimeExecutionOrdinal, correlated to the original `CommandId`, operation/work identity and required generation/revision evidence.

### 5.3 Fencing race

A valid fencing/control input does not physically interrupt a currently executing owner instruction sequence. However:

- once the current owner observes a valid newer fence, no further ordinary authoritative resolution may commit under the old generation;
- external/durable consumers must independently reject stale-generation writes/claims so a partitioned old process cannot win by not observing the local fence promptly;
- after fencing, old-generation outputs, worker results, timers and pending completions cannot regain authority.

This preserves the one-owner invariant across process races without making OS preemption semantics part of gameplay.

## 6. FND-02 command integration

FND-03 consumes FND-02 unchanged:

```text
CommandRef = (GameSessionId, CommandId)
```

Rules:

- per-GameSession `CommandId` order remains authoritative;
- no second client-command sequence is introduced;
- a command reserved by FND-02 ingress is never executed twice;
- later reserved commands from the same GameSession cannot commit authoritative effects ahead of an earlier reserved CommandId when FND-02 requires ordered commit;
- if an earlier command awaits asynchronous work, later same-session commands may be received/prepared within bounds but cannot violate the ordered-commit guarantee;
- unrelated sessions/scopes may continue making progress while one session has pending work;
- eligible reconnect preserving the same GameSession may continue only when the command high-water/pending/result/reconciliation state required by FND-02 remains safe and reconstructable.

A command is routed to the current authoritative owner of the character/spatial scope. After a committed ownership handoff/fence, the old source owner cannot accept it as authoritative merely because a stale transport or queue still contains it.

## 7. Cross-session and cross-source ordering

FND-02 does not define a global packet-arrival order among clients, and FND-03 does not create one from OS thread/socket wake-up timing.

Binding rules:

- each session preserves its own CommandId order;
- normalized ready inputs from different sources are admitted through bounded arbitration;
- one continuously busy session or timer/work source cannot monopolize the owner indefinitely;
- the current owner assigns the resulting RuntimeExecutionOrdinal;
- when the accepted cross-source order can affect gameplay, deterministic test/replay evidence must retain enough information to reproduce the chosen order;
- later gameplay contracts may define domain-specific simultaneous/conflict rules, but they may not bypass the owner execution boundary.

The exact scheduler data structure, weight and quantum remain implementation/PERF choices.

### 7.1 Control/fencing path

Valid ownership/fencing/shutdown/lifecycle control work requires separately bounded capacity or an equivalent reserved path that ordinary gameplay traffic cannot fully consume.

This is a safety/control lane only. It is not a general high-priority gameplay queue and cannot be used to grant selected players/actions arbitrary priority.

## 8. Clock and time model

FND-03 defines three distinct domains.

### 8.1 Wall clock

Wall clock is for human/operator/audit/calendar and cross-system correlation.

It is not the sole authority for in-process gameplay durations or runtime deadlines. NTP/system-clock changes must not make a cooldown/protection/runtime timeout repeat, move backward or fire early.

### 8.2 Process-local monotonic time

A monotonic clock is authoritative for in-process elapsed durations/deadlines such as:

- liveness/protection timing delegated to runtime;
- stale-transport cleanup timing;
- re-entry protection duration;
- worker/service timeout budgets;
- owner scheduling/deadline measurements;
- local cooldown/expiry timers whose owning gameplay contract keeps them process-local.

A process-local monotonic instant is valid only inside one process incarnation. It must not be serialized as a portable timestamp or compared directly after a new `NodeId` starts.

### 8.3 Authoritative execution order

`RuntimeExecutionOrdinal` plus domain/state revisions and FND-02 sequence/command evidence define logical authoritative order. Wall-clock timestamps do not replace those values.

### 8.4 No universal fixed tick

FND-03 selects no universal fixed-rate game loop.

Authoritative scopes execute bounded ordered work cycles with monotonic-deadline timers. Individual gameplay systems may later define cadences when their own semantics require them. A two-second gameplay rule is not a two-second server-loop requirement.

## 9. Durable/recoverable timer boundary

An opaque process-local monotonic instant can never be the durable representation of a timer that must survive process failure.

For every timer whose semantics cross a GameNode process lifetime, the owning gameplay/DUR contract must define domain semantic state sufficient to reconstruct it, including as applicable:

- what expires/occurs;
- whether offline/process-down time counts;
- trusted durable time basis or remaining-duration policy;
- idempotent occurrence identity where repetition matters;
- state revision/generation to which it belongs;
- restart/catch-up behavior.

Recovery converts that semantic durable state into a fresh process-local monotonic schedule under the new NodeId. FND-03 does not select the durable time encoding or PostgreSQL schema.

## 10. Timer execution contract

Mutation-capable timers are owner-scoped inputs, not direct callbacks.

### 10.1 Scheduling key

A timer is bound at minimum to:

- semantic runtime scope;
- current ownership generation;
- target entity/local generation when applicable;
- monotonic due deadline for process-local execution;
- deterministic equal-deadline order.

Equal-deadline order is derived from the owner resolution that scheduled the timer plus a deterministic within-resolution sequence; a separate globally visible timer counter is not required.

### 10.2 Firing

When due, the timer becomes a normalized authoritative input and receives a new RuntimeExecutionOrdinal when the current owner accepts it for resolution.

OS timer callback/wake-up order cannot mutate state directly.

### 10.3 Cancellation/staleness

A timer cannot mutate when:

- scope ownership generation changed;
- target entity/local generation no longer matches;
- owning state revision/condition invalidated it;
- it was explicitly cancelled/expired under its contract.

### 10.4 Zero-delay recursion

A timer created during one owner resolution cannot recursively execute unlimited mutation within that same resolution. Zero/immediate deadlines become subsequent bounded owner inputs unless a specific domain contract proves a bounded equivalent operation.

### 10.5 Catch-up taxonomy

Every periodic/repeating timer family declares one explicit bounded policy:

- `DEADLINE_STATE` — evaluate current deadline/state once; no synthetic missed ticks;
- `RUN_EACH_BOUNDED` — each occurrence matters, but only a registered bounded amount executes per owner cycle;
- `COALESCE_ELAPSED` — missed periods collapse into one deterministic elapsed-time calculation only when semantically equivalent;
- `SKIP_TO_LATEST` — intermediate work may be skipped only for explicitly non-semantic maintenance/AI-think-like work;
- `EXPIRE_OR_CANCEL` — work is invalid after deadline/state-generation change.

There is no implicit "run every missed tick without bound" behavior.

## 11. Deterministic test clocks

Runtime core timing must be injectable behind an explicit clock abstraction suitable for deterministic/virtual tests.

Tests must reproduce:

- immediately before / exactly at / immediately after a deadline;
- wall-clock jumps while monotonic time remains valid;
- multiple equal-deadline timers;
- liveness recovery racing protection activation;
- drain/fence/recovery boundaries;
- bounded catch-up after artificial scheduler delay.
Production builds do not expose test time-travel authority to gameplay clients or untrusted inputs.

## 12. Auxiliary parallel work

Auxiliary workers may perform expensive non-authoritative work such as pathfinding, AI planning, visibility candidate calculation, serialization/compression preparation or bounded data preparation.

### 12.1 Request context

Mutation-relevant work is bound to enough immutable context to validate its result, including as applicable:

- semantic scope identity;
- scope ownership generation;
- source state/domain revision;
- target entity/local generation;
- work/operation identity;
- deadline/cancellation state;
- ruleset/content revision.

Workers do not receive unrestricted mutable authoritative state.

### 12.2 Result revalidation

A worker/service result is only a proposal until the current owner revalidates:

- scope identity;
- current ownership generation;
- relevant state/entity revision;
- operation still pending/valid;
- deadline/cancellation status;
- current domain legality;
- resource bounds.

Stale/late/misrouted results are discarded as non-authoritative and counted diagnostically. No rollback is required because the worker never had mutation authority.

### 12.3 Completion timing

Worker completion order alone may not determine a gameplay outcome that cannot later be reproduced.

Where completion timing materially affects a result, the owning subsystem must define a deterministic conflict/deadline/fallback rule or retain the accepted RuntimeExecutionOrdinal/order evidence needed for replay.

### 12.4 No blocking remote work in the writer

The authoritative owner must not synchronously wait on network, database, remote service or expensive CPU work while holding the logical mutation lane.

An operation requiring external work enters explicit bounded `PENDING` state, issues asynchronous work and yields. The later completion/failure/timeout returns as a new normalized input and is revalidated before continuation.

## 13. WorldServices/dependency outcomes

A WorldServices/database/external result cannot mutate state from its callback thread.

The runtime emits a typed request from a valid owner resolution, retains bounded pending-operation state, and later consumes one normalized response/timeout/cancellation.

For durable operations, DUR/ANL contracts define the transaction/atomicity point. FND-03 does not decide whether local memory or PostgreSQL is the durable source of truth for that operation; it requires that asynchronous completion cannot bypass current ownership/session/state fencing.

Dependency loss is explicit. It is never hidden behind an unbounded retry loop.

## 14. Queue and executor contract

Every queue, pending set and executor influenced by clients/load is bounded. Hidden unbounded task spawning, callback accumulation and retry lists are prohibited.

Minimum runtime classes requiring explicit bounds include:

1. FND-02 session command ingress beyond already fixed protocol limits;
2. owner control/fencing/lifecycle input capacity;
3. ready session/source arbitration metadata;
4. timer population per scope and due/catch-up work;
5. pending asynchronous operations per scope/session;
6. auxiliary CPU queued/in-flight work;
7. service/database request/completion queues;
8. outbound authoritative/control state per session in entries and bytes;
9. hosted authoritative scopes per GameNode where runtime memory scales with scope count;
10. best-effort gameplay telemetry queues;
11. deterministic replay/test artifact inputs when attacker-controlled or amplification-prone.

### 14.1 Concrete numeric limits gate implementation, not this architecture decision

This contract intentionally does not guess benchmark-sensitive numbers.

Before a runtime implementation may be accepted as implementing FND-03, every externally influenced or amplification-prone runtime boundary above must have a concrete hard maximum in `RESOURCE_LIMITS_REGISTRY.json` or an explicitly superseding machine-readable registry, with:

- unit (`entries`, `bytes`, `in_flight`, work units or combination);
- configurable default where applicable;
- absolute hard maximum;
- boundary/negative tests;
- failure category;
- measurement/safety rationale;
- observability of saturation/drop/rejection.

Values may come from protocol-fixed constraints, explicit safety analysis, representative benchmark/stress evidence or a bounded implementation spike. "Unlimited" is not an accepted value.

## 15. Backpressure and overload semantics

### 15.1 Before FND-02 command reservation

If required safe runtime capacity is unavailable **before** a new command is reserved, reject/backpressure according to FND-02 capacity semantics. The command identity is not advanced merely to discard work.

### 15.2 After command reservation

Once FND-02 reserves a command identity:

- it cannot be silently dropped;
- a retry cannot become a second execution;
- bounded pending identity/result state must preserve the no-double-execution invariant;
- admission of additional unsafe work stops before bounded state is exceeded;
- the command eventually receives one terminal result or the logical session transitions through a separately governed terminal/recovery path that preserves command identity/order safety.

### 15.3 Control reserve

Gameplay saturation cannot consume all ability to process current ownership/fencing/shutdown signals.

### 15.4 Timer capacity

If committing a new operation requires registering a required authoritative timer and no safe bounded timer capacity exists, the operation fails before committing the state that depends on that timer.

Already accepted authoritative timers are not silently discarded because the due queue is congested.

### 15.5 Scope fairness

One busy ChannelRuntime/InstanceRuntime may not indefinitely starve unrelated scopes on the same GameNode.

NodeRuntime scheduling provides bounded work opportunity/yield across runnable scopes. Exact scheduling algorithm, weights and quantum are implementation/PERF choices.

### 15.6 Slow client

Outbound session state is bounded by entries and bytes.

A slow client cannot force unbounded delta retention or block a whole authoritative scope. When bounded outbound history is no longer sufficient:

- superseded deltas may be collapsed only where FND-02/state semantics permit;
- the session moves to explicit resynchronization/replacement-snapshot behavior;
- if the transport cannot recover within accepted liveness/capacity policy, the concrete transport may close;
- FND-04 owns logical GameSession continuity/reconnect eligibility.

The server never treats client slowness as authority to discard authoritative state.

### 15.7 Telemetry and audit

Best-effort gameplay telemetry may drop only under the explicit ADR-0006/ANL policy, with counted bounded loss.

Required durable economy/security audit never silently downgrades to best effort; owning DUR/ANL transaction semantics decide whether the risky operation must fail/hold.

## 16. Runtime lifecycle

FND-03 preserves ADR-0009 lifecycle vocabulary.

### 16.1 `Requested`

Assignment/request exists; no local gameplay authority is active or routable.

### 16.2 `Starting`

Process-local scope resources are being created under bounds; no gameplay routing.

### 16.3 `Warming`

Required immutable revisions and recoverable state are loaded/validated. Precomputation may occur but cannot publish authority by itself.

### 16.4 `Recovering`

A current ownership generation is established for recovery, stale old-generation work is invalid, and checkpoint/replay/session state is being validated. Clients are not routed until recovery readiness is proven.

### 16.5 `Ready`

Internal scope invariants, ownership generation and declared required dependencies/revisions pass. `Ready` is not itself Gateway/Registry routing authority.

### 16.6 `Open`

The scope may accept ordinary authorized gameplay/admission work subject to FND-04 and capacity policy.

### 16.7 `Full`

Existing authoritative gameplay continues; new admission/placement is rejected/redirected by policy. Fullness does not weaken existing session correctness.

### 16.8 `Degraded`

Named capabilities/dependencies are unavailable or constrained. Degradation cannot silently weaken fencing, authentication, item/currency safety or required audit. Risky operations whose required dependency is unavailable fail closed; unaffected local simulation may continue only when its owning contract permits it.

### 16.9 `Draining`

No new ordinary admissions/transfers enter the scope. Existing work progresses toward one bounded documented safe boundary; control/fencing remains serviceable.

### 16.10 `Checkpointing`

The scope is crossing an explicit checkpoint lifecycle barrier. FND-03 defines the authoritative cut; DUR-02 defines persistence encoding/atomic durability/RPO/RTO.

### 16.11 `Suspected`

Health/ownership certainty is degraded. `Suspected` grants no new authority. If current ownership/lease/fence proof required by an operation can no longer be established, risky/durable mutation fails closed under the owning contract.

### 16.12 `Fenced`

The old generation may commit no new authoritative gameplay mutation/result/readiness/durable write. Only non-authoritative cleanup/evidence work may continue.

### 16.13 `Failed`

The scope is not authoritative/routable and requires validated recovery/replacement.

### 16.14 `Stopped`

No authoritative mutation; process-local scope resources may be released. Semantic ChannelId/InstanceId history is independent of this runtime object lifetime.

## 17. Activation and ownership grant

A scope becomes authoritative only after:

1. semantic scope identity is validated;
2. required protocol/ruleset/content/World Bundle/build compatibility is accepted;
3. a current ownership generation/grant is established by the accepted ownership authority;
4. any previous ownership is durably/operationally fenced according to the owning contract;
5. required recovery/checkpoint state is validated;
6. required bounded runtime resources are available;
7. the lifecycle reaches `Ready` and then `Open` under control-plane policy.

A process cannot become authoritative merely by reconstructing state locally or discovering an old checkpoint.

## 18. Drain and checkpoint cut

### 18.1 Drain

When drain starts:

- new ordinary admission/transfer into the scope stops;
- new work that cannot safely complete before the declared drain boundary is not started;
- reserved commands are completed, rejected or preserved according to their FND-02/FND-04 semantics rather than silently discarded;
- risky durable/handoff operations reach a defined commit/abort/pending-recovery boundary;
- timers/pending work reach a documented checkpointable state;
- drain has a bounded operational deadline, but FND-03 does not choose its numeric value.

### 18.2 Checkpoint cut

A checkpoint request is a normalized authoritative input.

The owner captures one immutable authoritative cut identified by at least:

- semantic scope identity;
- current ownership generation;
- current RuntimeExecutionOrdinal boundary;
- required domain/state revisions;
- required pending command/session/timer/handoff metadata as defined by FND-04/DUR/gameplay contracts.

The cut means:

```text
all completed authoritative resolutions before the cut are represented
no later authoritative resolution is represented in that checkpoint version
```

Checkpoint serialization/storage may run asynchronously from an immutable captured representation. It cannot claim state beyond the captured cut and cannot commit under a stale ownership generation.

FND-03 does not select checkpoint format, journal technology, RPO/RTO or database schema.

## 19. Fencing

Once the local owner observes a valid newer ownership fence or its current authority is explicitly revoked:

- no new RuntimeExecutionOrdinal is assigned for ordinary gameplay under the old generation;
- no new authoritative mutation/result/readiness claim is published;
- stale generation persistence/external writes are rejected by downstream fences;
- pending worker/service/timer results from the old generation cannot regain authority;
- only non-authoritative cleanup/evidence may continue.

A stale process returning after partition/restart cannot self-reactivate. It requires a new valid ownership grant, which necessarily uses a newer generation and may run under a new NodeId.

## 20. Recovery

Recovery of the same channel or recoverable concrete instance may preserve semantic ChannelId/InstanceId while:

- process placement changes;
- replacement process receives a fresh NodeId;
- scope ownership generation strictly advances.

Recovery must validate compatible protocol/ruleset/content/World Bundle/build/persistence revisions before `Ready`/`Open`.

No silent channel hopping is allowed because a failed ChannelId and another ChannelId are different simulations.

### 20.1 Same-GameSession recovery

The runtime may claim same-GameSession resume only if FND-04/DUR can preserve or reconstruct every FND-02 session property required for safe continuity, including command high-water/pending/result state and reconciliation boundaries.

If that cannot be proven, the old logical GameSession terminates safely and recovery uses the fresh-session path. Runtime convenience cannot relax FND-02 no-double-execution/order guarantees.

## 21. Channel↔Instance handoff execution

FND-03 owns the runtime execution boundary of a previously authorized handoff; FND-04 owns session/admission authorization and DUR owns durable item/state safety.

Binding runtime properties:

- source and destination may prepare concurrently but at most one is authoritative for the transferred character's local simulation at a time;
- final ownership commit is one explicit ordered barrier correlated with canonical `HandoffId` and current generations;
- after commit, source local mutation authority is gone;
- retry/resume of the same logical handoff reuses the same HandoffId;
- stale handoff completion is rejected by generation/revision checks;
- failure before commit preserves/recovers previous safe owner;
- failure after commit recovers from destination authority evidence, not client claims;
- FND-02 snapshot/resync semantics establish client state before ordinary destination deltas continue.

Exact handoff credential/lease transaction/message encoding remains FND-04/DUR/FND-02-owned.

## 22. Liveness and reconnect interface with FND-04

FND-03 and FND-04 have a strict split.

### 22.1 FND-04 owns

FND-04 owns:

- which normalized current-generation transport/session evidence counts as **sufficient control/liveness evidence**;
- logical session states and terminality;
- reconnect credentials/proof and replay prevention;
- connection-generation transition authorization;
- character/account lease/takeover semantics;
- exact authoritative start/end semantics of the accepted 15-second logical reconnect grace;
- reconnect/re-entry eligibility, including graceful logout versus unexpected loss.

### 22.2 FND-03 owns

After FND-04/session logic advances accepted sufficient-control evidence for the current authoritative binding, FND-03:

- records the local observation/progress against the process monotonic clock;
- measures elapsed runtime durations;
- executes the accepted `2.0 s` PvE disconnect-protection boundary;
- executes the accepted five-second stale concrete transport cleanup timing without destroying required authoritative actor presence;
- executes the four-second PvE defensive re-entry effect after FND-04 authorizes valid unexpected-loss re-entry;
- exposes runtime scheduling/queue health needed to distinguish local GameNode stall from isolated player/path liveness loss.

Client/OS/Launcher/Guardian diagnostics are not an input that can directly advance this real-time authoritative liveness tracker.

### 22.3 Accepted timing composition

```text
last FND-04-accepted sufficient current-generation control evidence
    -> FND-03 monotonic elapsed timer
    -> elapsed < 2.0 s: ordinary PvE behavior
    -> elapsed >= 2.0 s: disconnect protection active
    -> 5 s stale concrete transport cleanup boundary

FND-04 logical session state
    -> accepted 15 s reconnect-grace semantics
    -> valid unexpected-loss re-entry decision
    -> FND-03 activates 4 s defensive PvE re-entry protection
```

The 15-second value is not redefined by FND-03 and is not inferred from a wall-clock timestamp.

## 23. Disconnect/re-entry execution

The four-second re-entry protection is prospective owner-scoped state on the same authoritative actor.

FND-03 preserves the accepted rules:

- movement/escape allowed;
- self-healing allowed under normal legality/cost/cooldown;
- health/mana/resource potions allowed under normal rules;
- protected character cannot heal another player;
- another player may legally heal the protected character under ordinary rules;
- no outgoing offensive PvE action executes;
- for the full four-second protection interval, no PvE monster may issue a new offensive attack against the protected character, including a monster that already targeted the character before re-entry;
- prohibited outgoing actions are never buffered for post-protection burst;
- effects already committed before protection activation may still resolve; this does not authorize any new monster attack during the protection interval;
- HP/resources/position/conditions/cooldowns/combat/PZ/logout/threat/encounter state are not automatically reset.

Runtime-health evidence is retained so local overload/stall is not mislabeled as player-side disconnect behavior.

## 24. Panic and invariant-failure containment

Unexpected failure inside authoritative execution is fail-stop, not "catch and continue" by default.

If an unexpected panic/internal invariant violation may have affected one scope's authoritative state:

1. stop ordinary authoritative commits for that scope;
2. move it toward `Fenced`/`Failed`;
3. stop new routing/admission;
4. invalidate/cancel old-generation auxiliary/pending work;
5. retain bounded diagnostic evidence;
6. require validated recovery under current ownership generation before authority resumes.

If the runtime cannot prove corruption is isolated to one scope—for example a shared unsafe/memory component or shared mutable authority may be affected—the safer contract is fail-stop the entire GameNode process and let external orchestration replace it.

Concrete Rust panic strategy/catch mechanism is implementation detail. FND-03 forbids blindly continuing ordinary mutation after an unexpected authoritative invariant failure.

## 25. Deterministic authoritative randomness

Gameplay randomness that can affect authoritative state uses explicit deterministic RNG state/streams owned by a named authoritative scope/domain.

Requirements:

- seed/source is server-controlled, never player authority;
- thread/worker placement does not change the sequence of authoritative random decisions;
- worker tasks needing randomness receive explicit deterministic inputs or return proposals whose final authoritative random choice occurs at the owner;
- replay/test evidence can reproduce the authoritative random sequence under the named ruleset/build;
- RNG algorithm/stream representation requires stable documented behavior and cross-platform fixtures before implementation acceptance.

Security randomness for tokens, nonces, keys or credentials is **separate** and cryptographically secure under FND-04/security contracts. Deterministic gameplay RNG never creates security secrets.

## 26. Deterministic simulation replay

FND-03 requires deterministic explanation/replay of accepted authoritative order, not identical live CPU/thread interleaving.

Replay evidence must be able to reconstruct concurrency-sensitive results using applicable data such as:

- semantic runtime scope;
- scope ownership generation;
- NodeId/process incarnation evidence;
- RuntimeExecutionOrdinal;
- `GameSessionId + CommandId`;
- normalized command/timer/worker/service/control input;
- timer deadlines/equal-deadline ordering evidence;
- HandoffId/generation;
- relevant state-domain revisions;
- deterministic gameplay RNG seed/stream evidence;
- protocol/ruleset/content/World Bundle/build revisions;
- injected/recorded deterministic clock progression in tests.

Replay tooling must not require original thread IDs, CPU count, worker wake-up order or wall-clock scheduling jitter.

Exact replay file/storage format and production retention remain later test/ANL/operations decisions.

## 27. Event emission versus replay

FND-03 distinguishes:

1. **authoritative state transition** — performed only by current owner;
2. **event/audit materialization** — produced according to ANL/DUR durability class;
3. **analytics/event replay** — re-delivery to read-only/idempotent consumers.

Best-effort analytical events may be emitted after authoritative transition according to bounded policy.

For required durable economy/security evidence, ANL/DUR define the atomic mutation+outbox/audit boundary; FND-03 does not permit a required audit event to be silently best-effort.

Analytics/event replay **never replays the authoritative gameplay mutation**.

## 28. Foundation error mapping

FND-03 consumes `FOUNDATION_ERROR_VOCABULARY.md`.

| Runtime condition | Foundation category | FND-03 semantic outcome |
|---|---|---|
| malformed/out-of-range runtime-local request | `INVALID_INPUT` | reject before unsafe mutation |
| incompatible protocol/ruleset/content/World Bundle/build activation | `UNSUPPORTED_REVISION` | fail closed; no mixed authoritative scope |
| stale scope/session/entity/work generation | `STALE_GENERATION` | no stale mutation/result/durable write |
| current owner/state blocks transition | `CONFLICT` | preserve current authority/state |
| registered queue/timer/work/resource limit reached | `CAPACITY_EXCEEDED` | bounded rejection/backpressure before unsafe acceptance |
| required world/database/service dependency unavailable | `DEPENDENCY_UNAVAILABLE` | explicit pending/degraded/fail-closed outcome under owning operation |
| named runtime/lifecycle deadline expires | `TIMEOUT` | deterministic bounded timeout state |
| explicit cancellation before commit | `CANCELLED` | deterministic cleanup; no hidden partial success |
| unexpected condition where safe containment is required | `INTERNAL_UNAVAILABLE` | fail closed; details remain internal |

`AUTHENTICATION_FAILED` and `SESSION_REJECTED` remain primarily FND-04/security categories; FND-03 consumes their already-authorized outcomes rather than authenticating users.

Narrower public/internal codes may be added by owning contracts but must map to this vocabulary. Raw Rust errors/panic/log strings are never stable client API behavior.

## 29. Foundation failure-scenario disposition

The catalogue requires each foundation contract to classify all applicable scenarios. The statuses below are **contract-level architecture dispositions**, not executable runtime proof.

| Scenario | FND-03 status | FND-03 requirement / later owner |
|---|---|---|
| `FS-PLATFORM-UNAVAILABLE` | `DEFERRED_BY_ACCEPTED_GATE` | FND-04/Platform own login/admission; FND-03 creates no alternate credential authority. |
| `FS-GATEWAY-AFTER-REDEEM` | `DEFERRED_BY_ACCEPTED_GATE` | FND-04/Platform reconciliation owns ticket/session outcome. |
| `FS-POSTGRES-UNAVAILABLE` | `PASS` | no blocking DB call in owner; dependency loss explicit; no runtime claim of unfenced durable success; DUR-02 finalizes transaction/recovery. |
| `FS-LEASE-RENEW-TIMEOUT` | `DEFERRED_BY_ACCEPTED_GATE` | FND-04 owns lease semantics; FND-03 stops stale durable/authoritative work once a valid fence/loss-of-authority result is established. |
| `FS-DUPLICATE-LOGIN` | `DEFERRED_BY_ACCEPTED_GATE` | FND-04 owns race/admission; runtime never creates a second local mutation owner from a login attempt. |
| `FS-STALE-GENERATION` | `PASS` | stale scope/session/entity/worker generation rejected before mutation. |
| `FS-DUPLICATE-COMMAND` | `PASS` | consumes FND-02 high-water/ordered semantics; no second execution. |
| `FS-CHANNEL-SPLIT-OWNER` | `PASS` | current scope generation + external durable fencing prevent dual accepted commits. |
| `FS-CHANNEL-DRAIN` | `PASS` | no new admission; bounded progress to named safe checkpoint/abort boundary; no silent reserved-work loss. |
| `FS-QUEUE-SATURATION` | `PASS` | bounded queues, explicit `CAPACITY_EXCEEDED`/backpressure/degradation, no unbounded growth. |
| `FS-SLOW-CLIENT` | `PASS` | bounded outbound state; resync/close direction; unrelated authoritative simulation not blocked. |
| `FS-CLOCK-SKEW` | `PASS` | process monotonic deadlines unaffected by wall-clock movement; signed timestamp skew remains FND-04/security. |
| `FS-KEY-ROTATION` | `DEFERRED_BY_ACCEPTED_GATE` | FND-04/security. |
| `FS-REVISION-MISMATCH` | `PASS` | incompatible scope cannot activate as authoritative; no implicit downgrade. |
| `FS-SNAPSHOT-DELTA-MISMATCH` | `PASS` | runtime preserves FND-02 explicit resync/replacement snapshot; no guessed partial application. |
| `FS-DB-OUTBOX-BOUNDARY` | `DEFERRED_BY_ACCEPTED_GATE` | DUR-02/ANL-01 own atomic durable mutation/outbox cut. |
| `FS-WORLD-BUNDLE-CORRUPT` | `DEFERRED_BY_ACCEPTED_GATE` | DUR-04 owns parser/checksum/decompression safety; FND-03 never activates a bundle rejected by that contract. |
| `FS-CLIENT-CUTOVER-ROLLBACK` | `NOT_APPLICABLE` | completed client-migration lifecycle, not runtime execution. |
| `FS-ANALYTICS-TELEMETRY-OVERFLOW` | `PASS` | best-effort telemetry queue is bounded/drop-counted and cannot alter gameplay authority. |
| `FS-AUDIT-OUTBOX-BACKLOG` | `DEFERRED_BY_ACCEPTED_GATE` | ANL-01/DUR-02 own durable backlog/transaction semantics; runtime cannot silently downgrade it. |
| `FS-EVENT-DUPLICATE-DELIVERY` | `DEFERRED_BY_ACCEPTED_GATE` | ANL-01 consumer idempotency; event redelivery never replays gameplay mutation. |
| `FS-EVENT-OUT-OF-ORDER` | `DEFERRED_BY_ACCEPTED_GATE` | ANL-01 consumer ordering/reconciliation; authoritative runtime state is unchanged by analytics arrival order. |
| `FS-AUDIT-MUTATION-MISMATCH` | `DEFERRED_BY_ACCEPTED_GATE` | ANL-01/DUR-02/DUR-03 own atomic mismatch prevention/recovery. |
| `FS-ANALYTICS-PRIVACY-POLICY` | `DEFERRED_BY_ACCEPTED_GATE` | ANL-01/privacy contracts. |
| `FS-DETECTOR-FALSE-POSITIVE` | `NOT_APPLICABLE` | ADR-0006/ANL security-review boundary; runtime never sanctions from analytics. |
| `FS-INVESTIGATION-MUTATION-ATTEMPT` | `NOT_APPLICABLE` | ADR-0006/ANL-04 least-privilege boundary, not runtime scheduler behavior. |

`PASS` here means the contract contains the required invariant. It does not mean implementation/E2E evidence exists.

## 30. Implementation acceptance evidence required later

A later FND-03 implementation package cannot claim `PROVEN` until exact revisions demonstrate at minimum:

1. multiple ChannelRuntimes/InstanceRuntimes progress concurrently without dual mutation authority;
2. one scope remains one logical writer under multithreaded auxiliary work;
3. Channel↔Instance committed handoff removes source mutation authority;
4. stale ownership generations/local handles/timers/worker results cannot mutate;
5. FND-02 per-session CommandId order/no-double-execution survives queue pressure and eligible reconnect;
6. one RuntimeExecutionOrdinal sequence provides reproducible owner input order without overloading CommandId/server_sequence/state revisions;
7. queue saturation stays within registered bounds and produces named backpressure/rejection;
8. slow clients cannot grow outbound memory unboundedly or stall unrelated scope progress;
9. wall-clock jumps do not change process-monotonic gameplay deadlines;
10. equal-deadline timers and catch-up policies are deterministic and bounded;
11. delayed dependency/worker completion is revalidated after generation/revision changes;
12. authoritative owner does not block on remote/database/expensive CPU work;
13. scope fairness/noisy-neighbor tests prove bounded scheduling opportunity;
14. drain/checkpoint/fence produces an observable ordered cut and old generation stops authority;
15. replacement process gets fresh NodeId while eligible recovered ChannelId/InstanceId stays stable under newer ownership generation;
16. split-owner fault injection proves only current generation can commit externally/durably;
17. deterministic replay reproduces concurrency-sensitive outcomes from normalized input/order/clock/RNG evidence;
18. authoritative panic/invariant failure does not continue ordinary mutation blindly;
19. concrete runtime hard limits are registered with boundary tests and evidence in `RESOURCE_LIMITS_REGISTRY.json`;
20. Tier 1 E2E/fault scenarios cover at least stale generation, queue saturation, slow client, clock skew, drain/recovery and split-owner behavior; Tier 2/3 follow ADR-0007 where user-visible client behavior is part of the claim.
21. re-entry tests prove the full 4.0-second inbound PvE suppression against all monsters, including an already-targeting monster, while effects committed before protection activation may still resolve and ordinary monster attacks resume only after protection expires.

## 31. Downstream ownership after FND-03

FND-03 freezes runtime execution semantics only.

### FND-04

Owns:

- Identity/Game Session/admission state machine;
- session/reconnect credentials and replay prevention;
- connection-generation authorization;
- character/account lease/takeover;
- exact liveness evidence acceptance/hysteresis;
- 15-second reconnect grace start/end semantics;
- duplicate login and takeover behavior;
- session-level handoff authorization.

### DUR-01 / DUR-02 / DUR-03

Own:

- physical durable identifier representation;
- PostgreSQL schema/migrations/isolation/locking/retries;
- durable checkpoint/journal/RPO/RTO;
- lease/session persistence where required;
- item/currency transaction and conservation/anti-duplication invariants.

### ANL-01 and later analytics gates

Own:

- event/audit IDs and schemas;
- durability classes/outbox/publication;
- privacy/retention/access;
- analytics consumer replay/deduplication;
- read-only investigation/detector policy.

### PERF-01

Owns:

- reference hardware/workloads;
- worker/thread/scheduling measurements;
- latency/resource objectives;
- measured capacity/headroom and regression thresholds.

### OPS-CHANNEL-01

Owns:

- production process/container/orchestrator topology;
- scope assignment/grant distribution;
- physical ownership-generation/fencing mechanism;
- dynamic placement/scaling;
- production drain/restart/recovery objectives and blast-radius policy.

### Gameplay/content/client contracts

Own:

- movement/combat/AI/interaction-specific ordering and timer semantics;
- exact periodic-effect/catch-up rules;
- durable timer meaning;
- client prediction/presentation;
- optional client diagnostics.

## 32. Consequences

### Positive

- multithreading does not create uncontrolled concurrent mutation;
- process/thread placement remains independent from semantic ownership;
- one owner-local execution ordinal gives a clear replay/debug linearization point without redundant global counters;
- FND-02 command ordering remains intact;
- wall-clock drift cannot corrupt runtime deadlines;
- timer and worker staleness is generationally fenced;
- overload/slow clients cannot create unbounded memory paths;
- drain/recovery have explicit authority cuts;
- deterministic gameplay randomness/replay remains testable across thread placement;
- implementation technology and numeric capacities remain evidence-driven.

### Costs

- every mutation-capable async path must normalize back into the owner;
- pending operations require explicit identity/generation/revision metadata;
- bounded queues require explicit rejection/degradation instead of unlimited buffering;
- deterministic order/replay evidence adds engineering/testing work;
- durable operations require later DUR/ANL coordination;
- recovery requires end-to-end generation/fence propagation across runtime, session, persistence and operations boundaries.

## 33. Rejected alternatives

### Concurrent direct mutation under locks

Rejected because lock safety alone does not define one reproducible gameplay order or stale-owner recovery semantics.

### One OS thread for the entire GameNode

Rejected because independent scopes and auxiliary work must scale concurrently. The contract is one logical writer per scope, not one process-wide thread.

### Dedicated OS thread required per scope

Rejected because logical authority does not require fixed thread affinity; executor placement remains an implementation/PERF choice.

### One global fixed tick chosen now

Rejected because no evidence selects a frequency and gameplay durations are not loop-frequency contracts.

### Network arrival order as global gameplay order

Rejected because concurrent socket/thread arrival is not a reproducible semantic contract.

### Two mandatory runtime-wide input/commit counters

Rejected as unnecessary architecture coupling. One RuntimeExecutionOrdinal linearizes normalized owner inputs; domain/state revisions represent committed authoritative state.

### Unbounded async queues/task spawning

Rejected as a denial-of-service/availability risk and violation of resource-limit policy.

### Drop already reserved commands under pressure

Rejected because it breaks FND-02 identity/order/retry guarantees.

### Direct worker/service callback mutation

Rejected because stale work can complete after state/generation changes and has no mutation authority.

### Persist process-monotonic instants

Rejected because they are process-incarnation local and meaningless after replacement.

### Self-granted ownership by a newly started NodeRuntime

Rejected because process identity is not authority and split-brain fencing must be external/durable enough to reject stale processes.

### Continue after unexpected authoritative panic/invariant failure

Rejected unless safe isolation is proven; availability does not justify silently continuing potentially corrupted authority.

## 34. Contract acceptance boundary

When this FND-03 contract is accepted and merged:

- the architecture gate for runtime execution semantics is complete;
- later implementation tasks may design code **only under separate explicit implementation authority**;
- this contract alone does not authorize runtime code, production traffic or deployment;
- FND-04 remains a separate required foundation gate before production admission/lease/reconnect behavior can be claimed;
- DUR/ANL gates remain required before authoritative durable gameplay/item/currency mutation claims;
- numeric runtime hard limits remain an implementation-acceptance prerequisite, not a guessed architecture constant.

## 35. Canonical concise rule

```text
one ChannelRuntime / one InstanceRuntime semantic scope
-> one current ownership generation
-> one logical authoritative owner
-> NodeId/OS thread placement does not grant authority

mutation-capable event
-> normalize and address to current owner
-> bounded arbitration/control lane
-> assign RuntimeExecutionOrdinal
-> resolve without interleaving another authoritative input
-> validate scope + generation + state/entity revision
-> commit zero or more deterministic domain mutations
-> domain state revisions / FND-02 output / durable request as applicable

async/worker/service work
-> immutable request / explicit pending operation
-> no direct mutation
-> return as new normalized input
-> revalidate current generation/revision/legality
-> stale result discarded

wall clock
-> correlation/calendar only
process monotonic time
-> in-process elapsed/deadline authority
RuntimeExecutionOrdinal + state revisions
-> authoritative logical order
opaque monotonic instant
-> never durable across NodeId replacement

queues/executors
-> always bounded
-> control/fencing cannot starve
-> reserved authoritative work never silently disappears
-> slow client converges to bounded resync/transport handling
-> concrete numeric maxima required before implementation acceptance

fence/recovery
-> old generation cannot regain authority
-> replacement process gets fresh NodeId
-> same ChannelId/InstanceId may survive eligible recovery
-> ownership generation strictly advances
-> same GameSession resumes only if FND-02/FND-04 required state is safely preserved
```