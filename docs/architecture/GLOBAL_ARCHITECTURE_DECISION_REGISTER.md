# Oteryn v2 Global Architecture Decision Register

- Status: Active coordination register
- Date: 2026-08-12
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
| Minimum GAME-VISION product contract | `ACCEPTED` | `GAME-VISION-01_MINIMUM_OWNER_BASELINE.md` plus the seven earlier owner baselines |
| First immutable Reference behavior cut: Global Tibia production-observable behavior after the 2026-07-28 server-save/maintenance boundary | `ACCEPTED` | `GAME-VISION-01_FIRST_REFERENCE_BASELINE_OWNER_BASELINE.md` |
| Baseline-neutral GAME-CHAR Stage-A semantics | `ACCEPTED` | `GAME-CHAR-01_STAGE_A_OWNER_BASELINE.md` |
| GAME-CHAR semantic architecture closure: Stage A + Reference-sensitive Stage B ownership/versioning/migration envelope, with unresolved per-behavior parity gates retained | `ACCEPTED` | `GAME-CHAR-01_STAGE_B_OWNER_BASELINE.md` |
| DUR-02 profile-neutral Character persistence partial baseline | `ACCEPTED` | `DUR-02_PROFILE_NEUTRAL_CHARACTER_PERSISTENCE_OWNER_BASELINE.md` |
| DUR-02 Persistence-v1 whole-gate architecture: migration authority, transaction correctness, durable audit/outbox substrate, durable-ack/checkpoint/RPO separation, PITR/restore safety and compatible schema evolution with all historical subjects reconciled | `ACCEPTED` | `DUR-02_PERSISTENCE_V1_OWNER_BASELINE.md` |
| GAME-ITEM-01 native item model, equipment/container legality and definition compatibility/migration semantics | `ACCEPTED` | `GAME-ITEM-01_ITEM_MODEL_AND_EQUIPMENT_CONTRACT.md`, delivery PR #205 final head `53d0189a114c99b4e7d44ca8d0db7a6bf5f3ea1a`, merge `5c502d24557621efc798def87b68f137ba23fad8` |
| DUR-03 item/currency/value transaction, conservation, idempotency, runtime↔durable handoff and anti-duplication architecture | `ACCEPTED` | `DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md`, delivery PR #207 final head `a1d949362e219373a5d314c0e9ddf8de110362dd`, merge `63380bcba469027e90677aaf4db571fa941be2f4` |

## Progressive execution policy

- `FND-01`, `VSL-02`, the destination migration and the source-only historical/non-canonical closeout are complete.
- `FND-ID-01`, `FND-02`, `FND-03`, `FND-04`, `DUR-01`, `DUR-02`, `DUR-03`, `ANL-01`, `GAME-ITEM-01` and the dual-transport architecture strategy are accepted/lifecycle-closed where recorded by `FOUNDATION_PROGRAMME_CURRENT_STATUS.md`; implementation status remains separate and is largely `NOT_STARTED`.
- The migrated client remains in ADR-0011 `pre-native-protocol`: it launches and fails closed before gameplay credential consumption, routing or gameplay transport.
- `protocol-canary` remains absent from the destination production graph and cannot re-enter as a production adapter/fallback without an explicit superseding owner decision.
- ADR-0009 fixes the GameNode/process/container boundary and recovery invariants; `PERF-01` gates supported capacity claims and `OPS-CHANNEL-01` gates automatic production channel scaling and claimed production recovery behavior.
- `GAME-VISION-01` minimum product-vision semantics are accepted in `GAME-VISION-01_MINIMUM_OWNER_BASELINE.md`; the immutable first Reference target remains Global Tibia production-observable behavior after the 2026-07-28 server-save/maintenance boundary.
- Official public sources are primary Reference evidence but not assumed exhaustive; controlled black-box observation may provide target evidence; community documentation remains corroborative/discovery input; Canary/crystalserver/other OTS are hypotheses/inventory only and never proof of Global behavior or production authority.
- `GAME-CHAR-01_STAGE_A_OWNER_BASELINE.md` and `GAME-CHAR-01_STAGE_B_OWNER_BASELINE.md` remain binding; GAME-CHAR acceptance does not mean complete July-28 behavior knowledge or `PARITY_CONFIRMED`.
- `DUR-02_PROFILE_NEUTRAL_CHARACTER_PERSISTENCE_OWNER_BASELINE.md` remains binding for the profile-neutral Character persistence sub-scope and is consumed by the accepted whole gate.
- `DUR-02_PERSISTENCE_V1_OWNER_BASELINE.md` accepts the six common Persistence-v1 rules and the exhaustive fourteen-subject disposition. It makes no SQL DDL, migration execution, Rust runtime or production claim.
- Whole-DUR-02 acceptance removes the historical accidental dependency on GAME-ITEM/DUR-03 for the **common persistence substrate**. A separately authorized server/persistence foundation implementation may consume accepted FND-02/FND-03/FND-04/DUR-01/DUR-02/ANL-01/GAME-CHAR architecture without waiting for item semantics.
- `GAME-ITEM-01` and `DUR-03` now close the paper-only item/value semantic and conservation prerequisites. Durable item/currency/value runtime mutation remains `NOT_STARTED` and requires a separate explicit implementation task plus the concrete ANL/resource-limit/transaction evidence named by DUR-03.
- Historical market/economy consistency belongs to `EXP-ECONOMY-01`, guild/social to `EXP-SOCIAL-01`, houses to `EXP-HOUSES-01`, recurring/meta rewards to `GAME-META-01`, encounter/event rewards to `EXP-EVENTS-01`; accepted DUR-03 supplies conservation/custody primitives wherever those domains later move value without absorbing their business policy.
- Partitioning/sharding and exact Rust DB/migration library selection remain implementation/PERF decisions unless evidence later proves an architecture constraint.
- `GAME-CHANNEL-01` is the earliest remaining owner-accepted recommended-order paper-only product gate. The Reference evidence/parity manifest, `DUR-04` and `SIM-DETERMINISM-01` may proceed in parallel only under separate ownership.
- ADR-0012 fixes native Character Authority versus Platform lifecycle/orchestration ownership without authorizing runtime or persistence implementation.
- ADR-0013 removes Platform database migration from the native gameplay critical path while preserving PostgreSQL for native game persistence and all ADR-0004 ownership/least-privilege invariants.
- ADR-0014 accepts TCP-default/future-QUIC dual transport only as architecture direction; ADR-0016 keeps every gameplay transport runtime mode unavailable until implemented and proven, and ADR-0015 preserves ADR-0009's one-process GameNode identity while leaving only internal decomposition and genuinely distinct adjacent-service placement evidence-driven.
- bounded technical spikes may inform contracts only when reversible, isolated, non-production and explicitly non-canonical.
- accepted `ANL-01` event/audit semantics constrain DUR-02/DUR-03 physical outbox/audit mechanics; analytics consumers never replace authoritative invariants.
- `QA-E2E-01` is accepted; its three-tier implementation and named evidence block completion of `VSL-01`.
- architecture acceptance is not implementation authorization; a spike, placeholder crate or passing compile does not prove runtime capability.

## Stage A — foundation and layer gates

### `GAME-VISION-01` — Product Vision, Parity Scope and World Profile Contract

- Status: `ACCEPTED` for the minimum product-vision gate scope and first Reference target; implementation `NOT_STARTED`.
- Canonical minimum owner source: `GAME-VISION-01_MINIMUM_OWNER_BASELINE.md`, incorporating the seven earlier dedicated owner baselines.
- Canonical first Reference target source: `GAME-VISION-01_FIRST_REFERENCE_BASELINE_OWNER_BASELINE.md`.
- Accepted first external evaluation is Reference-first; released Reference revisions are immutable while upstream evidence may be observed continuously and explicitly promoted into later revisions.
- Accepted player/product direction preserves recognizable Tibia depth and persistent-world identity, modern native quality, explicit/versioned/measurable intentional differences, `PvP = secondary pillar`, `solo viable, party rewarded`, and reliability/UX-first initial Evolved differentiation.
- Accepted core loop is player-chosen goal -> preparation -> risk/activity -> secure committed progress/value -> recovery/planning across persistent character/equipment/exploration/social/prestige horizons.
- Accepted economy direction is Reference mechanical source/sink parity rather than historical market-price/supply parity, with conservation before tuning, measurable provenance, semantic scarcity and no hidden macro tuning.
- Accepted success categories are Reference correctness, player interaction quality, progress/value trust, core-loop health, economy health and product/operational health; numeric targets remain milestone-owned.
- First Reference target is the Global Tibia production-observable behavior cut after the **2026-07-28 server-save/maintenance change boundary**.
- Target date does not imply exhaustive knowledge: `UNKNOWN` and `CONFLICT` remain valid evidence states and block the affected parity claim rather than being filled from implementation convenience.
- Evidence hierarchy/provenance and security/integrity/legal overrides in the first-Reference owner baseline are binding across downstream Reference-sensitive work.
- Preserve one engine, client and `protocol-oteryn`; differences remain versioned ruleset/content/product profiles and distinct logical worlds.
- Preserve default world-scoped character/economy isolation until a dedicated transfer contract proves safety.
- Exact Reference revision naming syntax, exact downstream gameplay/economy formulas, numeric KPI thresholds, branding, monetization, exact first Evolved feature inventory and LiveOps cadence remain downstream/deferred under their named gates.
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

- Status: **`ACCEPTED` architecture; implementation `NOT_STARTED`**.
- Canonical whole-gate source: `DUR-02_PERSISTENCE_V1_OWNER_BASELINE.md`.
- Canonical Character sub-baseline: `DUR-02_PROFILE_NEUTRAL_CHARACTER_PERSISTENCE_OWNER_BASELINE.md`.
- Whole-gate acceptance binds six common persistence rules: ordered game-owned migration history with immutable artifacts and dedicated migrator; anomaly-proof transaction isolation/locking/retry; ANL-compatible durable journal plus crash-safe publication checkpoint; strict durable-ack versus FND-03 checkpoint versus disaster-RPO separation; PITR-capable restore-tested fail-closed recovery with non-rollback fencing; and expand/migrate/validate/cut-over/contract schema evolution.
- The fourteen historical subjects are exhaustively reconciled. Character/session questions are satisfied by accepted Character/FND-04 architecture; item/currency/value conservation is moved to GAME-ITEM-01/DUR-03; market/social/house/reward semantics are moved to their exact domain owners; partitioning and exact Rust DB/migration library remain measured implementation choices unless evidence creates an architecture constraint.
- No PostgreSQL DDL/migrations, Rust persistence/runtime implementation, production backup policy or authoritative gameplay write is authorized merely by architecture acceptance.
- A separately authorized server/persistence foundation implementation may consume accepted common DUR-02 plus Character/FND-04 semantics without waiting for GAME-ITEM/DUR-03.

### `DUR-03` — Item Transaction and Anti-Duplication Invariants

- Status: **`ACCEPTED` architecture; lifecycle closed; implementation `NOT_STARTED`**.
- Canonical source: `DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md`; delivery PR #207 final head `a1d949362e219373a5d314c0e9ddf8de110362dd`, squash merge `63380bcba469027e90677aaf4db571fa941be2f4`.
- Consume DUR-01 ItemInstanceId, accepted GAME-ITEM legality, DUR-02 persistence and ANL/FND authority without redefining them.
- Accepted semantics include one typed immediate semantic location for every live durable ItemInstance; runtime-owned ground versus durable recoverability separation; non-blocking runtime reservation -> async game-DB durable linearization -> runtime reconciliation; transaction-scoped new ItemInstanceId allocation; explicit split/merge/transform lifecycle rules; exact transfer/mint/burn/transform/conversion conservation and source/sink lineage; CommandRef/OperationId/TransactionId retry and ambiguous-commit reconciliation; current GameSession/CharacterLease/runtime-generation fencing; typed custody; bounded durable audit; fail-closed restore reconciliation; and default cross-world value isolation.
- Runtime-only loot versus already-durable ground ItemInstance materialization timing remains downstream combat/content policy but must use one explicit idempotent model.
- Trade/market/bank/depot/mail/reward/house/crafting/entitlement business policy remains downstream-owned.
- Architecture acceptance creates no Rust/client runtime, PostgreSQL DDL/migration, production item transaction or Game Intelligence write authority. Future implementation must prove the exact concurrency/crash/recovery/receipt/audit/resource-limit evidence named in the contract.

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
- Preserve accepted `DUR-03` as the anti-duplication prevention authority; analytics never substitutes for prevention.

### `ANL-04` — Read-Only Investigation and AI

- Status: `EXPANSION`.
- Define least-privilege read-only evidence access, correlation, provenance reconstruction, human review and full auditability.
- Prohibit runtime/database mutation, autonomous bans, balance changes, rollback and deployment.

## Registered gameplay and product decision horizon

Detailed scope, dependencies and non-decisions are canonical in `GAMEPLAY_AND_PRODUCT_ARCHITECTURE_HORIZON.md`.

### Blocks durable gameplay

- `GAME-CHAR-01` — **ACCEPTED architecture**. Stage A + Stage B semantic closure plus the accepted DUR-02 Character persistence sub-baseline provide the profile-neutral Character durability envelope; unresolved per-behavior values/formulas/profile-specific facts remain hard parity gates.
- `GAME-ITEM-01` — **ACCEPTED / LIFECYCLE_CLOSED / implementation NOT_STARTED**. Typed item definition/instance semantics, equipment/container legality and definition compatibility are frozen by `GAME-ITEM-01_ITEM_MODEL_AND_EQUIPMENT_CONTRACT.md`; runtime authority remains separate.
- `DUR-03` — **ACCEPTED / LIFECYCLE_CLOSED / implementation NOT_STARTED**. Item/currency/value conservation, atomic location, idempotency, runtime↔durable handoff and anti-duplication semantics are frozen; production/runtime proof remains separately unauthorized and unproven.

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
12. FND-02/FND-03/FND-04/DUR-01/DUR-02/DUR-03/ANL-01 architecture acceptance does not imply runtime implementation; consume their canonical contracts and current status overlay rather than older progress prose.
13. Every gameplay/product package must reconcile `GAMEPLAY_AND_PRODUCT_ARCHITECTURE_HORIZON.md`; an unnamed domain may not be silently absorbed into an unrelated gate.
14. Every vertical-slice or client-visible package must consume `QA-E2E-01`; it may add scenarios and assertions but not a competing E2E lifecycle/evidence platform.
15. ADR-0008 is binding on all later client/server packages; no task may reintroduce Canary into production through an optional feature, fallback, compatibility listener or intermediate translation model.
16. ADR-0009 is binding on runtime, performance and operations packages: `NodeId` identifies the GameNode process incarnation rather than a physical host, each channel retains one logical writer, capacity claims require `PERF-01`, and automatic production scaling/recovery claims require `OPS-CHANNEL-01`.
17. Every material architecture decision must apply `docs/agents/ARCHITECTURE_DECISION_DISCIPLINE.md`: state whether it must be decided now, name blocked downstream work, identify future constraints and state evidence that would justify supersession.
18. ADR-0013 is binding on persistence/programme planning: PostgreSQL remains the native game target, but Platform database migration is not a native-game prerequisite and requires separate Platform authority and evidence.
19. ADR-0014 through ADR-0016 are binding within their named transport/GameNode-readiness scopes: one `protocol-oteryn` remains authoritative, ADR-0009's one-process GameNode identity remains intact, transport profile registration does not imply runtime readiness, and QUIC cannot activate without its later profile/reconciliation/evidence/implementation gates.
20. `GAME-VISION-01_MINIMUM_OWNER_BASELINE.md` is binding for the minimum product-vision gate.
21. `GAME-CHAR-01_STAGE_A_OWNER_BASELINE.md` remains binding for baseline-neutral Character safety/ownership/lifecycle semantics and is consumed by the accepted Stage-B closure rather than superseded.
22. `GAME-VISION-01_FIRST_REFERENCE_BASELINE_OWNER_BASELINE.md` is binding across Reference-sensitive domains: the first target cut is Global Tibia production-observable behavior after the 2026-07-28 server-save/maintenance boundary; later Global changes do not silently mutate it; `UNKNOWN`/`CONFLICT` evidence remains fail-closed; OTS implementations are not proof of Global behavior.
23. `GAME-CHAR-01_STAGE_B_OWNER_BASELINE.md` is binding for overall GAME-CHAR semantic closure. Architecture acceptance does not imply complete Reference parity; unresolved target rules remain per-behavior hard parity gates.
24. `DUR-02_PROFILE_NEUTRAL_CHARACTER_PERSISTENCE_OWNER_BASELINE.md` remains binding for the Character persistence sub-scope and is consumed, not superseded, by whole DUR-02.
25. `DUR-02_PERSISTENCE_V1_OWNER_BASELINE.md` is binding for whole Persistence-v1. Its accepted fourteen-subject disposition prevents generic persistence from becoming a second owner of item/economy/social/house/reward semantics, and its acceptance does not grant SQL/runtime authority.
26. `GAME-ITEM-01_ITEM_MODEL_AND_EQUIPMENT_CONTRACT.md` is binding for item definition/instance semantics, typed capability state, equipment/container legality and definition compatibility; it grants no runtime/entitlement authority.
27. `DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md` is binding for durable item/currency/value location, identity transitions, conservation, retry/idempotency, runtime↔durable handoff, custody, audit and restore integrity. It does not accept downstream business policy, select physical SQL/runtime implementation or grant production authority.

## Current next action

`GAME-ITEM-01` and `DUR-03` have closed the paper-only durable item/value semantic and anti-duplication architecture while implementation remains `NOT_STARTED`.

The owner-accepted 2026-08-10 recommended ordering still has one earlier unresolved product gate: **`GAME-CHANNEL-01`**. No open GAME-CHANNEL PR owns it at this closeout. The next ordered architecture action is therefore one bounded paper-only `GAME-CHANNEL-01` task consuming accepted multichannel/FND/GAME-VISION/GAME-CHAR/GAME-ITEM/DUR-03 boundaries without implementing runtime channel orchestration.

Separately, a bounded server/persistence or DUR-03 implementation programme may begin only after explicit owner implementation authorization. A safe first common programme decomposition remains:

```text
server bootstrap / GameNode shell
-> protocol-oteryn transport/runtime adapter
-> admission + GameSession + CharacterLease
-> PostgreSQL migration/persistence substrate
-> Character/FND-04 persistence
-> minimal ChannelRuntime
```

Do **not** execute implementation until the owner explicitly grants implementation authority. The Reference evidence/parity manifest, `DUR-04` and `SIM-DETERMINISM-01` may proceed in parallel under their own boundaries. Profile-specific PvP Character state remains blocked on its owning profile/channel policy.

No gameplay/runtime implementation, Platform write, PostgreSQL migration, persistence schema deployment, entitlement activation or production behavior is authorized by this register update.