# Oteryn Game — Durable Transport Reference Uniqueness Protocol Decision

- Date: 2026-08-26
- Issue: #197
- Affected implementation lane: #192 / Foundation reconnect durability boundary
- Downstream lanes: #167 / Durability, Server Seam, compatible Client/QA, Movement, Combat
- Decision ID: `DUR-RECONNECT-TRANSPORT-REF-UNIQUENESS-V1`
- Exact decision base: `main@f31453f65477ae9966d724d67bdd2c1857318be1`
- Status: **ACCEPTED WHEN THIS DECISION MERGES**
- Runtime / schema / migration / Cargo / workflow / registry authority: **NONE**

## 1. Decision timing

**Must decide now: YES.**

`DUR-RECONNECT-AUTHORITY-V1` requires `AuthenticatedTransportRefV1` to be collision-resistant, never reused and checked against durable uniqueness before publication, while Foundation must never block its FND-03 logical writer on database or network work. The accepted split-phase contract did not freeze the exact durable reservation phase or bounded collision behavior. Foundation Issue #192 therefore cannot finish safely, Durability #167 cannot consume the Foundation boundary, and Server Seam remains dependency-gated.

This decision freezes only the missing first-slice uniqueness protocol. It does not redesign reconnect authority, change the existing `ReconnectDurabilityRecordV1`, choose a production database topology, add a new resource limit, or authorize runtime/schema implementation.

## 2. Partial supersession of `DUR-RECONNECT-AUTHORITY-V1`

This decision preserves the existing owner-accepted `DUR-RECONNECT-AUTHORITY-V1` except for the collision ambiguity below.

It supersedes only the Section 4 rule that can be read as allowing Foundation to repeatedly remint the transport reference inside one `ReconnectAttemptRef` after durable collision, specifically the phrase:

```text
checks it against the durable uniqueness set, and retries generation on collision before publishing it
```

and the corresponding unspecified collision handling in Phase A of Section 6.

The replacement rule is:

```text
one ReconnectAttemptRef -> exactly one AuthenticatedTransportRefV1 candidate
successful PREPARE CAS -> durable uniqueness reservation
transport-ref collision -> terminal disposition for that ReconnectAttemptRef
replacement transport ref -> only on a new ReconnectAttemptRef
```

All FND-04/FND-02 authority evidence, final revalidation, PREPARE -> COMMIT -> RECONCILE semantics, deadlines, one-live-PREPARED rule, fencing and failure behavior from `DUR-RECONNECT-AUTHORITY-V1` remain binding.

## 3. Constraints

The selected protocol must preserve all of these accepted invariants:

- Foundation owns reconnect eligibility, proof/security/trust/revision validation, controller/lease/runtime-scope interpretation and final COMMIT authorization.
- Durability owns PostgreSQL CAS/transaction work and durable result classification.
- Foundation performs no synchronous database/network wait while holding one FND-03 logical-writer resolution.
- `AuthenticatedTransportRefV1` is 16 opaque non-zero bytes, equality-only, non-secret and never bearer authority.
- A process-local transport table is not durable uniqueness proof.
- A successfully reserved transport ref is never rebound to another physical transport.
- A PREPARED record is never silently mutated to recover from a later collision.
- `AMBIGUOUS` never authorizes minting a new candidate until the exact existing attempt is durably reconciled.
- same-attempt retry remains idempotent.
- retained distinct reconnect attempts per open `ControlLossEpoch` remain bounded by registered `FND04-RECONNECT-ATTEMPTS-PER-LOSS-EPOCH = 8`.

## 4. Options considered

### Option A — separate asynchronous transport-ref reservation phase

Foundation would submit `RESERVE_TRANSPORT_REF`, yield, receive a reservation completion, then later submit PREPARE.

Rejected for the first slice. It adds another durable state machine, another request/completion family, another DB round trip and another lost-response boundary without protecting an invariant that cannot be protected inside PREPARE itself.

### Option B — PREPARE reserves the ref, but Foundation remints inside the same attempt after collision

This keeps one DB phase, but a safe implementation needs a crash-stable same-attempt collision counter or candidate ordinal and an additional hard maximum. Without durable counting, process restart could reset the retry budget; with durable counting, the first slice gains extra persistent protocol/state solely for an astronomically rare CSPRNG collision.

Rejected for the first slice.

### Option C — PREPARE CAS reserves the ref; collision terminally rejects that attempt

**Selected.**

Each `ReconnectAttemptRef` carries exactly one `AuthenticatedTransportRefV1`. The PREPARE transaction is the single atomic durable reservation point. A collision is a durable terminal disposition for that attempt. If reconnect remains eligible, Foundation may start a new distinct reconnect attempt with a new transport ref, subject to the already registered eight-attempt loss-epoch ceiling.

This preserves one DB round trip, one idempotency key, one existing cardinality budget and one durable owner.

## 5. Single durable uniqueness owner

Durability is the sole owner of the durable transport-ref uniqueness set.

Foundation may generate a CSPRNG-backed 128-bit candidate and bind it to the live physical transport in process-local memory, but before successful PREPARE completion that binding is only an **unpublished candidate** and carries no durable/current-controller authority.

Durability must prove uniqueness atomically in the same PostgreSQL transaction/CAS that establishes the accepted PREPARED record.

A successful PREPARE transaction therefore atomically establishes:

```text
unique durable reservation of AuthenticatedTransportRefV1
+ exact ReconnectDurabilityRecordV1 PREPARED state
+ exact GameSessionId / ReconnectAttemptRef idempotency state
+ all already-required predecessor / lease / scope / deadline fences
```

There is no separate synchronous Foundation reservation API and no process-local uniqueness set that can satisfy this contract.

## 6. Publication boundary

The transport ref has three semantic stages:

```text
UNPUBLISHED_CANDIDATE
  Foundation has minted/bound it locally, but durable uniqueness is not proven.

RESERVED_PREPARED
  PREPARE completion proves the exact ref was durably reserved for the exact attempt.
  It may now identify the PREPARED candidate, but it is not current controller authority.

CURRENT_CONTROLLER
  only a successful durable COMMIT/CAS may make the ref the current durable controller reference.
```

A candidate must not be exposed as accepted/current reconnect authority before the corresponding successful PREPARE completion. Local routing needed to correlate the physical connection with the pending attempt is allowed but remains non-authoritative.

## 7. Durable non-reuse invariant

A successfully reserved `AuthenticatedTransportRefV1` is never reassigned.

The logical durable uniqueness set is monotonic across attempts and process restarts. Compaction of bulky reconnect-attempt payloads may not make a successfully reserved ref reusable. A future implementation may compact old rows to a minimal uniqueness tombstone containing only the data necessary to preserve non-reuse and audit provenance.

The exact table/index/tombstone representation is a Durability implementation detail for #167, but the database must enforce an atomic uniqueness guarantee equivalent to a unique constraint/ledger. Application-side `SELECT then INSERT` without database-enforced race exclusion is insufficient.

## 8. PREPARE request identity

The semantic PREPARE request remains version 1 and is identified by the exact immutable V1 record:

```text
ReconnectPrepareRequestV1
  record: ReconnectDurabilityRecordV1
```

Its idempotency key remains:

```text
(GameSessionId, ReconnectAttemptRef)
```

The exact request identity includes the complete immutable record, including `AuthenticatedTransportRefV1`.

A retry with the same `(GameSessionId, ReconnectAttemptRef)` but a different transport ref or different immutable record is an integrity/idempotency conflict and fails closed. Foundation may never use same-attempt retry as a vehicle for reminting the ref.

## 9. PREPARE transaction order

For a new attempt, Durability must perform all rejectable checks it can prove before consuming a new durable ref reservation, including applicable attempt-capacity, one-live-PREPARED, predecessor/session and accepted durable-fence checks.

The final durable PREPARE transaction then atomically attempts the transport-ref reservation and PREPARED creation.

Required outcomes:

### Reservation succeeds

The transaction commits the unique reservation plus exact PREPARED record. Completion is `PREPARED`.

### Transport-ref uniqueness collision

The existing owner/reservation of the collided ref is unchanged.

The new attempt must not become PREPARED and must not become current controller authority. Durability records a durable terminal collision disposition keyed by the exact `(GameSessionId, ReconnectAttemptRef)` so a lost collision response or same-attempt retry is deterministic.

The exact schema used to retain collision evidence is an implementation detail; it must not require assigning the collided ref to two uniqueness owners.

### Transaction definitely does not commit

No new successful ref reservation exists. Retry uses the same exact attempt and same exact transport ref.

### Transaction outcome is unknown

Completion is `AMBIGUOUS`. Foundation freezes the exact attempt/ref and reconciles it. It does not mint another ref and does not create another attempt until durable classification proves that the exact attempt reached a terminal state from which a new attempt is allowed.

## 10. Typed PREPARE completion/disposition

The semantic completion family must distinguish at least:

```text
ReconnectPrepareCompletionV1
  PREPARED
  EXISTING_PREPARED
  REJECTED_TRANSPORT_REF_COLLISION
  REJECTED_CONCURRENT_PREPARED
  REJECTED_STALE_AUTHORITY
  ATTEMPT_CAPACITY_EXCEEDED
  EXISTING_TERMINAL
  UNAVAILABLE
  AMBIGUOUS
  IDEMPOTENCY_CONFLICT
```

Exact Rust enum names may differ, but these classes may not be collapsed in a way that permits unsafe retry behavior.

### `PREPARED`

The exact request committed and the transport ref is durably reserved.

### `EXISTING_PREPARED`

Idempotent replay of the same exact attempt/record; return the existing PREPARED state. No additional slot/ref is consumed.

### `REJECTED_TRANSPORT_REF_COLLISION`

Terminal for this `ReconnectAttemptRef`. No PREPARED state or current-controller mutation occurs for this attempt.

### `REJECTED_CONCURRENT_PREPARED`

The existing one-live-PREPARED invariant blocked this candidate. Do not reserve/publish a new ref for the rejected attempt.

### `REJECTED_STALE_AUTHORITY`

Known fail-closed rejection under already accepted session/lease/scope/controller/fence rules.

### `ATTEMPT_CAPACITY_EXCEEDED`

The open loss epoch has exhausted the registered eight-distinct-attempt budget. Fail before allocating a new durable attempt/ref reservation.

### `EXISTING_TERMINAL`

Idempotent replay returns the already durable terminal disposition, including a previous transport-ref collision where applicable.

### `UNAVAILABLE`

No new candidate/ref is created. Foundation may retry only the same exact attempt/ref request when policy permits.

### `AMBIGUOUS`

No new candidate/ref or attempt may be created. Reconcile the same exact attempt until durable state is known.

### `IDEMPOTENCY_CONFLICT`

Same idempotency key with different immutable intent/record. Treat as integrity/security failure; never repair by reminting.

## 11. Collision recovery and existing resource bound

A transport-ref collision is terminal for that `ReconnectAttemptRef`.

Foundation may mint a replacement transport ref **only by creating a new `ReconnectAttemptRef`**, and only after the collision completion is accepted as a new normalized input and current Foundation authority proves reconnect remains eligible.

The new attempt:

- receives a newly minted `AuthenticatedTransportRefV1`;
- reconstructs/revalidates PREPARE eligibility rather than inheriting trust escrow;
- consumes one additional distinct-attempt slot in the same open `ControlLossEpoch`;
- remains subject to the existing one-live-PREPARED invariant;
- reuses proof/security evidence only if that evidence independently remains current under FND-04 rules.

No new collision-specific numeric limit is required.

The already registered:

```text
FND04-RECONNECT-ATTEMPTS-PER-LOSS-EPOCH = 8
```

is the correct bound because each collision replacement is deliberately a **new distinct reconnect attempt**, not a same-attempt retry. Same-attempt retries remain idempotent and consume no additional slot.

Therefore repeated collisions can never form an unbounded logical-writer or persistence loop: one collision completion may cause at most one newly scheduled distinct attempt, and the ninth distinct attempt is rejected before durable allocation.

If the eighth retained attempt terminates with a transport-ref collision, no collision-recovery attempt remains in that loss epoch.

## 12. No collision loop inside the logical writer

Foundation must not execute a `while collision { mint; durable_check; }` loop in one logical-writer resolution.

The only allowed flow is:

```text
normalized Foundation input
  -> mint exactly one ref for exactly one new attempt
  -> non-blocking PREPARE submit
  -> yield

PREPARE completion arrives as a later normalized input
  -> if collision and still eligible/capacity remains:
       create one new attempt + one new ref
       submit one new PREPARE
       yield
```

Every durable round trip crosses the existing asynchronous completion boundary.

## 13. Unavailable and ambiguous rules

`UNAVAILABLE` is not permission to create a different candidate. Retry, if allowed, is the same exact `(GameSessionId, ReconnectAttemptRef, AuthenticatedTransportRefV1, immutable record)`.

`AMBIGUOUS` freezes the exact attempt. The only next action is durable reconciliation of that attempt/ref.

A new attempt/ref may begin only after reconciliation proves a terminal pre-PREPARED result for which Foundation remains eligible to try again. Reconciliation that proves PREPARED, COMMITTED or another live/terminal authority outcome follows the existing V1 path and never remints.

This rule prevents an unknown PREPARE commit from creating two durable candidates.

## 14. COMMIT semantics remain unchanged

This decision does not change Phase B final Foundation revalidation, `ReconnectCommitAuthorizationV1`, authorization deadline derivation or Phase C COMMIT/CAS.

A successful PREPARE completion has already proven that its exact transport ref is durably unique/reserved. COMMIT may bind only that exact ref and exact attempt. A different ref requires a different attempt and a new PREPARE lifecycle.

No collision handling occurs by mutating a PREPARED or COMMITTING record.

## 15. Concurrency and crash invariants

The implementation must prove these cases:

### Two new attempts race with the same transport ref

At most one durable reservation can succeed. The other attempt receives/durably reconciles `REJECTED_TRANSPORT_REF_COLLISION`. No application-side race can make both PREPARED.

### Process dies after PREPARE commits but before completion delivery

Reconciliation returns the existing PREPARED state and same reserved ref. Foundation must not mint another ref.

### Process dies after collision disposition commits but before completion delivery

Reconciliation returns the terminal collision for the same attempt. Only then may a new distinct attempt/ref be created, subject to current eligibility and capacity.

### Same attempt is replayed with a different ref

Return `IDEMPOTENCY_CONFLICT`; fail closed.

### Old successfully reserved ref appears after attempt/session compaction

The uniqueness tombstone/ledger prevents reassignment. Missing process-local physical mapping never authorizes rebinding.

## 16. Foundation #192 implementation consequences

After this decision merges and the coordinator reconciles #192 against current protected main, Foundation #192 may implement only its already allocated Foundation paths.

Foundation must provide/test:

- exactly one `AuthenticatedTransportRefV1` mint per `ReconnectAttemptRef`;
- same-attempt immutable ref/record identity;
- typed handling for collision, unavailable, ambiguous, existing-prepared and idempotency-conflict completions;
- no synchronous uniqueness-reservation port;
- no same-attempt collision remint;
- creation of a new attempt only after terminal collision classification and only while the existing eight-attempt capacity permits;
- no new ref after PREPARED/AMBIGUOUS/COMMIT paths;
- full FND-04 revalidation for every new attempt and before COMMIT as already required.

Foundation does not implement SQLx or durable uniqueness storage.

## 17. Durability #167 implementation consequences

Durability #167 remains `WAITING_DEPENDENCY` until #192 merges and its Foundation ownership is released.

On its later fresh exact-base allocation it must implement/test, inside its already accepted journal-only scope:

- one database-enforced durable uniqueness owner for transport refs;
- atomic reservation as part of PREPARE CAS;
- durable terminal collision classification keyed by the exact attempt;
- exact same-attempt replay semantics;
- no duplicate reservation under concurrent PostgreSQL transactions;
- ambiguous/lost-response reconciliation for both successful PREPARE and terminal collision;
- successful-ref non-reuse after attempt compaction;
- one-live-PREPARED and existing 8/9 attempt limits at the DB layer;
- no current-controller mutation until the separately authorized COMMIT transaction.

The exact SQL table/index/tombstone layout remains implementation detail subject to the existing migration/schema allocation and independent persistence/fencing review.

## 18. Required evidence before implementation merge

### Foundation #192

- RED/GREEN proof that same-attempt ref replacement is rejected;
- exact one-ref-per-attempt codec/non-zero tests;
- collision completion -> new attempt path under retained capacity;
- collision on final available attempt -> no replacement;
- `UNAVAILABLE` -> same exact request only;
- `AMBIGUOUS` -> reconciliation only;
- idempotency conflict for same attempt/different ref or record;
- proof no DB/network wait occurs in the logical writer;
- existing FND-04 stale authority/security/revision/fence negative tests;
- Cargo 1.94 focused/component/workspace validation;
- genuinely independent exact-head security/authority review with no unresolved P0/P1/P2 findings.

### Durability #167

- real isolated PostgreSQL race test: two different attempts request the same 16-byte ref and exactly one reservation can become PREPARED;
- collision terminal receipt survives lost response/restart;
- same-attempt replay is deterministic;
- successful reservation tombstone survives compaction and prevents reuse;
- no reservation leak on known rolled-back pre-PREPARED rejection;
- ambiguous PREPARE result reconciles before any new candidate;
- existing migration compatibility, deadline/fencing, 8/9 capacity and one-live-PREPARED tests;
- genuinely independent exact-head persistence/fencing review.

## 19. Security, player and operational trade-offs

### Security

The design removes an unbounded retry surface and ensures only PostgreSQL can establish cross-process uniqueness. Same-attempt immutable intent prevents a caller from changing transport identity under one idempotency key.

### Player experience

A genuine random 128-bit collision is extraordinarily unlikely. If it occurs, the reconnect may transparently continue as a new bounded attempt while the same ControlLoss epoch remains valid. Ordinary network retries do not spend another attempt because same-attempt replay remains idempotent.

### Operations

No extra DB round trip is added to every reconnect. Successful reservations require a durable non-reuse ledger/tombstone that can be compact compared with full attempt records. Collision metrics should be observable because any repeated collision is strong evidence of RNG, corruption or adversarial problems, but alert thresholds are deliberately not frozen here.

## 20. What becomes harder later

This choice makes `ReconnectAttemptRef` the immutable unit containing exactly one transport-ref candidate. A future protocol that wants multiple transport candidates under one durable attempt would require an explicit versioned successor and migration/reconciliation rules.

That cost is accepted because the first slice gains much simpler crash/idempotency semantics and reuses an existing hard cardinality bound instead of inventing another retry state machine.

## 21. Evidence that would justify superseding this decision

A later decision may replace this protocol if evidence shows one or more of:

- measured collision/reconnect behavior makes new-attempt collision recovery materially harmful;
- the durable non-reuse tombstone set creates unacceptable storage/index cost;
- a second immediate consumer requires an independent reservation service/phase;
- a stronger stable-ID construction can mathematically/operationally eliminate the durable uniqueness lookup while preserving never-reuse guarantees;
- production fault evidence shows PREPARE-coupled reservation prevents safe recovery or availability targets.

Supersession must be versioned and must preserve old attempt/ref reconciliation semantics for already durable V1 data.

## 22. Deliberately not decided

This decision does not freeze:

- exact PostgreSQL table/index names or physical schema layout;
- the Rust CSPRNG crate/API used to produce the 16 bytes;
- production database sizing/index maintenance strategy;
- telemetry/alert numeric thresholds for collision detection;
- a general durable ID service;
- listener/transport lifecycle beyond the existing Foundation contracts;
- production deployment/secrets/credential topology;
- any new gameplay, item/value, transaction/outbox or external-repository semantics.

## 23. Implementation sequence after merge

```text
1. merge/read back this #197 architecture decision
2. Work coordinator reconciles #192 task/allocation against current main
3. Foundation #192 resumes on a fresh exact base and implements the V1 semantic port/tests
4. independent exact-head Foundation security/authority review + CI
5. Foundation #192 merges and releases its paths
6. Durability #167 receives a fresh exact-base allocation
7. Durability implements PostgreSQL reservation/PREPARE/collision/reconciliation semantics
8. independent exact-head persistence/fencing review + real PostgreSQL E2E
9. only then re-evaluate Server Seam readiness
```

No implementation worker receives authority merely from this unmerged decision document.
