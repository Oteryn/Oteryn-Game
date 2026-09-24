# Oteryn Game — Fresh-Admission Durability Authority Decision

- Date: 2026-09-05
- Source escalation: Issue #313
- Affected lane: Issue #247 / `OTV2-INTEGRATION-GAMEPLAY-SERVER-SEAM`
- Decision ID: `FND-DUR-FRESH-ADMISSION-V1`
- Exact decision base: `main@5639dc28c3ac27b7da2772778f71d797cfd60537`
- Status: **CANDIDATE — ACCEPTED ONLY AFTER REVIEWED PROTECTED-MAIN INTEGRATION**
- Runtime / schema / migration / Cargo / workflow / production authority: **NONE**

## 1. Architecture-resolution packet

```yaml
classification: ARCHITECTURE_RESOLUTION
main_sha: 5639dc28c3ac27b7da2772778f71d797cfd60537
source_escalation: 313
blocking_question: >-
  What is the smallest ownership-correct production-durable fresh-admission /
  initial GameSession boundary that preserves FND-03 enqueue/yield semantics,
  FND-04A atomic final authority, existing reconnect decisions and DUR-02
  immutable forward migration rules?
facts:
  proven:
    - current Foundation production-facing compatibility trait exposes synchronous ReconnectAttemptJournal<T>::commit_fresh and lifecycle methods
    - current PostgreSQL Durability implementation is an asynchronous reconnect PREPARE/COMMIT/reconciliation adapter, not a production fresh-admission adapter
    - released migration 0001 models reconnect/session/control-loss state and must not be edited in place
    - FND-04A requires one-time GrantNonce consumption plus AccountPresence/CharacterLease/GameSession/initial connection authority at one final game-domain linearization boundary
    - FND-03/Durability architecture forbids waiting on SQLx/database/network work while the logical writer lane is held
    - Server Seam remains WAITING_ARCHITECTURE and its preserved worker branch must not cross this boundary
  derived:
    - production fresh admission must use an asynchronous Foundation-to-Durability command/completion boundary rather than implementing the synchronous compatibility trait with SQLx or block_on
    - the existing durable current-session row can be evolved by forward migration to support an initial ACTIVE origin without fabricating reconnect/control-loss history
    - a durable immutable fresh-admission receipt keyed by the existing typed FreshAdmissionReplayKey is sufficient for retry/lost-response classification when coupled atomically to the current GameSession row
  unknown:
    - final Rust field layout and internal module helper layout inside the exact surfaces allocated below
    - physical retention/compaction policy for terminal historical fresh-admission receipts after replay safety can be proved
    - production deployment/database credential topology beyond already accepted Durability contracts
  conflict: []
accepted_decision: FND-DUR-FRESH-ADMISSION-V1
rejected_options:
  - synchronous SQLx or block_on behind ReconnectAttemptJournal<T>::commit_fresh
  - transport-local or process-local production admission journal
  - representing fresh admission as a fake reconnect/control-loss attempt
  - editing released migration 0001 in place
  - moving FND-04 verification/current-authority interpretation into Durability
  - allowing retry to mint a second canonical GameSession after an ambiguous or lost response
  - letting a committed historical receipt reacquire current controller authority without independently current facts
affected_contracts:
  - FND-03
  - FND-04A
  - FND-04B / DUR-RECONNECT-AUTHORITY-V1 preserved
  - DUR-01
  - DUR-02
  - accepted Durability topology packet
affected_paths:
  architecture_only_this_candidate:
    - docs/architecture/reviews/OTERYN_GAME_FRESH_ADMISSION_DURABILITY_AUTHORITY_DECISION_2026-09-05.md
    - docs/agents/tasks/active/OTV2-20260905-fresh-admission-architecture-313.md
  implementation_surfaces: see_section_10
implementation_owner: >-
  fresh coordinator allocations: Foundation semantic child first, then Durability
  persistence child, then a producer/composition integration child; existing Server
  Seam worker remains paused until all three are protected-main integrated and Work explicitly resumes it
resource_values_changed: false
stable_ids_changed: false
protocol_changed: false
schema_or_migration_required: true
production_authority_changed: false
cross_repository_authority_changed: false
supersedes:
  - only the unresolved fresh-admission implementation gap recorded by Issue #313
preserves:
  - FND-04A authority semantics
  - DUR-RECONNECT-AUTHORITY-V1 reconnect semantics and existing V1/V2 reconnect durable records
  - existing Server Seam transport/profile decision
  - immutable released migration history
validation:
  architecture_self_review: required
  repository_checks: required
  runtime_e2e_for_architecture_candidate: NOT_APPLICABLE
independent_review:
  required: true
  reason: admission/session/persistence authority boundary
next_action: >-
  Oteryn: work coordinator qualifies and integrates this architecture candidate
  through the normal protected review/check/Merge Queue lifecycle before allocating
  any Foundation or Durability implementation child.
```

## 2. Decision timing

**Must decide now: YES.**

Concrete downstream work blocked: Server Seam Task 3 cannot create a truthful production `GameSession`, and Client / physical Tier-1/Tier-2 QA / Movement / Combat remain transitively blocked on that seam.

What becomes harder later if chosen incorrectly: a synchronous database call in the FND-03 writer, a second process-local session owner, or a fake reconnect row would bake unsafe authority and recovery semantics into the first real listener and make later crash/reconnect correction invasive.

Evidence that may supersede this decision: a reviewed Foundation/Durability provider that proves equivalent one-time grant consumption, single-winner session creation, independently-current post-commit authority and lost-response reconciliation without blocking the logical writer. Such a successor must explicitly state which V1 rules remain binding.

Deliberately not decided: production database placement, secrets/credentials, deployment, final receipt retention/compaction horizon, new gameplay command/state IDs, listener addresses/ports, QUIC, product policy, or any new resource-limit number.

## 3. Selected boundary

Use one **Foundation-owned versioned fresh-admission authorization plus asynchronous Durability COMMIT/reconciliation port**.

```text
verified fresh credential + independently current game facts
        |
        v
Foundation final revalidation against published current authority guards
        |
        +-- build immutable FreshAdmissionCommitAuthorizationV1
        +-- candidate GameSessionId (not canonical until durable commit)
        +-- AuthenticatedTransportRefV1
        |
        v
bounded persistence submission -> YIELD FND-03 writer
        |
        v
Durability PostgreSQL transaction
        |
        +-- acquire all authority/claim/replay/session/transport serialization locks
        +-- L: final current-fence and trusted-time decision; reject stale authorization
        +-- establish AccountPresence / CharacterLease atomically
        +-- reserve transport ref in the shared fresh/reconnect uniqueness set
        +-- classify replay key
        +-- enforce account/character single-winner constraints
        +-- create current ACTIVE GameSession generation 1
        +-- consume replay key by immutable receipt
        +-- durable COMMIT confirms all conditional effects; never a new authorization
        |
        v
typed completion / atomic reconcile snapshot
        |
        v
new normalized Foundation input
        |
        +-- independently resolve current authority again
        +-- install/bind transport only when still current
        +-- otherwise fail closed and reconcile existing durable authority
```

There is **no durable fresh-admission PREPARE row** in V1. Fresh admission has one transaction containing the final guarded authorization decision `L` and the conditional atomic effects, followed by durable COMMIT. Foundation's earlier revalidation is eligibility only; it is not `L`. This is intentionally smaller than reconnect PREPARE/COMMIT because there is no predecessor controller to replace and no reconnect candidate whose durable PREPARED state must survive a two-step handoff.

The absence of a fresh PREPARE row does not permit waiting on the database in the logical writer. Submission remains asynchronous; the writer yields and consumes the completion as a later normalized input.

## 4. Ownership

### Foundation owns

- parsing/authentication and FND-04A classification;
- current Platform-security and signing-key/profile trust evidence interpretation;
- current AccountId -> CharacterId ownership before CharacterId -> WorldId eligibility;
- current route/runtime target, protocol/transport and independent rules/content/map/world-policy/offer revision interpretation;
- GrantNonce semantic eligibility;
- AccountPresence, CharacterLease and RuntimeScope authority interpretation;
- the candidate `GameSessionId` issuance rule;
- construction of `FreshAdmissionCommitAuthorizationV1` only after complete final revalidation;
- authenticated publication of game-current guard state and commit-before-publish activation of relevant authority changes under Section 6.1;
- interpreting a durable completion as a new FND-03 normalized input;
- independently-current post-commit adoption of the current durable session/controller projection.

### Durability owns

- physical PostgreSQL representation;
- atomic replay-key consumption/receipt;
- single-winner nonterminal AccountId and CharacterId constraints;
- atomic creation of the canonical current `GameSession` row at connection generation 1;
- exact idempotent replay classification;
- ambiguous/lost-response reconciliation;
- restart-safe atomic read of immutable fresh receipt plus current session authority;
- forward-only migration and schema compatibility.

Durability may compare/store typed Foundation-provided fences and deadlines. It does **not** authenticate Platform credentials, reinterpret account/world/runtime eligibility, choose a winner from stale facts, or manufacture current authority from persisted evidence.

## 5. Versioned Foundation semantic surface

The Foundation child introduces one explicit V1 semantic boundary. Exact Rust field ordering is implementation detail; these semantics are mandatory.

### `FreshAdmissionCommitAuthorizationV1`

```text
version = 1

replay identity
  FreshAdmissionReplayKey       // existing tagged 33-byte durable encoding

candidate identity
  candidate GameSessionId       // canonical only if COMMIT succeeds
  AccountId
  CharacterId
  WorldId
  ChannelId

initial authority binding
  character_lease_generation
  scope_ownership_generation
  connection_generation = 1
  AuthenticatedTransportRefV1   // existing accepted 16-byte equality-only ref

final FND-04A evidence fences
  account_security_generation/state fence
  Platform-security evidence source revision + decision identity + source_observed_at
  admission trust/profile evidence source revision + decision identity + source_observed_at
  route_revision
  runtime_observation_revision
  protocol_major
  transport_profile
  ruleset_revision
  content_revision
  map_revision
  world_policy_revision
  offer_revision
  credential expiration / accepted final authorization deadline

expected current guard bindings
  exact typed guard keys + expected accepted publication revisions
  authenticated source authority / purpose / scope and source revision
  expected current lease generation + proposed acquired generation
  // expected values only; never substitutes for transactionally current guard reads
```

The authorization deadline is derived only from already accepted FND-04A time/freshness semantics. This decision introduces **no new numeric freshness value**. At the final guarded authorization decision `L` defined in Section 6, trusted current time, credential validity and accepted source-age bounds must be provable or the transaction rejects without committing candidate authority. The deadline applies to `L`, not to subsequent WAL flush, fsync or COMMIT acknowledgement; those cannot refresh an expired authorization or create another decision.

The Foundation verifier/consumer must expose a verified durability-ready fresh result sufficient to construct this authorization without reparsing unauthenticated token material or reconstructing current evidence from a durable record. The existing `FreshAdmissionFacts` may remain the narrower compatibility value; production V1 durable authorization must not discard AccountId or the current final evidence fences required above.

### Typed durable results

The semantic result family must distinguish at least:

```text
FreshAdmissionPrepare/submit result: local bounded submission accepted or unavailable

FreshAdmissionDurableOutcomeV1
  COMMITTED
  EXISTING_COMMITTED            // exact same replay key and immutable committed binding
  REJECTED_REPLAY_CONFLICT      // same replay key, different immutable binding
  REJECTED_INCUMBENT            // account or character already has incompatible nonterminal authority
  REJECTED_STALE_AUTHORITY      // accepted deadline/fence invalid at guarded decision L
  AMBIGUOUS_OR_UNAVAILABLE      // caller must reconcile, never assume abort

FreshAdmissionDurableReconciliationSnapshotV1
  immutable FreshAdmissionCommit receipt
  + current GameSessionAuthoritySnapshot
  from one transaction/fenced linearization point
```

Names may be adjusted to existing Rust conventions, but the distinctions above are semantic requirements.

## 6. Durable linearization and retry rules

Fresh admission has one **logical game-domain linearization point `L`: the final guarded authorization decision inside one PostgreSQL transaction, conditional on that transaction successfully committing**. Durable COMMIT makes the entire decision/effect set visible and recoverable; it is not a second authorization decision and does not sample a new credential-validity instant.

This explicitly maps FND-04A Sections 7.1/7.2's final revalidation and all-or-nothing authority creation to a transaction primitive that FND-04A leaves to Durability. Every required current fact and time bound is evaluated at `L`; effects remain uncommitted and confer no externally usable authority until durable COMMIT. A rolled-back transaction contributes no successful linearization or candidate authority. No success, physical controller or gameplay action may be published from the tentative decision.

For a new replay key, the transaction must atomically:

1. prove structural/version validity and acquire all Section 6.1 serialization protections, including account/character incumbent and lease claims, runtime/trust guards, replay key, candidate session identity and the global transport reference; finish any potentially blocking acquisition before deciding admission;
2. at `L`, sample trusted current database time and atomically evaluate the complete conditional admission predicate against the locked current state: each expected fence, AccountId -> CharacterId before world eligibility, every independent revision, source provenance/anti-rollback and unchanged credential/source-age bounds including all elapsed time before `L`;
3. the same predicate proves no conflicting replay receipt or incumbent and that the candidate session identity and transport reference are available for this exact binding; if any predicate is invalid, reject without committing candidate authority;
4. atomically establish the AccountPresence claim and acquire/advance the CharacterLease under their current guard CAS, retaining the exact candidate session binding; an authorization is not a pre-acquired lease;
5. reserve the exact transport reference in the global fresh/reconnect uniqueness set under Section 9.6, insert the immutable receipt and create the current `GameSession` as `ACTIVE`, generation `1`, with the acquired CharacterLease, current scope generation and exact transport ref;
6. hold every serialization protection through durable COMMIT; all candidate claim, lease, reservation, receipt and session effects commit together or none do. Elapsed persistence time after `L` neither re-dates `L` nor constitutes a new authorization. A transaction retry must reacquire/revalidate and decide at a new `L`; an ambiguous outcome must reconcile the original binding.

No success is returned before the transaction commits.

### 6.1 Current authority publication and atomic guard protocol

The immutable authorization is an **expected binding**, never current authority by itself. The selected physical serialization point is PostgreSQL: typed current guard rows, published by the owning Foundation authority and locked in the same transaction as fresh-admission effects. This is a durable projection/enforcement of Game authority, not a second credential verifier, independent world owner, or a new Platform authority.

The guard domains are bounded, typed, and independently versioned; no generic nullable evidence bag or composite replacement for the signed revisions is permitted:

| Guard key | Owning source and guarded state |
|---|---|
| AccountId + fixed security scope | Authenticated Platform-security observation accepted by `Fnd04EvidenceAuthority`; source revision/decision/provenance, minimum generation/state; Game AccountPresence claim names its current CharacterId/session or absence |
| CharacterId | Current Game account ownership, world/lifecycle eligibility, CharacterLease current generation and holder; transfer/handoff/terminal supersession |
| RuntimeScopeRefV1 | Current externally granted FND-03 owner/generation and target lifecycle/readiness; independent route/runtime and protocol/transport/rules/content/map/world-policy/offer bindings |
| Fixed verifier trust scope + key/profile identity | Authenticated signing-key/profile trust/revocation observation accepted by `Fnd04EvidenceAuthority`, source revision/decision/provenance and validity |

Foundation introduces typed `AdmissionAuthorityPublicationV1` and `AdmissionAuthorityPublicationReceiptV1` families alongside the fresh semantic port. Only the owning authority adapter can construct a publication after independently authenticating/resolving its source; token claims, a caller-filled `FreshCurrentEvidence`, old fresh receipts and the admission authorization cannot seed current guard truth. A RuntimeScope publication cannot self-grant ownership from NodeId or a locally incremented generation.

Each publication binds its fixed domain/key, authenticated source authority/purpose/scope, source observation provenance and comparable source revision/decision identity, expected preceding accepted guard revision, and new typed state. PostgreSQL performs compare-and-set under the same guard locks used by fresh COMMIT. Lower source/publication revisions and same-revision different decisions reject; exact replay is idempotent and cannot refresh source time. Initial publication requires independent authoritative bootstrap evidence and single-winner insert; absence is not permission to infer a state from the admission candidate. Conflicting initialization rejects. Tombstones/denials and monotonic high-water marks survive restart and are not deleted to permit reinitialization.

For Game-owned eligibility/lease/runtime changes, publication COMMIT is the admission-relevant activation boundary: the owner submits asynchronously and yields, then may expose the new authoritative result/readiness only after receiving/reconciling the publication receipt. All producers that transfer ownership/world, replace a runtime, change admission-relevant revisions or acquire/release/fence a lease/presence must use this protocol. They must never change current authoritative state first and asynchronously mirror the change later. Multi-domain changes update all affected guards atomically. Other domain payload preparation may precede activation without becoming current authority.

For Platform-security/trust, this protocol governs Game's durable acceptance of an authenticated source observation, not the time of a remote Platform action. FND-04A's existing bounded source-age semantics remain unchanged. A newer accepted deny/revision cannot be hidden by a still-unexpired older authorization. Pending source acceptance is not represented as already accepted; an unavailable or uncertain source makes fresh authorization unavailable, and old persisted provenance is never re-aged on restart.

The transaction obtains all relevant protections in one canonical domain/key order before `L`: exclusive locks for account/character claims, locks conflicting with publisher updates for runtime/trust guards, and database-visible serialization for replay keys, candidate session IDs and transport references, including keys whose rows do not yet exist. Existing uniqueness constraints remain defense in depth. An absence read alone does not lock an absent key.

All competing fresh/reconnect/publication paths must follow the same serialization protocol. Durability may use transaction-scoped database key locks or equivalent lock rows for absent-key serialization; a lock-key collision may serialize unrelated work but must never grant authority or alias semantic identities. Checks still compare the full typed keys. Candidate uniqueness/foreign-key/claim prerequisites and any other constraint that could introduce semantic contention must be resolved under these protections before `L`. Implementation qualification must inventory the actual SQL statements and constraints rather than assume that row locks remove every wait. A newly required conflicting acquisition or transaction retry after the tentative decision invalidates that attempt: abort it and make a fresh decision in a new transaction, never continue candidate effects using its old time sample. Ordinary WAL/storage/backend delay after the fully guarded decision is persistence delay, not another authority acquisition. Publisher CAS and admission therefore have one order: a publisher completed before the admission acquires its guards is included at `L`; a publisher contending after those guards are held cannot activate its replacement until the admission transaction ends. Successful admission was current at `L`; subsequent replacement follows normal fencing/lifecycle rules. No database/network operation holds the FND-03 logical writer.

A provider cannot claim this contract by doing an unlocked second read immediately before INSERT, reconstructing matching guards from the request, or updating a cache after mutation. Locks preserve the decision's authority order through COMMIT, but do not stop wall-clock time. Time is sampled at `L` after acquisitions, never at BEGIN or before a lock wait.

The implementation must distinguish expiry before `L` (reject with no committed candidate effects) from a backend/WAL/fsync stall after a valid `L` (the already-decided transaction may commit later). PostgreSQL timeouts, deferred triggers and a final time read are not proof of a physical COMMIT deadline. This contract requires no such guarantee. Persist the immutable `authorization_decided_at` and the accepted source/deadline evidence needed to audit `L`; a persistence/acknowledgement timestamp is separate and cannot replace or re-age source provenance. Historical decision evidence is not current controller authority: Section 7 revalidation still governs adoption after the delay.

All account/character claim lifecycle operations, including the existing reconnect adapter when it changes a guarded holder/generation or performs terminal replacement, must preserve the same locking/CAS discipline. This adds serialization to the physical adapter without changing accepted reconnect V1/V2 decisions. A guard is not an independent current-session row; its holder must reference the canonical session, and fresh acquisition/release and canonical session changes are atomic.

Startup/admission readiness requires a registered owning publisher and restored non-rollback current guards for every required domain. Missing producer binding, absent or uncertain current state, unsupported source bootstrap, or an authority mutation path that bypasses publication keeps fresh admission unavailable. The architecture does not assert that those production sources already exist: Child C must supply and verify their Game-side bindings before Server Seam release. Unsupported future transfers/revisions remain fail-closed until their owning producer integrates this same protocol.

### Same-key retry

If the replay key already has an **exactly matching immutable committed identity/binding**, retry returns `EXISTING_COMMITTED` plus its original immutable receipt and a freshly read current-session authority snapshot. Replay comparison uses the original replay key/candidate binding, not a newly sampled decision time; the persisted `L` and audit evidence describe that prior decision and are never replaced by a retry. It never inserts a second session and never rewinds current lifecycle state.

If the same replay key is presented with any different immutable identity/binding, the result is `REJECTED_REPLAY_CONFLICT`. Arrival order cannot choose a new winner.

### Lost response / ambiguous outcome

`AMBIGUOUS_OR_UNAVAILABLE` is not an abort. The only legal next persistence action for that logical admission is reconciliation by the same replay key/candidate binding. A retry must not mint another canonical session or another transport ref while the original outcome is unresolved.

After process restart, the replay key resolves the immutable commit receipt and current session authority from PostgreSQL. The receipt can prove historical commit identity; it cannot by itself prove current controller/liveness/runtime authority.

## 7. Independently-current post-commit authority

A durable commit is the source of truth for GrantNonce consumption and canonical `GameSession` creation. It does not force a stale runtime worker to acquire current controller authority.

When the completion/reconciliation snapshot returns as a new normalized Foundation input, Foundation must independently resolve the current facts applicable to transport adoption, including at least:

- exact GameSession identity/lifecycle;
- current AccountId -> CharacterId and CharacterId -> WorldId eligibility where the owning current authority requires it;
- current CharacterLease generation;
- current RuntimeScope ownership generation and target readiness;
- current connection generation/controller binding;
- no newer transfer/handoff/fence/terminal authority;
- current security/trust facts where FND-04A still requires them for publishing this admission success.

Persisted final-authorization evidence defines the committed expected binding; it is **not** a substitute for independently current facts at this consumer boundary.

If current facts still match, Foundation installs the current in-process projection and only then may Server Seam publish `ServerAccepted` and bind the physical transport.

If they do not match, the caller must fail closed and reconcile through the canonical current session/lifecycle authority. It must not publish admission success, reactivate a reconnectable/terminal session, reuse the original grant to create a second session, or reconstruct supposedly current authority from the receipt.

## 8. Synchronous compatibility disposition

`ReconnectAttemptJournal<T>` remains a synchronous compatibility/test abstraction for existing in-memory Foundation tests and already proven local semantics. It is **not a production SQLx port**.

Binding rules:

- production SQLx Durability MUST NOT implement `commit_fresh` by blocking, `block_on`, spawning-and-waiting, or hiding database/network wait behind the synchronous trait;
- production Server Seam MUST use the new split-phase fresh-admission durability flow for initial admission;
- the synchronous `commit_fresh` surface may remain while tests/legacy in-process adapters need it, but documentation/API shape must mark it non-production compatibility so a later consumer cannot silently reintroduce a blocking persistence path;
- existing asynchronous reconnect V1/V2 records, authorization and PREPARE/COMMIT/reconcile semantics remain canonical and are not redesigned here;
- any production lifecycle operation exercised after fresh commit that requires PostgreSQL I/O must follow the same FND-03 submit/yield/normalized-completion rule rather than calling a synchronous journal method against SQLx.

That last rule prevents this decision from solving only the first call while leaving an immediate synchronous database wait on `load_session`, control-loss, terminate or scope-advance paths. It does not change those lifecycle semantics; it only fixes the execution boundary when they are backed by PostgreSQL.

## 9. Forward migration / schema disposition

Released migration `apps/game-server/migrations/0001_admission_reconnect_journal.sql` is immutable. The implementation uses a new forward migration:

```text
apps/game-server/migrations/0002_fresh_admission_authority.sql
```

The migration must make the smallest compatible schema evolution:

### 9.1 Immutable receipt

Add a dedicated immutable `game_durability_fresh_admission_receipts` table keyed by the existing tagged `FreshAdmissionReplayKey` durable encoding. It stores the exact committed initial binding needed for idempotent replay and lost-response classification:

- replay key;
- GameSessionId;
- AccountId;
- CharacterId;
- WorldId;
- ChannelId;
- CharacterLease generation;
- RuntimeScope ownership generation;
- initial connection generation, constrained to `1`;
- `AuthenticatedTransportRefV1` exact 16 bytes;
- semantic version;
- immutable `authorization_decided_at` for `L` plus the exact accepted source-time/deadline evidence for auditing that decision; no replay attempt overwrites or re-dates it;
- separate non-authoritative persistence/acknowledgement timestamp, if recorded, for operations/evidence.

The table is immutable after commit except for a future separately accepted retention/archival mechanism. Current lifecycle changes belong to the current-session row, not the immutable receipt.

### 9.2 Canonical current-session row

Reuse the existing `game_durability_reconnect_sessions` physical table as the current durable GameSession authority row instead of inventing a second current-session owner. Its historical name does not grant reconnect-only semantics.

Migration `0002` may relax only the reconnect-specific fields that cannot truthfully exist for a newly admitted `ACTIVE` session (`control_loss_epoch`, `original_grace_deadline`, `predecessor_generation`) so they can be absent until a real control-loss/reconnect transition supplies them. It must add fail-closed integrity checks so a state that semantically requires control-loss continuity cannot omit the required continuity fields.

No zero, dummy or fabricated reconnect/control-loss value is permitted for a fresh admission.

### 9.3 Account-global exclusion

The current schema already enforces one nonterminal session per CharacterId. `0002` must also enforce the accepted FND-04A account-global playable exclusion with a database-visible single-winner uniqueness rule for nonterminal AccountId state. Migration fails closed if existing data violates the invariant; it does not silently pick a winner.

### 9.4 Current authority guards

Forward `0002` adds typed guard storage for Section 6.1 and the required source/publication high-water marks, CAS versions, claim holder references and integrity constraints. Foundation owns meaning and authenticated publication; Durability only enforces typed comparisons and transactional effects. Existing session/lease state must be reconciled from authoritative owning sources, never invented from an old session row or a fresh grant. Migration does not bootstrap source truth; until independently authorized bootstrap/readback succeeds, affected admission remains closed.

Guard claim holders refer to the existing canonical current-session row. Their acquisition/advance/release must be in the same transaction as corresponding canonical session effects; no independently writable parallel session/presence owner is introduced.

### 9.5 Existing reconnect compatibility

All existing reconnect rows and V1/V2 durable reconciliation semantics remain valid after migration. Every reconnect read/write path affected by nullable initial-origin continuity fields must be updated so it never interprets absence as zero/default history and requires real continuity before entering reconnectable semantics.

### 9.6 One transport-reference uniqueness namespace

The existing `game_durability_transport_ref_reservations` primary key remains the one global non-reuse set for `AuthenticatedTransportRefV1`. Forward `0002` adds a closed reservation-owner discriminator and a typed fresh replay-key binding, while allowing `reconnect_attempt_ref` to be absent only for a fresh owner. CHECK constraints require exactly one truthful owner: fresh replay key or existing reconnect attempt binding, always with its exact GameSessionId. Existing rows retain their reconnect owner/binding; no dummy reconnect attempt is created.

Fresh COMMIT atomically reserves its reference with receipt/session creation. Reconnect PREPARE continues reserving in the same table and must reject a reference already owned by any other fresh/reconnect binding. An exact committed retry may reuse only its original reservation. No pruning or terminal transition releases a reference for another physical transport. A precommit collision returns a typed conflict without consuming the grant; Foundation may generate a new candidate only after proving the earlier transaction did not commit. Ambiguous outcomes reconcile the original candidate first.

Qualification must include fresh/fresh, fresh/reconnect and reconnect/fresh collisions, plus same-binding replay and migration/reload of pre-existing reconnect reservations.

## 10. Exact implementation ownership and sequencing

This architecture decision grants **no implementation lease**. After this decision is reviewed and protected-main integrated, Work must create fresh allocations in this order.

### Child A — Foundation fresh-admission durability semantics

Serialized owner: Foundation.

Exact allowed surfaces:

```text
apps/game-server/src/foundation/fresh_admission_durability.rs        # new
apps/game-server/src/foundation/admission_authority_publication.rs  # new
apps/game-server/src/foundation/admission.rs
apps/game-server/src/foundation/admission_facade.rs
apps/game-server/src/foundation/fnd04_verifier.rs
apps/game-server/src/foundation/mod.rs
apps/game-server/src/foundation/fresh_admission_durability_tests.rs  # new, if split tests are used
```

Responsibilities:

- define the V1 authorization/result/reconciliation semantic types and split-phase state machine;
- define typed owning-source publication/CAS/bootstrap ports, producer capabilities and normalized publication completion; prevent grant/receipt-derived current guard construction;
- expose durability-ready verified fresh evidence without weakening verifier classification;
- preserve synchronous compatibility as non-production;
- no SQLx, schema, migration, Cargo, listener or reconnect redesign.

### Child B — Durability fresh-admission/session adapter

Starts only from protected main containing Child A.

Serialized owner: Durability.

Exact allowed surfaces:

```text
apps/game-server/src/durability/fresh_admission.rs                   # new
apps/game-server/src/durability/admission_authority_guards.rs        # new
apps/game-server/src/durability/admission_journal.rs
apps/game-server/src/durability/db.rs
apps/game-server/src/durability/mod.rs
apps/game-server/src/durability/schema.rs
apps/game-server/migrations/0002_fresh_admission_authority.sql       # new
apps/game-server/tests/durability_postgres.rs
apps/game-server/tests/support/postgres.rs
```

Responsibilities:

- implement asynchronous COMMIT/reconcile semantics and publication guard CAS with one atomic presence/lease/session boundary;
- preserve source high-water marks and global fresh/reconnect transport reservations, including reconnect/session mutations that touch the guarded claims;
- apply only the forward schema change in Section 9;
- preserve reconnect V1/V2 semantics;
- real PostgreSQL qualification for fresh admission, replay, concurrency, restart and existing reconnect compatibility.

No Cargo/lockfile dependency is expected: current Durability already has the SQLx stack. If live implementation evidence proves a Cargo or other path is genuinely necessary, Work must stop and explicitly amend the allocation rather than letting the child seize it.

### Child C — owning producer and composition integration

Starts only from protected main containing A and B. Serialized owner: Foundation integration, with a coordinator-granted shared composition lease. Exact surfaces:

```text
apps/game-server/src/admission_authority.rs                         # new
apps/game-server/src/lib.rs                                        # serialized composition export only
apps/game-server/src/foundation/admission_authority_publication.rs
apps/game-server/src/foundation/fnd04_verifier.rs
apps/game-server/src/foundation/admission_facade.rs
apps/game-server/tests/admission_authority_postgres.rs              # new
apps/game-server/tests/support/postgres.rs
```

The composition module binds the authenticated `Fnd04EvidenceAuthority` security/trust providers and owning Game character/presence/lease/runtime/revision providers to B's publication adapter. It exposes the bounded registration/readiness and publication-completion interface that the Server Seam can consume; it does not open a listener or authorize deployment.

Mandatory delivery: an executable producer inventory mapping every Section 6.1 guard domain to its registered source, initialization, updates, denial/fencing, restart and public mutation entry points. Production constructors must require these owning source capabilities; an arbitrary fixture, caller-created fact struct or receipt cannot satisfy registration. Tests exercise the actual registered adapter and SQL path with independently controlled source mutations. A fixture proves only that test source's behavior; it does not establish production source availability.

C must prove commit-before-publish ordering, source anti-rollback, atomic multi-domain publication, stale publisher CAS rejection and admission races against each producer. The synchronous evidence lookup used during verification may read an already-published validated projection, but cannot block on SQLx or treat an unacknowledged publication as current. The coordinator must record any genuinely unavailable owning source as a precise implementation prerequisite and keep readiness closed; no production provider or source connectivity is claimed by this architecture artifact. Integration of A+B alone is insufficient.

These are prospective paths, not a lease. Any concrete existing mutation caller outside the enumerated surfaces requires a fresh exact coordinator allocation before editing; its bypass must be closed before C can qualify. Adding a new Game product authority or external-repository protocol is outside this decision.

### Child D — existing Server Seam resume

There is no architecture-authorized Server Seam mutation until A, B and C are merged/read back and C's registered owning-source readiness is proven. Work then re-reads protected main, active leases and the preserved `agent/otv2-gameplay-server-seam-01` branch. Only Work may decide whether the existing allocation can be lawfully resumed/reconciled or needs a fresh narrow amendment.

The preserved Server Seam worker does not receive Foundation-admission or Durability migration paths from this decision.

## 11. Required authority/recovery qualification

Use the repository model:

```text
AuthorityInvariant × ConsumerBoundary × MutationOperator
```

### Authority invariants

At minimum:

- GrantNonce/replay-key identity and one-time consumption;
- AccountId -> CharacterId binding before CharacterId -> WorldId;
- account-global incumbent exclusion;
- CharacterLease generation;
- RuntimeScope ownership generation;
- protocol/transport and independent revision bindings;
- security/trust source freshness and anti-rollback fence;
- exact candidate GameSessionId and transport ref;
- connection generation exactly `1` for first admission;
- current GameSession lifecycle/controller state.

### Consumer boundaries

At minimum:

1. Foundation final authorization construction;
2. Durability's final guarded decision `L` and conditional transaction effects;
3. Durability retry/lost-response reconciliation;
4. Foundation post-commit current-authority adoption;
5. first real control-loss/reconnect transition consuming a fresh-origin session row.

### Mutation operators

Each applicable negative case changes exactly one authority invariant while keeping unrelated facts valid. Cover at least:

- missing evidence;
- stale lease/scope/security/trust/revision evidence;
- mismatched account/character/world/channel/binding;
- expired/future/non-monotonic time evidence;
- replay of same key with exact binding;
- replay of same key with changed binding;
- concurrent distinct grants for the same AccountId and different CharacterIds;
- concurrent grants for the same CharacterId;
- candidate GameSessionId or transport-ref collision;
- response lost after DB commit;
- process restart before completion delivery;
- runtime ownership replacement while DB commit is in flight, in both publisher-before-admission and admission-before-publisher order;
- character-world transfer, lease/presence replacement, independent revision change or accepted security/trust deny between initial authorization and `L`, and a contending publication held until admission transaction completion;
- absent guard/source, forged bootstrap, stale publisher CAS, equal-revision contradiction and restart high-water-mark rollback;
- expiry during any acquisition before `L` rejects; a WAL/backend delay after a valid `L` may finish durable COMMIT later but cannot bypass current post-commit adoption;
- every cross-origin fresh/reconnect transport reservation collision;
- replay after session became reconnectable or terminal;
- PostgreSQL reload/reconnect and migration compatibility paths.

Persisted receipt-derived matching helpers may support an idempotent positive replay classification. They are forbidden as the source of supposedly current lease, scope, controller, security or runtime facts in negative/adoption cases.

## 12. Mandatory implementation evidence

### Foundation child

- RED -> GREEN for split-phase semantics before implementation completion;
- full final-revalidation evidence set and authorization deadline derivation from already accepted FND-04A rules;
- same-key exact replay vs changed-binding conflict;
- stale current facts after a committed completion cannot install a controller;
- synchronous compatibility cannot be accidentally selected as the production SQLx path;
- focused + component tests and independent exact-head authority/security review.

### Durability child

Real isolated PostgreSQL evidence must include:

- new fresh admission creates exactly one receipt and one ACTIVE session row atomically;
- same-key retry returns the original commit/current state;
- conflicting same-key retry fails without mutation;
- same-account and same-character concurrent candidates produce one durable winner;
- lost response after COMMIT reconciles without a second session;
- fresh row contains no fabricated control-loss/predecessor history;
- real control loss populates required continuity before reconnect semantics consume it;
- reconnect V1/V2 regression suite remains green across migration `0002`;
- stale deadline/fence at `L` fails closed without committed nonce, presence, lease, receipt or session effects, including source publication completed after initial authorization but before `L`;
- atomic guard bootstrap/publication, publisher-before-admission and admission-before-publisher races, lease/presence acquisition and global transport reservation collisions;
- expiry during each pre-`L` SQL/uniqueness/claim acquisition rejects; no BEGIN-time sampling;
- a backend/WAL stall after valid `L` may commit later with the original decision timestamp; post-commit adoption uses independently current facts and never reconstructs freshness from `L`;
- unexpected additional semantic contention or transaction retry abandons the tentative decision and revalidates in a fresh transaction; lost COMMIT response reconciles the original binding;
- restart/reload reconstructs exact receipt + current session snapshot;
- runtime credentials still cannot perform migration DDL;
- migration fresh/ahead/behind/checksum/locking rules remain intact.

High-risk exact-head independent review is mandatory before integration. Repository checks, protection and Merge Queue remain final merge authority.

## 13. Risks and trade-offs

### Benefits

- preserves one canonical durable session owner;
- no database wait inside FND-03 logical writer;
- no fake reconnect history;
- one-time grant replay and lost-response semantics are durable and deterministic;
- existing reconnect V1/V2 design is reused rather than replaced;
- no new dependency, service, resource value or public protocol ID.

### Costs

- requires a forward schema migration and a narrow Foundation semantic child before Durability implementation;
- current reconnect/session physical table name becomes historically narrow relative to its role;
- existing reconnect queries must be audited for assumptions that continuity fields are always present in an ACTIVE row.

### Primary failure risk

The highest-risk mistake is treating immutable authorization/receipt as current authority at durable COMMIT or completion/restart. Section 6.1 therefore requires independently published current guards and serialization with all authority-changing producers before any nonce/session effect. The implementation must therefore test independently-current post-commit adoption as a first-class boundary, not only idempotent database writes.

## 14. Explicit non-decisions

This decision does not select:

- production DB host/topology/credentials;
- receipt pruning horizon;
- deployment or listener configuration;
- AccountPresence product policy outside the atomic guarded claim/exclusion required here;
- gameplay command/state/event IDs;
- character transfer/takeover product policy;
- new reconnect grace/protection/resource numbers;
- QUIC or alternate transport;
- any Platform/external-repository change.

## 15. Handoff

Until this candidate receives required independent review, ordinary repository checks, protected Merge Queue integration and protected-main readback:

```text
ARCHITECTURE_RESULT = CANDIDATE_READY_FOR_CONTROL_PLANE_QUALIFICATION
ISSUE_313 = REMAINS_OPEN
SERVER_SEAM_247 = WAITING_ARCHITECTURE
SERVER_SEAM_WORKER = PRESERVE 9370b254c6ac4f6529e069c1968ae6bfa1e1750e
IMPLEMENTATION_AUTHORITY = NONE
```

After lawful protected-main integration, Work should mark the architecture decision accepted, close/reconcile #313 as appropriate, allocate Child A, then Child B, then Child C, and only after their protected-main integration and producer-readiness proof re-evaluate the preserved Server Seam.
