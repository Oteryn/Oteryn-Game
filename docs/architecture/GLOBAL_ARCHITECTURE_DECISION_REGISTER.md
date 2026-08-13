# Oteryn v2 Global Architecture Decision Register

- Status: Active coordination register
- Date: 2026-08-13
- Coordination ID: `OTV2-GLOBAL-ARCHITECTURE`
- Canonical foundation programme: `docs/agents/tasks/active/OTV2-20260805-foundation-preimplementation-contracts.md`
- Current execution status: `docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md`
- Coordinator prompt: `docs/agents/prompts/OTV2_GLOBAL_ARCHITECTURE_DECISION_COORDINATOR.md`

## Purpose

Preserve the complete global architecture decision horizon for Oteryn v2 without requiring chat history and without forcing distant expansion features to be designed before they become relevant.

This register is a coordination source, not an implementation claim. Accepted decisions live in ADRs and contracts. The ordered foundation gates live in `FOUNDATION_DECISION_BACKLOG.md`. The detailed gameplay/product horizon lives in `GAMEPLAY_AND_PRODUCT_ARCHITECTURE_HORIZON.md`.

Stable IDs are canonical across tasks, prompts and PRs. Stage labels are descriptive only.

## Status vocabulary

- `ACCEPTED` — frozen by an accepted ADR/contract.
- `BLOCKS_WORKSPACE_BOOTSTRAP` — required before canonical workspace bootstrap/completion.
- `BLOCKS_LAYER_IMPLEMENTATION` — required before canonical implementation of the named layer.
- `BLOCKS_DURABLE_GAMEPLAY` — required before authoritative durable gameplay mutation in the named scope.
- `BLOCKS_VERTICAL_SLICE` — required for the first complete client-to-server gameplay proof.
- `REQUIRED_FOR_ALPHA` — required before playable alpha completeness.
- `EXPANSION` — later product-growth gate.
- `DEFERRED` — deliberately not frozen now.

## Accepted foundation and product architecture

| Domain | Status | Canonical source |
|---|---|---|
| Native Rust client/server and project-owned `protocol-oteryn` | `ACCEPTED` | ADR-0001 |
| Multichannel world/channel/instance ownership baseline | `ACCEPTED` | ADR-0001 and scope matrix |
| Canonical repository, early Rust client cutover and atomic destination migration | `ACCEPTED` | ADR-0002 |
| Platform Identity and initial Go Game Gateway boundary | `ACCEPTED` | ADR-0003 |
| PostgreSQL and separate Platform/game ownership | `ACCEPTED` | ADR-0004 |
| Native world/content model, Oteryn Studio and legacy conversion boundary | `ACCEPTED` | ADR-0005 |
| Game Intelligence, analytics durability classes and read-only investigation boundary | `ACCEPTED` | ADR-0006 |
| Native three-tier E2E platform/evidence/cleanup contract | `ACCEPTED` | ADR-0007 |
| `protocol-canary` reference-only and excluded from production runtime/fallback | `ACCEPTED` | ADR-0008 |
| GameNode process identity, one-writer runtime, measured capacity and same-channel recovery | `ACCEPTED` | ADR-0009 |
| Reference/Evolved world profiles over one engine/client/protocol with world-scoped value | `ACCEPTED` | ADR-0010 + `PRODUCT_DIRECTION_BASELINE.md` |
| Fail-closed native client before accepted gameplay protocol implementation | `ACCEPTED` | ADR-0011 |
| Native Character Authority versus Platform lifecycle boundary | `ACCEPTED` | ADR-0012 + `CHARACTER_AUTHORITY_PLATFORM_BOUNDARY.md` |
| Platform database technology independence from native game PostgreSQL | `ACCEPTED` | ADR-0013 |
| TCP-default/future-QUIC-opt-in strategy over one `protocol-oteryn` | `ACCEPTED` | ADR-0014 + transport policy |
| GameNode remains one process; internal decomposition evidence-driven | `ACCEPTED` | ADR-0015 + ADR-0009 |
| Transport mode vocabulary does not imply runtime readiness | `ACCEPTED` | ADR-0016 + transport policy |
| FND-01/VSL-02 workspace/client cutover | `ACCEPTED` | FND-01/VSL-02 contracts + merge `78988f72a80cc904aa9176ae850c50d4efa0b0f0` |
| Foundation identifier vocabulary | `ACCEPTED` | `FND-ID-01_FOUNDATION_IDENTIFIER_CONTRACT.md` |
| Minimum GAME-VISION product contract | `ACCEPTED` | `GAME-VISION-01_MINIMUM_OWNER_BASELINE.md` |
| First immutable Reference cut: Global Tibia after 2026-07-28 server-save/maintenance | `ACCEPTED` | `GAME-VISION-01_FIRST_REFERENCE_BASELINE_OWNER_BASELINE.md` |
| GAME-CHAR Stage A + Stage B semantic closure | `ACCEPTED` | `GAME-CHAR-01_STAGE_A_OWNER_BASELINE.md` + Stage B baseline |
| DUR-02 profile-neutral Character persistence sub-baseline | `ACCEPTED` | `DUR-02_PROFILE_NEUTRAL_CHARACTER_PERSISTENCE_OWNER_BASELINE.md` |
| DUR-02 whole Persistence-v1 architecture | `ACCEPTED` | `DUR-02_PERSISTENCE_V1_OWNER_BASELINE.md` |
| GAME-ITEM-01 item definition/instance/equipment/container compatibility semantics | `ACCEPTED` | `GAME-ITEM-01_ITEM_MODEL_AND_EQUIPMENT_CONTRACT.md`, PR #205 merge `5c502d24557621efc798def87b68f137ba23fad8` |
| DUR-03 item/currency/value location, conservation, retry and anti-duplication architecture | `ACCEPTED` | `DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md`, PR #207 merge `63380bcba469027e90677aaf4db571fa941be2f4` |
| GAME-CHANNEL-01 channel selection/queue/co-location/anti-hopping/multiplicity/lifecycle product policy | `ACCEPTED` | `GAME-CHANNEL-01_CHANNEL_PRODUCT_POLICY_CONTRACT.md`, PR #209 exact head `ca1112191ede7d316c874189f3053ad7f8247579`, merge `54dddbcfcb64baaf66e6cfd9fcd2cc3e1b4f04f1` |
| DUR-04 deterministic content/package/bundle/migration and authoritative scripting boundary | `ACCEPTED` | `DUR-04_CONTENT_WORLD_AND_SCRIPTING_CONTRACT.md`, PR #212 exact head `77e68ffb9e9e0e31ca751d42ff5f7c03466b2b23`, merge `568236c33cd23da017bca1dbd1ed98afc8da71f4` |
| SIM-DETERMINISM-01 authoritative arithmetic/RNG/order/replay/state-hash boundary | `ACCEPTED` | `SIM-DETERMINISM-01_AUTHORITATIVE_SIMULATION_CONTRACT.md`, PR #214 exact final head `4c6684328123aebd657696808372a5855980d34e`, merge `1e16b32069868f14aa1761a512b6cd8b1024e277` |

## Progressive execution policy

- FND-01, VSL-02 and migration/source closeout are complete.
- FND-ID-01, FND-02, FND-03, FND-04, DUR-01, DUR-02, DUR-03, DUR-04, ANL-01, GAME-VISION-01, GAME-CHANNEL-01, GAME-CHAR-01, GAME-ITEM-01, SIM-DETERMINISM-01 and the dual-transport strategy are accepted/lifecycle-closed where recorded by `FOUNDATION_PROGRAMME_CURRENT_STATUS.md`; implementation remains separate and largely `NOT_STARTED`.
- The migrated client remains ADR-0011 `pre-native-protocol` and fails closed before gameplay credential/routing/transport consumption.
- `protocol-canary` remains absent from the production graph and may not re-enter through compatibility/fallback without explicit supersession.
- ADR-0009 fixes GameNode/process/Channel ownership; PERF-01 gates capacity claims and OPS-CHANNEL-01 gates automatic production scaling/recovery implementation.
- GAME-VISION minimum semantics and immutable 2026-07-28 first Reference target remain binding. Official public evidence is primary but not assumed exhaustive; controlled observation may prove behavior; OTS code is hypothesis/inventory only.
- GAME-CHAR Stage A/B remains binding; acceptance does not imply complete Reference parity.
- DUR-02 whole-gate acceptance grants no SQL DDL/migration execution/runtime authority.
- GAME-ITEM and DUR-03 together close item/value semantics and anti-duplication architecture, but do not implement item/value runtime or physical schema.
- GAME-CHANNEL closes the product-policy gap between technical multichannel capability and player-visible channels: selection/recommendation/queue/co-location, durable anti-hopping/prior-Channel semantics, explicit value-source multiplicity classes, qualitative create/drain/abort/retirement predicates, same-Channel recovery and one-World community/economy rules are accepted. Numeric capacity/headroom/windows/hysteresis remain PERF/OPS-owned and no runtime/Platform implementation is implied.
- DUR-04 closes the semantic content/package/compiler/bundle/activation/migration/scripting architecture gap: exact dependency locks, deterministic immutable artifact flow, bounded loader, explicit durable migration classes, provenance, Component Model + project-owned WIT capability boundary, proposal-only mutations and versioned script-execution determinism are accepted. Physical format, numeric limits, exact Wasmtime/WIT implementation and broad content/runtime activation remain separately gated.
- SIM-DETERMINISM closes the cross-domain deterministic arithmetic/RNG/order/replay/state-hash architecture gap: exact semantic revision binding, explicit numeric classes/rounding/failure semantics, purpose-isolated gameplay RNG, normalized time/external facts, replay provenance and supported-target determinism are accepted. Concrete libraries, exact gameplay formulas/RNG algorithm, global tick, runtime implementation and production replay remain separately gated.
- Historical market/economy consistency remains EXP-ECONOMY-01; guild/social remains EXP-SOCIAL-01; houses remains EXP-HOUSES-01; rewards remain their GAME-META/EXP-EVENTS owners; DUR-03 retains conservation wherever these move value.
- Partitioning/sharding and exact Rust DB/migration library remain measured implementation/PERF choices unless correctness evidence creates an architecture constraint.
- With SIM-DETERMINISM closed, the one selected next paper-only programme action is the versioned Reference evidence/parity manifest under its owning contract; do not invent a new stable gate ID without explicit registration.
- ADR-0014..0016 remain binding: TCP profile registration does not imply runtime readiness, QUIC remains future profile/reconciliation/evidence work, and GameNode remains one process.
- bounded spikes inform contracts only when reversible, isolated, non-production and explicitly non-canonical.

## Stage A — foundation and layer gates

### `GAME-VISION-01` — Product Vision, Parity Scope and World Profile Contract

- Status: `ACCEPTED` for minimum product scope and first Reference target; implementation `NOT_STARTED`.
- Canonical sources: `GAME-VISION-01_MINIMUM_OWNER_BASELINE.md` and first Reference owner baseline.
- Preserve Reference-first external evaluation, immutable released Reference revisions, recognizable Tibia depth/persistent-world identity, modern native quality, explicit/versioned Oteryn differences, `PvP = secondary pillar`, `solo viable, party rewarded`, reliability/UX-first initial Evolved differentiation, player-goal core loop, persistent progression horizons and mechanical source/sink parity with conservation before tuning.
- First Reference target remains production-observable Global Tibia after 2026-07-28 server-save/maintenance; evidence gaps remain `UNKNOWN/CONFLICT` rather than implementation guesses.
- One engine/client/`protocol-oteryn`; profile differences remain versioned policy/content and distinct logical Worlds.
- No runtime/content/persistence implementation authority follows from acceptance.

### `FND-01` — Workspace, Dependency and Existing-Rust Migration Contract

- Status: `ACCEPTED`; archived/applied by destination cutover `78988f72a80cc904aa9176ae850c50d4efa0b0f0`.
- Retained as canonical workspace/dependency/toolchain/migration evidence, not an active blocker.

### `VSL-02` — Exact Rust Client Migration and Cutover Contract

- Status: `ACCEPTED AND COMPLETE`.
- Destination migration and source-only closeout complete; historical contract remains provenance/cutover/rollback evidence.

### `FND-ID-01` — Foundation Identifier Vocabulary

- Status: `ACCEPTED`.
- Canonical source: `FND-ID-01_FOUNDATION_IDENTIFIER_CONTRACT.md`.
- Minimum catalogue includes AccountId, CharacterId, WorldId, ChannelId, NodeId, InstanceId, PartyId, GameSessionId and conditional HandoffId.
- Identity never grants authority. CommandId remains FND-02, runtime handles/generations FND-03, admission/session/lease FND-04, physical representation DUR, event/operation/transaction identities ANL/DUR.

### `FND-02` — `protocol-oteryn` v1

- Status: `ACCEPTED`; lifecycle closed, implementation `NOT_STARTED`.
- Preserve one protocol family, transport/security/framing/schema limits, stable errors, CommandId/order/replay/reconciliation and independent wire evidence. TCP profile 1 is registered architecturally; no listener/adapter implementation is implied.

### `FND-03` — Runtime Execution Contract

- Status: `ACCEPTED`; lifecycle closed, implementation `NOT_STARTED`.
- Preserve NodeRuntime/WorldServices/ChannelRuntime/InstanceRuntime boundaries, one logical writer, ownership generations, RuntimeExecutionOrdinal, bounded queues/backpressure, test clocks, stale-work rejection and recovery. GameNode remains one process under ADR-0009/0015.

### `FND-04` — Identity, Game Session, Admission and Character Lease

- Status: `ACCEPTED`; lifecycle closed, implementation `NOT_STARTED`.
- Preserve ownership-before-world admission, purpose-separated grants, GameSession/lease fencing, PREPARE/COMMIT recovery, healthy-binding non-preemption, ControlLossEpoch, four-second eligible defensive PvE re-entry protection and fail-closed stale authority.

## Stage B — blocks durable gameplay/content

### `DUR-01` — Durable Identifier Representation Contract

- Status: `ACCEPTED`; lifecycle closed, implementation `NOT_STARTED`.
- UUIDv7 native durability uses PostgreSQL `uuid`; persisted uint64 CommandId uses `numeric(20,0)`; ItemInstanceId is game-owned UUIDv7; no physical table is implied.

### `DUR-02` — Persistence v1

- Status: `ACCEPTED`; implementation `NOT_STARTED`.
- Whole gate binds one game-owned migration history, anomaly-proof transaction/locking/retry, ANL-compatible durable journal/publication state, durable-ack versus runtime checkpoint versus disaster-RPO separation, PITR/restore safety and expand/migrate/validate/cut-over/contract schema evolution.
- Character sub-baseline remains binding. Historical subjects are assigned to exact owners; item/value semantics remain GAME-ITEM/DUR-03, market/social/house/reward with their domains.
- No DDL/migration/runtime authority follows from acceptance.

### `DUR-03` — Item Transaction and Anti-Duplication Invariants

- Status: `ACCEPTED`; lifecycle closed, implementation `NOT_STARTED`.
- Canonical source: `DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md`.
- Freeze exactly-one durable item location, runtime↔durable pickup/drop handoff, ItemInstanceId transitions, transfer/mint/burn/transform/conversion conservation and lineage, CommandRef/OperationId/TransactionId idempotency/retry/ambiguous-commit behavior, current lease/runtime fencing, typed custody, bounded audit and fail-closed restore reconciliation.
- Does not absorb loot/trade/market/bank/depot/mail/reward/house/crafting business policy or grant runtime/DDL/production authority.

### `DUR-04` — Content, World Detail and Scripting Contract

- Status: `ACCEPTED`; lifecycle closed, implementation `NOT_STARTED`.
- Canonical source: `DUR-04_CONTENT_WORLD_AND_SCRIPTING_CONTRACT.md`; delivery PR #212 exact final head `77e68ffb9e9e0e31ca751d42ff5f7c03466b2b23`, squash merge `568236c33cd23da017bca1dbd1ed98afc8da71f4`.
- Freeze stable semantic package/content identity and exact dependency locking; deterministic compiler/client-server projection/immutable bundle/staging/activation; bounded fail-closed loading; durable migration classes; legacy provenance; Component Model + project-owned versioned WIT capability boundary; snapshot-bound reads; proposal-only mutations; authority-scoped action plans; deterministic script time/RNG/query/floating/fuel/resource behavior under `script_execution_profile_revision`; typed persistent extension state; and explicit Resource Limits Registry/spike gates.
- Does not freeze the physical serializer/container/chunk/floor/compression choice, exact WIT function inventory, exact Wasmtime version/features or numeric limits, and grants no runtime/client/compiler/loader/Studio/WIT-host/DDL/content-import/production authority.

### `SIM-DETERMINISM-01` — Authoritative Simulation Determinism Contract

- Status: `ACCEPTED`; lifecycle closed, implementation `NOT_STARTED`.
- Canonical source: `SIM-DETERMINISM-01_AUTHORITATIVE_SIMULATION_CONTRACT.md`; delivery PR #214 exact repaired final head `4c6684328123aebd657696808372a5855980d34e`, squash merge `1e16b32069868f14aa1761a512b6cd8b1024e277`.
- Freeze future-determining canonical state + exact owner-local normalized input order + exact semantic revision/profile set as the reproducibility basis; FND-03 RuntimeExecutionOrdinal remains owner-local and no global total order is created.
- Freeze `SimulationDeterminismProfileRevision`, explicit numeric semantic classes/rounding/failure rules, exact revision binding for retryable occurrences, purpose-isolated deterministic gameplay RNG and anti-prediction boundaries, logical time/order/tie-break semantics, normalized external facts, exact server/build/protocol/World Bundle replay provenance, canonical deterministic state hashing/divergence evidence and supported-target determinism.
- Historical independent review `4924203877` found one replay-provenance P1 on superseded head `5dc628f32ca4573725bcb4a42c3a7702536d7f35`; owner-authorized repair cycle 4 closed it. Terminal self-review `4924321455`, repeat self-review `4924423397`, Agent Governance `31676250271`, Dependency Review `31676250273` and CodeQL `31676250272` passed on final head. The owner explicitly overrode the fresh-independent-review-after-repair mechanism for that exact head and instructed finalization; this does not convert self-review into independent review.
- Does not select numeric/RNG/hash libraries, exact gameplay RNG algorithm, exact formulas/scales, global tick, scheduler/thread counts, replay backend/hash cadence or authorize runtime/combat/AI/script/DDL/production implementation.

### `ANL-01` — Game Event and Audit Foundation

- Status: `ACCEPTED`; lifecycle closed, implementation `NOT_STARTED`.
- Preserve minimal envelope + typed event-family payloads, durability/privacy classes, stable identities/correlation, at-least-once EventId-stable publication, finite retention/access policy and atomic durable mutation evidence. No event DB/broker/collector/warehouse is implied.

### `ANL-02` — Gameplay, Balance and World Analytics

- Status: `REQUIRED_FOR_ALPHA`.
- Define gameplay/balance/world/content metrics, dimensions, sample quality and dashboards; observational only.

### `ANL-03` — Economy Integrity and Security Analytics

- Status: `REQUIRED_FOR_ALPHA`.
- Define provenance/invariant consumers, alerts/cases, detector versioning, evidence quality and false-positive handling; DUR-03 remains prevention authority.

### `ANL-04` — Read-Only Investigation and AI

- Status: `EXPANSION`.
- Least-privilege read-only evidence access/correlation/human review; prohibit runtime/database mutation, autonomous bans, balance changes, rollback or deploy.

## Registered gameplay and product horizon

Detailed scope/dependencies/non-decisions remain canonical in `GAMEPLAY_AND_PRODUCT_ARCHITECTURE_HORIZON.md`.

### Accepted durable/product prerequisites

- `GAME-CHAR-01` — `ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED`; unresolved exact profile/Reference facts remain hard gates.
- `GAME-ITEM-01` — `ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED`; runtime item/value mutation remains separate implementation work.
- `GAME-CHANNEL-01` — `ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED`; selection/queue/co-location/switch/multiplicity/lifecycle policy is frozen, while runtime/Platform scaling implementation and numeric PERF/OPS values remain downstream.
- `DUR-04` — `ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED`; content/package/compiler/bundle/migration/scripting architecture is frozen while physical encoding, numeric limits and executable content infrastructure remain downstream.
- `SIM-DETERMINISM-01` — `ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED`; deterministic arithmetic/RNG/order/replay/state-hash architecture is frozen while formula values, libraries and executable SIM/combat/AI implementation remain downstream.

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
- `PROD-ENTITLEMENTS-01` — Entitlements, Premium and Commerce (`DEFERRED`).
- `MOD-ECOSYSTEM-01` — Modding and Plugin Ecosystem (`DEFERRED`).

Registration prevents omission; it does not accept technologies, formulas, schemas, service topology or implementation.

## Stage C — blocks foundation vertical slice

### `QA-E2E-01` — Native End-to-End Test Platform

- Status: contract `ACCEPTED`; implementation evidence `BLOCKS_VERTICAL_SLICE`.
- Use shared manifest-driven Tier 1 headless, Tier 2 instrumented native-client and Tier 3 production-binary smoke evidence. Preserve exact revisions, deterministic controls, first-divergence evidence and cleanup; no hidden retry-until-green.

### `VSL-01` — Foundation Vertical-Slice Programme

- Status: `BLOCKS_VERTICAL_SLICE`.
- Name owners/order/repos/PRs and real observable E2E. Include Platform/Gateway/admission/lease, native map, movement/visibility, combat/death/loot, retry-safe pickup, event/audit, persistence/checkpoint, logout/recovery, duplicate-session rejection and multichannel isolation.
- Consume QA-E2E-01 rather than inventing a feature-owned runner.

### `VSL-MOVE-01` — Movement, Collision and Visibility Contract

- Status: `BLOCKS_VERTICAL_SLICE`.
- Freeze movement/collision/floor/teleport and interest/snapshot/delta/reconciliation semantics; server-authoritative legality.

### `VSL-COMBAT-01` — Minimal Combat, Death and Loot Contract

- Status: `BLOCKS_VERTICAL_SLICE`.
- Freeze minimal action/target/damage/death/corpse/loot/XP attribution and retry-safe pickup integration, preserving later conditions/PvP/boss/ruleset extension points.

### `VSL-CONTENT-01` — Minimal Native Map, Compiler and Loader Contract

- Status: `BLOCKS_VERTICAL_SLICE`.
- Define the minimum World Project/World Bundle/compiler/loader/content keys for the vertical slice; consume accepted DUR-04 semantic architecture and the required physical-format/resource-limit evidence; do not implement full Studio/import breadth by default.

## Stage D — required for playable alpha

### `ALPHA-CLIENT-01` — Client Architecture
- Status: `REQUIRED_FOR_ALPHA`; renderer/UI/input/networking/client state/reconciliation/assets/audio/settings/accessibility/crash-reporting boundaries.

### `ALPHA-RULESET-01` — Ruleset Architecture
- Status: `REQUIRED_FOR_ALPHA`; data/policy versus typed domain modules for classic/modern differences without protocol forks.

### `ALPHA-CONTENT-01` — NPC, Quests, Shops and Content Runtime
- Status: `REQUIRED_FOR_ALPHA`; content execution/state/deterministic tests/error isolation/authoring workflow.

### `ALPHA-QUALITY-01` — Testing and Performance Baselines
- Status: `REQUIRED_FOR_ALPHA`; consume QA-E2E-01 and PERF-01; unit/property/fuzz/integration/protocol/determinism/database/recovery/multichannel/E2E/soak/migration evidence.

### `PERF-01` — Capacity, Performance and Scalability

- Status: `REQUIRED_FOR_ALPHA`.
- Define reference hardware/deployment cells, latency/scheduling/queue/resource objectives, representative workloads, separate Channel/GameNode/World capacity limits and safe headroom; exact artifact/content/ruleset/protocol/topology evidence required for claims.

### `OPS-CHANNEL-01` — GameNode Deployment and Dynamic Channel Orchestration

- Status: `BLOCKS_LAYER_IMPLEMENTATION`.
- Blocks automatic production Channel scaling and production recovery claims.
- Implement process/container packaging, orchestrator authority, registration/health/readiness/capacity reporting, placement, activation/deactivation, hysteresis, draining/closure, ownership fencing, replacement/checkpoint/replay/recovery and numeric operational policy.
- Must consume accepted GAME-CHANNEL qualitative product lifecycle predicates; OPS does not redefine create/drain/retirement legitimacy.
- Preserve no active-channel live migration and no silent failover to a different Channel.

### `ALPHA-MILESTONE-01` — Product Milestone Contract
- Status: `REQUIRED_FOR_ALPHA`; define Foundation/Alpha/Beta/Release outcomes/exclusions and prevent broad implementation before complete vertical slice.

## Stage E — expansion systems

### `EXP-EVENTS-01` — Dynamic Events, Raids and Bosses
- Status: `EXPANSION`; scheduling, scope/uniqueness, cooldown/anti-hopping, participation/scaling/persistence/recovery/rewards using ADR-0005 encounter geometry.

### `EXP-HOUSES-01` — Houses
- Status: `DEFERRED`; preserve one authoritative house state/world and anti-duplication; later topology/access/rent/doors/beds/guest/recovery ADR.

### `EXP-SOCIAL-01` — Party, Guild, Chat, Friends and Presence
- Status: `EXPANSION`; world/channel/cross-world scope, consistency, shared experience/buffs/rosters/messaging/presence.

### `EXP-ECONOMY-01` — Market, Trade and Economy
- Status: `EXPANSION`; offers/escrow/fees/fills/cancellation/delivery/concurrency/audit/fraud resistance.

### `EXP-SECURITY-01` — Security, Abuse and Administration
- Status: `EXPANSION` with earlier foundation requirements; rate limits/replay/enforcement/admin commands/least privilege/sanctions/secrets/supply-chain, consuming ANL-03 without autonomous anomaly enforcement.

### `EXP-UPDATE-01` — Launcher, Updater and Release Distribution
- Status: `DEFERRED`; signed manifests/deltas/rollback/channels/compatibility/CDN/archive safety.

### `EXP-OPS-01` — Deployment and Operations
- Status: `EXPANSION`; broader environment/config/secrets/rollout/migration/backup/DR consuming OPS-CHANNEL for GameNode/channel control-plane semantics.

### `EXP-OBS-01` — Observability
- Status: `EXPANSION`; structured logs/low-cardinality metrics/tracing/correlation/channel health/DB/protocol/privacy/alerts, distinct from gameplay analytics/durable audit.

### `EXP-SCALE-01` — Advanced Scaling and Prediction
- Status: `DEFERRED`; live channel migration, partitioning one channel, hundreds of channels, advanced prediction and extracted services; QUIC evolution remains ADR-0014/NET-TRANSPORT successor work.

## Programme ownership discipline

The foundation task is non-owning. Every substantial gate has one task/branch/PR/owner/path set and terminal archive. The programme checkpoint records accepted state and exactly one next action; it does not reserve future paths.

## Decision discipline

1. Resolve gates in dependency order using stable IDs.
2. This register never grants implementation authority beyond accepted task scope.
3. Update status only with accepted ADR/contract evidence.
4. Every accepted package updates this register narrowly and links canonical source.
5. Preserve deferred topics/extension points without inventing final designs.
6. Workspace members require immediate consumers; no speculative placeholder crates.
7. Enforce accepted dependency boundaries with executable CI where implementation exists.
8. Cross-repo locks use merged immutable revisions, never mutable PR heads.
9. Public contracts register applicable resource limits/errors/failure scenarios.
10. A decision is complete only after validated/audited unchanged merge + task archive/lifecycle closeout.
11. Completed gates are consumed unless explicitly superseded.
12. FND-02/FND-03/FND-04/DUR-01/DUR-02/DUR-03/DUR-04/ANL-01/GAME-CHANNEL/SIM-DETERMINISM-01 architecture acceptance does not imply runtime implementation.
13. Every gameplay/product package reconciles the gameplay/product horizon; unnamed domains cannot be absorbed silently.
14. Every vertical-slice/client-visible package consumes QA-E2E-01.
15. ADR-0008 forbids production Canary compatibility/fallback/translation without supersession.
16. ADR-0009 binds runtime/PERF/OPS: one GameNode process, one writer/Channel, PERF capacity evidence, OPS production scaling/recovery.
17. Every material decision applies `ARCHITECTURE_DECISION_DISCIPLINE.md`.
18. ADR-0013 keeps Platform DB migration out of the native-game prerequisite path.
19. ADR-0014..0016 preserve one protocol, one-process GameNode, no runtime-readiness inference and later QUIC profile/reconciliation/evidence gates.
20. GAME-VISION minimum and first Reference baselines remain binding.
21. GAME-CHAR Stage A/B baselines remain binding; acceptance is not full parity.
22. DUR-02 Character sub-baseline is consumed, not superseded, by whole DUR-02.
23. Whole DUR-02 prevents generic persistence from stealing item/economy/social/house/reward semantics and grants no SQL/runtime authority.
24. GAME-ITEM is binding for item semantics and grants no DUR-03/entitlement/runtime authority.
25. DUR-03 is binding for durable item/value integrity and grants no downstream business/runtime authority.
26. GAME-CHANNEL is binding for Channel product/lifecycle/multiplicity semantics; numeric capacity/windows/hysteresis remain PERF/OPS, and acceptance grants no runtime/client/Platform/scaling authority.
27. DUR-04 is binding for semantic content/package/compiler/bundle/migration/scripting architecture; physical encoding, numeric limits and executable content infrastructure remain separately gated.
28. SIM-DETERMINISM-01 is binding for deterministic numeric/RNG/order/replay/state-hash semantics; acceptance grants no formula/library/runtime/combat/AI/script/production authority and does not itself prove Reference correctness.

## Current next action

GAME-CHANNEL, GAME-ITEM, DUR-02, DUR-03, DUR-04 and SIM-DETERMINISM-01 are architecture-complete/lifecycle-closed while implementation remains `NOT_STARTED`.

The selected next paper-only programme action is to **build the versioned Reference evidence/parity manifest under its owning contract**. Preserve the accepted 2026-07-28 first Reference target and evidence hierarchy, record provenance/status per exercised mechanic, keep `UNKNOWN/CONFLICT` fail-closed, and do not invent a new stable gate ID unless explicitly registered.

Executable server/persistence/channel/item/content/SIM implementation still requires explicit owner implementation authorization. No gameplay/runtime implementation, Platform write, PostgreSQL migration/schema deployment, entitlement activation, broad content import or production behavior is authorized by this register update.