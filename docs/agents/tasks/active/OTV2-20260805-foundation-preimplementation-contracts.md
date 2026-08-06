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
base_sha: 1b91f9aa0abda8fccb0389972636708c4301ef88
head_sha: null
owner: unassigned
created_at: 2026-08-05T08:49:00+02:00
updated_at: 2026-08-06T13:30:00+02:00
execution_budget_minutes: 120
large_budget_reason: The programme checkpoint spans repository ownership, protocol, runtime, persistence, content, Platform integration, client migration and the global product architecture horizon, while each executable package remains separately bounded.
owned_paths: []
public_contracts:
  - docs/architecture/ADR-0001-native-rust-multichannel-platform.md
  - docs/architecture/ADR-0002-repository-ownership-and-client-migration.md
  - docs/architecture/ADR-0003-platform-identity-game-gateway-and-admission-boundary.md
  - docs/architecture/ADR-0004-postgresql-and-data-ownership.md
  - docs/architecture/ADR-0005-native-world-format-and-oteryn-studio.md
  - docs/architecture/ADR-0006-game-intelligence-analytics-and-audit.md
  - docs/architecture/FOUNDATION_DECISION_BACKLOG.md
  - docs/architecture/GLOBAL_ARCHITECTURE_DECISION_REGISTER.md
  - docs/architecture/GAMEPLAY_AND_PRODUCT_ARCHITECTURE_HORIZON.md
  - docs/contracts/CROSS_REPOSITORY_CONTRACT_LOCK.json
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - docs/contracts/FOUNDATION_ERROR_VOCABULARY.md
  - docs/contracts/FOUNDATION_FAILURE_SCENARIOS.md
  - docs/architecture/MULTICHANNEL_SYSTEM_SCOPE_MATRIX.md
  - docs/architecture/OTHERYN_REFERENCE_MIGRATION_PLAN.md
continuation_prompt: docs/agents/prompts/OTV2_GLOBAL_ARCHITECTURE_DECISION_COORDINATOR.md
depends_on:
  - ADR-0001 through ADR-0011 accepted foundation
  - FND-01 and VSL-02 accepted and archived
  - atomic destination cutover merged as 78988f72a80cc904aa9176ae850c50d4efa0b0f0
blocks:
  - FND-ID-01 start until the required source-only blakinio/otclient historical marker is merged and verified
  - protocol and Game Session identifier meanings until FND-ID-01 is accepted
  - canonical protocol implementation until FND-02 is accepted
  - authoritative runtime implementation until FND-03 is accepted
  - production admission and character lease implementation until FND-04 is accepted
  - authoritative durable gameplay mutation until DUR-01 through DUR-03 are accepted
  - final transactional event/outbox/audit boundaries until ANL-01 is accepted
  - production-grade balance/world and economy/security analytics claims until ANL-02 and ANL-03 are accepted
  - read-only AI investigation until ANL-04 is accepted
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

- `main` at this checkpoint is `1b91f9aa0abda8fccb0389972636708c4301ef88`.
- `FND-01` and `VSL-02` are accepted and archived.
- PR #50 delivered the accepted 19-member canonical Rust workspace and was squash-merged as `78988f72a80cc904aa9176ae850c50d4efa0b0f0`; PR #53 archived the destination cutover task.
- The native Rust client is now canonical in `blakinio/Oteryn-v2` and is in ADR-0011 `pre-native-protocol`.
- The destination production graph contains neither `protocol-canary` nor a speculative production `protocol-oteryn` adapter.
- No complete `protocol-oteryn` Rust client-to-server gameplay implementation or native gameplay E2E is proven.
- Oteryn Platform contains Identity, World Registry, Game Login Ticket and a standalone Go Game Gateway.
- No merged source-only historical marker in `blakinio/otclient` is proven by this checkpoint.

Every replacement agent must verify this baseline against the live default branch before repeating it.

### ACCEPTED

1. **Native Rust target**
   - Rust client and authoritative Rust server;
   - one project-owned `protocol-oteryn`;
   - multichannel-first world model.

2. **Repository ownership and client cutover state**
   - the Rust client is canonical in `blakinio/Oteryn-v2`;
   - client and shared Rust crates use the accepted 19-member canonical workspace;
   - `protocol-canary` is absent from the destination production graph;
   - `FND-01` and `VSL-02` are accepted and archived;
   - the atomic destination migration/workspace PR was merged as `78988f72a80cc904aa9176ae850c50d4efa0b0f0`;
   - the client is in ADR-0011 `pre-native-protocol` and fails closed before gameplay credential consumption, routing or transport;
   - the remaining cutover action is the source-only `blakinio/otclient` historical marker;
   - `FND-ID-01` follows after that marker is merged and verified.

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

6. **Game Intelligence direction**
   - one versioned event foundation supports separate gameplay/balance, world/content, economy/item and security consumers;
   - operational metrics, best-effort gameplay telemetry and durable transactional audit are distinct;
   - anti-duplication prevention remains in `DUR-03`;
   - investigation and AI are external, read-only and human-reviewed.

7. **Canonical workspace and pre-native client state**
   - the destination workspace, dependency policy, provenance and migration evidence are accepted and implemented;
   - ADR-0011 explicitly names the fail-closed state before `protocol-oteryn` exists;
   - destination completion does not prove protocol, server runtime, admission, durable gameplay or native gameplay E2E.

Canonical evidence: ADR-0001 through ADR-0011, `FND-01_WORKSPACE_AND_RUST_MIGRATION_CONTRACT.md`, `VSL-02_RUST_CLIENT_MIGRATION_AND_CUTOVER_CONTRACT.md` and destination merge `78988f72a80cc904aa9176ae850c50d4efa0b0f0`.

Registered open-decision coverage: `docs/architecture/GAMEPLAY_AND_PRODUCT_ARCHITECTURE_HORIZON.md`. Registration prevents omission but does not accept solutions.

## Stable gate IDs

- `FND-01` — Workspace, Dependency and Existing-Rust Migration Contract.
- `VSL-02` — Exact Rust Client Migration and Cutover Contract.
- `FND-ID-01` — Foundation Identifier Vocabulary.
- `FND-02` — `protocol-oteryn` v1 Contract.
- `FND-03` — Runtime Execution Contract.
- `FND-04` — Identity, Game Session, Admission and Character Lease Contract.
- `DUR-01` — Durable Identifier Representation Contract.
- `DUR-02` — Persistence v1 Contract.
- `DUR-03` — Item Transaction and Anti-Duplication Contract.
- `DUR-04` — Content, World Detail and Scripting Contract.
- `VSL-01` — Foundation Vertical-Slice Programme.
- `ANL-01` — Game Event and Audit Foundation Contract.
- `ANL-02` — Gameplay, Balance and World Analytics Contract.
- `ANL-03` — Economy Integrity and Security Analytics Contract.
- `ANL-04` — Read-Only Investigation and AI Contract.
- `GAME-CHAR-01` — Character Lifecycle and Progression.
- `GAME-ITEM-01` — Item Model and Equipment Rules.
- `GAME-ABILITY-01` — Ability, Spell and Condition Architecture.
- `GAME-AI-01` — Creature AI, Spawn and Pathfinding Architecture.
- `GAME-INTERACTION-01` — World Interaction and Environmental Mechanics.
- `PROD-LIVEOPS-01` — Live Operations and Runtime Configuration.
- `PROD-COMPAT-01` — Release Compatibility and Version Train.
- `SEC-CLIENT-01` — Client Integrity and Anti-Cheat Boundary.
- `DATA-PRIVACY-01` — Product Privacy and Data Lifecycle.
- `UX-I18N-A11Y-01` — Localization, Input, Onboarding and Accessibility.
- `OPS-GM-01` — Support, Moderation and GM Operations.
- `GAME-META-01` — Collections, Achievements and Recurring Progression.
- `GAME-INSTANCES-01` — Dungeons, Arenas, Matchmaking and Spectating.
- `GAME-WORLD-LIFECYCLE-01` — World Lifecycle, Transfer and Merge.
- `INTEGRATION-API-01` — External APIs, Notifications and Integrations.
- `PROD-ENTITLEMENTS-01` — Entitlements, Premium and Commerce Boundary.
- `MOD-ECOSYSTEM-01` — Modding and Plugin Ecosystem.

## Progressive implementation gates

### Completed migration gates

`FND-01`, `VSL-02` and the atomic destination migration/workspace PR are complete. The canonical destination merge is `78988f72a80cc904aa9176ae850c50d4efa0b0f0`. ADR-0011 defines the accepted `pre-native-protocol` state.

### Current cross-repository closeout

The next required action is a source-only `blakinio/otclient` historical marker. It must preserve the old repository as evidence, point to the exact destination merge, redirect new Oteryn v2 client work to `blakinio/Oteryn-v2` and contain no destination implementation.

`FND-ID-01` begins after that closeout is merged and verified.

### Layer gates

- `FND-ID-01` gates freezing identifier meanings in protocol and admission schemas after client migration.
- `FND-02` gates canonical protocol schemas/codecs and production compatibility claims.
- `FND-03` gates authoritative runtime ordering, lifecycle and recovery.
- `FND-04` gates production Game Session validation, admission and character lease behavior.
- `DUR-01` through `DUR-03` gate authoritative durable character, item and currency mutation.
- `DUR-04` gates broad content import and durable scripting.
- `VSL-01` gates completion of the native foundation vertical slice.
- `ANL-01` gates final transactional event/outbox/audit boundaries used by persistence and item transactions.
- `ANL-02`/`ANL-03` gate production-grade analytics claims; `ANL-04` gates later read-only AI investigation.
- `GAME-CHAR-01` and `GAME-ITEM-01` gate final character/item durable models before `DUR-02`/`DUR-03`.
- `GAME-ABILITY-01`, `GAME-AI-01` and `GAME-INTERACTION-01` gate Playable Alpha gameplay breadth.
- `PROD-LIVEOPS-01`, `PROD-COMPAT-01`, `SEC-CLIENT-01`, `DATA-PRIVACY-01`, `UX-I18N-A11Y-01` and `OPS-GM-01` gate Playable Alpha operational completeness.
- Expansion/deferred gameplay-product gates remain inactive until their milestone or explicit owner decision requires them.

## Foundation contract state

### `FND-01` — Workspace, Dependency and Existing-Rust Migration Contract

Status: **ACCEPTED AND APPLIED**.

- exact minimal initial applications, services and crates;
- pinned existing-Rust workspace inventory and migrate/rename/merge/split/rewrite/reference/drop disposition;
- immediate consumer and observable acceptance for every member;
- legal dependency directions and forbidden edges;
- canonical ownership of identifiers, domain types, protocol schemas, world/content schemas and fixtures;
- Rust edition/resolver/toolchain/`rust-version`/lockfile, workspace inheritance, feature and dependency policy;
- exact target triples and product-realistic target/feature CI matrix;
- retained machine-readable workspace-boundary contract and executable dependency-graph enforcement;
- criteria for later adding, splitting or merging crates;
- terminal handoff to `VSL-02`, not an isolated destination bootstrap.

### `VSL-02` — Exact Rust Client Migration and Cutover Contract

Status: **ACCEPTED; DESTINATION COMPLETE; SOURCE-ONLY CLOSEOUT PENDING**.

- exact cutover source SHA and reconciliation of open PRs, active tasks and source changes after the FND-01 inventory;
- squash-compatible provenance/history traceability through immutable source retention, exact SHA/range, machine-readable path mapping, copyright/license records and source links;
- per-crate/subsystem migration disposition from FND-01;
- one atomic destination PR containing migration, root-workspace creation/completion, dependency enforcement, protocol-canary isolation and validation;
- exact source/destination build and test evidence;
- `protocol-canary` isolation or removal from the target runtime graph;
- source freeze/non-canonical marker and destination ownership;
- one atomic destination task/branch/PR in Oteryn-v2 plus one later source-marker task/branch/PR in otclient under one rollout/rollback order;
- rollback conditions and procedure.

### `FND-ID-01` — Foundation Identifier Vocabulary

- semantic ownership, scope, uniqueness, reuse, durability and visibility of the minimum cross-boundary IDs;
- canonical wire/Game Session encoding constraints;
- no premature PostgreSQL layout selection; full durable representation remains in `DUR-01`.

### `FND-02` — `protocol-oteryn` v1 Contract

- reconcile, revise or explicitly supersede the exact latest merged Platform native contract;
- populate the machine-readable cross-repository contract lock and reject mutable PR heads as canonical;
- transport, TLS/ALPN, framing, stable schema/IDL and hard limits;
- revisions, capabilities and downgrade prevention;
- sequencing, command IDs, replay/idempotency;
- snapshots, deltas, reconnect/reconciliation and golden fixtures.

### `FND-03` — Runtime Execution Contract

- node/world/channel/instance responsibilities;
- tick, monotonic/wall-clock and deterministic test-clock model;
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
- `ANL-01` event and audit foundation before persistence/item contracts finalize their atomic evidence;
- `ANL-02`/`ANL-03` before production-grade analytics claims;
- `VSL-01` foundation vertical-slice programme with named observable evidence.

## Foundation vertical slice

The initial programme must prove at least:

1. Platform authentication and Gateway-issued Game Session;
2. Rust game-server admission and character lease;
3. entry to a minimal native map;
4. authoritative movement and two-player visibility on one channel;
5. one monster combat/death/loot path;
6. retry-safe item pickup;
7. correlated combat/death/loot/pickup events, atomic item audit and replay-safe analytics;
8. durable checkpoint and safe logout;
9. relog to another channel with shared character state preserved;
10. rejection of a simultaneous second session;
11. isolation of channel-local state and preservation of world-shared state.

## Wider global decision horizon

The global register preserves and stages movement, combat, inventory/economy, rulesets/scripting, client migration/architecture, World Project/Bundle/Registry/Studio, Game Intelligence/analytics/audit, raids/houses/social systems, security, updater, deployment, observability, testing, quantitative performance targets and product milestones.

These subjects must not be forgotten, but they must not be prematurely frozen when they do not block the current stage.

## Explicitly deferred

These do not block client migration, workspace bootstrap/completion after cutover or the foundation vertical slice when safe extension points remain:

- final house presence/entry topology;
- live channel migration;
- partitioning one channel across multiple nodes;
- QUIC;
- cross-world chat/guilds/parties;
- hundreds of dynamic channels;
- `GAME-INSTANCES-01` complete instance/matchmaking/spectator programme and full market programme;
- all classic rulesets;
- final launcher/updater;
- `MOD-ECOSYSTEM-01` public mod ecosystem;
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
- [x] ADR-0002 requires `VSL-02` and controlled client migration/cutover immediately after `FND-01`.
- [x] `FND-01` Workspace, Dependency and Existing-Rust Migration Contract accepted.
- [x] FND-01 classifies every existing Rust client crate/subsystem and preserves its exact evidence or explicitly rejects it.
- [x] `VSL-02` pins the exact cutover source SHA, provenance, open-PR/task disposition, path mapping, source freeze, destination ownership, history preservation and rollback.
- [x] The coordinated destination migration/cutover completed before canonical identifier, protocol, runtime or admission contracts were frozen; source-only closeout remains pending.
- [x] One atomic Oteryn-v2 destination PR created/completed the root workspace around the migrated client; no second destination bootstrap/consolidation PR exists.
- [ ] The later otclient PR only marks the source moved/non-canonical after the destination squash merge is verified.
- [x] Provenance records exact source SHA/range and path mapping without a false Git-ancestry claim.
- [x] Initial workspace members require an immediate consumer and observable acceptance; no empty layering crates.
- [x] Dependency directions are machine-enforced in the destination workspace.
- [ ] `FND-ID-01` accepted after migration and before protocol or admission freezes identifier meanings.
- [ ] `FND-02` accepted and reconciled with a merged, machine-locked Platform revision before canonical protocol implementation.
- [ ] `FND-03` accepted before authoritative runtime implementation.
- [ ] `FND-04` accepted before production admission/lease implementation.
- [x] PostgreSQL and separate Platform/game ownership accepted in ADR-0004.
- [ ] `DUR-01` accepted before durable database identity representations are frozen and before authoritative durable gameplay mutation.
- [ ] `DUR-02` accepted before durable gameplay mutations.
- [ ] `DUR-03` accepted before durable item/currency mutation.
- [ ] `GAME-CHAR-01` accepted before `DUR-02` freezes the durable character schema.
- [ ] `GAME-ITEM-01` accepted before `DUR-03` freezes item behavior and transaction semantics.
- [ ] Playable Alpha gameplay breadth is not claimed before `GAME-ABILITY-01`, `GAME-AI-01` and `GAME-INTERACTION-01`.
- [ ] Playable Alpha operational completeness is not claimed before live-ops, compatibility, client-integrity, privacy, localization/accessibility and GM-operation gates are accepted.
- [x] Native world/content/editor direction accepted in ADR-0005.
- [x] Game Intelligence, analytics durability classes, privacy and read-only investigation direction accepted in ADR-0006.
- [ ] `ANL-01` accepted before final outbox/audit boundaries are frozen in `DUR-02`/`DUR-03`.
- [ ] `ANL-02`/`ANL-03` accepted before production-grade analytics claims; `ANL-04` remains a separately authorized expansion gate.
- [ ] `DUR-04` accepted before broad content import or durable scripting.
- [ ] `VSL-01` accepted with named E2E evidence.
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

- ADR-0002 resolved canonical repository ownership, requires early client migration/cutover immediately after FND-01 and fixes one atomic destination migration/workspace PR followed by a source-only marker PR.
- ADR-0003 resolved the Platform Identity/Game Gateway boundary and retained the initial Go Gateway.
- ADR-0004 selected PostgreSQL and separate Platform/game database ownership.
- ADR-0005 accepted the native world/content model, Oteryn Studio and bounded legacy conversion direction.
- ADR-0006 accepted Oteryn Game Intelligence, separated observability/telemetry/durable audit, and introduced ANL-01 through ANL-04.
- ADR-0007 through ADR-0011 accepted the E2E platform, Canary exclusion, GameNode baseline, product profiles and fail-closed pre-native client state.
- `FND-01` and `VSL-02` are accepted and archived; destination PR #50 merged the canonical workspace as `78988f72a80cc904aa9176ae850c50d4efa0b0f0`.
- The source-only `blakinio/otclient` marker remains required before `FND-ID-01`.
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

Every package PR must challenge omitted boundaries, circular dependencies, placeholder crates, duplicate authority, unsafe concurrency/replay/recovery, item duplication, missing provenance/outbox evidence, telemetry/audit durability confusion, privacy/retention/access gaps, AI mutation authority, cross-repository rollout conflicts, unsupported runtime claims, premature freezing and stale decisions.

## Context checkpoint

```yaml
last_progress: FND-01 and VSL-02 are accepted and archived; PR #50 merged the canonical 19-member Rust workspace as 78988f72a80cc904aa9176ae850c50d4efa0b0f0; ADR-0011 defines the fail-closed pre-native-protocol client state.
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
  - docs/architecture/ADR-0006-game-intelligence-analytics-and-audit.md
  - docs/architecture/ADR-0007-native-end-to-end-test-platform.md
  - docs/architecture/ADR-0008-protocol-canary-reference-only-migration-disposition.md
  - docs/architecture/ADR-0009-game-node-execution-capacity-deployment-and-recovery-baseline.md
  - docs/architecture/ADR-0010-reference-and-evolved-world-product-profiles.md
  - docs/architecture/ADR-0011-native-client-pre-protocol-migration-state.md
  - docs/architecture/FOUNDATION_DECISION_BACKLOG.md
  - docs/architecture/GLOBAL_ARCHITECTURE_DECISION_REGISTER.md
  - docs/architecture/GAMEPLAY_AND_PRODUCT_ARCHITECTURE_HORIZON.md
  - docs/contracts/CROSS_REPOSITORY_CONTRACT_LOCK.json
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - docs/contracts/FOUNDATION_ERROR_VOCABULARY.md
  - docs/contracts/FOUNDATION_FAILURE_SCENARIOS.md
continuation_prompt: docs/agents/prompts/OTV2_GLOBAL_ARCHITECTURE_DECISION_COORDINATOR.md
validation_state: Destination workspace and dependency boundaries are implemented and validated; every future package still requires exact-head focused checks, audit and applicable E2E evidence.
audit_state: Source-only marker and future gate audits pending.
e2e_state: BLOCKED until VSL-01 implementation; destination cutover does not prove native gameplay.
ci_generation: null
run_ids: []
counters:
  ci_checks_for_current_head: 0
  terminal_ci_checks_for_current_generation: 0
  unchanged_state_checks: 0
  identical_failure_retries: 0
  repair_cycles_for_current_gate: 0
  stall_warnings: 0
blocker: The required source-only historical marker is not yet proven merged in blakinio/otclient.
next_action: Complete and merge the source-only blakinio/otclient marker for destination merge 78988f72a80cc904aa9176ae850c50d4efa0b0f0; after verification, begin FND-ID-01.
```
