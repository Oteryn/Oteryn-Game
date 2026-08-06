# FND-ID-01 Owner-Accepted Identifier Baseline

- Status: Owner-accepted pre-contract baseline
- Date: 2026-08-06
- Decision owner: Oteryn project owner
- Gate: `FND-ID-01`
- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Applies to: future identifier, protocol, runtime, admission, persistence, analytics and content contracts in `blakinio/Oteryn-v2`

## Purpose

Record the identifier model explicitly accepted by the project owner before the complete `FND-ID-01` contract is drafted.

This document is canonical architecture input. It is not the completed `FND-ID-01` contract and does not authorize protocol, runtime, admission, persistence or schema implementation.

The ordered programme gate remains unchanged: the complete `FND-ID-01` package begins after the source-only `blakinio/otclient` historical marker for destination merge `78988f72a80cc904aa9176ae850c50d4efa0b0f0` is merged and verified.

## Accepted model

Oteryn uses four semantically distinct identity and ordering classes. A type may belong to only one class unless a later accepted contract explicitly defines a safe conversion boundary.

### Class 1 — Durable cross-boundary identity

A durable cross-boundary identity identifies one semantic entity across process, service, protocol, persistence, event and recovery boundaries.

Required properties:

- stable for the lifetime defined by the owning domain;
- immutable after assignment;
- never reused for a different semantic entity;
- opaque and free from mutable business meaning;
- independently validated at every trust boundary;
- safe to correlate across named authorized boundaries;
- does not depend on a process address, array index, database row position or current display name.

Candidate members include identities such as `AccountId`, `CharacterId`, `WorldId`, `GameSessionId`, `ItemInstanceId`, `EventId`, `OperationId` and `TransactionId`, subject to the complete `FND-ID-01` and `DUR-01` catalogues.

This baseline does not select UUID, ULID, integer, byte width, textual representation or database column type.

### Class 2 — Scoped identity

A scoped identity is unique and meaningful only inside an explicitly named owner scope.

Required properties:

- the owner scope is part of canonical comparison and validation;
- the scoped value must not be compared, serialized, cached or logged as globally unique unless wrapped with its scope;
- scope changes do not silently preserve identity unless a dedicated migration or lifecycle contract permits it;
- APIs and events must carry enough context to reject cross-world, cross-channel, cross-instance or cross-revision misuse.

Canonical examples include concepts equivalent to:

```text
WorldId + ChannelId
WorldId + InstanceId
ContentRevision + compact runtime content ID
Channel ownership generation + channel-local runtime entity ID
```

Whether a particular identifier is globally unique in representation does not remove its semantic scope. A globally collision-resistant value may still be invalid outside its owning world, channel, instance, package or revision.

### Class 3 — Runtime-local generational handle

A runtime-local handle addresses transient in-memory state owned by one runtime boundary.

Required properties:

- local to the named process, runtime, channel, instance, arena or allocation domain;
- generation-fenced so stale references fail deterministically after reuse;
- never treated as durable identity;
- never exposed as the sole identity in public protocol, persistence, durable audit or cross-process contracts;
- invalid after the owning runtime boundary ends, reloads or advances the handle generation;
- convertible to durable identity only through an explicit owner-controlled lookup where such identity exists.

Candidate uses include local creature slots, component/entity handles, task handles, pathfinding work references and internal subscription registrations.

A raw memory address, collection index or reusable integer without a generation is not an accepted runtime identity model.

### Class 4 — Ordering, revision and fencing value

Revisions, generations, sequence numbers, epochs and ticks describe order, version or authority. They are not identities of semantic entities.

Required properties:

- owned by an explicit scope;
- monotonic or otherwise ordered according to the owning contract;
- compared only inside compatible scope and lifecycle;
- never reused to impersonate an entity ID;
- wraparound, exhaustion, reset and persistence behavior must be explicit;
- stale values must be rejected where they protect authority or causality;
- equality means the same ordering/version state, not the same semantic entity.

Examples include:

- `session_generation`;
- channel ownership generation;
- state revision;
- command sequence;
- snapshot/delta baseline revision;
- protocol, ruleset, content, map, schema and server-build revisions;
- simulation tick or journal position.

A generation may fence an identity but does not replace that identity. For example, `CharacterId + session_generation` identifies the current authority claim; the generation alone does not identify the character or session.

## Cross-cutting invariants

### Identity is semantically opaque

Canonical identifiers must not encode mutable or authorization-relevant facts such as:

- display name;
- world name;
- profession, class or level;
- account tier;
- current channel placement;
- timestamp whose disclosure is not explicitly required;
- database shard or physical host location;
- security or moderation status.

Routing and indexing hints may exist only as separately validated metadata. They must not become proof of ownership, authorization or current placement.

### Labels are not identity

The following are not canonical identity by themselves:

- names;
- slugs;
- aliases;
- display numbers;
- legacy numeric IDs;
- compact bundle-local IDs;
- database-generated row order;
- client-side list positions.

They may be unique within a declared namespace or revision and may resolve to an identity through an authoritative registry. Rename, localization, aliasing or reuse policy must not silently change semantic identity.

### Representation does not define semantics

The same wire or storage representation may be used for several identifier classes only when distinct strong types and validators prevent accidental substitution.

Conversely, different encodings of the same semantic identity must preserve exact equality and canonicalization rules across Rust, Platform contracts, PostgreSQL, event schemas and test fixtures.

### No implicit scope inference

Scope must not be inferred solely from:

- the process currently handling a value;
- the active connection;
- a database connection or schema;
- a thread-local or global variable;
- the current world selected in UI;
- a mutable route cache.

Public and durable operations carry or derive scope through an authoritative, validated binding.

### Authority requires identity plus fence

Security-sensitive and durable mutations require both the relevant semantic identity and the current authority/fencing values where applicable.

Examples include:

```text
CharacterId + GameSessionId + session_generation
ChannelId + channel ownership generation
ItemInstanceId + item/state revision
CommandId + GameSessionId + sequencing context
TransactionId + idempotency and ownership context
```

A valid identity with a stale generation is not current authority.

### Client-generated values are claims, not proof

A client may originate identifiers or correlation values only where the owning contract explicitly permits it. The server validates namespace, ownership, uniqueness, replay and resource limits before accepting them.

Client possession of an identifier never proves authorization to observe or mutate the identified entity.

## Boundary between `FND-ID-01` and `DUR-01`

The complete `FND-ID-01` contract owns:

- semantic meaning and owner of each minimum cross-boundary identifier;
- identity class;
- scope and uniqueness domain;
- reuse and lifecycle rules;
- public, protocol, session and event visibility;
- canonical comparison and validation rules;
- minimum encoding constraints required to keep Platform, client and server compatible.

`DUR-01` later owns:

- exact durable database representation;
- PostgreSQL column and index strategy;
- migration from legacy identifiers;
- storage width and binary/text trade-offs beyond cross-boundary compatibility;
- partitioning and locality implications;
- durable foreign-reference and archival behavior;
- item/entity-specific persistence representations not needed to freeze the foundation vocabulary.

`DUR-01` may refine representation but may not redefine identity semantics accepted by `FND-ID-01`.

## Required application to later contracts

This baseline is mandatory input to:

- `FND-ID-01` — complete identifier catalogue and semantics;
- `FND-02` — protocol fields, command sequencing, snapshots and reconciliation;
- `FND-03` — runtime handles, ticks, state revisions and ownership generations;
- `FND-04` — Game Session, admission, lease and stale-writer fencing;
- `DUR-01` through `DUR-03` — durable identities, revisions, items and transactions;
- `DUR-04` — content keys, package/revision scope and compact runtime IDs;
- `ANL-01` — event, operation, transaction, correlation, causation and analytics identities;
- `QA-E2E-01` — exact identity, generation and revision evidence;
- world, channel, instance, house, social, economy and lifecycle contracts.

## Still unresolved for the complete `FND-ID-01`

The following remain open and must not be inferred from this baseline:

- the exact minimum identifier catalogue;
- which durable identities are globally unique versus semantically scoped;
- UUID, ULID, integer, random, time-ordered or mixed generation strategy;
- byte width and canonical binary/text encoding;
- endianness and canonical string formatting;
- null, zero, nil and sentinel-value policy;
- where identities are generated and how generator failure behaves;
- collision handling and exhaustion policy;
- public exposure and enumeration-resistance requirements;
- redaction, logging and privacy classification;
- correlation/causation identity trust rules;
- exact revision and generation widths and wraparound policy;
- legacy identifier mapping and migration;
- PostgreSQL representation, indexes and partitioning;
- serialization/IDL technology;
- Rust crate names or concrete type definitions.

## Rejected interpretations

### Use names or slugs as canonical identity

Rejected because names are mutable, user-visible, subject to localization and possible reuse.

### Treat every identifier as globally interchangeable

Rejected because world, channel, instance, package, revision and runtime scopes carry security and consistency meaning even when representations are globally collision-resistant.

### Persist runtime handles

Rejected because handle reuse and runtime lifecycle make them unsafe as durable or cross-process identity.

### Use revision or generation as entity identity

Rejected because an ordering/fencing value has meaning only relative to its owner and cannot identify the underlying semantic entity.

### Encode current business state into identifiers

Rejected because mutable semantics, routing, authorization and privacy must remain independently validated.

## Programme effect

- This owner-accepted baseline is canonical input to `FND-ID-01`.
- It does not change the current ordered next action.
- The source-only `blakinio/otclient` historical marker remains required before the complete `FND-ID-01` package begins.
- No implementation is authorized by this document.
