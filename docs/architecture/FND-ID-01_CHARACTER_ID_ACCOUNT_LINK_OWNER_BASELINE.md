# FND-ID-01 CharacterId and Account Link Owner Baseline

- Status: Owner-accepted pre-contract baseline
- Date: 2026-08-07
- Decision owner: Oteryn project owner
- Gate: `FND-ID-01`
- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Applies to: character identity, account ownership linkage, Platform WWW projections, rename, world transfer, admission, persistence, analytics and future character lifecycle contracts

## Purpose

Record the owner-accepted canonical identity model for `CharacterId` and its relationship to Platform-owned `AccountId` before the complete `FND-ID-01` identifier catalogue is finalized.

This baseline is architecture input only. It does not authorize Rust runtime, protocol, persistence, Platform, web-portal or production implementation.

## Owner-accepted decision

Canonical character identity is:

```text
CharacterId = strongly typed UUIDv7, full 128 bits
CharacterRef = CharacterId
```

`CharacterId` is a **global durable game-domain identity**. It is not semantically scoped by `WorldId`.

The canonical issuer is the authoritative **game-domain Character authority**.

`AccountId` remains an externally owned identity issued and owned by **Oteryn Platform Identity** under ADR-0003.

At any point where a character has an account owner, the game-domain character record holds an explicit validated binding to the Platform-owned `AccountId`.

Conceptually:

```text
Platform AccountId A
    -> CharacterId C1
    -> CharacterId C2
    -> CharacterId C3
```

This relation enables Platform WWW to present the authenticated account's character portfolio without making Platform the issuer or authoritative gameplay owner of `CharacterId`.

## Global character identity

`CharacterId` identifies one semantic character independently from mutable placement and presentation state.

It is distinct from:

- `AccountId`;
- `WorldId`;
- `ChannelId`;
- `InstanceId`;
- character name;
- vocation/class;
- level or progression state;
- current Game Session;
- GameNode placement;
- database row position;
- public profile slug or URL.

Therefore the following are not canonical character identity:

```text
WorldId + CharacterId
character name
account id + character slot
world + character name
```

`WorldId` remains authoritative current-world membership/placement metadata, but it is not part of `CharacterId` equality.

## Rename semantics

Character display name is mutable presentation/namespace state, not identity.

Accepted consequences:

- renaming a character preserves `CharacterId`;
- historical analytics, audit, achievements, item provenance and portal references may continue to correlate through the same CharacterId;
- old and new names must not be interpreted as different characters merely because the label changed;
- exact name uniqueness, reservation, recycling, history visibility and public redirect policy remain future character/Platform product contracts.

## World-transfer semantics

A character preserves the same `CharacterId` when transferred between logical worlds.

Conceptually:

```text
CharacterId C
current_world = WorldId W1
    -> authorized world transfer
CharacterId C
current_world = WorldId W2
```

Accepted consequences:

- transfer does not mint a replacement `CharacterId`;
- character history remains attributable to the same semantic character;
- mutable world membership changes under an explicit transfer contract;
- source/destination eligibility, economy restrictions, inventory compatibility, housing, guild, social and transfer-settlement rules remain later product/durability contracts;
- no two-world concurrent gameplay authority is implied by preserving identity;
- transfer must eventually establish one authoritative current-world binding and fence stale source-world mutation rights.

The accepted rule that a logical world owns its active character population and economy remains compatible with a globally stable character identity: ownership of **current world membership** may change while semantic character identity remains constant.

## Account relationship and ownership boundary

`AccountId` and `CharacterId` have different authoritative owners.

```text
Oteryn Platform Identity
    owns AccountId identity/authentication

Game-domain Character authority
    owns CharacterId issuance and character lifecycle
    stores/validates current owner AccountId binding where applicable

Platform WWW
    consumes an authorized projection of AccountId -> CharacterId[]
    does not become the authoritative CharacterId issuer or gameplay-state owner
```

Accepted invariants:

- Platform must not invent, re-key or mint canonical `CharacterId` values;
- game-domain must not mint competing `AccountId` values;
- account-character linkage crosses the repository/service boundary through an explicit contract rather than implicit database coupling;
- knowing `AccountId` or `CharacterId` does not itself authorize observation or mutation;
- the Platform-facing character list is derived from authoritative account-character ownership state and must fail toward less disclosure when authorization or projection freshness cannot be proven;
- direct unrestricted cross-database foreign-key coupling is not required and remains contrary to the accepted database ownership model.

## Platform WWW consequence

Platform WWW may use the account-character relationship to provide authorized product surfaces such as:

- account character list;
- character profile;
- current world and public gameplay summary;
- level/vocation/progression projections;
- achievements and statistics;
- transfer workflows;
- Bazaar/marketplace-like workflows if later accepted;
- moderation/support history subject to permissions;
- Game Analytics-derived player-facing statistics subject to privacy policy.

These are projections/product capabilities, not transfer of character gameplay authority to Platform.

The exact portal API, projection schema, cache invalidation, public/private field catalogue and synchronization mechanism remain future cross-repository contracts.

## Character deletion and reuse

`CharacterId` is never reused for another semantic character.

Accepted lifecycle rules:

- nil/zero UUID is invalid;
- absence is explicit;
- deletion does not return CharacterId to an allocation pool;
- deleted/retired characters retain tombstone/audit semantics where required by security, economy, anti-duplication, support, legal-retention or recovery contracts;
- recreating a character with the same display name creates a different CharacterId;
- UUIDv7 timestamp ordering is not creation authorization, gameplay chronology or authority evidence.

Exact soft-delete, retention, restoration and final erasure policy remain later durability/privacy work.

## Account ownership changes

This baseline fixes the existence and ownership direction of the AccountId-character relationship but does not yet freeze all character ownership-transfer product semantics.

If a future approved system permits a character to move between Platform accounts, such as a marketplace/Bazaar transfer, that operation should normally preserve `CharacterId` while changing the validated owner `AccountId` binding through an explicit audited transaction.

The exact rules for sale, gifting, recovery, account merge/split, sanctions and rollback are not authorized by this baseline.

## Identity is not authority

`CharacterId` identifies a character; it does not prove current gameplay authority.

Security-sensitive mutations require the later session/admission/fencing contract, conceptually including relevant combinations such as:

```text
CharacterId
+ GameSessionId
+ session_generation
+ current world/channel or instance placement binding
+ current lease/fencing context
```

A stale session or incorrect AccountId binding must not gain authority merely because it knows the correct CharacterId.

Client-provided CharacterId values are claims to validate, never proof of ownership.

## Privacy and public identifiers

Internal CharacterId is not automatically a public identifier.

UUIDv7 may reveal approximate creation-time information, and stable identifiers can enable unwanted correlation.

Therefore:

- Platform WWW may expose character name, slug or an opaque public profile reference instead of raw CharacterId where product/privacy policy prefers it;
- internal analytics, moderation and support may correlate by CharacterId under appropriate access control;
- public profile renames must not require changing CharacterId;
- public knowledge of CharacterId grants no account linkage, alternate-character discovery or gameplay authority;
- alternate-character relationships remain private by default unless an explicit product/privacy contract exposes them.

## Protocol and persistence boundaries

`FND-ID-01` fixes:

- `CharacterId` as strongly typed UUIDv7 preserving all 128 bits;
- global uniqueness/semantic identity independent from WorldId;
- Character authority as canonical issuer;
- AccountId as Platform-owned external identity;
- explicit account-character linkage;
- stable identity through rename and world transfer;
- no reuse after deletion.

Later contracts own:

- exact wire encoding and compact session references in `FND-02`;
- runtime placement/ownership implications in `FND-03`;
- account/session/character admission and leases in `FND-04`;
- PostgreSQL representation, indexes, tombstones and transfer transactions in durability work;
- Platform WWW projection APIs and synchronization in coordinated Platform contracts;
- character lifecycle, creation, rename, transfer and deletion product rules.

No protocol schema, database DDL or Platform implementation is selected here.

## Rejected alternatives

### WorldId + CharacterId is canonical character identity

Rejected because accepted world transfer must preserve the same semantic character identity when current-world membership changes.

### Character name is canonical identity

Rejected because names are mutable, user-visible and may be subject to reuse/rename policies.

### Platform issues CharacterId

Rejected because Platform owns account identity/control-plane concerns while the game-domain Character authority owns the character lifecycle.

### Game-domain issues a replacement AccountId

Rejected because AccountId remains Platform Identity authority under ADR-0003.

### Rename or world transfer creates a new CharacterId

Rejected because that fragments durable history, item/economy provenance, analytics, moderation, achievements and portal continuity for the same semantic character.

### Deleted CharacterIds are reusable

Rejected because reuse creates audit, economy, security, analytics and recovery ambiguity.

## Required application to later contracts

This baseline is mandatory input to:

- the complete `FND-ID-01` identifier catalogue/issuer matrix;
- `FND-02` character references on the wire;
- `FND-03` runtime placement and ownership mappings;
- `FND-04` account/session/character admission and lease contracts;
- character lifecycle/progression architecture;
- world-transfer and handoff product contracts;
- Platform WWW account/character projection contract;
- social/presence/privacy contracts;
- Bazaar/marketplace ownership-transfer architecture if accepted later;
- durability and audit contracts;
- Game Analytics identity/redaction contracts;
- E2E tests for rename, world transfer, account ownership checks and stale-session rejection.

## Cross-repository effect

This baseline defines the Oteryn-v2 side of a future cross-repository contract with `blakinio/Oteryn-Platform`.

It does **not** authorize writes to Oteryn-Platform and does not claim the current Platform implementation already exposes the final AccountId-to-CharacterId projection contract.

A coordinated Platform change, if required, must be performed under separate explicit repository authorization and its own task/branch/PR.

## Programme effect

- `CharacterId` -> strongly typed UUIDv7, full 128 bits;
- semantic scope -> global game-domain identity, not `WorldId + CharacterId`;
- issuer -> authoritative game-domain Character authority;
- `AccountId` remains Platform Identity-owned;
- explicit AccountId -> CharacterId[] relationship supports Platform WWW;
- Platform consumes authorized character projections without becoming gameplay authority;
- rename preserves CharacterId;
- world transfer preserves CharacterId while changing authoritative current-world membership;
- deleted CharacterId is never reused;
- exact Platform API, character lifecycle mechanics, session lease, wire encoding and persistence remain later contract work;
- no runtime, Platform or production implementation is authorized by this baseline.
