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
base_sha: 083d68308549d7fa7e486a464f279e42b2f6a96e
head_sha: null
owner: unassigned
created_at: 2026-08-05T08:49:00+02:00
updated_at: 2026-08-05T10:11:00+02:00
execution_budget_minutes: 120
large_budget_reason: The contract programme spans repository ownership, protocol, runtime, persistence, content, Platform integration and a cross-repository migration boundary.
owned_paths: []
public_contracts:
  - docs/architecture/ADR-0001-native-rust-multichannel-platform.md
  - docs/architecture/ADR-0002-repository-ownership-and-client-migration.md
  - docs/architecture/ADR-0003-platform-identity-game-gateway-and-admission-boundary.md
  - docs/architecture/ADR-0004-postgresql-and-data-ownership.md
  - docs/architecture/FOUNDATION_DECISION_BACKLOG.md
  - docs/architecture/MULTICHANNEL_SYSTEM_SCOPE_MATRIX.md
  - docs/architecture/OTHERYN_REFERENCE_MIGRATION_PLAN.md
depends_on:
  - ADR-0001 through ADR-0004 accepted foundation
blocks:
  - creation of the real root Cargo workspace until blocking contracts are accepted
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

Accept the minimum contracts required to create the real Oteryn v2 Rust workspace and first durable vertical slice without locking the project into an incorrect repository boundary, dependency graph, protocol, runtime topology, admission model or persistence design.

This task is the canonical continuation point. The ordered current backlog is `docs/architecture/FOUNDATION_DECISION_BACKLOG.md`. The complete original 2026-08-05 handoff is preserved unchanged at `docs/agents/evidence/OTV2-20260805-foundation-original-handoff.md` for traceability.

## Current repository state

### PROVEN

- The repository contains architecture, governance and tools only.
- No root Cargo workspace, Rust game server, `protocol-oteryn` crate or native client/server gameplay E2E exists yet.
- The existing Rust client remains in `blakinio/otclient/oteryn-client` until a dedicated migration programme moves it.
- Oteryn Platform contains Identity, World Registry, Game Login Ticket and a standalone Go Game Gateway.
- Existing native producer-side work does not prove a complete native Rust client-to-server runtime.

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

Canonical evidence: ADR-0001 through ADR-0004.

## Remaining blocking contracts

The real Cargo workspace must not be created until these are accepted with enough detail:

1. **Workspace and dependency contract**
   - exact initial applications/crates;
   - dependency directions;
   - Rust edition/toolchain and feature policy;
   - client/server/shared ownership;
   - CI targets.

2. **`protocol-oteryn` v1 contract**
   - reconcile, revise or supersede the existing Platform native contract;
   - transport, TLS/ALPN, framing, stable schema/IDL and hard limits;
   - revisions/capabilities/downgrade prevention;
   - sequencing, command IDs, replay/idempotency;
   - snapshots, deltas, reconnect/reconciliation and golden fixtures.

3. **Runtime execution contract**
   - node/world/channel/instance responsibilities;
   - tick and timer model;
   - command ordering and bounded queues;
   - overload/backpressure;
   - parallel work and safe return to the channel writer;
   - lifecycle, checkpoint and recovery semantics.

4. **Identity, Game Session, admission and lease contract**
   - token format, issuer/audience and key rotation;
   - expiry, atomic consume/replay prevention and revocation;
   - reconnect, capacity routing and channel binding;
   - session generation and duplicate-login fencing;
   - lease owner/storage/timings;
   - partition and dependency failure behaviour.

The exact ordered questions and start gates are maintained in `docs/architecture/FOUNDATION_DECISION_BACKLOG.md`.

## Required before durable gameplay mutation

- identifier representation and visibility contract;
- detailed Persistence v1 contract on PostgreSQL;
- content migration and scripting contract before broad import;
- foundation vertical-slice programme with named observable evidence.

## Foundation vertical slice

The initial programme must prove at least:

1. Platform authentication and Gateway-issued Game Session;
2. Rust game-server admission and character lease;
3. entry to a minimal map;
4. authoritative movement and two-player visibility on one channel;
5. one monster combat/death/loot path;
6. retry-safe item pickup;
7. durable checkpoint and safe logout;
8. relog to another channel with shared character state preserved;
9. rejection of a simultaneous second session;
10. isolation of channel-local state and preservation of world-shared state.

## Explicitly deferred

These do not block the initial workspace or vertical slice when safe extension points remain:

- final house presence/entry topology;
- live channel migration;
- partitioning one channel across multiple nodes;
- QUIC;
- cross-world chat/guilds/parties;
- hundreds of dynamic channels;
- complete instance, market and classic-ruleset programmes;
- final launcher/updater;
- public mod ecosystem;
- advanced client prediction;
- extraction of world services into deployable microservices.

## Acceptance criteria

- [x] Repository ownership and client migration direction accepted in ADR-0002.
- [ ] Exact client migration source SHA, history/provenance mechanism and rollback accepted before migration.
- [ ] Workspace and dependency contract accepted.
- [ ] `protocol-oteryn` v1 contract accepted and reconciled with Platform.
- [ ] Runtime execution contract accepted.
- [ ] Identity/Game Session/admission/lease contract accepted across relevant repositories.
- [x] PostgreSQL and separate Platform/game ownership accepted in ADR-0004.
- [ ] Detailed Persistence v1 contract accepted before durable gameplay mutations.
- [ ] Identifier contract accepted before schema and wire types are frozen.
- [ ] Content migration and scripting contract accepted before broad content import.
- [ ] Foundation vertical-slice programme accepted with named E2E evidence.
- [ ] Each contract identifies canonical owner, producers, consumers and exact revisions.
- [ ] Cross-repository changes use separate authorized tasks/branches/PRs and explicit rollout order.
- [ ] Only after the blocking gates pass is the real Cargo workspace created.

## Excluded scope

This task does not itself:

- create the Rust workspace;
- move client code;
- implement protocol crates or runtime;
- modify external repositories;
- migrate Platform to PostgreSQL;
- provision a database;
- import content;
- select final house topology;
- claim any native E2E capability already exists.

## Implementation / findings

- ADR-0002 resolved canonical repository ownership and client migration direction.
- ADR-0003 resolved the Platform Identity/Game Gateway boundary and retained the initial Go Gateway.
- ADR-0004 selected PostgreSQL and separate Platform/game database ownership.
- `FOUNDATION_DECISION_BACKLOG.md` records the remaining decisions, start gates and recommended order.
- The original detailed handoff is preserved unchanged under `docs/agents/evidence/`.

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
- duplicate authority or source of truth;
- unsafe concurrency, replay or recovery assumptions;
- cross-repository rollout conflicts;
- unsupported runtime claims;
- stale decisions that conflict with accepted ADRs.

## Context checkpoint

```yaml
last_progress: ADR-0002 through ADR-0004 resolved repository ownership, Platform/Game Gateway boundaries and PostgreSQL direction; the remaining decisions were ordered in FOUNDATION_DECISION_BACKLOG.md.
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
  - docs/architecture/FOUNDATION_DECISION_BACKLOG.md
validation_state: Current decision register must pass Agent governance on its recording PR.
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
next_action: Draft and accept the Workspace and Dependency Contract.
```
