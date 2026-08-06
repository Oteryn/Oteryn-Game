# FND-ID-01 Owner-Accepted Identifier Baseline

- Status: Owner-accepted pre-contract baseline
- Date: 2026-08-06
- Decision owner: Oteryn project owner
- Gate: `FND-ID-01`
- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Applies to: future identifier, protocol, runtime, admission, persistence, analytics, party, instance and content contracts in `blakinio/Oteryn-v2`

## Purpose

Record the identifier model explicitly accepted by the project owner before the complete `FND-ID-01` contract is drafted.

This document is canonical architecture input. It is not the completed `FND-ID-01` contract and does not authorize protocol, runtime, admission, persistence or schema implementation.

The ordered programme gate remains unchanged: the complete `FND-ID-01` package begins after the source-only `blakinio/otclient` historical marker for destination merge `78988f72a80cc904aa9176ae850c50d4efa0b0f0` is merged and verified.

Detailed instance runtime, map-template and admission consequences are recorded in `INSTANCE_SCOPE_AND_RUNTIME_OWNER_BASELINE.md` and are mandatory input to the complete foundation contracts.

## Accepted identity model

Oteryn uses four semantically distinct identity and ordering classes. A type belongs to one class unless a later accepted contract explicitly defines a safe conversion boundary.

## Class 1 — Durable cross-boundary identity

A durable cross-boundary identity identifies one semantic entity across process, service, protocol, persistence, event and recovery boundaries.

Required properties:

- stable for the lifetime defined by the owning domain;
- immutable after assignment;
- never reused for a different semantic entity;
- opaque and free from mutable business meaning;
- independently validated at every trust boundary;
- safe to correlate only across named authorized boundaries;
- independent from process address, array index, database row position and current display name.

Candidate members include `AccountId`, `CharacterId`, `WorldId`, `GameSessionId`, `ItemInstanceId`, `EventId`, `OperationId` and `TransactionId`, subject to the complete `FND-ID-01` and `DUR-01` catalogues.

This baseline does not select UUID, ULID, integer, byte width, textual representation or database column type.

## Class 2 — Scoped identity

A scoped identity is unique and meaningful only inside an explicitly named owner scope.

Required properties:

- the owner scope is part of canonical comparison and validation;
- the value is not compared, serialized, cached or logged as globally unique unless wrapped with its scope;
- scope changes do not silently preserve identity unless an explicit lifecycle or migration contract permits them;
- APIs and events carry enough context to reject cross-world, cross-channel, cross-instance, cross-party and cross-revision misuse.

Canonical accepted examples include:

```text
WorldId + ChannelId
WorldId + InstanceId
WorldId + PartyId
ContentRevision + compact runtime content ID
Channel ownership generation + channel-local runtime entity ID
```

A globally collision-resistant technical representation does not remove semantic scope.

## Accepted world identity

- `WorldId` is a globally unique durable cross-boundary identity of one logical world.
- A logical world owns its character population, community, ruleset, economy and channel/instance topology.
- World names and display labels are not identity.
- Cross-world gameplay-value transfer, shared simulation and instances are forbidden unless a later explicit contract establishes a separate product feature.

## Accepted channel identity

The canonical semantic identity of a channel is:

```text
WorldId + ChannelId
```

Accepted invariants:

- every gameplay channel belongs to exactly one logical world at a time;
- `ChannelId` is never interpreted independently from its owning `WorldId` at public, durable or cross-process boundaries;
- equal `ChannelId` values under different worlds do not identify the same semantic channel;
- protocol messages, events, persistence records, logs and caches carry or authoritatively derive the world binding wherever channel identity crosses a boundary;
- moving a channel identifier between worlds cannot silently preserve semantic identity;
- display labels such as `Optional PvP 1` or `Channel 2` remain labels and cannot replace the scoped identity;
- channel placement is not public presence information by default.

This freezes semantic scope only, not representation, generation, width, database key shape or wire encoding.

## Accepted channel and instance topology

Channels remain the primary topology for exposing and distributing one persistent logical world. Instances are optional isolated gameplay contexts and do not replace channels or create independent worlds.

Examples of instance-capable contexts include boss rooms, dungeons, arenas, quest scenarios and bounded events.

The existence of instances does not create a second economy, character namespace or permanent community partition.

## Accepted instance identity

The canonical semantic identity of a concrete instance is:

```text
WorldId + InstanceId
```

Accepted invariants:

- every instance belongs to exactly one logical world;
- `InstanceId` is semantically scoped by `WorldId` even if its technical representation is globally collision-resistant;
- `WorldId + ChannelId + InstanceId` is not the canonical instance identity;
- a concrete instance is not semantically owned by the source channel that created it or by any participant's origin channel;
- instance identity remains independent from GameNode, process, container, host and physical placement;
- map coordinates, queue numbers, activity names and template identifiers are not concrete instance identity;
- cross-world instances are forbidden;
- instance membership and lifecycle are explicit state, not inferred only from map coordinates.

The complete identifier contract still owns the exact representation, generation, encoding, lifecycle and visibility catalogue.

## Accepted cross-channel instance admission consequence

Eligible characters from different channels of the same logical world may be admitted into one concrete `WorldId + InstanceId`.

After admission:

- one authoritative `InstanceRuntime` owns the participants and instance-local simulation;
- source channels do not remain concurrent gameplay owners;
- each participant retains validated origin-channel routing metadata for exit, reconnect, audit and recovery;
- entry and exit are explicit generation-fenced ownership transitions rather than client-side teleports;
- party membership alone does not authorize admission;
- cross-channel open-world visibility, combat, experience, loot, healing and proximity effects remain forbidden.

Detailed map, Party Finder, physical-trigger, no-relogin handoff, reward and recovery rules are defined by the later owner-accepted instance baseline.

## Accepted party identity and semantics

The canonical semantic identity of a party is:

```text
WorldId + PartyId
```

Accepted semantics:

- every party belongs to exactly one logical world;
- one party may contain characters currently placed on different channels of that world;
- membership, leadership, invitations, roles, readiness, selected activity and party chat may remain valid across channel placement changes;
- party membership does not merge channel or instance simulations;
- open-world cooperative gameplay requires participants to share one `WorldId + ChannelId` and satisfy activity/proximity rules;
- instanced cooperative gameplay requires participants to be admitted into one common `WorldId + InstanceId`;
- a remote party member receives no cross-channel shared combat, experience, loot, healing, local visibility or proximity bonuses;
- channel changes do not automatically destroy party membership;
- cross-world parties and cross-world shared progression remain outside this baseline.

This separates organization from execution:

```text
WorldId + PartyId
    owns organization, membership, leadership, roles and readiness

WorldId + ChannelId
    owns open-world visibility, combat and local simulation

WorldId + InstanceId
    owns isolated instanced simulation after admission
```

## Party Finder identifier consequences

- discovery and matching may operate across all channels of one world;
- an open-world activity must select or confirm one target channel before shared simulation starts;
- an instanced activity may admit members directly from several source channels into one concrete instance;
- Party Finder does not authorize silent movement without readiness, validation, capacity reservation and failure handling;
- Party Finder consumes the shared activity-instance admission contract rather than defining separate instance identity or map semantics.

Party size, roles, queue algorithm, replacement policy, penalties, channel reservation and activity formulas remain unresolved.

## Class 3 — Runtime-local generational handle

A runtime-local handle addresses transient in-memory state owned by one runtime boundary.

Required properties:

- local to a named process, runtime, channel, instance, arena or allocation domain;
- generation-fenced so stale references fail deterministically after reuse;
- never treated as durable identity;
- never exposed as sole identity in public protocol, persistence, durable audit or cross-process contracts;
- invalid after the owning runtime boundary ends, reloads or advances its generation;
- convertible to durable identity only through an explicit owner-controlled lookup where such identity exists.

Candidate uses include creature slots, entity/component handles, task handles, pathfinding work references and subscription registrations.

A raw memory address, collection index or reusable integer without a generation is not an accepted runtime identity model.

## Class 4 — Ordering, revision and fencing value

Revisions, generations, sequence numbers, epochs and ticks describe order, version or authority. They are not semantic entity identities.

Required properties:

- owned by an explicit scope;
- monotonic or otherwise ordered according to the owning contract;
- compared only inside compatible scope and lifecycle;
- never reused to impersonate an entity ID;
- wraparound, exhaustion, reset and persistence behavior are explicit;
- stale values are rejected where they protect authority or causality;
- equality means the same ordering/version state, not the same entity.

Examples include:

- `session_generation`;
- channel and instance ownership generation;
- state and party revision;
- command sequence;
- snapshot/delta baseline revision;
- protocol, ruleset, content, map, schema and server-build revisions;
- simulation tick or journal position.

A generation may fence an identity but does not replace it. For example, `CharacterId + session_generation` expresses a current authority claim; the generation alone does not identify the character or session.

## Cross-cutting invariants

### Identity is semantically opaque

Canonical identifiers do not encode mutable or authorization-relevant facts such as:

- display name;
- world name;
- profession, class or level;
- account tier;
- current channel or instance placement;
- unnecessary timestamp disclosure;
- database shard or physical host location;
- security or moderation status.

Routing and indexing hints may exist only as separately validated metadata and never become proof of ownership, authorization or current placement.

### Labels are not identity

Names, slugs, aliases, display numbers, legacy numeric IDs, compact bundle-local IDs, row order and client list positions are not canonical identities by themselves.

They may resolve through an authoritative namespace, registry or content revision. Rename, localization, aliasing and reuse policy do not silently change semantic identity.

### Representation does not define semantics

The same wire or storage representation may be reused across identity classes only when distinct strong types and validators prevent accidental substitution.

Different encodings of one semantic identity preserve exact equality and canonicalization rules across Rust, Platform contracts, PostgreSQL, event schemas and fixtures.

### No implicit scope inference

Scope is not inferred solely from:

- the process currently handling a value;
- an active connection;
- a database connection or schema;
- thread-local or global state;
- the world selected in UI;
- a mutable route cache;
- the player's current coordinates.

Public and durable operations carry or derive scope from authoritative validated bindings.

### Authority requires identity plus fence

Security-sensitive and durable mutations require both semantic identity and current fencing values where applicable.

Examples include:

```text
CharacterId + GameSessionId + session_generation
WorldId + ChannelId + channel ownership generation
WorldId + InstanceId + instance ownership generation
WorldId + PartyId + party revision
ItemInstanceId + item/state revision
CommandId + GameSessionId + sequencing context
TransactionId + idempotency and ownership context
```

A valid identity with a stale generation or revision is not current authority.

### Client-generated values are claims, not proof

A client may originate identifiers or correlation values only where the owning contract explicitly permits it. The server validates namespace, ownership, uniqueness, replay and resource limits before acceptance.

Client possession of an identifier never proves authorization to observe or mutate the entity.

## Boundary between `FND-ID-01` and later contracts

The complete `FND-ID-01` contract owns:

- semantic meaning and owner of each minimum cross-boundary identifier;
- identity class;
- scope and uniqueness domain;
- reuse and lifecycle rules;
- public, protocol, session and event visibility;
- canonical comparison and validation rules;
- minimum encoding constraints required for Platform, client and server compatibility.

`DUR-01` later owns:

- exact PostgreSQL representation and indexes;
- migration from legacy identifiers;
- storage width and binary/text trade-offs beyond cross-boundary compatibility;
- partitioning and locality consequences;
- durable foreign references and archival behavior.

`FND-02`, `FND-03` and `FND-04` later own protocol fields, runtime ownership, snapshots, admission, sessions, leases and fencing state machines.

`DUR-01` and later contracts may refine representation and lifecycle mechanics but cannot redefine owner-accepted semantic scope.

## Required application to later contracts

This baseline is mandatory input to:

- `FND-ID-01` — complete identifier catalogue and semantics;
- `FND-02` — protocol fields, command sequencing, snapshots and reconciliation;
- `FND-03` — runtime handles, ticks, revisions, instance ownership and generations;
- `FND-04` — Game Session, admission, handoff and stale-writer fencing;
- `DUR-01` through `DUR-03` — durable identities, revisions, items, rewards and transactions;
- `DUR-04` — content keys, package/revision scope and compact runtime IDs;
- `ANL-01` — event, operation, transaction, correlation, causation and analytics identities;
- `QA-E2E-01` — exact identity, generation, transition and revision evidence;
- future party, Party Finder, world, channel, instance, map, house, social, economy and lifecycle contracts.

## Still unresolved for the complete `FND-ID-01`

The following remain open and must not be inferred from this baseline:

- the complete minimum identifier catalogue beyond accepted world, channel, instance and party semantics;
- exact lifecycle, retention and persistence rules for `InstanceId` and `PartyId`;
- which remaining durable identities are globally unique versus semantically scoped;
- UUID, ULID, integer, random, time-ordered or mixed generation strategy;
- byte width and canonical binary/text encoding;
- endianness and canonical string formatting;
- null, zero, nil and sentinel-value policy;
- identifier generation ownership and generator-failure behavior;
- collision handling and exhaustion policy;
- public exposure and enumeration resistance;
- redaction, logging and privacy classification;
- correlation/causation trust rules;
- revision and generation widths and wraparound policy;
- legacy mapping and migration;
- PostgreSQL representation, indexes and partitioning;
- serialization or IDL technology;
- Rust crate names and concrete type definitions;
- exact instance admission, transfer-token, placement and recovery representations;
- Party Finder matching and activity-specific policies.

## Rejected interpretations

### Use names or slugs as canonical identity

Rejected because names are mutable, user-visible, localized and potentially reusable.

### Treat all identifiers as globally interchangeable

Rejected because world, channel, instance, party, content, revision and runtime scopes carry security and consistency meaning even when technical values are globally collision-resistant.

### Treat `ChannelId` as world-independent

Rejected because its canonical semantic identity is `WorldId + ChannelId`.

### Bind concrete instance identity to the source channel

Rejected because players from several channels of one world may enter the same concrete instance and each retains separate origin-channel metadata.

### Use `WorldId + ChannelId + InstanceId` as canonical instance identity

Rejected because source channel is routing/history metadata rather than the semantic owner of the instance.

### Treat instances as a replacement for channels

Rejected because channels are the primary persistent-world topology and instances are bounded isolated gameplay contexts.

### Bind party existence to one channel

Rejected because party organization and readiness are world-level, while simulation remains channel- or instance-local.

### Allow cross-channel combat through party membership

Rejected because party membership does not merge simulations.

### Persist runtime handles

Rejected because handle reuse and runtime lifecycle make them unsafe as durable or cross-process identity.

### Use revision or generation as entity identity

Rejected because an ordering/fencing value only has meaning relative to its owner.

### Encode current business state into identifiers

Rejected because mutable semantics, routing, authorization and privacy remain independently validated.

## Programme effect

- This owner-accepted baseline is canonical input to `FND-ID-01`.
- `WorldId` globally identifies a logical world.
- The accepted channel identity is `WorldId + ChannelId`.
- The accepted instance identity is `WorldId + InstanceId`.
- The accepted party identity is `WorldId + PartyId`.
- Channels remain the primary world topology.
- A party may organize members across channels of one world.
- Eligible players from several channels of that world may enter one shared concrete instance.
- Open-world simulation remains channel-local and instanced simulation remains instance-local.
- Exact technical representations, complete catalogues and implementation contracts remain future work.
- The source-only `blakinio/otclient` historical marker remains required before the complete `FND-ID-01` package begins.
- No implementation is authorized by this document.
