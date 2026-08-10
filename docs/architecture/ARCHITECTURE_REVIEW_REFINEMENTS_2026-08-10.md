# Oteryn v2 Architecture Review Refinements — 2026-08-10

- Status: Owner-accepted programme refinement
- Date: 2026-08-10
- Scope: architecture, delivery ordering and product/operations guardrails
- Does not authorize: runtime implementation, production changes or external-repository writes

## Review perspectives

The architecture was reviewed jointly from the perspective of:

- software architecture;
- systems architecture;
- senior Rust/backend development;
- game-engine development;
- networking;
- security;
- DevOps/SRE;
- game production;
- game design;
- MMO server administration;
- developer tooling/content production;
- production operations;
- end-player experience.

## Overall decision

The technical foundation is appropriate and should be preserved. The main project risk is no longer a wrong foundational technology choice; it is the gap between mature contracts and the absence of a complete native vertical runtime, plus scope pressure from trying to build client, server, protocol, multichannel, world tooling, Studio, analytics, admin tooling and multiple product profiles at once.

The programme therefore keeps the accepted authority/data/security model but shifts toward smaller evidence-producing slices, earlier product/channel decisions and earlier operational/admin readiness.

## Preserve without redesign

Unless explicitly superseded by a dedicated later decision, preserve:

- native Rust client and authoritative Rust game server;
- one project-owned application protocol: `protocol-oteryn`;
- Oteryn Platform ownership of reusable identity/OAuth/MFA/Game Gateway/World Registry boundaries;
- game-domain ownership of final admission, CharacterLease, `GameSessionId` and authoritative gameplay;
- explicit `WorldId`/`ChannelId`/`InstanceId`/`NodeId`/`GameSessionId` semantics;
- one logical authoritative mutation owner per channel/instance;
- explicit world-shared versus channel-local state ownership;
- generation fencing and one active authoritative character session;
- PostgreSQL as the authoritative game durability target with separate Platform/game ownership;
- idempotent commands, transactional outbox and durable economy/security evidence;
- native World Project -> deterministic World Bundle direction and legacy import boundaries;
- read-only Game Intelligence/AI investigation authority;
- layered real-boundary E2E testing.

## Server implementation shape

The first GameNode should be a **domain-modular monolith**, not a mesh of microservices.

Create a separate crate/service only when a real boundary requires it: process/deployment isolation, security boundary, independent data ownership/lifecycle, independently scaled failure domain, or a genuinely shared contract with multiple consumers.

Do not create empty layering crates because a box exists on an architecture diagram.

Gameplay/domain code must remain independent from Tokio sockets, Protobuf wire layout, PostgreSQL adapters and client UI/renderer state even if those adapters initially live in one GameNode process.

## World-shared authority

One-writer ChannelRuntime semantics do not by themselves solve mutable world-shared state. Market, houses, guilds, friends/presence, reward eligibility, global events/world bosses and cross-channel transfers each require:

- a named authority owner;
- typed idempotent commands;
- revision/generation/fence semantics;
- timeout/retry/duplicate behavior;
- recovery semantics;
- durable audit where value/ownership changes.

ChannelRuntime must not gain implicit shared write authority through process-global mutable caches.

## Product scope and GAME-VISION-01

`GAME-VISION-01` must be completed before broad gameplay/content production. At minimum it must define:

- the launch profile and player promise;
- core session loop and long-term loop;
- progression/risk/death baseline;
- party/cooperation baseline;
- PvP baseline if applicable;
- economic sources/sinks and scarcity goals;
- first intentional improvements beyond reference Tibia behavior;
- measurable success criteria.

The first externally evaluated build should normally use one profile, one small representative area, bounded professions/abilities/items/economy and one channel before a second channel is introduced specifically to prove multichannel semantics. Exact numbers remain game-design decisions, not architecture guesses.

## GAME-CHANNEL-01

Before multichannel becomes a product feature rather than a technical capability, freeze a dedicated channel-policy gate covering:

- channel creation/removal and capacity triggers;
- player choice versus automatic assignment;
- party/friend co-location;
- queues and channel visibility;
- channel-switch cooldowns and anti-hopping;
- spawn/loot/resource multiplication;
- world-global bosses/events and reward eligibility;
- PvP implications;
- social/community fragmentation safeguards;
- recovery requirement to avoid silently moving a player into a different live combat/spawn/loot state.

Multichannel is simultaneously a systems, economy, social and UX decision.

## Character and item ordering

- `GAME-CHAR-01` must be accepted before `DUR-02` freezes final durable character semantics/schema. It must define lifecycle, progression, death, rename/respec/transfer and ruleset-migration semantics relevant to persistence.
- `GAME-ITEM-01` must be accepted before `DUR-03` freezes item transaction semantics. It must define ItemType versus ItemInstance, stack/container/equipment semantics, binding, decay/transform, split/merge and ownership invariants.

Bounded `DUR-02` discovery may proceed using already accepted DUR/ANL prerequisites, but final character-bearing schema authority waits for `GAME-CHAR-01`.

## SIM-DETERMINISM-01

Before broad combat/AI implementation freeze deterministic simulation arithmetic:

- integer/fixed-point versus floating-point authority;
- rounding order and rules;
- overflow/saturation behavior;
- deterministic RNG ownership/seeding;
- modifier application order;
- replay inputs;
- state-hash/checkpoint comparison expectations.

Deterministic RNG alone is insufficient if authoritative arithmetic can diverge across builds/platforms.

## Vertical-slice programme

`VSL-01` remains the umbrella proof but should be delivered as small ordered real-boundary slices:

1. `VSL-ADMISSION-01` — auth/ticket/Gateway/admission/static room/logout.
2. `VSL-MOVE-01` — movement, visibility, sequencing and two real clients/headless peers.
3. `VSL-COMBAT-01` — one deterministic combat/death path with replay evidence.
4. `VSL-PERSISTENCE-01` — one loot/pickup/item persistence transaction plus durable outbox/audit and retry/idempotency evidence.
5. `VSL-RECOVERY-01` — disconnect/reconnect/stale-binding rejection/process kill and same-channel recovery.
6. `VSL-MULTICHANNEL-01` — second channel, channel switch and world-shared/channel-local isolation evidence.

A mock that bypasses a claimed Gateway/transport/GameNode/PostgreSQL boundary is not terminal proof for that boundary.

## Networking decision

`ADR-0014 / NET-TRANSPORT-01` defines the accepted transport direction:

```text
TCP + TLS 1.3: initial default + mandatory safe fallback
QUIC v1 + TLS 1.3: player-opt-in preferred transport
QUIC_ONLY: developer/diagnostic only
```

Both adapters carry the same `protocol-oteryn`; 0-RTT and baseline QUIC DATAGRAM remain disabled. QUIC implementation/default promotion requires measured evidence and registered numeric resource ceilings.

## Admin/GM control plane

A minimal typed admin plane is required before external alpha. It should support at least safe lookup/session/channel operations, drain/maintenance, item/economy provenance investigation, moderation/support workflows and controlled compensation.

Ad-hoc raw SQL is not an accepted repair mechanism. Mutating operator actions require:

- typed domain command;
- RBAC/least privilege;
- idempotency key;
- reason/case/incident link;
- actor audit;
- result evidence;
- dual control for high-risk identity/economy operations where appropriate.

AI/analytics may surface evidence but must not autonomously ban, delete value, roll back the economy or deploy changes.

## Security baseline before external alpha

Require a maintained threat model covering client, Gateway, GameNode, admin plane and update pipeline plus:

- DDoS/connection exhaustion and rate limiting;
- secret/key rotation;
- signed artifacts/updater and build provenance/SBOM strategy;
- GM/admin access audit;
- account/character abuse controls;
- privacy/retention/export policy;
- secure scripting capability boundaries;
- incident and economy-duplication response procedures.

Game/economy authority fails closed when required security authority is stale or ambiguous.

## SRE/production baseline

Before external alpha define and prove appropriate:

- SLOs for login/admission/latency/durability;
- RTO/RPO;
- PostgreSQL backup/PITR and restore drills;
- safe expand/contract migrations;
- rollout/version-skew rules and rollback strategy;
- runbooks for GameNode loss, Platform outage, DB degradation, outbox backlog and economy incident;
- capacity/headroom evidence;
- controlled game-day failure exercises.

Kubernetes is not a first-release requirement. A small statically managed GameNode pool is acceptable if health, readiness, drain, fencing, reproducible deployment and secret management are correct.

## Tooling/content direction

Do not let the full Oteryn Studio block the first server proof. Build shared headless semantics first:

1. source schema;
2. validator;
3. deterministic compiler;
4. World Bundle format;
5. runtime loader;
6. CLI/CI integration;
7. legacy importer;
8. Studio GUI on the same core.

Developer tooling should grow toward semantic diff, bundle inspector, replay viewer, protocol inspector, item-provenance explorer, scenario runner, headless bot client and load harness.

Content scripting requires a capability sandbox: no direct DB authority, no arbitrary filesystem/network access, bounded CPU/memory/time, deterministic runtime-provided RNG, versioned APIs and controlled activation/hot-reload boundaries.

## Player-facing correctness

The client must make operational state understandable without exposing secrets: world/channel, queue, reconnect, maintenance, version mismatch, session conflict and transport diagnostics.

Confirmed rollback/outage/server-caused death/entitlement/economy incidents require an explicit compensation policy using audited idempotent domain transactions rather than manual database edits.

FND-04's four-second eligible defensive PvE reconnect protection remains the accepted baseline, but it must be playtested for disconnect abuse, bosses, party interaction, potions and PvP. Any change requires an explicit superseding decision rather than hidden per-server drift.

## Governance refinement

Use `ARCHITECTURE_STATUS_MODEL.md` to separate:

- architecture decision status;
- task/delivery lifecycle status;
- actual implementation/proof/production status.

Accepted/closed architecture must never be reported as implemented runtime without separate evidence.

## Recommended ordering

```text
architecture/governance normalization
+ GAME-VISION-01 minimum
+ GAME-CHANNEL-01

GAME-CHAR-01
-> final character-bearing DUR-02

GAME-ITEM-01
+ accepted DUR-02
-> DUR-03

minimal DUR-04 headless toolchain
+ SIM-DETERMINISM-01
+ transport-neutral runtime boundaries

-> VSL-ADMISSION-01
-> VSL-MOVE-01
-> VSL-COMBAT-01
-> VSL-PERSISTENCE-01
-> VSL-RECOVERY-01
-> VSL-MULTICHANNEL-01

admin/security/SRE baseline
-> external alpha
```

## Deliberately deferred

Do not freeze now without evidence:

- final QUIC Rust library and exact QUIC limits/fallback timing;
- QUIC as default transport;
- live migration between GameNodes;
- Kubernetes/microservice decomposition;
- full Studio UX architecture;
- simultaneous broad Reference and Evolved launches;
- autonomous AI enforcement;
- detailed endgame/balance formulas outside their owning game-design gates.

## Concise programme rule

Preserve the accepted authority/data/security foundation, decide product/channel/determinism before the broad implementations they shape, build a modular-monolith GameNode and headless content toolchain first, prove the system through small real end-to-end slices, bring admin/security/reliability capability before external alpha, and treat player-visible recovery/economy correctness as first-class architecture.
