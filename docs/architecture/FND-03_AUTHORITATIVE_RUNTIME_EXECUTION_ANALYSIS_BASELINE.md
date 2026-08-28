# FND-03 Authoritative Runtime Execution Analysis Baseline

- Status: Owner-directed architecture analysis baseline; input to the later complete `FND-03` contract
- Date: 2026-08-08
- Gate: `FND-03`
- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Applies to: `NodeRuntime`, `WorldServices`, `ChannelRuntime`, `InstanceRuntime`, authoritative ordering, clocks, timers, scheduler, runtime-local handles, auxiliary work, queues/backpressure, failure isolation and replay evidence
- Preserves: ADR-0001, ADR-0009, FND-ID-01, FND-02, instance/runtime owner baseline, reconnect/disconnect owner baselines, Game Intelligence authority separation
- Does not authorize: Rust runtime implementation, production protocol listeners, admission/lease implementation, persistence schemas, production deployment, external-repository changes, or benchmark-sensitive implementation choices

## 1. Purpose

Persist the runtime-execution conclusions accepted for continued FND-03 architecture work and define the next bounded analysis direction without pretending that the complete FND-03 contract already exists.

This baseline freezes only the execution invariants that are already required by accepted architecture or were explicitly directed to be preserved by the owner. It also records additional derived recommendations that must still survive final FND-03 review before they become the complete contract.

The central design objective is:

> Oteryn may execute many gameplay scopes concurrently on a multithreaded GameNode, while each authoritative gameplay scope commits state through exactly one logical ordered mutation boundary whose correctness does not depend on uncontrolled thread scheduling.

## 2. Evidence classification

### PROVEN

The accepted repository architecture already establishes that:

- one GameNode process may host several ChannelRuntimes;
- the game server is multithreaded;
- each channel has exactly one logical authoritative mutation owner;
- auxiliary workers may perform bounded CPU-heavy work but may not directly mutate authoritative channel state;
- stale auxiliary results require channel/scope identity, ownership generation and source-state revision validation;
- every concrete InstanceRuntime likewise has one logical authoritative mutation owner;
- `NodeId` identifies one GameNode process incarnation and is not an ownership/fencing capability;
- FND-02 gives each logical GameSession a monotonic `CommandId` stream and requires ordered authoritative command effects/results inside that stream;
- FND-02 does not define the complete world-simulation clock/scheduler model;
- reconnect/protection timing is server-authoritative and must not be measured from untrusted client timestamps;
- queue saturation, slow clients, stale generations, split ownership and telemetry overflow already have named foundation failure scenarios;
- best-effort gameplay telemetry is distinct from durable audit/economy/security evidence.

### DERIVED

The following conclusions are derived from those accepted invariants and are the recommended FND-03 direction:

- ChannelRuntime and InstanceRuntime should use one authoritative execution model and differ by owned state scope, not by scheduler architecture;
- logical single-writer ownership must not be implemented as a public contract requiring one dedicated OS thread per scope;
- per-session `CommandId`, runtime input ordering, authoritative mutation ordering, state-domain revision and protocol `server_sequence` are separate concepts and must not be overloaded into one counter;
- fixed global tick frequency is not required for correctness and should not be frozen without gameplay/performance evidence;
- runtime control/fencing events need a bounded non-starvable path so gameplay overload cannot delay authority loss or safe shutdown indefinitely;
- asynchronous worker/service responses must re-enter through the authoritative owner as proposals/results, never mutate state from completion threads.

### UNKNOWN / DELIBERATELY UNDECIDED

This baseline does not select:

- Tokio or another async runtime for the authoritative server;
- work-stealing implementation or executor library;
- number of I/O, simulation or CPU workers;
- CPU affinity/NUMA strategy;
- exact simulation tick/step/quantum frequency;
- numeric runtime queue capacities not already owned by an accepted resource contract;
- exact checkpoint interval, replay horizon, RPO or RTO;
- exact persistence executor or journal technology;
- Docker/Kubernetes/systemd/orchestrator product;
- mandatory Launcher/Guardian process or heartbeat;
- final event/audit envelope owned by `ANL-01`.

## 3. One authoritative execution model for ChannelRuntime and InstanceRuntime

### Decision direction

`ChannelRuntime` and `InstanceRuntime` use the same logical execution contract:

```text
bounded normalized inputs
        -> authoritative owner
        -> legality / ordering / state transition
        -> authoritative commit
        -> versioned results/state/events
```

They differ in semantic scope:

- ChannelRuntime owns one `WorldId + ChannelId` public-world mutable simulation scope;
- InstanceRuntime owns one `WorldId + InstanceId` isolated mutable simulation scope.

The execution contract must not create a second scheduler, timer model, worker-result model or replay model merely because an activity is instanced.

### Why this must be decided now

- **Must decide now?** YES.
- **Blocked work:** FND-03 itself, later movement/combat/AI/instance runtime implementation, deterministic tests and crash recovery.
- **Harder later:** separate open-world and instance execution engines would duplicate ordering bugs, overload semantics and recovery logic.
- **Superseding evidence:** a future independently justified execution class whose semantic requirements cannot be expressed by the shared model without measurable unacceptable cost.
- **Deliberately undecided:** physical worker placement and whether a scope is temporarily dedicated to one worker for performance.

## 4. Logical writer is not a dedicated OS thread contract

The authoritative owner is a **logical mutation boundary**, not a permanent thread identity.

Allowed implementation properties include:

- the owner task/future may execute on different worker threads over time;
- several scopes may share simulation workers;
- different scopes may execute concurrently;
- CPU-heavy proposals may execute on bounded auxiliary pools;
- network/database/service I/O remains outside the mutation critical path where possible.

Forbidden behavior includes:

- two threads mutating one scope authoritatively at the same time;
- worker callbacks mutating entity/world state directly;
- treating a mutex around arbitrary shared mutable state as equivalent to the logical-owner contract;
- relying on process-global mutable gameplay state to avoid explicit scope ownership.

### Decision timing

- **Must decide now?** YES — otherwise implementation can accidentally equate architecture with one-thread-per-channel or uncontrolled shared-state locking.
- **Superseding evidence:** none expected for the semantic invariant; only implementation placement may change.
- **Deliberately undecided:** concrete executor/thread-pool topology.

## 5. Separate ordering and time domains

FND-03 must preserve distinct concepts.

### 5.1 `CommandId`

Owned by FND-02.

```text
CommandRef = (GameSessionId, CommandId)
```

It is:

- per logical GameSession;
- monotonic `uint64`;
- command identity and total order inside that GameSession;
- continuous across an eligible reconnect of the same logical GameSession.

It is **not** the global order of one ChannelRuntime/InstanceRuntime.

### 5.2 Runtime input order

A runtime-local concept is required to record the order in which normalized external/internal inputs cross the authoritative scope boundary.

Conceptual name in this baseline:

```text
RuntimeInputOrdinal
```

Properties:

- local to one authoritative scope incarnation/ownership generation;
- assigned by the authoritative owner boundary, not trusted from a client or worker;
- monotonic and never used as a credential;
- useful for deterministic tests, replay evidence, forensic traces and first-divergence diagnosis;
- not automatically durable or public;
- not a replacement for CommandId, canonical IDs or protocol server_sequence.

The exact Rust type/name/width remains final FND-03 work.

### 5.3 Runtime commit order

The owner also needs a conceptual monotonic mutation-order marker for authoritative state transitions.

Conceptual name:

```text
RuntimeCommitOrdinal
```

This may differ from RuntimeInputOrdinal because an input may be rejected/no-op or may produce several explicitly ordered internal mutations.

It is not automatically the same as:

- protocol `server_sequence`;
- a state-domain revision;
- database transaction ID;
- analytics EventId;
- CommandId.

Final FND-03 may choose to collapse internal counters where semantics are proven equivalent, but it must not overload unrelated cross-boundary meanings merely to reduce fields.

### 5.4 Monotonic time

Used for gameplay/runtime deadlines and elapsed-duration decisions such as:

- cooldown/exhaustion deadlines;
- scheduled simulation effects;
- liveness/session runtime deadlines where owned by later contracts;
- reconnect/disconnect protection timing when the session contract delegates runtime measurement;
- worker-result deadlines;
- timeout/cancellation budgets.

Monotonic deadlines must remain safe if wall clock moves forward/backward.

### 5.5 Wall clock

Used for human/audit/calendar semantics such as:

- logs and incident correlation;
- UTC timestamps in durable evidence where required;
- scheduled calendar events and LiveOps where later contracts require them;
- operational diagnostics.

Wall-clock adjustment must not retroactively shorten/extend a monotonic gameplay deadline.

### 5.6 Simulation step/turn

The owner may use an internal logical execution turn/step as the bounded unit in which it samples monotonic time, drains ready work and commits transitions.

This does **not** require one global fixed-frequency game tick.

### Decision timing

- **Must decide now?** YES for separation of meanings; NO for exact numeric tick/step frequency.
- **Blocked work:** timer scheduler, reconnect timing integration, combat/condition timing, deterministic replay/tests.
- **Harder later:** using wall clock for gameplay or overloading CommandId/server_sequence as world-order counters would create protocol/runtime coupling and migration burden.
- **Superseding evidence:** measured gameplay/performance evidence may justify a concrete fixed-step policy later without collapsing the semantic separation.

## 6. Tick model direction

### Accepted direction

Do not freeze one global fixed tick frequency merely for architectural completeness.

The preferred foundation is a **deterministic event/turn scheduler with monotonic deadlines**. An implementation may still use fixed or semi-fixed internal quanta for efficiency, but correctness must derive from explicit deadlines/order rather than from assuming every gameplay rule equals `N ticks`.

This supports:

- low-latency command handling;
- independent rates for AI/pathfinding/maintenance work;
- precise reconnect/cooldown timing;
- deterministic test clocks;
- later performance tuning without changing public gameplay contracts.

Gameplay rules may intentionally expose discrete cadence when the owning gameplay contract requires it; that cadence is then a product rule, not an accidental consequence of one server loop frequency.

### Decision timing

- **Must decide now?** YES for the semantic direction; NO for exact frequency/quantum.
- **Superseding evidence:** benchmark and gameplay evidence proving a fixed-step architecture materially improves determinism/performance while preserving all accepted timing semantics.

## 7. Deterministic authoritative input/commit ordering

### Problem

Commands, due timers, lifecycle/fencing signals, world-service responses and auxiliary-worker completions can become ready concurrently. Allowing whichever worker/thread happens to win a race to mutate state would make correctness depend on scheduler timing and make replay/debugging unreliable.

### Required invariant

Only the authoritative scope owner determines commit order.

No producer-side completion order is itself authoritative.

### Recommended owner-turn model

At a bounded execution turn, the owner conceptually:

1. checks control/authority state before gameplay work;
2. samples one monotonic `now` for the turn;
3. makes all timers with `deadline <= now` eligible;
4. observes bounded normalized inputs already available at the authoritative boundary;
5. assigns/uses runtime-local input ordering;
6. executes ready work through one deterministic arbitration policy;
7. commits mutations serially through the logical owner;
8. advances state revisions/commit evidence as required;
9. emits results/state/event records derived from committed outcomes;
10. yields after a bounded work budget so another scope cannot be starved.

### 7.1 Control/authority lane

A bounded control lane must be logically separate from ordinary gameplay backlog and must have reserved processing capacity.

It carries only authority/lifecycle operations that cannot safely starve, such as:

- ownership/fencing loss;
- scope stop/fail/transition to recovering;
- drain barriers;
- critical cancellation/invalidation required to stop stale mutation;
- other explicitly classified runtime-control events.

It must not become a generic priority path for ordinary gameplay.

At each execution turn, authority/fence loss is observed before any further ordinary authoritative commit that the local runtime is capable of preventing.

Durable external generation fencing remains required; local control-lane priority is not a substitute for database/service fencing against a partitioned stale writer.

### 7.2 Due timers

Due timers should be deterministic within one scope.

Recommended stable ordering key:

```text
(deadline, timer_registration_ordinal)
```

where `timer_registration_ordinal` is assigned by the authoritative owner when the timer is scheduled.

A timer callback does not mutate from an external executor. The timer becomes an authoritative input processed by the owner.

Zero-delay/self-rescheduling timers must not create unbounded synchronous recursion. They are deferred to a later owner turn or otherwise limited by an explicit bounded recursion/work policy.

### 7.3 Gameplay commands

FND-02 per-session CommandId order remains binding.

Across different GameSessions, FND-03 must not invent a global CharacterId/AccountId sort that creates permanent actor bias. Normalized command arrival at the authoritative boundary is external real-world input; the runtime records the accepted runtime input order and then behaves deterministically from that order onward.

A saturated/flooding session must not starve other sessions/scopes. Exact fair-queue implementation remains benchmark-sensitive, but bounded per-session/per-scope admission and owner work budgets are required extension points.

### 7.4 Auxiliary-worker results

Worker completion is never a direct mutation event.

A result must carry enough evidence to validate at least:

```text
scope identity
ownership generation
TaskId/runtime task identity
source state revision
applicable entity/generational handles
deadline/cancellation generation where relevant
```

The owner:

- rejects stale generation;
- rejects stale source revision when the proposal is no longer valid;
- rejects cancelled/expired work;
- revalidates domain legality where required;
- accepts/transforms the proposal only through the normal authoritative commit boundary.

Thread completion timing may determine when a proposal becomes available, but replay/test evidence must capture the normalized availability/acceptance order or use a deterministic injected executor. Correctness may not rely on undocumented worker race order.

### 7.5 Service/I/O completions

Database/world-service/network callbacks likewise cannot mutate authoritative scope state directly.

A completion enters as a bounded normalized response carrying correlation identity and freshness/generation evidence appropriate to its owner contract.

### 7.6 Determinism definition

For FND-03, deterministic execution should mean:

> given the same authoritative initial state, immutable content/ruleset revisions, injected clock values, normalized input order and deterministic domain algorithms, the authoritative owner produces the same state transitions/results.

It does **not** mean that two real network sessions must produce the same cross-player input order independent of physical packet arrival timing.

This definition preserves honest replay while avoiding fake determinism that ignores real external concurrency.

### Decision timing

- **Must decide now?** YES.
- **Blocked work:** runtime implementation, worker integration, timers, deterministic testing, replay, movement/combat/AI ordering.
- **Harder later:** allowing arbitrary concurrent mutation would make race-dependent bugs and exploit reproduction structural.
- **Superseding evidence:** only evidence that an alternative model preserves equivalent deterministic commit semantics and stronger performance/latency without weakening fencing or replay.
- **Deliberately undecided:** exact queue data structures, polling algorithm, batch size and worker/executor library.

## 8. Runtime-local generational handles

FND-03 should allow compact runtime-local handles for high-frequency internal lookup while preserving canonical identities at cross-boundary points.

A runtime-local handle must:

- be scoped to the owning runtime and its lifetime/generation;
- include or imply a generation so reuse cannot alias a removed entity/task/timer;
- fail closed on stale generation;
- never become a durable/public identity by convenience;
- not be accepted from an untrusted client as authority;
- be invalidated on entity removal, slot reuse or runtime replacement according to its owning semantics.

Candidate uses include entity slots, timer handles, worker task handles and local scheduler entries.

Exact representation (`index + generation`, slab key, etc.) is implementation work and should be benchmarked rather than frozen now.

## 9. NodeId, scope identity and ownership generation

`NodeId` answers which GameNode process incarnation is executing.

Ownership generation answers which incarnation is currently authorized for one authoritative scope.

The required conceptual relation is:

```text
WorldId + ChannelId + channel_ownership_generation -> current authoritative NodeId
```

and for instances:

```text
WorldId + InstanceId + instance_ownership_generation -> current authoritative NodeId
```

A fresh NodeId after restart/replacement does not grant authority by itself.

Every externally visible/durable mutation that can outlive an in-memory process must be protected by the applicable accepted generation/session/transaction fence owned by later durability/admission contracts.

## 10. Queue classes and overload policy

FND-03 must not use one unbounded generic queue for all work.

The architecture requires separately observable bounded classes at minimum:

1. runtime control/fencing/lifecycle lane;
2. validated session/command ingress to an authoritative scope;
3. due timer/scheduler work;
4. auxiliary work submissions;
5. auxiliary work completions/results;
6. world-service/dependency requests and responses where asynchronous;
7. outbound client state/result work;
8. persistence/event preparation work where later contracts attach it;
9. best-effort gameplay telemetry.

### Policy by class

#### Control/fencing

- bounded;
- reserved/non-starvable processing;
- overload is an internal safety failure, not permission to continue stale gameplay authority;
- may force scope/GameNode fail-stop/recovery if control safety cannot be preserved.

#### Authoritative gameplay input

- bounded;
- never silently dropped after the owning contract has accepted it as authoritative work;
- saturation maps to a stable bounded failure such as `CAPACITY_EXCEEDED` before new work is accepted where possible;
- no unbounded retry loop.

#### Timer work

- bounded by resource ownership/content policy;
- overdue catch-up must itself be bounded so a long stall cannot create an infinite catch-up loop;
- correctness of deadlines remains explicit even when work must be spread across turns.

#### Auxiliary work

- bounded submissions/results;
- cancellation and stale-result rejection are normal outcomes;
- overload may degrade optional AI/pathfinding quality only where a gameplay contract explicitly permits a safe deterministic fallback;
- no worker queue may block the logical writer indefinitely.

#### Outbound client state

- bounded per connection/session;
- a slow client cannot force unbounded server memory;
- state may be coalesced/rebased through explicit FND-02 reconciliation semantics when safe;
- when safe bounded recovery is impossible, disconnect/resync is preferable to blocking authoritative simulation.

#### Best-effort telemetry

- bounded and explicitly fail-open with loss counters/quality metadata as already required by the foundation failure catalogue;
- telemetry loss never blocks gameplay or masquerades as complete evidence.

#### Durable mutation/audit evidence

- not silently dropped;
- where required evidence cannot be committed/preserved, the owning durable mutation fails closed or the scope enters an explicit degraded state according to `DUR-*`/`ANL-01`;
- FND-03 must preserve this boundary but does not finalize outbox/database technology.

### Numeric limits

Concrete limits must enter `RESOURCE_LIMITS_REGISTRY.json` only when FND-03 or a later owning contract has evidence for them. A configurable queue size never removes an absolute hard maximum.

### Decision timing

- **Must decide now?** YES for queue classes/failure semantics; NO for benchmark-sensitive capacities.
- **Blocked work:** safe runtime implementation and overload tests.
- **Superseding evidence:** measured workload/latency/memory evidence may tune limits/algorithms without converting bounded queues to unbounded ones.

## 11. Reconnect/protection timing integration

The runtime must consume, not redefine, the accepted session/reconnect semantics.

Accepted existing policy includes conceptually:

- bounded liveness classification owned across FND-03/FND-04;
- same logical GameSession may survive eligible transport reconnect;
- connection generation advances on successful rebind;
- initial reconnect grace window is 15 seconds;
- unexpected loss of playable control may activate the accepted defensive PvE protection rules;
- valid re-entry receives the accepted four-second defensive PvE protection window;
- graceful logout/login does not manufacture that window;
- reconnect does not reset combat, position, conditions, cooldowns, threat, encounter or committed effects.

FND-03 must provide monotonic server-side deadline/timer mechanics capable of representing those policies. FND-04 remains owner of the final session/admission/lease/reconnect state machine and exact eligibility transitions.

## 12. Generation-fenced recovery

After GameNode/process failure or scope relocation:

- replacement process receives a new `NodeId`;
- semantic `WorldId + ChannelId` or `WorldId + InstanceId` remains stable where recovery preserves the same scope;
- recovery obtains a newer ownership generation before authoritative mutation resumes;
- stale worker results/timers/control messages from the old ownership generation are invalid;
- stale durable writes remain rejected by the relevant durable fence;
- client reconciliation uses the already accepted FND-02 snapshot/delta generation/sequence rules;
- recovery may not pretend to preserve a resumable GameSession if required CommandId/session ordering state is lost beyond safe reconstruction.

Exact checkpoint/journal implementation and RPO/RTO remain with `DUR-02`/`OPS-CHANNEL-01` and measured operational evidence.

## 13. Event emission and replay boundary

Authoritative state transition happens before non-authoritative analytics consumers act on the event.

Conceptually:

```text
normalized input
  -> authoritative transition
  -> committed authoritative outcome
  -> versioned event/audit/telemetry records
  -> external consumers
```

A telemetry/analytics consumer must not call back synchronously to mutate the same authoritative transition.

Replay has at least two distinct meanings that must not be conflated:

1. **deterministic simulation/test replay** — re-feed recorded normalized inputs/clocks into an isolated authoritative runtime/test model to reproduce state transitions;
2. **analytics/event replay** — re-deliver versioned events to read-only/idempotent analytical consumers without replaying gameplay mutation.

The exact event envelope, EventId/causation/correlation identity and transactional outbox behavior remain `ANL-01`/`DUR-*` work.

## 14. Initial modular-monolith topology

The accepted initial runtime direction remains a modular GameNode process capable of hosting multiple authoritative scopes.

Conceptually:

```text
GameNode process / NodeRuntime
├── transport/session adapters
├── explicit WorldServices clients/components
├── ChannelRuntime W/C1
├── ChannelRuntime W/C2
├── InstanceRuntime W/I1
├── bounded simulation scheduling
├── bounded auxiliary CPU workers
├── bounded persistence/event adapters
└── observability/health/capacity surfaces
```

`WorldServices` is not an untyped process-global bag. Each world-shared mutable domain must have a named owner/consistency model according to the multichannel scope matrix.

Process placement is not semantic ownership: a later service may move out of process without changing the domain contract if its ownership/consistency semantics are preserved.

## 15. Player/producer impact

### Player

This direction is intended to protect:

- responsive input without tying every action to a coarse global tick;
- fair ordering without CharacterId/account sorting bias;
- predictable cooldown/reconnect timing;
- no one-channel overload causing unbounded memory growth;
- no stale worker/process regaining authority;
- recoverable state with explicit snapshot/resync rather than guessed client state.

### Producer/operator

This direction preserves:

- one execution model instead of duplicated channel/instance engines;
- ability to tune executor/thread topology from benchmarks;
- deterministic fault reproduction;
- explicit overload/degraded states;
- migration path from modular monolith to more distributed placement without redefining gameplay identities.

The cost is deliberate internal metadata for generations/revisions/input ordering and more explicit queue/failure handling than a traditional single-thread game loop.

## 16. Required FND-03 continuation analysis

Before the final complete FND-03 contract can be accepted, continue resolving at least:

1. exact `NodeRuntime` versus `WorldServices` responsibilities and lifecycle boundaries;
2. scope lifecycle state machine and readiness/drain/recovery transitions;
3. deterministic timer catch-up/coalescing rules for long stalls;
4. precise owner-turn fairness/work-budget invariants across many scopes on one GameNode;
5. runtime panic/internal-invariant failure isolation policy;
6. exact behavior when required dependency responses are delayed/lost during an authoritative transition;
7. minimum runtime resource limits that genuinely need numeric hard maxima before implementation;
8. deterministic replay evidence format sufficient for QA without pre-empting ANL-01;
9. event-emission cut points relative to authoritative commit and later durable transaction boundaries;
10. mapping of every applicable `FOUNDATION_FAILURE_SCENARIOS.md` scenario to FND-03 expected outcome/owner.

These subjects should be decided only to the depth required to make the later runtime implementation safe and testable.

## 17. Non-authorization

This document is architecture analysis only.

It does not authorize:

- Rust runtime implementation;
- creation of speculative runtime crates solely to mirror diagrams;
- production `protocol-oteryn` listener/client adapter;
- Game Session admission/lease implementation;
- persistence/database schema;
- live channel scaling;
- production diagnostics/Guardian;
- Game Intelligence detector or sanctions;
- external-repository changes;
- production deployment.

The complete FND-03 contract requires its own final review, exact-head documentation/governance validation, independent architecture audit and merge before implementation claims may rely on it.
