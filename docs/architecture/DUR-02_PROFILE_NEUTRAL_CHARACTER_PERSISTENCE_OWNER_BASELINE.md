# DUR-02 — Profile-Neutral Character Persistence Owner Baseline

- Status: **OWNER_ACCEPTED PARTIAL BASELINE**
- Owner decision date: 2026-08-12
- Owner decision time: 08:25 +02:00
- Repository recording date: 2026-08-12
- Stable gate: `DUR-02 — Persistence v1`
- Accepted sub-scope: **profile-neutral core Character persistence architecture**
- Overall `DUR-02` DecisionStatus: **PROPOSED**
- Partial-baseline DeliveryStatus: **OPEN** during this recording delivery; **LIFECYCLE_CLOSED** after terminal closeout
- Overall `DUR-02` DeliveryStatus after partial-baseline closeout: **PLANNED**
- ImplementationStatus: **NOT_STARTED**
- Runtime / PostgreSQL DDL authority: **NONE**
- Source type: `USER_SOURCE`
- Decision source: `DUR-02_PROFILE_NEUTRAL_CHARACTER_SCHEMA_DECISION_PACKET.md`
- Does not authorize: PostgreSQL DDL/migrations, database provisioning, Rust persistence runtime, item/currency persistence, profile-specific PvP Character persistence, Platform writes, production rollout or unresolved Reference behavior

## 1. Owner source and acceptance

### USER_SOURCE — accepted 2026-08-12 08:25 +02:00

The lifecycle-closed pre-decision packet `DUR-02_PROFILE_NEUTRAL_CHARACTER_SCHEMA_DECISION_PACKET.md` presented seventeen recommended rules for the profile-neutral Character persistence sub-scope of `DUR-02`.

After those seventeen points were explained as the technical and safety architecture for storing Character/FND-04 state in PostgreSQL — rather than gameplay formulas — the owner instructed:

> wykonaj

This is recorded as explicit owner acceptance of:

- all seventeen recommendations in section 18 of the pre-decision packet;
- the detailed semantics in sections 4-17 that those recommendations summarize;
- the partial-scope effect in section 19;
- the supersession/reopening discipline in section 20;
- the explicit non-decisions in section 21.

This acceptance is deliberately **partial**. It does not accept the whole historical `DUR-02 — Persistence v1` gate.

## 2. Status meaning

The binding state after this partial-baseline delivery lifecycle closes is:

```text
DUR-02 Character persistence sub-scope
Owner baseline         = OWNER_ACCEPTED PARTIAL BASELINE
DeliveryStatus         = LIFECYCLE_CLOSED
ImplementationStatus   = NOT_STARTED
Runtime / DDL authority = NONE

DUR-02 overall
DecisionStatus         = PROPOSED
DeliveryStatus         = PLANNED
ImplementationStatus   = NOT_STARTED
Runtime authority      = NONE
```

Therefore:

```text
accepted Character persistence architecture
!= whole DUR-02 accepted
!= SQL schema created
!= migration implemented
!= persistence runtime implemented
!= production persistence enabled
```

A later whole-DUR-02 reconciliation must identify the remaining historical Persistence-v1 subjects and either accept them or explicitly supersede/narrow that stable gate before overall `DUR-02` may become `ACCEPTED`.

## 3. Binding architecture boundary

The accepted Character persistence sub-scope freezes:

- profile-neutral Character persistence relation-family ownership;
- Character semantic revision/fencing;
- account portfolio/quota serialization boundary;
- global Character naming persistence boundary;
- physical separation of FND-04 authority dimensions;
- fresh-admission and reconnect/recovery persistence linearization;
- typed extension requirements;
- durable operation/idempotency boundaries;
- lock/isolation policy principles;
- mandatory durable audit/publication atomicity;
- checkpoint/current-state authority;
- restore/no-authority-resurrection invariant;
- migration/retirement/privacy separation;
- the explicit profile-neutral and unresolved-value boundary.

It does not freeze final SQL identifiers, Rust database libraries, exact index syntax, profile-specific tables or operational numeric policy.

## 4. Accepted rule 1 — normalized current state

Use **normalized current-state persistence**, not full event sourcing, as the authoritative Character state model for Persistence v1.

`character_root` is the logical Character identity/lifecycle/current owner/current world/global-revision lock anchor. Physically distinct Character-owned state belongs in typed child relations/aggregates when ownership, lifecycle, size, contention or migration justify decomposition.

The relation names in the decision packet are architecture names, not final SQL DDL identifiers.

Full event sourcing is not the Character source of truth. ANL-01 durable events remain evidence/audit, not gameplay authority.

## 5. Accepted rule 2 — one global CharacterRevision

Every committed Character **semantic** transaction advances one global `CharacterRevision` exactly once for that Character transaction boundary.

`CharacterRevision` is distinct from:

```text
CharacterLease generation
GameSessionId
connection_generation
RuntimeScopeAuthority ownership generation
EventId / TransactionId
wall clock / UUID order
```

A transaction may mutate several typed Character child relations while advancing the single CharacterRevision once.

FND-04-only authority transitions do not advance CharacterRevision merely because session/control state changes.

The exact physical scalar type for CharacterRevision remains implementation-owned subject to the accepted non-reuse/monotonic/fencing semantics.

## 6. Accepted rule 3 — account portfolio guard rows

Use a game-owned `account_character_guard`-equivalent concurrency anchor keyed by AccountId for every Character Authority operation whose correctness depends on the account portfolio or quota eligibility.

This includes every quota-affecting create/restore/lifecycle/account-transfer transition, not only character creation.

The guard:

- serializes the account-scoped critical section;
- does not become Account authority;
- does not prove Platform account authorization/existence;
- does not become a second authoritative active-character counter.

Authoritative quota eligibility is evaluated from Character Authority state under the guard and current accepted policy.

Multi-account ownership transfer locks account guards in deterministic canonical AccountId order.

## 7. Accepted rule 4 — global name registry and policy-safe cutover

Use a separate global Character name registry/history relation family containing at least the semantic equivalents of:

- display name;
- complete canonical comparison key;
- naming-policy revision;
- current/former/reserved claim class;
- CharacterId relation;
- policy-owned effective/reservation evidence where applicable.

The Character domain computes the canonical comparison key under the accepted naming policy. PostgreSQL enforces authoritative equality/uniqueness over that complete value; database collation must not invent gameplay normalization.

Application-only `check availability -> later write` is not sufficient correctness.

A naming-policy revision must:

1. compute destination canonical keys for all conflict-participating claims;
2. validate destination collisions before cutover;
3. resolve/block conflicts explicitly;
4. make exactly one policy interpretation authoritative at cutover.

Two simultaneously authoritative canonicalization universes are forbidden.

Exact normalization/repertoire/recycling behavior remains a separate Reference parity gate where unresolved.

## 8. Accepted rule 5 — separate FND-04 durable authorities

Persist FND-04 authority dimensions separately from Character semantic state and from each other.

At minimum the architecture preserves distinct relation families/equivalent typed state for:

- `AccountPresenceClaim`;
- `CharacterLease` + lease generation;
- `GameSession` + connection generation/terminality/recovery bindings;
- actor-wide `ControlLoss` continuity;
- runtime-scope ownership evidence as required by accepted FND-04.

Character ownership in `character_root` does not substitute for account presence. CharacterRevision does not substitute for CharacterLease generation. GameSession does not own duplicate actor-wide ControlLoss truth.

The actor-wide ControlLoss continuity relation is the single persistence authority for the loss episode/protection/grace/re-arm facts that may outlive an individual GameSession.

## 9. Accepted rule 6 — atomic fresh admission

Fresh admission has one authoritative database linearization boundary.

At final commit, the persistence transaction/equivalent authority boundary must revalidate the accepted FND-04 mutable facts, including current Character ownership/world/lifecycle under Character-root fencing/locking sufficient to close concurrent transfer/lifecycle TOCTOU.

Only a successful final boundary may atomically establish the semantic equivalents of:

```text
consume GrantNonce
+ establish/advance AccountPresenceClaim as required
+ establish/acquire CharacterLease as required
+ create canonical GameSessionId
+ GameSession ACTIVE
+ connection_generation = 1
+ establish the initial authoritative reconciliation/control boundary
```

Failure before or during the boundary leaves no partial presence, lease, GameSession, connection-generation or consumed-success nonce authority.

Fresh-admission authority-state changes do not advance CharacterRevision solely because they are FND-04 state.

## 10. Accepted rule 7 — reconnect/recovery PREPARE has zero authority

Reconnect/recovery `PREPARE` persists only bounded **typed candidate/disposition state** needed for idempotency and lost-response recovery.

PREPARE grants no gameplay, liveness, fencing or binding authority.

A retry of the same eligible reconnect attempt resolves to the same logical candidate/disposition rather than minting independently authoritative alternatives.

A process-local socket/file-descriptor/transport object handle is never a restart-stable durable identity or authority. Durable candidate state may reference only stable semantic identifiers/evidence sufficient to re-establish or reject the candidate safely after restart.

## 11. Accepted rule 8 — reconnect/recovery COMMIT is the only binding switch

Reconnect/recovery `COMMIT` is the sole atomic boundary that may replace current playable transport authority.

A successful COMMIT atomically, as applicable:

- proves the candidate remains current and eligible;
- fences predecessor transport/proof authority;
- advances `connection_generation` to the strict successor;
- binds the candidate transport/control authority;
- activates successor proof and invalidates predecessor proof;
- preserves `GameSessionId` for same-session reconnect;
- preserves actor/gameplay state and FND-02 reconciliation continuity;
- records stable attempt disposition for lost-response reconciliation;
- consumes/activates an eligible four-second protection entitlement at most once under the authoritative ControlLoss lock/fence.

Failed/stale COMMIT changes no authority and cannot revive a predecessor.

The exact accepted FND-04 stable-control re-arm model remains binding: one entitlement is used at most once for one eligible loss epoch; a later entitlement requires accepted re-arm followed by a later new loss epoch.

## 12. Accepted rule 9 — post-grace recovery creates a new GameSession

After the old GameSession is terminal, eligible recovery of a still-present authoritative actor creates a **new `GameSessionId`** while preserving the existing actor/gameplay state.

The new session begins with its own initial connection generation and fresh reconnect-proof continuity as defined by FND-04.

Post-grace status does not itself create or re-arm a protection entitlement. Existing actor-wide ControlLoss semantics may be consumed only when FND-04 proves they remain eligible/current.

A terminal GameSessionId never revives.

## 13. Accepted rule 10 — typed profile extensions only

Character-owned profile/progression extensions use dedicated typed relation families/child aggregates with explicit:

- semantic owner;
- CharacterId relation;
- stable definition identity/key owner;
- schema/revision compatibility;
- CharacterRevision interaction;
- migration/rollback contract;
- retention/privacy boundary where applicable.

A generic JSON/KV/EAV `misc state`, `type + blob` or similar untyped escape hatch is forbidden as a way to defer semantic ownership.

This applies to later physical storage for systems such as Weapon Proficiency, charms, Hunting Task/Prey state, Wheel/Promotion Points, Animus Mastery and profile-specific PvP Character facts when their owning semantic gate authorizes them.

## 14. Accepted rule 11 — durable operation receipts and conditional CommandId persistence

Retryable Character Authority workflows use durable `OperationId`-keyed receipts/equivalent typed operation state sufficient to detect duplicate versus conflicting reuse and reconcile ambiguous outcomes.

Same OperationId + same semantic request denotes one logical operation. Same OperationId + conflicting request is a conflict, never last-write-wins reinterpretation.

Creation retry before CharacterId is known must resolve to the single created CharacterId or one stable terminal result.

Do not persist every gameplay `CommandId` universally merely because DUR-01 defines its physical scalar. Persist `(GameSessionId, CommandId)` only when an actual durable gameplay command boundary requires crash-safe dedup/idempotency/evidence.

Equal numeric CommandId in another GameSession is a different command identity.

## 15. Accepted rule 12 — explicit lock/isolation proof

Correctness-sensitive Character persistence must have explicit anomaly-closing lock/constraint proofs.

`READ COMMITTED` is acceptable only when the operation's invariants are closed by accepted locked authority anchors and database constraints, such as:

- Character root lock for Character semantic serialization/revision;
- account guard for portfolio/quota serialization;
- authoritative full canonical-key uniqueness for naming;
- lease/session/generation locks and compare-fences for FND-04;
- operation/command uniqueness for idempotency.

If the anomaly cannot be proven closed under that model, the bounded transaction must use `SERIALIZABLE` with same-semantic-operation retry/reconciliation or a later explicitly accepted equivalent.

Advisory locks are never the sole authority for ownership, name uniqueness, quota or lease fencing.

Multi-entity locks use deterministic full-identity ordering as defined by the packet unless a later reviewed contract proves a safe alternative.

## 16. Accepted rule 13 — retained immutable audit semantics and mutable publication state

For every Character/FND-04 mutation requiring mandatory durable audit:

```text
authoritative state mutation
+ required revision/operation evidence
+ every mandatory retained durable event record
+ publication enqueue/state
commit atomically
OR
none becomes authoritative
```

Retained ANL-01 event semantic content and exact payload bytes remain immutable for the event's accepted retention lifetime.

Mutable delivery/publication bookkeeping is separate from authoritative event content.

At-least-once publication, EventId-stable duplicate handling and read-only replay remain binding.

This immutability does not override later accepted privacy/retention/anonymization/deletion/legal-hold policy. Retention lifecycle is separately governed and may remove/redact data only under its explicit accepted policy; it may not silently rewrite retained event semantics.

Best-effort telemetry remains outside the mandatory mutation transaction.

## 17. Accepted rule 14 — normalized current state is the Character checkpoint

The normalized committed Character root + typed Character-owned relations are the canonical current durable state.

Do not create a second generic serialized Character snapshot/blob as a parallel source of truth.

Where a later runtime/profile owner requires a broader recoverable checkpoint, use a typed manifest referencing typed owner-specific components and exact revision/profile context. The manifest is not an arbitrary state payload.

A success response is never authoritative before the owning database transaction commits.

Lost success responses reconcile from durable operation/command receipts and current state; they do not blindly repeat the semantic mutation.

## 18. Accepted rule 15 — no authority resurrection after restore

A process restart, failover, PITR or disaster restore may not reset/reuse Character/FND-04 fences or revive rolled-back/terminal authority.

A restored database snapshot cannot safely assume its session/lease rows are still current merely because they exist in the restored data.

Before restored state serves new admission or authoritative mutation, the restore/operations contract must establish a strictly newer **non-rollback recovery/authority fence** outside any value that could have rolled back with the restored snapshot, or an equivalently proven mechanism, and reconcile/fence stale pre-restore authority.

Admission and authoritative mutation remain closed until restore validation proves safe state.

The exact physical issuer/storage for that recovery fence remains later DUR/OPS/security work.

## 19. Accepted rule 16 — staged migrations and lifecycle separation

Persistence evolution follows a staged model equivalent to:

```text
EXPAND
-> resumable/idempotent MIGRATE / BACKFILL / TRANSFORM
-> VALIDATE
-> CUT OVER compatible readers/writers
-> CONTRACT only after rollback window closes
```

Semantic/ruleset/profile interpretation changes require explicit source/destination revisions and migration evidence. Existing data may not be silently reinterpreted.

Incompatible writers are fenced before destructive contract steps.

Character retirement, physical row deletion and privacy erasure are different operations/policies.

`CharacterId` is never reused after retirement or physical cleanup.

Ordinary semantic retirement is not implemented as an implicit cascade hard delete.

## 20. Accepted rule 17 — profile-neutral core only

The accepted partial baseline does **not** turn unresolved Reference/profile/operational details into schema invariants.

Remain downstream/gated:

- unresolved Reference values/formulas;
- exact naming normalization/recycling rules not yet evidenced;
- profile-specific PvP/world Character facts;
- profile-specific progression table layouts;
- item/currency/market/house persistence and conservation;
- exact lease/liveness/reconnect numeric policies except already accepted semantics such as the four-second protection;
- retention/retry/backup/RPO/RTO numeric values;
- Rust DB/migration technology choices.

A later profile extension can be declared persistence-complete only after its owning semantic contract defines the additional typed durable facts.

## 21. FND-04 and CharacterRevision interaction

The partial baseline preserves two independent durability layers:

```text
Character semantic state
-> CharacterRevision

playable-control/session authority
-> AccountPresenceClaim
-> CharacterLease generation
-> GameSessionId + connection_generation
-> RuntimeScopeAuthority generation
-> ControlLoss continuity
```

A gameplay mutation that changes Character semantic state must pass the current FND-04 authority/fencing checks **and** the Character semantic revision contract where applicable.

An FND-04 transition that changes only control/session authority does not mutate Character semantic state merely to obtain a convenient common revision.

This prevents stale-writer aliasing while avoiding false coupling of domain revision to connection churn.

## 22. Audit, privacy and Platform boundary

- Character Authority remains the semantic writer for native Character state.
- Platform remains AccountId/Identity/commercial authority and may orchestrate workflows, but native Platform direct Character-table writes remain forbidden.
- Cross-database foreign keys between Platform and game remain prohibited.
- AccountId/CharacterId/GameSessionId knowledge is not authorization.
- Reconnect credentials/proofs/nonces never enter ordinary audit/analytics.
- Pseudonymous analytics families may not fall back to raw AccountId/CharacterId; legitimate restricted player-linked/security audit remains governed by its explicit ANL privacy class.
- Normal support/GM correction must eventually use typed audited domain commands rather than ad-hoc raw SQL.

## 23. Still unresolved and deliberately outside this partial baseline

The following are not accepted by this owner decision:

- the remaining whole-gate `DUR-02 — Persistence v1` subjects outside the Character packet;
- exact SQL table/column/index/constraint/schema names;
- exact migration framework/library;
- ORM/query builder/Rust PostgreSQL client;
- connection-pool settings;
- exact physical scalar representation for CharacterRevision/lease/session generations where multiple equivalent representations satisfy accepted semantics;
- exact stable ruleset/content definition-key scalar encoding where the owning gate has not frozen it;
- profile-specific child table layouts;
- item/currency/market/house schema and DUR-03 conservation mechanics;
- partitioning/sharding;
- operational retention/backup/RPO/RTO/retry values;
- reconnect-secret hashing/KMS representation;
- exact non-rollback disaster-recovery fence implementation/issuer;
- physical GrantNonce/security-evidence relation layout;
- production topology;
- runtime persistence implementation.

## 24. Supersession and reopening

A later proposal may supersede a clause of this partial baseline only with named evidence such as:

- a proven correctness anomaly not closed by the accepted lock/constraint/authority-transition model;
- measured contention showing the one Character root revision anchor is an unacceptable bottleneck plus a proven equivalent partitioned fence;
- an accepted profile semantic contract requiring additional typed durable state;
- evidence that a deferred Reference value/formula constrains physical representation or atomicity;
- a PostgreSQL implementation limitation requiring an equivalent safer relation/locking design;
- security/privacy/restore evidence requiring stronger separation/fencing;
- an explicit later owner decision superseding ADR-0004 or this partial scope.

ORM defaults, convenience, current Global database shape, Canary/crystalserver tables or generic JSON flexibility are not sufficient supersession evidence.

## 25. No implementation or production authority

This owner acceptance does **not** authorize:

- PostgreSQL schema creation or migration execution;
- Rust persistence adapter/repository implementation;
- database provisioning/credentials;
- Character runtime persistence;
- session/lease runtime implementation;
- item/currency/market/house persistence;
- ruleset/content implementation;
- Platform direct writes or Platform database changes;
- production backup/restore configuration;
- production deployment or traffic;
- filling unresolved Reference rules from implementation convenience.

Any future implementation package must consume this partial baseline together with accepted DUR-01, ANL-01, GAME-CHAR, FND-04 and the still-open whole-DUR-02 decisions applicable to its claimed scope, and must provide the concurrency/crash/migration/E2E evidence required by the pre-decision packet.

## 26. Canonical status after lifecycle closeout

After this owner-baseline recording task is merged, archived and ownership is released:

```text
DUR-02 profile-neutral Character persistence
Owner baseline         = OWNER_ACCEPTED PARTIAL BASELINE
DeliveryStatus         = LIFECYCLE_CLOSED
ImplementationStatus   = NOT_STARTED
Runtime / DDL authority = NONE

DUR-02 overall
DecisionStatus         = PROPOSED
DeliveryStatus         = PLANNED
ImplementationStatus   = NOT_STARTED
Runtime authority      = NONE
```

The next DUR-02 architecture action is a separate **whole-gate Persistence-v1 reconciliation**: identify the historical subjects still genuinely owned by DUR-02 after later gate splits, accept or explicitly reassign them, and only then decide whether overall `DUR-02` can become `ACCEPTED`.
