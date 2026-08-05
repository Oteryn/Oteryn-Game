# ADR-0009: GameNode execution, capacity, deployment and channel recovery baseline

- Status: Accepted foundation direction
- Date: 2026-08-05
- Decision owners: Oteryn project
- Applies to: `blakinio/Oteryn-v2`
- Related gates: `FND-03`, `FND-04`, `DUR-02`, `PERF-01`, `OPS-CHANNEL-01`, `QA-E2E-01`

## Context

Oteryn v2 must scale one logical world through multiple gameplay channels while preserving deterministic authoritative state, anti-duplication guarantees and predictable latency. The architecture therefore needs an explicit distinction between a host, container, process, GameNode and ChannelRuntime. It also needs a durable direction for multithreading, capacity measurement, dynamic channel creation and recovery when a process or container fails.

Exact player limits, tick frequency, worker counts, checkpoint intervals, recovery objectives and orchestration products cannot be selected honestly before representative implementation and benchmark evidence exists. This ADR freezes the architectural invariants and the method by which those values will later be accepted.

## Decision

### 1. Canonical runtime terminology

The following terms are distinct:

- **Host** — a physical server or virtual machine providing CPU, memory, networking and storage access.
- **Container** — an optional operating-system isolation and packaging boundary around a process.
- **GameNode process** — one running instance of the Oteryn game-server binary.
- **GameNode** — the logical runtime identity of that process, with its own `NodeId`, lifecycle, health, capacity and assigned channel set.
- **ChannelRuntime** — one authoritative simulation of a gameplay channel identified by `ChannelId`.
- **Orchestrator** — an external deployment control plane that starts, stops and replaces GameNode processes or containers.

A GameNode is not a physical host and must not be confused with a Kubernetes Node or another orchestrator host concept. One host may run multiple GameNodes. One GameNode may run multiple ChannelRuntimes.

`NodeId` remains the stable identifier term accepted by existing architecture. `GameNode` is the unambiguous name of the game-process runtime concept identified by that `NodeId`. `FND-ID-01` may later define representation or an explicit alias, but it must not overload the identity of a physical host or orchestrator node.

### 2. Process and container model

The game server is a normal Rust executable that can run directly as a native process for development, tests and controlled operations.

The production default is:

```text
one container
└── one GameNode process
    ├── zero or more ChannelRuntime instances
    ├── asynchronous network and service I/O
    ├── simulation scheduling
    └── bounded auxiliary worker capacity
```

A container does not replace the process; it packages and isolates it. Running one channel per GameNode remains a supported configuration for exceptional load, fault isolation or testing, but it is not the default architectural requirement.

The GameNode process must not call Docker, Kubernetes or another deployment API to create replacement containers. It exposes health, readiness, capacity and lifecycle state. An external orchestrator owns process/container lifecycle.

The exact initial orchestrator product is deferred. The same binary and runtime contract must remain usable under a native process supervisor, Docker Compose, Kubernetes or another accepted control plane.

### 3. Multithreaded server with one logical writer per channel

Oteryn v2 is a multithreaded server. It is not a single-threaded process and it does not require one dedicated operating-system thread per channel.

Each ChannelRuntime has exactly one logical authoritative mutation owner. All accepted gameplay mutations for that channel are committed through that owner in an explicit reproducible order.

Parallel work is allowed for different channels and for bounded auxiliary computation such as:

- pathfinding;
- expensive AI planning;
- visibility candidate calculation;
- compression and serialization preparation;
- persistence preparation;
- asset and content loading;
- other explicitly classified CPU-bound work.

Auxiliary workers return proposals or results; they never directly mutate authoritative channel state. Each result must carry enough identity to reject stale or misrouted work, including the relevant channel identity, ownership generation, source state revision, task identity and deadline where applicable.

The ChannelRuntime accepts, transforms or rejects the returned result. A result computed for a stale generation or invalidated state revision cannot be committed merely because the worker completed successfully.

### 4. I/O, simulation and CPU work remain separated

Network, database and service I/O must not block the authoritative simulation writer. Heavy CPU work must not block asynchronous I/O workers.

`FND-03` must define the exact runtime and worker topology, but must preserve these boundaries:

- asynchronous I/O and lifecycle coordination;
- deterministic authoritative channel execution;
- bounded CPU-bound auxiliary work;
- bounded persistence and event-publication work;
- explicit cancellation, deadlines and stale-result rejection;
- no unbounded queue or hidden background executor.

The exact async runtime library, CPU-pool implementation, worker count and channel-to-worker assignment remain implementation decisions requiring evidence. Domain and simulation crates remain independent from the selected async runtime.

### 5. Multiple channels may share one GameNode

The initial deployment topology is a modular GameNode capable of hosting several ChannelRuntimes. Simulation workers may schedule several channels, provided that:

- each channel retains one logical writer;
- a busy channel cannot starve unrelated channels;
- per-channel execution time, queue age, memory and outbound pressure are measurable;
- capacity admission prevents oversubscription;
- a channel can later be assigned to a dedicated GameNode without changing gameplay semantics;
- process failure blast radius is included in placement policy.

The architecture does not assume that `maximum players per channel × number of channels` equals GameNode capacity. Shared CPU pools, I/O, memory, cache, network, persistence and world services require separate measured limits.

### 6. Capacity is measured at channel, GameNode and world levels

Three distinct limits are required:

- `max_players_per_channel`;
- `max_players_per_game_node`;
- `max_players_per_world`.

No fixed values are accepted by this ADR. `PERF-01` must establish them from reproducible benchmarks on named reference hardware and exact release artifacts.

Capacity tests must include at least:

1. distributed idle players;
2. representative movement, hunting, creature AI, combat, loot and pathfinding;
3. crowded city or other large interest sets;
4. mass combat and area effects;
5. boss or raid concentration;
6. login and reconnect storms;
7. durable item/economy transaction pressure;
8. several channels on one GameNode, including a noisy-neighbor channel;
9. checkpoint, recovery and degraded-dependency pressure;
10. long-running soak and memory-growth verification.

A capacity claim must record at least:

- exact hardware, operating system, container/process limits and artifact revisions;
- world/content/ruleset and protocol revisions;
- player and creature behavior model;
- tick or scheduling model;
- p50/p95/p99 command and simulation latency;
- queue ages and rejection/degradation counts;
- CPU, memory, network and persistence pressure;
- the first violated service objective.

The production admission limit must remain below the measured saturation point. The initial safety policy targets at least 30% capacity headroom under the worst accepted representative scenario. `PERF-01` may replace this percentage only with explicit evidence and a superseding accepted decision.

Player count alone is insufficient for scaling decisions. Runtime placement and channel-opening policy must also consider latency, queue age, CPU headroom, memory headroom, network pressure, persistence health and predicted demand.

### 7. Dynamic channel creation

A channel may be created at runtime inside an existing GameNode when that node has sufficient accepted capacity and the required revisions are already available.

The lifecycle baseline is:

```text
Requested
→ Starting
→ Warming
→ Ready
→ Open
→ Full or Degraded
→ Draining
→ Checkpointing
→ Stopped
```

Failure-related states include at least:

```text
Suspected
Recovering
RecoveryRequired
Fenced
Failed
```

A channel is routable only after readiness and revision compatibility checks pass.

When existing GameNodes cannot safely host another channel, the external orchestrator starts another GameNode process/container. That GameNode registers its identity, build and revision compatibility, health and capacity before a channel is assigned and before the Gateway routes sessions.

Channel creation and closure require hysteresis, cooldown and a stable zero-player or draining policy. The system must not repeatedly create and destroy channels around a single threshold.

### 8. No active-channel live migration in the first implementation

The first production implementation does not live-migrate an actively mutating ChannelRuntime between GameNodes.

The initial safe relocation sequence is:

```text
stop new admissions
→ drain or disconnect according to policy
→ establish a durable checkpoint
→ fence the old ownership generation
→ start the channel on the destination GameNode
→ recover and validate
→ publish readiness
→ issue fresh Game Sessions
```

Live migration and hot standby are later scalability and availability subjects. They may not weaken the one-owner invariant.

### 9. GameNode or container failure behavior

When a GameNode process or its container fails:

1. affected client network connections close or time out;
2. the World Registry marks the GameNode unhealthy or suspected;
3. the Gateway immediately stops new routing to every affected channel;
4. affected ChannelRuntimes enter recovery state;
5. the orchestrator restarts or replaces the GameNode;
6. recovery obtains a newer channel ownership generation;
7. the channel loads immutable World Bundle data plus the latest valid durable checkpoint and accepted journal/replay data;
8. validation completes before the channel becomes ready;
9. clients reconnect through the Gateway using newly issued Game Sessions;
10. the server sends a full authoritative snapshot before normal delta flow resumes.

A stale GameNode that returns after partition or delayed shutdown cannot resume authority. Every durable write and externally visible ownership claim must be fenced by the current channel/session generation so that stale writers are rejected.

### 10. Players are not silently moved to another channel after failure

A failed channel is not equivalent to another channel. Channel-local map state, creatures, combat, ground items, encounters and timers may differ.

Therefore the normal recovery policy is:

- the client enters a bounded reconnecting state;
- the player reconnects to the same `ChannelId` after safe recovery;
- the Gateway issues a fresh Game Session;
- old client deltas and commands are discarded or reconciled by explicit protocol rules;
- no invisible automatic transfer to another channel occurs;
- no automatic temple teleport or combat-state erasure occurs solely because the process failed.

If recovery exceeds the accepted reconnect grace period, the character remains protected by a recovery/lease decision until the server can prove a safe final state. Only then may a new admission or an explicitly governed channel change occur.

This prevents crash or connection failure from becoming a way to escape combat, duplicate loot, repeat rewards or bypass encounter restrictions.

### 11. Durable and transient recovery classes

Critical durable operations must be atomic, idempotent and recoverable across GameNode failure. This includes at least:

- inventory and equipment changes;
- pickup, drop and loot transfer;
- trade;
- bank, depot, market and mail operations;
- currency changes;
- one-time rewards;
- durable death consequences;
- item transformations and ownership transitions.

Such operations require accepted transaction boundaries, stable operation/command identities, generation fencing and durable audit/outbox evidence. A crash cannot leave one item authoritatively present in two locations.

Transient simulation state may use checkpoint and bounded replay policy, but the exact maximum progress loss, checkpoint interval, journal scope, RPO and RTO are deferred to `DUR-02`, `FND-03` and `OPS-CHANNEL-01`.

Checkpoint and recovery evidence cannot exist only inside an ephemeral container filesystem.

### 12. Blast radius constrains GameNode placement

If one GameNode hosts several channels, process failure affects all of them. The accepted number of channels per GameNode must therefore be constrained by both capacity and availability goals.

A host capable of running many channels is not automatically permitted to place all of them in one process. `PERF-01` and `OPS-CHANNEL-01` must balance:

- CPU and memory efficiency;
- shared immutable data and cache reuse;
- maximum affected-player percentage for one process failure;
- channel recovery concurrency;
- restart and warm-up time;
- dependency connection pressure;
- operational complexity.

A high-load or high-risk channel may be assigned as the only ChannelRuntime in a GameNode without requiring a different server implementation.

### 13. Required follow-up contracts

This ADR registers two explicit additional gates:

#### `PERF-01` — Capacity, Performance and Scalability Contract

Must define:

- reference hardware and supported deployment cells;
- latency, tick/scheduling, queue and resource service objectives;
- benchmark workloads and bot/behavior validity;
- channel, GameNode and world capacity limits;
- safety headroom and overload policy;
- regression thresholds and CI/nightly/release performance evidence;
- profiling, soak, noisy-neighbor and recovery-load requirements.

`PERF-01` is required before publishing production player-capacity claims and before Playable Alpha may claim representative-load readiness.

#### `OPS-CHANNEL-01` — GameNode Deployment and Dynamic Channel Orchestration Contract

Must define:

- process/container packaging and exact production deployment topology;
- GameNode registration, health, readiness and capacity reporting;
- channel lifecycle, placement, dynamic creation, draining and closure;
- orchestrator responsibility and least-privilege control boundary;
- ownership generation and fencing;
- restart, replacement, checkpoint, replay and reconnect sequence;
- recovery grace periods, RPO/RTO and player-visible errors;
- blast-radius placement rules;
- rollout, rollback and disaster tests.

`OPS-CHANNEL-01` is required before automatic production channel scaling and before production recovery behavior is claimed.

Existing gates retain their ownership:

- `FND-03` owns authoritative execution, scheduling, clocks, queues and internal recovery boundaries;
- `FND-04` owns Game Session, admission, reconnect and character-lease semantics;
- `DUR-02` owns durable checkpoint, journal, RPO/RTO and persistence recovery guarantees;
- `DUR-03` owns item/currency conservation and anti-duplication invariants;
- `QA-E2E-01` owns physical failure and reconnect evidence.

### 14. Mandatory future failure tests

Before the recovery behavior is considered proven, automated system tests must physically terminate or isolate a GameNode during at least:

- ordinary movement;
- active combat and combat lock;
- creature AI/pathfinding work;
- pickup and loot transfer;
- direct trade;
- durable reward processing;
- checkpoint creation;
- mass login/reconnect;
- database or publication degradation;
- several co-located channels.

Acceptance requires:

- no simultaneous old and new channel owner;
- no stale durable write;
- no item or currency duplication/loss outside the accepted transaction result;
- bounded and observable reconnect behavior;
- deterministic or explicitly bounded recovery of transient state;
- full cleanup and retained evidence.

## Consequences

### Positive

- Multithreading and scalability do not introduce concurrent uncontrolled mutation of one channel.
- GameNode, host, process, container and channel identities are unambiguous.
- Oteryn can scale channels within a process and scale processes through an external orchestrator.
- Player-capacity claims will be based on representative evidence rather than arbitrary numbers.
- Process/container failure has a defined safe recovery path.
- Recovery cannot silently become a channel-hopping or duplication mechanism.
- The same game-server binary supports multi-channel and dedicated-channel deployment.

### Costs

- Ownership generations, checkpoints, replay, leases and durable audit must exist earlier than in a traditional single-process game server.
- Several channels per process improve efficiency but increase blast radius.
- Representative performance and failure testing requires realistic simulation clients and named reference hardware.
- Dynamic scaling requires an external control plane and capacity reporting.
- Fast recovery targets may later require replicated journals or standby capacity.

## Explicitly deferred

This ADR does not select:

- exact player limits;
- exact tick frequency or scheduling quantum;
- exact worker counts or CPU affinity;
- exact async runtime or CPU-pool libraries;
- exact Docker/Kubernetes/systemd configuration;
- exact orchestrator product;
- exact checkpoint interval, RPO, RTO or reconnect grace period;
- active-channel live migration;
- hot or warm standby implementation;
- multi-node partitioning of one ChannelRuntime;
- exact database/journal technology beyond accepted PostgreSQL ownership direction.

Those values require the named follow-up contracts and measured evidence.
