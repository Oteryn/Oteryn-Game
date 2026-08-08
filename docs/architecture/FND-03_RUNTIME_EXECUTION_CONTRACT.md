# FND-03 — Authoritative Runtime Execution Contract

- Status: Candidate architecture contract
- Date: 2026-08-08
- Gate: `FND-03`
- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Repository: `blakinio/Oteryn-v2`
- Applies to: authoritative Rust GameNode execution, channel and instance mutation ownership, clocks, timers, command scheduling, bounded work queues, auxiliary work, drain/checkpoint/recovery boundaries, runtime liveness integration and deterministic evidence
- Consumes: ADR-0001, ADR-0009, FND-ID-01, FND-02, the accepted instance/runtime baselines, disconnect/re-entry package, resource-limit policy, foundation error vocabulary and foundation failure scenarios
- Does not authorize: Rust runtime implementation, production listener/client adapter, FND-04 admission/lease implementation, persistence schema, production orchestration, production telemetry, client diagnostics, Launcher/Guardian implementation or deployment

## 1. Purpose

`FND-03` freezes the minimum runtime execution semantics required to implement the authoritative Rust game server without letting operating-system thread scheduling, unbounded queues, stale asynchronous work or process placement become implicit gameplay authority.

The contract preserves the accepted architecture:

```text
GameNode process
    NodeRuntime
      ├── explicit WorldServices clients/owners
      ├── zero or more ChannelRuntime owners
      ├── zero or more InstanceRuntime owners
      ├── bounded I/O completion paths
      └── bounded auxiliary compute capacity
```

The central rule is:

> concurrency may compute and transport work in parallel; authoritative gameplay mutation is committed only by the current logical owner of the affected simulation scope through one explicit ordered execution boundary.

This contract is not a promise that all gameplay runs on one operating-system thread. It is a promise that concurrency never creates multiple mutation authorities for the same channel or instance.

## 2. Decision timing

### Must decide now — YES

The following must be fixed before authoritative runtime implementation can safely consume FND-02:

- logical runtime ownership boundaries;
- distinction between canonical identity, ownership generation and physical placement;
- command/timer/lifecycle/completion entry into authoritative execution;
- clock authority for deadlines and gameplay timing;
- bounded queue topology and overload semantics;
- stale auxiliary-result rejection;
- writer/non-blocking I/O boundary;
- drain, fence, checkpoint-cut and recovery activation semantics;
- disconnect/liveness timer integration;
- deterministic evidence sufficient to explain and replay authoritative ordering.

### Concrete downstream work blocked

This contract blocks:

- authoritative `services/game-server` runtime implementation claims;
- safe integration of FND-02 command ingress with channel/instance execution;
- runtime-dependent movement/combat/AI vertical-slice packages;
- FND-04 finalization where reconnect/session states depend on runtime liveness and actor ownership;
- implementation of channel/instance recovery orchestration interfaces.

### What becomes harder if decided incorrectly now

If thread arrival order, process-global mutable state, blocking I/O or unbounded background queues become implicit authority, later correction would require rewriting gameplay ordering, reconnect/recovery, anti-duplication evidence and testing infrastructure.

If benchmark-sensitive capacities or executor technology are frozen now without evidence, the architecture would create avoidable migration cost and false performance claims.

### Evidence that may justify supersession

A later contract may supersede specific runtime choices when named evidence demonstrates a material need, including:

- representative latency/capacity benchmarks;
- deterministic replay failures;
- security findings around stale work or split ownership;
- fault-injection/recovery evidence;
- profiling that proves an accepted boundary causes unacceptable contention;
- changed product requirements that cannot be satisfied by the extension model.

### Deliberately not decided

This contract does not select:

- Tokio or another async runtime;
- worker-pool library;
- operating-system thread count or CPU affinity;
- exact worker count;
- fixed global tick frequency;
- numeric internal queue capacities that require benchmark evidence;
- persistence schema/checkpoint encoding/journal technology;
- RPO/RTO;
- orchestrator product;
- exact heartbeat cadence/hysteresis or reconnect credentials;
- gameplay-specific movement/combat tie-breaking;
- event broker or telemetry backend.

## 3. Runtime ownership model

### 3.1 `NodeRuntime`

`NodeRuntime` is the process-incarnation execution coordinator for one `NodeId`.

It owns process-local lifecycle and bounded shared execution resources such as:

- registration/readiness state for the current GameNode incarnation;
- the current set of assigned channel and instance runtime owners;
- bounded networking/service-I/O integration;
- bounded auxiliary compute scheduling;
- process-level cancellation/shutdown coordination;
- process health and resource-pressure observations;
- routing of typed results to their declared logical owner.

`NodeRuntime` does **not** become the semantic owner of all gameplay state merely because objects reside in its address space.

Process-global mutable gameplay state is prohibited unless a named world/service owner and scope explicitly authorize it.

A new GameNode process incarnation receives a fresh canonical `NodeId`. `NodeId` identifies the process incarnation; it does not grant channel or instance mutation authority.

### 3.2 `WorldServices`

World-shared systems remain explicit logical owners according to the multichannel scope matrix. Examples include world communication, market, party/social state, reward eligibility, selected character/persistence services and other world-scoped domains.

Initial implementations may colocate a service client or even an owner in the same GameNode process when separately accepted, but placement never changes its semantic scope.

Rules:

- `ChannelRuntime` and `InstanceRuntime` access world-shared state through typed owner/service boundaries;
- no runtime discovers a mutable default world or service through a process singleton;
- service requests carry explicit `WorldId` and other required scope identities;
- asynchronous service responses re-enter authoritative simulation as typed inputs and are revalidated before mutation;
- immutable revisioned caches are allowed when their scope/revision/invalidation rules are explicit;
- cache presence never converts the GameNode into the durable owner of world-shared state.

### 3.3 `ChannelRuntime`

Each active `WorldId + ChannelId` has exactly one current logical authoritative mutation owner.

The `ChannelRuntime` owns channel-local simulation, including the public-map runtime overlay, channel-local creatures/spawns/AI, channel-local movement/visibility/combat, local NPC runtime and other state classified as channel-local.

All authoritative channel-local mutation crosses one **Channel Execution Boundary**.

The boundary is logical. The implementation may move the owner between worker threads over time, but two threads/tasks may not concurrently commit channel-local authoritative mutation.

### 3.4 `InstanceRuntime`

Each active `WorldId + InstanceId` has exactly one current logical authoritative mutation owner.

After a committed Channel/Instance handoff, the `InstanceRuntime` owns admitted characters and all instance-local simulation. Source channels no longer mutate those characters' instance-local state.

An instance is not a hidden sub-channel and its canonical identity does not include origin `ChannelId` or current `NodeId`.

`InstanceRuntime` may execute on the same GameNode as an origin channel, on another GameNode, or later move under a separately accepted recovery/operations contract. Physical placement does not alter semantic instance identity.

### 3.5 One owner, multiple execution resources

A logical owner may delegate bounded non-authoritative work, including pathfinding, AI planning, visibility candidate computation, serialization preparation, compression preparation and persistence-request preparation.

Delegation never transfers mutation authority.

The result returns to the current owner as a proposal/input; only the owner may accept it into authoritative state.

## 4. Identity, placement and fencing are separate

Runtime correctness uses three independent concepts:

```text
canonical semantic identity
+ current ownership/fencing generation
+ current execution placement
```

Examples:

```text
ChannelRef = WorldId + ChannelId
InstanceRef = WorldId + InstanceId
NodeId     = one GameNode process incarnation
```

A channel or recoverable concrete instance may preserve identity while both placement and ownership generation change.

### 4.1 Ownership generation

Every authoritative ChannelRuntime and InstanceRuntime activation is bound to a current **ownership generation** scoped to that semantic runtime identity.

Ownership generation is an ordering/fencing value, not an entity identifier.

Required semantics:

- only the currently accepted generation may mutate, emit authoritative state or submit generation-fenced durable work for that scope;
- recovery/relocation/authority replacement establishes a strictly newer generation;
- a generation is never reused for a later authority period;
- stale generation work fails closed even when `NodeId`, channel/instance identity or payload is otherwise valid;
- exhaustion/wrap must fail closed rather than reuse authority;
- exact physical width/storage/allocator belongs to the owning operations/durability contract unless a public boundary later requires a fixed representation.

### 4.2 Runtime-local handles

Runtime-local entity/task/queue handles may use compact representations for performance, but they are valid only inside the explicitly named owner generation/allocation context.

A runtime-local handle cannot replace canonical identity in:

- cross-runtime messages;
- durable state;
- audit/evidence that outlives the allocation;
- recovery boundaries;
- security/authority checks.

Recycled local slots require a local generation/version so a stale handle cannot address a new object occupying the same slot.

## 5. Authoritative execution boundary

Every mutation-capable runtime input is normalized into a typed owner-scoped input before it can affect authoritative state.

Input classes include, as applicable:

- reserved player commands;
- due gameplay timers;
- ownership/lifecycle/fencing transitions;
- world/service completion results;
- auxiliary compute results;
- accepted handoff transitions;
- bounded administrative/system commands under their owning contract;
- recovery/reconciliation inputs.

Network packets, database callbacks, worker-thread completions and raw wall-clock callbacks do not mutate gameplay directly.

### 5.1 Owner execution ordinal

Each active authoritative owner generation maintains a monotonically increasing **execution ordinal** scoped to that generation.

The ordinal:

- is assigned only inside the authoritative execution boundary;
- records the accepted order in which mutation-capable inputs are resolved by that owner;
- starts from a defined non-zero value for a new owner generation;
- never moves backward or reuses a value within that generation;
- is an ordering/evidence value, not a durable entity identity or client credential;
- does not replace FND-02 `CommandId`, `server_sequence` or state-domain revisions.

The logical authoritative ordering key is therefore equivalent to:

```text
(runtime identity, ownership generation, execution ordinal)
```

This makes the accepted live order observable and replayable without pretending that operating-system thread wake-up order or packet-receive interleaving is a stable gameplay contract.

Exact compact in-memory representation is an implementation detail; if the value crosses a durable/public boundary, its representation must be frozen by that owning contract before use.

### 5.2 Commands

FND-03 consumes FND-02 command semantics unchanged:

```text
CommandRef = (GameSessionId, CommandId)
```

Within one `GameSessionId`:

- `CommandId` order is authoritative;
- an already reserved command is never executed twice;
- a later reserved CommandId cannot commit authoritative gameplay effects ahead of an earlier reserved CommandId when FND-02 requires ordered commit;
- reconnect preserving the same GameSession must preserve enough pending/high-water state to avoid duplicate execution or contradictory ordering.

FND-03 introduces no second client command sequence.

A command is mapped from the session-ingress stream to exactly one authoritative runtime owner according to current character/spatial ownership. A stale source owner cannot accept a command after a committed handoff/fence.

### 5.3 Cross-session ordering

FND-02 does not define one global packet-arrival order across clients. FND-03 therefore does not treat concurrent network-thread arrival as a canonical tie-break.

The runtime must provide bounded fair arbitration among ready session streams so one client cannot monopolize an owner by filling its valid command window.

Binding invariants:

- each session stream preserves its own CommandId order;
- arbitration does not drain one continuously busy stream without bounded opportunity for other ready streams;
- the owner assigns the resulting execution ordinal;
- the chosen accepted live order is retained in deterministic evidence when it can affect a gameplay result;
- later gameplay contracts may add domain-specific conflict/tie-break rules, but they may not bypass the owner boundary.

The exact data structure/round-robin implementation or scheduling quantum is benchmark-sensitive and not frozen here.

### 5.4 Control/fencing events

Authority-loss/fencing input has safety precedence once the current owner observes a valid newer fence.

After the fence becomes authoritative, the old generation must not perform "one final" gameplay mutation, persistence write, outbound authoritative delta or checkpoint commit.

Queued old-generation gameplay may be rejected/invalidated according to command/session recovery semantics; it is never allowed to mutate after fencing merely because it arrived earlier at a worker thread.

## 6. Clock model

FND-03 requires three distinct time/order domains.

### 6.1 Wall clock

Wall-clock time is used for operator, audit and cross-system correlation.

It is not authoritative for gameplay duration or in-process deadline progression.

NTP/system-clock adjustment must not make a gameplay timeout fire early, repeat, move backward or extend unexpectedly.

### 6.2 Monotonic elapsed time

A monotonic clock is authoritative for in-process elapsed durations and deadlines such as:

- disconnect/liveness elapsed time;
- stale-transport cleanup timing;
- re-entry protection duration;
- bounded runtime deadlines;
- queue/work time budgets and timeout measurement where applicable.

A process-local monotonic instant is valid only inside that process incarnation. It is not serialized as a cross-process timestamp and must not be compared directly after a new `NodeId` starts.

Durable timers that must survive process failure require semantic reconstruction under their owning persistence/gameplay contract rather than persistence of an opaque process monotonic instant.

### 6.3 Authoritative execution order

Execution ordinal plus state-domain revisions and FND-02 sequencing describe authoritative simulation order. Wall-clock timestamps do not replace them.

### 6.4 No fixed global tick contract

FND-03 does not impose one universal fixed-rate global simulation tick.

`ChannelRuntime` and `InstanceRuntime` execute bounded ordered work cycles and monotonic-deadline timers. A gameplay subsystem may define a cadence when its own contract requires one, but a two-second combat policy does not imply a two-second server loop.

The exact scheduling quantum/tick frequency is implementation/performance work and must be measured rather than guessed.

### 6.5 Deterministic test clocks

Runtime core logic must be testable with an explicit deterministic/virtual clock source.

Tests must be able to advance monotonic time without sleeping on wall clock and must reproduce ordering around boundaries such as:

- just before / exactly at / just after a deadline;
- clock skew or wall-clock jumps;
- multiple timers due at one instant;
- liveness recovery racing a protection boundary;
- drain/fence/recovery transitions.

Production clock implementations and test clocks share semantics; test-only time travel may not be exposed to production gameplay authority.

## 7. Timer contract

Timers are owner-scoped authoritative inputs, not independent mutation callbacks.

### 7.1 Scheduling

A mutation-capable timer is created/cancelled through the current authoritative owner and is bound to:

- the owner runtime identity;
- current ownership generation;
- a monotonic due deadline or owning durable semantic deadline;
- a stable owner-local schedule ordinal/token sufficient to order equal deadlines and reject stale cancellation/fire attempts;
- the target entity/state generation where applicable.

### 7.2 Firing order

Due timers are admitted through the execution boundary.

For timers with equal effective deadline, a stable owner-assigned schedule order must determine their relative runtime admission; operating-system wake-up order is not a tie-break.

### 7.3 Cancellation and stale timers

Timer cancellation/invalidating entity generation prevents a stale timer from mutating a replacement entity or recovered owner.

A timer from an old ownership generation is invalid after fencing/recovery.

### 7.4 Missed deadlines and stalls

A scheduler stall must not create an unbounded "fire every missed interval" burst.

Every recurring/periodic timer family must declare one bounded missed-deadline policy under its owning gameplay contract, such as coalescing, skipping to the next period, or bounded catch-up when gameplay semantics require it.

The generic runtime does not invent unlimited catch-up semantics.

If timer lateness exceeds accepted runtime health thresholds, the runtime exposes degradation evidence; it does not silently discard authoritative required timers.

## 8. Auxiliary parallel work

Parallel work is allowed only when its authority is explicit.

### 8.1 Work request

An auxiliary request contains or is bound to enough immutable input to validate its result, including where applicable:

- runtime identity;
- ownership generation;
- source state/domain revision;
- target entity/local-generation context;
- owner-assigned work token/ordinal;
- cancellation/deadline;
- revisioned content/ruleset input.

The worker receives data/proposals, not mutable authoritative references.

### 8.2 Work result

A worker result is untrusted with respect to current authority until the owner revalidates it.

Before acceptance the writer checks at minimum:

- same current runtime identity;
- same current ownership generation;
- compatible source state/entity generation;
- work has not been cancelled or expired;
- result still satisfies domain preconditions;
- result resource limits are valid.

Stale/late/misrouted results are discarded as non-authoritative and counted diagnostically. Discarding a stale proposal is not rollback because the worker never had mutation authority.

### 8.3 Completion timing cannot be gameplay authority

Operating-system thread completion order must not be the sole rule deciding conflicting gameplay outcomes.

When asynchronous completion can materially affect an outcome, the owning subsystem must provide at least one reproducible mechanism:

- a stable owner-assigned ordering/conflict key;
- a deterministic deadline/fallback rule;
- or retained authoritative evidence of the accepted result order sufficient for deterministic replay.

The runtime must not hide a player-visible outcome behind nondeterministic worker wake-up timing that cannot later be reconstructed.

### 8.4 No writer blocking

The authoritative owner must not synchronously wait on network, database, remote service or expensive CPU work while holding the mutation lane.

If a command depends on asynchronous work, its authoritative lifecycle remains pending until a typed completion/failure re-enters the owner boundary. Other legal owner work may continue according to ordering constraints.

## 9. Queue and executor topology

Every queue/executor is bounded. No hidden unbounded channel, task spawn, retry list or callback accumulation is permitted in the authoritative runtime.

Minimum queue/resource classes include:

1. FND-02 transport/frame decode limits;
2. per-GameSession reserved-command ingress, including the FND-02 hard maximum of 64 outstanding commands;
3. owner control/fencing input capacity;
4. owner ordinary gameplay-ready capacity or bounded ready-stream metadata;
5. scheduled timer population and due-work capacity;
6. auxiliary CPU work queue/in-flight work;
7. service/database I/O request and completion queues;
8. per-session outbound authoritative state/control capacity in both entries and bytes;
9. best-effort gameplay telemetry queue;
10. durable audit/outbox handoff boundaries under ANL/DUR contracts.

### 9.1 Numeric limit policy

FND-03 does **not** guess numeric internal capacities that depend on representative workload and memory/latency trade-offs.

Before any FND-03 runtime implementation is accepted, every externally influenced or memory-growth-relevant runtime queue/count/byte boundary must have a concrete entry in `RESOURCE_LIMITS_REGISTRY.json` (or an explicitly superseding registry), including boundary tests and failure category.

The selected implementation task must derive configurable defaults and hard ceilings from bounded stress/benchmark evidence while respecting already fixed FND-02 limits.

Architecture acceptance therefore freezes **boundedness and failure semantics now** and deliberately defers **benchmark-sensitive numeric values** until evidence exists.

## 10. Backpressure and overload

### 10.1 General rule

Overload is an explicit runtime state, not a reason to grow memory without bound or silently lose authoritative work.

Every bounded path defines:

- admission point;
- capacity unit (entries, bytes, in-flight work or both);
- rejection/backpressure behavior;
- whether work is authoritative, retryable or best-effort;
- metrics/health evidence;
- recovery/hysteresis behavior.

### 10.2 Before command reservation

If runtime capacity required to safely admit a **new** command is unavailable before FND-02 reservation, the command is not reserved, `next_command_id` does not advance and the bounded outcome maps to `CAPACITY_EXCEEDED`/the FND-02 retry semantics for that same CommandId.

### 10.3 After command reservation

Once a command identity has been reserved by authoritative session ingress, runtime congestion cannot silently drop it or reinterpret a retry as a new operation.

The runtime must preserve the pending identity in bounded state, stop admitting additional unsafe work and eventually produce one terminal authoritative result or enter a separately governed session/recovery terminal state that preserves the no-double-execution invariant.

### 10.4 Control-path reserve

Gameplay saturation must not prevent current ownership/fencing/shutdown signals from reaching the owner. Control/fence input therefore requires an independently bounded admission path or equivalent capacity reservation that cannot be consumed entirely by ordinary player traffic.

### 10.5 Required timers

If an operation would require registering an authoritative timer and the runtime cannot reserve bounded timer capacity, the operation fails before committing the state that requires that timer.

Already accepted authoritative timers are not silently dropped because the due queue is busy.

### 10.6 Slow clients

Per-session outbound state is bounded by entries and bytes.

When a client cannot consume state fast enough:

1. the runtime stops accumulating an unbounded sequence of obsolete deltas;
2. where FND-02 semantics allow, the session is marked for explicit resynchronization/full replacement snapshot rather than retaining every superseded delta;
3. gameplay/control authority remains server-side;
4. bounded liveness/transport policy may eventually close the stale/slow transport;
5. FND-04 decides logical GameSession continuity/reconnect eligibility.

A slow client cannot force unbounded memory, stall a whole channel writer or make the server discard authoritative state.

### 10.7 Telemetry versus audit

Best-effort gameplay telemetry may drop only under the explicit ADR-0006/ANL policy, with counted loss and bounded queues.

Required durable economy/security audit evidence never silently downgrades to best effort. Its failure behavior remains governed by ANL-01/DUR contracts and may block the owning risky durable mutation.

## 11. I/O and dependency boundary

Network, database and remote-service I/O execute outside the authoritative mutation lane.

A request emitted from authoritative state carries the identity/revision/generation required for the completion to be validated when it returns.

A completion callback never mutates state directly; it becomes a typed owner input.

For durable operations, FND-03 does not decide whether local state commits before, with or after PostgreSQL. The owning DUR contract defines the atomicity point. FND-03 only guarantees that no asynchronous persistence completion bypasses current owner generation, operation identity or state-revision validation.

Dependency degradation is represented explicitly. It is not hidden behind indefinite retries.

Bounded outcomes map to `DEPENDENCY_UNAVAILABLE`, `TIMEOUT`, `CANCELLED`, `STALE_GENERATION`, `CAPACITY_EXCEEDED` or another accepted foundation category as appropriate.

## 12. Disconnect and liveness integration

FND-03 consumes the accepted disconnect/liveness policies without moving FND-04-owned session eligibility into runtime execution.

### 12.1 Authority

Only server-observed control/liveness evidence from the currently authoritative connection generation may advance/recover real-time control liveness.

No gameplay-command silence, client self-report, client crash report, OS event or Launcher/Guardian observation is real-time protection authority.

### 12.2 Timer integration

Runtime timers use monotonic elapsed time and preserve the accepted policy inputs:

```text
elapsed < 2.0 s             -> ordinary PvE behavior
elapsed >= 2.0 s            -> disconnect protection active
5 s stale transport policy  -> transport cleanup boundary, not actor-state destruction
15 s reconnect grace input  -> logical GameSession continuity policy owned by FND-04
valid unexpected-loss return -> 4 s defensive PvE re-entry protection
```

The 15-second value is an accepted FND-04 policy input. FND-03 does not define the final authoritative start state for that window.

The five-second transport close must not discard or safe-log a combat/PZ/logout-locked actor whose authoritative gameplay presence remains required.

### 12.3 Server stall discrimination

GameNode/channel writer overload or process scheduling delay must be observable separately from player-side liveness loss.

A stalled authoritative owner must not blame all affected players for missing heartbeat progress when local runtime execution itself could not process the evidence.

Liveness evaluation therefore consumes runtime-health/queue-latency evidence sufficient for FND-04 and forensic consumers to distinguish local overload from isolated client/path loss.

### 12.4 Re-entry protection execution

The four-second re-entry interval is an owner-scoped monotonic timer/effect on the same authoritative actor.

Activation/expiry is prospective and ordered through the owner boundary. It does not rewind committed effects or reset character/combat/encounter state.

Allowed/prohibited actions follow the delivered re-entry decision. Prohibited outgoing actions are rejected/non-buffered and never burst at expiry.

## 13. Runtime lifecycle

FND-03 refines the ADR-0009 lifecycle without selecting an orchestrator.

### 13.1 GameNode bootstrap

A new process:

1. creates a fresh canonical `NodeId` for this process incarnation;
2. initializes bounded runtime resources and immutable configuration/revision evidence;
3. authenticates/registers according to the later operations/control-plane contract;
4. is not authorized to mutate any channel/instance until a current assignment and ownership generation are established;
5. exposes readiness only after required runtime invariants and assigned-runtime warmup checks pass.

### 13.2 Runtime activation

A ChannelRuntime/InstanceRuntime becomes authoritative only after:

- canonical identity and revision compatibility are validated;
- current ownership generation is established;
- any required recovery state is loaded/validated;
- stale prior owner is fenced by the authority mechanism;
- execution queues/timers required for activation have bounded capacity;
- readiness transition commits inside the runtime lifecycle boundary.

`Warming` or `Recovering` does not mean mutation authority is already public/routable.

### 13.3 Draining

Drain is an ordered state transition.

When drain begins:

- new admissions/routings are stopped by their owning control plane;
- the owner stops starting new work classes that cannot safely complete before the drain boundary;
- already reserved authoritative commands are resolved, rejected or preserved according to their command/session contract rather than silently dropped;
- handoffs and risky durable operations either reach a defined safe commit/abort boundary or prevent terminal drain completion;
- timers and owner state reach a documented checkpointable boundary;
- drain has a bounded deadline and observable reason/state.

### 13.4 Checkpoint cut

A checkpoint request becomes a typed owner input.

The checkpoint captures one explicit authoritative cut identified by the runtime identity, ownership generation, execution ordinal/state revisions and any required pending-command/session/timer/handoff metadata defined by downstream persistence/session contracts.

The cut means:

```text
all authoritative effects before cut are included/represented
all effects after cut are excluded from that checkpoint version
```

Checkpoint serialization/storage may occur asynchronously, but it cannot claim a later state than the captured cut.

FND-03 does not select checkpoint encoding, journal scope, RPO/RTO or PostgreSQL layout; those remain DUR-02/OPS work.

### 13.5 Fencing

Once a newer ownership generation is authoritative or the current generation is explicitly fenced:

- old owner mutation stops immediately at the logical boundary;
- old owner emits no new authoritative deltas/results/readiness claims;
- old generation durable writes are rejected by downstream fences;
- outstanding auxiliary/service completions from the old generation cannot regain authority;
- shutdown/cleanup may continue only as non-authoritative cleanup/evidence work.

### 13.6 Recovery

Recovery preserves semantic runtime identity when recovering the same channel or same permitted concrete instance lifecycle but establishes a fresh ownership generation and may use a new `NodeId`.

Recovery validates immutable World Bundle/content/ruleset/protocol/persistence revisions before readiness.

The runtime does not claim same-GameSession resume after process failure unless FND-04/DUR can prove all FND-02 session command high-water/pending/result/reconciliation state required for safe continuation is preserved or reconstructable. Otherwise the old logical session terminates safely and recovery follows the fresh-session path accepted by ADR-0009/FND-04.

A failed channel is not silently replaced by a different ChannelId for player convenience.

## 14. Handoff and ownership transfer integration

Channel↔Instance and later accepted runtime ownership transfers use the canonical `HandoffId` identity plus current source/destination ownership generations.

FND-03 owns the runtime execution part of prepare/commit/fence/activate; FND-04 owns admission/session authorization and credential semantics; DUR owns durable item/state safety.

Binding runtime properties:

- source and destination may prepare concurrently but at most one is authoritative for the character at any instant;
- final ownership commit is one explicit ordered barrier;
- after commit the source generation cannot mutate transferred state;
- retry/resume of the same logical handoff reuses the same HandoffId;
- stale handoff completion is rejected by generation/state checks;
- failure before commit preserves/restores the previous safe owner;
- failure after commit recovers from destination authority evidence, not client claims;
- full authoritative snapshot/resynchronization establishes destination client state before ordinary deltas continue.

Exact session/lease transaction and handoff token/message shape remain FND-04.

## 15. Determinism and replay evidence

Oteryn requires deterministic **explanation/replay of accepted authoritative ordering**, not the false claim that every live concurrent execution has identical wall-clock interleaving.

For any scenario where concurrency can change an authoritative result, retained test/audit/replay evidence must be sufficient to reconstruct the accepted order using applicable data such as:

- runtime identity and ownership generation;
- execution ordinals;
- `GameSessionId` + `CommandId`;
- timer schedule/due ordering;
- handoff identity/generation;
- accepted auxiliary/service completion order when material;
- deterministic random seed/state under the owning gameplay contract;
- content/ruleset/build/protocol revisions;
- state-domain revisions.

A deterministic replay harness must be able to feed recorded logical inputs through deterministic/virtual clocks without depending on original CPU count, thread IDs or task wake-up timing.

A subsystem that uses randomness must obtain it from an explicit deterministic source scoped/versioned by its own contract rather than process-global entropy during authoritative execution.

## 16. Failure and error semantics

Public/cross-component runtime outcomes use the foundation vocabulary.

Typical mappings:

| Runtime condition | Foundation category | Required mutation result |
|---|---|---|
| stale ownership/session/entity generation | `STALE_GENERATION` | no stale mutation commits |
| bounded work/queue capacity unavailable before admission | `CAPACITY_EXCEEDED` | rejected before affected work is committed/reserved as applicable |
| dependency unavailable | `DEPENDENCY_UNAVAILABLE` | no unsafe partial durable mutation |
| named runtime deadline expires | `TIMEOUT` | owning operation defines terminal/pending cleanup state |
| explicit cancellation before commit | `CANCELLED` | no hidden partial success |
| current owner/state prevents requested transition | `CONFLICT` | preserve current authority/state |
| unexpected internal condition | `INTERNAL_UNAVAILABLE` | fail closed, retain internal diagnostics without leaking implementation details |

Narrow contract-owned symbolic codes may be introduced by implementation/public-message contracts, but they must map to these stable categories and must not expose unstable exception/log text as protocol behavior.

## 17. Foundation failure-scenario disposition

The following table records **contract-level disposition**, not runtime implementation proof. `PASS` means FND-03 defines the required invariant; executable proof remains mandatory for later implementation/E2E acceptance.

| Scenario | FND-03 disposition | Runtime requirement / owning follow-up |
|---|---|---|
| `FS-PLATFORM-UNAVAILABLE` | `DEFERRED_BY_ACCEPTED_GATE` | FND-04 owns admission behavior; runtime creates no alternate credential authority. |
| `FS-GATEWAY-AFTER-REDEEM` | `DEFERRED_BY_ACCEPTED_GATE` | FND-04/Platform reconciliation. |
| `FS-POSTGRES-UNAVAILABLE` | `DEFERRED_BY_ACCEPTED_GATE` | FND-03 exposes bounded dependency degradation; DUR-02 defines durable-operation atomicity/recovery. |
| `FS-LEASE-RENEW-TIMEOUT` | `DEFERRED_BY_ACCEPTED_GATE` | FND-04 owns lease timer/renewal state; FND-03 must honor a resulting fence without stale mutation. |
| `FS-DUPLICATE-LOGIN` | `DEFERRED_BY_ACCEPTED_GATE` | FND-04 owns admission race; runtime permits at most current authoritative owner. |
| `FS-STALE-GENERATION` | `PASS` | stale owner/session/entity/worker result cannot mutate. |
| `FS-DUPLICATE-COMMAND` | `PASS` | consumes FND-02 reservation/high-water semantics; no duplicate execution. |
| `FS-CHANNEL-SPLIT-OWNER` | `PASS` | one current ownership generation; stale writer is fenced. |
| `FS-CHANNEL-DRAIN` | `PASS` | ordered drain; no silent reserved-work loss; safe checkpoint/abort boundary. |
| `FS-QUEUE-SATURATION` | `PASS` | bounded queues/backpressure; no unbounded growth or silent authoritative loss. |
| `FS-SLOW-CLIENT` | `PASS` | bounded outbound memory; resync/transport-close path, server remains authoritative. |
| `FS-CLOCK-SKEW` | `PASS` | wall-clock changes do not drive monotonic deadlines. |
| `FS-KEY-ROTATION` | `DEFERRED_BY_ACCEPTED_GATE` | FND-04/security contracts. |
| `FS-REVISION-MISMATCH` | `PASS` | no mixed-authority runtime activation; consumes FND-02/registry compatibility. |
| `FS-SNAPSHOT-DELTA-MISMATCH` | `PASS` | runtime preserves explicit FND-02 resync barrier; no partial guessed state. |
| `FS-DB-OUTBOX-BOUNDARY` | `DEFERRED_BY_ACCEPTED_GATE` | DUR-02/ANL-01 own atomic durable boundary. |
| `FS-WORLD-BUNDLE-CORRUPT` | `DEFERRED_BY_ACCEPTED_GATE` | DUR-04 loader/compiler owns corruption limits; FND-03 never activates invalid revisions. |
| `FS-CLIENT-CUTOVER-ROLLBACK` | `NOT_APPLICABLE` | completed migration lifecycle, not runtime execution. |
| `FS-ANALYTICS-TELEMETRY-OVERFLOW` | `PASS` | best-effort queue bounded/drop-counted; no gameplay authority. |
| `FS-AUDIT-OUTBOX-BACKLOG` | `DEFERRED_BY_ACCEPTED_GATE` | ANL-01/DUR-02. |
| `FS-EVENT-DUPLICATE-DELIVERY` | `DEFERRED_BY_ACCEPTED_GATE` | ANL-01 consumer semantics. |
| `FS-EVENT-OUT-OF-ORDER` | `DEFERRED_BY_ACCEPTED_GATE` | ANL-01 consumer semantics. |
| `FS-AUDIT-MUTATION-MISMATCH` | `DEFERRED_BY_ACCEPTED_GATE` | ANL-01/DUR-02/DUR-03. |
| `FS-ANALYTICS-PRIVACY-POLICY` | `DEFERRED_BY_ACCEPTED_GATE` | ANL-01/privacy contracts. |
| `FS-DETECTOR-FALSE-POSITIVE` | `NOT_APPLICABLE` | analytics enforcement boundary, not runtime scheduler. |
| `FS-INVESTIGATION-MUTATION-ATTEMPT` | `NOT_APPLICABLE` | ADR-0006/ANL-04 least-privilege boundary. |

## 18. Required implementation evidence

A later FND-03 implementation package cannot claim `PROVEN` until exact revisions demonstrate at minimum:

1. two independent ChannelRuntimes can execute concurrently without shared mutable gameplay authority;
2. one channel remains one logical writer under multithreaded auxiliary work;
3. one InstanceRuntime owns instance-local state after handoff and source channels cannot mutate it;
4. stale ownership generations, stale local handles and stale worker results are rejected;
5. per-session CommandId order and no-double-execution survive queue pressure and eligible reconnect rules;
6. queue saturation remains bounded with named rejection/backpressure rather than memory growth;
7. a slow client cannot grow outbound memory unboundedly or stall unrelated clients/channel mutation;
8. wall-clock jumps cannot change monotonic gameplay deadlines;
9. exact `2 s`, `5 s` and `4 s` runtime boundaries execute from monotonic/server-authoritative evidence, while FND-04 owns the 15-second eligibility start semantics;
10. equal-deadline timers have reproducible ordering and stale timers cannot hit recycled entities/recovered generations;
11. delayed auxiliary completion cannot mutate after source revision/generation invalidation;
12. the writer performs no blocking remote/database/CPU wait while holding mutation authority;
13. drain/checkpoint/fence establishes an observable ordered cut and old authority stops immediately after fencing;
14. GameNode restart creates a fresh NodeId and recovered channel identity remains unchanged;
15. split-owner fault injection proves only the current generation can commit;
16. deterministic replay can reproduce a concurrency-sensitive accepted outcome from retained logical ordering evidence;
17. concrete runtime queue/bytes/in-flight limits are registered with boundary tests before implementation acceptance;
18. exact-head Tier 1 scenarios cover queue saturation, stale generation, channel drain, slow client, clock skew and recovery; applicable native-client/release tiers follow ADR-0007 when user-visible behavior exists.

## 19. Consequences

### Positive

- multithreading is compatible with deterministic authoritative ownership;
- thread scheduling and callback order cannot silently become gameplay authority;
- channel and instance recovery have explicit stale-work fences;
- FND-02 command semantics remain intact through runtime pressure/reconnect;
- wall-clock drift cannot corrupt gameplay deadlines;
- slow clients and overload cannot create unbounded memory paths;
- process placement remains an operational concern rather than a hidden state owner;
- replay/evidence can explain real accepted live ordering;
- benchmark-sensitive capacities remain evidence-driven.

### Costs

- every mutation-capable path must route through an explicit owner boundary;
- service/worker completions require identity/revision/generation metadata and revalidation;
- bounded queues require backpressure/degradation paths rather than convenient unlimited buffering;
- deterministic evidence adds runtime/test instrumentation work;
- durable operations require later coordination with DUR/ANL rather than direct callback mutation;
- recovery requires explicit generation/fence propagation across runtime and persistence/operations boundaries.

## 20. Rejected alternatives

### Concurrent direct mutation of one channel under locks

Rejected because lock correctness alone does not define one reproducible gameplay order and makes replay/fencing/stale-work behavior substantially harder to prove.

### One operating-system thread for the entire GameNode

Rejected because the server must scale across channels and auxiliary work; the accepted model is one logical writer per authoritative scope, not one process-wide execution thread.

### One fixed global tick chosen now

Rejected because no evidence establishes the correct frequency and accepted disconnect/combat policy durations are not server-loop frequencies.

### Let network arrival order define cross-client gameplay order

Rejected because arrival across threads/sockets is not a reproducible semantic contract. The authoritative owner assigns/retains the accepted execution order.

### Unbounded async task/queue growth with later shedding

Rejected because peer/load-driven memory growth is a denial-of-service and availability risk and violates the resource-limit policy.

### Drop reserved commands under pressure

Rejected because it breaks FND-02 command identity/order/retry guarantees and can create ambiguous duplicate execution.

### Apply worker results directly to shared state

Rejected because the result may be stale by generation/revision and auxiliary computation has no mutation authority.

### Persist process-monotonic timestamps for recovery

Rejected because monotonic instants are process-incarnation-local and cannot be meaningfully compared after restart.

### Make Launcher/Guardian part of the GameNode runtime contract

Rejected for FND-03. Client-side diagnostic topology is not required to establish server-authoritative execution and is separately gated by the accepted privacy/timing refinement.

## 21. Downstream ownership boundary

After FND-03 acceptance:

- **FND-04** owns account/session admission, reconnect eligibility/start state, lease renewal/takeover, credentials and session/handoff authorization;
- **DUR-01/02/03** own physical identifiers, PostgreSQL schema/transactions/checkpoint durability/item conservation/RPO/RTO;
- **ANL-01** owns event/audit schemas, durable outbox and analytics queue/publication contracts;
- **PERF-01** owns measured scheduling/capacity targets, worker counts and capacity claims;
- **OPS-CHANNEL-01** owns production process/container/orchestrator topology, registration/assignment mechanism, placement, automatic scaling and production recovery objectives;
- gameplay contracts own movement/combat/AI/interaction semantics and subsystem-specific conflict/timer rules;
- client contracts own presentation/prediction and optional diagnostics.

FND-03 acceptance authorizes those later contracts to consume this execution model. It does not itself authorize runtime implementation or production use.

## 22. Canonical concise rule

```text
one semantic channel or concrete instance
-> exactly one current logical mutation owner
-> current ownership generation fences authority
-> current NodeId/placement does not itself grant authority

network/service/worker/timer input
-> bounded typed input
-> current owner execution boundary
-> owner-assigned execution ordinal
-> revalidate identity + generation + revision
-> authoritative mutation/result
-> state revisions / protocol output / durable request as applicable

parallel work
-> immutable proposal only
-> never direct mutation
-> stale/late result rejected by owner

wall clock
-> correlation only
monotonic time
-> in-process deadlines/durations
authoritative order
-> owner generation + execution ordinal + FND-02/state revisions

queues/executors
-> always bounded
-> explicit backpressure/degradation
-> no silent authoritative loss
-> numeric internal capacities require registered evidence before implementation acceptance

fence/recovery
-> old generation stops mutation immediately
-> semantic ChannelId/InstanceId may survive
-> replacement process gets fresh NodeId
-> recovery gets newer ownership generation
-> same-GameSession resume only when FND-02/FND-04 required state is safely preserved
```
