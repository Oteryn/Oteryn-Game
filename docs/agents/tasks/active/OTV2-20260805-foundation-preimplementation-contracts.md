# OTV2-20260805-foundation-preimplementation-contracts

```yaml
task_id: OTV2-20260805-foundation-preimplementation-contracts
title: Coordinate Oteryn v2 foundation contracts and staged implementation gates
mode: COORDINATE
status: ready
repository: blakinio/Oteryn-v2
base_branch: main
branch: null
pr: null
base_sha: bd87792c92e26835d44c633a6064808f487a58a2
head_sha: null
owner: unassigned
created_at: 2026-08-05T08:49:00+02:00
updated_at: 2026-08-05T12:42:00+02:00
execution_budget_minutes: 120
large_budget_reason: The programme checkpoint spans repository ownership, protocol, runtime, persistence, content, Platform integration, client migration and the global product architecture horizon, while each executable package remains separately bounded.
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
  - canonical root Cargo workspace bootstrap until FND-01 is accepted
  - canonical protocol implementation until FND-02 is accepted
  - authoritative runtime implementation until FND-03 is accepted
  - production admission and character lease implementation until FND-04 is accepted
  - authoritative durable gameplay mutation until DUR-01 through DUR-03 are accepted
  - broad content import and durable scripting until DUR-04 is accepted
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories:
  - blakinio/Oteryn-Platform
  - blakinio/Otheryn
  - blakinio/otclient
```

## Outcome

Coordinate the minimum contracts and staged implementation gates required for the Oteryn v2 Rust workspace and first durable vertical slice without locking the project into an incorrect repository boundary, dependency graph, protocol, runtime topology, admission model, persistence design, world/content model or client migration path.

This file is the non-owning programme checkpoint. It preserves accepted state, dependencies and exactly one next action. It must not claim owned paths or act as the implementation task for all contracts.

Every substantial gate requires a separate package task with its own owner, paths, branch, PR, validation, audit, merge and archive lifecycle.

The exact ordered gates are maintained in `docs/architecture/FOUNDATION_DECISION_BACKLOG.md`. The complete wider decision horizon is maintained in `docs/architecture/GLOBAL_ARCHITECTURE_DECISION_REGISTER.md`. The self-contained coordinator prompt is `docs/agents/prompts/OTV2_GLOBAL_ARCHITECTURE_DECISION_COORDINATOR.md`.

The complete original 2026-08-05 handoff remains preserved unchanged at `docs/agents/evidence/OTV2-20260805-foundation-original-handoff.md` for traceability.

## Current repository state

### PROVEN

- At this checkpoint the repository contains architecture, governance and tools, but no accepted canonical root Cargo workspace.
- No complete `protocol-oteryn` Rust client-to-server gameplay implementation or native gameplay E2E is proven.
- The existing Rust client remains in `blakinio/otclient/oteryn-client` until `VSL-02` pins and moves it.
- Oteryn Platform contains Identity, World Registry, Game Login Ticket and a standalone Go Game Gateway.
- Existing producer-side native work does not prove the complete target runtime.
- ADR-0001 through ADR-0005 and their lifecycle closeouts are merged on `main`.

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

## Stable gate IDs

- `FND-01` — Workspace and Dependency Contract.
- `FND-02` — `protocol-oteryn` v1 Contract.
- `FND-03` — Runtime Execution Contract.
- `FND-04` — Identity, Game Session, Admission and Character Lease Contract.
- `DUR-01` — Identifier Contract.
- `DUR-02` — Persistence v1 Contract.
- `DUR-03` — Item Transaction and Anti-Duplication Contract.
- `DUR-04` — Content, World Detail and Scripting Contract.
- `VSL-01` — Foundation Vertical-Slice Programme.
- `VSL-02` — Exact Rust Client Migration Contract.

## Progressive implementation gates

### Before `FND-01`

The canonical root Cargo workspace remains blocked. Read-only discovery and architecture work are allowed. A disposable spike must be isolated, reversible and explicitly non-canonical.

### After `FND-01`

A separately authorized minimal workspace-bootstrap task may create the root Cargo workspace, the smallest immediately consumed members, toolchain/CI metadata and executable dependency-boundary checks.

The wider candidate crate list is a capability horizon, not an initial checklist. Empty placeholder crates and speculative abstractions are prohibited.

This bootstrap does not authorize canonical protocol, authoritative runtime, production admission/lease, durable gameplay, broad client migration or broad content import.

### Layer gates

- `FND-02` gates canonical protocol schemas/codecs and production compatibility claims.
- `FND-03` gates authoritative runtime ordering, lifecycle and recovery.
- `FND-04` gates production Game Session validation, admission and character lease behavior.
- `DUR-01` through `DUR-03` gate authoritative durable character, item and currency mutation.
- `DUR-04` gates broad content import and durable scripting.
- `VSL-02` gates moving the Rust client source.
- `VSL-01` gates completion of the native foundation vertical slice.

## Remaining foundation contracts

### `FND-01` — Workspace and Dependency Contract

- exact minimal initial applications, services and crates;
- immediate consumer and observable acceptance for every member;
- legal dependency directions and forbidden edges;
- canonical ownership of identifiers, domain types, protocol schemas, world/content schemas and fixtures;
- Rust edition/toolchain, feature and dependency policy;
- initial target platforms and CI matrix;
- executable dependency-graph enforcement;
- criteria for later adding, splitting or merging crates.

### `FND-02` — `protocol-oteryn` v1 Contract

- reconcile, revise or explicitly supersede the exact current Platform native contract;
- transport, TLS/ALPN, framing, stable schema/IDL and hard limits;
- revisions, capabilities and downgrade prevention;
- sequencing, command IDs, replay/idempotency;
- snapshots, deltas, reconnect/reconciliation and golden fixtures.

### `FND-03` — Runtime Execution Contract

- node/world/channel/instance responsibilities;
- tick and timer model;
- command ordering and bounded queues;
- overload/backpressure;
- parallel work and safe return to the channel writer;
- lifecycle, checkpoint, replay and recovery semantics.

### `FND-04` — Identity, Game Session, Admission and Character Lease Contract

- token/session format, issuer/audience and key rotation;
- expiry, consume/replay prevention and revocation;
- reconnect, capacity routing and channel/revision binding;
- session generation and duplicate-login fencing;
- lease owner, storage and timings;
- partition and dependency failure behavior.

The exact questions, gates and order are maintained in `FOUNDATION_DECISION_BACKLOG.md`.

## Required before durable gameplay mutation

- `DUR-01` identifier representation and visibility contract;
- `DUR-02` detailed Persistence v1 contract on PostgreSQL;
- `DUR-03` item transaction and anti-duplication invariants;
- `DUR-04` content/world-detail/scripting contract before broad import or durable scripts;
- `VSL-01` foundation vertical-slice programme with named observable evidence.

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

The global register preserves and stages movement, combat, inventory/economy, rulesets/scripting, client migration/architecture, World Project/Bundle/Registry/Studio, raids/houses/social systems, security, updater, deployment, observability, testing, quantitative performance targets and product milestones.

These subjects must not be forgotten, but they must not be prematurely frozen when they do not block the current stage.

## Explicitly deferred

These do not block initial workspace bootstrap or the foundation vertical slice when safe extension points remain:

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

## Package ownership model

- This programme checkpoint remains unassigned and owns no paths.
- Every substantial gate uses a separate task containing its stable gate ID where practical.
- Each package owns only its declared files and public contracts.
- Each package has one dedicated branch and PR.
- Package completion requires focused validation, independent audit, exact-head CI, squash merge, archive and ownership release.
- No package may expand its authority through governance edited on its own unmerged branch.

## Acceptance criteria

- [x] Repository ownership and client migration direction accepted in ADR-0002.
- [ ] `FND-01` Workspace and Dependency Contract accepted.
- [ ] A separate minimal workspace-bootstrap task is authorized after `FND-01`; no other foundation contract blocks bootstrap.
- [ ] Initial workspace members require an immediate consumer and observable acceptance; no empty layering crates.
- [ ] Dependency directions become machine-enforced after bootstrap.
- [ ] `FND-02` accepted and reconciled with Platform before canonical protocol implementation.
- [ ] `FND-03` accepted before authoritative runtime implementation.
- [ ] `FND-04` accepted before production admission/lease implementation.
- [x] PostgreSQL and separate Platform/game ownership accepted in ADR-0004.
- [ ] `DUR-01` accepted before schema and wire identities are frozen.
- [ ] `DUR-02` accepted before durable gameplay mutations.
- [ ] `DUR-03` accepted before durable item/currency mutation.
- [x] Native world/content/editor direction accepted in ADR-0005.
- [ ] `DUR-04` accepted before broad content import or durable scripting.
- [ ] `VSL-01` accepted with named E2E evidence.
- [ ] `VSL-02` pins source SHA, provenance mechanism and rollback before migration.
- [ ] Each contract identifies canonical owner, producers, consumers and exact revisions.
- [ ] Cross-repository changes use separate authorized tasks/branches/PRs and explicit rollout order.
- [ ] Every executable package is separately authorized and bounded.

## Excluded scope

This programme checkpoint does not itself:

- create the Rust workspace;
- move client code;
- implement protocol crates or runtime;
- modify external repositories;
- migrate Platform to PostgreSQL;
- provision a database;
- import content or proprietary assets;
- select final house topology;
- claim any native E2E capability already exists;
- own files on behalf of future packages.

## Implementation / findings

- ADR-0002 resolved canonical repository ownership and client migration direction.
- ADR-0003 resolved the Platform Identity/Game Gateway boundary and retained the initial Go Gateway.
- ADR-0004 selected PostgreSQL and separate Platform/game database ownership.
- ADR-0005 accepted the native world/content model, Oteryn Studio and bounded legacy conversion direction.
- `FOUNDATION_DECISION_BACKLOG.md` records stable IDs, progressive gates and the non-owning programme model.
- `GLOBAL_ARCHITECTURE_DECISION_REGISTER.md` records the staged complete project decision horizon.
- `OTV2_GLOBAL_ARCHITECTURE_DECISION_COORDINATOR.md` supports explicit read-only analysis and bounded execution.

## Validation

### Focused

- method: architecture/governance document validation on each package PR
- result: pending per future package

### Component/integration

- result: `NOT_APPLICABLE` until executable packages are introduced

### E2E

- result: `BLOCKED` until `VSL-01` is implemented

### Exact-head CI

- head: pending each package PR
- workflow: `Agent governance` plus later retained Rust/component workflows
- result: pending

## Independent audit

Every package PR must challenge omitted boundaries, circular dependencies, placeholder crates, duplicate authority, unsafe concurrency/replay/recovery, item duplication, cross-repository rollout conflicts, unsupported runtime claims, premature freezing and stale decisions.

## Context checkpoint

```yaml
last_progress: ADR-0001 through ADR-0005 are accepted; stable foundation gates, progressive implementation policy and the non-owning programme model are durable continuation sources.
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
validation_state: Each package requires exact-head Agent governance and full-diff audit; dependency boundaries become executable checks after workspace bootstrap.
audit_state: Future package audits pending.
e2e_state: BLOCKED until VSL-01 is implemented.
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
next_action: Execute the global architecture coordinator prompt and draft, audit, accept, merge and archive FND-01 — the Workspace and Dependency Contract.
```