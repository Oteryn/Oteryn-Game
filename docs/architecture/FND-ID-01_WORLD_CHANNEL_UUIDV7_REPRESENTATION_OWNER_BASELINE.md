# FND-ID-01 World and Channel UUIDv7 Representation Owner Baseline

- Status: Owner-accepted pre-contract baseline
- Date: 2026-08-07
- Decision owner: Oteryn project owner
- Gate: `FND-ID-01`
- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Applies to: canonical target representation of `WorldId` and `ChannelId`, coordinated Platform World Registry contracts, later protocol/persistence/admission contracts

## Purpose

Record the project owner's accepted representation decision for the durable world and channel identities whose issuance authority was already assigned to the Platform World Registry / authoritative topology-control boundary.

This baseline resolves the remaining conditional wording in the earlier UUIDv7 direction specifically for `WorldId` and `ChannelId`.

It is canonical architecture input to the complete `FND-ID-01` contract. It does not authorize runtime, protocol, persistence, Platform, migration or production implementation.

## Owner-accepted decision

The canonical target representation is:

```text
WorldId   = strongly typed UUIDv7, full 128 bits
ChannelId = strongly typed UUIDv7, full 128 bits

ChannelRef = WorldId + ChannelId
```

`WorldId` and `ChannelId` are distinct semantic types even though they share the same physical UUIDv7 representation.

Canonical channel identity remains semantically scoped as:

```text
WorldId + ChannelId
```

A globally collision-resistant `ChannelId` value does not erase or relax its required `WorldId` scope.

## Authority remains unchanged

This representation decision does not change identifier issuance ownership.

The accepted authority remains:

```text
Platform World Registry / authoritative topology-control boundary
    -> issues canonical WorldId
    -> issues canonical ChannelId

Oteryn-v2 GameNode / ChannelRuntime
    -> consumes assigned identifiers
    -> never mints competing canonical world/channel identity
```

The service or library that physically generates UUID bytes does not become the semantic identity owner merely by performing generation. The authoritative World Registry contract remains responsible for allocation, uniqueness, lifecycle and registry ownership.

## Strong typing

Future Rust and cross-boundary contracts must preserve semantic type separation.

Conceptually:

```text
WorldId(UUIDv7)
ChannelId(UUIDv7)
```

The following are invalid architectural substitutions:

```text
WorldId == raw String
ChannelId == raw String
WorldId == ChannelId
ChannelId == channel ordinal
ChannelId == process/pod/replica identifier
```

APIs and domain boundaries must not accept an untyped UUID where the semantic type is known.

## Full 128-bit preservation

Every canonical cross-boundary representation must preserve all 128 UUIDv7 bits.

The following are prohibited as canonical durable identity:

- truncating a UUID to 64 or 32 bits;
- storing only a hash fragment;
- deriving identity from the UUID timestamp portion;
- converting it to a channel number or world slug and discarding the UUID;
- using an implementation-local integer as the only durable cross-boundary identity.

Compact runtime or session handles remain permitted where an authoritative mapping to the durable identity exists and the handle never substitutes for canonical durable identity.

## Display names, slugs and ordinals are labels

The following values are not canonical identity:

- world display name, for example `Antica`;
- world slug or URL slug;
- DNS/hostname;
- database row ordinal;
- `Channel 1`, `Channel 2`, `Channel 3` display numbers;
- GameNode name;
- process/container/pod/replica identifier.

They may change without changing the underlying `WorldId` or `ChannelId`.

A rename, localization change, display-channel reorder, server restart, GameNode replacement or infrastructure relocation therefore does not mint a new identity.

## Scope and comparison rules

Accepted comparison rules:

- `WorldId` compares only as `WorldId`;
- `ChannelId` compares only as `ChannelId`;
- a channel reference crossing a boundary that requires world scope carries or derives a validated `WorldId + ChannelId` pair;
- matching only `ChannelId` must not be used to infer that two channel references belong to the same logical world;
- names, slugs and display ordinals are never substituted into equality or authorization checks;
- UUIDv7 creation-time ordering is never used as gameplay causality, authority, freshness or fencing evidence.

## Lifecycle rules

For both identifiers:

- identity is immutable after authoritative assignment;
- nil/zero UUID is invalid as a real entity identity;
- absence is explicit rather than represented by a sentinel UUID;
- identifiers are never reused for a different semantic world or channel;
- rename does not change identity;
- restart or relocation does not change identity;
- stale execution ownership is handled by explicit ownership generation/fencing rather than by changing `ChannelId`;
- collision or duplicate insertion must fail closed and must never overwrite an existing identity;
- retirement/tombstone/reactivation policy remains to be completed by `FND-ID-01` and the owning topology contract, but may not permit reuse for a different semantic entity.

## Cross-repository migration boundary

`WorldId` and `ChannelId` are externally issued by Platform World Registry under the accepted repository boundary. Therefore this owner decision sets the Oteryn-v2 canonical target contract but does not claim that the current Platform implementation already uses UUIDv7.

Required consequences:

- Oteryn-v2 must not silently reinterpret, hash, truncate or re-key an existing Platform identifier into UUIDv7;
- the coordinated Platform contract must explicitly adopt UUIDv7 for `WorldId` and `ChannelId` before production cross-repository use claims conformance to this representation;
- if current Platform data uses a legacy slug/string/integer representation, migration must define an authoritative one-to-one mapping, uniqueness validation, backfill/rollback rules and audit evidence;
- legacy names or IDs may remain aliases/migration keys, but they do not remain canonical identity after the coordinated migration is accepted;
- mixed-version rollout must fail closed where an endpoint cannot prove the expected identifier representation and revision;
- no Oteryn-v2 task may mutate Platform data or schemas without separate explicit repository authorization.

The exact Platform-side migration task remains separate cross-repository work.

## Protocol boundary

`FND-ID-01` freezes the identity family and semantic constraints; `FND-02` later freezes exact wire encoding.

Mandatory `FND-02` consequences:

- preserve all 128 UUIDv7 bits;
- preserve strong semantic distinction between world and channel identity;
- carry sufficient validated world scope for channel references;
- define one canonical byte order / binary representation;
- define canonical textual representation for diagnostics/interchange where needed;
- define malformed/version/variant/nil rejection behavior;
- permit compact session-local handles only through an authoritative session mapping;
- never use UUIDv7 timestamp order as protocol sequence or state revision.

This baseline does not choose protobuf, another IDL, framing or transport.

## Persistence boundary

Later `DUR-01`/`DUR-02` work must preserve the UUIDv7 canonical identity and may optimize physical indexing without replacing it.

Accepted direction remains:

- PostgreSQL native `uuid` is preferred for an adopted canonical UUID identity;
- display names/slugs are separate columns/attributes, not primary semantic identity;
- auxiliary surrogate/partition/index keys may exist for performance but do not replace `WorldId`/`ChannelId` at system boundaries;
- migration from any current external representation requires explicit coordinated mapping and validation.

Exact primary keys, indexes, clustering and migration DDL remain outside this baseline.

## Privacy and exposure

UUIDv7 contains time-related information and is not automatically a public identifier.

Therefore:

- internal `WorldId`/`ChannelId` values are exposed to clients/public APIs only where the owning product/protocol contract requires them;
- public URLs and human-facing navigation may continue to use stable slugs or opaque public references;
- knowledge of `WorldId` or `ChannelId` never grants authorization;
- exact channel placement remains subject to the accepted privacy/presence rules and is not made public merely because a UUID exists.

## Rejected alternatives

### Keep world/channel names as canonical IDs

Rejected because names and slugs are mutable presentation/business identifiers and create rename, localization and routing coupling.

### Use small sequential integers as canonical IDs

Rejected as the canonical cross-boundary identity because allocation becomes more tightly centralized, values invite scope confusion and cross-repository migration/merge behavior becomes less robust. Compact integers remain valid only as local/session/runtime handles where explicitly scoped.

### Use UUIDv4 instead of UUIDv7

Rejected for these new durable identities because the accepted durable-identity architecture standard is UUIDv7, providing one consistent family and better locality characteristics without using timestamp order as authority.

### Make ChannelId alone the semantic channel identity

Rejected even though `ChannelId` is UUIDv7. The accepted model remains `WorldId + ChannelId` so scope is explicit and cannot be lost through implementation convenience.

### Generate a new ChannelId on every restart

Rejected because restart changes execution ownership, not semantic channel identity. Ownership generation/fencing handles stale writers.

## Required application to later contracts

This decision is mandatory input to:

- the complete `FND-ID-01` identifier catalogue and owner/issuer matrix;
- coordinated Platform World Registry identity contracts;
- `FND-02` wire representation and protocol field contracts;
- `FND-03` runtime assignment and fencing;
- `FND-04` Game Session world/channel binding;
- `DUR-01`/`DUR-02` persistence representation and migration;
- observability/audit schemas that reference world/channel identity;
- E2E scenarios proving stable identity across rename, restart and relocation and rejection of malformed/stale identity bindings.

## Programme effect

The previous conditional UUIDv7 wording for `WorldId` and `ChannelId` is resolved for the Oteryn-v2 target architecture:

- `WorldId` -> strongly typed UUIDv7, full 128 bits;
- `ChannelId` -> strongly typed UUIDv7, full 128 bits;
- canonical channel identity -> `WorldId + ChannelId`;
- World Registry remains the issuer/authority;
- names/slugs/channel numbers remain labels;
- no silent re-key of existing Platform identifiers is allowed;
- Platform-side adoption/migration requires a separately authorized coordinated contract;
- exact wire bytes/text form remain `FND-02` work;
- no implementation is authorized by this baseline.
