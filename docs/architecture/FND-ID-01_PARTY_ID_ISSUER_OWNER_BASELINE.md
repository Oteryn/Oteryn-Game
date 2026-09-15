# FND-ID-01 PartyId Issuer Owner Baseline

- Status: Owner-accepted pre-contract baseline
- Date: 2026-08-07
- Decision owner: Oteryn project owner
- Gate: `FND-ID-01`
- Coordination ID: `OTV2-NATIVE-FOUNDATION`

## Purpose

Record the accepted representation, semantic scope and issuer authority for canonical `PartyId` before the complete `FND-ID-01` identifier catalogue is finalized.

This baseline is architecture input only. It does not authorize runtime, protocol, persistence, Platform or production implementation.

## Owner-accepted decision

Canonical party identity is:

```text
PartyId  = strongly typed UUIDv7, full 128 bits
PartyRef = WorldId + PartyId
```

The canonical issuer is the authoritative **world-level game-domain Party/Social authority** responsible for the party lifecycle and membership state.

Platform Identity, Game Gateway and World Registry do not mint canonical `PartyId` values merely because they provide account identity, routing or world topology.

## Semantic scope

The accepted semantic identity remains:

```text
WorldId + PartyId
```

A globally collision-resistant UUID value does not remove the required `WorldId` scope.

A party belongs to exactly one logical world. It is distinct from:

- the current party leader;
- any member `CharacterId`;
- a member's current `ChannelId`;
- an `InstanceId` entered by the party;
- a Game Session;
- a GameNode process;
- an invitation token or Party Finder entry.

Leader changes, member movement between channels, GameNode relocation and entry into an instance do not change `PartyId`.

## Cross-channel party model

A party is world-level social/gameplay state, not channel-local identity.

Members of one party may be placed on different channels of the same world:

```text
World W + PartyId P
    member A -> Channel C1
    member B -> Channel C2
    member C -> Instance I
```

The party remains one semantic entity while placement changes.

Consequences:

- `ChannelId` must not be embedded into or derived from `PartyId`;
- changing channel does not require leaving/recreating the party;
- a common same-world instance admission flow may consume the same PartyId for members originating from different channels;
- exact visibility, distance, shared-exp, loot, combat and party-benefit rules across placements are separate gameplay contracts and are not frozen here.

Cross-world parties are not created by this decision; `PartyId` remains explicitly scoped by `WorldId`.

## Issuer boundary

The authoritative world-level Party/Social authority:

- creates a fresh canonical `PartyId` when a new party lifecycle is established;
- binds it to exactly one `WorldId`;
- owns authoritative party membership/lifecycle state under later social contracts;
- ensures uniqueness and no reuse;
- does not derive PartyId from leader CharacterId, channel, invite code, Party Finder entry or current GameNode placement.

The exact crate/service/process placement of this authority is deliberately not fixed here. Logical domain ownership is fixed; runtime topology and persistence boundaries remain later contract work.

## Lifecycle rules

Accepted identity rules:

- `PartyId` is immutable for one party lifecycle;
- a newly created party gets a fresh UUIDv7;
- a later newly created party gets a different PartyId even if it has the same leader/members;
- leadership transfer does not change PartyId;
- members joining/leaving do not change PartyId;
- channel transitions do not change PartyId;
- nil/zero UUID is invalid;
- absence is explicit rather than represented by a sentinel;
- PartyId is never reused for a different semantic party;
- UUIDv7 timestamp ordering is not membership order, authority, causality or revision evidence.

Exact rules for an empty party, disbandment, restoration/tombstones and whether a recently disconnected member preserves membership are deliberately deferred to the social lifecycle contract. Reuse of an identifier for a different party is forbidden.

## Identity is not membership authority

`PartyId` is an identifier, not a capability.

Knowing a PartyId does not prove that a character:

- is a member;
- is the leader;
- may invite or kick;
- may join a party-only instance;
- may receive party rewards;
- may see privacy-sensitive placement information.

Membership, roles, invitations, revisions and authorization must be validated separately by the authoritative Party/Social state.

Client-provided PartyId values never establish membership or authority.

## Relationship to Platform and account identity

Party identity remains game-domain authority even though account authentication is Platform-owned.

Accepted separation:

```text
Platform AccountId / Identity
    -> authenticates account/session boundary

CharacterId
    -> identifies game character

WorldId + PartyId
    -> identifies one game-domain party
```

Platform may receive projections or references required by product/control-plane features, but it does not become canonical PartyId issuer or party-membership mutation authority by storing or displaying those references.

## Protocol and persistence boundaries

`FND-ID-01` fixes UUIDv7/full-128-bit representation, world scope and logical issuer.

Later contracts must define:

- exact party membership/revision model;
- commands and optimistic/idempotent mutation semantics;
- wire byte order and IDL in `FND-02`;
- session/admission binding implications in `FND-04`;
- persistence rows, constraints, tombstones and indexes in durability work;
- presence/privacy projections and stale-data handling;
- recovery and authority behavior if party members are distributed across multiple ChannelRuntimes.

No protocol or database format is selected here beyond lossless UUIDv7 preservation.

## Rejected alternatives

### Platform issues PartyId

Rejected because party lifecycle and membership are game-domain authority rather than account identity, Gateway routing or World Registry topology state.

### PartyId is channel-local

Rejected because accepted architecture allows a party to span multiple channels within the same logical world.

### Leader CharacterId is the party identity

Rejected because leadership can change without creating a different semantic party and one character may participate in multiple distinct party lifecycles over time.

### Invite code or Party Finder entry is PartyId

Rejected because invitation/discovery artifacts have separate lifecycle and security semantics.

## Required application to later contracts

This baseline is mandatory input to:

- complete `FND-ID-01` catalogue/issuer matrix;
- `FND-02` party references and social commands on the wire;
- `FND-03` distributed runtime ownership/projection consequences;
- `FND-04` party-aware admission/instance handoff where applicable;
- social/presence/privacy architecture;
- durability contracts for party membership state;
- instance/activity contracts consuming a party as one admission group;
- E2E tests proving party identity survives channel transitions and leadership change while unauthorized membership claims fail closed.

## Programme effect

- `PartyId` -> strongly typed UUIDv7, full 128 bits;
- semantic identity -> `WorldId + PartyId`;
- issuer -> authoritative world-level game-domain Party/Social authority;
- Platform is not the canonical issuer;
- one party may contain members on different channels of the same world;
- leader, member placement, InstanceId and invite artifacts are not party identity;
- exact Party/Social service placement, membership revisions, protocol and persistence remain later contract work;
- no implementation is authorized by this baseline.
