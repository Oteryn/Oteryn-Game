# Oteryn Game — Terminal GameSession Replacement and Collision Reconciliation Decision

- Date: 2026-08-28
- Issue: #248
- Parent control plane: #162
- Blocked implementation lane: #167 / PR #243
- Downstream blocked lane: #247
- Decision ID: `DUR-TERMINAL-SESSION-REPLACEMENT-V1`
- Exact decision base: `main@0fa962b4e4f688331fea899ae496dbfdb914583d`
- Accepted predecessors: `DUR-RECONNECT-AUTHORITY-V1` (#187 / PR #190) and `DUR-RECONNECT-TRANSPORT-REF-UNIQUENESS-V1` (#197 / PR #200)
- Status: **ACCEPTED WHEN THIS DECISION MERGES**
- Candidate qualification: **REVIEW_RECONCILIATION_REPAIRED; FRESH EXACT-HEAD REVIEW REQUIRED**
- Runtime / schema / migration / Cargo / workflow / merge / production authority: **NONE**

## Architecture resolution packet

```yaml
classification: ARCHITECTURE_RESOLUTION
repository: Oteryn/Oteryn-Game
main_sha: 0fa962b4e4f688331fea899ae496dbfdb914583d
source_escalation: "Issue #248; dispatch comment #issuecomment-5455270337"
blocking_question: >-
  Define the Foundation evidence, exact predecessor-to-candidate fencing/CAS boundary,
  typed collision reconciliation, and complete bounded clean-successor path set needed to
  replace an authoritatively terminal GameSession without weakening one-live-session-per-character.
facts:
  proven:
    - "Protected main at decision start and review-repair start is 0fa962b4e4f688331fea899ae496dbfdb914583d."
    - "Issue #248 is the live ARCHITECTURE_ESCALATION_REQUIRED packet dispatched to the Sol Supervising Architect."
    - "PR #243 is the blocked Durability candidate at eb28c42125c346e7f6f1c72e69d51af35af8fc1f and is outside this role's write authority."
    - "The current Durability schema enforces actor-wide UNIQUE(character_id) while its session_state has no terminal replacement state."
    - "ReconnectPrepareDispositionV1 already distinguishes RejectedTransportRefCollision."
    - "ReconnectDurableReconciliationSnapshotV1 collapses collision, concurrent-prepared and stale terminal states into generic Terminal."
    - "ReconnectPrepareDispositionV1::ExistingTerminal is also generic, so direct same-PREPARE replay can lose the original terminal reason before reconciliation."
    - "Foundation validate_current_authority requires the current CharacterLease generation to equal the admission commit, but permits current ScopeOwnershipGeneration to advance monotonically above the committed generation."
    - "Protected main already provides game-server sqlx/serde_json/tokio dependencies, but has no apps/game-server/src/durability module composed by apps/game-server/src/lib.rs."
    - "The clean Durability successor PR #243 exercises build.rs, migration binary, durability mod/db/journal/schema and PostgreSQL support in addition to the originally listed repair paths."
    - "Fresh independent Codex review of PR #249 head 51620b49fc230bf858bf0a69c8ca33894955b992 returned P1 3882676421 and P2 findings 3882676411 / 3882676426; Work returned bounded architecture repair ownership to this role."
    - "DUR-RECONNECT-AUTHORITY-V1 assigns lifecycle/final authorization to Foundation and PostgreSQL CAS/classification to Durability."
    - "DUR-RECONNECT-TRANSPORT-REF-UNIQUENESS-V1 makes collision terminal for one attempt, forbids same-attempt remint and retains the 8-attempt ControlLossEpoch bound."
  derived:
    - "Durability cannot safely infer predecessor terminality from persisted deadline/age without taking Foundation lifecycle authority."
    - "A persisted predecessor scope generation can legitimately lag the current terminal Foundation scope generation; exact equality would strand a valid terminal session after an allowed scope-generation advance."
    - "The safe no-extra-round-trip repair is a Foundation-authorized monotonic forward synchronization of that persisted scope fence inside the same locked PREPARE replacement transaction."
    - "A generic ExistingTerminal completion cannot be allowed to terminal-complete the Foundation flow; a direct V2 replay must carry the typed durable terminal reason, while legacy generic V1 replay must force typed reconciliation."
    - "A fresh clean-successor allocation must include the complete exercised Durability module/support set plus a serialized lib.rs composition lease rather than knowingly requiring an immediate scope expansion."
  unknown:
    - "Exact PostgreSQL physical representation of the actor anchor/replacement receipt is intentionally deferred to the allocated Durability implementation."
    - "Exact Rust enum/field spelling for the versioned successor types is intentionally deferred provided the frozen semantics remain exact."
  conflict: []
accepted_decision: DUR-TERMINAL-SESSION-REPLACEMENT-V1
rejected_options:
  - "Remove actor-wide CharacterId exclusion and rely on GameSessionId."
  - "Let Durability infer terminal predecessor eligibility from deadline or row age."
  - "Require persisted predecessor scope generation to equal current Foundation terminal scope generation after legal scope advancement."
  - "Treat generic ExistingTerminal as a sufficient terminal completion without recovering the original durable terminal disposition."
  - "Add a separate mandatory synchronous/extra replacement service before PREPARE."
affected_contracts:
  - "DUR-RECONNECT-AUTHORITY-V1 — preserved"
  - "DUR-RECONNECT-TRANSPORT-REF-UNIQUENESS-V1 — preserved"
  - "TerminalGameSessionReplacementAuthorizationV1 — new versioned Foundation semantic authorization with current terminal scope generation"
  - "ReconnectDurableReconciliationSnapshotV2 / typed terminal disposition — new versioned reconciliation successor"
  - "V2 direct ExistingTerminal PREPARE replay — original typed terminal disposition is mandatory"
affected_paths:
  - apps/game-server/src/foundation/admission_recovery_inner.rs
  - apps/game-server/build.rs
  - apps/game-server/migrations/0001_admission_reconnect_journal.sql
  - apps/game-server/src/bin/oteryn-game-migrate.rs
  - apps/game-server/src/durability/admission_journal.rs
  - apps/game-server/src/durability/db.rs
  - apps/game-server/src/durability/mod.rs
  - apps/game-server/src/durability/schema.rs
  - apps/game-server/tests/durability_postgres.rs
  - apps/game-server/tests/support/postgres.rs
  - apps/game-server/src/lib.rs
implementation_owner: >-
  Fresh control-plane-allocated serialized Foundation/Durability repair lane with a serialized
  composition lease for apps/game-server/src/lib.rs; this architecture decision itself grants
  no implementation write authority.
implementation_scope: >-
  Only the eleven implementation/composition paths above. The coordinator-owned task/allocation
  record is governance metadata and is not part of this semantic implementation allowlist.
  Any further implementation path requires fresh RED evidence and explicit control-plane scope expansion.
resource_values_changed: false
production_authority_changed: false
cross_repository_authority_changed: false
supersedes: []
required_validation:
  - "Exact-head repository governance and merge gates."
  - "Foundation RED/GREEN authority-contract proof listed in Section 9."
  - "Real isolated PostgreSQL race/restart/idempotency proof listed in Section 9."
  - "Workspace/module-composition and migration-binary proof for the complete clean successor listed in Section 9."
  - "Exact-head self-review."
  - "Zero unresolved required review threads."
required_independent_review: >-
  CODEX_REQUIRED exact-head independent architecture/security/authority review under
  OTV2-CODEX-INDEPENDENT-REVIEW-01 because the decision changes SESSION/RECONNECT/FENCING
  semantics; the authoring architect is not the independent reviewer or review-request owner.
next_action: >-
  Uniquely active Work control plane verifies the repaired exact candidate, routes PR #249 exact
  head through the authorized independent-review owner, and integrates only after all gates pass.
```

## 1. Decision timing

**Must decide now: YES.**

The current Durability candidate correctly enforces actor-wide exclusion with a database `UNIQUE (character_id)` and correctly persists `RejectedTransportRefCollision` as an attempt state, but it cannot safely admit a later legitimate `GameSession` after the previous session is authoritatively terminal, and its V1 terminal surfaces can collapse collision into generic terminality.

Both defects cross the existing Foundation/Durability ownership boundary. Durability cannot infer Foundation lifecycle terminality from a deadline, and collision replay/reconciliation types are Foundation-owned. A lane-local repair would therefore either weaken one-live-session-per-character or expand Durability into Foundation lifecycle policy.

Independent review also proved two boundary details that must be explicit before implementation: a legal runtime-scope ownership-generation advance may leave the persisted Durability predecessor fence behind the current terminal Foundation generation, and a clean successor from protected `main` needs the complete Durability module/support/composition path set rather than only the four initially named Durability files.

This decision freezes only the missing terminal actor-anchor replacement, monotonic terminal scope synchronization, typed terminal replay/reconciliation boundary and complete clean-successor allocation. It does not reopen reconnect proof rules, attempt limits, transport-ref minting, Server Seam, Client, Movement, Combat, gameplay authority, production topology or database sizing.

## 2. Preserved decisions and bounded supersession

`DUR-RECONNECT-AUTHORITY-V1` remains binding:

- Foundation owns authentication, current lifecycle interpretation, controller/lease/runtime-scope policy and final authorization.
- Durability owns PostgreSQL persistence, exact fenced CAS/transaction work and durable classification.
- no SQLx/network wait occurs inside Foundation's logical writer;
- unknown or mismatched authority fails closed.

`DUR-RECONNECT-TRANSPORT-REF-UNIQUENESS-V1` remains binding without changing its resource behavior:

- one `ReconnectAttemptRef` owns exactly one `AuthenticatedTransportRefV1`;
- PREPARE is the durable transport-ref reservation point;
- transport-ref collision is terminal for that exact attempt;
- a replacement ref requires a new attempt and fresh Foundation checks;
- same-attempt remint is forbidden;
- `FND04-RECONNECT-ATTEMPTS-PER-LOSS-EPOCH = 8` remains the only collision-recovery attempt bound;
- `UNAVAILABLE` retries the same request and `AMBIGUOUS` reconciles the same attempt.

This decision adds two versioned semantic families only:

1. Foundation-issued authorization for replacing an exact terminal predecessor `GameSession` actor anchor with an exact candidate `GameSession`, including the current terminal scope generation and an authorized monotonic persistence synchronization rule;
2. typed durable terminal outcome semantics shared by direct terminal PREPARE replay and reconciliation, so a collision survives response loss/restart as collision rather than generic terminality.

## 3. Selected architecture

### 3.1 Foundation proves terminal replacement eligibility

Foundation is the sole owner of the fact that a predecessor `GameSession` is terminal and may no longer regain controller authority.

The implementation must expose one versioned semantic authorization, named here:

```text
TerminalGameSessionReplacementAuthorizationV1
  version = 1

  account_id
  character_id
  world_id

  predecessor_game_session_id
  predecessor_connection_generation
  predecessor_character_lease_generation
  predecessor_current_scope_ownership_generation

  candidate_game_session_id
```

The exact Rust field layout may differ, but none of these semantic bindings may be omitted or weakened. `predecessor_current_scope_ownership_generation` is explicitly the **current Foundation terminal authority generation**, not an assertion that the last persisted Durability actor row already contains that value.

Foundation may construct this authorization only after one normalized authority resolution proves all of the following from current Foundation-owned authority:

1. the predecessor `GameSessionAuthoritySnapshot` is the exact current snapshot for `predecessor_game_session_id`;
2. its state is `GameSessionState::Terminal`;
3. it has no current transport/controller;
4. its committed `CharacterId` and `WorldId` match the authorization;
5. its current connection generation, CharacterLease generation and current RuntimeScope ownership generation exactly match the corresponding authorization fields;
6. current Foundation identity/eligibility still maps the candidate account to the same `CharacterId` and the same `WorldId`;
7. `candidate_game_session_id != predecessor_game_session_id`;
8. the candidate `ReconnectDurabilityRecordV1` is for the exact account/character/world/candidate GameSession named by the authorization.

`GameSessionState::Terminal` is irreversible. A timeout, expired reconnect deadline, missing process-local transport, database row age or Durability-local inference is **not** replacement evidence.

The authorization is an internal typed authority object, not an external bearer credential. It authorizes only the exact predecessor-to-candidate replacement described above and the monotonic persisted predecessor scope-fence synchronization in Section 3.2; it does not authorize reconnect COMMIT, proof bypass, lease change, scope ownership change in Foundation, a backwards fence move or a different candidate.

### 3.2 Durability executes replacement and terminal scope-fence synchronization, but does not decide either authority fact

Durability owns the database-enforced actor anchor and therefore executes the replacement CAS. It does not decide whether terminality is true and does not independently advance Foundation scope authority.

The atomic replacement occurs inside the candidate's asynchronous **PREPARE transaction**, before:

- candidate attempt allocation is made authoritative;
- the candidate transport ref is reserved/published as PREPARED;
- candidate `attempt_count` is consumed;
- any candidate controller/current-generation mutation.

This introduces no synchronous database/network wait into the Foundation logical writer and no additional mandatory round trip for the replacement path.

For a candidate whose `CharacterId` already has a different durable actor anchor, PREPARE may proceed only when the exact `TerminalGameSessionReplacementAuthorizationV1` is present and valid.

The replacement transaction must lock the exact current actor anchor and atomically prove:

```text
current actor anchor.character_id == authorization.character_id
current actor anchor.game_session_id == authorization.predecessor_game_session_id
stored predecessor account/world == authorization account/world
stored predecessor current_generation == authorization.predecessor_connection_generation
stored predecessor character_lease_generation == authorization.predecessor_character_lease_generation
stored predecessor scope_ownership_generation <= authorization.predecessor_current_scope_ownership_generation
candidate record GameSessionId == authorization.candidate_game_session_id
candidate record AccountId/CharacterId/WorldId == authorization account/character/world
```

The scope comparison is deliberately monotonic rather than exact. Protected-main Foundation semantics permit `current_scope_generation` to advance above the committed generation, while CharacterLease validation remains exact. Therefore an older persisted scope fence for the same exact predecessor is not by itself stale terminal authority.

Inside the same locked transaction, after exact predecessor/candidate identity and terminal authorization have been accepted but before candidate PREPARED authority is created, Durability must perform this ordered operation:

```text
if stored predecessor scope_ownership_generation
   > authorization.predecessor_current_scope_ownership_generation:
    reject stale/mismatched authority with no mutation

if stored predecessor scope_ownership_generation
   < authorization.predecessor_current_scope_ownership_generation:
    advance the persisted predecessor scope fence exactly to the Foundation-authenticated
    current terminal generation; never infer or choose a generation locally

fence predecessor so it can never become ACTIVE/PREPARED/COMMITTED again
+ terminalize any predecessor PREPARED attempt as stale/superseded authority
+ clear any predecessor current/prepared controller projection in the durable session representation
+ persist an idempotent exact predecessor -> candidate replacement receipt/binding
+ establish candidate as the sole non-terminal actor anchor
+ continue the ordinary candidate PREPARE checks/reservation
```

This forward synchronization is authorized only for the exact terminal predecessor inside this replacement transaction. There is no standalone Durability scope-authority update, no backwards synchronization and no general permission for Durability to interpret runtime relocation policy.

The physical representation is a Durability implementation detail. A partial unique active-anchor index, a dedicated actor-anchor row, or another equivalent PostgreSQL representation is acceptable only if it preserves historical predecessor reconciliation and proves the invariants below. Application-side `SELECT` followed by an unfenced overwrite is forbidden.

If the durable actor anchor is neither the exact predecessor nor an idempotently proven prior replacement to the exact candidate, the request fails closed as stale/mismatched authority. It must not overwrite the anchor.

A repeated request after a lost replacement response is idempotent only when durable evidence proves the same `(CharacterId, predecessor_game_session_id, candidate_game_session_id)` replacement already committed. A different predecessor or candidate is a conflict, not a retry.

### 3.3 One-live-session invariant

At every commit boundary:

```text
for one CharacterId:
  number of authoritative non-terminal GameSessions <= 1
```

The replacement transaction must never expose a state in which predecessor and candidate are both authoritative/live. If the transaction aborts or its CAS predicate fails, the previous anchor remains unchanged and the candidate is not PREPARED.

A predecessor terminalization caused by this authorization also fences every outstanding predecessor reconnect attempt so a late COMMIT cannot reactivate it.

## 4. Typed durable terminal replay and reconciliation

The current Foundation V1 surfaces expose only:

```text
DurableReconnectStateV1::Terminal
ReconnectPrepareDispositionV1::ExistingTerminal
```

Both are insufficient when used generically because the accepted transport-ref decision requires a lost collision response to recover the collision-specific terminal disposition before Foundation may create a new attempt/ref.

The persistent implementation must therefore use one typed terminal-disposition domain across direct PREPARE replay and reconciliation:

```text
ReconnectDurableTerminalDispositionV1
  TRANSPORT_REF_COLLISION
  CONCURRENT_PREPARED
  STALE_AUTHORITY

ReconnectDurableOutcomeV2
  PREPARED
  COMMITTED {
    current_generation,
    current_transport_ref
  }
  TERMINAL {
    disposition: ReconnectDurableTerminalDispositionV1
  }

ReconnectDurableReconciliationSnapshotV2
  record: exact ReconnectDurabilityRecordV1
  outcome: ReconnectDurableOutcomeV2

ReconnectPrepareDispositionV2::ExistingTerminal {
  disposition: ReconnectDurableTerminalDispositionV1
}
```

Exact Rust enum/type spelling may differ; the semantic distinctions may not be collapsed.

Required mappings are exact:

```text
durable collision terminal -> TRANSPORT_REF_COLLISION
                             -> Foundation `RejectedTransportRefCollision`

durable concurrent-prepared terminal -> CONCURRENT_PREPARED
                                      -> Foundation `RejectedConcurrentPrepared`

durable stale/superseded terminal -> STALE_AUTHORITY
                                  -> Foundation `RejectedStaleAuthority`
```

For a direct retry of the **same exact PREPARE request** after the first terminal transaction committed but its response was lost, Durability must return the original typed terminal reason in the V2 `ExistingTerminal` completion. Foundation must apply that typed reason exactly as if it had received the original completion. In particular, `TRANSPORT_REF_COLLISION` must enter the attempt budget as collision-terminal so the already accepted bounded new-attempt rule can operate.

A legacy/generic `ReconnectPrepareDispositionV1::ExistingTerminal` is only a replay envelope. It must **not** drive `ReconnectDurabilityFlowV1::accept_prepare_completion` directly to a generic terminal completion on the migrated path. If that generic V1 envelope is encountered during compatibility migration, Foundation must transition to `ReconciliationRequired` / `ReconcileSameAttempt` and obtain the V2 typed snapshot before deciding the terminal reason.

The generic `ReconnectDurableReconciliationSnapshotV1::terminal(...)` must not be used by the PostgreSQL implementation as the recovery answer for a durable collision once the V2 boundary is allocated. V1 may remain as compatibility/test surface until callers are migrated, but V1 terminality must never be reinterpreted as proof that the terminal reason was a collision.

Foundation may permit `replacement_allowed_after_collision` only after a V2 direct `ExistingTerminal { TRANSPORT_REF_COLLISION }`, a V2 `TRANSPORT_REF_COLLISION` reconciliation, or the equivalent original direct collision completion, and only under the already accepted current-authority checks and remaining eight-attempt budget. `CONCURRENT_PREPARED` and `STALE_AUTHORITY` do not inherit collision-remint semantics.

## 5. Failure and concurrency rules

The following all fail closed with no actor-anchor replacement and no new PREPARED authority:

- missing terminal replacement authorization when another GameSession owns the actor anchor;
- predecessor GameSession mismatch;
- candidate GameSession mismatch;
- account/character/world mismatch;
- predecessor connection-generation mismatch;
- predecessor lease-generation mismatch;
- persisted predecessor scope generation greater than the Foundation-authenticated current terminal scope generation;
- any backwards or locally invented scope-generation synchronization;
- generic `ExistingTerminal` used as a terminal reason without typed replay/reconciliation;
- unknown/corrupt replacement receipt;
- inability to prove database uniqueness/serialization;
- integer/codec/version failure;
- any concurrent winner that changes the actor anchor before this transaction commits.

A persisted predecessor scope generation lower than the current terminal Foundation scope generation is **not** a failure by itself; it is synchronized forward only under Section 3.2 inside the exact replacement transaction.

A later request with a newer legitimate terminal predecessor must carry a fresh exact Foundation authorization. Durability may not synthesize it from persisted deadlines or previous replacement history.

## 6. Options considered

### Option A — remove actor-wide `UNIQUE (character_id)` and rely on `GameSessionId`

Rejected. It reopens the proven cross-session same-character double-authority defect.

### Option B — Durability infers predecessor terminality from deadline/state age

Rejected. It transfers Foundation lifecycle policy into Durability and can replace a still-authoritative session.

### Option C — separate synchronous/extra replacement service before PREPARE

Rejected for this slice. It introduces another durable request/completion/lost-response phase when the same exact CAS can be serialized at the existing asynchronous PREPARE boundary.

### Option D — Foundation-authorized replacement, Durability-executed PREPARE CAS with monotonic scope sync, typed direct replay and reconciliation V2

**Selected.** It preserves ownership, one-live-session exclusion, legal Foundation scope-generation advance, asynchronous logical-writer discipline and the existing collision attempt budget while adding only the missing cross-lane semantics.

## 7. Exact ownership after this decision

### Foundation owns

- determining that a predecessor `GameSession` is irreversibly terminal;
- proving current predecessor connection/lease/current-scope fences and candidate account/character/world eligibility;
- constructing `TerminalGameSessionReplacementAuthorizationV1` with the exact current terminal scope generation;
- constructing the candidate reconnect record/request;
- interpreting typed V2 direct terminal replay and reconciliation;
- forcing typed reconciliation when only a generic V1 `ExistingTerminal` envelope is available;
- deciding whether a collision permits a later new attempt under existing FND-04 rules.

### Durability owns

- durable actor-anchor representation;
- exact predecessor-to-candidate replacement receipt/idempotency evidence;
- the atomic replacement CAS inside PREPARE;
- monotonic forward synchronization of the exact predecessor persisted scope fence only under the Foundation terminal authorization and only inside the replacement transaction;
- database-enforced one-nonterminal-session-per-character exclusion;
- durable terminal reason storage;
- V2 direct `ExistingTerminal` and reconciliation mapping from stored terminal reason to the Foundation-owned typed disposition;
- migration/schema safety and PostgreSQL race behavior.

### Not owned here

Server Seam, Client, QA, Movement, Combat, gameplay, transport listener, production deployment, secrets, resource-limit registry values and merge authority remain unchanged and outside this decision.

## 8. Complete bounded subsequent cross-lane allocation

After this decision is merged and read back from protected `main`, the Work coordinator must create one fresh bounded Foundation/Durability cross-lane allocation with serialized shared/composition ownership. It must use a fresh exact base and initially include exactly the implementation/composition paths below so the clean successor can compile and exercise the PostgreSQL adapter without an already-known scope-expansion stop.

### Foundation contract

```text
apps/game-server/src/foundation/admission_recovery_inner.rs
```

The existing inline `durability_reconnect_v1_tests` / successor tests in that file are sufficient ownership for the public semantic contract unless fresh RED proves a separate existing Foundation test path is required. No `fnd04_verifier.rs`, protocol, gameplay, Server Seam or unrelated Foundation path is pre-authorized by this decision.

### Durability clean-successor adapter/schema/migration/tests

```text
apps/game-server/build.rs
apps/game-server/migrations/0001_admission_reconnect_journal.sql
apps/game-server/src/bin/oteryn-game-migrate.rs
apps/game-server/src/durability/admission_journal.rs
apps/game-server/src/durability/db.rs
apps/game-server/src/durability/mod.rs
apps/game-server/src/durability/schema.rs
apps/game-server/tests/durability_postgres.rs
apps/game-server/tests/support/postgres.rs
```

These are the complete non-task paths already exercised by the clean Durability successor #243. PR #243 itself remains untouched; the fresh allocation may use its exact admitted evidence/blobs only under the control plane's provenance rules, not by inheriting its stale authority state.

### Serialized composition lease

```text
apps/game-server/src/lib.rs
```

Protected `main` currently has no composed `durability` module. The fresh allocation must therefore receive serialized ownership of `src/lib.rs` sufficient only to compose/export the new Durability module as required by the existing game-server composition pattern and to make workspace/component validation exercise it. This is a composition lease, not authority to alter gameplay bootstrap semantics.

No `Cargo.toml`, `Cargo.lock`, workflow, resource registry, Server Seam or unrelated composition path is pre-authorized. Protected `main` already has the game-server `sqlx`, `serde_json` and `tokio` dependencies needed by the clean successor, so no Cargo mutation is justified by currently known evidence. The coordinator-owned allocation/task record is separate governance metadata and may be created/updated under the coordinator's normal task authority.

If fresh RED evidence proves a further implementation path is mechanically required, the worker must stop and return a scope-expansion packet to the control plane; it must not self-expand.

PR #243 remains untouched and `WAITING_ARCHITECTURE` until the decision is merged and the coordinator issues the new allocation. The new allocation, not this document, grants implementation write authority.

## 9. Required RED/GREEN proof obligations for the later allocation

### Foundation contract proof

- RED then GREEN: terminal replacement authorization cannot be built from `Active` or `Reconnectable` predecessor state;
- no-current-transport is required for terminal evidence;
- predecessor GameSession, connection generation and lease generation are exact in the authorization;
- the authorization carries the exact current Foundation terminal scope generation even when it is greater than the predecessor's last persisted Durability scope generation;
- candidate GameSession/account/character/world binding mismatch is rejected;
- V2 reconciliation preserves `TRANSPORT_REF_COLLISION` distinctly;
- V2 direct same-PREPARE `ExistingTerminal` replay preserves `TRANSPORT_REF_COLLISION` distinctly and updates the attempt budget as collision-terminal;
- legacy/generic V1 `ExistingTerminal` forces same-attempt typed reconciliation rather than generic terminal completion;
- only collision terminal reason can unlock collision replacement under remaining attempt capacity;
- `CONCURRENT_PREPARED` and `STALE_AUTHORITY` remain terminal without collision-remint semantics;
- no DB/network wait is introduced into the logical writer contract.

### Durability/PostgreSQL proof

- real isolated PostgreSQL race: two candidate GameSessions for one CharacterId cannot both become non-terminal authoritative anchors;
- terminal predecessor + exact authorization allows exactly one ordered predecessor->candidate replacement;
- active/reconnectable predecessor cannot be replaced merely because a Durability deadline expired;
- stale predecessor GameSession/connection/lease authorization fails without mutation;
- predecessor persisted scope generation lower than the current terminal Foundation generation is advanced exactly forward inside the same replacement transaction and does not strand the character;
- predecessor persisted scope generation greater than the Foundation terminal generation fails closed with no mutation;
- concurrent replacement CAS permits at most one winner;
- lost replacement response replays idempotently only for the exact predecessor/candidate binding;
- any predecessor PREPARED attempt is fenced/terminalized atomically with replacement and cannot later COMMIT;
- replacement transaction failure leaves predecessor anchor unchanged and candidate not PREPARED;
- collision terminal reason survives process restart, direct same-PREPARE replay and same-attempt reconciliation as `TRANSPORT_REF_COLLISION`;
- concurrent/stale terminal reasons likewise round-trip distinctly;
- existing transport-ref uniqueness race, one-ref-per-attempt, 8/9 attempt budget, ambiguous reconciliation and migration compatibility tests remain green.

### Clean-successor composition proof

- the complete Section 8 path set compiles from the fresh protected-main allocation base without relying on PR #243's branch authority;
- `apps/game-server/src/lib.rs` composes the Durability module without opening gameplay availability or unrelated runtime behavior;
- the migration binary builds against the same embedded ledger/schema used by the adapter;
- the real PostgreSQL harness uses `tests/support/postgres.rs` and passes the focused durability target;
- workspace/component validation exercises the composed Durability module, not only a test-local `#[path]` import.

## 10. Independent qualification and merge gate

The architecture artifact itself requires the repository's normal exact-head checks and genuinely independent exact-head architecture/governance review before merge.

The later implementation allocation requires:

- TDD RED owned by the fresh allocation before repair code;
- focused Foundation contract tests;
- real PostgreSQL E2E/race tests;
- clean-successor module/composition/migration-binary validation;
- component/workspace validation required by current repository policy;
- exact-head self-review;
- genuinely independent exact-head authority/persistence review with no unresolved P0/P1/P2;
- control-plane-only merge authorization.

The Supervising Architect does not merge either this decision PR or provider implementation PRs.

## 11. Resolution handback

When this decision is merged and read back from protected `main`:

```text
Issue #248 architecture question = RESOLVED
Decision = DUR-TERMINAL-SESSION-REPLACEMENT-V1
PR #243 = still paused; do not mutate it directly
Work coordinator = authorized to issue one fresh bounded serialized Foundation/Durability repair allocation using Section 8
Durability #167 = resumes only under that fresh allocation
Server Seam #247 = remains WAITING_DURABILITY_MERGE until repaired Durability is independently qualified and merged
```

No owner/product decision remains inside the bounded #248 architecture question.
