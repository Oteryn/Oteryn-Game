# FND-ID-01 InstanceId Issuer Owner Baseline

- Status: Owner-accepted pre-contract baseline
- Date: 2026-08-07
- Decision owner: Oteryn project owner
- Gate: `FND-ID-01`
- Coordination ID: `OTV2-NATIVE-FOUNDATION`

## Purpose

Record the accepted issuer, scope and representation semantics for canonical `InstanceId` before the complete `FND-ID-01` identifier catalogue is finalized.

This baseline is architecture input only. It does not authorize runtime, protocol, admission, persistence or production implementation.

## Owner-accepted decision

Canonical concrete instance identity is:

```text
InstanceId = strongly typed UUIDv7, full 128 bits
InstanceRef = WorldId + InstanceId
```

The canonical issuer is the authoritative **game-domain Instance/Activity allocator** responsible for creating the concrete `InstanceRuntime` lifecycle.

Platform Identity, Game Gateway and World Registry do not mint canonical `InstanceId` values merely because they participate in admission, routing or discovery.

## Semantic scope

The accepted semantic identity remains:

```text
WorldId + InstanceId
```

A globally collision-resistant UUID value does not remove the required `WorldId` scope.

`InstanceId` identifies one concrete instance/runtime lifecycle. It is distinct from:

- an activity/template/content key;
- a dungeon/boss definition;
- an origin `ChannelId`;
- a party;
- a Game Session;
- a GameNode process;
- a map coordinate or raw xyz position.

The same activity/template may produce many concrete `InstanceId` values over time.

## Issuer boundary

The authoritative Instance/Activity allocator:

- validates creation/admission prerequisites owned by its domain contract;
- creates the canonical `InstanceId` when a new concrete instance lifecycle is established;
- ensures uniqueness and no reuse;
- binds the instance to exactly one `WorldId`;
- establishes the initial instance ownership/lifecycle record consumed by runtime and persistence contracts;
- does not infer identity from a requesting party, origin channel, GameNode or template key.

The exact crate/service/process placement of the allocator is deliberately not frozen here. Logical domain authority is fixed; deployment topology remains later `FND-03`/operations work.

## Cross-channel instance consequence

Participants from multiple channels of the same logical world may enter one shared concrete instance.

Therefore origin channel is routing/handoff metadata, not canonical instance identity:

```text
World W / Channel A participants
World W / Channel B participants
        -> one concrete World W + InstanceId I
```

Once handoff completes, the authoritative `InstanceRuntime` owns the instance-local simulation under the accepted instance baseline.

A participant's source `ChannelId` must not be encoded into or derived from `InstanceId`.

## Lifecycle rules

- `InstanceId` is immutable for one concrete instance lifecycle.
- A newly created concrete instance gets a fresh UUIDv7.
- A later run of the same activity/template gets a different `InstanceId`.
- `InstanceId` is never reused for a different semantic instance.
- nil/zero UUID is invalid.
- absence is explicit rather than represented by a sentinel.
- UUIDv7 timestamp ordering is not gameplay causality, authority, lockout ordering or fencing.
- instance ownership generation/fencing is separate from `InstanceId` and may change during recovery while identity remains stable when recovering the same concrete instance.

Whether a fully terminated instance can ever be restored as the same concrete lifecycle versus always creating a new one remains a recovery/lifecycle contract detail; reuse for a different semantic instance is forbidden.

## Authority and security

`InstanceId` is identity, not admission authority.

Knowing an `InstanceId` does not grant a player or service the right to:

- enter the instance;
- observe its state;
- mutate it;
- receive rewards;
- resume a prior attempt;
- route another character into it.

Admission state, participant membership, session binding, instance ownership generation and other fences remain independently validated.

## Protocol and persistence boundaries

`FND-ID-01` fixes UUIDv7/full-128-bit identity and semantic scope.

`FND-02` later fixes exact wire byte order, IDL/text representation and any compact session handles.

`DUR-01`/`DUR-02` later fix physical PostgreSQL representation, indexes, lifecycle rows and recovery persistence while preserving the canonical identity.

No protocol or database representation is selected by this baseline beyond full, lossless UUIDv7 preservation.

## Rejected alternatives

### Platform issues InstanceId

Rejected because Platform is the external identity/routing control plane, while concrete gameplay instance lifecycle is game-domain authority.

### Origin ChannelId is part of InstanceId

Rejected because one concrete instance may admit participants originating from multiple channels in the same world and may survive movement/recovery independent of origin channel placement.

### Activity/template key is InstanceId

Rejected because a reusable definition and one concrete runtime occurrence have different lifecycle and cardinality.

### GameNode generates instance identity as infrastructure side effect

Rejected as an ownership rule. Physical UUID generation may later be hosted inside an authoritative allocator process, but GameNode placement itself does not define instance identity.

## Required application to later contracts

This baseline is mandatory input to:

- complete `FND-ID-01` catalogue/issuer matrix;
- `FND-02` instance references on the wire;
- `FND-03` InstanceRuntime lifecycle, ownership and recovery;
- `FND-04` instance admission/handoff/session continuation;
- `DUR-01`/`DUR-02` durable representation;
- activity, lockout and reward-settlement contracts;
- E2E scenarios for cross-channel same-world entry and stale/unauthorized instance access.

## Programme effect

- `InstanceId` -> strongly typed UUIDv7, full 128 bits;
- semantic identity -> `WorldId + InstanceId`;
- issuer -> authoritative game-domain Instance/Activity allocator;
- Platform is not the canonical issuer;
- origin channel, GameNode and template key are not instance identity;
- exact allocator deployment/API, wire encoding and persistence layout remain later contract work;
- no implementation is authorized by this baseline.
