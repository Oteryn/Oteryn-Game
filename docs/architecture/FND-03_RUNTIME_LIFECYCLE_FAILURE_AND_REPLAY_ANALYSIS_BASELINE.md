# FND-03 Runtime Lifecycle, Failure and Replay Analysis Baseline

- Status: Derived architecture analysis; companion input to the later complete `FND-03` contract
- Date: 2026-08-08
- Gate: `FND-03`
- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Companion: `FND-03_AUTHORITATIVE_RUNTIME_EXECUTION_ANALYSIS_BASELINE.md`
- Applies to: `NodeRuntime`, `WorldServices`, ChannelRuntime/InstanceRuntime lifecycle, timer catch-up, fairness, failure isolation, dependency loss, authoritative randomness, deterministic replay evidence, event cut points and foundation failure-scenario ownership
- Preserves: ADR-0001, ADR-0009, FND-ID-01, FND-02, multichannel scope matrix, instance runtime baseline, reconnect/disconnect baselines, foundation error vocabulary and failure catalogue
- Does not authorize: Rust runtime implementation, persistence schema, Game Session/lease implementation, production traffic, production deployment or external-repository writes

## 1. Purpose

Resolve the next technical FND-03 analysis layer after the authoritative ordering baseline without prematurely selecting executor libraries, queue sizes, worker counts, persistence technology or orchestration products.

The decisions in this document are classified as `DERIVED` unless an accepted source is explicitly named as `PROVEN`. They are intended to become inputs to the final FND-03 contract after independent review.

## 2. `NodeRuntime` responsibility boundary

### Problem

A GameNode process needs a process-level runtime owner, but that owner must not become a second gameplay authority or a replacement for explicit world/channel/instance ownership.

### Derived direction

`NodeRuntime` is the **process-incarnation supervisor and execution host** for one `NodeId`.

It should own or coordinate process-scoped concerns such as:

- process bootstrap and association with one fresh canonical `NodeId` incarnation;
- registration/lifecycle integration hooks required by later operations contracts;
- the registry of authoritative scopes currently hosted by this process;
- validation that a scope's required build/protocol/ruleset/content revisions are available before that scope can become ready;
- bounded scheduling capacity for ChannelRuntime and InstanceRuntime owners;
- bounded auxiliary CPU-work capacity;
- asynchronous transport/service/persistence adapters without making those adapters gameplay authorities;
- process-level memory/work/capacity accounting;
- health/readiness/capacity observations;
- orderly process drain/shutdown supervision;
- fault containment and escalation when a scope cannot remain safely authoritative.

`NodeRuntime` must **not**:

- mutate a ChannelRuntime or InstanceRuntime's gameplay state directly;
- infer the current world/channel from process-global mutable state;
- turn `NodeId` into ownership authority;
- become the owner of every world-shared domain merely because those domains are co-located;
- expose mutable domain objects for arbitrary cross-scope direct access.

The hosted-scope registry is keyed by explicit semantic scope identity and current ownership evidence. Conceptually:

```text
ChannelScope  = WorldId + ChannelId + ownership_generation
InstanceScope = WorldId + InstanceId + ownership_generation
```

### Multi-world process safety

Semantic correctness must not depend on `one world = one GameNode process`.

An operations policy may initially choose to place scopes from only one world on a process, but the runtime architecture must not contain a mutable process-global `current_world`, `current_ruleset`, `current_content_revision` or similar hidden singleton that would make later multi-world placement unsafe.

Every authoritative scope receives or derives an immutable, explicit world/revision context.

### Decision timing

- **Must decide now?** YES.
- **Blocked work:** GameNode composition, hosted-scope lifecycle, safe dependency injection and multichannel tests.
- **Harder later:** a process-global world singleton would recreate the legacy architecture and make placement/extraction expensive.
- **Superseding evidence:** measured evidence may constrain placement for performance, but not remove explicit scope identity/ownership.
- **Deliberately undecided:** exact NodeId generation/registration handshake, orchestrator API and runtime library.

## 3. `WorldServices` responsibility boundary

### Problem

World-shared functionality must exist without reintroducing one process-global mutable `Game` object or allowing channel-local code to become accidental authority for world-wide state.

### Derived direction

`WorldServices` is a **typed boundary/facade for explicitly world-scoped owners**, not one generic singleton and not necessarily one physical in-process component.

Each mutable world-shared domain must independently declare:

- semantic owner;
- world/character/account scope;
- consistency model;
- command/query interface;
- timeout/failure semantics;
- idempotency/fencing requirements where relevant;
- degradation policy;
- persistence authority;
- observability/audit requirements.

Examples already classified as world-shared or potentially world-shared include presence, world/guild/private communication, party membership, market, bank/depot/mail, rewards, guild state and selected event/house state. Their final semantics remain with their own gates.

### Access model

ChannelRuntime and InstanceRuntime consume world-shared capabilities through explicit typed operations. They do not receive unrestricted mutable references to world-shared state.

Conceptually:

```text
AuthoritativeScope
    -> typed WorldService command/query
    -> bounded asynchronous outcome
    -> normalized response to scope owner
    -> owner revalidates generation/revision/legality
    -> authoritative continuation
```

A co-located implementation may optimize transport internally, but it must preserve the same semantic ownership and failure model required if the service later moves to another process.

### Immutable world context is separate

Immutable/revisioned ruleset, content and World Bundle data are not mutable `WorldServices` state. Runtime scopes consume an explicit immutable revision context, while mutable world services remain separately owned.

### Deferred owners

FND-03 must not absorb:

- final Game Session/character lease authority from `FND-04`;
- final persistence transaction authority from `DUR-*`;
- final event/audit schema/outbox authority from `ANL-01`;
- final market/party/guild/social gameplay contracts.

It only defines how asynchronous world-service dependencies interact safely with authoritative runtime execution.

### Decision timing

- **Must decide now?** YES for the ownership/access shape; NO for physical service extraction/topology.
- **Blocked work:** runtime composition and cross-channel safety.
- **Harder later:** direct shared-state references create hidden coupling and prevent safe extraction/failure isolation.
- **Superseding evidence:** in-process optimization may be selected if it preserves the same typed owner boundary.

## 4. Authoritative scope responsibilities

### `ChannelRuntime`

Owns the mutable public-world simulation for exactly one `WorldId + ChannelId`, including where applicable:

- players currently assigned to that channel simulation;
- public-map mutable overlay;
- player/creature positions and local visibility;
- creatures, spawn runtime and local NPC runtime;
- channel-local combat, conditions and transient effects;
- ground items, corpses and transient local loot state;
- local speech and proximity-dependent interactions;
- channel-local timers, encounters and environmental runtime state;
- the scope's deterministic authoritative ordering and state revisions.

It does not become authoritative for world-wide market, guild, presence, reward eligibility or another shared domain simply because a local action invokes them.

### `InstanceRuntime`

Uses the same execution contract but owns exactly one `WorldId + InstanceId` isolated mutable activity scope.

Once authoritative transfer commits:

- participating characters are owned for instance-local simulation by that InstanceRuntime;
- source ChannelRuntimes no longer mutate those characters' instance-local position/combat/encounter state;
- local instance creatures, hazards, objectives, timers and transient items are isolated from other instances using the same template.

### Decision timing

- **Must decide now?** YES.
- **Blocked work:** movement/combat/instance implementation and ownership transfer contracts.
- **Harder later:** ambiguous scope ownership creates double mutation and cross-channel/instance leakage.
- **Deliberately undecided:** exact in-memory data structures and crate boundaries.

## 5. Scope lifecycle semantics

ADR-0009 already establishes the canonical lifecycle vocabulary. FND-03 should preserve it rather than invent a competing state model.

Normal states:

```text
Requested
-> Starting
-> Warming
-> Ready
-> Open
-> Full or Degraded
-> Draining
-> Checkpointing
-> Stopped
```

Failure/recovery states include:

```text
Suspected
Recovering
RecoveryRequired
Fenced
Failed
```

### `Requested`

- assignment/request exists but no gameplay authority is active locally;
- no player-visible simulation is routable.

### `Starting`

- runtime owner and bounded resources are being created;
- no gameplay admission;
- partial construction failure cleans up without publishing readiness.

### `Warming`

- required immutable revisions/data and recoverable state are loaded/validated;
- no new gameplay routing;
- expensive precomputation may occur through bounded workers but cannot publish authority by itself.

### `Recovering`

- a current ownership generation has been established for recovery work;
- checkpoint/journal/replay evidence is loaded and validated;
- clients are not routed until recovery validation completes;
- old ownership-generation work is stale.

### `Ready`

- internal runtime invariants, ownership generation, revision compatibility and required dependencies for declared readiness have passed;
- `Ready` is **not itself routing authority**;
- World Registry/Gateway/control-plane policy still decides whether sessions may be routed.

### `Open`

- the runtime may receive ordinary authorized gameplay/admission work subject to FND-04 and capacity policy.

### `Full`

- existing authoritative gameplay continues;
- new admission/placement is rejected or redirected by policy;
- fullness is not a reason to violate existing session correctness.

### `Degraded`

- explicitly named optional/affected capabilities are unavailable or limited;
- degradation must never silently weaken fencing, item/currency conservation, authentication, durable audit requirements or another safety invariant;
- risky operations whose required dependency is unavailable fail closed;
- unaffected local simulation may continue only when its owning contract explicitly permits it.

### `Draining`

- no new ordinary admissions/transfers into the scope;
- existing gameplay progresses toward a documented bounded safe boundary;
- drain cannot wait forever on unbounded queues or unreachable clients;
- control/fencing remains non-starvable.

### `Checkpointing`

- represents a lifecycle barrier around the checkpoint/recovery contract;
- FND-03 requires unambiguous ownership and safe suspension/transition semantics;
- exact durable checkpoint data/atomicity remains `DUR-02`.

### `Fenced`

- zero new authoritative gameplay commits are permitted under that old ownership authority;
- stale callbacks, timers and worker results are invalid for mutation;
- observation/cleanup may continue only where it cannot regain authority.

### `Failed`

- authoritative simulation for the scope is not usable;
- routing is stopped;
- validated recovery/replacement is required before authority returns.

### `Stopped`

- no authoritative mutation;
- local resources may be released;
- semantic ChannelId/InstanceId history remains distinct from process-local runtime object lifetime.

### `Suspected`

`Suspected` is an observation/health state, not a grant of authority. If the local runtime can no longer prove a required ownership/fence/lease condition, risky/durable mutation fails closed according to the owning contract.

### Decision timing

- **Must decide now?** YES for lifecycle semantics and authority per state; NO for exact timeout values.
- **Blocked work:** startup/readiness, drain, crash recovery and safe rollout tests.
- **Superseding evidence:** measured operations evidence may tune timeouts/transitions while preserving authority invariants.

## 6. Timer catch-up and coalescing taxonomy

### Problem

A runtime can stall, pause, recover from load or resume after scheduling delay. Blindly executing every missed timer callback may create an unbounded catch-up storm, while silently dropping all missed timers can corrupt gameplay semantics.

### Required rule

Every timer family whose repeated occurrence matters must declare a catch-up policy. There is no implicit global default that replays every missed tick.

Allowed conceptual policies:

### `DEADLINE_STATE`

The rule is represented by an absolute monotonic deadline/current state rather than repeated callbacks.

Examples: cooldown expiry, temporary permission/protection end, many expiration checks.

On delayed processing the runtime evaluates current deadline state once; it does not replay synthetic historical ticks.

### `RUN_EACH_BOUNDED`

Every occurrence is semantically required, but only a bounded number may execute per owner turn. Remaining overdue work carries forward while other scopes retain scheduling opportunity.

This policy requires an explicit hard backlog/work bound before production use.

### `COALESCE_ELAPSED`

Multiple overdue periods may be collapsed into one deterministic calculation based on elapsed monotonic duration only when the owning gameplay rule proves this is semantically equivalent.

### `SKIP_TO_LATEST`

Intermediate occurrences may be discarded and only the latest/current calculation executed. Suitable only for explicitly non-semantic maintenance/AI-think-like work where skipping cannot alter required gameplay outcomes.

### `EXPIRE_OR_CANCEL`

The work becomes invalid after its deadline or state-generation change and is dropped as a named cancellation/expiry outcome.

### Safety rules

- periodic combat/damage/healing rules must explicitly select their policy in the owning combat/ability contract;
- no zero-delay timer may recursively generate unbounded same-turn work;
- catch-up work participates in the same per-scope work budget/fairness controls;
- skipped/coalesced work must be observable enough for debugging where it affects player-visible latency;
- exact numeric catch-up limits remain evidence-driven.

### Decision timing

- **Must decide now?** YES for the taxonomy and explicit-policy requirement; NO for each gameplay timer's final policy and numeric bounds.
- **Blocked work:** scheduler implementation and deterministic stall tests.
- **Harder later:** implicit callback semantics create catch-up storms and inconsistent gameplay after lag.

## 7. Fair scheduling across authoritative scopes

### Invariant

One busy ChannelRuntime or InstanceRuntime may not indefinitely starve unrelated authoritative scopes hosted on the same GameNode.

### Derived direction

`NodeRuntime` scheduling should use a bounded work budget/quantum per runnable scope.

Semantic ordering remains local to each scope; delaying a scope changes when it progresses in real time but must not reorder its already accepted authoritative input stream internally.

Prefer deterministic work units for semantic budgets, such as bounded counts/classes of owner inputs/commits, rather than making correctness depend on variable wall-clock CPU duration. Wall-time watchdogs remain useful operational safety signals but should not define gameplay ordering.

The normal scheduling class should provide bounded fairness among runnable scopes. The dedicated control/fencing lane remains processed before ordinary scope work and must not be abused for gameplay priority.

The architecture should preserve later ability to:

- assign a high-load scope to a dedicated worker/GameNode;
- weight scopes for measured load;
- adapt worker placement;
- use work stealing or another executor implementation.

None of those mechanisms may permit simultaneous authoritative mutation of one scope.

### Decision timing

- **Must decide now?** YES for non-starvation/bounded-yield invariant; NO for exact algorithm/weights/quantum values.
- **Blocked work:** multi-channel scheduler and noisy-neighbor tests.
- **Superseding evidence:** benchmarks may replace scheduler implementation while preserving bounded fairness.

## 8. Panic and internal-invariant failure isolation

### Problem

Continuing mutation after an unexpected authoritative runtime failure can be more dangerous than temporarily losing availability because local state may be inconsistent.

### Derived direction

An unexpected panic or internal-invariant violation inside an authoritative scope is **fail-stop for that scope** unless the runtime can prove the failure occurred entirely outside authoritative state.

For a scope-level authoritative failure:

1. stop ordinary authoritative commits for that scope;
2. transition to `Fenced`/`Failed` or an equivalent safe lifecycle path;
3. stop new routing/admission to the scope;
4. invalidate/cancel in-flight auxiliary proposals and stale callbacks;
5. retain bounded diagnostic evidence;
6. require validated recovery with a current ownership generation before authority resumes.

If the process cannot prove that state corruption is confined to one scope — for example the failure may affect a shared allocator/unsafe component/shared mutable authority — the safer policy is to fail-stop the entire GameNode process and let the external orchestrator replace it.

A `catch_unwind` implementation, Rust panic strategy, process supervisor and crash-report mechanism are implementation details. The semantic contract is **never catch an unexpected authoritative failure and blindly continue ordinary mutation**.

Current workspace policy forbids `unsafe` and production code panics through lints; FND-03 should preserve that defensive posture but still define failure behavior for bugs, dependency panics and invariant violations.

### Decision timing

- **Must decide now?** YES.
- **Blocked work:** failure containment and recovery testing.
- **Harder later:** attempting in-place continuation can normalize corrupted state and make forensic recovery impossible.
- **Superseding evidence:** a proven isolation mechanism may allow narrower containment, not weaker correctness.

## 9. Required dependency delay/loss during authoritative operations

### Core rule

The authoritative owner must not block a simulation executor thread on network/database/service I/O while holding an implicit mutable execution critical section.

When an operation requires an external/durable result:

```text
validate local preconditions
-> create explicit pending operation state
-> issue bounded asynchronous request
-> yield authoritative owner
-> normalized response/timeout/cancel returns
-> revalidate scope/session/generation/revision/operation state
-> commit or fail deterministically
```

### Before authoritative/durable commit

If a required dependency is unavailable before the mutation can be committed safely:

- return/move toward `DEPENDENCY_UNAVAILABLE`, `TIMEOUT` or another contract-owned bounded error;
- do not claim success;
- do not leave partial authoritative mutation hidden in memory;
- unaffected gameplay may continue only according to the accepted degradation policy.

### Ambiguous request outcome

If a request may have reached a durable/service owner but its response is lost, retry/recovery uses the same stable command/operation identity or an authoritative reconciliation read. It must not invent a new logical operation merely because the caller timed out.

Exact cross-system `OperationId`, transaction identity and outbox mechanics remain `DUR-*`/`ANL-*` work.

### Decision timing

- **Must decide now?** YES for async pending/revalidation semantics; NO for specific persistence/service client libraries.
- **Blocked work:** persistence/world-service integration and dependency-loss tests.
- **Harder later:** blocking I/O in the writer or blind retry creates latency stalls and duplicate effects.

## 10. Authoritative randomness

### Problem

Randomness used by combat, loot, AI or world simulation can destroy deterministic replay if sourced from process-global/thread-local nondeterministic RNG without explicit ownership.

### Derived direction

Authoritative gameplay randomness uses explicit deterministic RNG streams owned by a named authoritative scope/domain.

Requirements:

- RNG seed/source is server-controlled, never player authority;
- stream identity/state or equivalent deterministic evidence is available to tests/replay;
- thread scheduling must not change the sequence of authoritative random decisions;
- worker tasks that need random choices receive explicit deterministic input/seed or return a proposal whose final random decision is made by the authoritative owner;
- moving a scope to another worker thread must not change gameplay randomness;
- randomness used for security credentials/tokens/nonces is completely separate and uses cryptographically secure security randomness under the owning security/FND-04 contract;
- deterministic gameplay RNG must never generate credentials or security secrets.

The exact RNG algorithm/stream representation is not selected here; final choice should have stable documented behavior and deterministic cross-platform fixtures.

### Decision timing

- **Must decide now?** YES for explicit deterministic gameplay-randomness ownership; NO for algorithm choice.
- **Blocked work:** deterministic simulation/replay, combat/loot/AI implementation.
- **Harder later:** hidden thread-local randomness makes replay and migration fragile.
- **Superseding evidence:** algorithm may change only through versioned ruleset/content/migration policy with deterministic fixtures.

## 11. Minimum deterministic replay evidence

### Purpose

FND-03 needs enough replay evidence to reproduce authoritative runtime divergence without prematurely defining the complete analytics/event schema owned by `ANL-01`.

A test/replay attempt should be able to record or reconstruct conceptually:

```text
scope identity
scope ownership generation
NodeId/process-incarnation evidence
server/build/protocol/ruleset/content/WorldBundle revisions
initial checkpoint/state identity or hash
injected monotonic clock values/advances
authoritative gameplay RNG seed/stream evidence
RuntimeInputOrdinal
input source class + bounded source identity
normalized command/timer/worker/service/control input
RuntimeCommitOrdinal
relevant state-domain revisions/result
bounded state/result hash or semantic probe
failure/cancellation/degradation markers
```

### Boundaries

- replay evidence is bounded and redacted;
- it contains no credentials, login tickets or reusable secrets;
- a production-wide verbose replay log is not authorized by this analysis;
- deterministic replay may use synthetic/test-only state hashes/probes that are excluded from production defaults;
- external service responses are replayed as recorded normalized inputs in an isolated environment rather than re-executing real payments/market/persistence side effects;
- first divergence is preserved rather than hidden by retries.

### Replay result

Given the same accepted initial state, revisions, RNG state, injected clock and normalized input order, replay should produce matching semantic results/state hashes until the first divergence.

### Decision timing

- **Must decide now?** YES for minimum replayability requirements; NO for file format/storage/retention.
- **Blocked work:** deterministic runtime tests and QA fault reproduction.
- **Harder later:** adding replay after nondeterministic hidden inputs exist may require pervasive instrumentation.

## 12. Event-emission cut points

FND-03 must separate authoritative state transition from downstream observability/analytics side effects.

### Transient/local authoritative outcome

For a purely in-memory authoritative transition whose final semantics do not require a durable transaction:

```text
owner commits authoritative local state
-> advances relevant state revision/order evidence
-> prepares client result/delta and event records
-> asynchronous consumers receive derived output
```

### Durable operation

If success semantically requires durable state/audit evidence:

```text
owner validates intent
-> durable operation boundary commits required mutation + required evidence atomically/idempotently
-> authoritative success becomes final
-> client success / durable-success event may be emitted
```

An attempt/start telemetry event may be emitted earlier only if it is clearly not represented as committed success.

### Failure policy

- best-effort telemetry enqueue failure does not roll back gameplay and is counted as evidence loss;
- required durable audit/outbox evidence is not silently dropped; the owning durable mutation fails closed or remains unresolved/degraded according to `DUR-*`/`ANL-01`;
- analytics/investigation output cannot synchronously call back to mutate the same gameplay transition;
- ordering/correlation identity remains owned by `ANL-01`/`DUR-*` where durable.

### Decision timing

- **Must decide now?** YES for semantic cut points/fail-open versus fail-closed classes; NO for event broker/outbox implementation.
- **Blocked work:** runtime event integration, anti-duplication evidence and analytics consumers.

## 13. Foundation failure-scenario ownership mapping

The complete FND-03 contract should classify every foundation scenario. The following mapping is the derived minimum for scenarios materially owned by runtime execution.

| Scenario | FND-03 minimum outcome | Later owner interaction |
|---|---|---|
| `FS-POSTGRES-UNAVAILABLE` | No blocking DB call inside writer; scope enters explicit degraded behavior; no risky/unfenced durable success | `DUR-02`, `FND-04` finalize persistence/admission policy |
| `FS-LEASE-RENEW-TIMEOUT` | Once current lease/fence cannot be proven, stale owner stops durable authoritative writes | `FND-04` owns lease state machine |
| `FS-STALE-GENERATION` | Reject before mutation; stale worker/scope/session evidence cannot regain authority | `FND-04`/`DUR-*` own specific fences |
| `FS-CHANNEL-SPLIT-OWNER` | Local generation checks plus external durable fencing prevent dual authoritative commits | `OPS-CHANNEL-01`, `DUR-*` finalize control-plane/durable fencing |
| `FS-CHANNEL-DRAIN` | No new admissions; bounded progress toward named safe barrier; control lane remains live | `FND-04`/`OPS-CHANNEL-01` own admission/orchestration details |
| `FS-QUEUE-SATURATION` | Bounded queue; explicit `CAPACITY_EXCEEDED`/degradation before unsafe acceptance; no silent loss/unbounded growth | resource registry owns accepted hard limits |
| `FS-SLOW-CLIENT` | Bounded outbound memory; coalesce/rebase/resync or disconnect; authoritative simulation is not blocked | FND-02 owns reconciliation wire semantics |
| `FS-CLOCK-SKEW` | Monotonic gameplay deadlines remain correct despite wall-clock changes | security/admission contracts own signed timestamp skew policy |
| `FS-REVISION-MISMATCH` | Scope cannot become `Ready/Open` for incompatible revisions; no mixed authoritative state | content/protocol/operations contracts own exact compatibility matrices |
| `FS-SNAPSHOT-DELTA-MISMATCH` | Runtime supplies consistent state revisions/snapshot source; FND-02 deterministic resync handles client mismatch | FND-02 |
| `FS-ANALYTICS-TELEMETRY-OVERFLOW` | Drop only best-effort telemetry under bounded policy; loss counted; gameplay continues | `ANL-01/02/03` |
| `FS-AUDIT-OUTBOX-BACKLOG` | Required durable audit is never silently discarded; risky durable operation follows fail-closed/degraded owning transaction policy | `ANL-01`, `DUR-*` |
| `FS-AUDIT-MUTATION-MISMATCH` | Runtime must not report clean committed success when required mutation/evidence boundary disagrees | `ANL-01`, `DUR-*` |
| `FS-EVENT-DUPLICATE-DELIVERY` | Runtime gameplay mutation is not replayed by analytics re-delivery | `ANL-01` consumer idempotency |
| `FS-EVENT-OUT-OF-ORDER` | Analytics reorder does not alter authoritative gameplay state | `ANL-01` |

Scenarios whose decisive semantics belong to protocol, authentication, key rotation, content bundle validation or client migration remain `DEFERRED_BY_ACCEPTED_GATE`/`NOT_APPLICABLE` for the final FND-03 matrix with a named owner; they must not be silently redefined in FND-03.

## 14. Resource-limit decisions still requiring evidence

FND-03 must eventually register hard maxima for externally or internally amplification-prone resources whose absence would permit unbounded memory/CPU growth.

Likely categories include:

- per-scope ordinary gameplay ingress backlog;
- per-scope control-lane backlog;
- active timers and overdue catch-up backlog;
- auxiliary work submissions/results;
- service-response backlog;
- outbound buffered bytes/messages per session;
- hosted scopes per GameNode deployment cell;
- maximum bounded work per owner turn/catch-up turn;
- deterministic replay/test artifact bounds where production/test input can be attacker-controlled.

This analysis **does not choose numbers**. Numeric values require either protocol-owned fixed constraints, representative benchmark evidence, explicit safety analysis or a bounded implementation spike. Any configurable value retains an absolute hard maximum.

## 15. Remaining FND-03 questions after this analysis

The following still require final contract treatment, but not necessarily new owner product decisions:

1. whether `RuntimeInputOrdinal` and `RuntimeCommitOrdinal` remain two physical counters or collapse after proving semantic equivalence for the implementation;
2. exact runtime-local type widths/rollover/exhaustion behavior;
3. exact lifecycle transition table and permitted commands per state;
4. exact deterministic arbitration between already-ready ordinary input classes inside one owner turn;
5. exact fairness/work-budget and timer catch-up numeric limits from benchmark/safety evidence;
6. exact GameNode bootstrap/registration failure semantics where registry/control-plane availability is required;
7. exact replay evidence file/schema placement and retention for QA;
8. final per-scenario classification table for all entries in `FOUNDATION_FAILURE_SCENARIOS.md`;
9. the boundary at which FND-03 hands final durable commit semantics to `DUR-02`/`ANL-01`;
10. verification that no FND-03 contract clause accidentally pre-empts FND-04 admission/lease state-machine authority.

These should be resolved in the complete FND-03 contract or explicitly deferred with named evidence/owner.

## 16. Non-authorization

This document does not authorize:

- a server runtime crate or implementation;
- Tokio or another executor as the server runtime choice;
- database/persistence/outbox implementation;
- Game Session/admission/lease implementation;
- concrete queue sizes/tick rates/worker counts;
- production runtime activation;
- live GameNode/channel scaling;
- production telemetry collection;
- external-repository writes.

It is a bounded architecture analysis input only.
