# OTV2 WP3 — Architecture / Complexity Audit and Recovery Decision

- Date: 2026-09-12
- Revision: 2 — consolidated source/consumer/quantitative pass
- Repository: `Oteryn/Oteryn-Game`
- Scope: WP3 SQLx/TLS resource accounting plus the exact Child B consumer lifecycle needed to qualify it
- Canonical WP3 Issue: `#351`
- Canonical WP3 Draft PR: `#356`
- WP3 branch: `agent/sqlx-driver-budget-351`
- WP3 exact source head inspected: `fe7891989b1247012e32c89c10cff6a10bacb943`
- Child B / WP4 Draft PR inspected: `#335`
- Child B exact source head inspected: `834db1d7118d751e31287715d3eaac7780a0c7b9`
- Protected `main` at final Revision-2 readback: `489e3e390a1bce1ce3439c66521ab75f8a826cd8`
- Previous full audit revision: branch head `aea03a69b5b6526eea4af1b413286f3ed2865ccd`, report blob `9b156703d9c06aa9c9f5c281ef9b669d2b201cdb`
- Evidence class: retained architecture audit evidence
- Architecture authority: **none** — this file does not itself accept, activate, supersede or implement architecture

## 1. Executive verdict

```text
WP3 architectural health:
RED

#356 disposition:
OPEN / DRAFT / PRESERVE EVIDENCE / NEEDS_DECISION

Recommended next architecture action:
WP3_V2_SUPERSEDING_DECISION

Implementation hold:
HOLD NEW BROAD RUSTLS/TOKIO OWNERSHIP EXPANSION

Confidence:
HIGH
```

The deeper audit confirms the original `WP3_BOUNDARY_REDESIGN_REQUIRED` conclusion and makes it more specific.

The core defect is no longer best described as “some allocations are still uncharged.” The stronger finding is that the current operation-owned connection model does not match the real production consumer lifecycle. Child B owns a shared `RuntimeBackend` and `PgPool`; pool bootstrap, TLS/connect work, idle connection residency, pool control state, reactor state and connection caches can exist before or after any individual active B operation.

At the same time, the actual Child B semantic workload is not yet routed through its own accepted eight-queued/two-active custody model, and its current database wait/retry behavior does not enforce the accepted `DFR-WAIT` contract. Therefore a perfectly instrumented dependency fork would still not make the composed product resource-safe.

The recommended direction is:

```text
one accepted DFR root ledger
    + finite executor/pool-shared reservations
    + bounded connection-resident reservations
    + bounded connect-transient phase reservation
    + exactly two active-slot budget views
    + one sealed B executor entrypoint
    + a much smaller SQLx seam
```

The broad rustls/Tokio custody work remains valuable source research, hostile-test evidence and bound derivation material. It should not automatically remain in the final product simply because it has already been written.

## 2. Live state and audit freshness

### 2.1 Protected repository state

At the final Revision-2 readback:

| Item | State |
|---|---|
| Protected branch | `main` |
| Protected `main` | `489e3e390a1bce1ce3439c66521ab75f8a826cd8` |
| Latest protected WP3 control-plane repair | PR #583, connect/socket owner correction |
| WP3 Issue | `#351` |
| WP3 PR | `#356` |
| WP3 state | OPEN / DRAFT / UNMERGED |
| WP3 exact head | `fe7891989b1247012e32c89c10cff6a10bacb943` |
| WP3 changed files | 928 |
| WP3 GitHub diff stats | `+257002 / -24` |
| Child B PR | `#335` |
| Child B exact head | `834db1d7118d751e31287715d3eaac7780a0c7b9` |
| Audit PR | `#588` |

The WP3 addition count is dominated by imported/patched vendored crates and must not be described as 257k lines of newly authored Oteryn production logic.

### 2.2 Exact-head CI evidence already established for #356

The prior audit verified exact-head GitHub Actions evidence for `fe7891989...`:

- Merge Gate `34699293933`: SUCCESS;
- Agent Governance `34699293903`: SUCCESS;
- Architecture Semantic Audit `34699293887`: SUCCESS;
- Linux workspace job `103568134958`: SUCCESS;
- Rust 1.94;
- PostgreSQL 17.6 service;
- build, strict workspace Clippy, workspace tests, Durability PostgreSQL E2E, synthetic harness and server smoke passed;
- `durability_postgres`: `125 passed; 0 failed`;
- `oteryn_resource_budget::owner_aware_aws_lc_tls_positive_and_denial_qualification`: PASS.

This is meaningful positive evidence for the exercised path. It is **not** proof that every accepted resource family, TLS mode, pool path, returned value, cancellation path or opaque runtime allocation is bounded.

### 2.3 Audit history preservation

Revision 2 intentionally consolidates the earlier 1300+ line report to reduce instruction/evidence duplication. The exact previous report remains immutable in Git history at branch head `aea03a69...` / blob `9b156703...`. No historical evidence is claimed deleted or retroactively invalidated.

## 3. Governing resource contract

The accepted authority remains:

`docs/architecture/reviews/OTERYN_GAME_DURABLE_FRESH_RESOURCE_ENVELOPE_DECISION_2026-09-06.md`

Relevant first-slice maxima are conjunctive hard ceilings:

| Resource | Accepted maximum |
|---|---:|
| `DFR-OPERATION-BYTES` | 65,536 B |
| `DFR-GUARD-BYTES` | 8,192 B |
| durable SQL row logical bytes | 131,072 B |
| variable-length SQL columns per durable row | 32 |
| returned SQL payload rows / pass | 32 |
| aggregate logical returned SQL payload / pass | 524,288 B |
| lock footprint | 64 logical keys / 16 relation classes |
| queue | 8 × 524,288 B = 4,194,304 B |
| active | 2 × 4,194,304 B = 8,388,608 B |
| completion | 131,072 B, inside active slot |
| pending checkpoint slots | 2 × 131,072 logical B |
| queue wait | 1,000 ms |
| one DB execution/reconciliation pass | 2,000 ms including lock waits |

The root queue+active ceiling is therefore:

```text
12 MiB per one logical Durability executor
```

Binding semantic rules include:

1. runtime/database-pool overhead retained on behalf of B is **not free**;
2. such overhead must be directly charged or covered by an explicit finite reservation inside the same root envelope;
3. if a dependency cannot supply a defensible finite bound, acceptance closes;
4. timeout never proves the backend stopped or rolled back and never releases outstanding custody;
5. there is no automatic retry loop inside one accepted DB pass;
6. the inherited 64 pending commands cannot be returned as 64 SQL rows — the accepted shape is one bounded ordered aggregate per exact attempt;
7. variable payload size must be checked in the same protected database snapshot before transferring oversized data into the driver.

The accepted provider-shared amendment additionally establishes an important precedent: root-shared residency can consume capacity from the same root and reduce what remains available to later queue/active work. The individual DFR maxima are ceilings, not a promise that every maximum is simultaneously fillable.

## 4. What WP3 actually has to prove

A terminal WP3/Child-B composition must prove all of the following, not merely a TLS handshake:

```text
root executor admission
  -> pool/bootstrap
  -> connection establishment or ready-idle selection
  -> optional connect/replacement transient peak
  -> active-slot admission
  -> pool checkout / liveness check
  -> transaction begin
  -> relation/key locks
  -> query write/flush
  -> PostgreSQL receive framing
  -> message decode / metadata
  -> returned row/value/error custody
  -> commit or ambiguity/reconciliation
  -> rollback/drain on cancellation/timeout
  -> connection cleanup
  -> return-to-pool or close
  -> no operation-owned backing remains
  -> active-slot release
```

The architecture must separately represent resources whose lifetime is longer than a single operation:

```text
executor runtime
pool control state
idle connections
connection caches/status
reactor registrations
shared TLS configuration/provider state
connection replacement/transient work
```

## 5. Current implementation shape

The #356 branch patches four upstream crates through workspace `[patch.crates-io]`:

- `vendor/sqlx-core-0.9.0`;
- `vendor/sqlx-postgres-0.9.0`;
- `vendor/tokio-1.53.1`;
- `vendor/rustls-0.23.43`.

It contains valuable primitives and evidence:

- `ResourceBudget` / `ResourceReservation` / `Charged<T>`;
- `BudgetOwner` / `BlockingJobOwner` experiments;
- owner-aware Tokio blocking queue/worker work;
- extensive rustls decoded/retained custody source research;
- SQLx PostgreSQL owner-aware direct connection/TLS plumbing;
- a real PostgreSQL 17.6 + TLS1.3 test harness;
- source-derived TLS phase-bound work.

But `PgConnection::establish_with_resource_budget()` explicitly describes itself as an **operation-owned connection without changing ordinary pool behavior**. Child B production composition uses ordinary `PgPool`. This is the central composition gap.

## 6. Original WP3 finding families — retained disposition

The earlier report's material findings remain valid unless explicitly corrected below.

### WP3-A01 — HIGH — rustls test/production owner representation differs

`PROVEN`.

Parts of rustls decoded custody are compiled differently under crate-local `cfg(test)`. Cross-crate production-compiled tests remain useful, but internal library tests do not prove the exact production owner representation.

Required disposition: one production/test semantics model, or explicit qualification proving why a test-only representation difference cannot invalidate the resource proof.

### WP3-A02 — HIGH — TLS verifier owner-aware scheme allocation is incomplete

`PROVEN / runtime reachability profile-dependent`.

SQLx verifier wrappers expose ordinary allocating `supported_verify_schemes()` paths while the owner-aware path fails closed unless a qualified seam exists. Every accepted SSL mode/profile must be tested under the exact final provider graph.

### WP3-A03 — HIGH — qualification provider differs from current consumer provider

`PROVEN`.

Workspace SQLx currently selects `tls-rustls-ring-webpki`. The current owner-aware positive helper is AWS-LC-oriented, and the owner-aware handshake path rejects unsupported provider composition.

A green helper is not proof of the frozen production Game feature graph.

### WP3-A04 — HIGH — PostgreSQL decode/count/cache accounting incomplete

`PROVEN`.

Peer-controlled message/count paths still allocate without a complete accepted preallocation gate. `DataRow`, `RowDescription`, `ParameterStatus`, statement metadata and connection caches remain part of the closure problem.

### WP3-A05 — HIGH — returned values can outlive query/connection operations

`PROVEN`.

`PgRow`, `PgValue`, `Bytes` descendants and metadata `Arc`s can retain shared backing after the immediate query call. Operation custody must follow physical backing or cross an explicit bounded-copy boundary.

### WP3-A06 — HIGH — owned and ordinary/unowned TLS buffering coexist

`PROVEN`.

This is valid for upstream owner-free callers, but the accepted B profile must structurally prove that no reachable budgeted path silently falls into unowned storage.

### WP3-A07 — HIGH — runtime owner Arc finality required a separate protocol

`PROVEN`.

An accounting reservation stored inside the same `Arc`-controlled allocation it charges can release before the Arc control allocation itself is deallocated. Protected architecture therefore selected a controlled final-owner direction based on exact finality rather than strong-count snapshots.

### WP3-A08 — HIGH — native thread metadata finality remains unresolved

`PROVEN`.

`JoinHandle::join()` does not prove destruction of every cloneable `std::thread::Thread` metadata/name owner. Protected architecture correctly keeps this as an explicit blocker rather than silently releasing at join.

### WP3-A09 — HIGH — DNS and reactor registration are infrastructure boundaries

`PROVEN`.

Protected #583 correctly shows that `BufferedSocket` begins too late. TCP/UDS reactor registration and hostname-resolution work can occur before it and can outlive the SQLx wrapper.

Revision 2 changes the architectural question: these resources should not automatically be forced into per-operation ownership if a finite executor/pool-shared reservation is more truthful.

### WP3-A10 — HIGH — making buffers fallible changes cleanup semantics

`PROVEN`.

SQLx contains cleanup/control paths written under assumptions that small protocol-control writes cannot fail; rollback queuing even uses `expect`. Resource denial therefore needs one coherent failure/ambiguity contract, not isolated `Result` conversions.

### WP3-A11 — HIGH — PostgreSQL TLS harness target binding needs hardening

`PROVEN`.

The real PG/TLS test is valuable, but privileged test mutation must fail before connect unless the configured URL and isolated test instance are proven identical, and cleanup must survive partial failure.

### WP3-A12 — MEDIUM — a clean ledger does not prove hook completeness

`PROVEN / DERIVED`.

`used == 0` after drop proves exercised hooks released. An allocation site that never calls the hook remains invisible. Closure therefore requires source census plus forced-denial families plus an independent allocation/memory observation strategy.

## 7. Consumer-lifecycle findings — Child B / #335

### B-A01 — P1 — production pool/bootstrap exists before WorkCustody

`PROVEN`.

The exact `registered_backend()` ordering is:

```text
schema::connect_runtime(database_url)
    -> PgPoolOptions::connect(...)
DurabilityCustody::acquire(&pool)
    -> real DB transaction / lock / pending read
WorkCustody::new(&pending)
RuntimeBackend { pool, custody, work, pending }
```

Therefore TCP/TLS/reactor/socket/pool connection residency cannot be exclusively owned by a later active-operation slot.

### B-A02 — P1 — WorkCustody is not the real production executor yet

`PROVEN`.

`WorkCustody` contains fixed `queued[8]` and `active[2]`, but its own source states that it is not yet the executor's queue/active-operation byte budget.

Only checkpoint submission is currently routed through this custody layer. `AdmissionRuntime` explicitly states that existing semantic APIs are not yet routed through the queue.

Direct handles remain available for:

- reconnect journal;
- fresh-admission store;
- guard publication store.

Those stores/journals call `backend.begin()` directly.

Consequence: the accepted `8 -> 2` scheduler, timeout-custody rule and future byte ledger can currently be bypassed by normal semantic APIs.

Required architecture: one sealed production executor entrypoint. Raw persistence methods should be crate-private/internal or explicitly fixture-only once the real executor is composed.

### B-A03 — P1 — accepted 2-second DB-pass deadline is not enforced

`PROVEN`.

Child B defines a five-second `PgPoolOptions::acquire_timeout`. Main semantic operations then perform `backend.begin().await`, locks and multiple SQL statements without one propagated two-second absolute deadline.

No production `statement_timeout` / `lock_timeout` enforcement was found in the #335 diff.

This conflicts with the accepted `DFR-WAIT` rule: one DB execution/reconciliation pass is bounded by 2,000 ms including lock waits.

Required repair:

```text
one absolute pass deadline
  -> executor admission
  -> pool checkout
  -> DB locks
  -> every SQL await
  -> commit/reconcile
```

A server-side timeout derived from the remaining deadline is a candidate physical-cleanup aid, not authority to release custody at timeout.

### B-A04 — P1 — ordinary PgPool connect path contains automatic retries

`PROVEN`.

Pinned SQLx `PoolInner::connect()` retries `ConnectionRefused` and transient database errors with exponential backoff until the acquire deadline. The accepted DFR contract says **no automatic retry loop** inside a pass.

The architecture must choose one of these truthful shapes:

1. pool connection maintenance/retry is root-shared infrastructure outside any active pass, while an active operation fails fast when no ready connection exists; or
2. the bounded SQLx seam exposes a one-shot connect path for DFR-owned work.

Leaving ordinary retrying acquire under the active pass is not compatible with the accepted first-slice contract.

### B-A05 — P1 — pool checkout and return perform extra I/O

`PROVEN`.

Pinned SQLx defaults `test_before_acquire=true`; an idle checkout pings the database. `PoolConnection::return_to_pool()` also performs liveness/cleanup work and a ping before returning the connection idle.

Therefore the operation resource lifecycle is larger than `BEGIN -> business queries -> COMMIT`.

### B-A06 — P1 — transaction drop only queues rollback

`PROVEN`.

`sqlx_core::Transaction::Drop` invokes `start_rollback()` when the transaction remains open. PostgreSQL `start_rollback()` queues a rollback query and returns; it does not await physical rollback completion.

Therefore cancellation/timeout cannot release the active slot when the future is dropped. The executor must retain ownership of the connection and slot until rollback/drain/return/close or ambiguity reconciliation reaches a truthful finality point.

### B-A07 — P1 — pending-command query shape violates accepted SQL-result policy

`PROVEN`.

#335 still contains production `fetch_all()` paths over reconnect pending-command child rows and checks vector length only after materialization.

The accepted DFR shape is one bounded ordered aggregate per exact attempt, not 64 returned child rows.

### B-A08 — P1 — variable `record_json` reads bypass same-snapshot size guards

`PROVEN`.

Multiple production queries still select `state, record_json` directly from reconnect attempts without the already-used `CASE WHEN octet_length(...) <= ... THEN ... END` pattern.

Even a perfect driver fork cannot make this consumer policy compliant.

### B-A09 — P1 — raw `sqlx::Error` can escape the active resource lifetime

`PROVEN`.

`DurabilityError::Database(sqlx::Error)` retains and formats the driver error. PostgreSQL errors/notices contain variable peer-originated message/detail/hint/context/object fields.

Recommendation: normalize the full driver error **inside the active slot** into a small bounded Durability disposition, bounded SQLSTATE/correlation information and no unbounded server prose. Destroy the driver error before slot release.

### B-A10 — P2 — global relation locking serializes the first implementation

`PROVEN`.

Child B uses a conservative relation-level locking strategy across roughly the accepted relation-class ceiling and intentionally makes no broad concurrency claim.

This supports reducing the pool ceiling from four, but it does not by itself prove that a one-connection pool is safe for all ambiguity/cleanup/liveness cases.

## 8. PgPool source audit and corrected pool recommendation

### 8.1 Current Child B pool policy

`PROVEN`.

`schema::connect_runtime()` currently uses `max_connections(4)` and Child B configures `acquire_timeout=5s`; other SQLx pool options remain at upstream defaults.

Relevant SQLx defaults include:

- `min_connections = 0`;
- `idle_timeout = 10 minutes`;
- `max_lifetime = 30 minutes`;
- `test_before_acquire = true`;
- fair acquisition.

### 8.2 Correction to the earlier audit candidate

The earlier Revision-1 candidate:

```text
min_connections = 2
max_connections = 2
idle_timeout = None
max_lifetime = None
```

is **SUPERSEDED AS A RECOMMENDATION** by the deeper pool-source audit.

Reasons:

1. upstream explicitly warns that an infinite connection lifetime can allow database-side resources to accumulate;
2. `min_connections > 0` causes background maintenance/replacement work;
3. the maintenance path uses the same retrying connection loop;
4. background replacement itself therefore needs a root-shared budget/deadline policy;
5. disabling all churn without evidence is not a valid substitute for bounding churn.

### 8.3 Current supported conclusion

`DERIVED — HIGH CONFIDENCE`:

```text
max_connections <= 2
```

is a sufficient first-slice ceiling **after** every production semantic DB operation is routed through the sealed two-active-slot executor and no nested independent second connection exists inside one pass.

`2` is **not** proven to be the mathematical minimum. One connection may be semantically possible, but ambiguity cleanup, liveness and progress guarantees require separate evaluation.

The following remain `NEEDS_DECISION`:

```text
min_connections
idle_timeout
max_lifetime / explicit retirement policy
background maintenance policy
ready-connection replacement policy
```

### 8.4 Pool queue itself is fixed resident backing

`PROVEN`.

SQLx creates a Crossbeam `ArrayQueue` with capacity derived from `max_connections`; Crossbeam allocates its fixed slot backing at construction. Reducing the ceiling from 4 to 2 therefore reduces real fixed pool memory even before considering connection objects.

### 8.5 Replacement connect is part of the root peak

`PROVEN / DERIVED`.

The SQLx reaper closes an expired idle connection before invoking minimum-connection maintenance. Missing minimum connections are opened sequentially.

For a two-connection ceiling, define:

```text
I = fixed executor/runtime/pool shared resident
R = settled resident cost of one ready connection
T = total cost attributable to one connection while it is at connect peak,
    including the part that will become its settled resident state
Q = current queue charge
A = current active charge
```

Then the connection portion of root residency must be able to satisfy at least:

```text
steady:
I + 2R + Q + A <= 12 MiB

bootstrap / replacement:
I + max(2R, R + T) + Q + A <= 12 MiB
```

If the root cannot fund `T`, replacement must wait or fail closed. No extra connection budget may be invented.

This is an audit formula candidate, not an accepted architecture equation until the exact final pool policy is selected and the lifetimes are independently reviewed.

## 9. Connection-resident state audit

### 9.1 Hard source-visible socket-buffer lower bound

`PROVEN`.

Pinned SQLx `BufferedSocket` starts with:

```text
write buffer: 8 KiB
read spare capacity: 8 KiB
```

Therefore the heap lower bound is at least:

```text
16,384 B per live connection
32,768 B for two live connections
```

before TLS state, socket boxing, reactor registration, statement/status/type caches or allocator overhead.

This is a lower bound only, not `CONNECTION_RESIDENT`.

### 9.2 Statement cache is finite but should not be disabled blindly

`PROVEN`.

PostgreSQL statement-cache default capacity is 100. Persistent queries create named prepared statements. Cache eviction sends `Close::Statement`.

If the cache is set to zero while queries remain persistent, named statements can be prepared without being retained in the client LRU and therefore without the normal eviction/close lifecycle.

Current recommendation:

- keep a finite statement cache initially;
- inventory exact unique B SQL strings and metadata size;
- include the maximum reachable statement metadata in `R`;
- if later disabling the cache, also make all B queries non-persistent/unnamed or otherwise prove server statement cleanup.

### 9.3 Custom PostgreSQL type/table cache growth is likely unreachable in the first B profile

`DERIVED — HIGH CONFIDENCE, REQUIRES FINAL QUERY/TYPE INVENTORY`.

Pinned SQLx resolves built-in OIDs without filling custom-type caches. Runtime query execution uses `resolve_column_origin=false`, so generic table-column origin discovery is not required by ordinary B execution.

The inspected B query corpus uses built-in PostgreSQL types.

Recommended first-slice profile:

```text
built-in PostgreSQL types only
no generic by-name custom type resolution
no table-origin discovery requirement
```

Any future custom type use requires a separately bounded profile rather than silently widening the connection-resident map.

### 9.4 ParameterStatus is a connection-resident unbounded-growth surface

`PROVEN`.

`PgStream::recv()` decodes `ParameterStatus` into owned `String` name/value pairs. Except for `server_version`, entries are inserted into a connection-lifetime `BTreeMap<String, String>`.

A peer can therefore send repeated unique names/large values and grow retained connection state.

The inspected SQLx describe path uses only a tiny subset of status names for alternate-server detection (`crdb_version`, `mz_version`, `questdb_version`). B production runtime does not need a general arbitrary-status store.

Required v2 disposition: exact allowlist and aggregate bytes/count ceiling, or no retained generic status map in the B profile.

### 9.5 Fail-closed connection return policy

`RECOMMENDATION`.

If connection residual state after an operation cannot be proven to fit the accepted resident reservation:

```text
close connection
```

rather than returning it idle.

This can be implemented with the existing SQLx pool release hooks after the final profile is selected. It does not remove the need to account the connection while it exists.

## 10. PostgreSQL receive/decode audit

### 10.1 Peer frame length is still accepted before a budget/profile gate

`PROVEN`.

`PgStream::recv_unchecked()` reads PostgreSQL's peer-provided message length and requests that size from ordinary `BufferedSocket::try_read(expected_len)`.

The final minimal seam still requires a checked bound before buffer growth/reserve.

### 10.2 Huge-count/tiny-body DataRow is a concrete exploit family

`PROVEN`.

`DataRow::decode_body()` reads a peer `u16` column count and immediately executes:

```text
Vec::with_capacity(count)
```

before validating the whole body.

A DataRow containing 65,535 NULL columns requires only:

```text
2 + 65,535 * 4 = 262,142 B body
```

which is below the accepted 524,288 B aggregate logical SQL-result byte ceiling, yet causes a huge metadata vector allocation.

Therefore byte limits alone are insufficient.

This is exactly the original #351 hostile family: **huge-count / tiny-body**.

Required seam: count/query-shape admission before the metadata vector allocation. For B, the accepted durable-row contract already gives a maximum of 32 variable-length SQL columns; the exact message/query-profile count rule must be frozen and enforced before allocation.

### 10.3 RowDescription has the same count/name pattern

`PROVEN`.

`RowDescription::decode_body()` reads a peer `u16` count, reserves a field vector of that count and clones every field name into an owned `String`.

Final qualification needs both byte and count controls for the accepted query profile.

### 10.4 Returned row backing remains operation custody

`PROVEN`.

A `PgRow` holds `DataRow` storage plus statement metadata `Arc`; values can retain `Bytes` slices or make copies. Active-slot release must wait until all operation-retained rows/values are consumed/dropped or deliberately copied through another charged boundary.

## 11. TLS phase-bound quantitative evidence

### 11.1 Exact ring decode bound already exists in #356 research

`PROVEN FOR THE PINNED RING PROFILE ONLY`.

Pinned `handshake_decode_heap_bound()` derives:

```text
non-ECH decoded-heap bound = 2,557,169 B
ECH decoded-heap bound     = 3,276,650 B
selected bound             = 3,276,650 B
```

The function explicitly states that this is **only a phase term**. Configuration, input buffers, fragment spans, crypto, retained certificates and returned errors are separate.

### 11.2 Known retained-fragment overlap term

Existing WP3 source research records a retained fragment-span/high-water term of approximately:

```text
327,680 B
```

that can overlap decoded handshake state in the pinned research profile.

A concrete partial phase term is therefore:

```text
3,276,650 + 327,680 = 3,604,330 B
```

Compared with one accepted 4 MiB active slot:

```text
4,194,304 - 3,604,330 = 589,974 B
```

Only ~590 KiB would remain for every other simultaneous operation-owned component if this connect/TLS phase were funded entirely from one active slot.

This does **not** prove that the 4 MiB slot is impossible. It does prove that per-operation connection/TLS accounting is very tight and gives strong evidence for a separate root `CONNECT_TRANSIENT` reservation.

### 11.3 Provider mismatch blocks final numeric adoption

The source-derived bound above is compiled for the ring/webpki profile, while the currently qualified owner-aware handshake is AWS-LC-oriented.

Therefore no final `CONNECT_TRANSIENT_PEAK` number may be accepted until the production provider/profile is frozen.

## 12. Timeout, cancellation and cleanup finality

### 12.1 A submitter timeout is not executor cancellation

`PROVEN / CONTRACTUAL`.

DFR already says timeout never releases outstanding custody. The executor must own work independently from a caller future/handle.

### 12.2 Transaction drop is not rollback completion

`PROVEN`.

Dropping an open SQLx transaction only queues rollback. The next asynchronous use of the connection performs the actual protocol work.

### 12.3 Recommended executor cleanup shape

`RECOMMENDATION`.

```text
submitter
  -> enqueue bounded original
  -> may time out/cancel its wait

executor owns active slot
  -> explicit PoolConnection
  -> transaction borrowing that connection
  -> execute under one absolute pass deadline
  -> on success: commit/reconcile
  -> on timeout/error: rollback/drain/reconcile
  -> consume/drop operation rows/driver errors
  -> shrink/validate connection residual state
  -> await return-to-pool OR close
  -> publish bounded completion / disposition
  -> only then release active slot
```

Using an explicit `PoolConnection` rather than letting `Pool::begin()` hide ownership is the clearer finality representation because the executor can retain the connection after transaction commit/drop and control the cleanup boundary.

## 13. Production DB/TLS profile is not frozen

### 13.1 Current SSL mode is URL/default driven

`PROVEN`.

`PgConnectOptions` defaults to `Prefer`; Child B passes the database URL through the ordinary pool constructor and does not enforce a production SSL mode in `db.rs`.

`Prefer` can fall back to plaintext when TLS is unavailable or refused.

This audit does **not** declare a new TLS policy. It records that the final profile is missing.

### 13.2 Candidate bounded first-slice transport identity split

`RECOMMENDATION / REQUIRES SUPERSEDING ARCHITECTURE`.

Evaluate:

```text
connect_address = literal SocketAddr / IP endpoint
TLS server_name = separate hostname used for SNI and VerifyFull
CA/root material = preloaded bounded shared configuration
```

This preserves hostname certificate verification while removing runtime hostname resolution from the DB connect path.

Current SQLx `hostaddr` parsing does not preserve a full libpq-like independent `hostaddr + host` model, so a small SQLx seam may be needed.

This recommendation must not be implemented under the current protected #583 authority without a reviewed superseding decision because #583 correctly forbids simply changing accepted hostname semantics as a local workaround.

## 14. Dedicated Durability runtime option

`RECOMMENDATION / REQUIRES MEASUREMENT`.

The accepted topology requires an async Durability worker/PgPool but does not require sharing the gameplay Tokio runtime.

Candidate:

```text
one dedicated Durability executor thread
    -> Tokio current-thread runtime
    -> DB I/O only
    -> bounded PgPool
```

Benefits:

- reactor allocations become attributable to the Durability root rather than global gameplay I/O;
- unrelated network registrations cannot inflate the DB pending-release structure;
- no multi-worker scheduler is required for two I/O-bound active operations;
- literal address + preloaded TLS data can remove the generic blocking-owner path.

Unknowns:

- exact OS thread stack and runtime resident bytes;
- exact Tokio current-thread runtime allocation baseline;
- exact registration/pending-release bound under two DB connections;
- whether this is smaller/simpler than a narrow shared-runtime reactor seam.

No choice is accepted by this report.

## 15. Minimal SQLx seam — current best candidate

The deeper audit does **not** support keeping a broad “fork every private allocation owner” strategy by default.

A much smaller final product fork appears sufficient if phase reservations and the final profile are accepted.

### SQLx core seam candidates

- root/connection budget/profile plumbing usable by ordinary `PgPool`, not only direct `PgConnection` fixture;
- separate transport address from TLS server name if the bounded profile selects it;
- one-shot connect mode or a composition that prevents retry loops from occurring inside an active DFR pass;
- prospective `BufferedSocket` read/write growth checks;
- exact cleanup/high-water information only where upstream API is insufficient.

### SQLx PostgreSQL seam candidates

- validate backend message frame length before receive-buffer reserve;
- validate DataRow/RowDescription count/profile before `Vec::with_capacity`;
- bound/allowlist retained `ParameterStatus` state;
- prevent unbounded custom type/table cache growth in the B production profile;
- expose or enforce the connection-cleanup contract needed before idle return;
- preserve full PostgreSQL semantics inside the frozen profile.

### Prefer not to patch in the final product if avoidable

- generic Tokio DNS behavior;
- generic Tokio blocking worker internals;
- broad rustls private AST/container ownership;
- unrelated Tokio TCP/UDS behavior;
- dependency paths unreachable under the final DB profile.

## 16. Updated dependency-fork disposition

| Component | Revision-2 disposition | Reason |
|---|---|---|
| `ResourceBudget` | KEEP | Useful single root admission abstraction. |
| `ResourceReservation` | KEEP | Valid RAII/phase reservation primitive. |
| `Charged<T>` | KEEP where exact backing custody is needed | Correct drop-order direction. |
| SQLx PostgreSQL fork | REWORK / NARROW | Frame/count/status/cache/transport/cleanup seams remain real. |
| SQLx core fork | REWORK / NARROW | Pool/root/buffer/profile plumbing may remain necessary. |
| broad rustls custody fork | PRESERVE AS EVIDENCE; candidate removal | Valuable census/bound research; may be replaced by finite phase reservation. |
| Tokio blocking-owner fork | PRESERVE AS EVIDENCE; candidate removal | May become unreachable with literal address + preloaded TLS + dedicated runtime. |
| Tokio reactor fork | NEEDS_DECISION | Could be replaced by finite root-shared DB-runtime registration reservation. |
| runtime-owner finality research | PRESERVE | Important evidence if any owner-aware runtime path survives. |
| real PG17.6/TLS harness | KEEP / HARDEN | High-value end-to-end evidence. |
| hostile denial/source census | KEEP | Needed regardless of implementation strategy. |

## 17. Quantitative status of the four requested v2 bounds

The audit can now replace “completely unknown” with partial source-derived constraints, but none of the four is terminally accepted.

### 17.1 `EXECUTOR_RUNTIME_RESIDENT`

`UNKNOWN — PARTIALLY CONSTRAINED`.

Must include at minimum:

- final executor state/queue bookkeeping;
- pool control/semaphore/idle queue backing;
- selected Tokio runtime attribution;
- shared TLS config/provider reservation;
- any maintenance/reaper task retained state;
- fixed connection-slot metadata not already counted in `R`.

The pool's `ArrayQueue` is fixed-capacity and therefore a real source-derived resident term once the final `max_connections` is selected.

Dedicated-current-thread runtime measurement is still required before accepting `I`.

### 17.2 `CONNECTION_RESIDENT`

`UNKNOWN — HARD LOWER BOUND ESTABLISHED`.

Source-visible lower bound:

```text
>= 16,384 B / connection
```

from SQLx socket buffers alone.

The final `R` must additionally include:

- socket object/control backing;
- reactor registration attributable to the connection;
- settled TLS state;
- bounded statement metadata cache;
- bounded ParameterStatus state;
- any reachable built-in-type metadata/caches;
- allocator/control overhead required by the accepted accounting method.

### 17.3 `CONNECT_TRANSIENT_PEAK`

`UNKNOWN — LARGE SOURCE-DERIVED PARTIAL TERM ESTABLISHED`.

For the pinned ring decode research profile:

```text
handshake decoded heap:      3,276,650 B
known overlapping span term:   327,680 B
partial connect term:         3,604,330 B
```

This is not a complete `T`: socket construction overlap, TLS configuration, crypto/KX, retained certificate state, reactor work and other selected-profile terms still need composition.

### 17.4 `ACTIVE_SQL_PEAK`

`UNKNOWN — CONSUMER REPAIR REQUIRED BEFORE MEASUREMENT`.

The current B consumer does not yet route all semantic work through active slots, still has pending-command/result-shape violations and unbounded error/record reads. Measuring a final active peak before those repairs would measure the wrong architecture.

Known accepted ceilings still apply:

```text
4 MiB / active slot
2 active slots maximum
```

A checkpoint-specific partial copy path currently can retain at most two 65,536-byte operation-string copies (~131,072 B) plus metadata, but this is not the full production active bound.

## 18. Root-envelope feasibility test for v2

Once the final profile supplies exact values, the architecture should be rejected or accepted with a simple root inequality rather than another chain of micro-amendments.

Define:

```text
I = executor/runtime/pool shared resident
R = settled one-connection resident
T = total one-connection connect-peak attribution
Q = actual current queue charge
A = actual current active charge
```

For a two-connection ceiling and sequential replacement:

```text
steady:
I + 2R + Q + A <= 12 MiB

connect/replacement:
I + max(2R, R + T) + Q + A <= 12 MiB
```

The root may deny connection replacement or new admission when there is insufficient headroom. Availability loss is acceptable before accounting escape; hidden overcommit is not.

This formula deliberately does not create a new DFR maximum.

## 19. Required WP3-v2 / Child-B-v2 decision packet

One superseding decision should replace further micro-amendment growth and answer all of the following in one place.

### Resource ownership

1. Which cells are executor/root-shared?
2. Which cells are per-connection resident?
3. Which cells are connect-transient?
4. Which cells are active-operation charged?
5. What exact event releases/converts each reservation?

### Consumer executor

6. What is the single sealed production executor API?
7. How are all fresh/reconnect/guard operations routed through `8 -> 2` custody?
8. How is one 1s queue deadline enforced?
9. How is one 2s DB-pass deadline propagated?
10. How does submitter cancellation leave executor custody intact?

### Pool

11. Final `max_connections` ceiling.
12. `min_connections` policy.
13. finite connection retirement/lifetime policy.
14. idle policy.
15. background maintenance/retry policy.
16. whether an active pass may ever trigger a new connect.
17. cleanup/return/close finality.

### SQL profile

18. exact static query corpus or allowed query family;
19. built-in/custom type policy;
20. statement cache policy;
21. returned-row/count/byte policy;
22. ParameterStatus retention policy;
23. bounded error representation;
24. pending-command aggregate shape;
25. same-snapshot oversized-value rejection.

### Transport/TLS

26. provider/features;
27. TLS version(s);
28. SSL mode;
29. transport address form;
30. TLS server-name/hostname verification form;
31. CA/root/client-auth input form;
32. DNS policy;
33. session resumption policy;
34. certificate compression policy;
35. exact connect-transient bound formula.

### Runtime

36. dedicated versus shared Tokio runtime;
37. reactor registration reservation/finality;
38. whether any blocking-owner/thread-owner seam remains production-reachable.

### Supersession

39. which protected WP3 amendments remain active authority;
40. which are superseded but retained as evidence;
41. which #356 code/tests are salvaged;
42. which broad vendor patches are removed from the final product diff.

## 20. Required implementation sequence after a protected v2 decision

This is a recommended sequence only; the audit grants no implementation authority.

### V2-0 — freeze expansion

Hold new broad rustls/Tokio ownership work. Continue read-only census and bounded experiments only.

### V2-1 — repair Child B consumer substrate

- create the real root ledger;
- route every production semantic DB operation through one executor;
- seal/remove bypass entrypoints;
- implement 8 queued / 2 active byte custody;
- enforce one absolute DB-pass deadline;
- repair pending-command aggregate query;
- repair same-snapshot payload guards;
- normalize driver errors inside the slot.

### V2-2 — freeze exact production DB/TLS profile

Decide provider, TLS/SSL mode, address/name split, root input form, DNS, resumption, compression and runtime topology.

### V2-3 — derive/measure `I`, `R`, `T`, `ACTIVE_SQL_PEAK`

Do not invent a new root maximum. If the exact profile cannot fit the existing root, return to architecture.

### V2-4 — implement only the remaining minimal SQLx seams

Prefer upstream rustls/Tokio behavior behind finite phase/root reservations where proof is sufficient.

### V2-5 — exact consumer qualification

Required minimum:

- exact production SQLx/rustls feature graph;
- real PostgreSQL 17.6;
- real hostname-verifying TLS;
- real `RuntimeBackend/PgPool` path;
- bootstrap before work admission;
- two active slots;
- queue 8 and max+1 rejection;
- 1s queue timeout;
- 2s pass deadline including lock waits;
- no active-pass automatic retry;
- connection unavailable/replacement behavior;
- cancellation with retained executor custody;
- rollback/drain finality;
- ambiguous/lost COMMIT reconciliation;
- pending-command bounded aggregate;
- max/max+1 returned row/count/bytes;
- huge-count/tiny-body protocol denial;
- oversized `record_json` denial before transfer;
- bounded ParameterStatus behavior;
- retained row/value/error lifetime;
- return-to-pool vs forced close behavior;
- hostile frame/error/status vectors;
- root inequality at worst accepted concurrency;
- independent memory/allocation observation;
- strict Rust 1.94 fmt/check/Clippy/tests;
- supply-chain/provenance checks;
- genuinely independent exact-head HIGH-risk review;
- canonical CI;
- FULL Merge Queue;
- protected-main readback.

## 21. What must not be inferred from this audit

This report does **not** prove or authorize any of the following:

- that `max_connections=2` is the mathematical minimum;
- that `min_connections=2` is correct;
- that infinite connection lifetime is acceptable;
- that a dedicated Tokio runtime is required;
- that production must use ring or AWS-LC;
- that production must use VerifyFull, although a hostname-verifying final profile needs to be explicitly decided;
- that production may discard hostname semantics merely to avoid DNS;
- that the 3.604 MB partial ring term is the full TLS/connect bound;
- that 4 MiB active slots must be enlarged;
- that broad rustls/Tokio forks can already be deleted;
- that #335 is authorized to mutate before its normal dependency/custody gates;
- that #356 can be closed before the evidence needed by the superseding decision is retained;
- that a green helper or CI run proves terminal WP3 readiness.

## 22. Integration impact

### WP2

Do not restart already-protected WP2 work. Preserve its semantic/replay/nonreuse guarantees.

### WP4 / Child B

WP4 remains owner of durable atomicity/reconciliation policy, but the real executor/resource substrate must be repaired before WP3 can be qualified as a production dependency.

### WP5

Source-owner/bootstrap readiness remains separate. Resource-accounting completion does not manufacture missing producers.

### G0 / Server Seam

No release follows from this audit alone. G0 and Server Seam require the composed exact state after terminal WP3, repaired/qualified WP4 and required WP5 composition.

## 23. Current unknowns that still block terminal architecture

`UNKNOWN` — exact `EXECUTOR_RUNTIME_RESIDENT` (`I`).

`UNKNOWN` — exact final per-connection resident bound (`R`).

`UNKNOWN` — exact final connect peak (`T`) after provider/profile selection.

`UNKNOWN` — exact active SQL peak after the real executor and query-shape repairs.

`UNKNOWN` — shared-vs-dedicated Tokio runtime comparison measured on the intended topology.

`UNKNOWN` — final production DB endpoint/DNS requirements.

`UNKNOWN` — final statement-cache size after exact unique SQL inventory.

`UNKNOWN` — exact status/type metadata allowance for the final profile.

No unknown above authorizes a guessed cap, silent TLS weakening or an additional independent resource budget.

## 24. Final go/no-go

### Current decision

**NO-GO for integrating #356 as terminal WP3 architecture in its audited state.**

**GO for preserving #356 as the canonical research/evidence lineage while a single superseding WP3-v2 architecture decision is prepared and reviewed.**

**NO-GO for further broad per-allocation rustls/Tokio ownership expansion until that decision answers the root/pool/active boundary.**

### Strongest blockers, ordered

1. real Child B workload bypasses its own accepted executor/custody model;
2. accepted 2-second DB pass deadline is not enforced;
3. ordinary PgPool acquire/connect contains automatic retry behavior forbidden by the DFR pass contract;
4. pool/bootstrap/idle residency exists outside an active operation;
5. PostgreSQL receive frame/count and ParameterStatus retained growth remain unbounded in the real consumer profile;
6. pending-command and variable-record query shapes still violate accepted DFR transfer rules;
7. cancellation/drop does not physically finish rollback/connection cleanup;
8. production TLS/provider/SSL-mode profile is not frozen and does not match the current owner-aware helper profile;
9. exact `I/R/T/ACTIVE_SQL_PEAK` values are not yet proven;
10. broad dependency-owner expansion has worse maintenance cost than the now-evidenced root/phase-reservation alternative.

### Positive work to preserve

- `ResourceBudget` / reservation primitives;
- exact source census and provenance;
- hostile denial vectors;
- real PostgreSQL 17.6/TLS harness;
- provider/native finite-bound work;
- handshake phase-bound research;
- sound backing-finality repairs;
- corrected socket/reactor/DNS source decomposition;
- repository governance and exact-head qualification discipline.

### Final markers

```text
WP3_V2_BOUNDARY_DECISION_REQUIRED
CHILD_B_DFR_EXECUTOR_REPAIR_REQUIRED
#356 = OPEN / DRAFT / PRESERVE EVIDENCE / NEEDS_DECISION
#335 = NOT READY FOR FINAL WP3/WP4 COMPOSED QUALIFICATION
```

This report is evidence and recommendation only. Protected repository architecture, live #162/#351/#329 authority and normal review/CI/Merge Queue controls remain binding until a separately reviewed/protected decision changes them.
