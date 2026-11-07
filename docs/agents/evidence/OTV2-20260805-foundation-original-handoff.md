# OTV2-20260805-foundation-preimplementation-contracts

```yaml
task_id: OTV2-20260805-foundation-preimplementation-contracts
title: Freeze Oteryn v2 pre-implementation foundation contracts
mode: CONTRACT
status: ready
repository: blakinio/Oteryn-v2
base_branch: main
branch: null
pr: null
base_sha: 272e625fdc397acedab643919cc289a5643b1156
head_sha: null
owner: unassigned
created_at: 2026-08-05T08:49:00+02:00
updated_at: 2026-08-05T08:49:00+02:00
execution_budget_minutes: 120
large_budget_reason: The contract programme spans repository ownership, protocol, runtime, persistence, content, Platform integration and a cross-repository migration boundary.
owned_paths: []
public_contracts:
  - docs/architecture/ADR-0001-native-rust-multichannel-platform.md
  - docs/architecture/MULTICHANNEL_SYSTEM_SCOPE_MATRIX.md
  - docs/architecture/OTHERYN_REFERENCE_MIGRATION_PLAN.md
depends_on:
  - ADR-0001 accepted native Rust and multichannel foundation
blocks:
  - creation of the root Cargo workspace
  - implementation of protocol-oteryn
  - migration of the existing Rust client
  - implementation of the authoritative Rust game server
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories:
  - blakinio/Oteryn-Platform
  - blakinio/Otheryn
  - blakinio/otclient
```

## Outcome

Produce and accept the minimum set of contracts required to create the real Oteryn v2 Rust workspace without locking the project into an incorrect repository boundary, dependency graph, network protocol, persistence model or runtime topology.

This task is the durable continuation point for the architecture discussion completed on 2026-08-05. A new agent must resume from this file and the live repository state. The previous chat transcript is not required.

## Current programme state

### Already completed

- The repository exists and contains accepted architecture documentation and agent governance.
- Agent policies were audited against the other project repositories, adapted to Oteryn v2, validated and merged.
- The repository currently contains documentation, governance and tools only.
- No root Rust `Cargo.toml`, application workspace, server implementation or `protocol-oteryn` crate exists yet.
- No active implementation branch or PR owns the future Rust workspace paths.

### Current repository boundary

The current durable top-level tree is approximately:

```text
.github/
AGENTS.md
AGENTS.override.md
README.md
docs/
  agents/
  architecture/
tools/
```

The planned `apps/`, `services/`, `crates/`, `content/`, `tests/` and `docs/contracts/` paths are targets only. They must not be presented as already implemented.

## Architecture and source of truth

### PROVEN — accepted product and architecture decisions

1. **Native Rust target stack**
   - authoritative game server in Rust;
   - native game client in Rust;
   - one project-owned gameplay protocol named `protocol-oteryn`.

2. **Legacy protocol direction**
   - `protocol-canary` and legacy Tibia packet families are not part of the target runtime;
   - historical gameplay eras are ruleset/content profiles, not separate wire protocols.

3. **Otheryn role**
   - the C++ Otheryn server is a behavioural, content and migration reference;
   - it is not the target runtime and must not be translated file by file or used to preserve global mutable singleton architecture.

4. **Multichannel-first world model**
   - one logical world, for example Antica, may expose several parallel gameplay channels;
   - example: one Antica world with Channel 1 through Channel 5;
   - a channel is a separate simulation of the public map, not a separate economy or community;
   - `WorldId`, `ChannelId`, `InstanceId`, `NodeId` and `GameSessionId` are distinct concepts.

5. **Shared and local state**
   - character persistence, guilds, market, world economy, progression and selected social systems are shared by all channels of one world;
   - creature state, combat, respawns, corpses, ground items, local NPC runtime and public-map overlays are channel-local.

6. **Cross-channel communication within one world**
   - world chat, guild chat and private messages operate across all channels of the same logical world;
   - local `say`, `whisper`, `yell` and position-dependent communication remain channel/instance-local;
   - cross-world chat is not currently part of the accepted target.

7. **One logical mutation owner per channel**
   - authoritative mutations inside one `ChannelRuntime` have explicit deterministic ordering;
   - parallel work may occur outside the logical writer, but results return through controlled queues and cannot mutate channel state directly.

8. **Character session exclusivity**
   - a character may have at most one active authoritative game session;
   - durable writes require session-generation fencing;
   - a stale or recovered channel cannot overwrite a newer session.

9. **Channel switching**
   - channel change is a safe exit/checkpoint followed by a new Game Session;
   - it is not an in-place teleport or protocol adapter switch;
   - combat lock, unresolved item mutations, direct trade, protected encounters and unsafe instance transitions block channel changes.

10. **Platform authority**
    - `blakinio/Oteryn-Platform` remains the Identity/OAuth/PKCE, Game Login Ticket, Game Gateway, World Registry and Game Session authority;
    - the game server must not introduce a second password or OAuth authority.

11. **Houses — provisional accepted invariant**
    - one house currently has one authoritative state for the whole logical world;
    - ownership, rent, access and items must not be copied per channel;
    - final house presence/entry topology is intentionally deferred;
    - a house may never become an implicit channel-switching mechanism.

### PROVEN — current client implementation facts

The existing Rust client is located in `blakinio/otclient/oteryn-client` and currently contains:

- Tokio as the asynchronous runtime;
- TCP transport through `tokio::net::TcpStream`;
- protocol-neutral transport/framing boundaries;
- `protocol-core`;
- a legacy `protocol-canary` crate;
- no implemented `protocol-oteryn` crate yet.

Tokio is not the wire protocol. It is the asynchronous runtime used to operate networking, tasks, queues and timeouts.

### UNKNOWN — decisions not yet accepted

The following remain undecided and must not be presented as final:

- whether the existing Rust client is moved into `Oteryn-v2` or remains in `blakinio/otclient`;
- exact root Cargo workspace membership and crate names;
- exact network transport security and serialization/IDL choices;
- exact simulation tick frequency and timer model;
- exact identifier representations;
- final database and data-access technology;
- lease ownership split between Platform and game server;
- content output formats and scripting runtime;
- first production ruleset and initial world name;
- quantitative performance, recovery and platform targets;
- final house topology.

### DERIVED — recommended defaults, not yet approved

These choices are recommended to prevent design paralysis, but each must be accepted by the appropriate ADR/contract before becoming normative:

```text
Repository shape:       one Oteryn-v2 Rust workspace for client, server and shared crates
Rust edition:           Rust 2024
Async runtime:          Tokio
Initial transport:      TCP
Transport security:     TLS 1.3 using rustls
Server topology:        modular monolith with extractable service boundaries
Persistence:            PostgreSQL
Internal messaging:     in-process queues plus transactional outbox
Initial ruleset:        modern-15
Content scripting:      Lua behind a restricted project-owned API
Channel execution:      one logical authoritative writer per channel
Initial CI targets:     Linux server and Windows client
```

## Required pre-implementation contracts

The following eight durable contracts are the minimum start gate. They may be separate ADRs/contracts or a tightly reviewed coordinated series, but each subject must receive an explicit decision.

### 1. Repository ownership and client migration ADR

Must decide:

- whether `blakinio/otclient/oteryn-client` moves into `blakinio/Oteryn-v2`;
- canonical ownership of client, server, shared types and `protocol-oteryn`;
- migration mechanism, source commit, history/provenance handling and rollback;
- status of `blakinio/otclient` after migration;
- how `protocol-canary` is removed or isolated from the target runtime.

Recommended target layout:

```text
Oteryn-v2/
  apps/client/
  services/game-server/
  crates/
  content/
  tests/
```

### 2. Workspace and dependency contract

Must define the first real crate/application boundaries and legal dependency directions.

Candidate initial modules:

```text
apps/client
services/game-server
crates/foundation
crates/domain-core
crates/protocol-core
crates/protocol-oteryn
crates/transport
crates/simulation-core
crates/world-runtime
crates/session-lease
crates/persistence-core
crates/persistence-postgres
crates/platform-contracts
crates/platform-client
crates/content-types
crates/content-runtime
crates/scripting
crates/test-support
```

Mandatory dependency rule:

- domain and simulation crates must not depend on Tokio, TCP, TLS, HTTP, SQL, a concrete database, Platform APIs, renderer code or a wire codec;
- infrastructure adapters may depend inward on domain contracts;
- protocol adapters translate typed domain/session messages but do not own gameplay rules.

### 3. `protocol-oteryn` v1 contract

Must define:

- Tokio as implementation runtime versus the actual transport and wire protocol;
- initial transport, encryption and certificate/key model;
- connection establishment and Game Session admission;
- frame header, endianness, message type namespace, payload length and hard limits;
- schema/IDL or explicit project-owned codec;
- protocol revision versus ruleset/content revision;
- capabilities and downgrade prevention;
- sequencing, acknowledgements where needed and error vocabulary;
- `command_id`, session generation, replay protection and idempotency;
- snapshot, delta and reconciliation contracts;
- reconnect/resume behaviour;
- compression rules and bounded allocation;
- golden fixtures and canonical schema ownership.

Do not use Rust memory layout or an unstable serializer as an implicit public wire contract.

### 4. Runtime execution contract

Must define:

- `NodeRuntime`, `WorldServices`, `ChannelRuntime` and `InstanceRuntime` responsibilities;
- modular-monolith starting topology;
- fixed or variable tick model and initial tick frequency;
- authoritative command ordering within one channel;
- timer and scheduler semantics;
- bounded command queues and overload behaviour;
- parallel pathfinding/AI/content work and safe return to the logical writer;
- deterministic replay requirements;
- channel startup, ready, full, draining, unhealthy, recovery and shutdown lifecycle;
- checkpoint and crash-recovery boundary.

### 5. Identity, admission, Game Session and lease contract

Must define the exact Platform-to-game-server boundary:

```text
Rust client
→ Oteryn Identity OAuth Authorization Code + PKCE
→ one-time Game Login Ticket
→ Oteryn Game Gateway
→ ticket redemption and World Registry routing
→ Game Session bound to account, character, world and channel
→ Rust game server admission
```

Minimum Game Session data to evaluate:

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

Must also decide token format, signature/key rotation, replay prevention, revocation, reconnect, capacity routing, lease acquisition/renewal/release and partial-failure behaviour.

### 6. Persistence v1 contract

Must define:

- selected database and migration mechanism;
- character state model and revision fencing;
- character lease storage and ownership;
- inventory/equipment and ground-item transfer transaction boundary;
- idempotency keys and duplicate-command handling;
- transaction/outbox boundaries for cross-module effects;
- market, guild, house and reward consistency classes;
- checkpoint strategy and maximum accepted progress loss;
- audit/journal scope without requiring full event sourcing for the whole game;
- backup, restore, schema compatibility and rollback expectations.

The initial recommendation is current-state tables plus revisions, idempotent commands, transactional outbox and append-only audit records for critical mutations—not full event sourcing of every simulation event.

### 7. Content migration and scripting contract

Must classify each source area from Otheryn as:

```text
COPY
CONVERT
REWRITE
REFERENCE_ONLY
REJECT
```

Must cover:

- maps;
- item/monster/spell catalogues;
- NPCs and spawns;
- quests and events;
- Lua/scripts;
- database/configuration data;
- formulas and behavioural fixtures;
- client assets and provenance.

Every migrated source must be pinned to an exact commit and receive license/provenance review. Conversion should use deterministic tooling and project-owned versioned output formats.

The scripting decision must define isolation, API surface, execution budget, error handling, persistence access, hot reload policy and explicit `WorldId`/`ChannelId`/`InstanceId` context. Scripts must not receive a global mutable `Game` object or direct SQL access.

### 8. Foundation vertical-slice programme

Must define an executable implementation order and acceptance evidence for the first playable end-to-end slice.

Recommended scenario:

1. Client authenticates through Platform.
2. Client receives a world and channel directory.
3. Client selects or accepts a recommended channel.
4. Gateway issues a bound Game Session.
5. Game server validates the session and acquires the character lease.
6. Character enters a minimal map.
7. Character movement is authoritative and visible.
8. Two clients can observe each other on the same channel.
9. Character attacks one monster.
10. Monster dies and creates one corpse/loot result.
11. Character picks up one item through a retry-safe transfer.
12. Character state is checkpointed and saved.
13. Character logs out safely.
14. Character logs into another channel.
15. Inventory and progression remain present.
16. A simultaneous second login of the same character is rejected.
17. Channel-local state remains isolated while world-shared state remains shared.

## Additional decisions required before the vertical slice expands

The following subjects may be developed while the first slice is being built, but must be explicit before their affected gameplay is enabled:

- identifier representation and visibility (`u64`, UUID or scoped identifiers);
- position, direction and entity-lifetime encoding;
- PvP, skull, frag and combat-lock world/channel scope;
- party membership versus shared-experience behaviour across channels;
- boss, raid, chest and daily reward anti-hopping policy;
- world communication and presence service boundary;
- event journal and checkpoint timing;
- metrics, tracing, log redaction and audit retention;
- updater, asset signing and release security;
- supported client platforms and server architectures;
- capacity, latency, reconnect, RPO and RTO targets.

## Explicitly deferred decisions

These subjects do not block creation of the initial workspace or foundation vertical slice, provided contracts preserve future extension points:

- final house presence/entry topology;
- live migration of an active channel;
- partitioning one channel across multiple nodes;
- QUIC support;
- cross-world chat or guilds;
- cross-world parties;
- hundreds of dynamically created channels;
- full dungeon/arena instance programme;
- complete market implementation;
- all classic rulesets;
- final launcher/updater implementation;
- public mod ecosystem;
- advanced client prediction;
- extraction of world services into independent deployable microservices.

## Non-functional targets to declare

The contract programme must provide initial measurable targets rather than leaving every queue and timeout unbounded:

- expected concurrent players per channel;
- expected channels per node;
- channel tick target and latency percentile;
- maximum frame and message sizes;
- inbound/outbound queue capacities;
- Game Session and reconnect windows;
- character lease timings;
- checkpoint interval and maximum accepted progress loss;
- supported server OS/architecture;
- supported client OS/platforms;
- observability and log-retention requirements;
- backup/RPO/RTO expectations for the first non-development environment.

The first targets may be conservative and revised through evidence. They must still be explicit.

## Start gate

Do **not** create the real root Rust workspace merely to populate the planned directory tree.

Workspace creation is authorized only after contracts 1–5 have accepted enough detail to prevent incorrect repository ownership, dependency direction, protocol framing, runtime execution or session admission. Contracts 6–8 may be completed in the same coordinated foundation programme and must be accepted before the vertical slice performs durable gameplay mutations.

A minimal non-gameplay experiment may be created only when it is owned by a bounded task and cannot be mistaken for the accepted production architecture.

## Acceptance criteria

- [ ] Repository ownership and client migration ADR accepted.
- [ ] Workspace and dependency contract accepted.
- [ ] `protocol-oteryn` v1 contract accepted.
- [ ] Runtime execution contract accepted.
- [ ] Identity/Game Session/lease contract accepted across relevant repositories.
- [ ] Persistence v1 contract accepted before durable gameplay mutation implementation.
- [ ] Content migration and scripting contract accepted before broad content import.
- [ ] Foundation vertical-slice programme accepted with named acceptance evidence.
- [ ] Each accepted contract identifies canonical owner, producers, consumers and exact revisions.
- [ ] Cross-repository work uses separate authorized tasks/branches/PRs.
- [ ] Only then is the real root Cargo workspace created.

## Excluded scope

This task does not implement the Rust workspace, move client code, create protocol crates, modify Platform, import content, select final house topology or claim that any target module already exists.

## Implementation / findings

- Architecture foundation exists and is accepted.
- Agent governance exists, passes its exact-head validator and provides durable task lifecycle rules.
- No active overlapping task or PR was found when this checkpoint was prepared.
- The next programme must begin with a repository-ownership/client-migration decision because it determines whether shared client/server crates can live in one workspace.

## Validation

### Focused

- method: review against accepted ADR-0001, multichannel scope matrix, repository map, project lanes and cross-repository policy
- result: checkpoint prepared; exact-head workflow pending on recording PR

### Component/integration

- result: `NOT_APPLICABLE` — no executable runtime or public code changed

### E2E

- result: `NOT_APPLICABLE` — this is a durable architecture handoff only

### Exact-head CI

- head: pending recording PR
- workflow/run: `Agent governance`, pending
- result: pending

## Independent audit

- method/auditor: pending PR review
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: one active task record expected
- unresolved review threads: pending
- related/superseded PRs: none discovered
- merge commit/result: pending
- ownership release: not applicable; future task is unassigned and owns no paths

## Context checkpoint

```yaml
last_progress: Durable pre-implementation state, accepted decisions, unknowns, recommended defaults, eight required contracts and the vertical-slice start gate were captured in this task record.
status: ready
branch: null
head_sha: null
pr: null
owned_paths: []
public_contracts:
  - docs/architecture/ADR-0001-native-rust-multichannel-platform.md
  - docs/architecture/MULTICHANNEL_SYSTEM_SCOPE_MATRIX.md
  - docs/architecture/OTHERYN_REFERENCE_MIGRATION_PLAN.md
validation_state: Recording PR must pass Agent governance on its exact head; no runtime validation applies.
audit_state: Recording PR review pending.
e2e_state: NOT_APPLICABLE because no runtime implementation exists in this task.
ci_generation: null
run_ids: []
counters:
  ci_checks_for_current_head: 0
  terminal_ci_checks_for_current_generation: 0
  unchanged_state_checks: 0
  identical_failure_retries: 0
  repair_cycles_for_current_gate: 0
  stall_warnings: 0
blocker: null
next_action: Draft and submit the Repository Ownership and Client Migration ADR deciding whether blakinio/otclient/oteryn-client becomes apps/client in blakinio/Oteryn-v2.
```
