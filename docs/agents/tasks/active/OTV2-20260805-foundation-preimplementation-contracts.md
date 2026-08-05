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
base_sha: 98b7193f285f64268bee16969a0d2d1c5b026132
head_sha: null
owner: unassigned
created_at: 2026-08-05T08:49:00+02:00
updated_at: 2026-08-05T12:04:00+02:00
execution_budget_minutes: 120
large_budget_reason: The contract programme spans repository ownership, protocol, runtime, persistence, content, Platform integration, client migration and the global product architecture horizon.
owned_paths: []
public_contracts:
  - docs/architecture/ADR-0001-native-rust-multichannel-platform.md
  - docs/architecture/ADR-0002-repository-ownership-and-client-migration.md
  - docs/architecture/ADR-0003-platform-identity-game-gateway-and-admission-boundary.md
  - docs/architecture/ADR-0004-postgresql-and-data-ownership.md
  - docs/architecture/ADR-0005-native-world-format-and-oteryn-studio.md
  - docs/architecture/FOUNDATION_DECISION_BACKLOG.md
  - docs/architecture/GLOBAL_ARCHITECTURE_DECISION_REGISTER.md
  - docs/architecture/MULTICHANNEL_SYSTEM_SCOPE_MATRIX.md
  - docs/architecture/OTHERYN_REFERENCE_MIGRATION_PLAN.md
continuation_prompt: docs/agents/prompts/OTV2_GLOBAL_ARCHITECTURE_DECISION_COORDINATOR.md
depends_on:
  - ADR-0001 through ADR-0005 accepted foundation
blocks:
  - creation of the real root Cargo workspace until blocking contracts are accepted
  - implementation of protocol-oteryn
  - migration of the existing Rust client
  - implementation of the authoritative Rust game server
  - authoritative durable gameplay mutation until identifier/persistence/item invariants are accepted
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories:
  - blakinio/Oteryn-Platform
  - blakinio/Otheryn
  - blakinio/otclient
```

## Outcome

Accept the minimum contracts required to create the real Oteryn v2 Rust workspace and first durable vertical slice without locking the project into an incorrect repository boundary, dependency graph, protocol, runtime topology, admission model, persistence design, world/content model or client migration path.

This task is the canonical continuation point. The exact ordered foundation gates are maintained in `docs/architecture/FOUNDATION_DECISION_BACKLOG.md`. The complete wider decision horizon is maintained in `docs/architecture/GLOBAL_ARCHITECTURE_DECISION_REGISTER.md`. The self-contained execution prompt is `docs/agents/prompts/OTV2_GLOBAL_ARCHITECTURE_DECISION_COORDINATOR.md`.

The complete original 2026-08-05 handoff remains preserved unchanged at `docs/agents/evidence/OTV2-20260805-foundation-original-handoff.md` for traceability.

## Current repository state

### PROVEN

- At this checkpoint the repository contains architecture, governance and tools, but no accepted real root Cargo workspace.
- No complete `protocol-oteryn` Rust client-to-server gameplay implementation or native gameplay E2E is proven.
- The existing Rust client remains in `blakinio/otclient/oteryn-client` until a dedicated migration programme pins and moves it.
- Oteryn Platform contains Identity, World Registry, Game Login Ticket and a standalone Go Game Gateway.
- Existing producer-side native work does not prove the complete target runtime.
- ADR-0005 and its lifecycle closeout are merged on `main`.

Every replacement agent must verify this baseline against the live default branch before repeating it.

### ACCEPTED

1. **Native Rust target**
   - Rust client and authoritative Rust server;
   - one project-owned `protocol-oteryn`;
   - multichannel-first world model.

2. **Repository ownership**
   - the Rust client will move into `blakinio/Oteryn-v2`;
   - client, server and shared Rust crates will use one canonical repository/workspace;
   - `protocol-canary` is not part of the target runtime.

3. **Platform boundary**
   - Identity, OAuth/PKCE, MFA, Game Login Ticket, Game Gateway and World Registry remain in `blakinio/Oteryn-Platform`;
   - no second classic login server or credential authority is introduced;
   - the initial Game Gateway remains implemented in Go;
   - the Rust game server validates the bounded Game Session and owns gameplay admission/runtime.

4. **Persistence direction**
   - PostgreSQL is selected;
   - Platform and game use separate logical databases, owners, credentials and migration histories;
   - Redis is non-authoritative.

5. **Native world/content/editor direction**
   - Oteryn owns a greenfield native world/content format;
   - OTBM and historical formats are bounded conversion inputs rather than the canonical runtime model;
   - Oteryn Studio is the integrated map, asset and content authoring direction;
   - semantic `Area`/`Subarea` geography is separate from technical `Region`/`Chunk` partitioning;
   - dynamic encounter execution uses validated `EncounterZone`/`RaidCell`/`RaidAnchor` scopes.

Canonical evidence: ADR-0001 through ADR-0005.

## Remaining blocking contracts

The real Cargo workspace must not be created until these contracts are accepted with enough detail to prevent an incorrect foundation.

1. **Workspace and Dependency Contract**
   - exact initial applications, services and crates;
   - legal dependency directions and forbidden edges;
   - canonical ownership of identifiers, domain types, protocol schemas, world/content schemas and fixtures;
   - Rust edition/toolchain, feature and dependency policy;
   - initial target platforms and CI matrix.

2. **`protocol-oteryn` v1 Contract**
   - reconcile, revise or explicitly supersede the exact current Platform native contract;
   - transport, TLS/ALPN, framing, stable schema/IDL and hard limits;
   - revisions, capabilities and downgrade prevention;
   - sequencing, command IDs, replay/idempotency;
   - snapshots, deltas, reconnect/reconciliation and golden fixtures.

3. **Runtime Execution Contract**
   - node/world/channel/instance responsibilities;
   - tick and timer model;
   - command ordering and bounded queues;
   - overload/backpressure;
   - parallel work and safe return to the channel writer;
   - lifecycle, checkpoint, replay and recovery semantics.

4. **Identity, Game Session, Admission and Character Lease Contract**
   - token/session format, issuer/audience and key rotation;
   - expiry, consume/replay prevention and revocation;
   - reconnect, capacity routing and channel/revision binding;
   - session generation and duplicate-login fencing;
   - lease owner, storage and timings;
   - partition and dependency failure behavior.

The exact questions, gates and order are maintained in `FOUNDATION_DECISION_BACKLOG.md`.

## Required before durable gameplay mutation

- identifier representation and visibility contract;
- detailed Persistence v1 contract on PostgreSQL;
- item transaction and anti-duplication invariants;
- remaining content/world-detail/scripting contract before broad import or durable scripts;
- foundation vertical-slice programme with named observable evidence.

## Foundation vertical slice

The initial programme must prove at least:

1. Platform authentication and Gateway-issued Game Session;
2. Rust game-server admission and character lease;
3. entry to a minimal native map;
4. authoritative movement and two-player visibility on one channel;
5. one monster combat/death/loot path;
6. retry-safe item pickup;
7. durable checkpoint and safe logout;
8. relog to another channel with shared character state preserved;
9. rejection of a simultaneous second session;
10. isolation of channel-local state and preservation of world-shared state.

## Wider global decision horizon

The global register preserves and stages additional project domains including:

- movement, collision, pathfinding and visibility;
- combat, conditions, death, loot and attribution;
- inventory, containers, trade, bank, depot, market and economy;
- rulesets and scripting;
- exact client migration and client architecture;
- World Project, World Bundle, Content Registry and Oteryn Studio details;
- raids, bosses, houses, party, guild, chat and presence;
- security, administration, updater, deployment, observability, testing and quantitative performance targets;
- Foundation, Playable Alpha, Beta and release scope.

These subjects must not be forgotten, but they must not be prematurely frozen when they do not block the current stage.

## Explicitly deferred

These do not block the initial workspace or vertical slice when safe extension points remain:

- final house presence/entry topology;
- live channel migration;
- partitioning one channel across multiple nodes;
- QUIC;
- cross-world chat/guilds/parties;
- hundreds of dynamic channels;
- complete instance and market programmes;
- all classic rulesets;
- final launcher/updater;
- public mod ecosystem;
- advanced client prediction;
- extraction of world services into independently deployable microservices.

## Acceptance criteria

- [x] Repository ownership and client migration direction accepted in ADR-0002.
- [ ] Exact client migration source SHA, history/provenance mechanism and rollback accepted before migration.
- [ ] Workspace and Dependency Contract accepted.
- [ ] `protocol-oteryn` v1 accepted and reconciled with Platform.
- [ ] Runtime Execution Contract accepted.
- [ ] Identity/Game Session/admission/lease contract accepted across relevant repositories.
- [x] PostgreSQL and separate Platform/game ownership accepted in ADR-0004.
- [ ] Identifier Contract accepted before schema and wire types are frozen.
- [ ] Persistence v1 accepted before durable gameplay mutations.
- [ ] Item transaction and anti-duplication invariants accepted before durable item/currency mutation.
- [x] Native world/content/editor direction accepted in ADR-0005.
- [ ] Remaining concrete world/content migration, asset-rights and scripting contracts accepted before broad content import.
- [ ] Foundation Vertical-Slice Programme accepted with named E2E evidence.
- [ ] Each contract identifies canonical owner, producers, consumers and exact revisions.
- [ ] Cross-repository changes use separate authorized tasks/branches/PRs and explicit rollout order.
- [ ] Only after blocking gates pass is implementation authorized by a separate programme.

## Excluded scope

This task does not itself:

- create the Rust workspace;
- move client code;
- implement protocol crates or runtime;
- modify external repositories;
- migrate Platform to PostgreSQL;
- provision a database;
- import content or proprietary assets;
- select final house topology;
- claim any native E2E capability already exists.

## Implementation / findings

- ADR-0002 resolved canonical repository ownership and client migration direction.
- ADR-0003 resolved the Platform Identity/Game Gateway boundary and retained the initial Go Gateway.
- ADR-0004 selected PostgreSQL and separate Platform/game database ownership.
- ADR-0005 accepted the native world/content model, Oteryn Studio and bounded legacy conversion direction.
- `FOUNDATION_DECISION_BACKLOG.md` records the ordered immediate gates.
- `GLOBAL_ARCHITECTURE_DECISION_REGISTER.md` records the staged complete project decision horizon.
- `OTV2_GLOBAL_ARCHITECTURE_DECISION_COORDINATOR.md` is the durable execution prompt for replacement agents.

## Validation

### Focused

- method: architecture/governance document validation on each contract PR
- result: pending per future contract

### Component/integration

- result: `NOT_APPLICABLE` until executable contracts are implemented

### E2E

- result: `BLOCKED` until the foundation vertical slice is implemented

### Exact-head CI

- head: pending each contract PR
- workflow: `Agent governance` plus later Rust/component workflows
- result: pending

## Independent audit

Every contract PR must challenge:

- omitted boundaries or layers;
- circular or convenience dependencies;
- duplicate authority or source of truth;
- unsafe concurrency, replay, stale-writer or recovery assumptions;
- item/currency duplication paths;
- cross-repository rollout conflicts;
- unsupported runtime claims;
- premature freezing of deferred systems;
- stale decisions that conflict with accepted ADRs.

## Context checkpoint

```yaml
last_progress: ADR-0001 through ADR-0005 are accepted; the global decision horizon and autonomous coordinator prompt are now durable continuation sources.
status: ready
branch: null
head_sha: null
pr: null
owned_paths: []
public_contracts:
  - docs/architecture/ADR-0001-native-rust-multichannel-platform.md
  - docs/architecture/ADR-0002-repository-ownership-and-client-migration.md
  - docs/architecture/ADR-0003-platform-identity-game-gateway-and-admission-boundary.md
  - docs/architecture/ADR-0004-postgresql-and-data-ownership.md
  - docs/architecture/ADR-0005-native-world-format-and-oteryn-studio.md
  - docs/architecture/FOUNDATION_DECISION_BACKLOG.md
  - docs/architecture/GLOBAL_ARCHITECTURE_DECISION_REGISTER.md
continuation_prompt: docs/agents/prompts/OTV2_GLOBAL_ARCHITECTURE_DECISION_COORDINATOR.md
validation_state: Each future contract package requires exact-head Agent governance and full-diff audit.
audit_state: Future contract audits pending.
e2e_state: BLOCKED until the vertical slice is implemented.
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
next_action: Execute the global architecture coordinator prompt and draft, audit, accept, merge and archive the Workspace and Dependency Contract.
```
