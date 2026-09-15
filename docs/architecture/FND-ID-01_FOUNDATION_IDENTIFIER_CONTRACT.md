# FND-ID-01 — Foundation Identifier Contract

- Status: Candidate for owner acceptance
- Date: 2026-08-07
- Gate: `FND-ID-01`
- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Repository: `blakinio/Oteryn-v2`
- Applies to: identifier semantics required by `FND-02`, `FND-03` and `FND-04`
- Does not authorize: Rust/runtime/protocol/persistence/Platform/database implementation

## 1. Purpose

This contract closes the minimum cross-boundary foundation identifier gate for Oteryn v2.

Its purpose is to give the native client, authoritative Rust server, Platform control plane and later protocol/runtime/admission contracts one unambiguous semantic vocabulary for identities that must already be understood before `FND-02`, `FND-03` and `FND-04` can be frozen safely.

The contract deliberately does **not** attempt to catalogue every future gameplay, economy, analytics, content, social or operations identifier.

The governing rule is:

```text
FND-ID-01
    = minimum cross-boundary identity contract needed by the foundation
    != whole-game identifier registry
```

## 2. Authority and source precedence

This contract consolidates and consumes the owner-accepted decisions in:

- ADR-0001 through ADR-0011;
- `FND-ID-01_MINIMUM_CROSS_BOUNDARY_SCOPE_OWNER_BASELINE.md`;
- `FND-ID-01_OWNER_ACCEPTED_BASELINE.md`;
- `UUIDV7_DURABLE_IDENTITY_OWNER_BASELINE.md`;
- `FND-ID-01_WORLD_CHANNEL_ID_ISSUANCE_OWNER_BASELINE.md`;
- `FND-ID-01_WORLD_CHANNEL_UUIDV7_REPRESENTATION_OWNER_BASELINE.md`;
- `FND-ID-01_NODE_ID_PROCESS_INCARNATION_OWNER_BASELINE.md`;
- `FND-ID-01_INSTANCE_ID_ISSUER_OWNER_BASELINE.md`;
- `FND-ID-01_PARTY_ID_ISSUER_OWNER_BASELINE.md`;
- `FND-ID-01_CHARACTER_ID_ACCOUNT_LINK_OWNER_BASELINE.md`;
- `FND-ID-01_GAME_SESSION_ID_OWNER_ISSUER_BASELINE.md`;
- `FND-ID-01_GAME_SESSION_RECONNECT_GENERATION_OWNER_BASELINE.md`;
- `FND-ID-01_ACCOUNT_SINGLE_ONLINE_CHARACTER_OWNER_BASELINE.md`;
- `INSTANCE_SCOPE_AND_RUNTIME_OWNER_BASELINE.md`.

Where an earlier unresolved-items list conflicts with a later owner-accepted baseline consumed here, the later accepted decision wins for that exact subject.

This contract does not silently supersede unrelated ADR semantics or downstream lifecycle contracts.

## 3. Identity and ordering classes

Oteryn foundation uses four distinct classes.

### 3.1 Durable cross-boundary identity

Identifies one semantic entity across process/service/protocol/persistence/recovery boundaries.

Properties:

- immutable for the defined semantic lifetime;
- never reused for another semantic entity;
- strongly typed;
- independently validated at every trust boundary;
- does not encode mutable business state;
- does not itself grant authorization.

### 3.2 Scoped identity

Identifies one semantic entity only together with an explicit owner scope.

The scope participates in canonical validation and comparison even if the physical ID is globally collision-resistant.

Foundation examples:

```text
ChannelRef  = WorldId + ChannelId
InstanceRef = WorldId + InstanceId
PartyRef    = WorldId + PartyId
HandoffRef  = WorldId + HandoffId
```

### 3.3 Runtime/session-local handle

A compact reference valid only inside a named runtime, allocation, snapshot or negotiated session context.

It may improve cache locality and bandwidth, but it never replaces canonical durable identity outside the owning context.

### 3.4 Ordering, revision and fencing value

A generation, sequence, epoch, revision or tick describes order/version/current authority inside an explicit scope. It is not an entity identity.

A generation can fence an identity; it cannot replace it.

## 4. Minimum canonical identifier catalogue

The final foundation catalogue is:

| Type | Identity class | Canonical semantic scope | Logical owner | Logical issuer | Canonical representation | Default visibility |
|---|---|---|---|---|---|---|
| `AccountId` | externally owned durable identity | Platform account | Oteryn Platform Identity | Oteryn Platform Identity | authoritative Platform representation, losslessly wrapped | restricted/internal cross-boundary |
| `CharacterId` | durable identity | global character | game-domain Character authority | game-domain Character authority | strongly typed UUIDv7, full 128 bits | internal; public only by explicit product contract |
| `WorldId` | durable identity | global logical world | Platform World Registry / topology authority | Platform World Registry / topology authority | strongly typed UUIDv7, full 128 bits, subject to coordinated Platform adoption/migration | controlled cross-boundary |
| `ChannelId` | scoped durable identity | `WorldId + ChannelId` | Platform World Registry / topology authority | Platform World Registry / topology authority | strongly typed UUIDv7, full 128 bits | restricted placement identity |
| `NodeId` | durable process-incarnation identity | one GameNode process incarnation | game-domain GameNode lifecycle boundary | authenticated GameNode bootstrap for that incarnation | strongly typed UUIDv7, full 128 bits | internal operations/runtime |
| `InstanceId` | scoped durable identity | `WorldId + InstanceId` | game-domain Instance/Activity authority | game-domain Instance/Activity allocator | strongly typed UUIDv7, full 128 bits | restricted placement/gameplay identity |
| `PartyId` | scoped durable identity | `WorldId + PartyId` | world-level game-domain Party/Social authority | world-level game-domain Party/Social authority | strongly typed UUIDv7, full 128 bits | internal/social contract only |
| `GameSessionId` | durable logical-session identity | global logical gameplay session | game-domain Game Session / Admission authority | game-domain Game Session / Admission authority after successful admission | strongly typed UUIDv7, full 128 bits | restricted security/runtime identity |
| `HandoffId` | scoped durable transition identity | `WorldId + HandoffId` | game-domain ownership-transition coordination boundary | authoritative coordinator opening one ownership transition | strongly typed UUIDv7, full 128 bits | internal runtime/admission/audit |

No raw UUID type is interchangeable with another semantic type in domain APIs merely because the physical representation is the same.

## 5. `AccountId`

### 5.1 Semantic identity

`AccountId` identifies one Platform-owned account identity.

Oteryn v2 consumes this identity; it does not create a competing account namespace.

### 5.2 Owner and issuer

Both canonical ownership and issuance remain with Oteryn Platform Identity under ADR-0003.

The game domain may store and validate account bindings required for character/session rules, but storage or caching does not transfer identity ownership.

### 5.3 Representation

`FND-ID-01` does **not** silently re-key `AccountId` to UUIDv7.

The canonical value must be preserved losslessly according to the authoritative Platform identity contract and wrapped as a distinct semantic `AccountId` type inside Oteryn v2.

A later coordinated Platform contract may adopt a different canonical representation only through an explicit migration with one-to-one mapping, validation and rollback evidence.

### 5.4 Authority and privacy

Knowledge of `AccountId` is not authentication and grants no character visibility or gameplay authority.

Account-to-character linkage is restricted data. Public/social surfaces must not expose alternate-character ownership merely because they can correlate AccountId internally.

## 6. `CharacterId`

### 6.1 Representation and scope

```text
CharacterId = strongly typed UUIDv7, full 128 bits
CharacterRef = CharacterId
```

`CharacterId` is globally unique and is not semantically scoped by `WorldId`.

### 6.2 Owner and issuer

The authoritative game-domain Character authority owns and issues `CharacterId`.

### 6.3 Lifecycle

- rename preserves CharacterId;
- legal world transfer preserves CharacterId;
- current account ownership may change only through an explicit audited product contract;
- deletion/retirement never returns CharacterId for reuse;
- recreation with the same name creates a fresh CharacterId;
- nil/zero UUID is invalid.

### 6.4 Authority

CharacterId is identity only. Gameplay mutation additionally requires current session/lease/placement/fencing authority.

## 7. `WorldId`

### 7.1 Representation and scope

```text
WorldId = strongly typed UUIDv7, full 128 bits
```

It globally identifies one logical world independent from display name, slug, endpoint or infrastructure placement.

### 7.2 Owner and issuer

Platform World Registry / authoritative topology control owns and issues WorldId.

Starting a GameNode does not create a world.

### 7.3 Lifecycle

- rename does not change WorldId;
- GameNode restart/relocation does not change WorldId;
- a retired WorldId is never reused for a different logical world;
- nil/zero UUID is invalid;
- exact retirement/reactivation workflow remains a world-lifecycle/topology contract, but reactivation cannot reinterpret the ID as another world.

### 7.4 Cross-repository migration

The Oteryn-v2 target representation is UUIDv7. If current Platform data differs, production conformance requires an explicit coordinated Platform migration. Oteryn-v2 must not hash, truncate, synthesize or silently re-key the external identity.

## 8. `ChannelId`

### 8.1 Representation and canonical scope

```text
ChannelId  = strongly typed UUIDv7, full 128 bits
ChannelRef = WorldId + ChannelId
```

`ChannelId` is semantically scoped by WorldId even though its UUID value is collision-resistant globally.

### 8.2 Owner and issuer

Platform World Registry / authoritative topology control owns and issues ChannelId.

GameNodes, ChannelRuntimes, containers and orchestrators consume assignments; they do not create canonical channel identity as an infrastructure side effect.

### 8.3 Lifecycle

- restart/recovery/relocation of the same semantic channel preserves `WorldId + ChannelId`;
- changing GameNode does not change ChannelId;
- display ordinal/name does not change ChannelId;
- ChannelId is never reused for another semantic channel;
- current writer authority is fenced separately by channel ownership generation.

### 8.4 Privacy

Exact channel placement is not public presence information by default. The server enforces visibility before disclosure.

## 9. `NodeId`

### 9.1 Representation and semantic lifetime

```text
NodeId = strongly typed UUIDv7, full 128 bits
```

NodeId identifies exactly one concrete GameNode process incarnation.

Every process start/restart/replacement creates a fresh NodeId.

### 9.2 Owner

The semantic owner is the Oteryn game-domain GameNode lifecycle boundary.

NodeId is not a host, VM, pod, container, replica ordinal or placement-slot identifier.

### 9.3 Issuance closure

The canonical issuance rule is:

```text
new GameNode process incarnation
    -> bootstrap generates a fresh NodeId locally
    -> process authenticates to the accepted registration/control boundary
    -> registration validates NodeId format/uniqueness and binds it to that authenticated incarnation
    -> later channel assignment requires separate current assignment/fencing authority
```

This closes the earlier issuer ambiguity while preserving the accepted no-central-UUID-service direction.

Mandatory consequences:

- a new process must never inherit a predecessor's NodeId from a pod name, environment default, stable volume or orchestrator ordinal;
- the orchestrator may launch the process but does not define semantic NodeId identity;
- successful registration does not by itself authorize any channel mutation;
- a generated NodeId from a process that never completes trusted registration grants no system authority and may be discarded with that failed incarnation;
- duplicate/collision registration fails closed;
- exact workload authentication, registration protocol and retry mechanics remain `FND-03`/operations work.

### 9.4 Lifecycle and fencing

NodeId is immutable for the process lifetime and never reused.

A valid NodeId plus a stale channel ownership generation is still unauthorized to mutate that channel.

## 10. `InstanceId`

### 10.1 Representation and scope

```text
InstanceId  = strongly typed UUIDv7, full 128 bits
InstanceRef = WorldId + InstanceId
```

### 10.2 Owner and issuer

The authoritative game-domain Instance/Activity allocator owns the concrete instance lifecycle and issues InstanceId.

### 10.3 Lifecycle

- one concrete instance lifecycle has one immutable InstanceId;
- another occurrence of the same activity/template receives a new InstanceId;
- origin ChannelId, PartyId, template key and GameNode are not instance identity;
- recovery of the same concrete instance may preserve InstanceId while advancing instance ownership generation;
- terminated identity is never reused for another instance;
- nil/zero UUID is invalid.

### 10.4 Authority and privacy

Knowing InstanceId grants neither admission nor observation. Membership/session/fencing state is independently validated.

Exact instance placement is restricted information by default.

## 11. `PartyId`

### 11.1 Representation and scope

```text
PartyId  = strongly typed UUIDv7, full 128 bits
PartyRef = WorldId + PartyId
```

### 11.2 Owner and issuer

The world-level game-domain Party/Social authority owns and issues PartyId.

### 11.3 Lifecycle

- leadership changes do not change PartyId;
- members joining/leaving do not change PartyId;
- members may occupy different channels of one world without changing PartyId;
- instance entry does not change PartyId;
- a new party lifecycle receives a fresh PartyId;
- PartyId is never reused for another party;
- nil/zero UUID is invalid.

Exact empty-party/disband/restoration semantics belong to the later Party/Social lifecycle contract.

### 11.4 Authority

PartyId is not membership proof, invite capability or leader authority. Membership, role and party revision remain independently validated.

## 12. `GameSessionId`

### 12.1 Representation and scope

```text
GameSessionId = strongly typed UUIDv7, full 128 bits
```

It is globally unique and identifies one authoritative logical gameplay session.

### 12.2 Owner and issuer

The game-domain Game Session / Admission authority owns and issues GameSessionId **only after successful authoritative gameplay admission**.

Platform tickets, Gateway route/session material and pre-admission credentials are not GameSessionId.

### 12.3 Lifecycle

- eligible short transport reconnect preserves the same GameSessionId;
- accepted reconnect establishes a newer transport/connection generation;
- terminal logical-session end permanently retires that GameSessionId;
- a later fresh admission receives a new GameSessionId;
- accepted channel transitions that establish a fresh logical Game Session receive a fresh GameSessionId according to the session/handoff contract;
- nil/zero UUID is invalid;
- no GameSessionId is reused.

### 12.4 Authority

GameSessionId is never a bearer credential. Knowing it does not authorize reconnect, commands, lease takeover, mutation or termination.

## 13. `HandoffId` — admitted conditional foundation identity

### 13.1 Why it belongs in FND-ID-01

The already accepted instance/runtime architecture requires every authoritative simulation-ownership transition to have a **unique transfer identity**, be idempotent and be generation-fenced across source and destination runtime boundaries.

That requirement crosses `FND-02`, `FND-03` and `FND-04`. Leaving its semantic identity unnamed until those downstream contracts would make each contract capable of inventing a different transfer namespace and would create a circular foundation dependency.

`HandoffId` therefore satisfies the conditional-foundation test from the minimum-scope baseline.

### 13.2 Representation and scope

```text
HandoffId  = strongly typed UUIDv7, full 128 bits
HandoffRef = WorldId + HandoffId
```

It identifies exactly one bounded authoritative gameplay ownership-transition lifecycle inside one logical world.

Examples include:

- ChannelRuntime -> InstanceRuntime;
- InstanceRuntime -> ChannelRuntime;
- a later accepted ChannelRuntime -> ChannelRuntime ownership handoff.

It is **not** the identity of a marketplace character transfer, world-transfer purchase, trade transaction or generic business workflow.

### 13.3 Owner and issuer

The semantic owner is the game-domain ownership-transition coordination boundary.

The authoritative coordinator that opens one transition issues the HandoffId before source/destination commit work can become externally ambiguous.

Logical ownership does not require a standalone microservice. The coordination module may later be co-located according to `FND-03`/`FND-04` while preserving the same identity semantics.

### 13.4 Lifecycle and idempotency

- retries/resume of the **same** logical handoff reuse the same HandoffId;
- a new handoff attempt/lifecycle receives a new HandoffId;
- commit, abort or terminal reconciliation ends the active lifecycle but the ID may remain in durable audit/recovery evidence;
- HandoffId is never reused for another transition;
- source/destination runtime identities, current ownership generations, CharacterId and GameSessionId remain explicit bindings rather than encoded into the UUID;
- nil/zero UUID is invalid.

### 13.5 Authority

HandoffId is correlation/identity, not authority. A stale source cannot mutate merely because it knows a current HandoffId. Current session, lease and source/destination ownership generations remain authoritative fences.

Exact prepare/commit/abort states, token shape, transport, durability and recovery transaction belong to `FND-03`, `FND-04` and `DUR-*`.

## 14. Identifiers deliberately not admitted now

### 14.1 No `AdmissionId` yet

A distinct durable `AdmissionId` is **not** added to the minimum foundation catalogue.

The current architecture can distinguish:

- Platform ticket/pre-admission material;
- one admission state machine;
- resulting GameSessionId after success;
- HandoffId where an ownership transition has a separately addressable cross-runtime lifecycle.

`FND-04` may request a narrow FND-ID amendment only if its final state machine proves that an admission attempt itself must be an independently addressable durable semantic entity for idempotency/recovery/audit and cannot be represented safely by existing credential/request/state-transition identities.

Do not invent AdmissionId merely because an admission process exists.

### 14.2 No `CharacterLeaseId` yet

A distinct durable `CharacterLeaseId` is also not added now.

The accepted foundation requires current character/session/account exclusion and lease **authority**, but it does not yet prove that the lease requires its own durable entity identity rather than:

```text
CharacterId
+ GameSessionId where applicable
+ current lease/session generation or fencing value
+ authoritative lease state
```

`FND-04` may request a narrow amendment if an independently addressable lease entity is demonstrably required.

### 14.3 No foundation `CommandId`

`CommandId` belongs to `FND-02` because its representation, scope, retry/idempotency behavior and sequencing are protocol-command mechanics.

### 14.4 Later-domain identifiers remain deferred

The following are not added by this foundation contract merely because they may eventually use UUIDv7:

- item/container identities;
- trades, market offers, mail and reward settlements;
- analytics/audit events and security cases;
- quests, boss attempts, lockouts and dynamic event occurrences;
- guilds, houses and social invitation entities;
- content definitions, tiles, chunks, zones and asset identifiers;
- deployment/placement/host identities.

They remain owned by their later bounded contracts.

## 15. Foundation fencing and revision vocabulary

The following values are explicitly **not entity identities**.

They are class-4 ordering/revision/fencing values and use the accepted unsigned 64-bit default unless an owning later contract proves another representation is necessary.

### 15.1 `connection_generation`

Scope:

```text
GameSessionId + connection_generation
```

Semantics:

- advances/replaces when an eligible logical session binds a newer authoritative transport;
- older generations lose command authority after the new binding commits;
- delayed packets from old generations cannot regain authority;
- no silent wraparound;
- exact increment/commit/wire mechanics belong to `FND-02`/`FND-04`.

### 15.2 channel ownership generation

Scope:

```text
WorldId + ChannelId + channel_ownership_generation
```

Semantics:

- identifies current mutation-authority epoch for one channel;
- changes on accepted ownership transfer/recovery as required;
- stale GameNodes fail closed even when they hold the correct WorldId/ChannelId;
- no silent wraparound;
- exact allocator/persistence/renewal mechanics belong to `FND-03`, `DUR-02` and operations contracts.

### 15.3 instance ownership generation

Scope:

```text
WorldId + InstanceId + instance_ownership_generation
```

Semantics mirror channel ownership fencing for one concrete instance lifecycle.

### 15.4 party revision

A party state revision is scoped to `WorldId + PartyId` and may guard stale membership/leadership mutations. It is not party identity.

Exact optimistic-concurrency and mutation semantics remain with the later Party/Social contract.

### 15.5 Additional future fences

If `FND-04` introduces an account-exclusion generation, character-lease generation or equivalent fencing value, it remains a class-4 scoped authority value unless the contract proves a separately addressable semantic entity requiring an FND-ID amendment.

## 16. Common representation rules

For every Oteryn-owned UUIDv7 identity admitted by this contract:

- all 128 bits are canonical and must be preserved losslessly;
- nil/zero UUID is invalid;
- absence is explicit, never a magic UUID;
- identifiers are never reused for another semantic entity;
- collision/duplicate creation fails closed and must never overwrite an existing entity;
- UUIDv7 timestamp ordering is never proof of causality, freshness, authority, lease ownership or gameplay order;
- raw UUID interchange between semantic types is prohibited at domain/API boundaries;
- names, slugs, ordinals, row positions and infrastructure IDs are labels/metadata, not replacements for canonical identity.

Exact binary byte order, IDL representation and textual formatting remain `FND-02`/interchange-contract work.

## 17. Scope validation rules

Scoped identities must cross durable/process/trust boundaries with enough authoritative context to prove their scope.

Mandatory examples:

```text
ChannelRef  requires validated WorldId + ChannelId
InstanceRef requires validated WorldId + InstanceId
PartyRef    requires validated WorldId + PartyId
HandoffRef  requires validated WorldId + HandoffId
```

A consumer must not infer scope solely from:

- current UI selection;
- current socket;
- thread-local/global mutable state;
- database connection/schema;
- cached route;
- current coordinates;
- process placement.

Malformed, nil, unknown, wrong-type or wrong-scope identifiers fail closed at trust boundaries.

## 18. Identity is not authorization

Possession of any identifier in this contract grants no capability by itself.

Examples:

- `AccountId` is not authentication;
- `CharacterId` is not character ownership proof;
- `WorldId`/`ChannelId` are not routing authority;
- `NodeId` is not registration or channel authority;
- `InstanceId` is not admission;
- `PartyId` is not membership;
- `GameSessionId` is not a reconnect credential;
- `HandoffId` is not permission to commit transfer.

Security-sensitive mutation requires independently validated credentials, membership/session/lease state and current generations/revisions appropriate to the operation.

## 19. Runtime-local and session-local handles

Canonical durable UUIDs must not become a hot-loop requirement for every entity reference.

### Runtime handles

`FND-03` may define compact generational runtime handles for transient entities/tasks/components.

Required invariant:

```text
runtime handle validity <= owning runtime/allocation generation
```

A runtime handle is never the sole durable/cross-process identity.

### Session handles

`FND-02`/`FND-04` may map canonical identities to compact session-local handles after an authoritative mapping/snapshot is established.

Required invariants:

- mapping is scoped to one validated session/context/epoch;
- stale mappings are explicitly invalidated;
- reconnect/handoff either proves safe continuity or establishes a fresh mapping from authoritative state;
- durable audit/persistence retains canonical identities where required.

## 20. Visibility and privacy defaults

Global uniqueness does not imply public visibility.

Default classifications are:

- `AccountId` — restricted identity/security data;
- `CharacterId` — internal durable identity; public product surfaces should prefer name/slug/opaque public reference unless a contract requires raw ID;
- `WorldId` — controlled product/control-plane identity; human-facing world names remain labels;
- `ChannelId` — restricted placement information under social-presence policy;
- `NodeId` — internal operations/diagnostics identity;
- `InstanceId` — restricted placement/encounter identity;
- `PartyId` — internal/social relationship identity;
- `GameSessionId` — restricted security/runtime correlation identity;
- `HandoffId` — internal runtime/admission/recovery identity.

Logs, traces and analytics may correlate canonical IDs only under their access/retention/privacy contracts. High-cardinality player/session IDs must not be exported casually as ordinary Prometheus labels.

## 21. Legacy/external mapping rules

A legacy/external identifier is never silently converted into a new canonical identity.

When an authoritative migration is required:

- exactly one authority owns the mapping;
- mapping is one-to-one where semantic identity is preserved;
- collisions and ambiguous mappings fail closed;
- backfill and rollback are explicit;
- historical provenance is retained;
- mixed-version nodes reject representations they cannot validate safely;
- aliases/migration keys do not silently remain canonical after cutover.

This is especially important for Platform-owned AccountId/WorldId/ChannelId boundaries.

## 22. Downstream contract boundaries

### `FND-02` owns protocol mechanics

`FND-02` consumes this semantic catalogue and owns:

- wire field placement;
- framing and transport;
- IDL/serialization;
- canonical UUID byte order/text form;
- `CommandId` and command/message sequencing;
- compact session-handle encoding;
- snapshot/delta/reconciliation representation;
- protocol capabilities/version negotiation;
- protocol resource limits and errors.

### `FND-03` owns runtime mechanics

`FND-03` owns:

- exact runtime handle representation;
- ChannelRuntime/InstanceRuntime scheduling;
- ownership-generation allocation/transition mechanics;
- Node registration/liveness/runtime lifecycle;
- worker/task handles;
- runtime recovery and stale-result rejection.

### `FND-04` owns admission/session/lease behavior

`FND-04` owns:

- pre-admission credential/ticket validation;
- logical Game Session creation state machine;
- character/account exclusion and lease lifecycle;
- reconnect credential and transport rebind state machine;
- takeover/revocation/terminal session states;
- Handoff prepare/commit/abort semantics where admission/session ownership is involved;
- any evidence-based request for a later `AdmissionId` or `CharacterLeaseId` amendment.

### `DUR-*` owns physical persistence

Durability work owns:

- PostgreSQL keys/indexes/partitions;
- migration DDL;
- tombstone/retention implementation;
- persistence-local surrogate/index optimizations;
- transaction/outbox storage;
- durable handoff/session/lease evidence where required.

It may optimize physical storage but cannot redefine canonical semantic identity.

## 23. Cross-repository Platform boundary

Oteryn Platform remains external and independently governed.

This contract establishes the Oteryn-v2 side of required identity semantics but does not authorize Platform writes.

Before production cross-repository conformance is claimed, coordinated Platform work must reconcile at least:

- AccountId representation/validation contract;
- WorldId/ChannelId UUIDv7 target adoption or explicit migration;
- World Registry issuer/registry ownership;
- pre-admission material terminology versus canonical GameSessionId;
- exact representation/version compatibility consumed later by `FND-02`/`FND-04`.

The older merged Platform native gameplay protocol contract remains reconciliation evidence only and cannot override this identifier contract where it conflicts with later accepted Oteryn-v2 semantics.

## 24. Required invariants for later implementation

Future implementation and E2E must eventually prove at minimum:

1. semantic strong types cannot be substituted accidentally at public/domain boundaries;
2. nil/zero canonical UUID identities are rejected;
3. scoped identities reject wrong-world combinations;
4. no canonical ID is reused for another semantic entity;
5. UUIDv7 timestamp order never determines gameplay authority or causal ordering;
6. CharacterId survives rename and accepted world transfer while stale world authority is fenced;
7. ChannelId survives GameNode restart/relocation while old ownership generation is rejected;
8. every GameNode process incarnation uses a fresh NodeId and registration does not confer channel authority;
9. InstanceId survives same-instance recovery but is not reused for a later activity occurrence;
10. PartyId survives leader/member placement changes without becoming cross-world;
11. GameSessionId survives eligible transport reconnect while stale connection generations lose authority;
12. failed/pre-admission attempts do not create canonical GameSessionId;
13. one ownership handoff reuses one HandoffId across retries while a distinct handoff uses a new ID;
14. HandoffId alone cannot commit or authorize transfer;
15. AccountId is never silently re-keyed by Oteryn-v2;
16. compact handles cannot escape their validated runtime/session scope as canonical identity;
17. unauthorized social/public consumers cannot infer exact ChannelId/InstanceId/NodeId placement from identifier availability.

## 25. Explicit non-decisions

This contract intentionally does not decide:

- final Rust crate/type/module names;
- UUID library implementation;
- exact UUIDv7 clock-regression algorithm;
- binary byte order or IDL;
- canonical textual UUID formatting;
- protocol framing/transport/TLS;
- `CommandId` representation;
- exact generation increment transaction or persistence;
- heartbeat/liveness representation;
- reconnect token construction;
- full Game Session/lease state machine;
- Handoff prepare/commit/abort transaction schema;
- PostgreSQL indexes/partitions;
- public URL/slug design;
- future later-domain identifier catalogues.

Those decisions remain with their owning gates.

## 26. Gate result

When this candidate is owner-accepted, independently audited on the exact final head, passes repository governance and is merged, `FND-ID-01` is complete at the architecture-contract level.

The next ordered foundation gate is then:

```text
FND-02 — canonical protocol-oteryn contract
```

`FND-02` must consume this contract and the accepted Platform protocol-reconciliation baseline. Acceptance of `FND-ID-01` does not authorize protocol/runtime implementation by itself.
