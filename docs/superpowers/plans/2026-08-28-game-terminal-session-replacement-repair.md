# Terminal GameSession Replacement Repair Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement canonical terminal predecessor -> candidate `GameSession` replacement plus typed durable terminal replay/reconciliation on a fresh protected-main branch, with real PostgreSQL race/restart proof and no authority expansion beyond the accepted architecture.

**Architecture:** Foundation remains the only lifecycle authority and emits a versioned exact terminal replacement authorization. Durability consumes that authorization inside the candidate PREPARE transaction, serializes the predecessor->candidate actor-anchor replacement, preserves historical terminal outcomes, and exposes one typed terminal-disposition domain for direct replay and reconciliation. The existing PR #243 bytes are admitted only as a clean baseline source; the repair produces fresh RED -> GREEN evidence and new exact-head qualification.

**Tech Stack:** Rust 1.94, SQLx 0.9.x already present on protected main, PostgreSQL 17, GitHub Actions, repository governance validators.

**Spec:** `docs/architecture/reviews/OTERYN_GAME_TERMINAL_SESSION_REPLACEMENT_COLLISION_RECONCILIATION_DECISION_2026-08-28.md`

## Global Constraints

- Admission starts only after the fresh allocation merge is read back from protected `main`.
- Worker branch: `impl/game-terminal-session-replacement-250`, created from the allocation merge SHA only.
- PR #243 head `eb28c42125c346e7f6f1c72e69d51af35af8fc1f` is file-content evidence only; do not inherit its commits or qualification.
- Existing `DUR-RECONNECT-AUTHORITY-V1`, `DUR-RECONNECT-TRANSPORT-REF-UNIQUENESS-V1`, one-ref-per-attempt, same-attempt-remint prohibition and `FND04-RECONNECT-ATTEMPTS-PER-LOSS-EPOCH = 8` remain unchanged.
- No Cargo/lockfile/workflow/resource-registry/Server Seam/production/external-repository mutation.
- `apps/game-server/src/lib.rs` lease is only for composing/exporting Durability; gameplay availability remains fail-closed.
- Every semantic increment is TDD RED -> GREEN. Historical #243 tests do not satisfy the new repair RED/qualification.

---

### Task 1: Reconstruct the admitted Durability baseline on clean ancestry

**Files:**
- Create from exact admitted blobs: `apps/game-server/build.rs`
- Create from exact admitted blobs: `apps/game-server/migrations/0001_admission_reconnect_journal.sql`
- Create from exact admitted blobs: `apps/game-server/src/bin/oteryn-game-migrate.rs`
- Create from exact admitted blobs: `apps/game-server/src/durability/admission_journal.rs`
- Create from exact admitted blobs: `apps/game-server/src/durability/db.rs`
- Create from exact admitted blobs: `apps/game-server/src/durability/mod.rs`
- Create from exact admitted blobs: `apps/game-server/src/durability/schema.rs`
- Create from exact admitted blobs: `apps/game-server/tests/durability_postgres.rs`
- Create from exact admitted blobs: `apps/game-server/tests/support/postgres.rs`
- Modify metadata only: `docs/agents/tasks/active/OTV2-20260828-terminal-session-replacement-repair.md`

**Interfaces:**
- Consumes: protected-main Foundation V1 reconnect contract and exact PR #243 source manifest from the allocation.
- Produces: a clean-history baseline byte-equivalent to the nine admitted PR #243 blobs, with no terminal-replacement repair yet.

- [ ] **Step 1: Fetch the exact historical source object without changing branch ancestry**

```bash
git fetch origin eb28c42125c346e7f6f1c72e69d51af35af8fc1f
```

Expected: the object is locally readable; current branch ancestry still begins at the allocation merge, not PR #243.

- [ ] **Step 2: Copy exactly the nine admitted file contents**

```bash
mkdir -p apps/game-server/migrations apps/game-server/src/bin apps/game-server/src/durability apps/game-server/tests/support
git show eb28c42125c346e7f6f1c72e69d51af35af8fc1f:apps/game-server/build.rs > apps/game-server/build.rs
git show eb28c42125c346e7f6f1c72e69d51af35af8fc1f:apps/game-server/migrations/0001_admission_reconnect_journal.sql > apps/game-server/migrations/0001_admission_reconnect_journal.sql
git show eb28c42125c346e7f6f1c72e69d51af35af8fc1f:apps/game-server/src/bin/oteryn-game-migrate.rs > apps/game-server/src/bin/oteryn-game-migrate.rs
git show eb28c42125c346e7f6f1c72e69d51af35af8fc1f:apps/game-server/src/durability/admission_journal.rs > apps/game-server/src/durability/admission_journal.rs
git show eb28c42125c346e7f6f1c72e69d51af35af8fc1f:apps/game-server/src/durability/db.rs > apps/game-server/src/durability/db.rs
git show eb28c42125c346e7f6f1c72e69d51af35af8fc1f:apps/game-server/src/durability/mod.rs > apps/game-server/src/durability/mod.rs
git show eb28c42125c346e7f6f1c72e69d51af35af8fc1f:apps/game-server/src/durability/schema.rs > apps/game-server/src/durability/schema.rs
git show eb28c42125c346e7f6f1c72e69d51af35af8fc1f:apps/game-server/tests/durability_postgres.rs > apps/game-server/tests/durability_postgres.rs
git show eb28c42125c346e7f6f1c72e69d51af35af8fc1f:apps/game-server/tests/support/postgres.rs > apps/game-server/tests/support/postgres.rs
```

- [ ] **Step 3: Verify every copied blob exactly**

```bash
test "$(git hash-object apps/game-server/build.rs)" = 3a8149ef075f6896a7435c716cb8a4de5d94606b
test "$(git hash-object apps/game-server/migrations/0001_admission_reconnect_journal.sql)" = 1281fae90744a1b906148a48453e7c09142300c5
test "$(git hash-object apps/game-server/src/bin/oteryn-game-migrate.rs)" = 80e72fcdeeb70359986a5f93fe287362c0d205a1
test "$(git hash-object apps/game-server/src/durability/admission_journal.rs)" = c4b289c16d12b41798268325a202c20e798d9971
test "$(git hash-object apps/game-server/src/durability/db.rs)" = 48746007625646dee9d8a44972005cacb2a97c73
test "$(git hash-object apps/game-server/src/durability/mod.rs)" = f37fd5e1d8ae50e8b71391a85da73369ac25fcb5
test "$(git hash-object apps/game-server/src/durability/schema.rs)" = 8c92e301bd420a386f8684025ba429903b1b6e91
test "$(git hash-object apps/game-server/tests/durability_postgres.rs)" = 2a1b99c670efc13e9464537129adeaa59b3c54c0
test "$(git hash-object apps/game-server/tests/support/postgres.rs)" = bcb243f6c4823a14ec8116b72439c2c79c115d94
```

Expected: all commands exit 0.

- [ ] **Step 4: Commit the clean baseline reconstruction**

```bash
git add apps/game-server/build.rs apps/game-server/migrations/0001_admission_reconnect_journal.sql apps/game-server/src/bin/oteryn-game-migrate.rs apps/game-server/src/durability apps/game-server/tests/durability_postgres.rs apps/game-server/tests/support/postgres.rs
git commit -m "test(durability): reconstruct admitted clean baseline"
```

Do not call this head GREEN for the #250 repair; it is only the admitted baseline.

---

### Task 2: Produce fresh Foundation and PostgreSQL RED for every canonical repair obligation

**Files:**
- Modify test-only sections: `apps/game-server/src/foundation/admission_recovery_inner.rs`
- Modify tests only: `apps/game-server/tests/durability_postgres.rs`

**Interfaces:**
- Consumes: V1 `GameSessionAuthoritySnapshot`, `ReconnectDurabilityRecordV1`, existing V1 prepare/reconciliation flow and reconstructed PostgreSQL journal.
- Produces: failing tests naming every new Section 9 terminal-replacement/V2 behavior before repair implementation exists.

- [ ] **Step 1: Add Foundation lifecycle and constructor-binding RED tests**

Add these exact test names under the existing durability reconnect test module:

```rust
#[test]
fn terminal_replacement_authorization_requires_terminal_transportless_predecessor() {
    // Active and Reconnectable snapshots must be rejected.
    // Terminal + Some(current_transport) must be rejected.
    // Terminal + None is the only eligible lifecycle shape.
}

#[test]
fn terminal_replacement_authorization_carries_current_scope_not_only_committed_scope() {
    // Build a terminal snapshot whose current scope generation is greater than the
    // fresh-admission committed generation and assert the authorization carries
    // the exact newer current generation.
}

#[test]
fn terminal_replacement_authorization_rejects_predecessor_session_mismatch() {
    // Exact constructor-level negative proof for predecessor GameSessionId.
}

#[test]
fn terminal_replacement_authorization_rejects_predecessor_connection_generation_mismatch() {
    // Exact constructor-level negative proof for predecessor connection generation.
}

#[test]
fn terminal_replacement_authorization_rejects_predecessor_lease_generation_mismatch() {
    // Exact constructor-level negative proof for predecessor CharacterLease generation.
}

#[test]
fn terminal_replacement_authorization_rejects_candidate_session_mismatch() {
    // Candidate ReconnectDurabilityRecordV1 GameSessionId must equal the authorization candidate.
}

#[test]
fn terminal_replacement_authorization_rejects_candidate_account_mismatch() {
    // Candidate account identity mismatch must fail before any durability request is emitted.
}

#[test]
fn terminal_replacement_authorization_rejects_candidate_character_mismatch() {
    // Candidate CharacterId mismatch must fail at the Foundation constructor boundary.
}

#[test]
fn terminal_replacement_authorization_rejects_candidate_world_mismatch() {
    // Candidate WorldId mismatch must fail at the Foundation constructor boundary.
}

#[test]
fn generic_v1_existing_terminal_requires_typed_same_attempt_reconciliation() {
    // Feed ReconnectPrepareDispositionV1::ExistingTerminal to the migrated V1
    // compatibility flow and assert ReconcileSameAttempt/ReconciliationRequired,
    // never a generic terminal completion that could be interpreted as collision.
}

#[test]
fn v2_direct_existing_terminal_collision_marks_budget_and_respects_capacity() {
    // Feed a direct V2 ExistingTerminal { TransportRefCollision } completion for the exact
    // same PREPARE request. Assert the exact attempt becomes collision-terminal and a fresh
    // attempt is permitted only when the unchanged eight-attempt budget has remaining capacity.
}

#[test]
fn v2_direct_existing_terminal_noncollision_never_unlocks_fresh_attempt() {
    // Feed ConcurrentPrepared and StaleAuthority direct terminal replay outcomes and prove
    // neither one receives collision-remint/fresh-attempt eligibility.
}

#[test]
fn v2_reconciliation_preserves_all_terminal_dispositions_and_collision_only_remint() {
    // Reconcile TransportRefCollision, ConcurrentPrepared and StaleAuthority separately.
    // Assert all three remain distinct terminal outcomes.
    // Assert only TransportRefCollision marks the attempt collision-terminal and can
    // unlock a fresh attempt when the existing eight-attempt budget still has capacity.
    // ConcurrentPrepared and StaleAuthority must never acquire collision-remint semantics.
}
```

Every constructor mismatch above is a separate assertion path. A PostgreSQL rejection later in the flow does not satisfy these Foundation-level RED obligations.

- [ ] **Step 2: Add PostgreSQL RED tests**

Add these exact test names to `durability_postgres.rs`:

```rust
#[tokio::test]
async fn terminal_replacement_forward_syncs_lagging_scope_fence_atomically() { /* fixture + assertion */ }

#[tokio::test]
async fn terminal_replacement_rejects_scope_fence_ahead_of_foundation_authority() { /* fixture + assertion */ }

#[tokio::test]
async fn terminal_replacement_rejects_live_or_mismatched_predecessor_without_mutation() { /* fixture + assertion */ }

#[tokio::test]
async fn terminal_replacement_lost_response_replays_only_exact_receipt_binding() {
    // Commit predecessor->candidate replacement, simulate response loss/process replacement,
    // then retry the exact request and prove idempotent success from the durable receipt.
}

#[tokio::test]
async fn terminal_replacement_conflicting_receipt_binding_fails_closed() {
    // After one replacement receipt exists, retry with a different predecessor or candidate
    // binding and prove conflict/stale rejection with no actor-anchor mutation.
}

#[tokio::test]
async fn terminal_replacement_fences_predecessor_prepared_attempt_against_late_commit() {
    // Leave a predecessor attempt PREPARED, perform the authorized replacement, then issue
    // the predecessor's late COMMIT and prove it cannot become ACTIVE/COMMITTED authority.
}

#[tokio::test]
async fn terminal_replacement_mid_transaction_failure_rolls_back_predecessor_and_candidate() {
    // In the isolated database, install a test-local PostgreSQL trigger/fault that raises after
    // the replacement transaction has reached the candidate-insert phase, i.e. after scope
    // synchronization/predecessor terminalization would have executed in transaction order.
    // Assert the error rolls back the predecessor scope/session/attempt changes, no replacement
    // receipt remains committed, the predecessor anchor remains unchanged, and the candidate
    // is absent/not PREPARED. Remove the fault in fixture cleanup.
}

#[tokio::test]
async fn collision_existing_terminal_replay_preserves_typed_collision_reason() { /* fixture + restart + replay */ }

#[tokio::test]
async fn v2_reconciliation_round_trips_collision_concurrent_and_stale_distinctly() {
    // Persist each terminal attempt class, replace/restart the process boundary as applicable,
    // reconcile the same exact attempt, and assert the exact typed terminal disposition.
}

#[tokio::test]
async fn concurrent_terminal_replacement_has_exactly_one_candidate_winner() { /* two transactions/tasks */ }
```

Use the existing isolated PostgreSQL fixture/helpers; do not add another database harness. The rollback test must fail after replacement mutation has logically begun, not at input validation, so it proves transaction atomicity rather than precondition rejection.

- [ ] **Step 3: Run the Foundation RED**

```bash
cargo +1.94.0 test --locked -p oteryn-game-server terminal_replacement_authorization -- --nocapture
cargo +1.94.0 test --locked -p oteryn-game-server generic_v1_existing_terminal_requires_typed_same_attempt_reconciliation -- --nocapture
cargo +1.94.0 test --locked -p oteryn-game-server v2_direct_existing_terminal -- --nocapture
cargo +1.94.0 test --locked -p oteryn-game-server v2_reconciliation_preserves_all_terminal_dispositions_and_collision_only_remint -- --nocapture
```

Expected: FAIL because terminal replacement/V2 typed direct replay/reconciliation semantics are not implemented. The `terminal_replacement_authorization` filter must execute lifecycle, current-scope, predecessor session/connection/lease and candidate session/account/character/world cases.

- [ ] **Step 4: Run the PostgreSQL RED**

```bash
cargo +1.94.0 test --locked -p oteryn-game-server --test durability_postgres terminal_replacement -- --nocapture
cargo +1.94.0 test --locked -p oteryn-game-server --test durability_postgres collision_existing_terminal_replay_preserves_typed_collision_reason -- --nocapture
cargo +1.94.0 test --locked -p oteryn-game-server --test durability_postgres v2_reconciliation_round_trips_collision_concurrent_and_stale_distinctly -- --nocapture
```

Expected: FAIL on the new assertions/contract gaps; PostgreSQL service must actually be available. A skipped target is not RED. The `terminal_replacement` filter must execute the scope, exact-receipt replay, conflicting-receipt, late-COMMIT fencing, mid-transaction rollback and contention cases above.

- [ ] **Step 5: Publish and preserve the RED head**

```bash
git add apps/game-server/src/foundation/admission_recovery_inner.rs apps/game-server/tests/durability_postgres.rs
git commit -m "test(reconnect): prove terminal replacement repair RED"
```

Open the worker PR as Draft and record the exact RED head + workflow/job evidence in the PR conversation before implementing the fix.

---

### Task 3: Implement the Foundation V2 terminal replacement and typed replay contract

**Files:**
- Modify: `apps/game-server/src/foundation/admission_recovery_inner.rs`

**Interfaces:**
- Consumes: current Foundation authority snapshot, candidate `ReconnectDurabilityRecordV1`.
- Produces: exact terminal replacement authorization, V2 prepare request/disposition/completion, typed terminal disposition, V2 reconciliation snapshot; legacy V1 `ExistingTerminal` compatibility routes to reconciliation.

- [ ] **Step 1: Add the terminal replacement authorization semantic type**

Use this target shape (exact private storage representation may follow local conventions, but these semantics are mandatory):

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalGameSessionReplacementAuthorizationV1 {
    account_id: String,
    character_id: CharacterId,
    world_id: WorldId,
    predecessor_game_session_id: GameSessionId,
    predecessor_connection_generation: ConnectionGeneration,
    predecessor_character_lease_generation: u64,
    predecessor_current_scope_ownership_generation: ScopeOwnershipGeneration,
    candidate_game_session_id: GameSessionId,
}
```

Expose read-only accessors and a constructor/helper that succeeds only after the current Foundation snapshot is Terminal, transportless, exact on predecessor identity/connection/lease/current scope, and the candidate record matches account/character/world/candidate session. Do not accept deadlines or Durability-local state as terminal proof. Each predecessor and candidate binding must be independently checked so any single mismatch fails construction before a V2 PREPARE request exists.

- [ ] **Step 2: Add one typed terminal domain used by direct replay and reconciliation**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconnectDurableTerminalDispositionV1 {
    TransportRefCollision,
    ConcurrentPrepared,
    StaleAuthority,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReconnectDurableOutcomeV2 {
    Prepared,
    Committed {
        current_generation: ConnectionGeneration,
        current_transport_ref: AuthenticatedTransportRefV1,
    },
    Terminal {
        disposition: ReconnectDurableTerminalDispositionV1,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconnectDurableReconciliationSnapshotV2 {
    record: ReconnectDurabilityRecordV1,
    outcome: ReconnectDurableOutcomeV2,
}
```

- [ ] **Step 3: Add V2 PREPARE request/disposition and state-machine handling so direct terminal replay is typed**

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconnectPrepareRequestV2 {
    record: Box<ReconnectDurabilityRecordV1>,
    terminal_replacement: Option<TerminalGameSessionReplacementAuthorizationV1>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconnectPrepareDispositionV2 {
    Prepared,
    ExistingPrepared,
    RejectedTransportRefCollision,
    RejectedConcurrentPrepared,
    RejectedStaleAuthority,
    AttemptCapacityExceeded,
    ExistingTerminal {
        disposition: ReconnectDurableTerminalDispositionV1,
    },
    Unavailable,
    Ambiguous,
    IdempotencyConflict,
}
```

Add matching V2 completion/action handling. For a direct same-request `ExistingTerminal { disposition: TransportRefCollision }`, Foundation must update the exact attempt budget to collision-terminal exactly as for the original direct collision completion, then permit a new attempt only if the unchanged loss-epoch budget has capacity and no PREPARED attempt blocks it. `ConcurrentPrepared` and `StaleAuthority` direct replays remain terminal and never unlock a fresh attempt. V2 reconciliation must preserve the same three distinctions.

- [ ] **Step 4: Make legacy V1 `ExistingTerminal` fail into typed reconciliation**

Change the V1 compatibility match so this path is explicit:

```rust
ReconnectPrepareDispositionV1::ExistingTerminal => {
    self.phase = ReconnectDurabilityPhaseV1::ReconciliationRequired;
    Ok(ReconnectPrepareActionV1::ReconcileSameAttempt)
}
```

Do not map generic V1 terminality to collision.

- [ ] **Step 5: Run Foundation tests to GREEN**

```bash
cargo +1.94.0 test --locked -p oteryn-game-server terminal_replacement_authorization -- --nocapture
cargo +1.94.0 test --locked -p oteryn-game-server generic_v1_existing_terminal_requires_typed_same_attempt_reconciliation -- --nocapture
cargo +1.94.0 test --locked -p oteryn-game-server v2_direct_existing_terminal -- --nocapture
cargo +1.94.0 test --locked -p oteryn-game-server v2_reconciliation_preserves_all_terminal_dispositions_and_collision_only_remint -- --nocapture
cargo +1.94.0 test --locked -p oteryn-game-server durability_reconnect_v1_tests -- --nocapture
```

Expected: PASS, including all constructor-binding negatives, direct V2 collision/noncollision replay state transitions and reconciliation outcomes.

- [ ] **Step 6: Commit Foundation contract implementation**

```bash
git add apps/game-server/src/foundation/admission_recovery_inner.rs
git commit -m "feat(foundation): authorize terminal session replacement"
```

---

### Task 4: Implement atomic PostgreSQL predecessor replacement and typed durable outcomes

**Files:**
- Modify: `apps/game-server/migrations/0001_admission_reconnect_journal.sql`
- Modify: `apps/game-server/src/durability/admission_journal.rs`
- Modify: `apps/game-server/src/durability/schema.rs`
- Modify tests: `apps/game-server/tests/durability_postgres.rs`

**Interfaces:**
- Consumes: `ReconnectPrepareRequestV2`, `TerminalGameSessionReplacementAuthorizationV1`, typed terminal disposition.
- Produces: serialized terminal predecessor->candidate replacement, idempotent replacement receipt, typed direct terminal replay and V2 reconciliation.

- [ ] **Step 1: Extend the migration to preserve historical terminal rows while retaining one live actor anchor**

Use session states `1=RECONNECTABLE`, `2=ACTIVE`, `3=TERMINAL`. Replace actor-wide unconditional `UNIQUE(character_id)` with a partial unique live-anchor index:

```sql
session_state SMALLINT NOT NULL DEFAULT 1 CHECK (session_state BETWEEN 1 AND 3);

CREATE UNIQUE INDEX game_durability_one_nonterminal_session_per_character
    ON game_durability_reconnect_sessions (character_id)
    WHERE session_state IN (1, 2);
```

Add an exact idempotency receipt:

```sql
CREATE TABLE game_durability_session_replacements (
    character_id UUID NOT NULL,
    predecessor_game_session_id UUID NOT NULL,
    candidate_game_session_id UUID NOT NULL,
    predecessor_scope_ownership_generation NUMERIC(20, 0) NOT NULL
        CHECK (predecessor_scope_ownership_generation BETWEEN 1 AND 18446744073709551615),
    replaced_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (character_id, predecessor_game_session_id, candidate_game_session_id),
    UNIQUE (character_id, candidate_game_session_id)
);
```

Keep historical predecessor attempts intact for reconciliation.

- [ ] **Step 2: Add locked actor-anchor loading by CharacterId before candidate insertion**

In `admission_journal.rs`, add a helper with the semantic query:

```sql
SELECT *
FROM game_durability_reconnect_sessions
WHERE character_id = encode($1, 'hex')::uuid
  AND session_state IN (1, 2)
FOR UPDATE
```

The candidate must not first rely on `INSERT ... ON CONFLICT DO NOTHING` to decide replacement. When another nonterminal session owns the character, proceed only with an exact terminal replacement authorization.

- [ ] **Step 3: Implement ordered replacement inside one PostgreSQL transaction**

Before candidate attempt/ref becomes PREPARED:

```text
lock exact current actor anchor
verify character/session/account/world/current_generation/lease exact
verify stored_scope <= Foundation-authorized current terminal scope
if stored_scope < authorized current: update exactly forward to that value
terminalize predecessor PREPARED attempts as STALE_TERMINAL
clear predecessor prepared/current transport projection
set predecessor session_state = TERMINAL
insert exact replacement receipt, or verify an exact existing receipt for idempotent replay
insert/establish candidate as the sole RECONNECTABLE nonterminal actor anchor
continue ordinary candidate PREPARE reservation/classification
commit once, only after all steps succeed
```

Any mismatch, `stored_scope > authorized`, live/nonterminal Foundation evidence mismatch, conflicting existing replacement receipt, or competing winner returns stale/mismatched authority with no partial candidate PREPARED state. A lost response may replay idempotently only when the exact existing `(CharacterId, predecessor_game_session_id, candidate_game_session_id)` receipt proves the same replacement committed. Replacement must atomically terminalize/fence every predecessor PREPARED attempt so a late predecessor COMMIT cannot restore controller authority.

No intermediate step may commit independently. Any database error after scope synchronization or predecessor terminalization but before candidate PREPARED authority must abort the same SQL transaction so every predecessor mutation and replacement receipt is rolled back.

- [ ] **Step 4: Return typed terminal reason on existing-attempt direct replay and reconciliation**

Map stored attempt states exactly:

```rust
COLLISION_TERMINAL => ReconnectDurableTerminalDispositionV1::TransportRefCollision,
CONCURRENT_TERMINAL => ReconnectDurableTerminalDispositionV1::ConcurrentPrepared,
STALE_TERMINAL => ReconnectDurableTerminalDispositionV1::StaleAuthority,
```

An existing terminal V2 PREPARE replay returns `ReconnectPrepareDispositionV2::ExistingTerminal { disposition }`. Reconciliation returns the same disposition through `ReconnectDurableReconciliationSnapshotV2`. Only the collision disposition may mark the attempt collision-terminal for the existing bounded fresh-attempt rule; concurrent/stale outcomes remain terminal without remint eligibility.

- [ ] **Step 5: Update schema contract tests**

Assert the migration contains:

```text
session_state BETWEEN 1 AND 3
partial unique nonterminal CharacterId index
replacement receipt table and exact predecessor/candidate binding
full-range NUMERIC(20,0) scope fence
```

- [ ] **Step 6: Run the PostgreSQL repair suite to GREEN**

```bash
cargo +1.94.0 test --locked -p oteryn-game-server --test durability_postgres terminal_replacement -- --nocapture
cargo +1.94.0 test --locked -p oteryn-game-server --test durability_postgres collision_existing_terminal_replay_preserves_typed_collision_reason -- --nocapture
cargo +1.94.0 test --locked -p oteryn-game-server --test durability_postgres v2_reconciliation_round_trips_collision_concurrent_and_stale_distinctly -- --nocapture
cargo +1.94.0 test --locked -p oteryn-game-server --test durability_postgres concurrent_terminal_replacement_has_exactly_one_candidate_winner -- --nocapture
cargo +1.94.0 test --locked -p oteryn-game-server --test durability_postgres -- --nocapture
```

Expected: all PASS against an actual isolated PostgreSQL 17 service, including exact-receipt lost-response replay, conflicting-receipt rejection, predecessor late-COMMIT fencing, forced mid-transaction rollback after predecessor mutation begins, all typed terminal reconciliation outcomes, collision-only remint behavior, and the pre-existing journal regressions.

- [ ] **Step 7: Commit the durable implementation**

```bash
git add apps/game-server/migrations/0001_admission_reconnect_journal.sql apps/game-server/src/durability/admission_journal.rs apps/game-server/src/durability/schema.rs apps/game-server/tests/durability_postgres.rs
git commit -m "feat(durability): replace terminal session anchor atomically"
```

---

### Task 5: Compose Durability and prove the clean successor as a package/workspace candidate

**Files:**
- Modify: `apps/game-server/src/lib.rs`
- Verify unchanged baseline support: `apps/game-server/build.rs`
- Verify unchanged baseline support: `apps/game-server/src/bin/oteryn-game-migrate.rs`
- Verify unchanged baseline support: `apps/game-server/src/durability/db.rs`
- Verify module export: `apps/game-server/src/durability/mod.rs`
- Modify task metadata before freeze: `docs/agents/tasks/active/OTV2-20260828-terminal-session-replacement-repair.md`

**Interfaces:**
- Consumes: repaired Foundation and Durability modules.
- Produces: game-server package that actually compiles the Durability module and migration binary without making gameplay available.

- [ ] **Step 1: Compose the Durability module in the game-server library**

Add the module alongside the existing content/domain/foundation modules:

```rust
pub mod content;
pub mod domain;
pub mod durability;
pub mod foundation;
```

Do not change `GameplayAvailability::UnavailableBootstrap` or `GAMEPLAY_UNAVAILABLE_REASON`.

- [ ] **Step 2: Prove migration binary and composed package build**

```bash
cargo +1.94.0 check --locked -p oteryn-game-server --bin oteryn-game-migrate
cargo +1.94.0 test --locked -p oteryn-game-server bootstrap_is_explicitly_gameplay_unavailable -- --nocapture
```

Expected: PASS; Durability is compiled while gameplay remains unavailable.

- [ ] **Step 3: Run formatting, lint and component/workspace validation**

```bash
cargo +1.94.0 fmt --all -- --check
cargo +1.94.0 clippy --locked -p oteryn-game-server --all-targets --all-features -- -D warnings
cargo +1.94.0 test --locked -p oteryn-game-server
cargo +1.94.0 test --locked --workspace
```

Also run the current repository/governance commands required by `BUILD_TEST_MATRIX.md` and exact-head GitHub workflows.

- [ ] **Step 4: Commit composition and finish tracked task metadata before freezing**

```bash
git add apps/game-server/src/lib.rs docs/agents/tasks/active/OTV2-20260828-terminal-session-replacement-repair.md
git commit -m "feat(game-server): compose repaired durability boundary"
```

Do not make a later commit solely to write this commit's own SHA into the task file.

- [ ] **Step 5: Freeze and self-review the whole diff**

```bash
git diff --check origin/main...HEAD
git diff --name-only origin/main...HEAD
```

Expected changed semantic paths: exactly the canonical eleven implementation/composition paths plus the worker task metadata. Review every changed line against the canonical ADR and confirm no Cargo/workflow/Server Seam/resource-registry path appears.

- [ ] **Step 6: Require fresh exact-head CI and independent review**

On the frozen head, require repository workflows plus the real PostgreSQL focused run. Then, as the allocated lane lead, post one top-level PR comment:

```text
@codex review
```

Bind the request to the exact full head in the accompanying text and request strict read-only review focused on SESSION / RECONNECT / FENCING / durable schema / contention / replay / authority.

- [ ] **Step 7: Return only a clean candidate to Work**

Return `READY_FOR_INTEGRATION` only when:

```text
exact-head focused PostgreSQL: PASS
exact-head component/workspace validation: PASS
whole-diff self-review: PASS
independent Codex exact-head review: PASS/no blocking findings
unresolved required review threads: 0
head unchanged after review: YES
```

Do not merge the worker PR. Work performs final current-main reconciliation and expected-head integration.
