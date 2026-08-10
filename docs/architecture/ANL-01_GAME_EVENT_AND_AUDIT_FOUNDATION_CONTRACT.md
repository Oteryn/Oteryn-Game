# ANL-01 — Game Event and Audit Foundation Contract

- Status: Candidate; canonical only when the owning ANL-01 delivery merges
- Date: 2026-08-10
- Gate: `ANL-01`
- Issue: #135
- Trusted base: `main@ef42fa47ab054ab8aa304c017307c1945f931b59`
- Scope: event/audit semantics, interchange, privacy and evidence foundation only

## 1. Purpose

ANL-01 freezes the evidence contract required by ADR-0006 before DUR-02/DUR-03 may finalize transactional outbox and critical economy/security evidence. It is not event sourcing and never becomes gameplay authority.

```text
authoritative gameplay/session/runtime -> FND/gameplay owners
physical persistence/transactions       -> DUR-02
item/currency conservation              -> DUR-03
event/audit evidence semantics          -> ANL-01
analytics/security consumers            -> ANL-02/03
investigation/AI                        -> ANL-04 read-only
```

## 2. Event interchange v1

```text
family: oteryn-game-events
envelope revision: 1
serialization: Protocol Buffers binary
source IDL: proto3, docs/contracts/game-events/v1/foundation.proto
registry: docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json
application event compression: none
```

This is independent from `protocol-oteryn`. No broker, DB schema, warehouse/lake/dashboard product is selected.

Protobuf bytes are **not assumed to be a universal canonical semantic serialization**. Same-EventId retry/redelivery therefore preserves exact payload bytes and all immutable semantic envelope values rather than decode+reserialize.

## 3. Canonical identities

### EventId

Strong UUIDv7, full 128 bits, globally identifies one immutable event, event-producer owned, nil invalid, never reused. Delivery retry/redelivery/replay reuses EventId. Same EventId with conflicting immutable event content is an integrity conflict, never an update.

A best-effort observation dropped before event admission need not mint EventId. A durable EventId is fixed before/inside the owning transaction.

### OperationId

Strong UUIDv7 for one independently durable multi-step/retry-capable logical operation. Optional; not generated for every command. Reconciliation/retries of the same logical operation retain it. CommandRef is distinct.

### TransactionId

Strong UUIDv7 for one logical atomic durable mutation transaction, issued by its authoritative durable transaction coordinator/owner. Ambiguous commit/retry retains the same ID. Physical DB attempt IDs are implementation-local. A new TransactionId requires the prior logical transaction to be proven terminal and an intentionally new logical transaction to begin.

### CorrelationId

Strong UUIDv7 for one trusted bounded correlation context. Grouping only; never authorization, causality or order. Untrusted client trace/correlation input does not become canonical CorrelationId by assertion.

### CausationRef

No separately minted CausationId UUID exists. Immediate cause is one typed reference:

```text
Event(EventId)
Command(GameSessionId, CommandId)
Operation(OperationId)
Transaction(TransactionId)
```

A genuine root event has no cause.

### AnalyticsActorId

Strong UUIDv7 issued by analytics privacy-identity authority and interpreted only as:

```text
analytics_identity_domain + identity_epoch + AnalyticsActorId
```

Rules:

- never AccountId/CharacterId/public identity;
- never reversible/truncated derivation from raw identity/name/email;
- purpose/domain scoped rather than one universal pseudonym;
- identity_epoch is nonzero and policy-owned;
- **a new epoch requires a fresh AnalyticsActorId for the same operational actor**, so ordinary datasets cannot link epochs by ID equality;
- any privileged cross-epoch/operational mapping is separately protected, purpose-authorized and access-audited;
- pseudonymous families never fall back to raw CharacterId when mapping is unavailable;
- audit requiring raw identity uses RESTRICTED_PLAYER_LINKED or SECURITY_SENSITIVE instead.

Exact epoch duration remains privacy-policy owned and must be concretely accepted before production use.

## 4. Durable physical representation

Where ANL-owned UUIDv7 identities are stored in native game PostgreSQL, DUR-01 applies unchanged: PostgreSQL `uuid`, full 128 bits, strong type, nil invalid, no reuse. If CommandId is persisted, DUR-01 full-range `numeric(20,0)` remains authoritative.

ANL-01 does not define table/index layout.

## 5. Durability classes

### BEST_EFFORT_TELEMETRY

Bounded asynchronous collection; registered sampling/aggregation/drop allowed; gameplay fail-open on analytics dependency loss; accepted/dropped/retried/lag evidence counted; gaps explicit; never sole economy/security completeness proof.

### DURABLE_AUDIT

Never sampled/dropped; mandatory mutation evidence shares the owning accepted atomic transaction; publish only after commit; at-least-once delivery; immutable EventId/event content across retry; consumer dedupe required; replay never replays gameplay mutation; committed backlog never discarded for in-memory pressure.

A non-mutating security observation may be durable audit without transaction context only when the event type declares `atomic_mutation_evidence=false`.

Operational metrics/logs remain separate.

## 6. Envelope v1

Normative source is `docs/contracts/game-events/v1/foundation.proto`.

Every admitted event requires: envelope revision 1, EventId, registered event type ID/schema revision, durability class, privacy class, accepted retention profile ID before production, trusted-server wall timestamp, bounded build ID, registered exact payload bytes and SHA-256 of those exact bytes.

Optional typed context includes world/channel/instance/node, GameSession + connection generation/CommandId, RuntimeOrderRef, OperationId, TransactionEventRef, CorrelationId, CausationRef, AnalyticsActorRef and applicable protocol/ruleset/content revisions.

Generic actor/subject/object IDs and arbitrary metadata maps are excluded. Domain identities live in typed payloads.

All semantic envelope fields become immutable when EventId is admitted. Retry/redelivery of that EventId reuses the same values. Raw protobuf envelope byte equality is not required because field serialization order is not semantic.

Validation:

- UUID fields are exact 16-byte non-nil values and UUIDv7 where owned as such;
- ChannelId/InstanceId preserve WorldId scope;
- CommandId and connection_generation are nonzero and require GameSessionId;
- RuntimeOrderRef requires concrete WorldId+ChannelId or WorldId+InstanceId;
- TransactionEventRef is atomic/all-or-none context;
- registry/revision/policy strings are bounded by resource/profile grammar;
- payload SHA-256 matches exact payload bytes;
- credentials, tickets, reconnect proofs, private keys and secrets are forbidden.

Wall timestamp is correlation evidence only, never ordering/fencing authority.

## 7. RuntimeOrderRef

A naked RuntimeExecutionOrdinal is prohibited because FND-03 scopes it to a semantic runtime scope plus ownership generation.

```text
RuntimeOrderRef {
  scope_ownership_generation > 0
  runtime_execution_ordinal > 0
}
```

Envelope must additionally identify `WorldId + ChannelId` or `WorldId + InstanceId`.

RuntimeOrderRef is internal evidence, stripped from ordinary/public analytical projections unless an accepted purpose explicitly requires it. It never becomes protocol/client authority or cross-generation/global order.

## 8. TransactionEventRef

Transaction context is structurally represented as one message, preventing partial TransactionId/ordinal/count combinations:

```text
TransactionEventRef {
  transaction_id: UUIDv7
  ordinal: 1..count
  count: nonzero
}
```

For each committed TransactionId event set:

- every event declares the same count;
- ordinals are exactly contiguous `1..count`;
- no ordinal duplicates;
- TransactionEventRef is immutable;
- this order never extends across transactions.

If a non-atomic event only needs to say “transaction X caused this later observation”, it uses `CausationRef::Transaction` rather than incomplete transaction membership.

## 9. Exact payload byte stability

Protobuf serialization is not used as semantic canonicalization.

For each event:

1. producer materializes exact registered payload bytes once;
2. computes SHA-256 over those bytes;
3. binds EventId + all immutable semantic envelope values + exact payload bytes before the first possibly ambiguous durable commit/publication attempt;
4. same-EventId retry/reconciliation/redelivery reuses all those values and exact payload bytes;
5. it must not reconstruct/reserialize the event from mutable domain state;
6. durable implementation retains exact bytes or an equivalently proven byte-preserving representation;
7. payload hash is integrity/conflict evidence, not event identity.

Conflict validation compares immutable semantic envelope values plus exact payload bytes/hash, not raw serialized envelope bytes.

## 10. Event type registry/evolution

- type ID 0 invalid; positive uint32 IDs never reused;
- every type declares owner, payload schema/message, current nonzero schema revision, durability, privacy floor, retention profile, atomic-mutation-evidence flag;
- removed protobuf fields are reserved;
- additive evolution allowed only when older consumers stay safe;
- unsafe semantic change requires new event type/family-major decision;
- producer cannot emit unregistered canonical production schema;
- unsupported durable schema quarantines/rejects without evidence loss/reinterpretation;
- unsupported best-effort schema may counted-drop but never claim completeness;
- EventId content never changes because registry revision advances.

The initial registry intentionally contains no speculative gameplay-domain event IDs.

## 11. Ordering model

No global event total order.

```text
CommandRef                  per-GameSession command order
RuntimeOrderRef             scope+ownership-generation-local runtime order
TransactionEventRef         one atomic transaction event set/order
CausationRef                immediate causal edge
CorrelationId               grouping only
domain/state revisions       authoritative state progression
wall time                    cross-system correlation only
EventId UUIDv7 sorting       storage locality only
```

Cross-transaction operation progression uses causation + owning domain revisions. No synchronized global/operation ordinal is introduced.

## 12. Atomic mutation/audit boundary

For each mutation whose owner requires durable audit, before the first possibly ambiguous commit the TransactionEventRef set, EventIds, all immutable semantic envelope fields and exact event payload bytes are fixed.

Valid outcome:

```text
authoritative mutation + every mandatory durable audit record commit
OR
neither becomes authoritative
```

Mutation committed with missing mandatory audit evidence is invalid and must not remain silent. DUR-02/DUR-03 prove the physical transaction/outbox mechanism.

## 13. Publication, duplicate delivery and replay

```text
committed record -> at-least-once publish -> idempotent consumer
```

Only committed records publish. Lost ack may redeliver. EventId/event content remains unchanged. Transport delivery IDs are not EventId. Consumer applies at most one derived effect per identical EventId and advances checkpoint only after its projection effect is safely accepted. No broker-level exactly-once assumption.

Same EventId + same immutable event = idempotent duplicate. Same EventId + conflicting immutable event = `ANL_EVENT_ID_CONFLICT`, no overwrite/last-write-wins.

Replay is read-only toward gameplay and never resubmits original commands/credentials.

## 14. Out-of-order behavior

Consumers use TransactionEventRef complete-set information, CausationRef and authoritative domain revisions to bounded-buffer, defer, quarantine/reject or rebuild from explicit derived checkpoints. They never fabricate missing authoritative state.

## 15. Privacy classes

1. `INTERNAL_NON_PERSONAL`.
2. `PSEUDONYMOUS_ANALYTICS` — approved AnalyticsActorRef, raw AccountId/CharacterId forbidden by family contract.
3. `RESTRICTED_PLAYER_LINKED` — operational player/session/item audit/admin evidence; least privilege + access audit.
4. `SECURITY_SENSITIVE` — abuse/recovery/security evidence; strongest role/export/redaction/access controls.

Privacy class is a minimum floor and cannot be silently downgraded by projection.

## 16. Retention/production collection gate

Every concrete event type binds an accepted retention profile before production collection. Profile defines purpose, privacy class, finite ordinary retention duration/ceiling, allowed roles, aggregation/anonymization transition if any, deletion/anonymization, export/redaction, explicit legal-hold authorization/audit and policy revision/rollout/rollback.

Ordinary unlimited retention is forbidden. Legal hold is explicit exception. Unresolved purpose/privacy/retention/access rejects production collection/projection; privacy never fails open.

ANL-01 intentionally does not guess jurisdiction/product-specific duration numbers.

## 17. Pseudonym/access boundary

Mapping AnalyticsActorId to operational identity is separate from ordinary analytical storage, least-privilege/purpose-authorized and every lookup audited. New epoch gets new pseudonym and ordinary consumers receive no implicit cross-epoch join key. Investigation/AI remains read-only and cannot mutate runtime/DB, sanction, balance, rollback or deploy.

If pseudonym resolution is unavailable for best-effort pseudonymous telemetry, policy may count/drop; it may never substitute raw identity.

## 18. Observability/cardinality

AccountId, CharacterId, AnalyticsActorId, ItemInstanceId, TransactionId, EventId, GameSessionId and similar high-cardinality/player-linked IDs are forbidden as ordinary Prometheus labels. Operational logs use bounded/redacted correlation references only; secrets/private payloads are not debug logging material.

## 19. Error contract

| Code | Category | Disposition |
|---|---|---|
| `ANL_TELEMETRY_CAPACITY_DROPPED` | `CAPACITY_EXCEEDED` | counted best-effort loss; gameplay unchanged |
| `ANL_AUDIT_DEPENDENCY_UNAVAILABLE` | `DEPENDENCY_UNAVAILABLE` | no downgrade; DUR owner decides fail/hold |
| `ANL_EVENT_MALFORMED` | `INVALID_INPUT` | no consumer effect |
| `ANL_EVENT_SCHEMA_UNSUPPORTED` | `UNSUPPORTED_REVISION` | no reinterpretation/downgrade |
| `ANL_EVENT_ID_CONFLICT` | `CONFLICT` | no overwrite/merge |
| `ANL_EVENT_ORDER_BLOCKED` | `CONFLICT` | bounded defer/quarantine; no invented state |
| `ANL_REPLAY_CONFLICT` | `CONFLICT` | affected projection stops; gameplay unchanged |
| `ANL_PRIVACY_POLICY_REJECTED` | `CONFLICT` | no collection/projection/disclosure |
| `ANL_INVESTIGATION_ACCESS_DENIED` | `CONFLICT` after authentication | no disclosure |
| `ANL_EVIDENCE_INTEGRITY_FAILURE` | `INTERNAL_UNAVAILABLE` | fail safe; retain bounded evidence |

Unauthenticated investigation requests remain `AUTHENTICATION_FAILED`. Diagnostics redact restricted IDs/payloads/credentials.

## 20. Resource limits

Shared `RESOURCE_LIMITS_REGISTRY.json` is normative. ANL entries bound event/payload/string/nesting, best-effort queue, publication/quarantine/replay batch, replay window, query page/result and evidence package.

Limits are absolute security/allocation ceilings, not tuned defaults/capacity guarantees. Lower configuration and PERF/OPS evidence are allowed/required. Committed durable backlog is not dropped to satisfy in-memory limits.

## 21. Failure scenarios

- FS-ANALYTICS-TELEMETRY-OVERFLOW: PASS.
- FS-AUDIT-OUTBOX-BACKLOG: semantic PASS; physical DUR-02.
- FS-EVENT-DUPLICATE-DELIVERY: PASS.
- FS-EVENT-OUT-OF-ORDER: PASS.
- FS-AUDIT-MUTATION-MISMATCH: semantic PASS; physical DUR-02/03.
- FS-ANALYTICS-PRIVACY-POLICY: PASS.
- FS-DB-OUTBOX-BOUNDARY: semantic PASS; physical DUR-02.
- FS-DETECTOR-FALSE-POSITIVE: DEFERRED_BY_ACCEPTED_GATE to ANL-03.
- FS-INVESTIGATION-MUTATION-ATTEMPT: DEFERRED_BY_ACCEPTED_GATE to ANL-04 implementation evidence.

Architecture PASS does not claim runtime evidence.

## 22. Required implementation evidence

Future conformance requires protobuf golden/cross-language fixtures; malformed/oversized/fuzz/property corpus; identity/scope/CommandRef validation; exact full-event semantic-value + payload-byte preservation across ambiguous commit/publication retry; EventId duplicate/conflict tests; RuntimeOrderRef scope/generation tests; TransactionEventRef complete-set/duplicate/gap/inconsistent-count tests; commit/publication crash tests; consumer checkpoint recovery; out-of-order behavior; unsupported schema by durability class; best-effort overflow counters; durable backlog no-loss; pseudonymous raw-ID rejection; fresh pseudonym across epochs + privileged mapping access audit; unresolved retention policy collection rejection; bounded replay/query/export; and proof replay/investigation cannot mutate gameplay.

Physical PostgreSQL atomicity tests belong to DUR-02/DUR-03 when implementation is authorized.

## 23. Downstream/non-goals

After ANL-01 lifecycle closeout, DUR-02 may finalize persistence/outbox/checkpoints and DUR-03 may define item/currency critical event families/conservation evidence. ANL-02/03/04 refine consumers under their own gates.

ANL-01 selects no DB layout/isolation/locks/migrations/RPO/RTO, broker/warehouse/lake/dashboard, collector, exact legal/product retention duration, speculative domain payload catalogue, detector threshold/auto-ban, AI mutation, Platform write, deployment or production collection.

## 24. Acceptance rule

ANL-01 is accepted only when analysis/contract/IDL/registry/resource limits are internally consistent with accepted ADR-0006/FND/DUR-01, exact-head governance/CI pass, terminal architecture/security/privacy/data-integrity review has zero material findings, zero material review threads remain, accepted head is squash-merged unchanged, and a separate lifecycle closeout archives ownership and closes Issue #135.
