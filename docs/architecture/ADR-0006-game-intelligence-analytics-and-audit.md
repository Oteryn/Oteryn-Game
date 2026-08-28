# ADR-0006: Oteryn Game Intelligence, analytics and authoritative audit architecture

- Status: Accepted
- Date: 2026-08-05
- Decision owners: Oteryn project owner and Oteryn v2 architecture programme
- Coordination ID: `OTV2-GAME-INTELLIGENCE`
- Related: ADR-0001 through ADR-0005, `FOUNDATION_DECISION_BACKLOG.md`, `GLOBAL_ARCHITECTURE_DECISION_REGISTER.md`, `RESOURCE_LIMITS_REGISTRY.json`, `FOUNDATION_ERROR_VOCABULARY.md`, `FOUNDATION_FAILURE_SCENARIOS.md`

## Context

Oteryn v2 needs first-class facilities for understanding gameplay, players, items, the economy and the world. The required outcomes include balance analysis, regression detection, world/content usage analysis, bot and exploit investigation, item-duplication detection and evidence-backed diagnosis of engine defects.

This is not a greenfield idea without prior evidence. The project-owned Canary line previously implemented an optional Gameplay Analytics subsystem. Canary PR #30, merged as `684ff1b520a5d296bc4018e32fb9e1c068cea0b6`, introduced buffered hunt sessions, MariaDB aggregates, combat/healing/experience/death metrics, vocation/level/party context and per-monster, spell, damage-type, supply and loot dimensions. Later work added idempotent writes, bounded retry and dead letters, batching, schema migrations, retention, daily aggregates, Grafana reporting, hunt-area tooling and layered Lua/Python/MariaDB validation.

Canary PR #330, merged as `d061dbe72265c89df9ab683717b18b598a106964`, recorded the Oteryn handoff and correctly separated three concepts: a common event/telemetry foundation, Gameplay Analytics, and future Security Analytics with a read-only AI investigation layer. It also recorded that the Canary implementation did not prove production callback ordering, full gameplay-event coverage, sustained-load behavior or item-duplication detection.

Otheryn PR #109, merged as `a6e2993ed32b1316168045ad0b97ddebb50a2128`, classified the inherited Gameplay Analytics package as `EXPERIMENTAL_ONLY`. That disposition was correct for Otheryn because no canonical consumer existed and privacy, retention, deletion and production-readiness boundaries were unresolved. It does not reject analytics as a product capability; it rejects copying the Lua/MariaDB subsystem as the target Oteryn architecture.

Otheryn also contains reusable observability evidence. For example, PR #188, merged as `ebef902691882f9a3678f29a5273d05bc6369bed`, exported bounded checkpoint metrics without player names, GUIDs or generations as metric labels. This supports a strict distinction between low-cardinality operational observability and player/item-linked investigation data.

The central architecture problem is that gameplay analytics and anti-duplication/security auditing have different durability requirements. Combat and world-usage telemetry may be sampled, aggregated and fail open. Item, currency, reward and transaction evidence cannot be an uncorrelated best-effort metrics stream because missing or duplicated records would undermine the exact investigation it is intended to support.

## Decision

### 1. Establish Oteryn Game Intelligence as a first-class subsystem

Oteryn adopts **Oteryn Game Intelligence** as the umbrella architecture for game analytics, world/content analytics, economy integrity, security analytics and read-only investigation.

The target shape is:

```text
Oteryn authoritative runtime
    |
    +-- Operational observability
    |     `-- OpenTelemetry / Prometheus / structured logs
    |
    +-- Versioned game-event foundation
    |     +-- best-effort gameplay telemetry
    |     +-- durable transactional audit/outbox
    |     `-- deterministic replay fixtures
    |
    +-- Gameplay & Balance Analytics
    +-- World & Content Analytics
    +-- Economy & Item Integrity
    +-- Security Analytics
    `-- Read-only Investigation / AI
```

The subsystem is observational and investigative. It does not become a second authority over gameplay state.

### 2. Separate three data classes

#### Operational observability

Operational observability covers process and service health, including tick latency, queue depth, channel health, checkpoint age, database latency, protocol failures and resource saturation.

Rules:

- low-cardinality labels only;
- no character names, player GUIDs, item IDs, transaction IDs or session generations as Prometheus labels;
- bounded logs with redaction and correlation IDs;
- operational metrics may reset on process restart unless a separate durable SLO store requires otherwise;
- observability failure must not change authoritative gameplay outcomes.

#### Best-effort gameplay telemetry

Gameplay telemetry covers high-volume analytical facts such as damage, healing, experience, spell outcomes, monster kills, area presence and session summaries.

Rules:

- asynchronous collection through bounded queues;
- no synchronous database write per hit or per movement step;
- explicit sampling and aggregation policy where volume requires it;
- fail-open gameplay behavior when analytics storage is unavailable;
- observable queue drops, retry counts and lag;
- deterministic derivation and replay tests for aggregators;
- no claim that an incomplete best-effort stream proves item or currency conservation.

#### Durable economy and security audit

Security-relevant durable mutations require evidence emitted with the authoritative transaction, normally through a transactional outbox or an equivalently atomic mechanism.

This class includes item creation/destruction, split/merge, ownership/location changes, loot, pickup, trade, market, mail, depot, rewards, currency changes, rollback/retry resolution, transaction commit, session ownership and security-relevant storage transitions.

Rules:

- the authoritative mutation and its outbox/audit record succeed or fail under one accepted transaction boundary;
- stable idempotency and deduplication keys;
- ordered causation within an operation/transaction;
- replaying delivery may not replay gameplay effects;
- missing, duplicate and conflicting audit events are themselves detectable conditions;
- consumers remain read-only and cannot repair production state implicitly;
- retention, legal hold, access and deletion requirements are explicit and distinct from ordinary gameplay telemetry.

Oteryn does not adopt full event sourcing of every simulation event. Current-state tables, revisions, idempotent commands, transactional outbox and bounded critical audit records remain the default persistence direction.

### 3. Define a common versioned event envelope

The detailed schema remains gated by `ANL-01`, but every event family must converge on a common semantic envelope where applicable:

```text
event_id
event_type
event_schema_revision
occurred_at_wall
occurred_at_tick
world_id
channel_id
instance_id
node_id
actor_id
subject_id
analytics_actor_id
command_id
operation_id
transaction_id
causation_id
correlation_id
game_session_id
session_generation
state_revision
protocol_revision
content_revision
ruleset_revision
server_build
privacy_class
retention_class
durability_class
payload
payload_hash
```

Not every event contains every field. `ANL-01` must define required/optional fields by event family, scope and durability class. Runtime Rust memory layout and undocumented serializer output are not public schemas.

The following meanings must be aligned with `FND-ID-01` and `DUR-01` rather than reinvented by analytics:

- `EventId`;
- `OperationId`;
- `TransactionId`;
- `CorrelationId`;
- `CausationId`;
- `AnalyticsActorId`;
- `ItemInstanceId`;
- `CommandId`;
- world/channel/instance/session/revision identifiers.

### 4. Gameplay and balance analytics

The Gameplay & Balance consumer must support analysis of:

- experience per hour and progression rate;
- damage dealt and received;
- effective healing and overhealing;
- deaths and survival;
- mana and consumable use;
- loot, NPC value, trusted market value and profit/cost per hour;
- spell efficiency, targets, delayed effects and failed casts;
- monster performance and kill attribution;
- vocation/class balance by level bracket;
- solo versus party and shared-experience performance;
- equipment/power bands when privacy and cardinality permit;
- comparisons across `ruleset_revision`, `content_revision`, protocol revision and server build;
- gameplay regressions after code, balance or content changes.

Statistical conclusions must preserve sample size and relevant dimensions. A default dashboard must not imply that a profession is overpowered when the actual cause may be one item, spell, hunt area, party composition, content revision or small sample.

Analytics must never automatically change balance or combat outcomes.

### 5. World and content analytics

The World & Content consumer must support analysis of:

- activity and occupancy by `Area`, `Subarea`, `EncounterZone`, `RaidCell`, `RaidAnchor`, `Region` and `Chunk` at the appropriate semantic level;
- unused, overloaded or unusually lethal locations;
- travel flow without retaining unnecessary precise movement trails;
- spawn utilization and monster lifetime/kill rate;
- pathfinding, collision, teleport and accessibility problem indicators;
- content participation, abandonment and completion;
- item, spell, NPC, quest, raid and event usage;
- channel/instance distribution and cross-channel behavior;
- changes caused by a world bundle, content revision or ruleset revision.

For dynamic encounters, the system must help validate whether a player-facing `Subarea` is too broad for execution and whether `EncounterZone`/`RaidCell`/`RaidAnchor` placement better matches actual player behavior. Analytics does not redefine the authored world hierarchy accepted in ADR-0005.

Precise movement history is not a default retention requirement. Coarse or aggregated geography should be preferred unless an explicitly authorized security investigation requires finer data.

### 6. Economy and item integrity

`DUR-03` remains responsible for preventing duplication through authoritative transaction design. Game Intelligence provides independent evidence, invariant monitoring and investigation.

Every durable item instance uses `ItemInstanceId` as accepted by `DUR-01`/`DUR-03`. Relevant event families include:

```text
ITEM_CREATE
ITEM_DESTROY
ITEM_SPLIT
ITEM_MERGE
ITEM_MOVE
ITEM_PICKUP
ITEM_DROP
ITEM_LOOT
ITEM_TRADE
ITEM_MARKET_ESCROW
ITEM_MARKET_DELIVER
ITEM_MAIL
ITEM_DEPOT
ITEM_REWARD
ITEM_TRANSFORM
ITEM_ROLLBACK
CURRENCY_CREDIT
CURRENCY_DEBIT
TRANSACTION_COMMIT
TRANSACTION_ABORT
```

An item/currency event must carry enough state to verify its operation, including applicable previous/new owner and location, quantity/value before and after, item revision, source/sink reason, command/operation/transaction identity, session generation and world/channel/instance scope.

Required deterministic invariants include:

- one live `ItemInstanceId` cannot exist in two authoritative locations;
- duplicate delivery of one event does not duplicate an item, currency or aggregate;
- a split conserves total quantity and produces non-conflicting identities;
- a merge conserves total quantity and retires identities according to the accepted rule;
- every creation and currency credit has an allowed source;
- every destruction and debit has an allowed sink;
- a reward cannot commit twice for the same idempotency key;
- stale sessions/generations cannot transfer ownership;
- transaction commit/abort and outbox evidence cannot disagree silently;
- retries, timeouts, crash recovery and rollback cannot create unexplained value;
- inventory, ground, container, depot, trade, market and mail views reconcile to the authoritative owner/location model.

Alerts identify evidence requiring investigation. They are not an authorization to mutate data automatically.

### 7. Security analytics

Security Analytics is a separate consumer, not an overloaded combat dashboard. It may detect or investigate:

- bot and automation patterns;
- impossible action rates or movement;
- cooldown and sequencing violations;
- replayed commands or stale session generations;
- suspicious login, reconnect or channel-switch patterns;
- repeated rewards and exploit loops;
- unusual transfers, laundering graphs or account clusters;
- market/trade/mail/depot inconsistencies;
- protocol anomalies and capability/downgrade misuse;
- engine defects exposed by invariant violations.

Deterministic authorization, transaction safety, rate limits, fencing and invariants remain enforcement mechanisms in the authoritative runtime. An anomaly score alone must not ban a player or alter gameplay.

Security cases must retain the exact detector/rule/model version, input evidence references, timestamps, confidence, reviewer actions and final disposition so false positives can be audited.

### 8. Read-only investigation and AI

AI and investigation tooling run outside the authoritative game process and outside transaction commit paths.

They may:

- read restricted views, replicas or exported evidence packages;
- correlate multiple deterministic alerts;
- reconstruct item/currency provenance;
- identify the first affected build, content revision, ruleset or module;
- compare behavior before and after a change;
- generate hypotheses and proposed regression tests;
- prepare a human-reviewable incident package.

They may not:

- mutate runtime or production databases;
- ban or sanction solely from an anomaly/model score;
- change balance or content;
- execute rollback;
- deploy code;
- receive gameplay mutation credentials;
- turn generated hypotheses into proven findings without named evidence.

Human review, authorization and auditable case disposition are mandatory for enforcement or remediation decisions.

### 9. Privacy, access and retention

The previous Canary design demonstrated that suppressing a character name is not sufficient anonymization when player IDs remain directly linkable.

Oteryn therefore requires:

- a pseudonymous `AnalyticsActorId` for ordinary analytical datasets;
- separation between analytics identity and operational `CharacterId`/`AccountId`;
- controlled, audited mapping available only to explicitly authorized security/administration roles;
- no player identity as a standard dashboard variable;
- no player/item/transaction identity in Prometheus labels;
- explicit `privacy_class` and `retention_class` on event families/datasets;
- separate retention for operational metrics, gameplay telemetry, raw security evidence, aggregate reports and case records;
- access separation for balance analysts, operators, security investigators and administrators;
- redaction and export rules;
- audit of access to player-linked evidence;
- deletion/anonymization and legal-hold behavior defined before production collection;
- protection against re-identification through overly precise location, time or party dimensions.

`ANL-01` must produce a data-classification and retention matrix. Production collection remains disabled until the relevant classes have an accepted purpose, retention and access policy.

### 10. Reliability and failure behavior

The in-process collector must be lightweight and bounded. It must not block combat on remote analytics storage.

Gameplay telemetry may drop according to an explicit policy under sustained overload. Durable audit may not silently downgrade to best effort. When the authoritative outbox/audit write cannot satisfy its transaction contract, the owning durable gameplay operation must follow the failure semantics accepted by `DUR-02`/`DUR-03`.

Required health evidence includes:

- queue capacity, depth and oldest age;
- accepted, dropped, retried, dead-lettered and persisted counts;
- delivery lag and consumer lag;
- schema incompatibility;
- outbox backlog and oldest unpublished transaction;
- deduplication conflicts;
- invariant violations;
- privacy/redaction failures;
- bounded cardinality and storage growth.

`ANL-01` must register every externally controlled memory, allocation and transfer boundary in `RESOURCE_LIMITS_REGISTRY.json`, including event-envelope and payload bytes, queue entries/bytes, batch rows/bytes, dead-letter capacity, replay batch/window, decompression, query/result/export and evidence-package limits. Durable outbox backlog is governed by explicit operational thresholds and recovery behavior; it is never silently discarded to satisfy a memory limit.

Public and cross-component failures must map to `FOUNDATION_ERROR_VOCABULARY.md`. At minimum, `ANL-01` must define stable contract-owned outcomes for telemetry drop/capacity, audit dependency unavailable, event/schema rejection, duplicate or ordering conflict, replay conflict, privacy-policy rejection and investigation access denial without exposing private payloads.

The following stable scenarios from `FOUNDATION_FAILURE_SCENARIOS.md` are mandatory inputs to the owning contracts: `FS-ANALYTICS-TELEMETRY-OVERFLOW`, `FS-AUDIT-OUTBOX-BACKLOG`, `FS-EVENT-DUPLICATE-DELIVERY`, `FS-EVENT-OUT-OF-ORDER`, `FS-AUDIT-MUTATION-MISMATCH`, `FS-ANALYTICS-PRIVACY-POLICY`, `FS-DETECTOR-FALSE-POSITIVE` and `FS-INVESTIGATION-MUTATION-ATTEMPT`.

Replay means replaying immutable events into read-only consumers or test fixtures. It never means applying the authoritative gameplay effect again.

### 11. Canary migration classification

The Canary implementation is design evidence and a regression corpus, not the target engine subsystem.

| Canary element | Oteryn decision |
|---|---|
| Hunt/session aggregation model | `ADAPT` — derive from explicit versioned engine events. |
| UTC-day split | `REUSE` as a reporting rule with deterministic time tests. |
| Combat/death eligibility and short-death retention | `REUSE` as aggregate semantics. |
| Bounded queues | `REUSE` for all in-process collectors. |
| Retry and dead letters | `ADAPT` with explicit lifecycle, observability and operator handling. |
| Idempotent writes | `REUSE` through stable event/session IDs and deduplication. |
| Schema versioning and migration checksums | `REUSE`. |
| Retention and daily aggregates | `ADAPT` after Oteryn privacy/operations decisions. |
| Grafana metrics and minimum-sample warnings | `ADAPT`; metric definitions require reconciliation. |
| Lua callback wrappers | `DO_NOT_MIGRATE`; replace with explicit Rust domain-event emission. |
| Selected potion/rune/loot hooks | `REWRITE`; use authoritative consumption/item events. |
| MariaDB schema and integration | `REVALIDATE`; PostgreSQL is the Oteryn target. |
| Name-only anonymization | `REWRITE` as pseudonymization and role-controlled identity mapping. |
| Canary Lua/Python/shell tests | `ADAPT` scenarios; rewrite in Oteryn tooling. |
| Aggregate Gameplay Analytics as anti-duplication proof | `REJECT`; use transactional item/currency evidence and invariants. |
| AI in the engine or transaction path | `DO_NOT_MIGRATE`. |

### 12. Architecture gates

This ADR accepts the direction but does not freeze every event schema, datastore, detector or dashboard. The following stable gates govern completion:

#### `ANL-01` — Game Event and Audit Foundation Contract

Must define event envelope schemas, event families, durability classes, producers/consumers, ordering, idempotency, outbox, delivery, replay, limits, failure behavior, privacy classification, retention, schema compatibility and test fixtures.

It must be accepted before `DUR-02` and `DUR-03` are finalized where their transactions require durable audit/outbox evidence.

#### `ANL-02` — Gameplay, Balance and World Analytics Contract

Must define session/aggregate semantics, dimensions, statistical quality, version comparison, geography privacy, storage/retention, dashboards and regression acceptance.

It is required before a playable alpha claims production-grade balance/world analytics, but it does not block the minimal workspace bootstrap.

#### `ANL-03` — Economy Integrity and Security Analytics Contract

Must define item/currency provenance consumers, deterministic invariant catalogue, alert/case lifecycle, detector versioning, evidence quality, false-positive handling and enforcement separation.

`DUR-03` remains the authoritative anti-duplication prevention contract. `ANL-03` cannot weaken or replace it.

#### `ANL-04` — Read-Only Investigation and AI Contract

Must define read-only data access, case/evidence APIs, model/rule provenance, human review, prohibited actions, auditability and safe rollout. It is an expansion gate and is not required for the foundation vertical slice.

### 13. Integration with existing gates

- `FND-01` must reserve legal dependency directions and ownership boundaries for event contracts and consumers without creating speculative analytics crates.
- `FND-ID-01` must cover foundation meanings for event, operation, transaction, causation, correlation and pseudonymous analytics identities.
- `FND-03` must define in-process emission, bounded queues, clock/tick attribution, deterministic replay fixtures and fail-open/fail-closed behavior by durability class.
- `DUR-01` must freeze durable representations for event/operation/transaction identities and `ItemInstanceId`.
- `DUR-02` must define transactional outbox, audit storage, publication checkpoints, deduplication and recovery.
- `DUR-03` must define authoritative item/currency invariants and produce sufficient atomic evidence for monitoring.
- `DUR-04` must expose stable content/world/revision identifiers needed for content and world analysis.
- `VSL-01` must prove that combat, death, loot and retry-safe pickup emit correlated events and that replay into consumers does not duplicate items or analytical aggregates.

## Rejected alternatives

### Copy Canary Gameplay Analytics into Oteryn

Rejected. It is valuable evidence but depends on Lua callback interception, selected script hooks, MariaDB-specific persistence and incomplete identity/privacy semantics.

### Use only Prometheus metrics and logs

Rejected. High-cardinality player/item/transaction evidence is unsafe as metric labels and ordinary logs do not provide transactional item/currency provenance.

### Make every simulation event durable event sourcing

Rejected as the default. It would greatly increase storage, migration and operational complexity without being required for authoritative current-state persistence. Only critical durable mutations require atomic audit/outbox evidence.

### Let analytics or AI enforce gameplay automatically

Rejected. Enforcement belongs to deterministic authoritative rules and explicitly authorized human/admin workflows.

### Add analytics after the engine is complete

Rejected. Event identity, transaction/outbox boundaries, privacy classes and item provenance must be designed before persistence and transaction contracts become expensive to change.

## Consequences

### Positive

- balance and world decisions can be based on versioned evidence;
- item/currency anomalies can be investigated from authoritative provenance;
- best-effort analytics consumer or storage failure cannot normally stall combat;
- durable audit cannot silently degrade to incomplete best effort;
- privacy and retention are architecture concerns rather than late dashboard settings;
- AI remains useful without receiving mutation authority;
- Canary work is preserved as a requirements/test source without inheriting its runtime constraints.

### Costs and risks

- more contract work before final persistence and item-transaction design;
- additional outbox/audit storage and operational monitoring;
- privacy/access/retention governance is mandatory;
- detector quality and false positives require continuous validation;
- event schema evolution and cross-version replay require disciplined compatibility tests;
- exact event volume and storage targets require measurement before production defaults.

## Deferred details

The following remain for `ANL-01` through `ANL-04` or later implementation packages:

- exact serialization/IDL and topic/table naming;
- broker versus direct PostgreSQL outbox consumption;
- analytical datastore selection;
- exact sampling rates and retention periods;
- exact pseudonym rotation/linkability policy;
- detector algorithms and thresholds;
- case-management UI;
- model/provider selection;
- production dashboard technology and deployment;
- long-term archival/legal-hold technology;
- cross-world aggregate and multi-region topology.

No deferred choice may violate the accepted separation of authority, durability classes, privacy boundaries or read-only investigation.

## Acceptance evidence for later implementation

Later packages must provide, as applicable:

- duplicate delivery and out-of-order event tests;
- atomic mutation/outbox integration tests;
- item/currency conservation and provenance reconciliation tests;
- stale-session, retry, timeout, crash and rollback tests;
- bounded queue/load/soak evidence;
- schema upgrade/downgrade and replay fixtures;
- PostgreSQL integration and recovery tests;
- privacy/redaction/access-control tests;
- detector mutation tests and labelled false-positive/true-positive fixtures;
- dashboard sample-size and dimension-integrity tests;
- proof that AI/investigation credentials cannot mutate runtime or production state;
- exact-head vertical-slice evidence for correlated combat/loot/pickup events.

## Current implementation status

This ADR is architecture only. It does not create an analytics crate, event schema, database table, outbox, collector, dashboard, detector, AI agent or production collection path. Those remain blocked by their named gates and separately authorized tasks.
