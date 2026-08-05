# Oteryn v2 Foundation Decision Backlog

- Status: Active architecture backlog
- Date: 2026-08-05
- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Canonical programme task: `docs/agents/tasks/active/OTV2-20260805-foundation-preimplementation-contracts.md`

## Purpose

Record the remaining architecture decisions in one durable, ordered register. This file distinguishes accepted decisions, decisions that block creation of the real Rust workspace, decisions required before durable gameplay, and subjects deliberately deferred.

Chat history is not authoritative. Accepted ADRs, this backlog, the active programme task and live PR/CI state are the continuation sources.

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

Canonical decisions: ADR-0001 through ADR-0005.

## Decisions blocking the real Rust workspace

The real root Cargo workspace must not be created until these contracts have enough accepted detail to prevent an incorrect foundation.

### B1. Workspace and dependency contract

Decide:

- exact initial workspace members and crate/application names;
- legal dependency directions;
- ownership of shared identifiers, domain contracts, protocol schemas and test fixtures;
- feature policy and dependency review rules;
- target Rust edition and minimum supported toolchain;
- initial CI targets, at minimum Linux server and Windows client;
- whether optional client-only and server-only dependencies are isolated cleanly.

Candidate shape:

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

Mandatory direction: domain and simulation crates do not depend on Tokio, TCP, TLS, HTTP, SQL, PostgreSQL, Platform APIs, renderer state or UI widgets. World/content schema crates do not depend on Tauri, editor UI or renderer implementation.

### B2. `protocol-oteryn` v1 contract

Decide:

- whether the existing Platform native contract is adopted, revised or explicitly superseded for the Rust server target;
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
- golden fixtures shared by client and server.

Do not create a second silent native protocol beside the existing Platform contract. Rust memory layout or unstable serializer output cannot be the public wire contract.

### B3. Runtime execution contract

Decide:

- `NodeRuntime`, `WorldServices`, `ChannelRuntime` and `InstanceRuntime` responsibilities;
- modular-monolith initial deployment topology;
- fixed or variable tick model and initial tick frequency;
- authoritative command ordering;
- timer and scheduler semantics;
- bounded inbound/outbound/work queues;
- overload and backpressure behaviour;
- parallel pathfinding, AI and content work and safe return to the logical writer;
- deterministic replay requirements;
- channel lifecycle: starting, ready, full, draining, unhealthy, recovering and stopped;
- checkpoint and crash-recovery boundary.

### B4. Identity, Game Session, admission and lease contract

The ownership boundary is accepted; the exact mechanism is not.

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
- network partition and dependency failure behaviour.

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

### D1. Identifier contract

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
- `ProtocolRevision`;
- `ContentRevision`;
- `RulesetRevision`;
- `SessionGeneration`.

Decide UUID/UUIDv7/integer usage, global versus scoped uniqueness, wire encoding, database encoding, public visibility and entity-ID reuse rules.

### D2. Persistence v1 contract

PostgreSQL is selected. Still decide:

- Rust migration mechanism and schema ownership;
- character state model and revision fencing;
- character lease schema and ownership;
- inventory/equipment and ground-item transfer transaction boundaries;
- idempotency keys and duplicate-command handling;
- isolation levels, locking and retry policy;
- transactional outbox boundaries;
- critical append-only audit/journal scope;
- checkpoint interval and maximum accepted progress loss;
- market, guild, house and reward consistency classes;
- partitioning where justified;
- backup, point-in-time recovery, restore tests, RPO and RTO;
- compatible migration rollout and rollback.

The default direction remains current-state tables plus revisions, idempotent commands, transactional outbox and bounded append-only critical audit records, not full event sourcing of every simulation event.

### D3. Remaining content migration and scripting contract

ADR-0005 accepts the native world format, Oteryn Studio, stable content identity, chunk/semantic geography separation, encounter-placement hierarchy and legacy-conversion boundary. Still decide:

- exact pinned Otheryn, Remere's Map Editor, Beats Assets Editor and other source revisions used as evidence or fixtures;
- classification of each source area as `COPY`, `CONVERT`, `REWRITE`, `REFERENCE_ONLY` or `REJECT` after licensing and provenance review;
- concrete versioned schemas and migration rules for maps, items, monsters, spells, NPCs, spawns, quests, events and assets;
- exact World Project and World Bundle encoding contracts;
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

### D4. Foundation vertical-slice programme

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
11. character state is checkpointed and saved;
12. logout is safe;
13. the character logs into another channel with inventory/progression preserved;
14. a simultaneous second login of the same character is rejected;
15. channel-local state remains isolated while world-shared state remains shared.

## Decisions required as the vertical slice expands

- position, direction and entity-lifetime encoding;
- PvP, skull, frag and combat-lock scope;
- party membership versus shared-experience behaviour across channels;
- boss, raid, chest and daily-reward anti-hopping policy;
- encounter uniqueness and cooldown scope across channels;
- world communication and presence service boundary;
- event journal and checkpoint timing;
- metrics, tracing, log redaction and audit retention;
- updater, asset signing and release security;
- supported client platforms and server architectures;
- quantitative capacity, latency, reconnect, RPO and RTO targets.

## Explicitly deferred

These do not block the initial workspace or foundation vertical slice when extension points remain safe:

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

## Recommended decision order

```text
1. Workspace and dependency contract
2. protocol-oteryn v1 contract
3. Runtime execution contract
4. Identity, Game Session, admission and lease contract
5. Identifier contract
6. Persistence v1 contract
7. Complete the remaining content migration and scripting contract under ADR-0005
8. Foundation vertical-slice programme
9. Pin and migrate the existing Rust client
10. Create the real workspace and begin implementation
```

Contracts may be developed in parallel only when ownership and dependencies do not overlap. Cross-repository changes require separate authorized tasks, branches and PRs with one coordination ID and explicit rollout order.

## Start gates

- Contracts B1 through B4 must be accepted before creation of the real Rust workspace.
- Identifier and Persistence v1 contracts must be accepted before durable gameplay mutations.
- ADR-0005 is the accepted world/content direction; its remaining concrete format, migration, asset-rights and scripting contracts must be accepted before broad content import.
- The vertical-slice programme must name observable E2E evidence before implementation is called complete.
