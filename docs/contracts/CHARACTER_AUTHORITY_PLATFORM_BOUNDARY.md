# Character Authority / Platform Boundary Contract

## Status

`ACCEPTED ARCHITECTURE CONTRACT — IMPLEMENTATION NOT YET AUTHORIZED`

- Date: 2026-08-07
- Coordination ID: `OTV2-CHARACTER-LIFECYCLE-BOUNDARY`
- Canonical owner-side ADR: `docs/architecture/ADR-0012-character-authority-and-platform-lifecycle-boundary.md`
- Producer/authority repository: `blakinio/Oteryn-v2`
- Consumer/orchestrator repository: `blakinio/Oteryn-Platform`

This contract defines semantic ownership and integration obligations. It deliberately does not select an HTTP/RPC framework, protobuf/JSON schema, transport, framing, authentication envelope or `protocol-oteryn` wire representation.

## 1. Authorities

### Oteryn Platform

Authoritative for:

- canonical native `AccountId` identity and issuance;
- Identity authentication/security policy;
- OAuth/PKCE and Platform sessions;
- Game Login Ticket lifecycle within the accepted Platform boundary;
- Platform-owned entitlements and commercial/business state;
- Character Bazaar auction/bid/wallet/commission workflow state;
- portal UX, notifications and Platform-local read-model/cache state.

### Oteryn-v2 Character Authority

Authoritative for:

- canonical `CharacterId` identity and issuance;
- character aggregate lifecycle;
- authoritative current owner `AccountId` binding;
- current character world membership;
- final name reservation/uniqueness enforcement;
- create/rename/delete/restore/finalize/world-transfer/account-transfer gameplay invariants;
- game-domain mutation transactions;
- authoritative mutation result/receipt state;
- source data for authorized account-character projections.

## 2. Canonical identity relation

```text
AccountId = Platform-owned native identity
CharacterId = game-owned UUIDv7 identity

AccountId A
  owns -> CharacterId C1
  owns -> CharacterId C2
```

The authoritative ownership relation is held by the game domain.

Platform may cache or project this relation for authorized UX but does not acquire mutation authority or a second canonical ownership source.

## 3. Read contract

A native account-authorized character portfolio is semantically equivalent to:

```text
ListCharactersForAccount(AccountId) -> CharacterSummary[]
```

The producer must derive results from authoritative or contractually defined game-owned ownership state.

A `CharacterSummary` may include only fields explicitly approved for the caller, for example:

- `CharacterId`;
- current `WorldId`;
- current display name;
- lifecycle/availability state required by the workflow;
- class/vocation and progression summary where product policy permits;
- projection revision/freshness metadata.

Requirements:

- alternate-character ownership is private by default;
- public profile/search consumers do not receive the AccountId relation unless explicitly authorized;
- stale, unavailable or contradictory ownership state fails toward less disclosure;
- a cached/projection result never proves current mutation or gameplay authority.

Exact schema and transport remain deferred.

## 4. Admission ownership rule

Before authoritative gameplay admission for `CharacterId C` under `AccountId A`, the game boundary must revalidate current authoritative ownership.

At minimum:

```text
owns(A, C) == true
character C is currently admissible
world/channel placement is valid
session/lease/fencing state permits entry
```

A Platform-issued ticket may authorize an attempt to enter the game flow but does not replace this final ownership validation.

A stale Platform character list or pre-transfer/pre-sale ticket must not allow the former owner to obtain a new authoritative gameplay session after ownership has changed.

Exact ticket/session/lease mechanics remain FND-04 work.

## 5. Mutation contract family

Native character mutations cross the Platform/game boundary only through versioned game-owned commands or equivalent explicit service contracts.

Required mutation families include, when the product enables them:

```text
CreateCharacter
RenameCharacter
ScheduleCharacterDeletion
RestoreCharacter
FinalizeCharacterLifecycle
TransferCharacterWorld
TransferCharacterOwnership
```

Not every command must exist at initial launch. A disabled product capability may remain absent. Once supported, it must follow this ownership contract.

### Common semantic requirements

Every retryable cross-system mutation must provide or derive a stable idempotency/operation identity in the owning later contract.

The game authority must:

- authenticate/authorize the trusted caller context;
- revalidate current authoritative AccountId/CharacterId ownership where applicable;
- validate lifecycle/session/world/transfer conflicts;
- execute the game-owned mutation atomically within its persistence boundary;
- return a bounded result or durable operation receipt suitable for safe retry/reconciliation;
- never expose raw database error details as a public contract;
- preserve durable audit evidence where the operation is security-, ownership- or economy-sensitive.

The exact `OperationId` type and command envelope are intentionally deferred to their owning foundation/durability/analytics contracts.

## 6. Character creation semantics

Platform supplies authenticated intent and Platform-owned context; Character Authority decides the authoritative result.

Minimum conceptual request:

```text
AccountId
requested_name
requested_world
allowed creation choices
idempotency context
```

Character Authority is responsible for:

- allocating the canonical fresh `CharacterId`;
- final atomic name reservation/conflict decision;
- authoritative game-domain character quota/eligibility rules;
- applying canonical starter state for the selected ruleset/content revision;
- writing all required dependent state consistently;
- returning created/existing/conflict/denied/unavailable style bounded outcomes as later specified.

A Platform name-availability preview is advisory only unless backed by an accepted reservation protocol owned by Character Authority.

## 7. Identity continuity

The following preserve `CharacterId`:

```text
rename
legal world transfer
legal AccountId ownership transfer
```

The following does not permit ID reuse:

```text
terminal deletion / retirement
```

A later character using a recycled display name receives a new CharacterId.

## 8. Deletion/restore/finalization

Deletion lifecycle is game-owned because it may affect gameplay state, sessions, guilds, houses, market state, inventory, retention and audit evidence.

Platform may own user-facing workflow/orchestration metadata but must not treat its workflow row as proof that the game character was deleted, restored or finalized.

Any supported lifecycle must define bounded states and idempotent recovery. Exact grace period, retention, tombstone and erasure policy remain `GAME-CHAR-01`, durability and privacy decisions.

## 9. World transfer

World transfer preserves `CharacterId` while changing authoritative current `WorldId` after the transfer contract commits.

The game authority must eventually establish exactly one authoritative current-world membership and fence stale source-world mutation rights.

Transfer eligibility, economy restrictions, guild/house/social effects, content compatibility and rollback are later character/world-lifecycle decisions.

Platform cannot implement native world transfer by directly rewriting game persistence fields.

## 10. Character Bazaar / account ownership transfer

The commercial saga and character mutation are intentionally separated.

### Platform owns

- listing and auction lifecycle;
- bids and winner selection;
- wallet holds/debits/credits;
- commission/business rules;
- customer-facing saga state and notifications;
- commercial reconciliation/audit.

### Character Authority owns

- current seller ownership validation;
- transfer eligibility and lifecycle/session conflict checks;
- authoritative `AccountId` rebinding;
- one atomic ownership mutation inside the game domain;
- durable idempotent transfer result/receipt.

Conceptual flow:

```text
Platform marks commercial operation pending
  -> Character Authority TransferCharacterOwnership
  -> authoritative receipt/result
  -> Platform reconciles commercial saga
```

Rules:

- no distributed ACID is assumed;
- timeout is not success or failure proof;
- retries must not duplicate ownership mutation;
- Platform settlement recovery uses authoritative operation/ownership state;
- stale seller/buyer projections do not override current Character Authority state;
- transfer must interact safely with active session/lease rules defined by later contracts.

## 11. No native direct database mutation from Platform

The target architecture forbids Platform application credentials from having general native character-table write authority.

Specifically, native steady-state Platform integration must not require arbitrary direct:

```text
INSERT character rows
UPDATE character owner/world/name/lifecycle fields
DELETE character rows
```

A temporary migration bridge requires a separately accepted migration contract with narrow privileges, compatibility window, rollback and removal criteria. It is not precedent for the native service boundary.

## 12. Canary compatibility classification

Current Platform Canary adapters and their contracts remain valid only in their declared legacy/current compatibility scope.

Examples of compatibility-only identifiers/operations include:

- Canary `accounts.id` / `canary_account_id`;
- Canary numeric `players.id`;
- direct bounded insert into Canary `players` for current compatibility creation;
- direct bounded update of Canary `players.account_id` for current compatibility Bazaar transfer;
- Canary-specific deletion discovery/lifecycle constraints.

These do not define Oteryn-v2 Character Authority behavior.

A Platform-side consumer contract should later label these paths explicitly as `LEGACY / COMPATIBILITY`, without breaking them until their migration/cutover is separately authorized.

## 13. Failure and reconciliation rules

Cross-boundary workflows must distinguish at least:

- deterministic business rejection;
- authorization/ownership rejection;
- lifecycle/session/transfer conflict;
- stale expected state/revision;
- dependency unavailable;
- ambiguous result requiring reconciliation.

For ambiguous results:

1. do not fabricate success or failure;
2. reread authoritative operation/current ownership state through the owning game contract;
3. reconcile idempotently;
4. escalate to bounded recovery state if authoritative classification remains unavailable.

## 14. Versioning and rollout

The semantic producer is Oteryn-v2 Character Authority. Platform is a consumer/orchestrator.

Rollout must be producer-compatible first:

1. accept/freeze owner-side semantic contract;
2. later implement and version the game-owned command/query surface under the appropriate foundation/durability gates;
3. add Platform consumer capability while existing Canary compatibility remains explicitly separated;
4. prove dual-path/mixed-version failure behavior as needed;
5. cut native workflows only after both producer and consumer revisions are compatible;
6. retire Canary direct-write compatibility only through a separately accepted migration/removal task.

No client or Platform release may assume a new command/projection exists before the authoritative game producer supports that version.

## 15. Deferred details

This contract intentionally does not freeze:

- HTTP vs RPC vs another private service transport;
- protobuf/JSON/other IDL;
- TLS/ALPN/framing;
- `protocol-oteryn` packet/message layouts;
- `CommandId` or exact `OperationId` representation;
- physical PostgreSQL tables/indexes;
- name namespace scope;
- exact slot/quota values;
- exact deletion grace/retention/erasure policy;
- exact progression formulas;
- Bazaar pricing/commission/product rules;
- FND-04 session/lease state machine.

These remain with their owning gates.

## 16. Conformance scenarios required before native activation

Later implementation must prove at least:

1. **stale projection after sale** — previous owner sees stale Platform cache but admission/mutation is rejected by current game ownership;
2. **duplicate create retry** — ambiguous response followed by retry produces one semantic character, not duplicates;
3. **same-name race** — final authoritative namespace produces one deterministic winner;
4. **ownership transfer timeout** — Platform reconciles authoritative receipt/state without double settlement or duplicate transfer;
5. **active-session transfer conflict** — transfer cannot create two owners or bypass accepted session/lease fencing;
6. **world-transfer stale writer** — destination ownership/current-world state fences stale source mutation;
7. **terminal deletion** — deleted CharacterId is never reissued;
8. **projection privacy** — unauthenticated/public consumers cannot infer AccountId-to-alt-character relationships;
9. **mixed-version rollout** — unsupported command/query version fails explicitly and safely rather than falling back to Canary/direct SQL.

## Decision

`NATIVE CHARACTER AUTHORITY: OTERYN-V2 GAME DOMAIN`

`PLATFORM ROLE: ACCOUNT AUTHORITY + UX/COMMERCIAL ORCHESTRATION + AUTHORIZED READ PROJECTIONS`

`NATIVE PLATFORM DIRECT CHARACTER-DB WRITES: FORBIDDEN AS STEADY-STATE DESIGN`
