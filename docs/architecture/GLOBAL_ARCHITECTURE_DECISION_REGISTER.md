# Oteryn v2 Global Architecture Decision Register

- Status: Active coordination register
- Date: 2026-08-11
- Coordination ID: `OTV2-GLOBAL-ARCHITECTURE`
- Canonical foundation programme: `docs/agents/tasks/active/OTV2-20260805-foundation-preimplementation-contracts.md`
- Current execution status: `docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md`
- Coordinator prompt: `docs/agents/prompts/OTV2_GLOBAL_ARCHITECTURE_DECISION_COORDINATOR.md`

## Purpose

Preserve the complete global architecture decision horizon for Oteryn v2 without requiring chat history and without forcing distant expansion features to be designed before they become relevant.

This register is a coordination source, not an implementation claim. Accepted decisions live in ADRs and contracts. The ordered foundation gates live in `FOUNDATION_DECISION_BACKLOG.md`. This file ensures that global project domains are not lost while the programme resolves them in stages. The detailed open gameplay and product horizon is retained in `GAMEPLAY_AND_PRODUCT_ARCHITECTURE_HORIZON.md`.

Stable IDs are canonical across tasks, prompts and PRs. Stage labels are descriptive only.

## Status vocabulary

- `ACCEPTED` — frozen by an accepted ADR/contract.
- `BLOCKS_WORKSPACE_BOOTSTRAP` — must be accepted before creating or completing the canonical root Cargo workspace.
- `BLOCKS_LAYER_IMPLEMENTATION` — must be accepted before canonical production implementation of the named layer.
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
| GameNode process identity, multithreaded single-writer execution, measured capacity, external orchestration and same-channel fenced recovery baseline | `ACCEPTED` | ADR-0009 |
| Reference and evolved world product profiles over one engine, client and `protocol-oteryn`, with isolated world-scoped gameplay value | `ACCEPTED` | ADR-0010 and `PRODUCT_DIRECTION_BASELINE.md` |
| Fail-closed native client state before the accepted gameplay protocol exists | `ACCEPTED` | ADR-0011 |
| Native Character Authority owns CharacterId/lifecycle/account-character ownership while Platform owns AccountId and orchestration | `ACCEPTED` | ADR-0012 and `CHARACTER_AUTHORITY_PLATFORM_BOUNDARY.md` |
| Platform database technology is independent from native game PostgreSQL; a Platform PostgreSQL migration requires separate evidence-backed Platform authority | `ACCEPTED` | ADR-0013 |
| Dual gameplay transport strategy keeps TCP+TLS profile 1 as the initial/default architecture intent and QUIC as a future player-opt-in target without creating a second application protocol | `ACCEPTED` | ADR-0014 and `../contracts/PROTOCOL_OTERYN_TRANSPORT_POLICY.json` |
| GameNode remains one game-server process under ADR-0009; internal module/crate decomposition is not frozen and distinct adjacent services require their own evidence-backed boundary | `ACCEPTED` | ADR-0015 with ADR-0009 |
| Gameplay transport mode vocabulary does not imply runtime readiness; all gameplay transport modes remain unavailable until their concrete path is implemented and proven | `ACCEPTED` | ADR-0016 and `../contracts/PROTOCOL_OTERYN_TRANSPORT_POLICY.json` |
| FND-01 workspace contract, VSL-02 migration contract, canonical 19-member destination cutover and source-only closeout | `ACCEPTED` | `FND-01_WORKSPACE_AND_RUST_MIGRATION_CONTRACT.md`, `VSL-02_RUST_CLIENT_MIGRATION_AND_CUTOVER_CONTRACT.md`, destination merge `78988f72a80cc904aa9176ae850c50d4efa0b0f0`, otclient #274/#275 |
| Minimum cross-boundary foundation identifier contract | `ACCEPTED` | `FND-ID-01_FOUNDATION_IDENTIFIER_CONTRACT.md`, merge `2c584543cd1e3758958755478a6cc6ed3d39a8a9` |

## Progressive execution policy

- `FND-01`, `VSL-02`, the destination migration and the source-only historical/non-canonical closeout are complete.
- `FND-ID-01`, `FND-02`, `FND-03`, `FND-04`, `DUR-01`, `ANL-01` and the dual-transport architecture strategy are accepted/lifecycle-closed where recorded by `FOUNDATION_PROGRAMME_CURRENT_STATUS.md`; their implementation status remains separate and is largely `NOT_STARTED`.
- The migrated client remains in ADR-0011 `pre-native-protocol`: it launches and fails closed before gameplay credential consumption, routing or gameplay transport.
- `protocol-canary` remains absent from the destination production graph and cannot re-enter as a production adapter/fallback without an explicit superseding owner decision.
- ADR-0009 fixes the GameNode/process/container boundary and recovery invariants; `PERF-01` gates supported capacity claims and `OPS-CHANNEL-01` gates automatic production channel scaling and claimed production recovery behavior.
- `GAME-VISION-01` minimum product-vision semantics are accepted in `GAME-VISION-01_MINIMUM_OWNER_BASELINE.md`: Reference-first/hybrid tracking, product promise, reliability/UX-first Evolved ordering, PvP-secondary, solo-viable/party-rewarded, Reference parity precedence, accepted core/session/long-term loop, Reference-rule-first economy/scarcity and category-level success evidence now form the minimum product direction.
- The exact first Global Tibia Reference baseline remains `DEFERRED WITH HARD GATE`: any broad Reference mechanics/content implementation or final parity fixtures requiring concrete semantics must stop until that baseline is explicitly selected; baseline-neutral architecture analysis may proceed without guessing.
- `GAME-CHAR-01` is the next product-sensitive architecture gate and must precede final character-bearing `DUR-02`; `GAME-CHANNEL-01` and bounded `DUR-02` discovery may proceed in parallel within their existing boundaries.
- ADR-0012 fixes native Character Authority versus Platform lifecycle/orchestration ownership without authorizing runtime or persistence implementation.
- ADR-0013 removes Platform database migration from the native gameplay critical path while preserving PostgreSQL for native game persistence and all ADR-0004 ownership/least-privilege invariants.
- ADR-0014 accepts TCP-default/future-QUIC dual transport only as architecture direction; ADR-0016 keeps every gameplay transport runtime mode unavailable until implemented and proven, and ADR-0015 preserves ADR-0009's one-process GameNode identity while leaving only internal decomposition and genuinely distinct adjacent-service placement evidence-driven.
- bounded technical spikes may inform contracts only when reversible, isolated, non-production and explicitly non-canonical.
- `DUR-02` and `DUR-03` remain hard gates before their named authoritative durable gameplay mutation scopes; `DUR-03` also waits for `GAME-ITEM-01`.
- accepted `ANL-01` event/audit semantics constrain `DUR-02`/`DUR-03` transactional outbox and critical audit evidence; analytics consumers never replace authoritative invariants.
- `QA-E2E-01` is accepted; its three-tier implementation and named evidence block completion of `VSL-01`.
- a spike, placeholder crate or passing compile does not prove a public contract or runtime capability.

## Stage A — foundation and layer gates

### `GAME-VISION-01` — Product Vision, Parity Scope and World Profile Contract

- Status: `ACCEPTED` for the minimum product-vision gate scope; implementation `NOT_STARTED`.
- Canonical minimum owner source: `GAME-VISION-01_MINIMUM_OWNER_BASELINE.md`, incorporating the seven earlier dedicated owner baselines.
- Accepted first external evaluation is Reference-first; released Reference revisions are immutable while upstream evidence may be observed continuously and explicitly promoted into later revisions.
- Accepted player/product direction preserves recognizable Tibia depth and persistent-world identity, modern native quality, explicit/versioned/measurable intentional differences, `PvP = secondary pillar`, `solo viable, party rewarded`, and reliability/UX-first initial Evolved differentiation.
- Accepted core loop is player-chosen goal -> preparation -> risk/activity -> secure committed progress/value -> recovery/planning across persistent character/equipment/exploration/social/prestige horizons.
- Accepted economy direction is Reference mechanical source/sink parity rather than historical market-price/supply parity, with conservation before tuning, measurable provenance, semantic scarcity and no hidden macro tuning.
- Accepted success categories are Reference correctness, player interaction quality, progress/value trust, core-loop health, economy health and product/operational health; numeric targets remain milestone-owned.
- Preserve one engine, client and `protocol-oteryn`; differences remain versioned ruleset/content/product profiles and distinct logical worlds.
- Preserve default world-scoped character/economy isolation until a dedicated transfer contract proves safety.
- Exact first Global Tibia patch/date/behavior baseline is deliberately deferred but is a hard blocker before broad Reference mechanics/content or final parity fixtures that require concrete target semantics.
- Exact gameplay/economy formulas, numeric KPI thresholds, branding, monetization, exact first Evolved feature inventory and LiveOps cadence remain downstream/deferred under their named gates.
- Acceptance of this gate authorizes no runtime, persistence, content or production implementation by itself.

### `FND-01` — Workspace, Dependency and Existing-Rust Migration Contract

- Status: `ACCEPTED`; contract archived and applied by the atomic destination cutover merged as `78988f72a80cc904aa9176ae850c50d4efa0b0f0`.
- The retained contract remains canonical evidence for workspace membership, dependency directions, toolchain/build matrix and migration dispositions.
- It no longer blocks later foundation architecture work.

### `VSL-02` — Exact Rust Client Migration and Cutover Contract

- Status: `ACCEPTED AND COMPLETE`.
- Destination migration and source-only moved/non-canonical closeout are complete.
- The historical contract remains canonical evidence for provenance, cutover and rollback rules but is not an active blocker.

### `FND-ID-01` — Foundation Identifier Vocabulary

- Status: `ACCEPTED`.
- Canonical source: `FND-ID-01_FOUNDATION_IDENTIFIER_CONTRACT.md`.
- Minimum catalogue: `AccountId`, `CharacterId`, `WorldId`, `ChannelId`, `NodeId`, `InstanceId`, `PartyId`, `GameSessionId` and the accepted conditional `HandoffId`.
- Preserve accepted owner/issuer/scope/lifecycle/visibility/strong-typing semantics and the rule that identity does not itself grant authority.
- `AdmissionId` and `CharacterLeaseId` are not part of the accepted foundation catalogue; FND-04 may request a narrow amendment only if it proves a separate semantic lifecycle is required.
- `CommandId`, command sequencing/acknowledgement, wire/IDL/byte order and compact session-handle encoding belong to `FND-02`.
- runtime-local entity/worker/task/generational handles and ownership-generation mechanics belong to `FND-03`.
- admission/session/lease/reconnect/takeover state machines belong to `FND-04`.
- physical database representation and later durable-domain identifiers belong to `DUR-*`.
- event/operation/transaction/correlation/causation/pseudonymous analytics identity catalogues belong to `ANL-*`/durability contracts as appropriate.
- protocol/ruleset/content revisions and connection/ownership generations are revision/fencing values, not foundation entity identities.

### `FND-02` — `protocol-oteryn` v1

- Status: `ACCEPTED`; architecture lifecycle closed, implementation `NOT_STARTED`.
- Canonical source: `FND-02_PROTOCOL_OTERYN_V1_CONTRACT.md` plus current registries and reconciliation baseline.
- Consume `FND-ID-01` meanings without redefining them.
- Preserve one project-owned application protocol, transport/security/framing/schema limits, stable error vocabulary, CommandId/sequencing/replay/reconciliation semantics and independent wire-evidence requirements already accepted.
- TCP+TLS transport profile `1` is registered architecturally; no gameplay listener/adapter/runtime path is implied.
- Preserve ADR-0008: no Canary opcode, packet, negotiation, fallback or translation compatibility is a protocol requirement.

### `FND-03` — Runtime Execution Contract

- Status: `ACCEPTED`; architecture lifecycle closed, implementation `NOT_STARTED`.
- Canonical source: `FND-03_RUNTIME_EXECUTION_CONTRACT.md` and accepted analysis baselines.
- Preserve `NodeRuntime`, `WorldServices`, `ChannelRuntime` and `InstanceRuntime` ownership responsibilities, one logical writer, ownership generations, RuntimeExecutionOrdinal, bounded queues/backpressure, test clocks, fail-closed stale work and recovery semantics.
- ADR-0009 remains binding that one `GameNode` is one game-server process; internal decomposition remains evidence-driven under ADR-0015.

### `FND-04` — Identity, Game Session, Admission and Character Lease

- Status: `ACCEPTED`; architecture lifecycle closed, implementation `NOT_STARTED`.
- Canonical sources include `FND-04A_AUTHORITY_FRESH_ADMISSION_CONTRACT.md`, `FND-04B_RECONNECT_RECOVERY_CONTINUITY_CONTRACT.md` and `FND-04C_ERROR_DIAGNOSTICS_FAILURE_COMPATIBILITY_CONTRACT.md`.
- Preserve ownership-before-world admission, purpose-separated grants, GameSession/lease fencing, PREPARE/COMMIT recovery, healthy-binding non-preemption, ControlLossEpoch, accepted four-second eligible defensive PvE re-entry protection and fail-closed stale authority.
- No production admission/session/lease runtime is implied by architecture acceptance.

## Stage B — blocks durable gameplay

### `DUR-01` — Durable Identifier Representation Contract

- Status: `ACCEPTED`; architecture lifecycle closed, implementation `NOT_STARTED`.
- Canonical source: `DUR-01_DURABLE_IDENTIFIER_REPRESENTATION_CONTRACT.md`.
- UUIDv7 native durability uses PostgreSQL `uuid`; persisted uint64 CommandId uses `numeric(20,0)`; ItemInstanceId is game-owned UUIDv7; legacy imports use stable source namespace identity.
- Accepted representation does not itself create physical tables or authorize migrations.

### `DUR-02` — Persistence v1

- Status: `BLOCKS_DURABLE_GAMEPLAY`.
- Bounded discovery may proceed from accepted DUR-01 + ANL-01, but final character-bearing schema waits for accepted `GAME-CHAR-01`.
- Freeze schema and migration ownership.
- Define character revisions/fencing, lease schema, checkpoint boundaries and maximum accepted progress loss.
- Define isolation, locking, retries, idempotency, transactional outbox publication/recovery and critical audit/journal scope.
- Keep best-effort gameplay telemetry separate from atomic item/currency/security evidence.
- Define backup, PITR, restore tests, RPO/RTO and compatible rollout/rollback.

### `DUR-03` — Item Transaction and Anti-Duplication Invariants

- Status: `BLOCKS_DURABLE_GAMEPLAY`.
- Waits for accepted `DUR-02`, `GAME-ITEM-01` and ANL-01 evidence semantics.
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

- Status: `ACCEPTED`; architecture lifecycle closed, implementation `NOT_STARTED`.
- Canonical sources: `ANL-01_GAME_EVENT_AND_AUDIT_FOUNDATION_CONTRACT.md`, `docs/contracts/game-events/v1/foundation.proto`, `GAME_EVENT_FOUNDATION_REGISTRY.json` and ANL resource-limit entries.
- Preserve the minimal common envelope + typed/versioned event-family payload model, explicit durability classes, stable identity/correlation semantics, at-least-once publication with EventId-stable idempotency, privacy/pseudonymization rules and atomic durable mutation evidence requirements.
- No event table, outbox implementation, broker, collector, detector, warehouse or production collection is implied.

### `ANL-02` — Gameplay, Balance and World Analytics

- Status: `REQUIRED_FOR_ALPHA`.
- Define combat/progression/session metrics, class/vocation balance, party/hunt dimensions, world/content usage, sample quality, version comparisons and dashboards.
- Keep analytics observational; no automatic balance mutation.

### `ANL-03` — Economy Integrity and Security Analytics

- Status: `REQUIRED_FOR_ALPHA`.
- Required for a production-grade integrity/security analytics claim.
- Define provenance/invariant consumers, alerts, cases, detector versioning, evidence quality, false-positive handling and separation from authoritative enforcement.
- Preserve `DUR-03` as the anti-duplication prevention authority.

### `ANL-04` — Read-Only Investigation and AI

- Status: `EXPANSION`.
- Define least-privilege read-only evidence access, correlation, provenance reconstruction, human review and full auditability.
- Prohibit runtime/database mutation, autonomous bans, balance changes, rollback and deployment.

## Registered gameplay and product decision horizon

Detailed scope, dependencies and non-decisions are canonical in `GAMEPLAY_AND_PRODUCT_ARCHITECTURE_HORIZON.md`.

### Blocks durable gameplay

- `GAME-CHAR-01` — Character Lifecycle and Progression. **Next product-sensitive architecture gate**; must precede the final `DUR-02` character schema.
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
- Implement incrementally after the relevant protocol, admission, persistence and content contracts exist.
- Require the minimum evidence named in ADR-0007 before `VSL-01` may be called complete.

### `VSL-01` — Foundation Vertical-Slice Programme

- Status: `BLOCKS_VERTICAL_SLICE`.
- Name owners, implementation order, repositories, branches/PRs and exact observable E2E.
- Include Platform authentication, one-time ticket/Gateway routing, bounded pre-admission material, game-domain canonical Game Session establishment, server admission, lease, minimal native map, movement, two-player visibility, combat, death, loot, retry-safe pickup, correlated game events, atomic durable item audit, replay-safe analytics, checkpoint, logout, cross-channel transition/relog, duplicate-session rejection and channel/world-state isolation.
- Consume the accepted `QA-E2E-01` tiers and evidence contract; do not invent a feature-owned E2E runner or count environment startup as product proof.

### `VSL-MOVE-01` — Movement, Collision and Visibility Contract

- Status: `BLOCKS_VERTICAL_SLICE`.
- Freeze position/direction representation, movement ordering, diagonal rules, collision, floor transitions and teleport behavior.
- Define interest management, snapshots/deltas, view range and lag/reconciliation behavior.
- Keep legality server-authoritative.

### `VSL-COMBAT-01` — Minimal Combat, Death and Loot Contract

- Status: `BLOCKS_VERTICAL_SLICE`.
- Define action ordering, target legality, damage pipeline boundary, death, corpse, loot ownership, experience/kill attribution and retry-safe pickup integration.
- Preserve extension points for conditions, PvP, boss contribution and rulesets.

### `VSL-CONTENT-01` — Minimal Native Map, Compiler and Loader Contract

- Status: `BLOCKS_VERTICAL_SLICE`.
- Define the minimum World Project, World Bundle, compiler, loader and content keys required by the vertical slice.
- Avoid prematurely implementing the complete Studio or all legacy importers.

## Stage D — required for a playable alpha

### `ALPHA-CLIENT-01` — Client Architecture

- Status: `REQUIRED_FOR_ALPHA`.
- Renderer, UI, input, networking, client state, prediction/reconciliation, assets, audio, settings, accessibility and crash reporting boundaries.
- Decide which low-level renderer components may be shared with Oteryn Studio.

### `ALPHA-RULESET-01` — Ruleset Architecture

- Status: `REQUIRED_FOR_ALPHA`.
- Define which classic/modern differences are data/policy and which use capability-bounded domain modules.
- Cover combat, movement, regeneration, death penalties, professions, item behavior and client presentation without protocol forks.

### `ALPHA-CONTENT-01` — NPC, Quests, Shops and Content Runtime

- Status: `REQUIRED_FOR_ALPHA`.
- Define content execution APIs, state ownership, deterministic tests, error isolation and authoring workflow.

### `ALPHA-QUALITY-01` — Testing and Performance Baselines

- Status: `REQUIRED_FOR_ALPHA`.
- Consume `QA-E2E-01` rather than creating a second E2E platform.
- Unit, property, fuzz, integration, golden protocol, deterministic simulation, database concurrency, crash recovery, multichannel, E2E, soak and migration tests.
- Consume `PERF-01` for players/channel, channels/GameNode, logical-world capacity, tick/scheduling budget, latency, reconnect, memory, startup and database-throughput targets.

### `PERF-01` — Capacity, Performance and Scalability

- Status: `REQUIRED_FOR_ALPHA`.
- Define named reference hardware and supported deployment cells.
- Freeze latency, scheduling, queue-age, resource and overload service objectives.
- Use representative movement, hunting, crowd, mass-combat, raid, reconnect-storm, persistence, noisy-neighbor, recovery and soak workloads.
- Establish separate channel, GameNode and logical-world capacity limits with accepted safety headroom.
- Record exact artifact, content, ruleset, protocol, topology and first violated objective for every supported claim.
- Block published production capacity claims and representative-load readiness until reproducible evidence exists.

### `OPS-CHANNEL-01` — GameNode Deployment and Dynamic Channel Orchestration

- Status: `BLOCKS_LAYER_IMPLEMENTATION`.
- Blocks automatic production channel scaling and claimed production GameNode/channel recovery behavior.
- Define process/container packaging, external orchestrator authority, registration, health, readiness and capacity reporting.
- Freeze channel placement, lifecycle, dynamic creation, hysteresis, draining and closure.
- Define ownership-generation fencing, replacement, checkpoint, bounded replay, fresh-session reconnect and full-snapshot recovery.
- Set reconnect grace, RPO, RTO, blast-radius and recovery-concurrency policy from named evidence.
- Preserve the initial prohibition on active-channel live migration and silent failover to a different channel.

### `ALPHA-MILESTONE-01` — Product Milestone Contract

- Status: `REQUIRED_FOR_ALPHA`.
- Define Foundation, Playable Alpha, Beta and Release scopes with observable outcomes and excluded systems.
- Prevent broad parallel implementation without a complete vertical slice.

## Stage E — expansion systems

### `EXP-EVENTS-01` — Dynamic Events, Raids and Bosses

- Status: `EXPANSION`.
- Scheduler, channel-local/world-shared uniqueness, cooldowns, anti-hopping, participation, scaling, persistence, crash recovery and reward ownership.
- Use ADR-0005 `EncounterZone`/`RaidCell`/`RaidAnchor` geometry.

### `EXP-HOUSES-01` — Houses

- Status: `DEFERRED`.
- Preserve one authoritative house state per world and anti-duplication invariants.
- Later decide presence/topology, channel access, simulation owner, rent, auctions, doors, beds, guest lists and crash recovery in a dedicated ADR.

### `EXP-SOCIAL-01` — Party, Guild, Chat, Friends and Presence

- Status: `EXPANSION`.
- Define world/channel/cross-world scope, consistency, shared experience, buffs, rosters, messaging, offline delivery and presence.

### `EXP-ECONOMY-01` — Market, Trade and Economy

- Status: `EXPANSION`.
- Define offers, escrow, fees, partial fills, cancellation, delivery, concurrency, audit and fraud/duplication resistance.

### `EXP-SECURITY-01` — Security, Abuse and Administration

- Status: `EXPANSION` with foundation requirements applied earlier where relevant.
- Rate limits, anti-replay, deterministic enforcement, trusted admin/GM commands, least privilege, sanctions, secret management, dependency security and supply-chain controls.
- Consume ADR-0006/`ANL-03` alerts and cases without treating anomaly scores as autonomous enforcement authority.

### `EXP-UPDATE-01` — Launcher, Updater and Release Distribution

- Status: `DEFERRED`.
- Signed manifests, delta updates, rollback, channels, asset/client compatibility, CDN/mirrors and archive/path/decompression safety.

### `EXP-OPS-01` — Deployment and Operations

- Status: `EXPANSION`.
- Broader environment, configuration, secret, rolling-update, migration, backup and disaster-recovery architecture.
- Consume `OPS-CHANNEL-01` for GameNode process/container lifecycle, channel placement, dynamic scaling, fencing and reconnect recovery instead of defining a competing control plane.

### `EXP-OBS-01` — Observability

- Status: `EXPANSION`.
- Structured logs, low-cardinality metrics, tracing, correlation IDs, tick/queue/channel health, DB latency, protocol errors, privacy, retention and alerts.
- Remain distinct from high-cardinality gameplay analytics and transactional economy/security audit defined by ADR-0006.

### `EXP-SCALE-01` — Advanced Scaling and Prediction

- Status: `DEFERRED`.
- Live channel migration, partitioning one channel across nodes, hundreds of dynamic channels, advanced client prediction and independently deployable world services.
- QUIC transport evolution is governed by ADR-0014 and its later `NET-TRANSPORT-02` implementation/evidence gate rather than by this scaling gate.

## Programme ownership discipline

The canonical foundation task is a non-owning programme checkpoint. Each substantial gate has one separate task, branch, PR, owner, owned paths and terminal archive. The programme checkpoint records accepted state and exactly one next action; it does not reserve all future architecture paths.

## Decision discipline

1. Resolve gates in dependency order using stable IDs.
2. Do not use this register as permission to implement beyond the progressive execution policy.
3. Update status only when an accepted ADR/contract provides evidence.
4. Every accepted package must update this register narrowly and link the canonical source.
5. Preserve deferred topics and safe extension points without inventing final designs.
6. Initial workspace members require an immediate consumer and acceptance; do not create speculative placeholder crates.
7. Enforce accepted dependency boundaries with executable CI checks in the canonical workspace.
8. Cross-repository locks accept only merged canonical commits and immutable schema identifiers; mutable PR heads remain pending evidence.
9. Public contracts must register applicable resource limits, stable error categories and named failure scenarios.
10. A decision is not complete until its PR is validated, audited, squash-merged and its task archived.
11. Completed historical foundation gates are consumed rather than reopened unless a dedicated superseding decision is explicitly accepted.
12. FND-02/FND-03/FND-04/DUR-01/ANL-01 architecture acceptance does not imply runtime implementation; consume their canonical contracts and current status overlay rather than older progress prose.
13. Every gameplay/product package must reconcile `GAMEPLAY_AND_PRODUCT_ARCHITECTURE_HORIZON.md`; an unnamed domain may not be silently absorbed into an unrelated gate.
14. Every vertical-slice or client-visible package must consume `QA-E2E-01`; it may add scenarios and assertions but not a competing E2E lifecycle/evidence platform.
15. ADR-0008 is binding on all later client/server packages; no task may reintroduce Canary into production through an optional feature, fallback, compatibility listener or intermediate translation model.
16. ADR-0009 is binding on runtime, performance and operations packages: `NodeId` identifies the GameNode process incarnation rather than a physical host, each channel retains one logical writer, capacity claims require `PERF-01`, and automatic production scaling/recovery claims require `OPS-CHANNEL-01`.
17. Every material architecture decision must apply `docs/agents/ARCHITECTURE_DECISION_DISCIPLINE.md`: state whether it must be decided now, name blocked downstream work, identify future constraints and state evidence that would justify supersession.
18. ADR-0013 is binding on persistence/programme planning: PostgreSQL remains the native game target, but Platform database migration is not a native-game prerequisite and requires separate Platform authority and evidence.
19. ADR-0014 through ADR-0016 are binding within their named transport/GameNode-readiness scopes: one `protocol-oteryn` remains authoritative, ADR-0009's one-process GameNode identity remains intact, transport profile registration does not imply runtime readiness, and QUIC cannot activate without its later profile/reconciliation/evidence/implementation gates.
20. `GAME-VISION-01_MINIMUM_OWNER_BASELINE.md` is binding for the minimum product-vision gate. Future gameplay architecture must preserve its accepted loop/economy/success direction, while exact first Reference semantics fail closed on the separately selected named Reference baseline whenever a downstream decision cannot remain baseline-neutral.

## Current next action

Start a separate bounded **paper-only `GAME-CHAR-01` architecture task** as the next product-sensitive gate, consuming the accepted minimum GAME-VISION baseline plus existing character authority/session/persistence invariants. Keep exact Reference formulas/rules unresolved unless the task proves they are required; if a decision cannot remain baseline-neutral, stop that scope on the hard prerequisite to select the exact first Reference baseline rather than guessing.

`GAME-CHANNEL-01` and bounded `DUR-02` discovery may proceed in parallel under their existing gates and without path/ownership collision.

No gameplay/runtime implementation, Platform write or production behavior is authorized by this register update.
