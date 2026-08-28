# ADR-0012: Character Authority and Platform lifecycle boundary

- Status: Accepted
- Date: 2026-08-07
- Decision owners: Oteryn project
- Coordination ID: `OTV2-CHARACTER-LIFECYCLE-BOUNDARY`
- Applies to: native character ownership/lifecycle, Platform character projections, admission ownership validation, character creation/rename/deletion/world transfer/account transfer/Bazaar orchestration
- Does not authorize: Rust runtime, persistence schema, protocol wire format, Platform runtime changes, Canary changes or production activation

## Context

Oteryn has already accepted three separate identity and data-ownership facts:

1. Oteryn Platform Identity owns and issues canonical native `AccountId`.
2. Oteryn-v2 game-domain Character authority owns and issues canonical `CharacterId`.
3. Character and durable progression data belong to the game persistence boundary rather than to the Platform database.

The remaining ambiguity was operational: which side is authoritative for the current `AccountId <-> CharacterId` relation, how Platform WWW obtains a user's character portfolio, and which side performs character create/rename/delete/transfer mutations.

The current Oteryn Platform has carefully bounded Canary compatibility adapters. They can create a Canary `players` row through a dedicated least-privilege principal and Character Bazaar can transfer a Canary player by changing `players.account_id` through another narrowly scoped principal. Those mechanisms are valid current compatibility evidence, but copying that direct-database writer model into Oteryn-v2 would violate ADR-0004 and create two mutation authorities for the native character aggregate.

The native design therefore needs a single authoritative Character boundary while retaining Platform ownership of authentication, AccountId, commerce and user-facing orchestration.

## Decision

### 1. Character Authority owns canonical character state

The authoritative **Oteryn-v2 Character Authority** is the sole semantic owner of:

- canonical `CharacterId` issuance;
- the native character aggregate and lifecycle state;
- authoritative current `AccountId <-> CharacterId` ownership binding;
- current logical world membership of a character;
- final character-name reservation and namespace conflict enforcement;
- gameplay invariants attached to create, rename, deletion/restore/finalization, world transfer and account ownership transfer;
- game-domain transaction boundaries and durable receipts/events for those mutations.

Platform does not become a character authority merely because it authenticates the user or displays character data.

### 2. Platform retains AccountId and orchestration ownership

Oteryn Platform Identity remains the sole owner/issuer of canonical native `AccountId`.

Platform may own product workflows around characters, including:

- authenticated web/launcher UX;
- account and entitlement checks owned by Platform;
- Character Bazaar auction records, pricing, bids, wallet settlement and commercial state;
- support/moderation workflow metadata within Platform permissions;
- notifications and user-facing orchestration state;
- privacy-safe portal read models and caches.

These responsibilities do not authorize Platform to write the native character aggregate directly.

### 3. Authoritative account-character relation is game-owned

The canonical relation is conceptually:

```text
Platform AccountId A
    -> game-domain CharacterId C1
    -> game-domain CharacterId C2
    -> game-domain CharacterId C3
```

The game domain stores and validates the current owner `AccountId` for each character where account ownership exists.

Platform consumes this relation through an authorized game-owned query or projection. Platform does not maintain a second authoritative ownership table for native characters.

A Platform cache/read model may be eventually consistent. It must carry enough revision/freshness semantics for its declared use and must fail toward less disclosure when ownership cannot be proven.

### 4. Platform character lists are projections, not authority

A native Platform-facing character portfolio is conceptually:

```text
GetCharactersForAccount(AccountId)
    -> CharacterSummary[]
```

The exact API, transport and encoding are deferred. The semantic result may contain only explicitly authorized account-visible fields such as:

- `CharacterId`;
- current `WorldId`;
- current name;
- class/vocation summary;
- level/progression summary where product policy permits;
- lifecycle/availability state needed by the caller;
- projection revision/freshness metadata.

The projection must not expose private alternate-character relations publicly merely because Platform can correlate them for the authenticated owner.

### 5. Admission revalidates ownership

Neither a browser-supplied `CharacterId`, a Platform cache entry nor a previously issued routing/ticket context is final proof that an account still owns a character.

Before creating authoritative gameplay authority, the game admission/session boundary must revalidate at minimum:

```text
AccountId A currently owns CharacterId C
AND character C is eligible for admission
AND current world/channel placement is valid
AND session/lease/fencing invariants permit entry
```

A stale Platform projection after rename, transfer, sale or deletion cannot grant gameplay authority.

This ADR does not freeze the FND-04 admission/token/lease state machine.

### 6. Native mutations use game-owned command boundaries

The target mutation direction is:

```text
Platform / trusted caller
    -> versioned Character command boundary
    -> Oteryn-v2 Character Authority
    -> game-domain validation + transaction
    -> bounded idempotent outcome / operation receipt
    -> projection/event reconciliation
```

This applies to at least:

- character creation;
- rename;
- deletion scheduling where supported;
- restore/cancel deletion where supported;
- terminal lifecycle finalization;
- legal world transfer;
- legal account ownership transfer, including Character Bazaar settlement.

Platform direct SQL `INSERT`, `UPDATE` or `DELETE` against native Oteryn-v2 character/game tables is not an approved target mutation model.

A separately bounded migration bridge may exist only under an explicit migration contract, least privilege and a defined removal path. It must not silently become the native steady-state API.

### 7. Character creation belongs to Character Authority

Platform may collect user intent and perform Platform-owned checks, but Character Authority performs the authoritative creation transaction.

Conceptually:

```text
CreateCharacter
    AccountId
    requested name
    requested world
    requested creation choices allowed by product/ruleset
    idempotency key
```

Character Authority owns:

- current account eligibility checks that depend on authoritative character state;
- final name canonicalization/reservation enforcement that must be atomic with creation semantics;
- character-slot/quota enforcement where that quota is game/character-domain state;
- canonical starter state/template application;
- allocation of fresh `CharacterId` UUIDv7;
- durable creation transaction and idempotent outcome.

A Platform `check name availability` feature may exist for UX but is advisory. It cannot reserve or guarantee the name unless it uses the accepted Character Authority reservation/creation contract.

The exact name namespace (global, per world or another accepted scope) remains a `GAME-CHAR-01` product decision and is not frozen here.

### 8. Stable CharacterId across legal lifecycle mutations

Accepted identity continuity is:

```text
rename                     -> same CharacterId
legal world transfer       -> same CharacterId
legal account transfer     -> same CharacterId
```

Mutable name, current `WorldId` and current owner `AccountId` are not part of CharacterId equality.

Terminal deletion/retirement never permits reuse of that CharacterId for another semantic character. Recreating a character with a recycled display name creates a fresh CharacterId.

Exact restoration and retention semantics remain later `GAME-CHAR-01`, durability and privacy work.

### 9. Bazaar remains a Platform commercial saga

Character Bazaar is deliberately split by semantic ownership.

Platform owns commercial orchestration such as:

- listing/auction state;
- bids;
- Platform wallet holds and settlement;
- commissions;
- user-facing recovery state;
- notification and commercial audit metadata.

Oteryn-v2 Character Authority owns whether the character can be transferred and the final authoritative ownership mutation.

Target settlement shape:

```text
Platform Bazaar saga
    -> request/claim transfer operation for CharacterId
    -> Character Authority checks current owner + transfer eligibility + session/lifecycle conflicts
    -> Character Authority atomically rebinds owner AccountId
    -> durable idempotent operation receipt/outcome
    -> Platform reconciles wallet/auction saga from authoritative result
```

No cross-database distributed ACID transaction is required. The integration uses durable saga state, idempotency keys/operation identity, bounded outcomes and reconciliation after ambiguous failures.

Platform must never infer that ownership changed merely because a timeout occurred.

### 10. Legacy Canary adapters remain compatibility-only

Existing Oteryn Platform Canary-specific character creation, transfer and deletion contracts may continue to describe their current explicitly scoped compatibility behavior until those paths are retired.

They are not native Character Authority contracts and do not authorize the native Platform to become a writer of Oteryn-v2 character tables.

Numeric Canary `players.id`, Canary `accounts.id`, `canary_account_id` and direct `players` mutation are anti-corruption/migration compatibility state only.

A future Platform-side documentation package should classify those contracts explicitly against this native boundary before native runtime integration is enabled.

### 11. Cross-system consistency model

Cross-boundary character workflows must assume independent Platform and game transactions.

Required design properties include:

- stable operation/idempotency identity for mutation requests where retries are possible;
- bounded deterministic result classes rather than raw persistence errors;
- authoritative reread/reconciliation after ambiguous outcomes;
- no duplicate ownership mutation from retries;
- no stale projection becoming mutation or admission authority;
- explicit versioning and rollout compatibility;
- auditability for ownership changes and destructive lifecycle operations;
- fail-closed behavior for security/ownership ambiguity.

The exact `OperationId`, command envelope, acknowledgement and wire representation remain owned by later `ANL-*`/durability/FND-02 decisions as applicable. This ADR fixes semantics, not their final transport encoding.

### 12. This is a boundary decision, not full GAME-CHAR-01 completion

This ADR partially resolves `GAME-CHAR-01` by fixing **authority and cross-repository orchestration**.

`GAME-CHAR-01` remains open for at least:

- final character-name namespace and history/recycling policy;
- character slots/quotas and entitlement interaction;
- creation choices and starter templates;
- level/experience/skills/attributes/capacity formulas;
- vocation/class/promotion/mastery/respec;
- death/respawn/penalties/blessings;
- offline training/progression;
- exact deletion/restore/retention product semantics;
- world-transfer eligibility/economy/content restrictions;
- safe progression migrations and deterministic formula fixtures.

`DUR-01`/`DUR-02` remain responsible for physical durable representation, schema, locking, revisions, outbox/recovery and database implementation. `FND-02` remains responsible for protocol mechanics. `FND-04` remains responsible for admission/session/lease semantics.

## Security and privacy consequences

- Knowledge of AccountId or CharacterId grants no authority by itself.
- Account-to-character ownership is restricted data; public surfaces require explicit disclosure policy.
- Platform projections must be authorization-scoped and stale-safe.
- Mutating commands must authenticate the calling service/user context and revalidate current authoritative ownership.
- Destructive lifecycle operations and account transfers require durable audit/recovery evidence appropriate to their risk.
- Support/admin tooling must use audited domain commands rather than bypassing Character Authority with raw SQL.

## Consequences

### Positive

- one writer/owner exists for the native character aggregate;
- gameplay invariants cannot be bypassed by portal SQL;
- Platform and game databases remain independently evolvable;
- stale portal data cannot become login authority;
- Character Bazaar can retain robust saga/recovery patterns without owning gameplay state;
- stable CharacterId preserves audit, analytics and provenance across rename/world/account transfer;
- the design remains compatible with later API, protocol and persistence choices.

### Costs

- Platform character features require explicit game APIs/projections rather than convenient shared-table writes;
- character workflows need idempotent cross-system orchestration and reconciliation;
- native Character Authority implementation becomes a prerequisite before retiring Canary direct-write compatibility paths;
- Platform and game deployments require versioned rollout coordination.

## Rejected alternatives

### Platform owns native AccountId-to-CharacterId mapping

Rejected because the mapping is current character ownership state and must be transactional with character lifecycle/game invariants.

### Platform writes native character tables directly

Rejected because it creates multiple writers, couples Laravel to game persistence layout and can bypass gameplay transaction/invariant boundaries.

### Game domain owns AccountId issuance

Rejected because Platform Identity is already the accepted account authority.

### Treat a Platform character projection as sufficient login proof

Rejected because projections can be stale after sale, transfer, deletion or administrative state changes.

### Change CharacterId when the character changes world/account/name

Rejected because those are mutable relationships/labels, not replacement of the semantic character.

### Distributed ACID between Platform and game databases

Rejected as the default because the bounded contexts must remain independently deployable and recoverable; durable saga/reconciliation is the safer cross-system model.

## Required follow-up

1. Continue `GAME-CHAR-01` for remaining character product/lifecycle/progression decisions before final character schema.
2. Define native Character Authority command/query semantics without pre-empting FND-02 wire decisions.
3. Define durable operation/idempotency/revision/audit mechanics in the owning durability/analytics gates.
4. Define admission ownership revalidation in FND-04.
5. Create a separately authorized Oteryn Platform consumer/compatibility documentation package using coordination ID `OTV2-CHARACTER-LIFECYCLE-BOUNDARY`.
6. Before native cutover, prove stale-projection, duplicate-command, transfer-timeout, ownership-race and admission-after-transfer failure scenarios.

No runtime, schema, production, Platform or Canary implementation is authorized by this ADR alone.
