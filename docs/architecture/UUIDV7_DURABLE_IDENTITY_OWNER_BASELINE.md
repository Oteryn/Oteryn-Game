# UUIDv7 Durable Identity Owner-Accepted Baseline

- Status: Owner-accepted pre-contract baseline
- Date: 2026-08-06
- Decision owner: Oteryn project owner
- Gate: `FND-ID-01`
- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Applies to: future identifier, protocol, runtime, admission, persistence, analytics, content and observability contracts in `blakinio/Oteryn-v2`

## Purpose

Record the project owner's accepted technical direction for durable identifiers after the semantic identity classes, world/channel/instance/party scopes and runtime ownership rules were accepted.

This document is canonical architecture input. It does not authorize runtime, protocol, database, schema, migration or client implementation.

## Supersession rule

This document resolves and supersedes only the following representation questions previously left open by `FND-ID-01_OWNER_ACCEPTED_BASELINE.md`:

- the default UUID/ULID/integer strategy for Oteryn-owned durable cross-boundary identities;
- the separation between durable identity, runtime handles, session handles, content identifiers and ordering/fencing values;
- the default strong-type, nil/sentinel, collision and generator-ownership rules;
- the high-level database, wire, privacy and capacity constraints that later contracts must preserve.

All previously accepted semantic scopes and ownership rules remain unchanged. In particular:

- canonical channel identity remains `WorldId + ChannelId`;
- canonical instance identity remains `WorldId + InstanceId`;
- canonical party identity remains `WorldId + PartyId`;
- a globally unique physical value does not erase its semantic scope;
- identity never grants authority without the required session, ownership, revision or fencing context;
- ADR-0003 remains authoritative for Platform Identity, Game Gateway, World Registry and admission boundaries.

Where an older unresolved-item list conflicts with this accepted baseline, this later owner decision takes precedence.

## Accepted primary rule

Every semantic entity for which Oteryn-v2 is the accepted identity authority and which can cross a process, service, GameNode, protocol, persistence, durable-event, audit, migration, backup or recovery boundary uses a strongly typed UUIDv7 as its canonical durable identity.

A cross-repository or externally owned identity adopts UUIDv7 only through its authoritative contract or an explicitly accepted coordinated migration. Oteryn-v2 does not silently re-key identities owned by Platform or another accepted authority.

The rule is based on lifecycle, authority and boundary crossing, not on whether the entity is currently large, small, common or rare.

A durable identity is:

- stable for its defined lifetime;
- immutable after assignment;
- never reused for another semantic entity;
- generated without a central global sequence service;
- compared only through its strong semantic type and required scope;
- independent from current process, memory address, database row position, GameNode, channel placement, display name and mutable business state.

## Candidate catalogue direction

Subject to the complete ownership and lifecycle catalogue in `FND-ID-01`, UUIDv7 is the accepted representation direction for durable identities such as:

- `WorldId` and `ChannelId` when adopted by their authoritative World Registry/topology contract;
- `InstanceId`;
- `ZoneId` when the zone is a durable cross-boundary entity rather than a bundle-local content region;
- `GameNodeId`;
- `CharacterId`;
- `PartyId`;
- `GuildId`;
- `HouseId`;
- `GameSessionId`, `AdmissionId` and `CharacterLeaseId` when accepted by the coordinated `FND-04` contract;
- `TransferId` and `HandoffId`;
- `ItemInstanceId` and `ContainerInstanceId`;
- `TradeId`, `MarketOfferId` and `MailId`;
- `QuestRunId`, `BossAttemptId` and `LockoutId`;
- `RewardGrantId` and `LootSettlementId`;
- `TransactionId`, `OperationId`, `EventId`, `SnapshotId` and `RecoveryCaseId`;
- durable social relationship, invitation and Party Finder entry identities where a separate semantic entity exists.

This list establishes a representation direction but does not assign ownership by itself. The complete `FND-ID-01` catalogue must prove that each candidate represents a real semantic lifecycle and name exactly one authoritative owner or coordinated issuer.

## Externally owned and coordinated identities

Oteryn does not create a competing identifier for an entity owned by another authoritative system.

In particular:

- `AccountId` remains owned by Oteryn Platform Identity under ADR-0003;
- `WorldId` and authoritative channel-route policy remain under the accepted Platform World Registry boundary until a coordinated contract says otherwise;
- `GameSessionId`, admission identifiers and lease identifiers require the cross-repository ownership decision in `FND-04` before their generator is assigned;
- identifiers already owned by Platform, World Registry or another accepted authority preserve that authority's canonical representation until an explicitly coordinated migration is accepted;
- Oteryn wraps external identifiers in distinct strong types and never silently converts, rekeys or aliases them as Oteryn-owned UUIDv7 values;
- new jointly defined cross-repository identities should use UUIDv7 when compatibility, issuer ownership and migration contracts permit it.

A mapping record may bridge a legacy or external identifier to an Oteryn UUIDv7 only when the mapping has an explicit owner, uniqueness rule, migration lifecycle and audit trail.

## Strong semantic types

Raw UUID values are not passed through domain APIs as interchangeable values.

The future Rust contracts must use separate strong types conceptually equivalent to:

```text
WorldId(UUIDv7)
ChannelId(UUIDv7)
InstanceId(UUIDv7)
CharacterId(UUIDv7)
ItemInstanceId(UUIDv7)
TransactionId(UUIDv7)
```

The examples describe the target representation where the authoritative cross-repository contract has adopted UUIDv7. They do not override current external ownership or migration requirements.

The common physical representation does not permit substitution between semantic types.

Cross-boundary references include their required scope, for example:

```text
ChannelRef  = WorldId + ChannelId
InstanceRef = WorldId + InstanceId
PartyRef    = WorldId + PartyId
```

The exact Rust crate, wrapper implementation, serialization traits and compile-time validation remain owned by the complete contracts and implementation tasks.

## UUIDv7 is identity, not authorization

UUIDv7 is never treated as a bearer secret, capability or proof of ownership.

Examples:

- `GameSessionId` identifies a session; a separately validated admission or continuation credential authorizes use;
- `CharacterId` identifies a character; current lease, session generation and ownership fencing authorize mutation;
- `ItemInstanceId` identifies an item; current inventory ownership and state revision authorize transfer;
- `InstanceId` identifies an instance; admission state and instance ownership generation authorize participation or mutation.

Possession or knowledge of an identifier grants no observation or mutation right.

## Runtime-local handles

UUIDv7 is not the primary addressing mechanism inside the hot simulation loop.

Transient in-memory entities use runtime-local generational handles with conceptually separate slot/index and generation components.

Candidate uses include:

- player and creature runtime entities;
- ECS/entity-store entries;
- projectiles and short-lived effects;
- timers and scheduled tasks;
- combat contexts;
- pathfinding work and temporary nodes;
- visibility-set entries;
- component and subscription slots.

Accepted constraints:

- a handle is valid only inside its named runtime/allocation domain;
- stale handles fail after slot reuse because generation is checked;
- handles are not persisted as durable identity;
- handles do not cross process or GameNode boundaries as sole identity;
- a durable entity may simultaneously have UUIDv7 identity and a local runtime handle;
- exact handle width and index/generation bit allocation remain performance decisions for `FND-03` and benchmark evidence.

## Session-local protocol handles

Frequent gameplay traffic does not repeatedly transmit durable UUIDv7 values for every entity reference.

At session or snapshot establishment, the authoritative server may map durable or runtime identities to compact session-local handles. Movement, visibility, combat and frequent delta messages then use those handles inside the validated session epoch.

Accepted constraints:

- session handles are scoped to one negotiated session/context and cannot be reused as durable identity;
- remapping, invalidation, snapshot reset and stale-handle behavior are explicit;
- reconnect and handoff either preserve a safely fenced mapping or establish a new mapping through an authoritative snapshot;
- cross-service, persistence, audit and recovery records retain canonical durable identities;
- exact handle width, binary encoding and reuse policy remain owned by `FND-02` and `FND-04`.

## Ordering, revision and fencing values

UUIDv7 does not replace values whose purpose is strict order, version, generation or authority fencing.

The default representation for the following class is an explicitly scoped unsigned 64-bit value unless a later accepted contract proves another width is necessary:

- simulation ticks;
- command and message sequences;
- state and snapshot revisions;
- session generations;
- channel and instance ownership generations;
- fencing tokens and epochs;
- journal positions and event offsets;
- content, map, ruleset, schema and build revisions where a numeric revision is appropriate.

Accepted constraints:

- no silent wraparound;
- exhaustion, reset and epoch transitions fail closed or follow an explicit rollover contract;
- a sequence or generation never substitutes for semantic entity identity;
- UUIDv7 timestamp ordering is not used to decide gameplay causality, writer authority or exact command order.

## Content and map identifiers

UUIDv7 is not assigned automatically to every static content definition, map tile or coordinate.

Static/revisioned content uses a separate model such as:

```text
stable content key + ContentRevision + compact bundle/runtime ID
```

Candidate definitions include spells, monsters, item definitions, quests, activities, map templates and ruleset data.

Accepted constraints:

- a concrete item instance may have `ItemInstanceId` UUIDv7 while its item definition uses a content key/revision;
- frequently accessed content uses compact runtime identifiers resolved inside one validated content revision;
- tiles, coordinates, chunks, sectors and pathfinding nodes are not each durable UUID entities;
- positions remain scoped by `ChannelSpace` or `InstanceSpace` plus coordinates;
- durable package/release identities may use UUIDv7 when they represent independent cross-boundary entities, but internal bundle members do not require it.

The complete content identifier and revision contract remains owned by `DUR-04` and related content architecture work.

## Database baseline

The canonical durable identity is UUIDv7 regardless of physical indexing optimizations once its authoritative contract has adopted UUIDv7.

Accepted database constraints for later `DUR-01` work:

- PostgreSQL uses its native `uuid` representation for canonical UUID identities rather than `varchar(36)` or mutable display strings;
- externally owned non-UUID identifiers retain the representation required by their authoritative contract or an explicit mapping/migration layer;
- UUID text is an interchange/debug representation, not the preferred internal storage form;
- auxiliary local surrogate keys, partition keys or ordering columns may be introduced only as physical optimizations and never replace canonical semantic identity at system boundaries;
- all uniqueness, foreign-reference, index, WAL, replication, backup and archival costs must be measured on representative datasets;
- large event and item tables may combine UUIDv7 identity with separately scoped sequence/time/partition columns;
- exact primary-key, clustering, partitioning and index choices remain owned by `DUR-01` through `DUR-03`.

## Wire and serialization baseline

Cross-boundary protocols preserve all 128 bits of an adopted UUIDv7 identity.

Accepted constraints:

- binary protocols prefer a canonical 16-byte representation rather than a 36-character textual UUID;
- endianness, canonical byte order, IDL and textual formatting are frozen by `FND-02`, not inferred by implementations;
- frequent gameplay deltas use compact session handles where the session context already establishes identity;
- logs and operator tooling may render a canonical textual form, subject to privacy and redaction policy;
- no lossy truncation, hashing or implicit conversion is permitted as identity.

## Generator ownership

No central global UUID service is introduced.

The complete identifier catalogue assigns exactly one logical generator or coordinated issuer to every durable identity. Subject to that catalogue:

- Platform World Registry or controlled world provisioning generates `WorldId` if the coordinated contract adopts UUIDv7;
- authoritative topology control generates `ChannelId` if that contract adopts UUIDv7;
- the activity/instance allocator generates `InstanceId`;
- the party authority generates `PartyId`;
- the character domain generates `CharacterId`;
- the coordinated Game Session authority generates `GameSessionId` after `FND-04` assigns ownership;
- the authoritative item/inventory owner generates `ItemInstanceId`;
- the transaction owner generates `TransactionId`;
- the authoritative event producer generates `EventId`;
- persistence/recovery components generate snapshot and recovery identities they own.

No implementation may infer generator ownership merely from the service that currently receives or stores an identifier.

## Clock, collision, nil and reuse policy

Accepted defaults:

- identifiers are never reused for a different semantic entity;
- nil/zero UUID is not a valid entity identity or magic sentinel;
- absence is represented explicitly by an optional/nullable field when the domain permits absence;
- durable stores enforce uniqueness at authoritative write boundaries;
- a detected collision or duplicate insertion never overwrites an existing entity;
- generation is retried or the operation fails explicitly with audit evidence;
- imported identities are validated against namespace, ownership and collision rules;
- deleted entities retain tombstone/audit semantics where required to prevent unsafe reuse;
- UUIDv7 time ordering is an indexing/locality property, not an authority or causality guarantee;
- clock regression, equal timestamps and generator restart must not produce duplicate identities or silently weaken uniqueness;
- exact clock-regression and per-generator monotonicity behavior is frozen by the implementation contract and conformance tests.

## Privacy and public exposure

UUIDv7 contains time-ordering information and may reveal an approximate creation time. Internal durable IDs therefore are not automatically public IDs.

Accepted constraints:

- public APIs, clients and social/search surfaces expose only identifiers required by the product contract;
- security-sensitive or privacy-sensitive resources may use a separate opaque public reference, signed capability or short-lived token;
- public references never derive authority solely from UUID possession;
- alternate-character, social, anti-cheat, moderation, recovery and internal topology identities follow explicit redaction and visibility policy;
- logs preserve operational correlation while applying access control, retention and redaction requirements;
- exact public-ID and token strategy remains owned by security, privacy, Platform and protocol contracts.

## Capacity and performance principle

UUIDv7 is chosen for durable identity, not for every hot-path reference.

The architecture must preserve player capacity by ensuring that:

- hot simulation structures use compact generational handles;
- frequent network messages use session-local handles;
- content and map hot paths use compact revision-scoped identifiers;
- UUID generation does not occur per tick, per tile, per pathfinding node or per transient visual effect unless the object genuinely becomes a durable cross-boundary entity;
- durable UUID lookups are moved off hot loops or cached/mapped under authoritative ownership where safe;
- memory layout, cache behavior, hashing, serialization, database indexes and network bandwidth are measured with representative workloads.

Later capacity work must compare at minimum:

1. UUIDv7 used directly in hot runtime structures;
2. compact numeric identities without a coherent boundary model;
3. the accepted hybrid: durable UUIDv7 plus runtime/session/content handles and `u64` ordering values.

The accepted hybrid is the architectural default. Benchmarks may optimize physical representation and indexing but may not remove an accepted canonical durable UUIDv7 identity without a new architecture decision.

## Deliberately unresolved

The following remain open for complete contracts and implementation evidence:

- the final exhaustive durable identifier catalogue and owner/issuer matrix;
- exact lifecycle and retention of each identity;
- compatibility and migration for current Platform-owned or legacy identifiers;
- whether each coordinated `WorldId`, `ChannelId`, `GameSessionId`, admission and lease identifier adopts UUIDv7 immediately or through migration;
- exact UUIDv7 library and conformance tests;
- canonical endianness and byte/string formatting;
- exact clock-regression and monotonic-generation algorithm;
- exact runtime-handle and session-handle widths and reuse rules;
- exact PostgreSQL primary keys, indexes, partitions and clustering;
- exact protocol/IDL serialization;
- public opaque-reference and token formats;
- `CommandId` design, including client-clock trust, replay windows and relation to session command sequence;
- analytics correlation/causation trust rules;
- quantitative CPU, RAM, cache, database, WAL, backup and network acceptance thresholds;
- concrete Rust crate names and implementation types.

## Rejected interpretations

### UUIDv7 for every integer-like value

Rejected because ticks, generations, revisions, sequences, indexes and fencing values have ordered numeric semantics rather than entity identity.

### UUIDv7 as the primary hot-loop handle

Rejected because runtime-local generational handles provide better cache locality, stale-reference detection and capacity without weakening durable identity.

### UUIDv7 in every frequent gameplay packet

Rejected because session-local handles preserve bandwidth and serialization efficiency after identity has been authoritatively established.

### UUIDv7 as an authorization token

Rejected because an identifier is not a secret, capability or ownership proof.

### Central UUID generator service

Rejected because UUIDv7 permits domain-owned distributed generation and a central service would add latency and a failure bottleneck.

### Raw interchangeable UUID values in domain APIs

Rejected because strong semantic types and scoped references are required to prevent accidental substitution.

### Replace external authoritative IDs silently

Rejected because Platform and other accepted authorities retain ownership until a coordinated migration contract is accepted.

### Assign generator ownership from repository location

Rejected because ADR-0003 and later cross-repository contracts define authority; storing or consuming an identifier does not make Oteryn-v2 its issuer.

### Store UUID as `varchar(36)` by default

Rejected because PostgreSQL native `uuid` is the canonical physical baseline for adopted UUID identities; textual rendering remains an interchange and operator concern.

## Programme effect

- The durable identity representation question is accepted for Oteryn-owned identities: durable cross-boundary identities use strongly typed UUIDv7.
- Cross-repository and Platform-owned identities adopt UUIDv7 only through their authoritative coordinated contract or migration.
- UUIDv7 does not erase `WorldId` scope for channels, instances or parties.
- Runtime entities use local generational handles.
- Frequent gameplay protocol references use session-local handles.
- Ordering, revisions, generations and fencing default to scoped `u64` values.
- Static content and map hot paths use revision-scoped compact identifiers rather than UUID per definition access or tile.
- PostgreSQL native `uuid` and canonical 16-byte wire representation are required baselines for adopted UUID identities, while exact schema and codecs remain future contract work.
- UUID identifiers are not bearer secrets and are not automatically public references.
- Future capacity tests must measure the accepted hybrid rather than treating UUIDv7-in-every-hot-path as the target design.
- No runtime, protocol, persistence, schema, migration or client implementation is authorized by this document.
