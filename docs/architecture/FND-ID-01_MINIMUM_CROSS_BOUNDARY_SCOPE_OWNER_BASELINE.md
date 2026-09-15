# FND-ID-01 Minimum Cross-Boundary Scope Owner Baseline

- Status: Owner-accepted pre-contract baseline
- Date: 2026-08-07
- Decision owner: Oteryn project owner
- Gate: `FND-ID-01`
- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Applies to: completion of the foundation identifier contract before `FND-02`, `FND-03` and `FND-04`
- Related: `FND-ID-01_OWNER_ACCEPTED_BASELINE.md`, `UUIDV7_DURABLE_IDENTITY_OWNER_BASELINE.md`, `FND-ID-01_OWNER_DECISION_CHECKPOINT_2026-08-07.md`, `FOUNDATION_PROGRAMME_CURRENT_STATUS.md`

## Purpose

Record the owner-accepted scope boundary for the final `FND-ID-01` contract.

The complete foundation identifier contract must be large enough to make the native protocol, authoritative runtime and admission/session contracts unambiguous, but it must **not** become an exhaustive catalogue of every identifier that the full Oteryn game, economy, analytics, content system or operations platform may ever require.

This is architecture only. It does not authorize Rust, protocol, runtime, persistence, Platform, database or production implementation.

## Owner-accepted decision

The final `FND-ID-01` contract is a **minimum cross-boundary foundation identifier contract**.

Its purpose is:

```text
freeze the semantic identity vocabulary required by the foundation
    -> so FND-02 can define protocol fields safely
    -> so FND-03 can define runtime ownership and handles safely
    -> so FND-04 can define admission/session/lease fencing safely
```

It is explicitly **not**:

```text
an exhaustive inventory of every future gameplay, economy, persistence,
analytics, content, social, LiveOps or tooling identifier
```

A later domain must not be pulled into `FND-ID-01` merely because that domain will eventually use UUIDv7 or another strongly typed identifier.

## Minimum catalogue included by foundation scope

The final `FND-ID-01` contract must cover the identities already required to interpret accepted foundation boundaries across repositories, processes, sessions or authoritative runtime scopes.

The minimum catalogue includes:

- `AccountId` — externally owned Platform Identity anchor; Oteryn-v2 consumes the authoritative Platform identity and does not silently re-key it;
- `CharacterId` — global durable game-domain character identity;
- `WorldId` — logical-world identity;
- `ChannelId` — channel identity semantically scoped by `WorldId`;
- `NodeId` — one concrete GameNode process-incarnation identity;
- `InstanceId` — concrete instance identity semantically scoped by `WorldId`;
- `PartyId` — world-level party identity semantically scoped by `WorldId`;
- `GameSessionId` — logical admitted gameplay-session identity, distinct from the currently bound transport/connection generation.

This list is minimum foundation scope, not a claim that all listed details are already resolved. Existing detailed owner baselines remain normative for decisions already accepted. Any unresolved minimum property required to make `FND-02`, `FND-03` or `FND-04` unambiguous must be resolved by the complete `FND-ID-01` package or explicitly assigned to the downstream owning contract without creating circular dependency.

## Conditional foundation identifiers

A candidate identifier beyond the minimum list enters the final `FND-ID-01` catalogue only when all of the following are true:

1. the concept is required by an accepted foundation boundary rather than only by a later gameplay/domain feature;
2. downstream `FND-02`, `FND-03` or `FND-04` cannot define a safe public/runtime contract without knowing its semantic identity class, owner or scope first;
3. delaying the identity decision would create ambiguity, incompatible duplicate types or an authority/fencing risk;
4. `FND-ID-01` can define the identity without prematurely freezing the detailed state machine, storage model or domain rules owned elsewhere.

Examples that may be admitted conditionally if the final foundation design proves they are distinct durable semantic entities include concepts such as an admission, lease or handoff/transfer identity.

Their presence is not automatic. The complete contract must justify each addition against the four conditions above.

## Required per-identifier dimensions

For every identifier admitted to the final `FND-ID-01` catalogue, the contract freezes only the cross-boundary properties necessary for correct composition.

At minimum it must state:

1. **canonical semantic term** — one name for one concept, without silent aliases;
2. **identity class** — durable cross-boundary identity, scoped identity, runtime-local handle, or ordering/revision/fencing value;
3. **semantic entity and lifetime** — what one value identifies and when that identity begins/ends;
4. **scope and uniqueness domain** — global or explicitly scoped comparison semantics such as `WorldId + ChannelId`;
5. **authoritative owner** — which domain owns the semantic lifecycle;
6. **logical issuer/generator authority** where the identity is minted;
7. **reuse, nil/absence and collision rules**;
8. **canonical representation direction** where already required for cross-language/cross-boundary compatibility;
9. **lossless comparison/validation rules** across Rust, Platform contracts and later persistence/protocol adapters;
10. **visibility classification** — internal by default versus intentionally exposed cross-boundary/publicly;
11. **authority relationship** — which separate generation, lease, session, revision or fencing state is required before the identity can authorize mutation;
12. **legacy/external mapping rule** where another authority already owns the identity;
13. **hot-path mapping boundary** where compact runtime/session handles may replace repeated durable IDs without replacing semantic identity.

The contract must not infer ownership from the process that happens to hold a value or the database that happens to store it.

## Existing accepted decisions remain binding

This scope decision does not reopen previously accepted identifier decisions.

In particular:

- strong semantic types remain mandatory;
- adopted Oteryn-owned durable cross-boundary identities use the accepted UUIDv7 direction and preserve all 128 bits;
- nil/zero UUID is invalid for canonical entity identity;
- identity values are never reused for another semantic entity;
- UUIDv7 ordering is not gameplay causality, authority, freshness or fencing;
- canonical channel identity remains `WorldId + ChannelId`;
- canonical instance identity remains `WorldId + InstanceId`;
- canonical party identity remains `WorldId + PartyId`;
- `CharacterId` remains a global durable game-domain identity rather than `WorldId + CharacterId`;
- `NodeId` remains one GameNode process incarnation and changes on process restart/replacement;
- World Registry/topology control remains the logical issuer authority for `WorldId` and `ChannelId`;
- the game-domain Instance/Activity allocator remains the logical `InstanceId` issuer;
- the world-level game-domain Party/Social authority remains the logical `PartyId` issuer;
- `GameSessionId` remains distinct from the current transport/connection generation;
- identity never substitutes for session, lease, revision, ownership generation or fencing authority.

Where an existing detailed owner baseline defines stronger or more precise rules, that baseline remains the normative source.

## Explicitly deferred domain catalogues

The final `FND-ID-01` contract must **not** freeze a domain-specific identifier merely to make the project look complete.

The following classes remain with their owning later contracts unless they satisfy the conditional-foundation test above:

### Durability, economy and item lifecycle

Examples:

- `ItemInstanceId`;
- `ContainerInstanceId`;
- `TradeId`;
- `MarketOfferId`;
- `MailId`;
- `RewardGrantId`;
- `LootSettlementId`;
- economy-specific transaction entities.

These are primarily `DUR-*`, `GAME-ITEM-*` and economy-contract concerns.

The UUIDv7 durable-identity baseline may constrain their future representation direction without requiring their full lifecycle catalogue in `FND-ID-01`.

### Analytics, audit and investigation

Examples:

- `EventId`;
- analytics episode/case identifiers;
- correlation/causation identifiers;
- detector-run identities;
- security investigation/case identities.

These remain primarily `ANL-*`, security and Game Intelligence contract concerns.

### Quests, bosses, events and progression

Examples:

- `QuestRunId`;
- `BossAttemptId`;
- `LockoutId`;
- dynamic-event occurrence IDs;
- progression-run or reward-cycle identities.

These remain owned by their gameplay/content and durability contracts.

### Static content and world definitions

Stable content definitions continue to use the accepted content model such as:

```text
stable content key + revision + compact bundle/runtime identifier
```

The final `FND-ID-01` contract does not assign UUIDv7 automatically to every spell, monster definition, quest, zone, map tile, chunk, area, subarea, content object or asset.

`ZoneId` remains semantically distinct where used, but its exact durable/content identity model is frozen by the appropriate world/content contract unless a foundation boundary proves that a durable cross-process Zone identity is required earlier.

### Operations and infrastructure

A future stable placement/deployment/host-like identifier is not automatically part of `FND-ID-01` merely because `NodeId` deliberately does not represent those concepts.

Operations/topology contracts own any future `PlacementId`, deployment-target identity or similar concept unless it becomes a required foundation cross-boundary identity.

## Boundaries with downstream gates

### `FND-02` owns protocol mechanics

`FND-ID-01` defines the semantic identities that the protocol may carry.

`FND-02` owns:

- exact wire fields;
- byte order and IDL;
- framing;
- compact session-handle width and mapping/reset rules;
- command/message sequencing;
- `CommandId` representation where it is protocol-specific;
- capability negotiation;
- snapshot/delta encoding;
- protocol error representation.

`FND-ID-01` must not design a wire schema to resolve an identity question.

### `FND-03` owns runtime mechanics

`FND-ID-01` preserves the distinction between durable identity and runtime-local generational handles.

`FND-03` owns:

- exact handle widths and allocation domains;
- ChannelRuntime/InstanceRuntime scheduling;
- ownership generations;
- worker/task handles;
- runtime lifecycle and recovery mechanics.

### `FND-04` owns admission/session/lease state machines

`FND-ID-01` must make the relevant semantic identities and ownership boundaries unambiguous.

`FND-04` owns:

- Game Session issuance/validation state machine;
- admission and reconnect credentials;
- character lease lifecycle;
- duplicate-login/takeover transitions;
- transport rebinding and generation mechanics;
- exact terminal session states;
- authorization/replay/expiry behavior.

The two gates must not create circular ownership. If `FND-04` requires a distinct semantic identity, `FND-ID-01` may classify that identity and owner; `FND-04` still owns the behavior and credential/state-machine semantics around it.

### `DUR-*` owns physical persistence

`FND-ID-01` may freeze canonical durable representation where cross-boundary compatibility requires it.

Durability contracts own:

- PostgreSQL primary/secondary key layout;
- indexes and clustering;
- partitioning;
- migration from legacy identifiers;
- retention/tombstones beyond the semantic no-reuse rule;
- transaction schema and storage-local optimizations.

## Public exposure principle

Internal canonical identifiers are not automatically public identifiers.

The final `FND-ID-01` contract needs only enough visibility classification to prevent accidental exposure or reliance on obscurity.

Detailed public profile URLs, opaque public references, privacy-redaction policy and social/search exposure remain with the relevant product/security/privacy contracts.

Possession of an identifier never grants authority.

## Why this boundary is required

Expanding `FND-ID-01` into every future domain would create several architectural problems:

- circular dependencies between foundation and unaccepted gameplay/economy/analytics contracts;
- premature lifecycle decisions for systems whose semantics are not designed yet;
- pressure to assign UUIDs to entities that should remain bundle-local or runtime-local;
- accidental public exposure of stable internal identities;
- a foundation gate that cannot close until the entire game design is complete;
- duplicated or conflicting ownership decisions when later domain contracts mature.

The minimum-scope rule avoids those failure modes while preserving one coherent identity model.

## Acceptance test for the final FND-ID-01 contract

Before the complete `FND-ID-01` package is accepted, review every included identifier with two questions:

```text
1. Does FND-02, FND-03 or FND-04 need this semantic identity decision
   before that downstream contract can be safely frozen?

2. Is FND-ID-01 defining only identity/scope/ownership/representation
   rather than stealing the later domain's lifecycle or behavior contract?
```

If the answer to question 1 is no, the identifier belongs to a later gate.

If the answer to question 2 is no, the definition is too broad and must be reduced.

## Programme effect

Accepted now:

```text
FND-ID-01 final scope = minimum cross-boundary foundation identity contract

minimum dimensions =
    semantic type
    identity class
    scope / uniqueness
    owner
    issuer
    lifetime / reuse / absence
    required representation constraints
    visibility
    validation/comparison
    authority/fencing relationship

future domain IDs != automatically part of FND-ID-01
UUIDv7 direction != requirement to design every future ID now
FND-02 owns wire/sequence mechanics
FND-03 owns runtime handles/ownership mechanics
FND-04 owns session/admission/lease state machines
DUR/ANL/gameplay/content/operations own their detailed identifier catalogues
```

The next complete `FND-ID-01` contract must consume this baseline and the earlier detailed owner baselines. It must not expand into a whole-game identifier registry without a new explicit owner decision.

No implementation is authorized by this baseline.
