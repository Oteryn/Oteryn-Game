# DUR-01 — Durable Identifier Representation Analysis

- Status: bounded decision analysis for Issue #111
- Date: 2026-08-10
- Trusted base: `main@adb0882a5ddbe42944fe955f5effb78fd5495422`
- Gate: `DUR-01`
- Does not authorize: schema/runtime/Platform/deployment implementation

## 1. Purpose

This analysis resolves the physical/durable identity questions that must be closed before `DUR-02` can define persistence schema/transactions and before `DUR-03` can define item/economy conservation without identity ambiguity.

It does not redefine accepted semantic identity ownership. `FND-ID-01` remains authority for foundation identifier meaning, owner, issuer, scope and lifetime.

The guiding split is:

```text
FND-ID-01 = what an identity means
DUR-01    = how durable identity is represented safely
DUR-02    = how persistent state and transactions are structured
DUR-03    = how item/currency mutations conserve authoritative state
ANL-01    = how event/audit identities and envelopes are defined
```

## 2. Accepted inputs and reconciled cross-repository facts

### 2.1 PostgreSQL boundary

ADR-0004 fixes PostgreSQL as the native game relational store and requires a separate `oteryn_game` ownership boundary. Platform/game links are stable contract identities, not cross-database foreign keys.

### 2.2 Canonical AccountId is no longer ambiguous

Platform ADR 0028 is accepted and defines:

```text
AccountId = Platform-issued strongly typed UUIDv7, full 128 bits
```

The Platform-local `identities.id` integer remains a local surrogate and is **not** native `AccountId`. `canary_account_id` is legacy compatibility state and is also **not** native `AccountId`.

Therefore DUR-01 does not need an opaque text/byte compatibility representation for native AccountId. Native AccountId can use the same lossless PostgreSQL UUID physical class as the other accepted UUIDv7 identities while preserving Platform ownership/issuance.

### 2.3 Foundation UUID identities

Accepted UUIDv7 foundation identities include `AccountId`, `CharacterId`, `WorldId`, `ChannelId`, `NodeId`, `InstanceId`, `PartyId`, `GameSessionId` and conditional `HandoffId`, preserving all 128 bits.

The fact that they share a 128-bit physical representation does not make them interchangeable semantic types.

### 2.4 Wire/runtime authority remains separate

- FND-02 owns gameplay wire UUID encoding and `CommandId` semantics.
- FND-03 owns runtime-local/generation-fenced handles and execution ordering.
- FND-04 owns session/admission/reconnect/lease authority.

DUR-01 cannot use database identity layout to redefine any of those contracts.

## 3. Decision question: PostgreSQL representation

### Options considered

1. `uuid` native PostgreSQL type.
2. `bytea(16)`-style binary storage.
3. canonical UUID text (`char(36)`/`text`).
4. two `bigint` halves.
5. extracted timestamp + random suffix columns.

### Analysis

Native PostgreSQL `uuid` preserves the complete 128-bit UUID value, supports exact equality/indexing without text parsing, avoids endianness conventions invented by application code, and does not expose a custom storage encoding that Rust/Laravel/SQL tooling could interpret differently.

`bytea`, text, two-integer and decomposed layouts add canonicalization, byte-order, parsing or migration hazards without adding semantic value. Timestamp decomposition is especially unsafe because UUIDv7 time order is not authority, causal order, fencing or business chronology.

### Decision

All canonical UUID-backed durable identities stored in `oteryn_game` use PostgreSQL native `uuid` as the physical scalar representation.

DUR-01 does not require PostgreSQL `DOMAIN` objects for each identity type. Semantic typing is mandatory in Rust/domain APIs and schema relationships, while table/column layout remains DUR-02-owned. A later DUR-02 implementation may introduce SQL domains only if migration/tooling evidence proves they improve enforcement without creating disproportionate coupling.

## 4. Canonicalization and validation

For Oteryn-owned and accepted Platform-native identities:

- complete 128 bits are preserved;
- RFC UUID variant and UUID version 7 are validated when values enter a boundary that claims an accepted UUIDv7 identity;
- nil/zero UUID is invalid;
- malformed or wrong-version values fail before they become an authoritative row relation;
- no truncation, hashing, signed-integer reinterpretation or endian swapping is allowed;
- no identity may be synthesized from display name, account integer, Canary numeric ID, table sequence or another semantic identity.

When a textual representation is required for diagnostics/configuration/fixture data, the canonical form is lowercase hyphenated UUID text. Text is an interchange/debug representation, not the game database storage choice and not the FND-02 wire representation.

## 5. Strong typing across Rust and persistence

The physical PostgreSQL scalar is common, but domain APIs must expose distinct strong types such as:

```text
AccountId
CharacterId
WorldId
ChannelId
NodeId
InstanceId
PartyId
GameSessionId
HandoffId
ItemInstanceId
```

A generic raw UUID parameter is prohibited in authoritative domain APIs where the semantic type is known.

Persistence adapters may convert strong types to/from PostgreSQL UUID at the repository boundary, but conversion must be explicit and type-directed. A value successfully parsed as UUID is not automatically valid as every identity type.

## 6. Scoped identities

FND-ID-01 already fixes:

```text
ChannelRef  = WorldId + ChannelId
InstanceRef = WorldId + InstanceId
PartyRef    = WorldId + PartyId
HandoffRef  = WorldId + HandoffId
```

### Decision

Durable storage preserves each component independently. It must not:

- concatenate the two UUIDs into a proprietary binary key;
- hash them into a replacement identity;
- drop `WorldId` merely because the UUID component is globally collision-resistant;
- reinterpret a globally unique UUID as globally scoped semantic authority.

Where a durable relation depends on scoped identity, the owning schema must carry/validate the scope explicitly. DUR-02 decides exact composite primary/foreign-key layout, but cannot erase the semantic scope defined above.

Cross-world equality of a scoped component alone never proves semantic equality or authorization.

## 7. Null and absence semantics

Nil UUID is never an absence sentinel.

Rules:

- required identity relation => non-null valid typed UUID;
- semantically optional relation => SQL `NULL` / typed Option-like absence;
- nil UUID => invalid input/data-integrity failure;
- empty string => invalid textual UUID, never translated to nil or NULL automatically.

This prevents the database and application from having two competing absence encodings.

## 8. Identity is not order, version or authority

UUIDv7 embeds time information useful for generation/locality properties, but it is not a semantic timeline.

The following remain separate values and must not be replaced/inferred by comparing identity UUIDs:

- `connection_generation`;
- runtime scope ownership generation;
- `CommandId`;
- `server_sequence`;
- state/domain revisions;
- optimistic row revision/version;
- transaction/operation ordering;
- wall-clock occurred/created timestamps;
- RuntimeExecutionOrdinal;
- lease/security/profile revisions or fences.

If durable business logic needs creation/commit/order time, it stores an explicit authoritative timestamp/revision/order value owned by that contract. `ORDER BY uuid` may be an implementation optimization only when result correctness does not depend on UUID chronology.

### 8.1 Lossless durable CommandId representation

FND-02 owns `CommandId` semantics and defines it as a monotonic non-zero `uint64` scoped by `GameSessionId`, with command identity `(GameSessionId, CommandId)`. DUR-01 may define only its physical persistence when a durable deduplication/idempotency/recovery relation needs it.

PostgreSQL `bigint` is signed and cannot represent the full legal `uint64` range. Restricting CommandId to `INT64_MAX`, applying an undocumented signed offset/xor transform, or storing it in floating point would silently change the accepted FND-02 contract.

Decision when CommandId is persisted:

```text
Rust/domain scalar: u64, as owned by FND-02
PostgreSQL scalar:  numeric(20,0)
valid range:        1 .. 18446744073709551615
semantic scope:     GameSessionId
logical identity:   (GameSessionId, CommandId)
```

Rules:

- zero is invalid because FND-02 requires non-zero post-admission command identity;
- preserve exact integer value and natural unsigned ordering across round-trip;
- never store CommandId as `double precision`, textual decimal as the canonical DB scalar, signed-remapped `bigint`, truncated integer or hash-only value;
- a numeric CommandId equal in two different GameSessions is not the same command;
- a repeated `(GameSessionId, CommandId)` is the same command identity and downstream idempotency semantics remain FND-02/DUR-02/DUR-03-owned;
- no global database sequence may replace the client/session command identity;
- exact composite key/index/check syntax belongs to DUR-02, but it must preserve the complete pair and range.

Required persistence evidence includes round-trip of boundary values `1`, `9223372036854775807`, `9223372036854775808` and `18446744073709551615`, rejection of `0`, negative values, fractional values and values above `uint64::MAX`, and proof that identical numeric CommandId values in different GameSessions do not collide semantically.

## 9. Minimum new durable-domain identity catalogue

### 9.1 Avoid a generic DurableEntityId

A single catch-all `EntityId` was considered and rejected. It would encourage type erasure between characters, items, houses, rewards, containers, events and future entities and would make persistence relations easier to connect incorrectly.

DUR-01 adds identities only when a later hard gate already requires one now.

### 9.2 ItemInstanceId

`DUR-03` requires a stable identity for one concrete durable item instance. ADR-0006 also requires item-integrity evidence to converge on this identity.

Decision:

```text
ItemInstanceId = game-domain strongly typed UUIDv7, full 128 bits
PostgreSQL physical representation = uuid
semantic scope = global item instance
owner/issuer = authoritative game item lifecycle boundary
reuse = never
nil = invalid
```

`ItemInstanceId` identifies one concrete item-instance lifecycle. It does not encode item type/template, quantity, owner, location, world, container, creation source or revision.

DUR-03 will decide exact split/merge/transform identity retention/retirement rules. DUR-01 only freezes that every live concrete item instance has a stable canonical identity and no two distinct item-instance lifecycles may intentionally share it.

### 9.3 Other candidate durable IDs

The following are deliberately **not** frozen by DUR-01 because their semantic owner is not required to unblock durable representation now:

- generic entity/row IDs;
- house/quest/achievement/content identifiers already owned by content/product contracts or future domain contracts;
- ledger-entry/reward-delivery identifiers whose exact lifecycle belongs to DUR-02/DUR-03;
- `EventId`, `OperationId`, `TransactionId`, `CorrelationId`, `CausationId`, `AnalyticsActorId` — ANL-01 owns their semantic catalogue, with DUR-02/DUR-03 consuming the result for durable outbox/audit;
- database migration IDs, row-lock IDs or surrogate sequence keys — implementation metadata, not automatically domain identity.

## 10. Internal foreign keys and ownership

Within `oteryn_game`, durable relations may use database foreign keys when:

- both sides are game-owned durable data under compatible migration authority;
- the relation represents a real semantic dependency;
- scope and deletion/retirement semantics are compatible with the owning domain contract.

DUR-02 owns the actual FK/deferrability/cascade choices.

Across `oteryn_platform` and `oteryn_game`:

- no cross-database foreign key is permitted as authority proof;
- `AccountId`, `WorldId`, `ChannelId` and other Platform-owned values are contract references stored/validated by the game as needed;
- existence in a cached/projected game row does not transfer Platform ownership;
- browser/client-supplied equality does not establish authority.

## 11. Surrogate keys

A local persistence surrogate may exist for a table only when an implementation contract proves a storage benefit. It never replaces the canonical domain identity outside that local relation.

Rules:

- no local bigint surrogate may leak into service/protocol/domain contracts as the canonical identity;
- no external relation may infer authority from a local row number;
- uniqueness and lifecycle rules of the canonical identity remain enforced independently;
- using a surrogate does not authorize dropping the canonical UUID column.

This matches Platform ADR 0028's distinction between local `identities.id` and canonical AccountId.

## 12. Legacy/import anti-corruption

Legacy Canary/Otheryn numeric identifiers are migration source identifiers only.

The stable logical identity key for one legacy source entity is:

```text
(source_system, source_namespace, source_entity_kind, legacy_identifier)
    -> native typed identity
```

`source_namespace` identifies the stable legacy identity namespace (for example one authoritative database/world/domain namespace), not an individual export or snapshot. `source_revision`, snapshot hash/version, import run and migration classification are provenance attached to observations/import attempts; they are **not** part of the stable mapping key unless a separately accepted migration contract proves that a revision actually denotes a different identity namespace.

Rules:

- first accepted import allocates or consumes one canonical native identity according to the owning native lifecycle;
- retry or later snapshot of the same proven source entity reuses the existing stable mapping even when source revision/import-run metadata changes;
- the same stable source key mapping to a different native identity is a conflict and fails closed;
- a source revision may update provenance or mutable imported state but cannot silently mint a second native identity for the same stable source key;
- two different semantic source entities may not silently collapse onto one native identity;
- collision/duplicate ambiguity never overwrites an existing native entity;
- legacy numeric IDs are not embedded into UUID bytes or used as native UUID entropy/authority;
- mapping provenance includes exact source revision/snapshot/classification and import-run evidence sufficient for deterministic re-import review;
- migration tools have no authority to mint Platform-owned AccountId/WorldId/ChannelId values; they consume exact Platform-issued mappings where those identities are required.

DUR-02/DUR-04 decide physical mapping-table/tooling layout. DUR-01 freezes the anti-corruption semantics.

## 13. Deletion, retirement and tombstones

Canonical durable identity is never reused after semantic retirement.

Physical row deletion does not authorize identity reuse.

When later contracts need replay protection, audit correlation, foreign-reference safety or import deduplication after deletion, they must retain enough tombstone/provenance evidence to preserve non-reuse and conflict detection. Exact tombstone schema/retention belongs to DUR-02/ANL/domain-specific privacy policy.

An anonymization requirement may remove identifying attributes while preserving a non-reusable technical identity where legally and operationally appropriate; privacy policy owns the exact retention decision.

## 14. Privacy and public-reference boundary

UUIDv7 contains approximate generation-time structure. Internal durable identity is therefore not automatically an appropriate public identifier.

Rules:

- `AccountId`, `GameSessionId`, `NodeId`, exact channel/instance placement and security/recovery identifiers are restricted by default;
- `CharacterId`, `ItemInstanceId` and other durable IDs are not exposed publicly merely because they are stable;
- public APIs/UI may use a separately owned opaque public reference/slug when exposure of internal identity enables correlation, enumeration or unwanted creation-time disclosure;
- a public reference is presentation/API identity only under its owning product contract; it must not become a second hidden authority over the durable entity;
- analytics datasets follow ADR-0006 pseudonymization rather than publishing operational IDs.

DUR-01 does not invent one universal `PublicId` type.

## 15. Indexing

Exact index layout belongs to DUR-02, but DUR-01 freezes representation constraints:

- indexes must operate on lossless UUID values or complete scoped component sets;
- no index may depend on truncated/hash-only identity as the sole authoritative equality key unless collision-free equivalence is mathematically/provably preserved by an accepted later contract;
- UUIDv7 ordering may be exploited for storage locality/performance only; correctness must use explicit semantic fields;
- scoped uniqueness must include scope where the domain contract requires it.

## 16. Representation evolution

Changing physical identity representation is a breaking durability concern even if the semantic name stays unchanged.

Any future change requires an explicit migration contract with:

1. old/new representation profiles;
2. lossless conversion proof;
3. expand-before-contract rollout where mixed versions exist;
4. uniqueness/non-reuse preservation;
5. foreign/reference reconciliation;
6. rollback that does not mint alternate identities;
7. exact consumer compatibility matrix;
8. negative fixtures for truncation, wrong type, wrong scope and stale/conflicting legacy mapping.

A migration may change storage shape; it may not reinterpret an existing identity as a different semantic entity.

## 17. Error/disposition mapping

DUR-01 representation failures map into the Foundation Error Vocabulary as follows, subject to later operation-specific codes:

| Condition | Foundation category | Minimum mutation outcome |
|---|---|---|
| malformed/nil/wrong UUID version for claimed typed identity | `INVALID_INPUT` | no authoritative relation created/changed |
| invalid/out-of-range persisted CommandId | `INVALID_INPUT` | no authoritative command identity relation created/changed |
| wrong semantic identity type at a typed boundary | `INVALID_INPUT` | no mutation |
| unsupported representation/schema profile | `UNSUPPORTED_REVISION` | no silent downgrade |
| duplicate/collision/native uniqueness conflict | `CONFLICT` | no overwrite/merge |
| legacy mapping points to conflicting native identity | `CONFLICT` | fail closed; preserve existing mapping/state |
| required external identity authority unavailable during an operation that needs fresh proof | `DEPENDENCY_UNAVAILABLE` | operation-specific fail-closed behavior; cached identity existence is not authorization |
| persisted value violates an invariant discovered internally | `INTERNAL_UNAVAILABLE` or later narrower integrity code | no compensating guess/re-key; evidence retained |

Diagnostics must not expose restricted IDs unnecessarily.

## 18. Required implementation evidence

A later implementation may claim DUR-01 conformance only with deterministic evidence covering at least:

### Scalar round trip

For every implemented UUID-backed durable identity:

```text
strong Rust type
 -> PostgreSQL uuid
 -> strong Rust type
```

must preserve all 128 bits exactly.

For persisted CommandId, `u64 -> numeric(20,0) -> u64` must preserve the exact full unsigned range and the `(GameSessionId, CommandId)` scope.

### Cross-language/cross-repository fixtures

For Platform-owned native UUID identities actually consumed by Oteryn-v2, exact fixtures must prove canonical UUID equivalence across producer and consumer. Canary numeric IDs and Platform local integer surrogates must be rejected by native-only fixtures.

### Negative typing

Tests must prove that APIs/repositories cannot accidentally substitute e.g. CharacterId for AccountId or ItemInstanceId for CharacterId merely because both are UUID values.

### Nil/version/canonicalization

Reject nil UUID and wrong-version UUID where the contract claims UUIDv7. Text fixtures must reject malformed/non-canonical forms where canonical form is required.

### Scoped relations

Prove wrong-World scoped references fail rather than matching by the component UUID alone.

### CommandId boundaries

Prove exact persistence of `1`, `INT64_MAX`, `INT64_MAX + 1`, and `UINT64_MAX`; reject zero, negative, fractional and greater-than-uint64 values; prove equal numeric CommandId in two distinct GameSessions does not collide.

### Legacy import

Prove:

- repeat import and later snapshots of the same stable source key are idempotent to one native mapping;
- changed source revision/provenance alone cannot allocate a second native identity;
- conflicting stable source mapping fails closed;
- duplicate/collision does not overwrite native state;
- Platform-owned identities are not minted by game migration tooling.

### Mixed-version migration

When representation changes are introduced, fixtures must demonstrate expand/backfill/validate/cutover/rollback behavior without identity drift.

Runtime/component/browser E2E is not required for this architecture gate, but database integration tests are mandatory for any later physical persistence implementation claiming conformance.

## 19. Failure-scenario integration

DUR-01 consumes relevant Foundation scenarios conceptually:

- `FS-STALE-GENERATION`: identity never substitutes for fence/generation;
- `FS-DUPLICATE-COMMAND`: persistence preserves exact `(GameSessionId, CommandId)` while duplicate effect/idempotency handling remains FND-02/DUR-02/03;
- `FS-DB-OUTBOX-BOUNDARY`: deferred to DUR-02/ANL-01 after identity representation is fixed;
- `FS-AUDIT-MUTATION-MISMATCH`: deferred to ANL-01/DUR-02/03;
- item-duplication prevention remains DUR-03 authority.

No DUR-01 rule permits a persistence adapter to repair an ambiguous identity by guessing, hashing, re-keying or choosing the newest-looking UUID.

## 20. Closed decisions

This analysis closes Issue #111's eight question groups:

1. PostgreSQL UUID representation: **native `uuid`, full 128 bits**.
2. Durable-domain catalogue: **add `ItemInstanceId`; reject generic catch-all identity; leave audit/event IDs to ANL-01**.
3. Identity/revision/fence/order and CommandId durability: **strict separation; UUIDv7 order is non-authoritative; persisted full-range FND-02 CommandId uses `numeric(20,0)` scoped by GameSessionId**.
4. Foreign/cross-boundary rules: **game-local typed relations allowed; no Platform/game cross-DB FKs; scope explicit**.
5. Legacy migration: **stable source-namespace mapping key, revision/snapshot as provenance, no numeric-to-native identity reinterpretation, conflicts fail closed**.
6. Privacy/public reference: **internal UUIDv7 not automatically public; product-owned opaque refs when needed**.
7. Evolution: **explicit lossless versioned migration; no silent representation change or re-key**.
8. Evidence: **UUID and CommandId round-trip/boundary tests, negative typing/scope, stable legacy mapping/revision-conflict and mixed-version fixtures required**.

No unresolved decision in this analysis prevents a final DUR-01 contract. Numeric persistence performance choices, table layout and transaction semantics remain intentionally downstream.