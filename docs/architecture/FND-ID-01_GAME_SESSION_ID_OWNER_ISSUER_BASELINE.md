# FND-ID-01 GameSessionId Owner and Issuer Baseline

- Status: Owner-accepted foundation baseline
- Date: 2026-08-07
- Decision owner: Oteryn project owner
- Gate: `FND-ID-01`
- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Applies to: `GameSessionId` semantic identity, ownership and issuer boundary
- Related: `ADR-0003-platform-identity-game-gateway-and-admission-boundary.md`, `FND-ID-01_MINIMUM_CROSS_BOUNDARY_SCOPE_OWNER_BASELINE.md`, `FND-ID-01_GAME_SESSION_RECONNECT_GENERATION_OWNER_BASELINE.md`, `FND-ID-01_ACCOUNT_SINGLE_ONLINE_CHARACTER_OWNER_BASELINE.md`

## Purpose

Freeze the minimum `GameSessionId` identity semantics required by the native Platform -> Gateway -> game-domain admission boundary.

The decision separates:

1. **authorization to attempt gameplay admission**, which belongs to Platform/Gateway boundaries; from
2. **existence of an authoritative logical gameplay session**, which belongs to the game domain.

This is architecture only. It does not authorize Rust, protocol, persistence, Platform, database, deployment or production implementation.

## Owner-accepted decision

`GameSessionId` is owned and logically issued by the **game-domain Game Session / Admission authority** after successful authoritative gameplay admission.

Platform does not issue the canonical gameplay-session identity.

The intended trust boundary is:

```text
Platform Identity / Game Login Ticket
    -> proves identity and bounded authorization to attempt admission

Game Gateway / World Registry
    -> validates/routs according to Platform-owned contracts

Game Session / Admission authority
    -> evaluates game-domain admission requirements
    -> establishes an authoritative logical gameplay session
    -> issues GameSessionId

ChannelRuntime
    -> receives admitted authoritative gameplay ownership
    -> executes simulation
```

The distinction is intentional: a valid Platform authorization can exist even when no gameplay session is ultimately admitted.

## Canonical representation

The accepted representation is:

```text
GameSessionId = strongly typed UUIDv7, full 128 bits
```

Rules:

- `GameSessionId` is a durable cross-boundary logical-session identity for the lifetime of one admitted logical gameplay session;
- all 128 bits are preserved across language/process/persistence/protocol adapters that carry the canonical ID;
- nil/zero UUID is invalid as a canonical `GameSessionId`;
- values are never reused for another logical gameplay session;
- UUIDv7 timestamp ordering is not authority, freshness, causality, lease ownership or transport-generation evidence;
- raw interchangeable UUIDs are not the domain model; the semantic type remains `GameSessionId`.

## Scope and uniqueness

`GameSessionId` is globally unique as a canonical identity.

Its semantic identity is:

```text
GameSessionId
```

not:

```text
WorldId + GameSessionId
```

`WorldId`, `ChannelId`, `AccountId`, `CharacterId` and runtime authority remain required session bindings/context, but they do not form the identity key of `GameSessionId` itself.

This avoids making session identity dependent on current placement or current topology while retaining explicit validation of every bound scope.

## Canonical owner

The canonical semantic owner is:

```text
Game Session / Admission authority in the game domain
```

This owner controls the logical session lifecycle boundary.

Logical ownership does **not** require a standalone microservice.

An initial implementation may be co-located inside the authoritative Rust game-server deployment while preserving the logical module/domain boundary. Future extraction into another process or service must not change the semantics of `GameSessionId`.

Therefore:

```text
logical owner != mandatory deployment unit
```

## Logical issuer

The logical issuer/generator authority for new `GameSessionId` values is the game-domain Game Session / Admission authority.

A new canonical `GameSessionId` is created only for an admission that becomes an authoritative logical gameplay session according to the accepted admission contract.

The following are **not** issuers:

- Platform Identity;
- OAuth/PKCE boundary;
- Game Login Ticket issuer;
- Game Gateway;
- World Registry;
- ChannelRuntime;
- GameNode transport/network layer;
- host/container/pod;
- external orchestrator;
- client.

A client-supplied or Platform-supplied session-looking value cannot establish canonical gameplay-session identity unless a future accepted migration contract explicitly says otherwise.

## Platform authorization versus gameplay session

Platform may issue credentials/grants/tickets that allow a bounded admission attempt.

Those artifacts are not `GameSessionId`.

The conceptual sequence is:

```text
Account authenticated
    -> Platform authorization / Game Login Ticket
    -> Gateway routing
    -> game-domain admission evaluation
    -> authoritative session admitted
    -> GameSessionId exists
```

This prevents pre-admission or failed-admission attempts from being treated as completed gameplay sessions.

It also keeps Platform from becoming the owner of game-domain lifecycle state such as character leases, duplicate-login arbitration, reconnect continuity or session termination.

## Required session bindings

A live logical gameplay session must be bound to at least the authoritative identities/scopes required by the surrounding accepted architecture, including conceptually:

- `AccountId`;
- `CharacterId`;
- `WorldId`;
- current `ChannelId` or other accepted current gameplay placement context;
- current character/session lease or equivalent mutation-authority fence;
- current transport/connection generation;
- applicable protocol/ruleset/content/map/schema/server revision fences required by accepted contracts.

Exact storage structs and field layouts are deliberately not frozen here.

A binding change does not automatically imply a new session identity unless the accepted lifecycle contract defines that change as a new logical session.

## Identity is not authority

`GameSessionId` is never a credential.

Knowledge or possession of a valid `GameSessionId` alone must not authorize:

- reconnect;
- gameplay commands;
- character mutation;
- channel admission;
- lease takeover;
- session recovery;
- inventory/economy mutation;
- session termination.

Authoritative mutation requires the current accepted combination of credentials, lease/session state, generation/revision fences and runtime ownership.

In particular:

```text
GameSessionId != bearer token
GameSessionId != character lease
GameSessionId != connection_generation
GameSessionId != current channel ownership generation
```

## Reconnect continuity

The existing reconnect baseline remains binding.

For an eligible short reconnect of the same logical session:

```text
GameSessionId = unchanged
transport/connection generation = advanced
```

Example:

```text
GameSessionId S
connection_generation 1
    -> transport lost
GameSessionId S
connection_generation 2
    -> transport lost
GameSessionId S
connection_generation 3
```

Older transport generations lose command authority once superseded according to the accepted fencing contract.

Reconnect must not create a new logical gameplay identity merely because a socket or transport instance changed.

## Terminal end and fresh sessions

Once a logical gameplay session reaches a terminal end, its `GameSessionId` is never reused or resurrected as another gameplay session.

A later fresh admission receives a new `GameSessionId`.

Conceptually:

```text
GameSessionId A -> terminal
later admission -> GameSessionId B
```

The accepted multichannel architecture also treats a channel change as a session transition rather than an in-place protocol adapter/teleport switch. Where that transition creates a fresh Game Session under the accepted channel-switch contract, the destination session receives a fresh `GameSessionId`.

The exact channel-switch/handoff transaction sequence remains owned by the relevant admission/runtime contracts.

## ChannelRuntime and GameNode boundary

`ChannelRuntime` is not the semantic owner or issuer of `GameSessionId`.

A channel executes authoritative simulation after admission, but session identity must survive distinctions among:

- logical session;
- transport connection;
- GameNode process incarnation;
- channel recovery/relocation;
- runtime ownership generation.

Likewise, `NodeId` identifies one GameNode process incarnation and must not be conflated with logical gameplay-session identity.

This allows process restart/recovery mechanisms to reason about session continuity without redefining the identity model around current infrastructure placement.

## Crash and partial-admission safety direction

The architecture requires the future admission implementation to avoid creating two active authoritative sessions or leaving an ambiguous authoritative session after partial failure.

However, this baseline deliberately does **not** freeze the exact atomic point at which `GameSessionId` becomes committed relative to:

- character lease acquisition;
- duplicate-login arbitration;
- session-row persistence;
- outbox/audit emission;
- destination ChannelRuntime readiness;
- transport binding;
- reconnect credential issuance.

That exact state machine and commit boundary belong to `FND-04` and the durability contracts.

The future contract must preserve these invariants:

1. a failed admission must not become an active authoritative gameplay session;
2. crash/retry must be idempotent or fenced;
3. no character may gain two simultaneous authoritative mutation sessions;
4. stale admission/session attempts cannot overwrite a newer accepted session;
5. externally observable success must correspond to one unambiguous authoritative session state.

## AdmissionId remains unresolved

This decision does not introduce `AdmissionId` merely because an admission process exists.

A distinct `AdmissionId` may be added to the minimum foundation catalogue only if the final `FND-04` design proves that it is a separately addressable cross-boundary semantic entity required for idempotency, recovery, observability or fencing and cannot safely be represented by an existing identifier/state transition.

If not required, it should not be invented.

The same rule applies to candidate `CharacterLeaseId`, handoff IDs and other possible foundation identifiers.

## Protocol boundary

`FND-ID-01` freezes semantic identity, not wire mechanics.

`FND-02` remains responsible for:

- field/message placement;
- binary representation and byte order;
- framing;
- transport encoding;
- capabilities/version negotiation;
- sequencing/command identifiers;
- compact session-local handle design if used;
- downgrade/fallback behavior.

Any compact wire/session handle that represents a `GameSessionId` is scoped and replaceable; it does not become the canonical durable identity.

## Persistence boundary

This decision does not freeze:

- PostgreSQL table layout;
- indexes;
- partitioning;
- retention;
- tombstones;
- durable session-row lifecycle;
- recovery journal implementation;
- outbox schema.

Those belong to `FND-04`/`DUR-*` as appropriate.

Persistence must preserve the canonical identity and accepted fencing/lifecycle semantics rather than redefining them.

## Privacy and visibility

`GameSessionId` is internal/cross-boundary infrastructure identity by default.

Global uniqueness does not imply that it should be exposed publicly to players, URLs, logs, analytics exports or third parties without a dedicated visibility/privacy decision.

Diagnostic and audit systems may correlate it where authorized, but public exposure should use purpose-specific opaque references when appropriate.

## Consequences

### Positive

- Platform authorization and actual gameplay-session existence remain cleanly separated;
- failed admission does not conceptually create a real gameplay session;
- ownership follows game-domain lifecycle semantics rather than deployment topology;
- reconnect can preserve logical session continuity while fencing stale transports;
- GameNode/channel recovery can be designed without redefining session identity;
- duplicate-login and anti-dupe contracts have one clear logical session identity;
- future protocol/persistence implementations receive an unambiguous semantic anchor.

### Costs

- `FND-04` must explicitly define admission/session creation and failure transaction semantics;
- Platform and game-domain contracts must distinguish authorization artifacts from the resulting `GameSessionId`;
- observability must correlate attempts, tickets and admitted sessions without pretending they are the same entity;
- channel transitions and recovery flows must be explicit about whether they preserve or replace the logical Game Session.

## Explicit non-decisions

Still unresolved after this baseline:

- exact atomic admission commit point for `GameSessionId` creation;
- whether `AdmissionId` exists;
- whether `CharacterLeaseId` exists;
- exact `connection_generation` representation/width;
- exact reconnect credential form;
- exact terminal session-state vocabulary;
- admission persistence/outbox transaction shape;
- exact channel-switch/handoff protocol;
- wire encoding/byte order and compact handles;
- public/opaque external session-reference design;
- implementation/deployment topology of the logical Game Session / Admission authority.

## Final owner-accepted rule

```text
Platform may authorize and route an attempt to enter the game.

Only the game-domain Game Session / Admission authority establishes
that an authoritative logical gameplay session exists and issues its
canonical GameSessionId.

GameSessionId is a strongly typed globally unique UUIDv7 identity.
It survives eligible transport reconnects, never acts as a credential,
and is replaced after terminal session end / a new logical session.
```
