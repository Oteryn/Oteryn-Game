# Oteryn v2 Foundation Decision Backlog

- Status: Active architecture backlog
- Date: 2026-08-05
- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Canonical programme task: `docs/agents/tasks/active/OTV2-20260805-foundation-preimplementation-contracts.md`
- Global decision register: `docs/architecture/GLOBAL_ARCHITECTURE_DECISION_REGISTER.md`
- Coordinator prompt: `docs/agents/prompts/OTV2_GLOBAL_ARCHITECTURE_DECISION_COORDINATOR.md`

## Purpose

Record the remaining architecture decisions in one durable, ordered register. This file distinguishes decisions that block root-workspace bootstrap, decisions that gate specific implementation layers, decisions required before durable gameplay, and subjects deliberately deferred.

The complete later project horizon is maintained in `GLOBAL_ARCHITECTURE_DECISION_REGISTER.md`. That register preserves gameplay, client, content, operations and product domains without forcing them to be designed before they block the current stage.

Chat history is not authoritative. Accepted ADRs, this backlog, the global register, the active programme task and live PR/CI state are the continuation sources.

## Stable gate identifiers

Use these identifiers in tasks, PRs, prompts and cross-repository coordination. Section numbers or stage letters are presentation only and must not replace the stable ID.

- `FND-01` — Workspace, Dependency and Existing-Rust Migration Contract.
- `FND-ID-01` — Foundation Identifier Vocabulary.
- `FND-02` — `protocol-oteryn` v1 Contract.
- `FND-03` — Runtime Execution Contract.
- `FND-04` — Identity, Game Session, Admission and Character Lease Contract.
- `DUR-01` — Durable Identifier Representation Contract.
- `DUR-02` — Persistence v1 Contract.
- `DUR-03` — Item Transaction and Anti-Duplication Contract.
- `DUR-04` — Content, World Detail and Scripting Contract.
- `VSL-01` — Foundation Vertical-Slice Programme.
- `VSL-02` — Exact Rust Client Migration and Cutover Contract.
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

2. **Repository ownership**
   - the existing Rust client will move from `blakinio/otclient/oteryn-client` into `blakinio/Oteryn-v2`;
   - client, server, shared Rust types and `protocol-oteryn` will have one canonical repository and workspace;
   - `blakinio/otclient` remains migration/history evidence after the move;
   - `protocol-canary` is not part of the target runtime.

3. **Identity and Game Gateway boundary**
   - Platform Identity remains the reusable-credential, OAuth/PKCE, MFA and account-security authority;
   - Oteryn v2 does not introduce a second classic login server or password/OAuth authority;
   - Game Gateway remains in `blakinio/Oteryn-Platform`;
   - the initial Game Gateway remains implemented in Go;
   - the Rust game server accepts the bounded Game Session admission contract and owns gameplay admission and runtime state.

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

Canonical decisions: ADR-0001 through ADR-0006.

## Progressive implementation policy

Architecture contracts remain mandatory for public and durable behavior, but compile-time and prototype evidence are part of architecture discovery.

### Gate 0 — before `FND-01`

- Do not create the canonical root Cargo workspace.
- Documentation, repository discovery and read-only technical research are allowed.
- A disposable experiment may exist only outside the canonical product paths or on a clearly bounded spike branch, with no compatibility, production-readiness or source-of-truth claim.

### Gate 1 — after `FND-01`

A separately authorized workspace-bootstrap task may create the smallest compilable root workspace and executable architecture checks. It may include only members required by an immediate consumer, test or vertical proof.

Allowed:

- root Cargo metadata and toolchain configuration;
- minimal placeholder-free crates/applications selected by `FND-01`;
- compile-only interfaces and test fixtures that do not silently freeze an unresolved public contract;
- dependency-graph checks and forbidden-edge validation;
- bounded, reversible technical spikes whose results are recorded as evidence.

Not yet allowed unless its own gate has passed:

- production `protocol-oteryn` wire compatibility claims;
- authoritative channel/runtime behavior;
- live Game Session admission or character lease behavior;
- durable authoritative gameplay mutation;
- broad client migration or broad content import.

### Gate 2 — layer-specific implementation

- `FND-ID-01` gates freezing identifier representations in protocol, Game Session and admission contracts.
- `FND-02` gates canonical protocol schemas/codecs, production framing and compatibility claims.
- `FND-03` gates authoritative runtime scheduling, ordering, lifecycle and recovery behavior.
- `FND-04` gates production admission, Game Session validation and character lease behavior.
- `DUR-01` through `DUR-03` gate authoritative durable character, item and currency mutation.
- `DUR-04` gates broad content import and durable scripting behavior.
- `VSL-01` gates the claim that the first native gameplay slice is complete.
- `ANL-01` gates final transactional event/outbox/audit boundaries used by `DUR-02` and `DUR-03`.
- `ANL-02` and `ANL-03` gate production-grade balance/world and economy/security analytics claims.
- `ANL-04` gates read-only AI investigation and is not required for the foundation vertical slice.

A spike is never a substitute for an accepted contract. It must be reversible, isolated from public contracts, excluded from production defaults and either removed or deliberately promoted by a later accepted task.

## Foundation contracts

### `FND-01` — Workspace, Dependency and Existing-Rust Migration Contract

This is the only contract that blocks creation of the canonical root Cargo workspace. It is also the mandatory migration audit for the existing Rust implementation in `blakinio/otclient/oteryn-client`; it must not design a duplicate workspace in isolation.

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
- criteria for adding, splitting or merging crates later.

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

Mandatory direction: domain and simulation crates do not depend on Tokio, TCP, TLS, HTTP, SQL, PostgreSQL, Platform APIs, renderer state or UI widgets. World/content schema crates do not depend on Tauri, editor UI or renderer implementation. These boundaries must become machine-checked after bootstrap.

### `FND-ID-01` — Foundation Identifier Vocabulary

Accept this minimum vocabulary after `FND-01` and before `FND-02` or `FND-04` freezes wire/session schemas. Decide only the semantics required across process and repository boundaries:

- meaning, canonical owner and scope of `AccountId`, `CharacterId`, `WorldId`, `ChannelId`, `InstanceId`, `NodeId`, `GameSessionId`, `CommandId`, `EntityId`, `EventId`, `OperationId`, `TransactionId`, `CorrelationId`, `CausationId`, `AnalyticsActorId`, `ProtocolRevision`, `RulesetRevision`, `ContentRevision` and `SessionGeneration`;
- global versus scoped uniqueness and whether reuse is permitted;
- durable versus runtime-only identity;
- public/client-visible versus opaque identity;
- canonical comparison, textual diagnostics and wire/Game Session encoding constraints;
- compatibility and migration rules when an external producer uses a different storage representation.

Do not freeze PostgreSQL column types, indexes, full item-instance identity or every durable entity here. Those remain in `DUR-01`; the purpose of `FND-ID-01` is to prevent protocol and admission contracts from inventing incompatible meanings first.

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
- sequencing, acknowledgement and error vocabulary;
- `command_id`, replay protection and idempotency;
- snapshot, delta and reconciliation contracts;
- reconnect/resume semantics;
- compression rules and bounded allocation;
- entries in the resource-limit registry for every externally controlled size/depth/count;
- stable machine error categories and mapping into the foundation error vocabulary;
- golden fixtures shared by client and server.

Do not create a second silent native protocol beside the existing Platform contract. Rust memory layout or unstable serializer output cannot be the public wire contract.

### `FND-03` — Runtime Execution Contract

Decide:

- `NodeRuntime`, `WorldServices`, `ChannelRuntime` and `InstanceRuntime` responsibilities;
- modular-monolith initial deployment topology;
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

- Game Session token format: signed, opaque or hybrid;
- issuer, audience and key ownership;
- key discovery, rotation and emergency revocation;
- issue, expiry and reconnect windows;
- atomic consume or equivalent replay prevention;
- capacity routing and channel binding;
- exact admission error vocabulary;
- `session_generation` allocation and fencing;
- lease owner and storage;
- lease acquisition, renewal, grace, expiry and release timings;
- duplicate login handling;
- safe channel switching;
- network partition and dependency failure behaviour;
- stable public/internal error mapping, correlation/redaction rules and expected results for the shared foundation failure-scenario catalogue.

Minimum data to evaluate:

```text
account_id
character_id
world_id
channel_id
game_session_id
session_generation
protocol_revision
ruleset_revision
content_revision
issued_at
expires_at
issuer
audience
```

## Decisions required before durable gameplay mutation

### `DUR-01` — Durable Identifier Representation Contract

Freeze representations and visibility for:

- `AccountId`;
- `CharacterId`;
- `WorldId`;
- `ChannelId`;
- `InstanceId`;
- `NodeId`;
- `GameSessionId`;
- `CommandId`;
- `EntityId`;
- `ItemInstanceId`;
- `ProtocolRevision`;
- `ContentRevision`;
- `RulesetRevision`;
- `SessionGeneration`;
- `EventId`;
- `OperationId`;
- `TransactionId`;
- `CorrelationId`;
- `CausationId`;
- `AnalyticsActorId`.

Decide UUID/UUIDv7/integer usage, global versus scoped uniqueness, wire encoding, database encoding, public visibility and entity-ID reuse rules.

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

This contract must be accepted before `DUR-02` and `DUR-03` finalize the outbox/audit evidence required by their transactions. It does not block minimal workspace bootstrap.

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

## `VSL-01` — Foundation Vertical-Slice Programme

Approve ownership, implementation order and evidence for this minimum scenario:

1. client authenticates through Platform;
2. client receives world/channel directory data;
3. Gateway issues a bound Game Session;
4. Rust game server validates admission and acquires the character lease;
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
17. a simultaneous second login of the same character is rejected;
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
- `VSL-02` exact Rust client migration revision, provenance, open-PR disposition, development freeze/cutover, history-preservation mechanism and rollback;
- event journal, transactional outbox, publication checkpoint and replay timing;
- gameplay/balance/world analytics dimensions and sample-quality policy;
- item/currency provenance, invariant monitoring and security case lifecycle;
- metrics, tracing, log redaction, pseudonymization, access control and differentiated retention;
- updater, asset signing and release security;
- supported client platforms and server architectures;
- quantitative capacity, latency, reconnect, RPO and RTO targets;
- Foundation, Playable Alpha, Beta and release scope.

## Explicitly deferred

These do not block the initial workspace bootstrap or foundation vertical slice when extension points remain safe:

- final house presence and entry topology;
- live migration of an active channel;
- partitioning one channel across multiple nodes;
- QUIC support;
- cross-world chat, guilds or parties;
- hundreds of dynamically created channels;
- complete dungeon/arena instance programme;
- full market implementation;
- all classic rulesets;
- final launcher/updater implementation;
- public mod ecosystem;
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
1. Accept FND-01 Workspace, Dependency and Existing-Rust Migration Contract
2. Run a separate minimal workspace-bootstrap implementation task
3. Accept FND-ID-01 Foundation Identifier Vocabulary, including event/operation/transaction identities
4. Merge and lock the final canonical Platform native-contract correction
5. Accept FND-02 protocol-oteryn v1 Contract
6. Accept FND-03 Runtime Execution Contract, including clock and event-emission semantics
7. Accept FND-04 Identity, Game Session, Admission and Character Lease Contract
8. Accept DUR-01 full Identifier Contract for database and durable-state representation
9. Accept ANL-01 Game Event and Audit Foundation Contract
10. Accept DUR-02 Persistence v1 Contract with transactional outbox/audit recovery
11. Accept DUR-03 Item Transaction and Anti-Duplication Contract, if not complete in DUR-02
12. Draft ANL-02 and ANL-03 on the accepted event/persistence/item foundations
13. Run the bounded world-format spike and complete DUR-04 under ADR-0005
14. Accept VSL-01 Foundation Vertical-Slice Programme with correlated event/audit evidence
15. Accept VSL-02 Exact Rust Client Migration and Cutover Contract before moving client code
16. Execute the separately authorized vertical-slice implementation programme
17. Complete ANL-02/ANL-03 before production-grade alpha analytics claims; defer ANL-04 until read-only investigation is authorized
```

Contracts may be developed in parallel only when ownership and dependencies do not overlap. Cross-repository changes require separate authorized tasks, branches and PRs with one coordination ID and explicit rollout order.

## Start gates

- `FND-01` must be accepted before creation of the canonical root Cargo workspace.
- After `FND-01`, only a separate minimal bootstrap task is authorized; unresolved layer contracts still gate their production behavior.
- `FND-ID-01` must be accepted before `FND-02` or `FND-04` freezes identifier meanings on the wire or in Game Sessions.
- `FND-02`, `FND-03` and `FND-04` gate canonical protocol, authoritative runtime and production admission/lease implementation respectively.
- `DUR-01`, `DUR-02` and `DUR-03` must be accepted before authoritative durable character, item or currency mutation.
- ADR-0005 is the accepted world/content direction; `DUR-04` must be accepted before broad content import or durable scripting.
- `VSL-01` must name observable E2E evidence before implementation is called complete.
- `VSL-02` must pin source SHA, provenance, open-PR disposition, source freeze, cutover, history preservation and rollback before moving client code.
- `ANL-01` must be accepted before final `DUR-02`/`DUR-03` outbox and audit boundaries are frozen.
- `ANL-02`/`ANL-03` are required before production-grade balance/world and economy/security analytics claims; `ANL-04` remains a later read-only investigation gate.

## Current next action

Execute `docs/agents/prompts/OTV2_GLOBAL_ARCHITECTURE_DECISION_COORDINATOR.md` and draft, audit, accept, merge and archive `FND-01` — the **Workspace, Dependency and Existing-Rust Migration Contract**.