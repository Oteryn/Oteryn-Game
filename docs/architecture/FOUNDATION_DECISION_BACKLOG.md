# Oteryn v2 Foundation Decision Backlog

- Status: Active architecture backlog
- Date: 2026-08-07
- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Canonical programme task: `docs/agents/tasks/active/OTV2-20260805-foundation-preimplementation-contracts.md`
- Global decision register: `docs/architecture/GLOBAL_ARCHITECTURE_DECISION_REGISTER.md`
- Current execution status: `docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md`
- Coordinator prompt: `docs/agents/prompts/OTV2_GLOBAL_ARCHITECTURE_DECISION_COORDINATOR.md`

## Purpose

Record the remaining architecture decisions in one durable, ordered register. This file distinguishes decisions that block root-workspace bootstrap, decisions that gate specific implementation layers, decisions required before durable gameplay, and subjects deliberately deferred.

The complete later project horizon is maintained in `GLOBAL_ARCHITECTURE_DECISION_REGISTER.md`. That register preserves gameplay, client, content, operations and product domains without forcing them to be designed before they block the current stage.

Chat history is not authoritative. Accepted ADRs/contracts, this backlog, the global register, the current-status overlay, the active programme task and live PR/CI state are the continuation sources.

## Stable gate identifiers

Use these identifiers in tasks, PRs, prompts and cross-repository coordination. Section numbers or stage letters are presentation only and must not replace the stable ID.

- `FND-01` — Workspace, Dependency and Existing-Rust Migration Contract.
- `VSL-02` — Exact Rust Client Migration and Cutover Contract.
- `FND-ID-01` — Foundation Identifier Vocabulary.
- `FND-02` — `protocol-oteryn` v1 Contract.
- `FND-03` — Runtime Execution Contract.
- `FND-04` — Identity, Game Session, Admission and Character Lease Contract.
- `GAME-VISION-01` — Product Vision, Parity Scope and World Profile Contract.
- `DUR-01` — Durable Identifier Representation Contract.
- `DUR-02` — Persistence v1 Contract.
- `DUR-03` — Item Transaction and Anti-Duplication Contract.
- `DUR-04` — Content, World Detail and Scripting Contract.
- `VSL-01` — Foundation Vertical-Slice Programme.
- `QA-E2E-01` — Native End-to-End Test Platform Contract.
- `PERF-01` — Capacity, Performance and Scalability Contract.
- `OPS-CHANNEL-01` — GameNode Deployment and Dynamic Channel Orchestration Contract.
- `ANL-01` — Game Event and Audit Foundation Contract.
- `ANL-02` — Gameplay, Balance and World Analytics Contract.
- `ANL-03` — Economy Integrity and Security Analytics Contract.
- `ANL-04` — Read-Only Investigation and AI Contract.

## Already accepted

The following are no longer open questions:

1. **Native target stack**
   - authoritative game server in Rust;
   - native game client in Rust;
   - one project-owned gameplay protocol: `protocol-oteryn`.

2. **Repository ownership and migration order**
   - the existing Rust client moved from `blakinio/otclient/oteryn-client` into `blakinio/Oteryn-v2`;
   - client, server, shared Rust types and `protocol-oteryn` have one canonical repository/workspace boundary;
   - `blakinio/otclient` is migration/history evidence and its source marker is complete;
   - `protocol-canary` is not part of the target runtime;
   - `FND-01` audited and classified the exact source workspace;
   - `VSL-02` and the controlled client migration/cutover are complete;
   - PR #50 delivered the canonical destination workspace as `78988f72a80cc904aa9176ae850c50d4efa0b0f0`;
   - `blakinio/otclient#274` and #275 completed the moved/non-canonical source marker and lifecycle closeout.

3. **Identity and Game Gateway boundary**
   - Platform Identity remains the reusable-credential, OAuth/PKCE, MFA and account-security authority;
   - Oteryn v2 does not introduce a second classic login server or password/OAuth authority;
   - Game Gateway remains in `blakinio/Oteryn-Platform`;
   - the initial Game Gateway remains implemented in Go;
   - the Rust game server accepts bounded pre-admission material and owns gameplay admission/runtime state;
   - canonical `GameSessionId` is game-domain-owned and issued only after successful authoritative admission.

4. **Database direction**
   - PostgreSQL is the target relational database technology;
   - Platform and game use separate logical databases, owners, runtime credentials and migration histories;
   - the initial physical cluster may be shared, but `oteryn_platform` and `oteryn_game` remain separate security and ownership boundaries;
   - Redis is non-authoritative.

5. **Multichannel foundation**
   - one logical world may expose multiple gameplay channels;
   - one logical mutation owner exists per channel;
   - character persistence and selected world services are shared across channels;
   - channel-local simulation state is isolated;
   - one character has at most one active authoritative session and durable writes require generation fencing.

6. **Native world, content and editor direction**
   - Oteryn defines a project-owned world/content format from zero rather than using or extending OTBM as its canonical model;
   - the editable World Project, canonical world/content model and deterministic runtime World Bundle are separate representations;
   - OTBM, OTB, appearances, sprites and legacy XML are bounded conversion inputs, with constrained export only where semantics permit;
   - one integrated Oteryn Studio edits maps, areas, assets, sprites/appearances, items and related content through one Content Registry;
   - stable namespaced content keys are canonical; legacy and compact runtime numeric IDs are mappings;
   - logical `Area`/`Subarea` geography is independent of technical `Region`/`Chunk` partitioning;
   - dynamic encounters use validated `EncounterZone`, `RaidCell` and `RaidAnchor` scopes rather than treating an oversized subarea as the execution boundary;
   - authored static world definitions remain separate from authoritative dynamic PostgreSQL state;
   - Remere's Map Editor and Beats Assets Editor are reference/migration tools, not target runtime dependencies or automatically reusable code bases.

7. **Game Intelligence, analytics and audit direction**
   - Oteryn Game Intelligence is a first-class subsystem rather than a late dashboard add-on;
   - operational observability, best-effort gameplay telemetry and durable economy/security audit are distinct data classes;
   - gameplay/balance, world/content, economy/item integrity and security analytics are separate consumers of one versioned event foundation;
   - item/currency/security evidence is emitted atomically with authoritative transactions through an accepted outbox/audit boundary;
   - anti-duplication prevention remains authoritative in `DUR-03`; analytics supplies independent evidence and investigation;
   - AI and investigation remain read-only, external to gameplay and unable to ban, mutate, roll back or deploy autonomously;
   - privacy requires pseudonymous analytics identity, role-separated access and explicit retention, not name-only suppression.

8. **Native E2E platform direction**
   - one shared manifest-driven E2E platform owns lifecycle, evidence, timeouts and cleanup;
   - Tier 1 headless system E2E is the broad deterministic mechanism for Platform/Gateway/protocol/server/PostgreSQL coverage;
   - Tier 2 instrumented native-client E2E proves real client networking, input, reconciliation, UI and rendering without bypassing server authority;
   - Tier 3 production-binary smoke E2E proves exact release-candidate artifacts without the in-process test adapter;
   - hidden retry-until-green is forbidden and every counted physical attempt remains visible;
   - exact revisions, deterministic controls, first-divergence evidence and cleanup certification are mandatory;
   - `QA-E2E-01` blocks completion of `VSL-01`, but does not block architecture discovery.

9. **Legacy protocol migration disposition**
   - `protocol-canary` is fixed as `REFERENCE_ONLY` migration evidence;
   - it is excluded from the production workspace dependency graph, binaries, negotiation, fallback and translation paths;
   - `protocol-oteryn` remains an independently designed native protocol.

10. **GameNode execution, capacity, deployment and recovery direction**
   - `GameNode` is the logical runtime identity of one game-server process identified by `NodeId`, distinct from the host, container and orchestrator node;
   - production defaults to one GameNode process per container, while one GameNode may host several ChannelRuntimes when measured capacity and blast-radius policy allow it;
   - the process is multithreaded, but each channel retains one logical authoritative writer and stale auxiliary results are rejected by identity, generation and revision;
   - capacity is measured separately per channel, GameNode and logical world; no fixed player limits are accepted without representative benchmarks;
   - the initial production headroom target is at least 30% below measured saturation until `PERF-01` provides superseding evidence;
   - existing GameNodes may start channels within accepted capacity, while an external orchestrator starts, replaces and stops processes or containers;
   - initial failure recovery uses fencing, checkpoint plus bounded replay, fresh Game Sessions and a full snapshot, without silently moving players to another channel.

11. **Reference and evolved world product direction**
   - Global Tibia behavioral parity is the initial reference target; controlled Oteryn improvements and original expansion may follow.
   - reference and evolved worlds are versioned product/ruleset/content profiles over one canonical engine, native client and `protocol-oteryn`; they are not forks.
   - reusable Platform identity may be shared, while characters, progression, items, currency, houses, market, guilds and rankings remain world-scoped by default.
   - reference and evolved profiles use distinct logical worlds; every channel of one `WorldId` inherits the same profile family and compatible revision.
   - cross-profile gameplay-value transfer is forbidden until a dedicated contract proves conservation, balance and migration safety.
   - `GAME-VISION-01` must define measurable parity scope, first-launch strategy and initial intentional differences before broad gameplay/content production.

12. **Canonical workspace and pre-native client cutover state**
   - `FND-01` and `VSL-02` are accepted, applied and archived;
   - PR #50 delivered the accepted 19-member canonical Rust workspace as `78988f72a80cc904aa9176ae850c50d4efa0b0f0`;
   - source-only closeout is complete in `blakinio/otclient`;
   - the client remains in ADR-0011 `pre-native-protocol` and fails closed before gameplay credential consumption, routing or transport;
   - the production graph contains neither `protocol-canary` nor a speculative production `protocol-oteryn` adapter.

13. **Foundation identifier contract**
   - `FND-ID-01` is accepted and canonical through `FND-ID-01_FOUNDATION_IDENTIFIER_CONTRACT.md` and merge `2c584543cd1e3758958755478a6cc6ed3d39a8a9`;
   - the minimum catalogue is `AccountId`, `CharacterId`, `WorldId`, `ChannelId`, `NodeId`, `InstanceId`, `PartyId`, `GameSessionId` plus the accepted conditional `HandoffId`;
   - `AdmissionId` and `CharacterLeaseId` are not created by the foundation contract;
   - `CommandId`/wire sequencing remain `FND-02`, runtime-local handles remain `FND-03`, admission/session/lease mechanics remain `FND-04`, and later durable/analytics identity catalogues remain with `DUR-*`/`ANL-*`.

Canonical decisions: ADR-0001 through ADR-0011 plus accepted dedicated foundation contracts/baselines.

## Progressive implementation policy

Architecture contracts remain mandatory for public and durable behavior, but compile-time and prototype evidence are part of architecture discovery.

### Completed foundation migration sequence

- `FND-01` was accepted and archived with the exact source inventory, migration dispositions, toolchain, target matrix and machine-enforced workspace boundaries.
- `VSL-02` was accepted and archived with exact cutover/provenance/rollback contracts.
- the atomic destination migration/workspace PR #50 was squash-merged as `78988f72a80cc904aa9176ae850c50d4efa0b0f0`;
- the source-only historical marker and lifecycle archive are complete in `blakinio/otclient`;
- `FND-ID-01` was accepted and squash-merged as `2c584543cd1e3758958755478a6cc6ed3d39a8a9`;
- its task lifecycle was archived on `main` by PR #87.

### Current foundation transition

The migration/cutover and foundation identifier gate are complete. The current ordered architecture transition is to `FND-02` after the coordination-register reconciliation tracked by issue #86 is merged and verified.

No `FND-02` protocol/runtime implementation is authorized by this backlog reconciliation.

### Gate 3 — layer-specific implementation

- `FND-ID-01` is complete and its semantic identities are mandatory inputs to later layers.
- `FND-02` gates canonical `protocol-oteryn` schemas/codecs, production framing, command identity/sequencing and compatibility claims.
- `FND-03` gates authoritative runtime scheduling, ordering, runtime-local handles, lifecycle and recovery behavior.
- `FND-04` gates production admission, Game Session validation, reconnect/takeover and character lease behavior.
- `GAME-VISION-01` gates broad gameplay/content production claims.
- `PERF-01` gates published player-capacity claims and representative-load readiness.
- `OPS-CHANNEL-01` gates automatic production channel scaling and claimed production GameNode/channel recovery behavior.
- `DUR-01` through `DUR-03` gate authoritative durable character, item and currency mutation.
- `DUR-04` gates broad content import and durable scripting behavior.
- `QA-E2E-01` gates the shared three-tier E2E implementation and named evidence required for `VSL-01` completion.
- `VSL-01` gates the claim that the first native gameplay slice is complete.
- `ANL-01` gates final transactional event/outbox/audit boundaries used by `DUR-02` and `DUR-03`.
- `ANL-02` and `ANL-03` gate production-grade balance/world and economy/security analytics claims.
- `ANL-04` gates read-only AI investigation and is not required for the foundation vertical slice.

A spike is never a substitute for an accepted contract. It must be reversible, isolated from public contracts, excluded from production defaults and either removed or deliberately promoted by a later accepted task.

## Foundation contracts

### `FND-01` — Workspace, Dependency and Existing-Rust Migration Contract

Status: **ACCEPTED AND APPLIED**. The retained detail below is the historical contract baseline implemented by the atomic destination cutover.

This contract blocks creation of a competing destination workspace and is the mandatory migration audit for the existing Rust implementation in `blakinio/otclient/oteryn-client`. It must not design a duplicate workspace in isolation.

Decide:

- exact minimal initial workspace members and crate/application names;
- an exact source-SHA inventory of existing Rust crates, public contracts, consumers, tests and dependency edges;
- one migration disposition per existing crate or subsystem: `MIGRATE_AS_IS`, `MIGRATE_AND_RENAME`, `MERGE`, `SPLIT`, `REWRITE`, `REFERENCE_ONLY` or `DROP`;
- legal dependency directions and forbidden edges;
- ownership of shared identifiers, domain contracts, protocol schemas and test fixtures;
- feature policy and dependency review rules;
- target Rust edition, resolver, pinned toolchain, `rust-version`, root lockfile and `--locked` policy;
- inheritance policy for workspace package metadata, dependencies and lints;
- initial CI targets and exact target triples, at minimum Linux server and Windows client;
- a product-realistic feature/target matrix; `--all-features` is supplemental evidence and must not be the sole proof;
- whether optional client-only and server-only dependencies are isolated cleanly;
- a retained machine-readable `workspace-boundaries.toml` contract and executable `cargo metadata --locked` checks that enforce the accepted dependency graph;
- canonical ownership and required locations for the cross-repository contract lock, resource-limit registry, error vocabulary and failure-scenario catalogue;
- criteria for adding, splitting or merging crates later;
- which destination members must exist during migration and which must wait for immediate consumers.

The following is a **capability horizon**, not an accepted initial workspace and not a checklist to create empty crates:

```text
apps/client
apps/oteryn-studio
services/game-server
crates/foundation
crates/domain-core
crates/protocol-core
crates/protocol-oteryn
crates/transport
crates/simulation-core
crates/world-runtime
crates/world-schema
crates/world-project
crates/world-bundle
crates/world-compiler
crates/session-lease
crates/persistence-core
crates/persistence-postgres
crates/platform-contracts
crates/platform-client
crates/content-types
crates/content-registry
crates/content-runtime
crates/scripting
crates/test-support
```

An initial member requires an immediate named consumer and observable acceptance. Empty layering crates, speculative abstractions and convenience cycles are prohibited.

Mandatory direction: domain and simulation crates do not depend on Tokio, TCP, TLS, HTTP, SQL, PostgreSQL, Platform APIs, renderer state or UI widgets. World/content schema crates do not depend on Tauri, editor UI or renderer implementation. These boundaries must become machine-checked after the destination workspace exists.

`FND-01` is complete and no longer blocks later foundation architecture work.

### `VSL-02` — Exact Rust Client Migration and Cutover Contract

Status: **ACCEPTED AND COMPLETE**.

The destination migration and source-only historical/non-canonical closeout are complete. The historical contract remains evidence for provenance/cutover rules but is not an active blocker.

### `FND-ID-01` — Foundation Identifier Vocabulary

Status: **ACCEPTED AND MERGED**.

Canonical source: `docs/architecture/FND-ID-01_FOUNDATION_IDENTIFIER_CONTRACT.md`.

The foundation catalogue is deliberately minimal:

- `AccountId`;
- `CharacterId`;
- `WorldId`;
- `ChannelId` scoped by `WorldId`;
- `NodeId` for one GameNode process incarnation;
- `InstanceId` scoped by `WorldId`;
- `PartyId` scoped by `WorldId`;
- `GameSessionId`;
- conditional `HandoffId` scoped by `WorldId` for one bounded authoritative ownership transition.

The contract freezes semantic owner/issuer/scope/lifecycle/visibility and strong-typing rules required by downstream foundation work. It does not become a whole-game identifier registry.

Explicit downstream ownership:

- `FND-02`: `CommandId`, protocol sequencing/acknowledgement, wire/IDL/byte order and compact session-handle encoding;
- `FND-03`: runtime-local entity/worker/task/generational handles and runtime ownership-generation mechanics;
- `FND-04`: admission/session/lease/reconnect/takeover state machines and any evidence-based later request for `AdmissionId` or `CharacterLeaseId`;
- `DUR-*`: physical database representations, item/economy/durable-domain identifiers and persistence mechanics;
- `ANL-*`: event/operation/transaction/correlation/causation/pseudonymous analytics identity catalogues and their privacy/durability rules.

`ProtocolRevision`, `RulesetRevision`, `ContentRevision`, `connection_generation` and ownership generations are revisions/fences, not entity identities in `FND-ID-01`.

### `FND-02` — `protocol-oteryn` v1 Contract

Decide:

- whether the exact latest merged Platform native contract is adopted, revised or explicitly superseded for the Rust server target; mutable PR heads are evidence only and cannot be canonical;
- the machine-readable `docs/contracts/CROSS_REPOSITORY_CONTRACT_LOCK.json` entry with merged commit, schema revision/hash, producer/consumer status and rollout order;
- initial transport;
- TLS model, certificate/key ownership and ALPN;
- frame header, endianness, message namespace and hard size limits;
- stable schema/IDL and canonical schema owner;
- protocol revision versus ruleset/content revision;
- capability negotiation and downgrade prevention;
- `CommandId` representation/scope plus sequencing, acknowledgement, replay protection and idempotency;
- snapshot, delta and reconciliation contracts;
- reconnect/resume semantics and compact session mapping where accepted;
- compression rules and bounded allocation;
- entries in the resource-limit registry for every externally controlled size/depth/count;
- stable machine error categories and mapping into the foundation error vocabulary;
- golden fixtures shared by client and server.

The merged Platform native contract is reconciliation input only according to `FND-02_PLATFORM_PROTOCOL_RECONCILIATION_OWNER_BASELINE.md`. Do not create a second silent native protocol or silently adopt the old external wire tuple. Rust memory layout or unstable serializer output cannot be the public wire contract.

### `FND-03` — Runtime Execution Contract

Decide:

- `NodeRuntime`, `WorldServices`, `ChannelRuntime` and `InstanceRuntime` responsibilities;
- modular-monolith initial deployment topology;
- runtime-local entity/worker/task/generational handle semantics and invalidation boundaries;
- fixed or variable tick model and initial tick frequency;
- monotonic versus wall-clock ownership, clock-skew tolerance, tick-to-time conversion and injected deterministic test clocks;
- authoritative command ordering;
- timer and scheduler semantics;
- bounded inbound/outbound/work queues;
- overload and backpressure behaviour;
- parallel pathfinding, AI and content work and safe return to the logical writer;
- deterministic replay requirements, including replay into read-only analytics/test consumers without replaying gameplay effects;
- versioned event emission, bounded gameplay-telemetry queues and fail-open/fail-closed behavior by durability class;
- channel lifecycle: starting, ready, full, draining, unhealthy, recovering and stopped;
- checkpoint and crash-recovery boundary;
- required outcomes for the named foundation failure scenarios covering overload, stale generations, dependency loss, split ownership and recovery.

### `FND-04` — Identity, Game Session, Admission and Character Lease Contract

The ownership boundary is accepted; the exact mechanism is not. This contract consumes the accepted `FND-ID-01` meanings and may not redefine them.

Decide:

- Game Session token/credential representation: signed, opaque or hybrid;
- issuer, audience and key ownership;
- key discovery, rotation and emergency revocation;
- issue, expiry and reconnect windows;
- atomic consume or equivalent replay prevention;
- capacity routing and channel binding;
- exact admission error vocabulary;
- `connection_generation` and any account/character lease-generation allocation/fencing semantics;
- lease owner and storage;
- lease acquisition, renewal, grace, expiry and release timings;
- duplicate login handling;
- safe channel/instance handoff state machines consuming `HandoffId`;
- network partition and dependency failure behaviour;
- stable public/internal error mapping, correlation/redaction rules and expected results for the shared foundation failure-scenario catalogue.

Minimum data to evaluate includes the accepted FND-ID identities plus protocol/ruleset/content revisions, current fencing values, credential timestamps, issuer and audience. The exact schema remains owned by FND-04/FND-02 rather than this backlog.

## Capacity and channel operations follow-up contracts

ADR-0009 accepts the execution, capacity-measurement, deployment-boundary and recovery direction without inventing unsupported numerical limits or selecting an orchestrator product.

### `PERF-01` — Capacity, Performance and Scalability Contract

Decide and prove:

- named reference hardware, operating system and process/container resource cells;
- command, simulation, queue-age and resource service objectives;
- representative idle, movement, hunting, crowded-interest-set, mass-combat, raid, reconnect-storm, persistence-pressure, multi-channel noisy-neighbor and recovery workloads;
- separate channel, GameNode and logical-world capacity limits;
- safety headroom, overload admission and graceful-degradation policy;
- profiling, soak, memory-growth and performance-regression evidence;
- CI, nightly and release placement for repeatable benchmark evidence.

Player count alone is not a scaling signal. Accepted limits must also account for latency, queue age, CPU, memory, network, persistence health and the first violated service objective.

### `OPS-CHANNEL-01` — GameNode Deployment and Dynamic Channel Orchestration Contract

Decide:

- exact process/container packaging and production control-plane topology;
- GameNode registration, health, readiness, capacity and compatible-revision reporting;
- channel lifecycle, placement, dynamic creation, hysteresis, draining and closure;
- external orchestrator authority and least-privilege boundaries;
- ownership generation, fencing, restart, replacement, checkpoint, bounded replay and reconnect sequencing;
- reconnect grace, player-visible outcomes, RPO, RTO and disaster-test requirements;
- blast-radius limits and recovery-concurrency policy for several channels on one GameNode;
- rollout and rollback behavior without active-channel live migration in the initial implementation.

`FND-03`, `FND-04`, `DUR-02`, `DUR-03` and `QA-E2E-01` retain ownership of their execution, admission, persistence, anti-duplication and physical failure-evidence boundaries.

## Decisions required before durable gameplay mutation

### `DUR-01` — Durable Identifier Representation Contract

Consume the semantic identities already frozen by `FND-ID-01` and define physical durable/database representation where persistence requires it. Additionally freeze the durable-domain identifiers owned by persistence/gameplay domains, including item/entity identities and other later durable records.

Requirements:

- do not redefine `AccountId`, `CharacterId`, `WorldId`, `ChannelId`, `NodeId`, `InstanceId`, `PartyId`, `GameSessionId` or `HandoffId` semantics;
- `CommandId` semantic representation/scope remains owned by `FND-02`; DUR may only define how an accepted CommandId is physically persisted when required for idempotency/audit;
- event/operation/transaction/correlation/causation and pseudonymous analytics identities are finalized with `ANL-01`/durability ownership rather than being pulled back into FND-ID;
- decide UUID/UUIDv7/integer/surrogate use only where the owning semantic contract permits it;
- decide database encoding, indexing, retention/tombstone behavior and entity-ID reuse rules without changing cross-boundary semantics.

### `DUR-02` — Persistence v1 Contract

PostgreSQL is selected. Still decide:

- Rust migration mechanism and schema ownership;
- character state model and revision fencing;
- character lease schema and ownership;
- inventory/equipment and ground-item transfer transaction boundaries;
- idempotency keys and duplicate-command handling;
- isolation levels, locking and retry policy;
- transactional outbox boundaries, publication checkpoints, deduplication and recovery;
- critical append-only audit/journal scope and its separation from best-effort gameplay telemetry;
- atomic production of item/currency/security evidence with the owning authoritative transaction;
- checkpoint interval and maximum accepted progress loss;
- market, guild, house and reward consistency classes;
- partitioning where justified;
- backup, point-in-time recovery, restore tests, RPO and RTO;
- compatible migration rollout and rollback.

The default direction remains current-state tables plus revisions, idempotent commands, transactional outbox and bounded append-only critical audit records, not full event sourcing of every simulation event.

### `DUR-03` — Item Transaction and Anti-Duplication Contract

Decide, either in Persistence v1 or a separate contract:

- canonical item-instance identity and ownership;
- inventory/equipment/container/ground transfer atomicity;
- pickup, drop, loot, trade, reward, bank and depot retry behavior;
- idempotency and duplicate-command outcomes;
- stale-session and stale-writer rejection;
- crash and partial-failure rollback/recovery;
- audit evidence proving that items or currency cannot be duplicated;
- authoritative event semantics for create, destroy, split, merge, move, pickup, drop, loot, trade, market, mail, depot, reward, transform, currency credit/debit, commit, abort and rollback;
- deterministic conservation, provenance and single-authoritative-location invariants that can be independently reconciled by Game Intelligence.

### `DUR-04` — Content, World Detail and Scripting Contract

ADR-0005 accepts the native world format, Oteryn Studio, stable content identity, chunk/semantic geography separation, encounter-placement hierarchy and legacy-conversion boundary. Still decide:

- exact pinned Otheryn, Remere's Map Editor, Beats Assets Editor and other source revisions used as evidence or fixtures;
- classification of each source area as `COPY`, `CONVERT`, `REWRITE`, `REFERENCE_ONLY` or `REJECT` after licensing and provenance review;
- concrete versioned schemas and migration rules for maps, items, monsters, spells, NPCs, spawns, quests, events and assets;
- exact World Project and World Bundle encoding contracts;
- a bounded non-canonical format spike proving deterministic bundle hashes, random access, corruption/decompression failure, source-to-bundle-to-load equivalence and measured 32x32 versus 64x64 chunk/floor packing before final encoding is frozen;
- chunk-size/floor-packing benchmark and spatial indexing details;
- Content Registry package/version/dependency rules;
- client asset provenance and rights;
- scripting language/runtime;
- script API, world/channel/instance context and capability limits;
- CPU/time/memory budgets;
- failure isolation and error policy;
- persistence access policy;
- hot-reload policy;
- deterministic conversion tooling and fixture corpus.

Scripts must not receive a global mutable `Game` object or direct SQL access. Legacy tools and proprietary assets must not be copied without confirmed rights and pinned provenance.

## Cross-cutting Game Intelligence contracts

ADR-0006 accepts the subsystem direction. The following contracts freeze its implementable boundaries without creating runtime code in the architecture package.

### `ANL-01` — Game Event and Audit Foundation Contract

Decide:

- canonical `EventId`, `OperationId`, `TransactionId`, `CorrelationId`, `CausationId` and pseudonymous `AnalyticsActorId` semantics/representation/scope where required by the event/audit foundation;
- common versioned event envelope and event-family ownership;
- operational, best-effort telemetry and durable audit classifications;
- producers, consumers, ordering, causation/correlation and schema compatibility;
- bounded in-process queues, overload, drop and fail-open behavior for gameplay telemetry;
- transactional outbox/atomic audit behavior for economy and security mutations;
- idempotent delivery, deduplication, publication checkpoints and replay into read-only consumers;
- resource limits registered in `RESOURCE_LIMITS_REGISTRY.json`, backpressure, dead-letter lifecycle and observability;
- stable error/outcome mapping through `FOUNDATION_ERROR_VOCABULARY.md`;
- privacy classes, pseudonymous identity, access roles, retention, deletion/anonymization and legal hold;
- named analytics/audit/privacy/investigation scenarios from `FOUNDATION_FAILURE_SCENARIOS.md`;
- golden fixtures and duplicate/out-of-order/schema-evolution tests.

This contract must be accepted before `DUR-02` and `DUR-03` finalize the outbox/audit evidence required by their transactions. It does not block the completed migration/FND-ID gates.

### `ANL-02` — Gameplay, Balance and World Analytics Contract

Decide:

- hunt/session and aggregate semantics derived from explicit events;
- vocation/class, level, equipment/power, party, hunt-area and version dimensions;
- damage, healing, death, experience, spell, monster, supplies, loot and profit metrics;
- `Area`/`Subarea`/`EncounterZone`/`RaidCell`/`RaidAnchor` and technical region/chunk analysis;
- minimum sample sizes, confidence, regression comparison and misleading-series prevention;
- geography/privacy trade-offs, storage, retention, dashboards and reconciliation.

Analytics remains observational and may not change balance automatically.

### `ANL-03` — Economy Integrity and Security Analytics Contract

Decide:

- item/currency provenance consumers and deterministic invariant catalogue;
- alert and investigation-case lifecycle;
- bot, exploit, replay, cooldown, transfer-graph and protocol anomaly signals;
- detector/rule/model versioning, evidence references and false-positive handling;
- separation between deterministic enforcement in runtime and observational alerting;
- human authorization and audited disposition for sanctions or remediation.

This contract supplements but never replaces the prevention guarantees of `DUR-03`.

### `ANL-04` — Read-Only Investigation and AI Contract

Decide:

- read-only views, replicas and evidence-package APIs;
- case correlation, provenance reconstruction and report generation;
- model/rule provenance, confidence and human review;
- least-privilege credentials and full access auditing;
- hard prohibitions on runtime/database mutation, autonomous bans, balance changes, rollback, deployment and unsupported proof claims.

This is an expansion gate and is not required for the foundation vertical slice.

## `QA-E2E-01` — Native End-to-End Test Platform Contract

ADR-0007 accepts the architecture. Implementation must provide:

- one shared versioned scenario contract and orchestration platform;
- Tier 1 headless system E2E using production transport and `protocol-oteryn` through the supported Platform/Gateway path;
- Tier 2 instrumented native-client E2E whose test-only adapter may observe and submit normal actions but may not bypass admission or mutate authoritative state;
- Tier 3 smoke E2E using exact production-default release-candidate artifacts;
- deterministic seeds, clocks, topology and fault profiles;
- stable semantic observations and read-only probes rather than gameplay mutation shortcuts;
- one result envelope per attempt with exact revisions/hashes, ordered phases, first divergence and cleanup result;
- visible repeated-run populations with no hidden retry-until-green;
- PR/main/nightly/release placement proportional to tier and risk.

The canonical client migration and FND-ID gate are complete. Protocol, admission, durable mutation and content scenarios consume their respective accepted contracts. `QA-E2E-01` blocks completion of `VSL-01`, not architecture discovery.

## `VSL-01` — Foundation Vertical-Slice Programme

Approve ownership, implementation order and evidence for this minimum scenario using the accepted `QA-E2E-01` platform and evidence contract:

1. client authenticates through Platform;
2. client receives world/channel directory data;
3. Gateway issues bounded pre-admission material;
4. Rust game server validates admission, establishes canonical game-domain Game Session and acquires the character lease;
5. character enters a minimal native map;
6. movement is authoritative;
7. two clients observe each other on one channel;
8. one monster can be attacked and killed;
9. one corpse/loot result is produced;
10. one item is picked up through a retry-safe transfer;
11. combat, death, loot and pickup emit correlated versioned events;
12. durable item/pickup evidence is atomic with the authoritative mutation;
13. duplicate delivery/replay into analytics does not duplicate an item or analytical aggregate;
14. character state is checkpointed and saved;
15. logout is safe;
16. the character logs into another channel with inventory/progression preserved;
17. a simultaneous second authoritative character session is rejected according to the accepted account/character policy;
18. channel-local state remains isolated while world-shared state remains shared.

## Decisions required as the vertical slice expands

- position, direction and entity-lifetime encoding;
- movement, collision, pathfinding and visibility contracts;
- minimal combat, death, corpse, loot and attribution contracts;
- PvP, skull, frag and combat-lock scope;
- party membership versus shared-experience behaviour across channels;
- boss, raid, chest and daily-reward anti-hopping policy;
- encounter uniqueness and cooldown scope across channels;
- world communication and presence service boundary;
- event journal, transactional outbox, publication checkpoint and replay timing;
- gameplay/balance/world analytics dimensions and sample-quality policy;
- item/currency provenance, invariant monitoring and security case lifecycle;
- metrics, tracing, log redaction, pseudonymization, access control and differentiated retention;
- updater, asset signing and release security;
- supported client platforms and server architectures;
- quantitative capacity, latency, reconnect, RPO and RTO targets;
- Foundation, Playable Alpha, Beta and release scope.

## Registered gameplay and product decision horizon

The complete open-decision scope is canonical in `GAMEPLAY_AND_PRODUCT_ARCHITECTURE_HORIZON.md`.

- `GAME-CHAR-01` and `GAME-ITEM-01` are new durable-gameplay gates: character semantics must precede final `DUR-02`, and item semantics must precede final `DUR-03`.
- `GAME-ABILITY-01`, `GAME-AI-01` and `GAME-INTERACTION-01` are required before Playable Alpha gameplay breadth is claimed; bounded vertical-slice contracts may precede them.
- `PROD-LIVEOPS-01`, `PROD-COMPAT-01`, `SEC-CLIENT-01`, `DATA-PRIVACY-01`, `UX-I18N-A11Y-01` and `OPS-GM-01` are required before Playable Alpha operational completeness is claimed.
- `PERF-01` is required before Playable Alpha claims representative-load readiness or publishes supported channel, GameNode or world capacity.
- `OPS-CHANNEL-01` is required before automatic production channel scaling or production GameNode/channel recovery is claimed.
- `GAME-META-01`, `GAME-INSTANCES-01`, `GAME-WORLD-LIFECYCLE-01` and `INTEGRATION-API-01` are expansion gates.
- `PROD-ENTITLEMENTS-01` and `MOD-ECOSYSTEM-01` remain explicitly deferred until an owner decision activates them.

Registering a gate does not accept its implementation choice.

## Explicitly deferred

These do not block the completed migration/FND-ID gates or the foundation vertical slice when extension points remain safe:

- final house presence and entry topology;
- live migration of an active channel;
- partitioning one channel across multiple nodes;
- QUIC support;
- cross-world chat, guilds or parties;
- hundreds of dynamically created channels;
- `GAME-INSTANCES-01` complete dungeon/arena/matchmaking/spectator programme;
- full market implementation;
- all classic rulesets;
- final launcher/updater implementation;
- `MOD-ECOSYSTEM-01` public mod ecosystem;
- advanced client prediction;
- extraction of world services into independent deployable microservices.

## Programme task model

The canonical foundation task is a non-owning programme checkpoint. It preserves accepted state, dependencies and one next action, but it must not claim files or act as the implementation task for all contracts.

Every substantial gate uses its own:

- task ID containing the stable gate ID where practical;
- owner, owned paths and public contracts;
- dedicated branch and PR;
- focused validation, independent audit and exact-head CI;
- terminal archive and ownership release.

No package may edit another active package's owned contract without explicit coordination.

## Recommended decision and implementation order

```text
1. COMPLETED — Accept FND-01 Workspace, Dependency and Existing-Rust Migration Contract
2. COMPLETED — Accept VSL-02 Exact Rust Client Migration and Cutover Contract
3. COMPLETED — Freeze/reconcile migration source and deliver canonical Oteryn-v2 workspace
4. COMPLETED — Close source-only otclient historical/non-canonical marker
5. COMPLETED — Accept and merge FND-ID-01 Foundation Identifier Contract
6. CURRENT CLEANUP — Reconcile issue #86 coordination views to the accepted FND-ID scope
7. NEXT — Accept FND-02 protocol-oteryn v1 Contract
8. THEN — Accept FND-03 Runtime Execution Contract, including clock/event-emission/runtime-handle semantics
9. THEN — Accept FND-04 Identity, Game Session, Admission and Character Lease Contract
10. Accept DUR-01 durable/database identifier representation contract
11. Accept GAME-CHAR-01 before DUR-02 freezes the durable character schema
12. Accept GAME-ITEM-01 before DUR-03 freezes item behavior and transfer semantics
13. Accept ANL-01 Game Event and Audit Foundation Contract
14. Accept DUR-02 Persistence v1 Contract with transactional outbox/audit recovery
15. Accept DUR-03 Item Transaction and Anti-Duplication Contract, if not complete in DUR-02
16. Draft ANL-02 and ANL-03 on the accepted event/persistence/item foundations
17. Run the bounded world-format spike and complete DUR-04 under ADR-0005
18. Implement QA-E2E-01 incrementally as protocol, admission, persistence and content prerequisites land
19. Accept VSL-01 Foundation Vertical-Slice Programme with correlated event/audit evidence and named QA-E2E-01 tiers
20. Execute the separately authorized vertical-slice implementation programme
21. Accept PERF-01 before publishing supported capacity or claiming Playable Alpha representative-load readiness
22. Accept OPS-CHANNEL-01 before automatic production channel scaling or production GameNode/channel recovery behavior is claimed
23. Accept GAME-ABILITY-01, GAME-AI-01 and GAME-INTERACTION-01 before Playable Alpha gameplay breadth is claimed
24. Accept PROD-LIVEOPS-01, PROD-COMPAT-01, SEC-CLIENT-01, DATA-PRIVACY-01, UX-I18N-A11Y-01 and OPS-GM-01 before Playable Alpha operational completeness is claimed
25. Complete ANL-02/ANL-03 before production-grade alpha analytics claims; defer ANL-04 until read-only investigation is authorized
26. Activate expansion/deferred gameplay-product gates only when their milestone or explicit owner decision requires them
```

Contracts may be developed in parallel only when ownership and dependencies do not overlap. Cross-repository changes require separate authorized tasks, branches and PRs with one coordination ID and explicit rollout order.

## Start gates

- `FND-01`, `VSL-02`, destination migration and source-only closeout are complete and verified.
- `FND-ID-01` is accepted and merged; its semantics are mandatory input to `FND-02`, `FND-03` and `FND-04` and may not be redefined there.
- issue #86 is coordination cleanup only; it does not reopen FND-ID semantics.
- after #86 is merged and verified, `FND-02` is the clean next ordered architecture gate.
- `FND-02`, `FND-03` and `FND-04` gate canonical protocol, authoritative runtime and production admission/lease implementation respectively.
- `DUR-01`, `DUR-02` and `DUR-03` must be accepted before authoritative durable character, item or currency mutation.
- `GAME-CHAR-01` must be accepted before `DUR-02` finalizes the character schema; `GAME-ITEM-01` must be accepted before `DUR-03` finalizes item semantics.
- `GAME-ABILITY-01`, `GAME-AI-01` and `GAME-INTERACTION-01` are required before Playable Alpha gameplay breadth is claimed; bounded vertical-slice contracts may precede them.
- `PROD-LIVEOPS-01`, `PROD-COMPAT-01`, `SEC-CLIENT-01`, `DATA-PRIVACY-01`, `UX-I18N-A11Y-01` and `OPS-GM-01` are required before Playable Alpha operational completeness is claimed.
- ADR-0005 is the accepted world/content direction; `DUR-04` must be accepted before broad content import or durable scripting.
- `QA-E2E-01` must be implemented with its required exact evidence before `VSL-01` implementation is called complete; feature packages add scenarios/assertions, not competing lifecycle/evidence runners.
- `VSL-01` must name observable E2E evidence before implementation is called complete.
- `ANL-01` must be accepted before final `DUR-02`/`DUR-03` outbox and audit boundaries are frozen.
- `ANL-02`/`ANL-03` are required before production-grade balance/world and economy/security analytics claims; `ANL-04` remains a later read-only investigation gate.
- `PERF-01` must be accepted before publishing player/channel/GameNode/world capacity claims or calling Playable Alpha representative-load ready.
- `OPS-CHANNEL-01` must be accepted before automatic production channel scaling or production GameNode/channel recovery behavior is claimed.

## Current next action

Complete and merge issue #86 coordination reconciliation on the two long-lived architecture registers. After exact-head validation, audit and closeout, begin a separate bounded **architecture-only `FND-02` contract task** consuming:

- `FND-ID-01_FOUNDATION_IDENTIFIER_CONTRACT.md`;
- `FND-02_PLATFORM_PROTOCOL_RECONCILIATION_OWNER_BASELINE.md`;
- the current cross-repository contract lock and shared resource/error/failure catalogues.

Do not implement `protocol-oteryn`, runtime listeners/codecs or Platform changes until a later explicitly authorized implementation task passes its owning architecture gates.
