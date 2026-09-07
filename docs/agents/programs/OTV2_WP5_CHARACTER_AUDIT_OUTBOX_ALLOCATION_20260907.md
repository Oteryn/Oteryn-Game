# WP5 Character Authority durable-audit / outbox companion allocation

Coordinator: #162. Source readiness: #319. Programme: #364.
Companion to `OTV2_WP5_GAME_CHARACTER_AUTHORITY_ALLOCATION_20260907`.

## Status

Prospective Game-only audit companion. **NOT_ACTIVE**. It grants no registry,
runtime, SQL, migration, analytics-backend, broker, production or external write
lease until a later explicit Work application.

```yaml
allocation_id: OTV2-WP5-CHARACTER-AUDIT-OUTBOX-20260907
repository: Oteryn/Oteryn-Game
allocation_state: NOT_ACTIVE
owner_contracts:
  - ANL-01_GAME_EVENT_AND_AUDIT_FOUNDATION_CONTRACT.md
  - DUR-02_PROFILE_NEUTRAL_CHARACTER_PERSISTENCE_OWNER_BASELINE.md
  - CHARACTER_AUTHORITY_PLATFORM_BOUNDARY.md
character_mutation_activation_without_this_dependency: FORBIDDEN
```

## Why this companion is mandatory

Accepted Character Authority requires durable audit evidence for ownership-
sensitive mutations. Accepted DUR-02 requires the authoritative mutation,
CharacterRevision/receipt, mandatory event records and publication state to
commit atomically. ANL-01 further requires `DURABLE_AUDIT`, immutable same-EventId
semantics, TransactionEventRef completeness, at-least-once publication and
EventId deduplication.

Protected `docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json` intentionally has
an empty `event_types` list. Therefore Character Authority must not invent event
IDs/payloads in runtime code or claim owner/bootstrap/transfer completion before
an exact domain-event registration is separately protected.

## Prospective conditional surfaces

After explicit Work application and fresh ownership readback:

```text
docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json              # exact event registration
docs/contracts/game-events/v1/character_authority.proto         # new payload IDL
apps/game-server/src/durability/character_authority_audit.rs     # new typed outbox/audit owner
apps/game-server/src/durability/character_authority.rs           # integration inside same txn
apps/game-server/src/durability/db.rs                            # exact adapter hook only
apps/game-server/src/durability/schema.rs                        # exact codec/schema hook only
apps/game-server/migrations/<NEXT>_character_authority.sql       # same Character Authority forward migration
apps/game-server/tests/character_authority_postgres.rs           # same dedicated PG target
```

Registry, existing Durability files and migration paths are conditional future
leases. No current path is writable from this document. Exact event IDs and the
next migration number are selected only after fresh protected registry/WP4
readback under one serialized coordinator allocation.

No broker/warehouse/analytics detector is required for the first authoritative
commit path. The Durability-owned transactional outbox is the publication source;
external consumers remain later ANL owners.

## Event registration contract

Every Character Authority mutation enabled in the selected first slice must map
to a registered ANL-01 event type. At minimum the eventual registration must
cover the exact enabled semantics among:

- initial authoritative Character owner/world bootstrap or create;
- AccountId ownership transfer;
- WorldId transfer;
- lifecycle transition/terminal retirement;
- rename if the selected product slice enables it and its accepted owner requires
  durable audit.

Disabled mutation families require no speculative event ID, but cannot be
silently enabled later without registration.

Each registered type must define all fields required by
`GAME_EVENT_FOUNDATION_REGISTRY.json`: unique never-reused uint32 event type ID,
payload schema/message, nonzero schema revision, `DURABLE_AUDIT`, appropriate
privacy floor, a real registered finite retention profile, and explicit atomic
mutation evidence. Placeholder retention IDs are forbidden; production
activation remains blocked until the required profile exists.

Payloads carry only the minimum audit facts required to prove the semantic
transition and correlation. AccountId/CharacterId relations are
`RESTRICTED_PLAYER_LINKED` or stricter and are not public projection data.

## Transactional semantics

For one accepted Character mutation transaction:

1. allocate stable OperationId/TransactionId/EventId values under their owning
   contracts before authoritative commit;
2. validate current owner/world/lifecycle/revision/session fences;
3. stage the Character mutation and exactly one CharacterRevision successor;
4. stage immutable operation receipt;
5. stage every mandatory registered `DURABLE_AUDIT` event with a complete
   TransactionEventRef (`TransactionId`, contiguous ordinal, final count);
6. stage durable outbox/publication state;
7. COMMIT all of the above atomically.

If required event encoding/registration/outbox reservation cannot be established,
the authoritative Character mutation does not commit. There is no mutation-first
best-effort audit.

Publication is at-least-once **after** commit. Publisher outage does not roll back
the committed Character state; the durable outbox remains pending. Retry reuses
the same EventId and exact immutable payload bytes. Consumer deduplication uses
EventId. Publication cannot replay authoritative gameplay/domain mutation.

Ambiguous Character operation reconciliation returns the same committed receipt
and corresponding immutable audit identities. A retry must not create duplicate
semantic events or a second CharacterRevision.

## RED/GREEN evidence

RED before implementation:

- a Character owner/world/lifecycle mutation can currently be described without
  any registered domain audit event/outbox, proving the accepted atomic audit
  invariant is absent.

GREEN requires actual PostgreSQL 17.6 tests (through the protected dedicated-PG
CI routing prerequisite) proving:

- valid owner bootstrap/transfer commits Character root + revision + receipt +
  mandatory outbox records atomically;
- injected event-registration/encoding/outbox failure rolls back the Character
  mutation and revision;
- DB rollback after staged event produces neither mutation nor durable event;
- lost commit response reconciles one receipt and the same EventIds;
- retry after committed result does not duplicate CharacterRevision/event;
- same EventId with changed semantic envelope or payload bytes rejects;
- TransactionEventRef ordinals/count are complete and deterministic;
- pending publication survives restart and is delivered at least once without
  replaying the Character mutation;
- duplicate delivery is deduplicated by EventId at the consumer fixture boundary;
- publication outage/backlog never downgrades `DURABLE_AUDIT` to best-effort;
- privacy projection does not expose AccountId/alternate-character relation;
- unregistered event type/schema/retention profile blocks production activation.

## Integration prerequisites

This companion and the main Character Authority allocation are both required
before any ownership-sensitive Character mutation is activated. Material source
acceptance additionally requires:

- protected WP3/WP4 release and exact migration/path application;
- protected `OTV2_WP5_DEDICATED_POSTGRES_CI_ROUTING_20260907` activation for
  `character_authority_postgres`;
- exact event-type/retention registration through a separately reviewed registry
  mutation;
- independent high-risk whole-diff review;
- canonical CI, full Merge Queue and protected readback.

Until all are true, ownership transfer/create/bootstrap/lifecycle mutation remains
fail-closed/disabled. A read-only projection cannot be relabeled as complete
Character Authority implementation.

## Excluded scope

No analytics warehouse/broker/detectors, Platform write, public alternate-
character disclosure, production retention policy invention, workflow source,
registry edit, SQL/migration source, external system or release/deployment action.

Runtime E2E is NOT_APPLICABLE to this allocation-only document.
