# DUR-01 — Durable Identifier Representation Contract

- Status: Candidate; canonical only when the owning DUR-01 delivery merges
- Date: 2026-08-10
- Gate: `DUR-01`
- Issue: #111
- Repository: `blakinio/Oteryn-v2`
- Trusted base: `main@adb0882a5ddbe42944fe955f5effb78fd5495422`
- Scope: durable identity representation only

## 1. Purpose

This contract freezes the durable physical representation rules required before `DUR-02` persistence and `DUR-03` item/economy transaction design may become canonical.

It consumes, and does not redefine, `FND-ID-01` identifier semantics and the accepted FND-02/FND-03/FND-04 authority model.

Canonical split:

```text
semantic identity owner/lifecycle     -> FND-ID-01 / owning domain contract
wire encoding and CommandId           -> FND-02
runtime handles/generations/order     -> FND-03
admission/session/lease authority     -> FND-04
durable physical identity form        -> DUR-01
persistence schema/transactions       -> DUR-02
item/currency mutation invariants     -> DUR-03
event/audit identity catalogue        -> ANL-01
```

## 2. Canonical UUID durability profile

For every accepted native identity whose semantic representation is UUIDv7, the durable Oteryn game representation is:

```text
semantic type: strongly typed UUIDv7
width:         full 128 bits
PostgreSQL:    native uuid
nil UUID:      invalid
reuse:         forbidden after semantic retirement
```

This applies to native persisted uses of:

- `AccountId`;
- `CharacterId`;
- `WorldId`;
- `ChannelId`;
- `NodeId` where durable operational/recovery evidence stores it;
- `InstanceId`;
- `PartyId`;
- `GameSessionId` where durable session/recovery evidence stores it;
- `HandoffId` where the accepted transition contract stores it;
- `ItemInstanceId` introduced by this contract.

Not every identifier above must have a dedicated table. DUR-02/domain contracts decide whether and where a durable row exists.

## 3. AccountId reconciliation

For native Oteryn integration, Platform ADR 0028 is authoritative:

```text
AccountId = Platform-issued strongly typed UUIDv7, full 128 bits
```

Therefore:

- Oteryn-v2 stores native AccountId references as PostgreSQL `uuid` when persistence requires them;
- Oteryn-v2 never mints AccountId;
- Platform-local integer `identities.id` is not native AccountId;
- `canary_account_id` is not native AccountId;
- no integer, email, name or Canary identifier may be converted/derived into AccountId by the game;
- equality of a stored AccountId is identity correlation only, not fresh authorization/ownership proof.

Any Platform rollout that has not yet materialized canonical AccountId storage/production issuance remains a producer implementation concern; the native Oteryn consumer contract does not downgrade to legacy IDs.

## 4. PostgreSQL representation

### 4.1 Required scalar type

Canonical UUID-backed durable identities use PostgreSQL native `uuid`.

The following are rejected as canonical storage substitutes:

- text/char UUID columns;
- `bytea` custom UUID encoding;
- two integer halves;
- truncated/hash-only values;
- extracted timestamp plus random suffix;
- legacy numeric IDs embedded into UUID bytes.

A compatibility/import table may store the legacy source identifier in its original bounded representation, but its native target identity remains the typed UUID defined here.

### 4.2 SQL domains

DUR-01 does not mandate PostgreSQL `DOMAIN` objects per semantic identity. Strong typing is mandatory at domain/application boundaries and in schema relation design. DUR-02 may add SQL domains only with explicit migration/tooling evidence.

### 4.3 Canonical textual form

Where an accepted boundary uses textual UUID representation for configuration, fixtures or diagnostics, the canonical textual form is lowercase hyphenated UUID text.

Text form is not the `oteryn_game` canonical storage choice and does not replace FND-02's accepted 16-byte gameplay wire form.

## 5. Validation rules

A boundary claiming one of the UUIDv7 identity types in this contract must reject:

- malformed UUID;
- nil/zero UUID;
- wrong UUID version where the contract claims UUIDv7;
- prohibited legacy/native identifier substitution;
- truncated or lossy representation;
- wrong semantic type where the boundary is typed;
- wrong scope for a scoped identity.

Validation never turns an invalid value into a new UUID automatically.

## 6. Strong semantic typing

The common PostgreSQL scalar does not weaken type safety.

Authoritative Rust/domain APIs must use distinct types, including at minimum:

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

A generic UUID parameter is prohibited when the semantic identity type is known.

Persistence adapters may perform explicit typed conversion to/from PostgreSQL `uuid`; they must not expose a generic conversion path that lets one semantic ID substitute for another without deliberate code.

## 7. Scoped identity persistence

Accepted semantic references remain:

```text
ChannelRef  = WorldId + ChannelId
InstanceRef = WorldId + InstanceId
PartyRef    = WorldId + PartyId
HandoffRef  = WorldId + HandoffId
```

Durable representation requirements:

- store/validate the scope explicitly where the durable relation depends on scoped identity;
- preserve both UUID values losslessly;
- do not hash/concatenate them into a replacement semantic ID;
- do not drop WorldId because the component UUID is globally collision-resistant;
- a component match in the wrong world is invalid.

Exact composite key/index/foreign-key syntax belongs to DUR-02.

## 8. Null and absence

There is one absence model:

```text
optional semantic relation -> NULL / typed absence
required semantic relation -> valid non-null typed identity
nil UUID                    -> invalid, never absence
```

Empty textual value is invalid and is not silently normalized to NULL or nil.

## 9. Identity versus order/revision/fence

No code or schema may infer current authority, chronology or causal order from UUIDv7 ordering.

Identity remains distinct from:

- connection/ownership generation;
- CharacterLease/session fences;
- `CommandId`;
- `server_sequence`;
- state/domain/content/ruleset/protocol revisions;
- optimistic row revision;
- operation/transaction order;
- RuntimeExecutionOrdinal;
- wall/monotonic time;
- lease/security/profile revisions.

When semantic ordering matters, the owning contract stores an explicit timestamp/revision/ordinal/fence.

UUID sorting may be a storage/performance optimization only when it cannot change correctness.

### 9.1 CommandId durable scalar profile

FND-02 remains the semantic authority: `CommandId` is a monotonic non-zero `uint64` scoped by `GameSessionId`; command identity is `(GameSessionId, CommandId)`.

When persistence requires CommandId for deduplication, idempotency, recovery or evidence, the canonical durable scalar is:

```text
Rust/domain:       u64
PostgreSQL:        numeric(20,0)
valid range:       1 .. 18446744073709551615
semantic scope:    GameSessionId
logical identity:  (GameSessionId, CommandId)
```

PostgreSQL signed `bigint` is rejected as the canonical representation because it cannot represent the full accepted FND-02 `uint64` range. Also rejected are floating point, canonical text storage, signed-offset/xor remapping, truncation and hash-only replacement.

Rules:

- zero, negative, fractional and above-`uint64` values are invalid;
- round-trip preserves the exact unsigned integer and ordering;
- equal numeric CommandId values in distinct GameSessions are distinct command identities;
- repeated `(GameSessionId, CommandId)` denotes the same command identity; effect/idempotency semantics remain FND-02/DUR-02/DUR-03-owned;
- a database sequence or surrogate row ID never replaces CommandId;
- DUR-02 owns exact composite constraint/index/check syntax but may not narrow the FND-02 legal range silently.

## 10. ItemInstanceId

DUR-01 introduces the minimum new durable-domain identity required by DUR-03 and Game Intelligence:

```text
ItemInstanceId
  class: durable game-domain identity
  semantic scope: one concrete item-instance lifecycle, globally unique
  owner: authoritative game item lifecycle boundary
  issuer: authoritative item lifecycle operation that creates a new concrete instance
  representation: strongly typed UUIDv7, full 128 bits
  PostgreSQL: uuid
  nil: invalid
  reuse: never
  default visibility: restricted/internal
```

`ItemInstanceId` does not encode:

- item/template/type ID;
- quantity;
- owner CharacterId;
- container/location;
- WorldId/ChannelId/InstanceId;
- source/sink reason;
- revision;
- creation time as business truth.

DUR-03 owns create/destroy/split/merge/transform identity-transition rules and conservation semantics. DUR-01 only requires stable identity for each live concrete item instance and prohibits intentional identity reuse/collapse.

## 11. Deliberately deferred identities

DUR-01 does not create a generic `EntityId` or `RowId` domain identity.

The following remain owned elsewhere until their lifecycle is defined:

- `EventId`;
- `OperationId`;
- `TransactionId`;
- `CorrelationId`;
- `CausationId`;
- `AnalyticsActorId`;
- reward/ledger/outbox record IDs;
- content/quest/house/achievement identifiers not already frozen by their owning domain contract;
- local database migration/surrogate keys.

ANL-01 must freeze the event/audit identity catalogue before DUR-02/DUR-03 finalize atomic audit/outbox evidence.

## 12. Foreign-key and ownership boundary

### 12.1 Inside oteryn_game

Game-owned durable data may use database foreign keys when the relation is semantically valid and both sides share compatible game migration authority.

DUR-02 owns exact FK, deferrability, cascade and constraint layout.

### 12.2 Platform versus game

Cross-database foreign keys between `oteryn_platform` and `oteryn_game` are prohibited.

Platform-owned identities may be stored in game-owned rows as contract references, but:

- storage does not transfer ownership;
- cached existence does not prove current authorization;
- browser/client input does not prove account/character/world/channel authority;
- Platform/game lifecycle changes require explicit versioned orchestration/projection/API semantics.

## 13. Local surrogate keys

A table may later use a local persistence surrogate only if DUR-02/domain implementation proves a reason.

A surrogate:

- never replaces the canonical domain identity outside its local persistence relation;
- never becomes a protocol/service/public identity by convenience;
- does not authorize dropping canonical identity uniqueness/non-reuse rules;
- cannot be used to derive/mint canonical UUID identity.

Platform `identities.id` versus canonical AccountId is the reference example of this separation.

## 14. Legacy/import anti-corruption contract

Legacy identifiers are source provenance, not native identity.

Every accepted import that needs identity continuity uses a stable mapping conceptually equivalent to:

```text
(source_system,
 source_namespace,
 source_entity_kind,
 legacy_identifier)
    -> native_typed_identity
```

`source_namespace` identifies the stable identity namespace in which the legacy identifier is meaningful. Export/snapshot revision, source commit/hash, migration classification and import-run identity are provenance fields and do **not** alter the mapping key merely because a later snapshot is imported. A separately accepted migration contract may define a different namespace only when it proves that the source system itself reused/changed identifier semantics across namespaces.

Required invariants:

1. Retry or later snapshot of one proven stable source entity resolves to the same mapping even when revision/import-run provenance changes.
2. The same stable source key cannot resolve to two native identities.
3. A changed source revision may update provenance or mutable imported state but cannot silently mint another native identity for the same stable source key.
4. Two distinct semantic source entities cannot silently collapse into one native identity.
5. Collision or ambiguity fails closed; existing native entity is never overwritten.
6. Legacy numbers are not encoded into UUID bytes as native identity.
7. Import tooling cannot mint Platform-owned AccountId/WorldId/ChannelId; it consumes Platform-authorized native mappings.
8. Mapping provenance preserves exact source revision/snapshot/classification/import-run evidence for deterministic audit and re-import.
9. A failed/partial import cannot leave a second alternative canonical identity for the same proven stable source entity.

DUR-02/DUR-04 own physical mapping schema/tooling and transaction mechanics.

## 15. Retirement, deletion and reuse

A canonical durable identity is never reused for another semantic entity after retirement/deletion.

Physical row deletion does not free an identity for reuse.

Where replay protection, foreign-reference integrity, audit or migration idempotency requires evidence after deletion, later contracts retain a tombstone/provenance form sufficient to enforce non-reuse/conflict detection.

Exact retention/anonymization belongs to DUR-02/ANL/privacy/domain contracts.

## 16. Privacy/public identity

UUIDv7 includes approximate generation-time structure. Internal durable IDs are therefore not automatically safe/product-appropriate public identifiers.

Default restrictions:

- AccountId, GameSessionId, NodeId and session/placement/security IDs are not public;
- ItemInstanceId is restricted/internal by default;
- CharacterId/public profile exposure requires an explicit product/public-data contract;
- scoped placement IDs are disclosed only under visibility rules;
- analytics uses pseudonymous identity under ADR-0006/ANL-01 rather than exposing operational IDs.

When a public API/UI requires a stable identifier but internal UUID exposure creates enumeration/correlation/time-leakage risk, the owning product contract may define a separate opaque public reference/slug.

DUR-01 does not create a universal PublicId namespace and a public reference never becomes hidden mutation authority.

## 17. Indexing constraints

DUR-02 chooses exact index definitions, but it must preserve these rules:

- authoritative equality uses the full identity value;
- scoped identity uniqueness/equality includes scope where required;
- truncated/hash-only values cannot be the sole authoritative equality key unless a later proof establishes collision-free equivalence;
- UUIDv7 order may be used for physical locality/performance only, never semantic correctness;
- queries needing business chronology use explicit semantic time/order columns.

For persisted CommandId, any index/constraint participating in command identity must preserve the complete `GameSessionId + CommandId` pair; a globally unique CommandId index is semantically wrong.

## 18. Representation migration/versioning

A future change to durable identity representation is a compatibility/migration event, not an implementation refactor.

The owning migration contract must provide:

- explicit old/new representation profiles;
- lossless full-value conversion;
- expand/backfill/validate/cutover/contract phases where mixed versions exist;
- uniqueness/non-reuse preservation;
- reference/FK reconciliation;
- no minting of alternate identity during retry/rollback;
- exact consumer compatibility and rollback order;
- deterministic negative fixtures for truncation, wrong type, wrong scope, CommandId range loss and conflicting legacy mappings.

The semantic identity remains the same across a representation migration unless a separately accepted domain decision explicitly creates a new entity.

## 19. Failure contract

Minimum stable disposition:

| Condition | Foundation category | Required effect |
|---|---|---|
| malformed/nil/wrong-version UUID | `INVALID_INPUT` | no authoritative relation/mutation |
| invalid/out-of-range persisted CommandId | `INVALID_INPUT` | no authoritative command identity relation/mutation |
| wrong semantic identity type | `INVALID_INPUT` | no mutation |
| unsupported durable representation/profile revision | `UNSUPPORTED_REVISION` | no silent downgrade |
| native uniqueness/collision conflict | `CONFLICT` | no overwrite/collapse |
| conflicting legacy mapping | `CONFLICT` | preserve existing canonical state; fail closed |
| required external identity authority unavailable when fresh proof is needed | `DEPENDENCY_UNAVAILABLE` | operation-specific fail closed; cached identity is not authorization |
| internal persisted identity invariant violation | `INTERNAL_UNAVAILABLE` or later narrower integrity code | no guessed repair/re-key; produce bounded evidence |

Client/public errors may redact restricted identifiers. Correlation evidence must not contain credentials.

## 20. Required implementation evidence

Any future persistence implementation claiming DUR-01 conformance must prove at least:

### 20.1 UUID 128-bit round trip

For each implemented UUID-backed identity:

```text
strong domain type -> PostgreSQL uuid -> strong domain type
```

preserves all 128 bits exactly.

### 20.2 CommandId full-range round trip

For persisted CommandId:

```text
u64 -> PostgreSQL numeric(20,0) -> u64
```

must preserve exact values and scope. Fixtures include `1`, `9223372036854775807`, `9223372036854775808` and `18446744073709551615`; reject `0`, negative, fractional and above-`uint64` values; prove same numeric CommandId in different GameSessions remains distinct.

### 20.3 Producer/consumer UUID fixtures

For Platform-owned native IDs actually consumed by game persistence, exact fixtures prove producer/consumer equivalence and reject Platform local integer/Canary identifiers on native boundaries.

### 20.4 Type-confusion negatives

Tests prove semantically different UUID types cannot be interchanged through authoritative repository/domain APIs without explicit invalid conversion.

### 20.5 Nil/version/canonical text

Reject nil, wrong UUID version and malformed/non-canonical text where textual profile applies.

### 20.6 Scoped identity negatives

Wrong-world scoped references fail even when a component UUID is well formed.

### 20.7 Legacy import

Prove:

- repeated import and later snapshots of the same stable source key resolve to one native identity;
- source revision/provenance changes alone cannot allocate a second native identity;
- conflicting stable source mapping fails closed;
- collision/ambiguity does not overwrite/collapse native state;
- Platform-owned identities are not minted by game migration tooling.

### 20.8 Migration compatibility

Any later representation change proves mixed-version expand/backfill/validate/cutover/rollback without identity drift or CommandId range loss.

Database integration tests become mandatory when a physical schema implementation is authorized. Runtime/component/browser E2E remains `NOT_APPLICABLE` to this architecture-only delivery.

## 21. Downstream gates

After DUR-01 acceptance/lifecycle closeout:

- `DUR-02` may define PostgreSQL schema/migrations/transactions using this representation;
- `DUR-03` may consume `ItemInstanceId` and define anti-duplication/conservation semantics;
- `ANL-01` must still define event/audit identities and envelope before DUR-02/DUR-03 finalize transactional audit/outbox evidence;
- `DUR-04` remains independent content/world/scripting work;
- no authoritative durable gameplay mutation is enabled merely by DUR-01 acceptance.

## 22. Non-goals

This contract does not authorize:

- database provisioning or migration SQL;
- concrete table/schema layout;
- ORM/query layer implementation;
- transaction isolation/locking/retry/outbox/checkpoint/RPO/RTO decisions;
- item movement/split/merge/market/reward/currency transaction implementation;
- protocol/runtime/admission code changes;
- Platform repository/database changes;
- production deployment or traffic.

## 23. Acceptance rule

DUR-01 is accepted only when:

- the bounded analysis and this contract are internally consistent with current FND-ID-01/ADR-0004/ADR-0006/FND-02/FND-03/FND-04 and Platform ADR 0028/0029;
- exact-head repository governance/CI pass;
- an architecture/security/data-integrity review finds zero material issues;
- zero unresolved material review threads remain;
- the accepted head is squash-merged unchanged;
- a separate lifecycle closeout archives/releases ownership and closes Issue #111.
