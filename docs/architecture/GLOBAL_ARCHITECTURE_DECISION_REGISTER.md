# Oteryn v2 Global Architecture Decision Register

- Status: Active coordination register
- Date: 2026-08-05
- Coordination ID: `OTV2-GLOBAL-ARCHITECTURE`
- Canonical foundation programme: `docs/agents/tasks/active/OTV2-20260805-foundation-preimplementation-contracts.md`
- Coordinator prompt: `docs/agents/prompts/OTV2_GLOBAL_ARCHITECTURE_DECISION_COORDINATOR.md`

## Purpose

Preserve the complete global architecture decision horizon for Oteryn v2 without requiring chat history and without forcing distant expansion features to be designed before they become relevant.

This register is a coordination source, not an implementation claim. Accepted decisions live in ADRs and contracts. The ordered foundation gates live in `FOUNDATION_DECISION_BACKLOG.md`. This file ensures that global project domains are not lost while the programme resolves them in stages.

## Status vocabulary

- `ACCEPTED` — frozen by an accepted ADR/contract.
- `BLOCKS_WORKSPACE` — must be accepted before creating the real root Cargo workspace.
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

## Stage A — blocks the real workspace

### A1. Workspace and dependency contract

- Status: `BLOCKS_WORKSPACE`
- Decide exact initial applications, services and crates.
- Freeze legal dependency directions and forbidden edges.
- Assign canonical ownership of identifiers, domain contracts, protocol schemas, world/content schemas and fixtures.
- Select Rust edition, minimum toolchain, target platforms, feature policy and baseline CI.
- Keep domain/simulation independent from transports, SQL, renderer and UI.
- Keep world/content schema independent from Tauri, editor UI and renderer implementation.

### A2. `protocol-oteryn` v1

- Status: `BLOCKS_WORKSPACE`
- Reconcile with the exact current Platform native contract.
- Freeze transport, TLS/ALPN, framing, schema/IDL, hard limits and error vocabulary.
- Separate protocol, capability, content and ruleset revisions.
- Define sequencing, command IDs, replay/idempotency, snapshots, deltas, reconciliation and reconnect/resume.
- Produce shared golden fixtures and downgrade protection.

### A3. Runtime execution contract

- Status: `BLOCKS_WORKSPACE`
- Freeze `NodeRuntime`, `WorldServices`, `ChannelRuntime` and `InstanceRuntime` responsibilities.
- Define tick/timer model, command ordering, bounded queues, overload/backpressure and parallel work return to the logical writer.
- Define channel lifecycle, draining, checkpoint, crash recovery and replay boundaries.

### A4. Identity, Game Session, admission and character lease

- Status: `BLOCKS_WORKSPACE`
- Freeze token/session representation, issuer/audience, key rotation, replay prevention and revocation.
- Define world/channel/revision binding, reconnect windows and admission errors.
- Define `session_generation`, lease storage/timings, duplicate login, stale-writer fencing and safe channel switching.
- Define Platform, Gateway, PostgreSQL and network failure behavior.

## Stage B — blocks durable gameplay

### B1. Identifier contract

- Status: `BLOCKS_DURABLE_GAMEPLAY`
- Cover account, character, world, channel, instance, node, game session, command, entity, item-instance and revision identities.
- Decide global versus scoped uniqueness, wire/database/public representation and reuse rules.

### B2. Persistence v1

- Status: `BLOCKS_DURABLE_GAMEPLAY`
- Freeze schema and migration ownership.
- Define character revisions/fencing, lease schema, checkpoint boundaries and maximum accepted progress loss.
- Define isolation, locking, retries, idempotency, transactional outbox and critical audit/journal scope.
- Define backup, PITR, restore tests, RPO/RTO and compatible rollout/rollback.

### B3. Item transaction and anti-duplication invariants

- Status: `BLOCKS_DURABLE_GAMEPLAY`
- Define item instance identity and ownership.
- Freeze inventory/equipment/container/ground transfer transaction boundaries.
- Define pickup, drop, loot, trade, bank, depot, reward and retry semantics.
- Prove that duplicate commands, crashes, stale sessions and partial failures cannot duplicate items or currency.

This may be part of Persistence v1 only if that contract is sufficiently complete and independently auditable.

### B4. Content, world-detail and scripting contract

- Status: `BLOCKS_DURABLE_GAMEPLAY` for broad import and scripted durable behavior
- Continue ADR-0005 with exact World Project and World Bundle encodings.
- Freeze Content Registry package/version/dependency behavior.
- Pin migration source revisions and classify source areas as `COPY`, `CONVERT`, `REWRITE`, `REFERENCE_ONLY` or `REJECT`.
- Select scripting runtime and define capabilities, limits, failure isolation, persistence access and hot reload.
- Preserve asset rights and provenance gates.

## Stage C — blocks the foundation vertical slice

### C1. Foundation vertical-slice programme

- Status: `BLOCKS_VERTICAL_SLICE`
- Name owners, implementation order, repositories, branches/PRs and exact observable E2E.
- Include Platform authentication, Gateway-issued Game Session, server admission, lease, minimal native map, movement, two-player visibility, combat, death, loot, retry-safe pickup, checkpoint, logout, cross-channel relog, duplicate-login rejection and channel/world-state isolation.

### C2. Movement, collision and visibility contract

- Status: `BLOCKS_VERTICAL_SLICE`
- Freeze position/direction representation, movement ordering, diagonal rules, collision, floor transitions and teleport behavior.
- Define interest management, snapshots/deltas, view range and lag/reconciliation behavior.
- Keep legality server-authoritative.

### C3. Minimal combat, death and loot contract

- Status: `BLOCKS_VERTICAL_SLICE`
- Define action ordering, target legality, damage pipeline boundary, death, corpse, loot ownership, experience/kill attribution and retry-safe pickup integration.
- Preserve extension points for conditions, PvP, boss contribution and rulesets.

### C4. Exact Rust client migration contract

- Status: `BLOCKS_VERTICAL_SLICE`
- Pin exact source SHA from `blakinio/otclient/oteryn-client`.
- Define history/provenance preservation, path mapping, exclusions and rollback.
- Classify code as migrate, adapt, rewrite or reference-only.
- Do not claim target-runtime readiness from source presence alone.

### C5. Minimal native map/content/compiler/loader contract

- Status: `BLOCKS_VERTICAL_SLICE`
- Define the minimum World Project, World Bundle, compiler, loader and content keys required by the vertical slice.
- Avoid prematurely implementing the complete Studio or all legacy importers.

## Stage D — required for a playable alpha

### D1. Client architecture

- Status: `REQUIRED_FOR_ALPHA`
- Renderer, UI, input, networking, client state, prediction/reconciliation, assets, audio, settings, accessibility and crash reporting boundaries.
- Decide which low-level renderer components may be shared with Oteryn Studio.

### D2. Ruleset architecture

- Status: `REQUIRED_FOR_ALPHA`
- Define which classic/modern differences are data/policy and which use capability-bounded domain modules.
- Cover combat, movement, regeneration, death penalties, professions, item behavior and client presentation without protocol forks.

### D3. NPC, quests, shops and content runtime

- Status: `REQUIRED_FOR_ALPHA`
- Define content execution APIs, state ownership, deterministic tests, error isolation and authoring workflow.

### D4. Testing and performance baselines

- Status: `REQUIRED_FOR_ALPHA`
- Unit, property, fuzz, integration, golden protocol, deterministic simulation, database concurrency, crash recovery, multichannel, E2E, soak and migration tests.
- Freeze quantitative targets for players/channel, channels/node, tick budget, latency, reconnect, memory, startup, bundle size and database throughput.

### D5. Product milestone contract

- Status: `REQUIRED_FOR_ALPHA`
- Define Foundation, Playable Alpha, Beta and Release scopes with observable outcomes and excluded systems.
- Prevent broad parallel implementation without a complete vertical slice.

## Stage E — expansion systems

### E1. Dynamic events, raids and bosses

- Status: `EXPANSION`
- Scheduler, channel-local/world-shared uniqueness, cooldowns, anti-hopping, participation, scaling, persistence, crash recovery and reward ownership.
- Use ADR-0005 `EncounterZone`/`RaidCell`/`RaidAnchor` geometry.

### E2. Houses

- Status: `DEFERRED`
- Preserve one authoritative house state per world and anti-duplication invariants.
- Later decide presence/topology, channel access, simulation owner, rent, auctions, doors, beds, guest lists and crash recovery in a dedicated ADR.

### E3. Party, guild, chat, friends and presence

- Status: `EXPANSION`
- Define world/channel/cross-world scope, consistency, shared experience, buffs, rosters, messaging, offline delivery and presence.

### E4. Market, trade and economy

- Status: `EXPANSION`
- Define offers, escrow, fees, partial fills, cancellation, delivery, concurrency, audit and fraud/duplication resistance.

### E5. Security, abuse and administration

- Status: `EXPANSION` with foundation requirements applied earlier where relevant
- Rate limits, anti-replay, cheat detection, trusted admin/GM commands, least privilege, audit trails, secret management, dependency security and supply-chain controls.

### E6. Launcher, updater and release distribution

- Status: `DEFERRED`
- Signed manifests, delta updates, rollback, channels, asset/client compatibility, CDN/mirrors and archive/path/decompression safety.

### E7. Deployment and operations

- Status: `EXPANSION`
- Deployment topology, environments, configuration, secrets, discovery, health/readiness, draining, rolling updates, migrations, backup and disaster recovery.

### E8. Observability

- Status: `EXPANSION`
- Structured logs, metrics, tracing, correlation IDs, tick/queue/channel health, DB latency, protocol errors, privacy, retention and alerts.

### E9. Advanced scaling and prediction

- Status: `DEFERRED`
- Live channel migration, partitioning one channel across nodes, hundreds of dynamic channels, QUIC, advanced client prediction and independently deployable world services.

## Decision discipline

1. Resolve stages in dependency order.
2. Do not use this register as permission to implement.
3. Update status only when an accepted ADR/contract provides evidence.
4. Every accepted package must update this register narrowly and link the canonical source.
5. Preserve deferred topics and safe extension points without inventing final designs.
6. A decision is not complete until its PR is validated, audited, squash-merged and its task archived.

## Current next action

Draft and accept the **Workspace and Dependency Contract**, then reconcile this register and the canonical foundation checkpoint before proceeding to `protocol-oteryn` v1.
