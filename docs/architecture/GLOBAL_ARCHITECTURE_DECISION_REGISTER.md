# Oteryn v2 Global Architecture Decision Register

- Status: Active coordination register
- Date: 2026-08-05
- Coordination ID: `OTV2-GLOBAL-ARCHITECTURE`
- Canonical foundation programme: `docs/agents/tasks/active/OTV2-20260805-foundation-preimplementation-contracts.md`
- Coordinator prompt: `docs/agents/prompts/OTV2_GLOBAL_ARCHITECTURE_DECISION_COORDINATOR.md`

## Purpose

Preserve the complete global architecture decision horizon for Oteryn v2 without requiring chat history and without forcing distant expansion features to be designed before they become relevant.

This register is a coordination source, not an implementation claim. Accepted decisions live in ADRs and contracts. The ordered foundation gates live in `FOUNDATION_DECISION_BACKLOG.md`. This file ensures that global project domains are not lost while the programme resolves them in stages.

Stable IDs are canonical across tasks, prompts and PRs. Stage labels are descriptive only.

## Status vocabulary

- `ACCEPTED` — frozen by an accepted ADR/contract.
- `BLOCKS_WORKSPACE_BOOTSTRAP` — must be accepted before creating the canonical root Cargo workspace.
- `BLOCKS_LAYER_IMPLEMENTATION` — must be accepted before canonical production implementation of the named layer, but does not block a minimal workspace after `FND-01`.
- `BLOCKS_DURABLE_GAMEPLAY` — must be accepted before authoritative durable gameplay mutation.
- `BLOCKS_VERTICAL_SLICE` — required for the first complete client-to-server gameplay proof.
- `REQUIRED_FOR_ALPHA` — required before a playable alpha can be called complete.
- `EXPANSION` — required as the product grows, but not a current foundation gate.
- `DEFERRED` — deliberately not frozen now; safe extension points must remain.

## Accepted foundation

| Domain | Status | Canonical source |
|---|---|---|
| Native Rust client/server and project-owned `protocol-oteryn` | `ACCEPTED` | ADR-0001 |
| Multichannel world/channel/instance ownership baseline | `ACCEPTED` | ADR-0001 and scope matrix |
| Canonical repository and Rust client migration direction | `ACCEPTED` | ADR-0002 |
| Platform Identity and initial Go Game Gateway boundary | `ACCEPTED` | ADR-0003 |
| PostgreSQL and separate Platform/game ownership | `ACCEPTED` | ADR-0004 |
| Native world/content model, Oteryn Studio and legacy conversion boundary | `ACCEPTED` | ADR-0005 |

## Progressive execution policy

- Before `FND-01`, do not create the canonical root Cargo workspace.
- After `FND-01`, a separate task may create the smallest compilable workspace and machine-check dependency boundaries.
- `FND-02`, `FND-03` and `FND-04` independently gate canonical protocol, authoritative runtime and production admission/lease behavior.
- Bounded technical spikes may inform contracts only when reversible, isolated, non-production and explicitly non-canonical.
- `DUR-01` through `DUR-03` remain hard gates before authoritative durable character, item or currency mutation.
- A spike, placeholder crate or passing compile does not prove a public contract or runtime capability.

## Stage A — foundation and layer gates

### `FND-01` — Workspace and Dependency Contract

- Status: `BLOCKS_WORKSPACE_BOOTSTRAP`
- Decide the smallest initial applications, services and crates; every member requires an immediate consumer and observable acceptance.
- Freeze legal dependency directions and forbidden edges.
- Assign canonical ownership of identifiers, domain contracts, protocol schemas, world/content schemas and fixtures.
- Select Rust edition, minimum toolchain, target platforms, feature policy and baseline CI.
- Define machine-enforced dependency-graph and forbidden-edge checks.
- Keep domain/simulation independent from transports, SQL, renderer and UI.
- Keep world/content schema independent from Tauri, editor UI and renderer implementation.
- Treat the wider crate list as a capability horizon, not an instruction to create empty layering crates.

### `FND-02` — `protocol-oteryn` v1

- Status: `BLOCKS_LAYER_IMPLEMENTATION`
- Reconcile with the exact current Platform native contract.
- Freeze transport, TLS/ALPN, framing, schema/IDL, hard limits and error vocabulary.
- Separate protocol, capability, content and ruleset revisions.
- Define sequencing, command IDs, replay/idempotency, snapshots, deltas, reconciliation and reconnect/resume.
- Produce shared golden fixtures and downgrade protection.
- Gate canonical wire schemas/codecs and production compatibility claims, not minimal workspace bootstrap.

### `FND-03` — Runtime Execution Contract

- Status: `BLOCKS_LAYER_IMPLEMENTATION`
- Freeze `NodeRuntime`, `WorldServices`, `ChannelRuntime` and `InstanceRuntime` responsibilities.
- Define tick/timer model, command ordering, bounded queues, overload/backpressure and parallel work return to the logical writer.
- Define channel lifecycle, draining, checkpoint, crash recovery and replay boundaries.
- Gate authoritative runtime behavior, not compile-only interfaces or isolated spikes.

### `FND-04` — Identity, Game Session, Admission and Character Lease

- Status: `BLOCKS_LAYER_IMPLEMENTATION`
- Freeze token/session representation, issuer/audience, key rotation, replay prevention and revocation.
- Define world/channel/revision binding, reconnect windows and admission errors.
- Define `session_generation`, lease storage/timings, duplicate login, stale-writer fencing and safe channel switching.
- Define Platform, Gateway, PostgreSQL and network failure behavior.
- Gate production admission and lease behavior, not minimal workspace bootstrap.

## Stage B — blocks durable gameplay

### `DUR-01` — Identifier Contract

- Status: `BLOCKS_DURABLE_GAMEPLAY`
- Cover account, character, world, channel, instance, node, game session, command, entity, item-instance and revision identities.
- Decide global versus scoped uniqueness, wire/database/public representation and reuse rules.

### `DUR-02` — Persistence v1

- Status: `BLOCKS_DURABLE_GAMEPLAY`
- Freeze schema and migration ownership.
- Define character revisions/fencing, lease schema, checkpoint boundaries and maximum accepted progress loss.
- Define isolation, locking, retries, idempotency, transactional outbox and critical audit/journal scope.
- Define backup, PITR, restore tests, RPO/RTO and compatible rollout/rollback.

### `DUR-03` — Item Transaction and Anti-Duplication Invariants

- Status: `BLOCKS_DURABLE_GAMEPLAY`
- Define item instance identity and ownership.
- Freeze inventory/equipment/container/ground transfer transaction boundaries.
- Define pickup, drop, loot, trade, bank, depot, reward and retry semantics.
- Prove that duplicate commands, crashes, stale sessions and partial failures cannot duplicate items or currency.

This may be part of Persistence v1 only if that contract is sufficiently complete and independently auditable.

### `DUR-04` — Content, World Detail and Scripting Contract

- Status: `BLOCKS_DURABLE_GAMEPLAY` for broad import and scripted durable behavior.
- Continue ADR-0005 with exact World Project and World Bundle encodings.
- Freeze Content Registry package/version/dependency behavior.
- Pin migration source revisions and classify source areas as `COPY`, `CONVERT`, `REWRITE`, `REFERENCE_ONLY` or `REJECT`.
- Select scripting runtime and define capabilities, limits, failure isolation, persistence access and hot reload.
- Preserve asset rights and provenance gates.
- Permit only bounded format/compiler/loader spikes before acceptance; no compatibility or canonical-format claim may escape the spike.

## Stage C — blocks the foundation vertical slice

### `VSL-01` — Foundation Vertical-Slice Programme

- Status: `BLOCKS_VERTICAL_SLICE`
- Name owners, implementation order, repositories, branches/PRs and exact observable E2E.
- Include Platform authentication, Gateway-issued Game Session, server admission, lease, minimal native map, movement, two-player visibility, combat, death, loot, retry-safe pickup, checkpoint, logout, cross-channel relog, duplicate-login rejection and channel/world-state isolation.

### `VSL-MOVE-01` — Movement, Collision and Visibility Contract

- Status: `BLOCKS_VERTICAL_SLICE`
- Freeze position/direction representation, movement ordering, diagonal rules, collision, floor transitions and teleport behavior.
- Define interest management, snapshots/deltas, view range and lag/reconciliation behavior.
- Keep legality server-authoritative.

### `VSL-COMBAT-01` — Minimal Combat, Death and Loot Contract

- Status: `BLOCKS_VERTICAL_SLICE`
- Define action ordering, target legality, damage pipeline boundary, death, corpse, loot ownership, experience/kill attribution and retry-safe pickup integration.
- Preserve extension points for conditions, PvP, boss contribution and rulesets.

### `VSL-02` — Exact Rust Client Migration Contract

- Status: `BLOCKS_VERTICAL_SLICE`
- Pin exact source SHA from `blakinio/otclient/oteryn-client`.
- Define history/provenance preservation, path mapping, exclusions and rollback.
- Classify code as migrate, adapt, rewrite or reference-only.
- Do not claim target-runtime readiness from source presence alone.

### `VSL-CONTENT-01` — Minimal Native Map, Compiler and Loader Contract

- Status: `BLOCKS_VERTICAL_SLICE`
- Define the minimum World Project, World Bundle, compiler, loader and content keys required by the vertical slice.
- Avoid prematurely implementing the complete Studio or all legacy importers.

## Stage D — required for a playable alpha

### `ALPHA-CLIENT-01` — Client Architecture

- Status: `REQUIRED_FOR_ALPHA`
- Renderer, UI, input, networking, client state, prediction/reconciliation, assets, audio, settings, accessibility and crash reporting boundaries.
- Decide which low-level renderer components may be shared with Oteryn Studio.

### `ALPHA-RULESET-01` — Ruleset Architecture

- Status: `REQUIRED_FOR_ALPHA`
- Define which classic/modern differences are data/policy and which use capability-bounded domain modules.
- Cover combat, movement, regeneration, death penalties, professions, item behavior and client presentation without protocol forks.

### `ALPHA-CONTENT-01` — NPC, Quests, Shops and Content Runtime

- Status: `REQUIRED_FOR_ALPHA`
- Define content execution APIs, state ownership, deterministic tests, error isolation and authoring workflow.

### `ALPHA-QUALITY-01` — Testing and Performance Baselines

- Status: `REQUIRED_FOR_ALPHA`
- Unit, property, fuzz, integration, golden protocol, deterministic simulation, database concurrency, crash recovery, multichannel, E2E, soak and migration tests.
- Freeze quantitative targets for players/channel, channels/node, tick budget, latency, reconnect, memory, startup, bundle size and database throughput.

### `ALPHA-MILESTONE-01` — Product Milestone Contract

- Status: `REQUIRED_FOR_ALPHA`
- Define Foundation, Playable Alpha, Beta and Release scopes with observable outcomes and excluded systems.
- Prevent broad parallel implementation without a complete vertical slice.

## Stage E — expansion systems

### `EXP-EVENTS-01` — Dynamic Events, Raids and Bosses

- Status: `EXPANSION`
- Scheduler, channel-local/world-shared uniqueness, cooldowns, anti-hopping, participation, scaling, persistence, crash recovery and reward ownership.
- Use ADR-0005 `EncounterZone`/`RaidCell`/`RaidAnchor` geometry.

### `EXP-HOUSES-01` — Houses

- Status: `DEFERRED`
- Preserve one authoritative house state per world and anti-duplication invariants.
- Later decide presence/topology, channel access, simulation owner, rent, auctions, doors, beds, guest lists and crash recovery in a dedicated ADR.

### `EXP-SOCIAL-01` — Party, Guild, Chat, Friends and Presence

- Status: `EXPANSION`
- Define world/channel/cross-world scope, consistency, shared experience, buffs, rosters, messaging, offline delivery and presence.

### `EXP-ECONOMY-01` — Market, Trade and Economy

- Status: `EXPANSION`
- Define offers, escrow, fees, partial fills, cancellation, delivery, concurrency, audit and fraud/duplication resistance.

### `EXP-SECURITY-01` — Security, Abuse and Administration

- Status: `EXPANSION` with foundation requirements applied earlier where relevant.
- Rate limits, anti-replay, cheat detection, trusted admin/GM commands, least privilege, audit trails, secret management, dependency security and supply-chain controls.

### `EXP-UPDATE-01` — Launcher, Updater and Release Distribution

- Status: `DEFERRED`
- Signed manifests, delta updates, rollback, channels, asset/client compatibility, CDN/mirrors and archive/path/decompression safety.

### `EXP-OPS-01` — Deployment and Operations

- Status: `EXPANSION`
- Deployment topology, environments, configuration, secrets, discovery, health/readiness, draining, rolling updates, migrations, backup and disaster recovery.

### `EXP-OBS-01` — Observability

- Status: `EXPANSION`
- Structured logs, metrics, tracing, correlation IDs, tick/queue/channel health, DB latency, protocol errors, privacy, retention and alerts.

### `EXP-SCALE-01` — Advanced Scaling and Prediction

- Status: `DEFERRED`
- Live channel migration, partitioning one channel across nodes, hundreds of dynamic channels, QUIC, advanced client prediction and independently deployable world services.

## Programme ownership discipline

The canonical foundation task is a non-owning programme checkpoint. Each substantial gate has one separate task, branch, PR, owner, owned paths and terminal archive. The programme checkpoint records accepted state and exactly one next action; it does not reserve all future architecture paths.

## Decision discipline

1. Resolve gates in dependency order using stable IDs.
2. Do not use this register as permission to implement beyond the progressive execution policy.
3. Update status only when an accepted ADR/contract provides evidence.
4. Every accepted package must update this register narrowly and link the canonical source.
5. Preserve deferred topics and safe extension points without inventing final designs.
6. Initial workspace members require an immediate consumer and acceptance; do not create speculative placeholder crates.
7. After workspace bootstrap, enforce accepted dependency boundaries with executable CI checks.
8. A decision is not complete until its PR is validated, audited, squash-merged and its task archived.

## Current next action

Draft and accept `FND-01` — the **Workspace and Dependency Contract**. After it is terminal, authorize a separate minimal workspace-bootstrap implementation task before proceeding with layer-specific implementation.