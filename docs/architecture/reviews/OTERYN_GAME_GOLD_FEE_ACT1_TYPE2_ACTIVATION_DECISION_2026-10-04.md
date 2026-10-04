# GOLD-FEE-ACT-PACKET-1: the event type 2 activation to V2

```yaml
decision_id: GOLD-FEE-ACT-PACKET-1
status: CANDIDATE
date: 2026-10-04
owner: Sol Supervising Architect
requested_by: control plane D497 (6a; GOLD-FEE-ACT-1 moves out of ARCH-BATCH-ROOT-PACKETS-V1, #1733)
amends: ARCH-BATCH-ROOT-PACKETS-V1 §1.7 phase 2 (the switch), which now points here
writes_on_other_prs: none
```

ARCH-BATCH-ROOT-PACKETS-V1 (#1733) §1.7 moves every event type 2 producer from
`DUR03_ONE_ITEM_DURABLE_AUDIT_RETENTION_V1` at schema revision 1 to the successor
`DUR03_ONE_ITEM_DURABLE_AUDIT_RETENTION_V2` at revision 2, in two phases. Phase 1 is GOLD-FEE-2 and
stays in #1733: every node reads, verifies and accepts `(1, V1)` and `(2, V2)`, and still emits
`(1, V1)`. This decision packets phase 2, the switch. It also answers the two #1733 findings
against that switch, P1 4177113877 (in-flight inserts) and P1 4177113872 (every type-2 writer).

The decision changes no code, contract or wire. When this was written, `main` had no
GOLD-FEE-2, no BANK-1 and no activation table. The type-2 outbox is `0010`'s
`game_item_audit_outbox`, and every writer binds the compile-time constants
`item_mint_audit::EVENT_SCHEMA_REVISION` and `item_mint_audit::RETENTION_PROFILE_ID` directly:
`item_mint.rs`, `item_transfer.rs`, `reward_claim_mint.rs`, `item_decay_retire.rs`,
`item_timed_state.rs` and `item_fee_burn.rs`. Five of them open their own transaction with
`begin_semantic_transaction` and then take the recovery fence, the admission relations and the
Character root. The fee writer, `burn_fee_in_transaction`, opens none: it runs inside its source's
sequenced Character transaction, after those locks. `character_revision_sequencer.rs` allows that
call only from `charm_state.rs` today.

## 1. Rulings

### 1.1 Two packets, so no deploy runs old and new emitters across the boundary

A constant cannot change at a runtime boundary. So the switch needs code that picks its tuple at
runtime, deployed everywhere before the boundary, and then the boundary itself:

1. **GOLD-FEE-ACT-1, readiness.** Adds the activation table (empty), the fence (§1.2), the outbox
   trigger (§1.3), and routes every type-2 writer's tuple through a transaction-scoped activation
   read (§1.4). With the table empty every writer still emits `(1, V1)`, so it changes no
   behaviour. The registry stays at V1, revision 1.
2. **GOLD-FEE-ACT-2, the switch.** One migration inserts the activation row (§1.6), and in the same
   PR the registry's type-2 entry gets `retention_profile_id` V2 and `current_schema_revision` 2.
   - **Precondition.** Deploy evidence, recorded on the PR, that every node runs GOLD-FEE-ACT-1
     code, with no older node alive. Applying it needs no code change.

A GOLD-FEE-2 node alive during the GOLD-FEE-ACT-1 rollout emits its constant `(1, V1)`. The table
is empty then, so the trigger accepts it. Every insert, old binary included, takes the fence in
the trigger, so the switch cannot overtake it.

### 1.2 The fence (#1733 P1 4177113877)

Under READ COMMITTED, a trigger alone sees the snapshot of its statement. A V1 insert that started
before the activation commit, in a transaction that commits after it, would pass. So activation
and every type-2 insert are serialized by one transaction-scoped advisory lock:

- **The key.** One fixed 64-bit key, `TYPE2_AUDIT_ACTIVATION_FENCE`. It is defined once in
  `item_mint_audit.rs` and in the migration, and a test checks that the two values agree.
- **Writers take it shared, at the transaction opener.** Every transaction that can insert a
  type-2 event takes `pg_advisory_xact_lock_shared(key)` as its **first statement**, right after
  `BEGIN`. That is before the recovery fence, the admission relations, the Character root fence and
  any row lock, and it holds the lock until commit or rollback. The opener is the code that calls
  `begin_semantic_transaction`, not the writer function. So for the fee burn the opener is its
  source (§1.4), never `burn_fee_in_transaction` itself (#1746 P1 4177155975). Taking it first means
  no transaction waits for the fence while holding a row or root lock, so the fence adds no wait
  cycle.
- **Activation takes it exclusive.** The activation transaction runs `SET LOCAL lock_timeout = '5s'`
  and then `pg_advisory_xact_lock(key)`. Only then does it insert the row, and it commits.
- **Why this is enough.** The exclusive lock is granted only when no writer holds the shared one.
  So every transaction that read "absent" has committed or rolled back before the row commits. A
  writer that asks for the shared lock while activation holds it, or is waiting for it, waits until
  activation commits. Its next statement then sees the row.
- **Timeout.** A timeout aborts the activation with no effect, and the operator retries. Writers
  that waited behind it continue with the table still empty. A deadlock cannot form, because the
  activation transaction takes no other lock. If PostgreSQL's deadlock detector aborts anything,
  the abort has no effect.

### 1.3 The trigger, as a backstop

The migration adds a `BEFORE INSERT` trigger on `game_item_audit_outbox`:

- It runs `pg_advisory_xact_lock_shared(key)` itself. For a fenced writer this is a lock it
  already holds, so it is granted at once. It also fences an insert from a binary that does not
  take the fence (GOLD-FEE-2 during the GOLD-FEE-ACT-1 rollout, or any older binary).
- It then reads the row in a fresh statement. The function is `VOLATILE`, so the read sees every
  commit up to that statement.
- It refuses `(1, V1)` once the row exists and `(2, V2)` while it does not, with a distinct SQLSTATE.
  The transaction aborts with no effect. There is one exception, the grandfathered candidate of
  §1.5: once the row exists, a `(1, V1)` insert is admitted when its `event_id` and
  `envelope_sha256` equal those of a row in a reservation table that persists the exact envelope
  (`game_item_mint_reservations`, `game_item_decay_retire_reservations`).

For a fenced writer, the refusal cannot happen: its read (§1.4) and its insert are in one fence
hold, so they agree. A refusal is therefore a typed error, never retried. It only stops an
unfenced or older binary from writing V1 after the boundary. An unfenced binary takes the lock
in the trigger, after its row locks. GOLD-FEE-ACT-2's precondition (§1.1) is that no such binary
is alive when the exclusive request is made, so no wait cycle forms. If one is alive against the
precondition, PostgreSQL's deadlock detector aborts one side, with no effect.

### 1.4 Every type-2 transaction reads the activation when it opens (#1733 P1 4177113872, #1746 P1 4177155975)

`db.rs` gains `begin_type2_transaction(holder, deadline)`. It is `begin_semantic_transaction`
followed by the shared fence and the activation read, in that order, as the first statements. It
returns a `Type2Transaction`, which wraps the transaction and carries the selected tuple, `(1, V1)`
or `(2, V2)`. The tuple type has no other constructor. Every type-2 outbox insert and audit
encoding takes its tuple from a `Type2Transaction`, so a type-2 insert compiles only inside a
transaction opened this way. The compile-time revision and profile constants stop being bound in
SQL.

The openers that change:

- `item_mint.rs`, `item_transfer.rs`, `reward_claim_mint.rs`, `item_decay_retire.rs` and
  `item_timed_state.rs`. Each transaction that reaches its outbox insert is opened with
  `begin_type2_transaction`. Their read-only transactions are not changed.
- `charm_state.rs`, the fee source. The `charm_state.rs` transaction that calls the fee burn is
  opened with `begin_type2_transaction`, before its recovery fence, admission relations and
  Character root. It passes its `Type2Transaction` to the fee burn.
- `item_fee_burn.rs`. `burn_fee_in_transaction` takes the `Type2Transaction` in place of a bare
  transaction. It takes no lock of its own for the fence. With `(1, V1)`, a fee with `T < F` is
  refused as in stage 1. With `(2, V2)`, the bank part of GOLD-FEE-2 is open. This replaces the fixed
  `(1, V1)` of phase 1 (#1733 §1.7).
- Any other caller of `burn_fee_in_transaction` on `main` when the packet is allocated is in this
  packet's owned paths and changes the same way. That covers a later fee source (NPC-TRADE-1,
  NPC-TRAVEL-1, IMBUE-1, FORGE-1, CHARM-6) if one has merged by then. The control plane adds its
  file at allocation. The sequencer's caller allowlist stays the gate.

Coverage, checked by tests:

- **Insert sites.** A source test enumerates the outbox insert sites and the
  `burn_fee_in_transaction(` call sites in `apps/game-server/src/`. It fails if any of them is
  reached from a transaction not opened by `begin_type2_transaction`.
- **Ordering.** A source test checks that in each type-2 opener, `begin_type2_transaction` comes
  before `assert_recovery_fence`, `lock_admission_relations` and the gameplay fence.
- **New writers.** A type-2 writer added later uses the same opener.

### 1.5 Candidates frozen before the switch (#1746 P1 4177181872)

`item_mint.rs` and `item_decay_retire.rs` freeze the exact envelope in a durable reservation and
commit or reconcile it later, possibly after a restart, with the same bytes. Re-encoding the same
EventId as V2 would break that exact-byte retry rule. So the tuple of such an event is the one
persisted with its candidate, not a fresh read:

- **Reservation under the fence.** The transaction that creates a reservation with a persisted
  envelope is opened with `begin_type2_transaction`, and the envelope is encoded with its tuple. A
  reservation that commits before activation is therefore V1, and one that commits after is V2.
- **Resume uses the frozen tuple.** A transaction that commits or reconciles a reservation is
  still opened with `begin_type2_transaction`, but it takes the tuple from the reservation:
  `Type2Transaction::frozen_tuple(&reservation)` decodes the revision and profile from the persisted
  envelope. It refuses any value other than `(1, V1)` or `(2, V2)` as a typed error. This is the
  tuple type's second constructor, and it exists only on a `Type2Transaction`. A V2 tuple frozen
  before activation cannot exist, because the reservation took the fence.
- **The trigger admits it** (§1.3) only by exact match with the persisted reservation, so no other
  V1 insert passes after activation.
- **Bounded.** The grandfathered set is the reservations that exist when the row commits. No new
  one can be V1, and each is consumed by its existing commit or reconcile path. Activation needs no
  drain, and no reservation is left holding value.

Transfer and reward-claim reservations hold only the identifiers, and their envelope is encoded at
commit. They use the fresh tuple, so they are not grandfathered.

### 1.6 The switch

GOLD-FEE-ACT-2's migration takes the exclusive fence (§1.2) and inserts the one row
(`id = 1`, `activated_at = now()`). It uses `ON CONFLICT DO NOTHING`, so applying it twice is a
no-op. The table is insert-only: the migration that creates it grants no UPDATE, DELETE or
TRUNCATE, and a trigger refuses UPDATE and DELETE. Once the row commits, every new type-2 candidate
is `(2, V2)`. Every event before it keeps `(1, V1)`, and so does a candidate frozen before it
(§1.5)
(`existing_envelope_binding: ORIGINAL_RETENTION_PROFILE_ID`).

Rollback: no rollback by deleting the row. If V2 must stop, a reviewed successor decision is
needed, because events already admitted under V2 keep V2.

## 2. Packets

### 2.1 GOLD-FEE-ACT-1

```yaml
task_id: GOLD-FEE-ACT-1
decision: this decision §1.1-§1.5; ARCH-BATCH-ROOT-PACKETS-V1 §1.7
worker: oteryn-hard-worker
review: persistence review
branch: allocated by the control plane
base: main (GOLD-FEE-2 merged)
migration_lease: the next free number, leased by the control plane at allocation
depends_on: [GOLD-FEE-2]
owned_paths:
  - apps/game-server/migrations/<leased>_type2_audit_activation.sql  # the empty table, its grants and immutability trigger, the outbox BEFORE INSERT trigger and the fence key
  - apps/game-server/src/durability/item_mint_audit.rs  # the fence key and the tuple type
  - apps/game-server/src/durability/db.rs  # begin_type2_transaction only
  - apps/game-server/src/durability/charm_state.rs  # the fee source's transaction opener only (§1.4)
  - apps/game-server/src/durability/character_revision_sequencer.rs  # only if its fee-caller allowlist test must name the new signature
  - any other burn_fee_in_transaction caller on main at allocation, added by the control plane (§1.4)
  - apps/game-server/src/durability/item_mint.rs
  - apps/game-server/src/durability/item_transfer.rs
  - apps/game-server/src/durability/reward_claim_mint.rs
  - apps/game-server/src/durability/item_decay_retire.rs
  - apps/game-server/src/durability/item_timed_state.rs
  - apps/game-server/src/durability/item_fee_burn.rs
  - apps/game-server/tests/type2_audit_activation_*.rs
validation:
  - cargo test --locked -p oteryn-game-server type2_audit_activation
  - cargo test --locked -p oteryn-game-server item_mint
  - cargo test --locked -p oteryn-game-server item_transfer
  - cargo test --locked -p oteryn-game-server reward_claim_mint
  - cargo test --locked -p oteryn-game-server item_decay_retire
  - cargo test --locked -p oteryn-game-server item_timed_state
  - cargo test --locked -p oteryn-game-server item_fee_burn
  - cargo test --locked -p oteryn-game-server charm_state
  - cargo test --locked -p oteryn-game-server character_revision_sequencer
  - cargo check --locked --workspace --all-targets
  - python3 tools/agents/validate_governance.py
```

Tests:

- **Empty table.** Every writer writes `(1, V1)`, exactly as before. A fee with `T < F` is refused.
  The existing tests of the six writers pass unchanged.
- **Row present** (inserted by a test). Every writer writes `(2, V2)`, and GOLD-FEE-2's bank tests
  pass.
- **In-flight insert** (P1 4177113877). A V1 writer is paused by a test hook after its read and
  its insert, before commit. A concurrent activation waits and does not commit first. Once the
  writer commits, the activation commits, and the next writer writes `(2, V2)`. No `(1, V1)` row
  commits after the activation row.
- **Waiting writer.** A writer that asks for the fence while activation holds it writes `(2, V2)`.
- **Timeout.** An activation behind a writer that is held longer than 5 s aborts with no effect,
  and the writer commits `(1, V1)`.
- **Unfenced insert.** A raw `(1, V1)` insert that does not take the fence, from a connection
  holding no lock, is still fenced by the trigger and refused after the row. A raw `(2, V2)` is
  refused before it.
- **Coverage** (#1733 P1 4177113872, #1746 P1 4177155975). These are the two source tests of §1.4.
  Also, the fence key in Rust equals the key in the migration.
- **Fee source lock order** (#1746 P1 4177155975). A charm fee transaction and a concurrent
  activation are run with the activation's exclusive request queued. No wait cycle forms: the charm
  transaction either holds the fence from `BEGIN` and commits, or waits for the fence before it
  takes any Character root or row lock.
- **Frozen candidate** (#1746 P1 4177181872). A mint reservation and a decay retire reservation are
  created with the table empty, and then activation commits. Each one then commits through its normal
  path with its persisted `(1, V1)` bytes, unchanged. A reconcile after a simulated restart also
  works. A new reservation after activation is `(2, V2)`.
- **Grandfather is exact.** After activation, a `(1, V1)` insert whose `event_id` has no reservation,
  or whose `envelope_sha256` differs from the reservation's, is refused.
- **Grandfather coverage.** A source test lists every reservation table in the migrations that has
  an `envelope` column. It fails unless the trigger's list is exactly that set, and unless every
  writer that creates such a reservation opens it with `begin_type2_transaction`.
- **Immutability.** The row cannot be updated or deleted.
- **Mixed nodes.** A GOLD-FEE-2 binary and a GOLD-FEE-ACT-1 binary on one database, with the table
  empty, verify each other's events.

Acceptance: the tests above, the persistence review on the PR, and the migration merge condition
of ARCH-BATCH-ROOT-PACKETS-V1 §0.1.

### 2.2 GOLD-FEE-ACT-2

```yaml
task_id: GOLD-FEE-ACT-2
decision: this decision §1.1, §1.6
worker: oteryn-hard-worker
review: persistence review
branch: allocated by the control plane
base: main (GOLD-FEE-ACT-1 merged and deployed to every node)
migration_lease: the next free number, leased by the control plane at allocation
depends_on: [GOLD-FEE-ACT-1, BANK-RET-0]
owned_paths:
  - apps/game-server/migrations/<leased>_type2_audit_activate_v2.sql  # the exclusive fence and the row
  - docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json  # event type 2: retention_profile_id V2, current_schema_revision 2
  - apps/game-server/tests/type2_audit_activation_*.rs
  - tools/agents/tests/** (only if a registry test must name the binding)
validation:
  - cargo test --locked -p oteryn-game-server type2_audit_activation
  - python3 tools/agents/validate_governance.py
  - python3 tools/repository/validate_repository_policy.py
  - python3 -m unittest discover -s tools/agents/tests
```

Tests:

- the registry binding and the row agree: V2 at revision 2 with the row present;
- applying the migration twice is a no-op;
- after the migration, every writer writes `(2, V2)`, and stored `(1, V1)` events still verify as
  V1;
- an older binary that emits a constant `(1, V1)` is refused by the trigger rather than writing
  V1.

Acceptance: the tests above, the persistence review, the deploy evidence of §1.1, and the
migration merge condition.

## 3. Rejected options

- **The trigger alone, with retry** (the earlier #1733 text). Under READ COMMITTED it misses an
  insert already in progress when activation commits (P1 4177113877).
- **The switch in GOLD-FEE-ACT-1's own migration.** Its first node to start would insert the row
  while GOLD-FEE-2 nodes still emit a constant `(1, V1)`. The trigger would refuse them, so every
  type-2 operation on those nodes would fail until they were replaced.
- **`SERIALIZABLE` for type-2 transactions.** It aborts the late writer only on a read-write
  conflict with the activation row, so a writer that never re-reads is not caught, and it adds
  retries to every item operation. The advisory fence is narrower.
- **Drain or quiesce before activation** (#1746 P1 4177181872). Activation would wait until no
  reservation with a frozen V1 envelope remains. A decay retire reservation lives until its
  node-incarnation fence is reconciled, so this needs a gameplay stop and a proof that it is empty.
  Grandfathering the persisted tuple keeps exact-byte retry and needs neither.
- **A table lock on the outbox.** `LOCK TABLE ... SHARE ROW EXCLUSIVE` in activation would
  also serialize, but it blocks every outbox insert, the ones of other event types included if
  any are added, and it holds a heavier lock than one advisory key.

## 4. Decision test

- **Must decide now:** YES. #1733's phase 2 had two open P1 findings, and GOLD-FEE-2's bank part
  stays closed until the switch.
- **Minimum sufficient:** one empty table, one fence key, one trigger, one transaction opener used by
  every type-2 transaction, and one migration with a registry line. No new event type, payload or verifier.
- **Superseding evidence:** a persistence review showing an insert path the fence does not cover;
  a different migration number from the control plane.
- **Deliberately not decided:** the deploy tooling that produces the evidence of §1.1; any later
  retention profile.
- **Harder later:** every new type-2 transaction, a new fee source included, must be opened with
  `begin_type2_transaction`. The tuple type and the coverage tests enforce this.
