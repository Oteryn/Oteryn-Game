# ANL-01 — Game Event and Audit Foundation Analysis

- Status: bounded decision analysis for Issue #135
- Date: 2026-08-10
- Trusted base: `main@ef42fa47ab054ab8aa304c017307c1945f931b59`
- Gate: `ANL-01`
- Does not authorize: runtime, PostgreSQL schema, broker, warehouse, detector, Platform or production implementation

## 1. Purpose and fixed boundaries

ADR-0006 already accepts three distinct data classes: low-cardinality operational observability, bounded best-effort gameplay telemetry and durable economy/security audit. ANL-01 closes identity, event interchange, causation/order, publication/idempotency and privacy/retention semantics so DUR-02/DUR-03 can proceed without inventing incompatible evidence rules.

ANL-01 is not event sourcing or gameplay authority. Command/session/runtime remain FND-owned; persistence transactions remain DUR-02; item/currency conservation remains DUR-03; consumers/detectors/investigation remain ANL-02/03/04.

## 2. Interchange technology

Options considered: custom binary, JSON, Avro, Protobuf, implementation-defined serialization.

Decision: `oteryn-game-events` v1 uses Protocol Buffers binary with proto3 source IDL. It is broker/storage independent and separate from `protocol-oteryn`. Application-level event compression is disabled in v1.

Protobuf is an interchange schema, **not a universal canonical semantic byte encoding**. This distinction drives the exact-byte retry rules below.

## 3. Identity catalogue

- **EventId**: producer-owned global UUIDv7 for one immutable admitted event; nil invalid; never reused.
- **OperationId**: owner-issued UUIDv7 for one independently durable multi-step/retry logical operation; optional, not per-command by default.
- **TransactionId**: durable transaction-owner UUIDv7 for one logical atomic transaction; same ID through ambiguous commit/retry; physical DB attempts are internal.
- **CorrelationId**: trusted-boundary UUIDv7 for bounded grouping only; external client trace text is not canonical authority.
- **CausationRef**: typed immediate reference to EventId, CommandRef, OperationId or TransactionId; no separately minted causation UUID.
- **AnalyticsActorId**: privacy-authority UUIDv7 scoped by analytics identity domain + nonzero epoch; pseudonymous only.

A new analytics identity epoch must produce a **fresh AnalyticsActorId for the same operational actor**, otherwise epoch rotation would preserve ordinary cross-epoch linkability and defeat its privacy purpose. Privileged mapping may correlate epochs only under explicit authorization/audit.

## 4. Durability classes

`BEST_EFFORT_TELEMETRY`: bounded asynchronous collection, sampling/aggregation/drop allowed by registered policy, counted loss, gameplay fail-open, no conservation/security completeness claim.

`DURABLE_AUDIT`: never sampled/dropped, mandatory mutation evidence atomic with the authoritative mutation, publish only after commit, at-least-once, EventId/content immutable, consumer dedupe required, replay read-only, committed backlog never discarded for RAM pressure.

Operational observability remains outside the game-event durability enum.

## 5. Envelope design

Common envelope carries stable event identity/type/schema/durability/privacy/retention/time/build/payload plus optional typed world/runtime/session/operation/correlation context.

Rejected: generic actor/subject/object UUIDs and arbitrary metadata maps. These erase semantic identity type, blur privacy classification and create unbounded/cardinality hazards. Domain IDs belong to registered typed payloads.

All semantic envelope values become immutable once EventId is admitted. Same-EventId retry/redelivery reuses those values. Raw serialized envelope byte equality is not required because Protobuf field serialization order is not semantic.

## 6. Repair cycle 1 findings

### 6.1 Exact payload byte stability

Problem: reserializing equivalent Protobuf payload state on a retry can produce different valid bytes/hash.

Repair: materialize exact payload bytes once, compute SHA-256 over those bytes, bind them to EventId/type/schema and immutable envelope state before the first possibly ambiguous durable attempt, and reuse the exact bytes on retry/redelivery. Do not reconstruct from mutable domain objects. Durable implementation must retain exact bytes or prove equivalent byte preservation.

### 6.2 Runtime execution order

Problem: naked RuntimeExecutionOrdinal is ambiguous after ownership-generation change because FND-03 scopes it to semantic runtime scope + generation.

Repair: typed `RuntimeOrderRef(scope_ownership_generation, runtime_execution_ordinal)` plus required concrete WorldId+ChannelId or WorldId+InstanceId scope. Both values nonzero. It is internal evidence, not protocol/global order.

### 6.3 Transaction event completeness

Problem: ordinal alone cannot prove whether an event set is complete.

Initial repair added count; cycle 2 improves this structurally via TransactionEventRef below.

## 7. Repair cycle 2 findings

### 7.1 Transaction context must be all-or-none

Three independent TransactionId/ordinal/count fields permit malformed partial combinations.

Repair: one typed `TransactionEventRef { transaction_id, ordinal, count }`. Any event claiming transaction membership carries the complete reference. For a committed transaction event set, every event has the same count and ordinals are exactly contiguous `1..count`, with no duplicates. A later observation merely caused by a transaction uses `CausationRef::Transaction` instead of incomplete membership.

### 7.2 Epoch rotation must actually reduce linkability

Problem: scoping AnalyticsActorId by epoch still allowed the same UUID value to be reused in another epoch, preserving ordinary linkability.

Repair: same operational actor receives a fresh pseudonym in each new epoch. Ordinary consumers have no implicit cross-epoch join key; only separately authorized/audited mapping may correlate.

### 7.3 Full semantic envelope immutability

Problem: exact payload bytes alone do not prevent same EventId from drifting in wall time, scope, correlation, privacy or other envelope bindings across an ambiguous retry.

Repair: EventId binds **all semantic envelope field values plus exact payload bytes** at event admission/materialization. Same-EventId retry/redelivery reuses them. Envelope protobuf raw byte ordering itself need not be identical.

## 8. Registry/schema evolution

Separate event registry with nonzero uint32 IDs, never reused; each concrete type declares owner, payload schema/message, current nonzero schema revision, durability, privacy floor, retention profile and atomic-mutation-evidence flag. Removed protobuf fields reserved. Additive evolution only when old consumers remain safe; unsafe semantic change requires new type/family-major decision. Durable unsupported schemas quarantine/reject without loss/reinterpretation; best-effort unsupported may counted-drop but never claim completeness.

The initial type list is intentionally empty; gameplay/DUR owners register real domain events later.

## 9. Ordering and causality

No global event order.

- CommandRef: per-session command order.
- RuntimeOrderRef: semantic-scope + ownership-generation-local runtime order.
- TransactionEventRef: one atomic transaction event set/order.
- CausationRef: immediate causal edge.
- CorrelationId: grouping only.
- Domain/state revisions: authoritative state progression.
- Wall time: cross-system correlation only.
- EventId UUIDv7 order: storage locality only.

No globally synchronized operation ordinal is invented.

## 10. Atomic audit/publication

For mutations requiring audit, before the first ambiguous commit the TransactionEventRef set, EventIds, all immutable semantic envelope values and exact payload bytes are fixed.

Valid commit: authoritative mutation + every mandatory audit record, or neither.

After commit, publication is at-least-once. Lost acknowledgement may redeliver identical EventId/content. Consumer deduplicates EventId and advances its checkpoint only after its own projection is safely accepted. No broker exactly-once assumption or gameplay replay.

## 11. Privacy/retention

Privacy classes: INTERNAL_NON_PERSONAL, PSEUDONYMOUS_ANALYTICS, RESTRICTED_PLAYER_LINKED, SECURITY_SENSITIVE. Privacy is a minimum floor.

Raw AccountId/CharacterId cannot enter pseudonymous families. AnalyticsActor mapping is separated from ordinary datasets and privileged lookups are purpose-authorized/audited. High-cardinality player/item/event/session IDs are forbidden as ordinary Prometheus labels.

Every production event type requires an accepted retention profile with purpose, privacy class, **finite ordinary retention duration/ceiling**, permitted roles, export/redaction, deletion/anonymization, legal-hold rules and policy revision. Ordinary unlimited retention is forbidden. Missing policy blocks production collection; privacy never fails open.

Exact jurisdiction/product durations remain policy-owned rather than guessed by ANL-01.

## 12. Errors/resource safety

ANL narrower codes map to existing Foundation categories: telemetry capacity->CAPACITY_EXCEEDED; audit dependency->DEPENDENCY_UNAVAILABLE; malformed->INVALID_INPUT; unsupported schema->UNSUPPORTED_REVISION; event/order/replay/privacy/access conflicts->CONFLICT after authentication where applicable; evidence integrity->INTERNAL_UNAVAILABLE.

Shared Resource Limits Registry now bounds event/payload/string/nesting, best-effort queue entries/bytes, publication/quarantine/replay batches, replay window, query page/result and evidence-package size. These are absolute security/allocation ceilings, not tuned throughput claims. Implementations may configure lower values and require PERF/OPS evidence. Durable committed backlog cannot be dropped because an in-memory limit is reached.

## 13. Failure scenarios

Semantic PASS: FS-ANALYTICS-TELEMETRY-OVERFLOW, FS-AUDIT-OUTBOX-BACKLOG (physical DUR-02), FS-EVENT-DUPLICATE-DELIVERY, FS-EVENT-OUT-OF-ORDER, FS-AUDIT-MUTATION-MISMATCH (physical DUR-02/03), FS-ANALYTICS-PRIVACY-POLICY and FS-DB-OUTBOX-BOUNDARY (physical DUR-02).

Deferred: detector false positive to ANL-03; investigation mutation-attempt implementation proof to ANL-04. ADR-0006 still prohibits automatic sanctions/mutations.

## 14. Required future evidence

Cross-language/golden Protobuf fixtures; malformed/oversized/fuzz/property corpus; ID/scope/CommandRef validation; exact event semantic-field + payload-byte preservation across ambiguous retry; EventId duplicate/conflict; RuntimeOrderRef generation/scope; TransactionEventRef complete set/gap/duplicate/inconsistent count; commit/publication crash paths; consumer checkpoint recovery; out-of-order behavior; unsupported schema class behavior; telemetry overflow counters; durable backlog no-loss; raw-ID rejection in pseudonymous families; fresh pseudonym per epoch + mapping access audit; retention collection gate; bounded replay/query/export; and read-only replay/investigation.

## 15. Closed decisions

ANL-01 now closes event interchange, identity ownership, typed causation, durability classes, immutable envelope semantics, exact byte stability, owner-scoped runtime ordering, atomic TransactionEventRef completeness, no global order, mutation/audit atomicity, at-least-once/idempotent delivery, privacy/pseudonym/retention gate, error/failure/resource integration.

No unresolved ANL-01 semantic issue blocks DUR-02/DUR-03 architecture. Physical DB/broker topology, exact retention durations, domain payload families and analytics products remain downstream.