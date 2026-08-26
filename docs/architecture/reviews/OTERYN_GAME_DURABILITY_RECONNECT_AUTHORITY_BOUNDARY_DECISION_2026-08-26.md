# Oteryn Game — Durability Reconnect Authority Boundary Decision

- Date: 2026-08-26
- Issue: #187
- Affected implementation lane: #167 / `OTV2-IMPL-DURABILITY`
- Decision ID: `DUR-RECONNECT-AUTHORITY-V1`
- Exact decision base: `main@2019d501d22614720ef37718e16913d81728e0a2`
- Status: **ACCEPTED WHEN THIS DECISION MERGES**
- Runtime / schema / migration / Cargo / workflow authority: **NONE**

## 1. Decision timing

**Must decide now: YES.**

The current synchronous `foundation::admission_facade::ReconnectAttemptJournal<T>` cannot carry all FND-04A/FND-04B final-revalidation facts, cannot persist an exact non-process-local candidate transport binding safely, and cannot satisfy the FND-03 enqueue/yield/completion model without synchronously waiting on SQLx. This blocks #167 and therefore Server Seam and its dependent client/QA/gameplay gates.

The decision freezes only the boundary required by the first journal-only Durability slice. It deliberately does not choose a production database deployment topology, reconnect-proof hash/KMS product, a general transaction/outbox model, item/value durability, or production listener configuration.

## 2. Options considered

### Option A — persist the current generic `T`

Rejected. `T` may be an fd/socket handle, pointer-like token, process-local slot or other value that has no safe meaning after process replacement. Persisting it would violate FND-03 recovery rules and would allow stale process-local transport identity to masquerade as durable authority.

### Option B — Foundation-owned versioned record + split-phase asynchronous Durability port

**Selected.** Foundation constructs and revalidates authority evidence. Durability stores the versioned record, performs exact fenced PostgreSQL CAS/transaction work, classifies the durable outcome and returns a typed completion. Runtime authority consumes that completion as a new FND-03 normalized input.

### Option C — let the synchronous journal call SQLx internally

Rejected. It would block the logical writer on DB/network work and violates FND-03 §12.4/§13.

### Option D — move reconnect eligibility/security authority into Durability

Rejected. Durability is a persistence/reconciliation owner, not the owner of FND-04 credential, controller, lease, runtime-scope or security/trust policy.

## 3. Ownership boundary

### Foundation owns

- authentication/proof-class validation;
- current AccountId -> CharacterId and CharacterId -> WorldId eligibility;
- controller-health / no-preemption decision;
- CharacterLease and RuntimeScopeAuthority interpretation;
- FND-02 reconciliation-safety interpretation;
- security/trust/revision evidence validation and anti-rollback floors;
- prepared and final commit eligibility;
- construction of the versioned durable request and final commit authorization;
- installing the returned durable result into the in-process authority projection only after successful durable classification.

### Durability owns

- PostgreSQL schema representation of the accepted versioned record;
- idempotency by `(GameSessionId, ReconnectAttemptRef)`;
- exact PREPARED record CAS and deadline persistence;
- exact COMMIT transaction/CAS against stored predecessor, lease and runtime-scope fences;
- durable attempt disposition and current session/controller binding;
- ambiguous/lost-response classification;
- restart-safe atomic read required by reconciliation;
- schema compatibility and migration safety already frozen by the Durability topology packet.

Durability never accepts a raw Platform credential as authority and never invents a new gameplay/session winner.

## 4. Stable transport reference

The first durable boundary introduces one Foundation-owned semantic value:

```text
AuthenticatedTransportRefV1 = 16 opaque non-zero bytes
```

Rules:

- minted exactly once by Foundation for one authenticated candidate transport before PREPARE;
- collision-resistant and never reused; the implementation uses a CSPRNG-backed 128-bit value or an equivalently strong non-reuse construction;
- canonical durable codec is exactly the 16 bytes; all-zero is invalid;
- equality-only: byte order provides no time, recency, priority or authority ordering;
- non-secret and **not bearer authority**; possession never authorizes reconnect;
- never derived from fd/socket number, pointer, memory address, local task id, source IP/port, `NodeId`, TLS object address or process-local slot;
- process-local transport tables may map `AuthenticatedTransportRefV1 -> live connection`, but a missing mapping after process restart means the physical transport is gone; the durable reference is used only for fencing/classification/reconciliation;
- an old reference can never be rebound to a different physical transport.

The persisted reference proves which logical authenticated candidate won; it does not resurrect a socket after process loss.

## 5. `ReconnectDurabilityRecordV1`

Foundation-to-Durability PREPARE uses one versioned immutable semantic record. Exact Rust field layout is implementation detail, but the following semantic members are mandatory and no member may be silently dropped:

```text
ReconnectDurabilityRecordV1
  version = 1

  identity
    GameSessionId
    ReconnectAttemptRef
    AccountId
    CharacterId
    WorldId
    RuntimeScopeRefV1        // Channel(WorldId, ChannelId) OR Instance(WorldId, InstanceId)

  connection fence
    predecessor_connection_generation
    candidate_connection_generation     // exact strict successor
    AuthenticatedTransportRefV1

  authority fence
    character_lease_generation
    scope_ownership_generation
    expected_session_state = RECONNECTABLE
    expected_no_current_controller = true

  continuity
    ControlLossEpochRefV1
    original_grace_deadline_unix_seconds
    prepared_deadline_unix_seconds
    protection_entitlement_state/ref

  proof
    proof_class = FAST_RECONNECT | REAUTHENTICATED_RECOVERY
    reconnect_proof_generation/fence when FAST_RECONNECT
    RecoveryGrantNonce replay key when REAUTHENTICATED_RECOVERY

  FND-02 reconciliation fence
    command high-water / pending-order evidence required by FND-02
    current server_sequence
    bounded typed domain revisions required for safe resume

  compatibility/security evidence
    protocol_major
    transport_profile
    ruleset_revision
    content_revision
    map_revision
    world_policy_revision
    account_security_generation
    authenticated Platform-security source revision/fence + source-observed time
    authenticated proof/key/profile trust source revision/fence + source-observed time
    credential expiration when the proof class has one
```

Existing Foundation identifier/revision types keep their already accepted canonical durable encodings. New scalar generations are non-zero uint64-class values with no wrap/reuse. The two deadline fields use signed Unix seconds, matching the existing FND-04 NumericDate time domain; an implementation that cannot establish trustworthy current time after restart treats the record as unusable/expired rather than extending a deadline.

`RuntimeScopeRefV1` is tagged and versioned so Channel and Instance remain distinct. `NodeId` is never stored as scope authority.

No bearer reconnect proof/JWT plaintext is part of this record. If a later implementation persists a secret verifier, that is a separately reviewed FND-04 secret-storage implementation choice; the journal record stores only the proof class and non-secret fence/replay identity required by this slice.

## 6. Split-phase asynchronous handoff

The synchronous journal is superseded **for durable implementation** by a split-phase semantic port. The old trait may remain temporarily as an in-memory/test compatibility adapter, but SQLx-backed Durability must not implement it by blocking inside the writer.

### Phase A — prepare request

During one normalized Foundation owner resolution:

1. prove PREPARE eligibility;
2. mint/bind `AuthenticatedTransportRefV1`;
3. construct `ReconnectDurabilityRecordV1`;
4. non-blockingly submit one bounded PREPARE persistence request;
5. transition the reconnect operation to explicit `PENDING_PREPARE` and yield the logical writer.

No DB connect/acquire/query/transaction wait occurs in that resolution.

Durability performs the PostgreSQL transaction asynchronously and returns exactly one typed completion: `PREPARED`, stable existing disposition, known reject, ambiguous/unavailable, or terminal error. That completion enters Foundation as a **new normalized authoritative input**.

### Phase B — final revalidation and commit authorization

Only after a PREPARED completion is accepted does Foundation perform the FND-04B final revalidation again using current facts. PREPARE is not trust escrow.

Foundation constructs `ReconnectCommitAuthorizationV1`, bound to the exact V1 record and exact attempt. It contains the freshly revalidated mutable evidence and one derived absolute authorization deadline:

```text
authorization_deadline = min(
  prepared_deadline,
  original_same_session_grace_deadline,
  credential_expiration_if_any,
  platform_security_source_observed_at + accepted 5s ceiling,
  trust_source_observed_at + accepted 5s ceiling
)
```

No new security freshness number is introduced; the 5-second ceiling is inherited unchanged from accepted FND-04A/B.

Foundation then non-blockingly submits COMMIT and yields again.

### Phase C — durable COMMIT/CAS

At transaction start Durability must prove:

- the stored PREPARED row is exact-byte/typed-field equivalent to the authorization target;
- attempt/session/predecessor/candidate/transport reference still match;
- session is still durably reconnectable with no current controller;
- CharacterLease and RuntimeScope ownership fences exactly match the authorization;
- candidate generation is still the strict successor;
- current trusted time is not later than `authorization_deadline`;
- no stored newer terminal/fence/commit state supersedes the candidate;
- recovery nonce replay state is available when that proof class requires consumption.

Only one PostgreSQL transaction may then atomically:

```text
mark the attempt COMMITTED
+ make candidate connection_generation current
+ bind AuthenticatedTransportRefV1 as current durable controller reference
+ fence predecessor connection/proof generation
+ consume the recovery replay key when applicable
+ preserve GameSessionId, CharacterLease and runtime-scope fences
+ preserve the exact FND-02 reconciliation fence
```

If the transaction outcome is unknown, Durability returns `AMBIGUOUS` and Foundation performs reconciliation by the same attempt; it never retries by inventing another candidate or assuming abort.

### Phase D — reconcile projection

The COMMIT/abort/ambiguous completion returns as another normalized Foundation input. Foundation atomically re-reads/reconciles the durable attempt + current session authority and only then installs the in-process controller projection. A stale old runtime generation cannot adopt a completion.

The durable database is therefore the crash-recovery source for the journal/session binding, while Foundation remains the semantic authority that decides whether a commit request may be issued.

## 7. Mandatory final revalidation set

`ReconnectCommitAuthorizationV1` is valid only after Foundation revalidates the complete FND-04B set at that moment:

1. exact candidate exists, is unexpired and bound to the same attempt/session/transport ref;
2. predecessor generation still matches;
3. session remains reconnect-eligible and original grace remains valid;
4. no healthy current controller regained authority;
5. AccountPresence still denotes the same CharacterId;
6. CharacterLease remains current/compatible;
7. RuntimeScopeRef + scope ownership generation remain current;
8. FND-02 command/server-sequence/domain-revision reconciliation remains safe;
9. no newer handoff/takeover/fence/terminal transition supersedes the candidate;
10. proof-specific mutable security/trust/revision/nonce facts remain current;
11. AccountId -> CharacterId is proven before CharacterId -> WorldId;
12. independent `protocol_major`, `transport_profile`, `ruleset_revision`, `content_revision`, `map_revision` and `world_policy_revision` still match.

Durability may compare/carry these values but does not reinterpret Platform credentials or downgrade this set.

## 8. ControlLoss continuity

`ControlLossEpochRefV1` is an internal non-zero non-reused uint64-class equality/fence value, not a public entity identifier. The durable record must retain the original epoch and original same-session grace deadline; retry, candidate replacement, process restart and runtime relocation never create a new epoch or extend that deadline.

The journal also retains whether the current loss epoch has an unused protection entitlement and the original activation/expiry state required by FND-04B. COMMIT may consume/activate that entitlement at most once; retry/lost response cannot create a second 4-second window.

## 9. Bounded retained state

The first slice freezes two semantic hard bounds:

```text
live PREPARED candidates per GameSession = 1
retained distinct reconnect attempts per open ControlLossEpoch = 8
```

These are safety/idempotency cardinalities, not performance tuning knobs.

Rules:

- retry of the same `ReconnectAttemptRef` consumes no additional slot;
- the ninth distinct attempt in the same open loss epoch fails before new durable allocation with `CAPACITY_EXCEEDED` / the Foundation reconnect-capacity equivalent;
- at most one of the retained attempts may be `PREPARED`; a second live candidate is `RejectedConcurrent`/equivalent;
- every live PREPARED record has a finite `prepared_deadline`;
- after the loss epoch closes by successful control restoration or terminality, non-winning per-attempt rows may be compacted into closed-epoch terminal evidence because every non-winning lookup is then deterministically `TerminallySuperseded`; the committed winner remains replayable for lost-response reconciliation as required;
- integer overflow or inability to prove the epoch/slot count fails closed.

A serialized coordinator registry PR must register the `8 attempts / open ControlLossEpoch` hard maximum before the corresponding Foundation child implementation is accepted. The `1 live PREPARED / GameSession` bound is an authority-state invariant already represented by the existing kernel and is not configurable.

## 10. Implementation sequencing and exact ownership

This decision does **not** authorize #167 to edit Foundation paths directly.

The safe sequence is:

### Child 1 — Foundation reconnect durability boundary

Create one separate High/XHigh implementation allocation that owns only the exercised Foundation paths, expected to include:

```text
apps/game-server/src/foundation/admission.rs
apps/game-server/src/foundation/admission_facade.rs
apps/game-server/src/foundation/admission_recovery_inner.rs
apps/game-server/src/foundation/fnd04_verifier.rs
apps/game-server/src/foundation/recovery_tests.rs
```

Its job is to introduce the V1 semantic record/ref, expose the complete evidence that the current verifier discards, and expose the split-phase request/completion contract. It must not implement SQLx, migrations or the game-server Durability module.

A serialized `RESOURCE_LIMITS_REGISTRY.json` lease registers the retained-attempt bound before that child is accepted.

### Child 2 — existing Durability #167

Only after Child 1 merges and its Foundation lease is released may the existing `impl/game-durability-journal` task receive a fresh exact base/allocation and implement its already owned paths:

```text
apps/game-server/src/durability/**
apps/game-server/src/bin/oteryn-game-migrate.rs
apps/game-server/migrations/**
apps/game-server/build.rs
apps/game-server/tests/durability_postgres.rs
apps/game-server/tests/support/postgres.rs
```

Its scope remains journal-only. DUR03-RL-01..08, transactions/outbox/items/value/rewards remain excluded fail-closed.

### Child 3 — serialized composition only if needed

`apps/game-server/src/lib.rs` remains a later coordinator-owned serialized composition lease. Server Seam stays `WAITING_DEPENDENCY` until the real durable adapter is merged and composed.

## 11. Required evidence before merge of implementation children

Foundation child:

- focused tests for 16-byte transport-ref codec/non-reuse/all-zero rejection;
- PREPARE/COMMIT evidence completeness tests;
- stale controller/lease/runtime/security/revision/FND-02 fence tests;
- 1-live-prepared and 8/9 distinct-attempt boundary tests;
- split-phase proof that no DB/network wait occurs in the logical writer contract;
- independent exact-head security/authority review.

Durability child:

- real isolated PostgreSQL E2E;
- migration fresh/ahead/behind/checksum/dirty/locking/runtime-DDL denial;
- exact PREPARED CAS and one-live-candidate enforcement;
- 8/9 retained-attempt boundary at the DB layer;
- lost COMMIT response, ambiguous commit and process-recovery classification;
- stale lease/scope/controller/authorization-deadline rejection;
- recovery nonce single-consumption where exercised;
- exact-head persistence/fencing review.

## 12. Failure behavior

Missing/unknown version, malformed transport ref, missing required evidence, expired prepared/authorization deadline, untrusted current time, stale security evidence, stale lease/scope/controller state, unsafe FND-02 reconciliation, capacity exhaustion, DB unavailability or ambiguous commit all fail closed. None advance `connection_generation`, revive predecessor authority, consume a recovery nonce as success, activate protection, or manufacture a new GameSession.

## 13. Deliberately deferred

This decision does not freeze:

- concrete Rust async channel/executor library;
- SQL query layout beyond the already accepted SQLx/PostgreSQL topology;
- secret reconnect-proof hash/KMS product;
- numeric prepared **duration** or same-session grace duration;
- production DB pool size/connection count;
- item/value transaction resources or DUR03-RL-01..08;
- listener/TLS deployment or Server Seam implementation.

## 14. Supersession evidence

Reopen this boundary only with evidence that the V1 record cannot preserve FND-04 authority/fencing, that the split-phase port creates an unavoidable correctness race, that the 8-attempt bound causes material valid-player recovery failure under measured production-like reconnect evidence, or that a later accepted FND-04 revision changes the required authority facts.

## 15. Resolution

When this document is merged with required review/CI:

```text
Issue #187 architecture question = RESOLVED
OTV2-IMPL-DURABILITY architecture hold = replaced by WAITING_DEPENDENCY on the Foundation boundary child
Work coordinator = authorized to allocate the Foundation child and serialized retained-attempt registry lease
Server Seam = remains WAITING_DEPENDENCY until the actual Durability adapter merges
```

No owner/product decision remains inside #187 after this merge.