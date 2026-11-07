# FND-ID-01 Owner Decision Checkpoint — 2026-08-07

- Status: Owner-accepted coordination checkpoint
- Date: 2026-08-07
- Decision owner: Oteryn project owner
- Gate: `FND-ID-01`
- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Scope: consolidation of owner-accepted foundation identity/protocol-boundary decisions recorded in PRs #63 through #68

## Purpose

This document preserves, in one recoverable place, the complete set of owner-accepted `FND-ID-01` decisions established during the 2026-08-07 architecture conversation.

It is a coordination/recovery checkpoint, not a replacement for the detailed owner baselines. Where a detailed owner baseline exists, that detailed baseline remains the normative source for the topic.

This checkpoint does not authorize implementation. Runtime, protocol, persistence, Platform, web-portal, orchestrator and production changes remain prohibited until separately authorized and gated.

## Source packages

The accepted decisions are currently recorded in these bounded packages:

- PR #63 — Platform native gameplay contract reconciliation input;
- PR #64 — `WorldId` / `ChannelId` UUIDv7 representation;
- PR #65 — `NodeId` process-incarnation identity;
- PR #66 — `InstanceId` issuer and scope;
- PR #67 — `PartyId` issuer and scope;
- PR #68 — `CharacterId` global identity and Platform account linkage.

The checkpoint is deliberately downstream of these detailed packages and must not be interpreted as silently superseding them.

## 1. Global identity model

Oteryn preserves distinct semantic classes:

1. durable cross-boundary identity;
2. scoped identity;
3. runtime-local generational handle;
4. ordering/revision/fencing value.

UUIDv7 is the accepted canonical representation direction for independently addressable Oteryn-owned durable identities where the owning contract has adopted it.

Accepted cross-cutting rules:

- strong semantic types are mandatory; raw interchangeable UUIDs are not the domain model;
- adopted UUIDv7 identities preserve all 128 bits;
- nil/zero UUID is invalid for canonical entity identity;
- absence is explicit;
- identifiers are immutable for their defined semantic lifetime;
- identifiers are never reused for a different semantic entity;
- UUIDv7 ordering is not authority, causality, freshness, membership revision or fencing evidence;
- identity alone never grants observation or mutation authority;
- current session, revision, generation, lease or fencing state is validated separately;
- names, slugs, display numbers, infrastructure labels and mutable business attributes are not identity;
- compact runtime/session handles may be used in hot paths but cannot replace canonical durable identity at system boundaries.

## 2. Platform native protocol reconciliation

The existing `blakinio/Oteryn-Platform` native gameplay protocol contract remains useful evidence and reconciliation input, but it is not the final `protocol-oteryn` authority for Oteryn v2.

Accepted disposition:

- preserve compatible concepts such as Platform Identity authority, one-time game ticketing, Game Gateway, World Registry, fail-closed replay/expiry/mismatch behavior, exact session/route binding, downgrade prevention, explicit version compatibility and security/redaction/observability expectations;
- do not import Canary production negotiation/fallback;
- do not import C++ Otheryn runtime assumptions;
- do not treat the historical exact transport/IDL/schema/capability tuple as the final Oteryn-v2 freeze;
- preserve ordered gates `FND-ID-01 -> FND-02 -> FND-03/FND-04`;
- the external contract is `RECONCILIATION_INPUT_ONLY`, not final protocol authority;
- only project-owned `protocol-oteryn` is the target production game protocol;
- `protocol-canary` remains reference-only and must not become production fallback, translator or downgrade path.

Exact framing, IDL, schema, byte order, capability negotiation, message set, transport and compatibility matrix remain `FND-02` work.

## 3. WorldId

Accepted canonical target:

```text
WorldId = strongly typed UUIDv7, full 128 bits
```

Semantics:

- identifies one logical world;
- globally unique durable identity;
- world name/slug is a mutable label, not identity;
- restart, relocation or game-server scaling does not change WorldId.

Issuer/authority:

- Platform World Registry / authoritative topology-control boundary is the logical issuer and registry owner;
- GameNode, ChannelRuntime, container, host and external orchestrator do not establish canonical WorldId.

Cross-repository caveat:

- Oteryn-v2 records the target contract;
- current Platform representation is not silently re-keyed;
- coordinated Platform adoption/migration must be explicit.

## 4. ChannelId

Accepted canonical target:

```text
ChannelId = strongly typed UUIDv7, full 128 bits
ChannelRef = WorldId + ChannelId
```

Semantics:

- canonical semantic channel identity remains `WorldId + ChannelId` even though the physical UUID is collision-resistant;
- `ChannelId` must not be interpreted without its `WorldId` scope at durable/cross-boundary semantic interfaces;
- labels such as `Channel 1`, `Channel 2`, PvP label, slug or route display name are not identity;
- moving/recovering/restarting the same semantic channel preserves `WorldId + ChannelId`.

Issuer/authority:

- Platform World Registry / topology-control authority issues and registers ChannelId;
- external orchestrator may request capacity and place processes but does not mint canonical channel identity;
- GameNode consumes an authorized assignment;
- ChannelRuntime owns authoritative gameplay mutation after assignment, not topology identity.

Current execution authority is separate:

```text
WorldId + ChannelId + current ownership generation/fence
```

A stale runtime cannot regain authority merely by knowing valid IDs.

## 5. NodeId

Canonical target:

```text
NodeId = strongly typed UUIDv7, full 128 bits
```

Canonical term is `NodeId`. Historical/candidate `GameNodeId` must not silently become a second independent semantic identifier.

Semantics:

- `NodeId` identifies exactly one concrete GameNode process incarnation;
- every process start receives a fresh NodeId;
- process restart receives a new NodeId;
- process replacement receives a new NodeId;
- the same running process keeps its NodeId for that incarnation;
- retired NodeId is never reused for a different process incarnation.

NodeId is not:

- physical host identity;
- VM identity;
- Kubernetes node identity;
- pod/container identity;
- Docker/container runtime ID;
- replica ordinal;
- deployment name;
- stable orchestrator slot;
- rack/AZ identity.

If stable placement identity is later required, it receives a separate semantic type.

NodeId is execution identity, not gameplay authority.

The same semantic channel may move:

```text
World W + Channel C
NodeId A + generation 41
    -> recovery/relocation
NodeId B + generation 42
```

without changing `WorldId + ChannelId`.

The exact NodeId generation/registration/bootstrap handshake remains deliberately unresolved for later `FND-ID-01`/`FND-03`/operations contracts.

## 6. InstanceId

Canonical target:

```text
InstanceId = strongly typed UUIDv7, full 128 bits
InstanceRef = WorldId + InstanceId
```

Semantics:

- canonical instance identity remains `WorldId + InstanceId`;
- origin `ChannelId` is routing/history/context metadata, not semantic instance identity;
- `WorldId + ChannelId + InstanceId` is not canonical instance identity;
- GameNode placement is not instance identity;
- activity/template/content key is not concrete InstanceId;
- concrete instances belong to one logical world;
- eligible participants from different channels of the same world may enter one common concrete instance;
- after handoff one authoritative InstanceRuntime owns instance-local simulation.

Issuer/authority:

- authoritative game-domain Instance/Activity allocation authority is the logical issuer;
- Platform Identity, Game Gateway and World Registry do not mint canonical InstanceId;
- client cannot mint authoritative InstanceId;
- orchestrator does not mint InstanceId.

Logical ownership is frozen, but exact service/process placement of the allocator remains deferred to runtime topology work.

InstanceId is identity, not admission proof or ownership/fencing authority.

Still unresolved unless separately accepted later:

- exact creation transaction boundary;
- exact recovery lifetime/tombstone semantics;
- whether a recovered same logical instance always preserves InstanceId across every failure class;
- allocator deployment/API;
- wire/storage encoding beyond lossless UUIDv7.

## 7. PartyId

Canonical target:

```text
PartyId = strongly typed UUIDv7, full 128 bits
PartyRef = WorldId + PartyId
```

Semantics:

- canonical party identity remains `WorldId + PartyId`;
- party is world-level social/gameplay organization, not channel-local identity;
- one party may contain members placed on different channels of the same world;
- party may include members temporarily placed in a shared same-world instance;
- leader change does not change PartyId;
- member join/leave does not by itself change PartyId;
- member channel transition does not change PartyId;
- entry into an instance does not change PartyId;
- GameNode placement does not change PartyId;
- invitation token, Party Finder entry and leader CharacterId are not PartyId.

Issuer/authority:

- authoritative world-level game-domain Party/Social authority issues PartyId and owns canonical party lifecycle/membership state;
- Platform Identity, Game Gateway and World Registry do not mint canonical PartyId.

Party identity does not merge simulations:

- open-world combat/visibility/proximity remains channel-local;
- instance simulation remains instance-local;
- remote party membership alone grants no shared combat, experience, loot, healing or local visibility.

PartyId is not membership/role/authorization proof.

Exact membership revision model, invite semantics, shared-exp/loot rules, empty-party lifecycle, reconnect handling, persistence and service placement remain later work.

## 8. CharacterId

Canonical target:

```text
CharacterId = strongly typed UUIDv7, full 128 bits
CharacterRef = CharacterId
```

CharacterId is a global durable game-domain identity and is deliberately **not** semantically `WorldId + CharacterId`.

Issuer/authority:

- authoritative game-domain Character authority issues CharacterId and owns character lifecycle identity;
- Platform Identity owns AccountId and must not mint competing CharacterIds;
- game-domain must not mint competing AccountIds.

CharacterId is independent from:

- AccountId;
- WorldId;
- ChannelId;
- InstanceId;
- character name;
- level/vocation/progression state;
- current Game Session;
- GameNode placement;
- public profile slug/URL.

### Rename

Renaming preserves CharacterId.

Consequences:

- analytics/history/achievements/item provenance/moderation/support can continue to correlate the same semantic character;
- a new display name does not create a new character identity.

### World transfer

Authorized world transfer preserves CharacterId while changing authoritative current-world membership.

Conceptually:

```text
CharacterId C
current_world = W1
    -> transfer
CharacterId C
current_world = W2
```

Transfer does not permit concurrent two-world gameplay authority. The final transfer contract must fence stale source-world mutation rights and establish one authoritative destination membership.

### Deletion/reuse

- CharacterId is never reused for a different character;
- deletion does not return the ID to an allocation pool;
- recreating the same character name creates a different CharacterId;
- tombstone/audit semantics remain available as required by durability/security/economy/privacy contracts.

## 9. AccountId and Platform WWW linkage

`AccountId` remains owned by Oteryn Platform Identity.

The accepted cross-boundary ownership relationship is conceptually:

```text
Platform AccountId A
    -> CharacterId C1
    -> CharacterId C2
    -> CharacterId C3
```

The game-domain character state stores/validates the authoritative account-owner binding where applicable.

Platform WWW consumes an authorized projection such as:

```text
AccountId -> CharacterId[] + allowed character projection fields
```

Platform may use the projection for product surfaces such as:

- account character list;
- character profile;
- public gameplay summary;
- world, level, vocation and progression projections;
- achievements/statistics;
- transfer workflows;
- future Bazaar/marketplace workflows if explicitly accepted;
- support/moderation views subject to permissions;
- player-facing Game Analytics projections subject to privacy policy.

This does not make Platform the gameplay owner or CharacterId issuer.

Account-character linkage must use an explicit cross-repository/service contract rather than unrestricted cross-database coupling.

When authorization or projection freshness cannot be proven, Platform-facing disclosure fails toward less information.

Alternate-character/account relationships remain private by default unless an explicit privacy/product contract exposes them.

## 10. Possible future account ownership transfer

This checkpoint records only the accepted direction already established for future marketplace/Bazaar compatibility:

If a later explicitly approved system transfers one character between Platform accounts, the default architectural direction is to preserve CharacterId and update the authoritative AccountId ownership binding through an audited transaction.

This is not yet an implemented or fully frozen Bazaar contract.

Exact sale/gift/account-merge/recovery/sanction/rollback semantics remain unresolved.

## 11. Identity versus authority matrix

Accepted separation:

```text
WorldId
    identifies logical world

WorldId + ChannelId
    identifies semantic channel

NodeId
    identifies one GameNode process incarnation

WorldId + InstanceId
    identifies concrete same-world instance

WorldId + PartyId
    identifies world-level party

CharacterId
    identifies global semantic character

AccountId
    identifies Platform-owned account
```

Current authority additionally requires the relevant independently validated context, for example:

```text
WorldId + ChannelId + ownership generation/fence
WorldId + InstanceId + instance ownership generation/fence
CharacterId + GameSessionId + session_generation + lease/fence
WorldId + PartyId + party/membership revision
```

No UUID is a bearer credential or capability merely because it is hard to guess.

## 12. Runtime and wire efficiency

The durable UUIDv7 model does not require UUIDs in every hot-path operation.

Accepted direction:

- hot simulation uses runtime-local generational handles where safe;
- frequent protocol deltas may use compact session-local handles established by an authoritative snapshot/session context;
- static content uses stable content key + revision + compact bundle/runtime IDs rather than UUID-per-definition/tile;
- nested snapshot-local transient state may use closed aggregate-local references where it has no independent durable lifecycle;
- canonical durable identities remain available at service/protocol/persistence/audit/recovery boundaries where required.

Exact handle widths, mapping reset rules, wire byte order and IDL remain later contracts.

## 13. Public exposure and privacy

UUIDv7 is internal identity by default, not automatically public identity.

Reasons include stable cross-context correlation and approximate creation-time leakage.

Accepted direction:

- public profile surfaces may use names/slugs/opaque public references rather than raw internal UUIDs;
- public references do not grant authority;
- hidden account/alternate-character relations must not be inferable merely from internal IDs;
- analytics, moderation and support may correlate canonical identities under explicit access control/redaction/retention policy.

## 14. No silent re-key or implicit translation

Cross-repository identities preserve their owning authority.

Accepted rules:

- Oteryn-v2 does not silently re-key Platform-owned AccountId;
- Platform does not silently invent replacement CharacterId/InstanceId/PartyId values;
- WorldId/ChannelId UUIDv7 adoption across Platform must be explicit and coordinated;
- mixed-version ambiguity fails closed rather than silently translating semantic identities;
- names/slugs/legacy numeric IDs can be mapped only through explicit owner-controlled mapping/migration contracts.

## 15. Deliberately unresolved work

The following remain open and must not be inferred as accepted implementation details:

- full exhaustive `FND-ID-01` identifier catalogue/issuer matrix;
- NodeId bootstrap/generator/registration/attestation/heartbeat contract;
- exact InstanceId lifecycle across recovery, abandonment and recreation;
- Party lifecycle, revisions, membership/invitation/role mechanics;
- Character creation/rename/delete/restore product rules beyond the identity invariants above;
- AccountId/CharacterId projection API and synchronization mechanism with Platform;
- world-transfer settlement, economy/social/house/guild consequences;
- future Bazaar/marketplace ownership-transfer contract;
- GameSessionId, AdmissionId and CharacterLeaseId ownership — `FND-04`;
- exact protocol framing, IDL, serialization, byte order, session-handle widths and capability negotiation — `FND-02`;
- runtime ownership, threading, handle widths, snapshot execution and process topology details — `FND-03`;
- exact PostgreSQL keys/indexes/partitioning/tombstones/retention — durability programme;
- public-ID/opaque-reference policy and field-level privacy catalogue;
- generator clock-regression implementation/conformance details;
- implementation code of any kind.

## 16. Ordered programme effect

This checkpoint does not change the foundation ordering.

The active architecture sequence remains:

1. complete `FND-ID-01` identifier catalogue and ownership/lifecycle semantics;
2. `FND-02` native `protocol-oteryn` contract;
3. `FND-03` runtime ownership/process/channel/instance execution contracts;
4. `FND-04` Game Session/admission/lease/handoff/fencing contract;
5. durability, analytics, gameplay-domain and content contracts in their accepted programme order.

The client and server remain separate architectural responsibilities even when stored in the same canonical repository.

## 17. Implementation authorization status

No runtime implementation is authorized by this checkpoint.

Specifically, this document does not authorize:

- Rust type/crate implementation;
- protocol schema/transport changes;
- database DDL/migrations;
- Platform WWW changes;
- Oteryn-Platform repository writes;
- Game Gateway/World Registry implementation changes;
- Party/Character/Instance service implementation;
- orchestrator behavior;
- production rollout.

Architecture documentation writes that record owner-accepted decisions remain allowed under the current work mode.

## Recovery summary

If future work must reconstruct the accepted state quickly, the minimum semantic model is:

```text
Platform AccountId
    -> authorized ownership projection -> CharacterId[]

CharacterId
    global UUIDv7, game-domain Character authority
    stable through rename and world transfer

WorldId
    UUIDv7, Platform World Registry/topology authority

ChannelRef
    WorldId + ChannelId(UUIDv7)
    topology identity; runtime authority additionally fenced

NodeId
    UUIDv7 per GameNode process incarnation

InstanceRef
    WorldId + InstanceId(UUIDv7)
    issued by game-domain Instance/Activity allocator

PartyRef
    WorldId + PartyId(UUIDv7)
    issued by world-level game-domain Party/Social authority

protocol-oteryn
    sole target production game protocol
    historical Platform native contract = reconciliation input only
```

These identities describe who/what an entity is. They do not replace current session, membership, placement, revision, lease, admission or ownership authority checks.
