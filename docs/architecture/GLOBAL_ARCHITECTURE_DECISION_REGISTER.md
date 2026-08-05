# Oteryn v2 Global Architecture Decision Register

- Status: Active coordination register
- Date: 2026-08-05
- Coordination ID: `OTV2-GLOBAL-ARCHITECTURE`
- Canonical foundation programme: `docs/agents/tasks/active/OTV2-20260805-foundation-preimplementation-contracts.md`
- Coordinator prompt: `docs/agents/prompts/OTV2_GLOBAL_ARCHITECTURE_DECISION_COORDINATOR.md`

## Purpose

Preserve the complete global architecture decision horizon for Oteryn v2 without requiring chat history and without forcing distant expansion features to be designed before they become relevant.

This register is a coordination source, not an implementation claim. Accepted decisions live in ADRs and contracts. The ordered foundation gates live in `FOUNDATION_DECISION_BACKLOG.md`. This file ensures that global project domains are not lost while the programme resolves them in stages. The detailed open gameplay and product horizon is retained in `GAMEPLAY_AND_PRODUCT_ARCHITECTURE_HORIZON.md`.

Stable IDs are canonical across tasks, prompts and PRs. Stage labels are descriptive only.

## Status vocabulary

- `ACCEPTED` — frozen by an accepted ADR/contract.
- `BLOCKS_WORKSPACE_BOOTSTRAP` — must be accepted before creating or completing the canonical root Cargo workspace.
- `BLOCKS_LAYER_IMPLEMENTATION` — must be accepted before canonical production implementation of the named layer, but does not block a minimal workspace after the client migration/cutover sequence.
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
| Canonical repository, early Rust client cutover and one atomic destination migration PR | `ACCEPTED` | ADR-0002 |
| Platform Identity and initial Go Game Gateway boundary | `ACCEPTED` | ADR-0003 |
| PostgreSQL and separate Platform/game ownership | `ACCEPTED` | ADR-0004 |
| Native world/content model, Oteryn Studio and legacy conversion boundary | `ACCEPTED` | ADR-0005 |
| Game Intelligence, analytics durability classes and read-only investigation boundary | `ACCEPTED` | ADR-0006 |
| Native three-tier E2E platform, evidence, cleanup and stability contract | `ACCEPTED` | ADR-0007 |
| `protocol-canary` fixed as reference-only evidence and excluded from every production runtime/dependency/fallback path | `ACCEPTED` | ADR-0008 |

## Progressive execution policy

- Before `FND-01`, do not create the canonical root Cargo workspace.
- After `FND-01`, `VSL-02` is the next mandatory gate; it must pin and reconcile the exact client cutover before shared client/server contracts are frozen.
- One atomic `blakinio/Oteryn-v2` destination PR must contain the accepted client migration, root-workspace creation/completion, dependency enforcement, ADR-0008 `protocol-canary` exclusion, provenance and exact-head validation; no separate destination bootstrap PR may follow.
- A later `blakinio/otclient` PR is source-only cutover closeout and may merge only after the verified destination squash merge.
- Do not create a competing placeholder client or claim a complete canonical workspace before the controlled migration/cutover.
- `FND-ID-01` gates identifier meanings required by protocol and admission contracts after migration.
- `FND-02`, `FND-03` and `FND-04` independently gate canonical protocol, authoritative runtime and production admission/lease behavior.
- Bounded technical spikes may inform contracts only when reversible, isolated, non-production and explicitly non-canonical.
- `DUR-01` through `DUR-03` remain hard gates before authoritative durable character, item or currency mutation.
- `ANL-01` must be accepted before `DUR-02`/`DUR-03` finalize transactional outbox and critical audit evidence; analytics consumers never replace authoritative invariants.
- `QA-E2E-01` is accepted; its three-tier implementation and named evidence block completion of `VSL-01`, but do not block `FND-01`, `VSL-02` or continued architecture work.
- A spike, placeholder crate or passing compile does not prove a public contract or runtime capability.

## Stage A — foundation and layer gates

### `FND-01` — Workspace, Dependency and Existing-Rust Migration Contract

- Status: `BLOCKS_WORKSPACE_BOOTSTRAP`
- Decide the smallest initial applications, services and crates; every member requires an immediate consumer and observable acceptance.
- Inventory the exact existing Rust client workspace at a pinned SHA and classify every reusable crate/subsystem as migrate, rename, merge, split, rewrite, reference-only or drop.
- Apply the binding ADR-0008 classification `protocol-canary = REFERENCE_ONLY`; it may not become a destination production workspace member, dependency, adapter, negotiation candidate, fallback or translation layer.
- Freeze legal dependency directions and forbidden edges in a retained machine-readable boundary contract.
- Assign canonical ownership of identifiers, domain contracts, protocol schemas, world/content schemas and fixtures.
- Select Rust edition, Cargo resolver, pinned toolchain, `rust-version`, root lockfile, workspace metadata/lint inheritance, exact target triples, feature policy and baseline CI.
- Define product-realistic target/feature builds in addition to supplemental all-feature checks.
- Define machine-enforced `cargo metadata --locked` dependency-graph and forbidden-edge checks.
- Keep domain/simulation independent from transports, SQL, renderer and UI.
- Keep world/content schema independent from Tauri, editor UI and renderer implementation.
- Treat the wider crate list as a capability horizon, not an instruction to create empty layering crates.
- End with `VSL-02` as the next gate; do not bootstrap a competing destination client.

### `VSL-02` — Exact Rust Client Migration and Cutover Contract

- Status: `BLOCKS_WORKSPACE_BOOTSTRAP`
- Execute immediately after `FND-01` and before `FND-ID-01`, `FND-02`, `FND-03` or `FND-04`.
- Pin the exact cutover source SHA from `blakinio/otclient/oteryn-client` and reconcile every open PR, active task and post-inventory source change.
- Define provenance/history traceability compatible with squash merge, path mapping, exclusions, migration dispositions, source-repository freeze, destination ownership, cutover and rollback.
- Require one atomic destination PR containing the accepted import, canonical root-workspace creation/completion, dependency enforcement, complete ADR-0008 exclusion of `protocol-canary` from production and complete validation.
- Prove the migrated client and complete workspace build/test matrix on the exact destination head before the later source-marker PR marks the source non-canonical.
- Prefer excluding Canary adapter source from the destination product tree; any evidence retained under ADR-0008 must be outside Cargo workspace membership and release packaging with pinned provenance and license treatment.
- Use one atomic destination task/branch/PR in Oteryn-v2 and one later source-marker task/branch/PR in otclient under one coordination ID and explicit rollout/rollback order.
- Preserve exact source SHA/range and machine-readable provenance/path mapping; do not claim source commits became destination mainline ancestry through squash merge.
- Do not claim target-runtime readiness merely because source files exist in the destination.

### `FND-ID-01` — Foundation Identifier Vocabulary

- Status: `BLOCKS_LAYER_IMPLEMENTATION` for protocol and admission schemas.
- Begin only after the accepted client migration/cutover and destination workspace bootstrap/completion.
- Freeze semantic ownership, scope, uniqueness, reuse, durability and visibility for the minimum cross-boundary identifiers.
- Define canonical comparison and wire/Game Session encoding constraints without prematurely selecting every PostgreSQL column type.
- Include event, operation, transaction, correlation, causation and pseudonymous analytics identities required by ADR-0006.
- Keep full durable/database/item identity representation in `DUR-01`.

### `FND-02` — `protocol-oteryn` v1

- Status: `BLOCKS_LAYER_IMPLEMENTATION`
- Reconcile only with the exact latest merged Platform native contract; record pending PR corrections as unresolved, not canonical.
- Populate the machine-readable cross-repository contract lock with merged commit, schema revision/hash, producers, consumers and rollout order.
- Freeze transport, TLS/ALPN, framing, schema/IDL, hard limits and error vocabulary.
- Separate protocol, capability, content and ruleset revisions.
- Define sequencing, command IDs, replay/idempotency, snapshots, deltas, reconciliation and reconnect/resume.
- Produce shared golden fixtures and downgrade protection.
- Register every externally controlled size/depth/count and map stable failures into the common resource-limit and error vocabularies.
- Gate canonical wire schemas/codecs and production compatibility claims, not migration/bootstrap evidence.
- Preserve ADR-0008: no Canary opcode, packet, negotiation, fallback or translation compatibility is a protocol requirement.

### `FND-03` — Runtime Execution Contract

- Status: `BLOCKS_LAYER_IMPLEMENTATION`
- Freeze `NodeRuntime`, `WorldServices`, `ChannelRuntime` and `InstanceRuntime` responsibilities.
- Define tick/timer model, monotonic/wall-clock ownership, skew tolerance, deterministic test clocks, command ordering, bounded queues, overload/backpressure and parallel work return to the logical writer.
- Define channel lifecycle, draining, checkpoint, crash recovery and replay boundaries.
- Define versioned event emission, bounded gameplay-telemetry queues and fail-open/fail-closed behavior by durability class.
- Prove named foundation failure scenarios for overload, dependency loss, stale generations, split ownership and recovery.
- Gate authoritative runtime behavior, not compile-only interfaces or isolated spikes.

### `FND-04` — Identity, Game Session, Admission and Character Lease

- Status: `BLOCKS_LAYER_IMPLEMENTATION`
- Consume `FND-ID-01` meanings without redefining them.
- Freeze token/session representation, issuer/audience, key rotation, replay prevention and revocation.
- Define world/channel/revision binding, reconnect windows and admission errors.
- Define `session_generation`, lease storage/timings, duplicate login, stale-writer fencing and safe channel switching.
- Define Platform, Gateway, PostgreSQL and network failure behavior using the shared failure-scenario catalogue and stable public/internal error mapping.
- Gate production admission and lease behavior, not migration/bootstrap evidence.

## Stage B — blocks durable gameplay

### `DUR-01` — Durable Identifier Representation Contract

- Status: `BLOCKS_DURABLE_GAMEPLAY`
- Cover account, character, world, channel, instance, node, game session, command, entity, item-instance, event, operation, transaction, correlation, causation, analytics-actor and revision identities.
- Decide global versus scoped uniqueness, wire/database/public representation and reuse rules.

### `DUR-02` — Persistence v1

- Status: `BLOCKS_DURABLE_GAMEPLAY`
- Freeze schema and migration ownership.
- Define character revisions/fencing, lease schema, checkpoint boundaries and maximum accepted progress loss.
- Define isolation, locking, retries, idempotency, transactional outbox publication/recovery and critical audit/journal scope.
- Keep best-effort gameplay telemetry separate from atomic item/currency/security evidence.
- Define backup, PITR, restore tests, RPO/RTO and compatible rollout/rollback.

### `DUR-03` — Item Transaction and Anti-Duplication Invariants

- Status: `BLOCKS_DURABLE_GAMEPLAY`
- Define item instance identity and ownership.
- Freeze inventory/equipment/container/ground transfer transaction boundaries.
- Define pickup, drop, loot, trade, bank, depot, market, mail, reward, split, merge, transform, currency and retry semantics.
- Produce atomic provenance evidence and deterministic conservation/single-location invariants consumable by Game Intelligence.
- Prove that duplicate commands, crashes, stale sessions and partial failures cannot duplicate items or currency.

This may be part of Persistence v1 only if that contract is sufficiently complete and independently auditable.

### `DUR-04` — Content, World Detail and Scripting Contract

- Status: `BLOCKS_DURABLE_GAMEPLAY` for broad import and scripted durable behavior.
- Continue ADR-0005 with exact World Project and World Bundle encodings.
- Freeze Content Registry package/version/dependency behavior.
- Pin migration source revisions and classify source areas as `COPY`, `CONVERT`, `REWRITE`, `REFERENCE_ONLY` or `REJECT`.
- Select scripting runtime and define capabilities, limits, failure isolation, persistence access and hot reload.
- Preserve asset rights and provenance gates.
- Require a bounded format/compiler/loader spike before final encoding selection, including deterministic hashes, random access, corruption/decompression failure, round-trip equivalence and measured chunk/floor packing. No compatibility or canonical-format claim may escape the spike.

### `ANL-01` — Game Event and Audit Foundation

- Status: `BLOCKS_DURABLE_GAMEPLAY`
- Applies before `DUR-02`/`DUR-03` finalize transactional audit evidence.
- Freeze the common event envelope, durability classes, producers/consumers, ordering, idempotency, outbox, publication checkpoints, deduplication, replay and schema compatibility.
- Define bounded fail-open gameplay telemetry separately from atomic durable economy/security audit.
- Define privacy classes, pseudonymous analytics identity, access, retention, deletion/anonymization and test fixtures.

### `ANL-02` — Gameplay, Balance and World Analytics

- Status: `REQUIRED_FOR_ALPHA`
- Define combat/progression/session metrics, class/vocation balance, party/hunt dimensions, world/content usage, sample quality, version comparisons and dashboards.
- Keep analytics observational; no automatic balance mutation.

### `ANL-03` — Economy Integrity and Security Analytics

- Status: `REQUIRED_FOR_ALPHA`
- Required for a production-grade integrity/security analytics claim.
- Define provenance/invariant consumers, alerts, cases, detector versioning, evidence quality, false-positive handling and separation from authoritative enforcement.
- Preserve `DUR-03` as the anti-duplication prevention authority.

### `ANL-04` — Read-Only Investigation and AI

- Status: `EXPANSION`
- Define least-privilege read-only evidence access, correlation, provenance reconstruction, human review and full auditability.
- Prohibit runtime/database mutation, autonomous bans, balance changes, rollback and deployment.

## Registered gameplay and product decision horizon

Detailed scope, dependencies and non-decisions are canonical in `GAMEPLAY_AND_PRODUCT_ARCHITECTURE_HORIZON.md`.

### Blocks durable gameplay

- `GAME-CHAR-01` — Character Lifecycle and Progression. Must precede the final `DUR-02` character schema.
- `GAME-ITEM-01` — Item Model and Equipment Rules. Must precede the final `DUR-03` item transaction model.

### Required for Playable Alpha completeness

- `GAME-ABILITY-01` — Ability, Spell and Condition Architecture.
- `GAME-AI-01` — Creature AI, Spawn and Pathfinding Architecture.
- `GAME-INTERACTION-01` — World Interaction and Environmental Mechanics.
- `PROD-LIVEOPS-01` — Live Operations and Runtime Configuration.
- `PROD-COMPAT-01` — Release Compatibility and Version Train.
- `SEC-CLIENT-01` — Client Integrity and Anti-Cheat Boundary.
- `DATA-PRIVACY-01` — Product Privacy and Data Lifecycle.
- `UX-I18N-A11Y-01` — Localization, Input, Onboarding and Accessibility.
- `OPS-GM-01` — Support, Moderation and GM Operations.

### Expansion or deferred

- `GAME-META-01` — Collections, Achievements and Recurring Progression (`EXPANSION`).
- `GAME-INSTANCES-01` — Dungeons, Arenas, Matchmaking and Spectating (`EXPANSION`).
- `GAME-WORLD-LIFECYCLE-01` — World Lifecycle, Transfer and Merge (`EXPANSION`).
- `INTEGRATION-API-01` — External APIs, Notifications and Integrations (`EXPANSION`).
- `PROD-ENTITLEMENTS-01` — Entitlements, Premium and Commerce Boundary (`DEFERRED`).
- `MOD-ECOSYSTEM-01` — Modding and Plugin Ecosystem (`DEFERRED`).

Registration prevents omission; it does not accept technologies, formulas, schemas, service topology or implementation.

## Stage C — blocks the foundation vertical slice

### `QA-E2E-01` — Native End-to-End Test Platform

- Status: contract `ACCEPTED`; required implementation evidence `BLOCKS_VERTICAL_SLICE`.
- Canonical source: ADR-0007.
- Use one shared manifest-driven platform with Tier 1 headless system E2E, Tier 2 instrumented native-client E2E and Tier 3 production-binary smoke E2E.
- Preserve exact revision/artifact identity, deterministic controls, first-divergence evidence, cleanup certification and every counted physical attempt.
- Prohibit hidden retry-until-green and overclaiming headless, instrumented or environment-only evidence.
- Implement incrementally after the canonical client migration and the relevant protocol, admission, persistence and content contracts exist.
- Require the minimum evidence named in ADR-0007 before `VSL-01` may be called complete.

### `VSL-01` — Foundation Vertical-Slice Programme

- Status: `BLOCKS_VERTICAL_SLICE`
- Name owners, implementation order, repositories, branches/PRs and exact observable E2E.
- Include Platform authentication, Gateway-issued Game Session, server admission, lease, minimal native map, movement, two-player visibility, combat, death, loot, retry-safe pickup, correlated game events, atomic durable item audit, replay-safe analytics, checkpoint, logout, cross-channel relog, duplicate-login rejection and channel/world-state isolation.
- Consume the accepted `QA-E2E-01` tiers and evidence contract; do not invent a feature-owned E2E runner or count environment startup as product proof.

### `VSL-MOVE-01` — Movement, Collision and Visibility Contract

- Status: `BLOCKS_VERTICAL_SLICE`
- Freeze position/direction representation, movement ordering, diagonal rules, collision, floor transitions and teleport behavior.
- Define interest management, snapshots/deltas, view range and lag/reconciliation behavior.
- Keep legality server-authoritative.

### `VSL-COMBAT-01` — Minimal Combat, Death and Loot Contract

- Status: `BLOCKS_VERTICAL_SLICE`
- Define action ordering, target legality, damage pipeline boundary, death, corpse, loot ownership, experience/kill attribution and retry-safe pickup integration.
- Preserve extension points for conditions, PvP, boss contribution and rulesets.

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
- Consume `QA-E2E-01` rather than creating a second E2E platform.
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
- Rate limits, anti-replay, deterministic enforcement, trusted admin/GM commands, least privilege, sanctions, secret management, dependency security and supply-chain controls.
- Consume ADR-0006/`ANL-03` alerts and cases without treating anomaly scores as autonomous enforcement authority.

### `EXP-UPDATE-01` — Launcher, Updater and Release Distribution

- Status: `DEFERRED`
- Signed manifests, delta updates, rollback, channels, asset/client compatibility, CDN/mirrors and archive/path/decompression safety.

### `EXP-OPS-01` — Deployment and Operations

- Status: `EXPANSION`
- Deployment topology, environments, configuration, secrets, discovery, health/readiness, draining, rolling updates, migrations, backup and disaster recovery.

### `EXP-OBS-01` — Observability

- Status: `EXPANSION`
- Structured logs, low-cardinality metrics, tracing, correlation IDs, tick/queue/channel health, DB latency, protocol errors, privacy, retention and alerts.
- Remain distinct from high-cardinality gameplay analytics and transactional economy/security audit defined by ADR-0006.

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
7. After the client migration/cutover, enforce accepted dependency boundaries with executable CI checks in the destination workspace.
8. Cross-repository locks accept only merged canonical commits and immutable schema identifiers; mutable PR heads remain pending evidence.
9. Public contracts must register applicable resource limits, stable error categories and named failure scenarios.
10. A decision is not complete until its PR is validated, audited, squash-merged and its task archived.
11. FND-01 must terminate into VSL-02; no isolated workspace bootstrap may bypass the accepted client cutover sequence.
12. VSL-02 uses one atomic Oteryn-v2 destination PR; the later otclient PR is source-marker closeout only.
13. Every gameplay/product package must reconcile `GAMEPLAY_AND_PRODUCT_ARCHITECTURE_HORIZON.md`; an unnamed domain may not be silently absorbed into an unrelated gate.
14. Every vertical-slice or client-visible package must consume `QA-E2E-01`; it may add scenarios and assertions but not a competing E2E lifecycle/evidence platform.
15. ADR-0008 is binding on `FND-01`, `VSL-02`, `FND-02` and all later client/server packages; no task may reintroduce Canary into production through an optional feature, fallback, compatibility listener or intermediate translation model.

## Current next action

Draft and accept `FND-01` — the **Workspace, Dependency and Existing-Rust Migration Contract**. Its terminal next action is `VSL-02`, followed by one atomic destination migration/workspace PR, the source-only cutover marker and only then layer-specific contracts. The registered gameplay/product gates remain ordered future work and do not replace this immediate action.
