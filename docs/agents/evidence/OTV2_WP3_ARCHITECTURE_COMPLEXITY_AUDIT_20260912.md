# OTV2 WP3 — Architecture / Complexity Audit and Recovery Decision

- Date: 2026-09-12
- Revision: 3 — consolidated source, consumer, quantitative and concurrent-review pass
- Repository: `Oteryn/Oteryn-Game`
- Scope: WP3 SQLx/TLS resource accounting plus the exact Child B consumer lifecycle needed to qualify it
- Canonical WP3 Issue: `#351`
- Canonical WP3 Draft PR: `#356`
- WP3 branch: `agent/sqlx-driver-budget-351`
- WP3 exact source head inspected: `fe7891989b1247012e32c89c10cff6a10bacb943`
- Child B / WP4 PR inspected: `#335` — open, non-draft at the final audit readback
- Child B exact source head inspected: `834db1d7118d751e31287715d3eaac7780a0c7b9`
- Protected `main` at final Revision-3 readback: `489e3e390a1bce1ce3439c66521ab75f8a826cd8`
- Previous full audit revisions remain immutable in Git history; Revision 3 consolidates them into this one evidence file
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

Child B composition:
DFR EXECUTOR / DEADLINE / QUERY-SHAPE REPAIR REQUIRED

Confidence in source-backed NO-GO:
HIGH

Completeness of executable replacement qualification:
NOT ESTABLISHED
```

The deeper audit confirms the original `WP3_BOUNDARY_REDESIGN_REQUIRED` conclusion and makes it more precise.

The problem is no longer best described as “some allocations are still uncharged.” The stronger finding is that the current operation-owned connection model does not match the real production consumer lifecycle. Child B owns a shared `RuntimeBackend` and `PgPool`; configuration, schema bootstrap, pool control state, TLS/connect work, idle connection residency, reactor state, connection caches and maintenance can exist before or after any individual active B operation.

At the same time, the actual Child B semantic workload is not yet routed through its own accepted eight-queued/two-active custody model, its current database wait/retry behavior does not enforce the accepted `DFR-WAIT` contract, and several SQL transfer shapes bypass the accepted resource policy.

The recommended direction remains promising but is **not yet a proven replacement**:

```text
one accepted DFR root ledger
    + finite executor/pool-shared reservations
    + bounded connection-resident reservations
    + bounded connect-transient phase reservation
    + exactly two active-slot budget views
    + one sealed B executor entrypoint
    + a much smaller SQLx seam
```

Round-3 source review adds important constraints to that proposal:

- eager/minimum-connection and maintenance paths can overlap; sequential loops do not prove one handshake at a time;
- awaited connection return is not necessarily pool quiescence;
- `after_release` occurs before the final return ping;
- `close_hard` can still perform asynchronous TLS shutdown work;
- schema compatibility inspection is itself an unbounded bootstrap transfer today;
- environment/pgpass/OS configuration input remains outside a simple “literal IP + preloaded cert” profile unless explicitly frozen;
- source-derived or measured phase peaks must prove all overlapping lifetimes and failure exits, not merely observe one sample maximum.

The broad rustls/Tokio custody work remains valuable source research, hostile-test evidence and bound derivation material. It should not automatically remain in the final product simply because it has already been written.

## 2. Live state and audit freshness

### 2.1 Protected repository state

At the final Revision-3 readback:

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
| Child B state | OPEN / NON-DRAFT / UNMERGED |
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

No new Rust/PostgreSQL/concurrency/performance execution is claimed by Revision 3. This pass is source/API/repository evidence analysis.

### 2.3 Audit history preservation

Revision 3 consolidates the earlier long-form audit and the concurrent Round-3 supplement into this one file. Prior bytes remain in Git history. Consolidation is intended to reduce evidence fragmentation, not to erase historical findings.

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

The root queue+active ceiling is:

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

The accepted provider-shared amendment establishes an important precedent: root-shared residency can consume capacity from the same root and reduce what remains available to later queue/active work. The DFR maxima are ceilings, not a promise that every maximum is simultaneously fillable.

### 3.1 No automatic free allowance for shared infrastructure

`PROVEN / DERIVED`.

The nominal queue ceiling is 4 MiB and nominal two-active ceiling is 8 MiB, exactly 12 MiB total. Therefore if every queue/active ceiling were simultaneously fully reserved, any additional positive executor/pool/shared reservation would exceed the root.

This does **not** prove the architecture is impossible. It proves that the final v2 design needs a non-double-counted admission schedule using **actual current charges** and must be willing to deny new queue/active work when infrastructure residency has consumed root headroom.

A connection/runtime reservation is not a bonus budget.

## 4. What WP3 actually has to prove

A terminal WP3/Child-B composition must prove all of the following, not merely a TLS handshake:

```text
configuration / allowed ambient inputs
  -> pool/bootstrap control state
  -> eager connect and any maintenance connect overlap
  -> schema compatibility inspection
  -> executor custody takeover / pending restore
  -> queue and active-operation admission
  -> pool checkout / liveness check
  -> transaction begin
  -> relation/key locks
  -> query write/flush
  -> PostgreSQL receive framing
  -> message decode / metadata
  -> returned row/value/error custody
  -> commit or ambiguity/reconciliation
  -> rollback/drain on cancellation/timeout
  -> after_release decision
  -> final ping / protocol drain
  -> idle return OR asynchronous close
  -> root-funded minimum-connection repair if selected
  -> last result/error/cache/driver/runtime owner
  -> truthful debit release
```

The architecture must separately represent resources whose lifetime is longer than a single operation:

```text
executor runtime
pool control state
idle connections
connection caches/status
reactor registrations
shared TLS configuration/provider state
connection maintenance/replacement/transient work
provider process/thread-resident state
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

But `PgConnection::establish_with_resource_budget()` explicitly describes itself as an **operation-owned connection without changing ordinary pool behavior**. Child B composition uses ordinary `PgPool`. This remains the central WP3 composition gap.

## 6. Original WP3 finding families — retained disposition

### WP3-A01 — HIGH — rustls test/production owner representation differs

`PROVEN`.

Parts of rustls decoded custody are compiled differently under crate-local `cfg(test)`. Cross-crate production-compiled tests remain useful, but internal library tests do not prove the exact production owner representation.

### WP3-A02 — HIGH — TLS verifier owner-aware scheme allocation is incomplete

`PROVEN / runtime reachability profile-dependent`.

SQLx verifier wrappers expose ordinary allocating `supported_verify_schemes()` paths while the owner-aware path fails closed unless a qualified seam exists. Every accepted SSL mode/profile must be tested under the exact final provider graph.

### WP3-A03 — HIGH — qualification provider differs from current consumer provider

`PROVEN`.

Workspace SQLx currently selects a ring/webpki edge. The current owner-aware positive helper is AWS-LC-oriented, and the owner-aware handshake path rejects unsupported provider composition.

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

An accounting reservation stored inside the same `Arc`-controlled allocation it charges can release before the Arc control allocation itself is deallocated. Protected architecture therefore selected controlled final-owner semantics rather than strong-count snapshots.

### WP3-A08 — HIGH — native thread metadata finality remains unresolved

`PROVEN`.

`JoinHandle::join()` does not prove destruction of every cloneable `std::thread::Thread` metadata/name owner. Protected architecture correctly keeps this explicit rather than releasing at join.

### WP3-A09 — HIGH — DNS and reactor registration are infrastructure boundaries

`PROVEN`.

Protected #583 correctly shows that `BufferedSocket` begins too late. TCP/UDS reactor registration and hostname-resolution work can occur before it and can outlive the SQLx wrapper.

Revision 3 changes the architectural question: these resources should not automatically be forced into per-operation ownership if a finite executor/pool-shared reservation is more truthful.

### WP3-A10 — HIGH — making buffers fallible changes cleanup semantics

`PROVEN`.

SQLx contains cleanup/control paths written under assumptions that small protocol-control writes cannot fail; rollback queuing uses `expect`. Resource denial therefore needs one coherent failure/ambiguity contract, not isolated `Result` conversions.

### WP3-A11 — HIGH — PostgreSQL TLS harness target binding needs hardening

`PROVEN`.

The real PG/TLS test is valuable, but privileged test mutation must fail before connect unless the configured URL and isolated test instance are proven identical, and cleanup must survive partial failure.

### WP3-A12 — MEDIUM — a clean ledger does not prove hook completeness

`PROVEN / DERIVED`.

`used == 0` after drop proves exercised hooks released. An allocation site that never calls the hook remains invisible. Closure requires source census plus forced-denial families plus an independent allocation/memory observation strategy.

## 7. Consumer-lifecycle findings — Child B / #335

### B-A01 — P1 — production pool/bootstrap exists before WorkCustody

`PROVEN`.

The exact `registered_backend()` ordering is:

```text
schema::connect_runtime(database_url)
    -> PgPoolOptions::connect(...)
    -> schema compatibility inspect
DurabilityCustody::acquire(&pool)
    -> real DB transaction / lock / pending read
WorkCustody::new(&pending)
RuntimeBackend { pool, custody, work, pending }
```

Therefore configuration/TCP/TLS/reactor/socket/pool/bootstrap residency cannot be exclusively owned by a later active-operation slot.

### B-A02 — P1 — schema bootstrap transfers before validating the ledger

`PROVEN`.

`schema::inspect()` performs:

```text
SELECT version, checksum, success
FROM _sqlx_migrations
ORDER BY version ASC
```

with `fetch_all()`, then compares the returned count to the embedded migration set and materializes checksums as `Vec<u8>`.

This occurs during `connect_runtime()`, before `WorkCustody` exists.

Required closure: bound migration-ledger row count and checksum transfer before client materialization while preserving complete incompatibility detection and the no-runtime-DDL rule.

### B-A03 — P1 — WorkCustody is not the real production executor yet

`PROVEN`.

`WorkCustody` contains fixed `queued[8]` and `active[2]`, but its own source states that it is not yet the executor's complete queue/active-operation byte budget.

Only checkpoint submission is currently routed through this custody layer. `AdmissionRuntime` explicitly states that existing semantic APIs are not yet routed through the queue.

Direct handles remain available for reconnect, fresh admission and guard publication, and those paths call `backend.begin()` directly.

Required architecture: one sealed production executor entrypoint. Raw persistence methods should be internal/fixture-only once the executor is composed.

### B-A04 — P1 — accepted 2-second DB-pass deadline is not enforced

`PROVEN`.

Child B defines a five-second pool acquire timeout. Main semantic operations then perform `backend.begin().await`, locks and multiple SQL statements without one propagated two-second absolute deadline.

No production `statement_timeout` / `lock_timeout` enforcement was found in the #335 diff.

Required repair:

```text
one absolute pass deadline
  -> pool checkout
  -> DB locks
  -> every SQL await
  -> commit/reconcile
```

Server-side timeout derived from remaining time is a candidate cleanup aid, not permission to release custody at timeout.

### B-A05 — P1 — ordinary PgPool connect path contains automatic retries

`PROVEN`.

Pinned SQLx `PoolInner::connect()` retries `ConnectionRefused` and transient database errors with exponential backoff until the acquire deadline. The DFR contract says **no automatic retry loop** inside one pass.

Truthful choices are:

1. connection maintenance/retry is root-shared infrastructure outside the active pass and an active operation fails fast when no ready connection exists; or
2. the bounded SQLx seam exposes one-shot connect semantics for DFR-owned work.

### B-A06 — P1 — pool checkout and return perform extra I/O

`PROVEN`.

Pinned SQLx defaults `test_before_acquire=true`; idle checkout pings the database. Return processing also performs cleanup and a ping before idle release.

The operation resource lifecycle is larger than `BEGIN -> business queries -> COMMIT`.

### B-A07 — P1 — after_release is not the final resident-state observation point

`PROVEN`.

The internal return path invokes `after_release` before its final ping/protocol drain. `clear_cached_statements()` and `shrink_buffers()` also do not clear every retained type/table/status map.

Therefore an `after_release` memory check cannot alone prove the final idle resident bound.

### B-A08 — P1 — transaction drop only queues rollback

`PROVEN`.

Dropping an open SQLx transaction invokes `start_rollback()`, which queues rollback rather than awaiting physical completion.

Cancellation/timeout cannot release the active slot when the submitter future disappears. The executor must retain connection and slot custody until rollback/drain/return/close or ambiguity reconciliation reaches truthful finality.

### B-A09 — P1 — awaited return does not necessarily mean pool quiescence

`PROVEN / DERIVED`.

`PoolConnection::Drop` can spawn return/maintenance work, especially when `min_connections > 0`. Minimum-connection maintenance without a supplied deadline can use its own long internal deadline.

Operation-specific finality and root-funded background maintenance must therefore be distinct. An active slot must not be held for an unrelated 300-second maintenance allowance, and released operation budget must not fund later background repair.

### B-A10 — P1 — `close_hard` is not synonymous with immediate physical release

`PROVEN / source-backed`.

The PostgreSQL/TLS close path is asynchronous. Rustls shutdown can send close-notify and drive I/O before the underlying socket is fully shut down; readiness may remain pending.

The architecture needs an explicitly tested stalled-I/O close/cancel policy that retains accounting until the true resource owner is gone.

### B-A11 — P1 — pending-command query shape violates accepted SQL-result policy

`PROVEN`.

#335 still contains production `fetch_all()` paths over reconnect pending-command child rows and checks vector length after materialization.

The accepted DFR shape is one bounded ordered aggregate per exact attempt, not 64 returned child rows.

### B-A12 — P1 — variable `record_json` reads bypass same-snapshot size guards

`PROVEN`.

Multiple production queries still select variable `record_json` directly without the already-used guarded projection pattern.

Even a perfect driver fork cannot make this consumer policy compliant.

### B-A13 — P1 — raw `sqlx::Error` can escape the active resource lifetime

`PROVEN`.

`DurabilityError::Database(sqlx::Error)` retains and formats the driver error. PostgreSQL errors/notices contain variable peer-originated message/detail/hint/context/object fields.

Recommendation: normalize the driver error **inside the active slot** into a small bounded Durability disposition plus bounded SQLSTATE/correlation data, then destroy the full error before release.

### B-A14 — P1 — production custody does not yet prove repeated active-slot recovery

`PROVEN / INCOMPLETE`.

Current `WorkCustody` tests prove fixed queue/active admission and cancellation of queued work. Active custody is intentionally not cleared merely because a submitter cancels. The final definitive-disposition + owner-acknowledgement release layer is not yet the general production executor path.

Closure must include more than two sequential acknowledged operations in one process, two occupied ambiguous slots, same-slot reconciliation and capacity recovery without a third slot.

### B-A15 — P2 — global relation locking serializes the first implementation

`PROVEN`.

Child B uses a conservative relation-level locking strategy and intentionally makes no broad concurrency claim.

This supports reducing the pool ceiling from four, but does not prove that a one-connection pool is safe for all ambiguity/cleanup/liveness cases.

## 8. Configuration and credential-input boundary

### 8.1 URL size is not the whole configuration bound

`PROVEN`.

SQLx PostgreSQL options can consume ambient `PG*` environment values, OS username information and pgpass configuration in addition to the explicit URL.

Therefore a bounded URL plus literal transport address plus preloaded TLS material does **not** by itself prove finite configuration intake.

### 8.2 pgpass has unbounded line intake in the inspected path

`PROVEN`.

Pinned `pgpass.rs` uses `BufRead::read_line(&mut String)` without a project hard line-size bound.

Malformed lines are logged including the complete line.

Consequences:

- configuration-time memory remains unbounded by the URL limit;
- a malformed local credential record can be copied into diagnostics before later B-level error normalization.

No actual credential leak is claimed by this audit.

### 8.3 Required v2 decision

Freeze the production configuration source set:

```text
explicit bounded URL/struct only?
selected PG* environment fields?
pgpass allowed or disabled?
OS username fallback allowed?
credential/root paths or preloaded bytes?
```

Any enabled source needs a finite intake bound and redaction before library diagnostic sinks. Do not silently remove a required credential mechanism merely to simplify accounting.

## 9. PgPool source audit and corrected pool recommendation

### 9.1 Current Child B pool policy

`PROVEN`.

`schema::connect_runtime()` uses `max_connections(4)` and Child B configures `acquire_timeout=5s`; other SQLx pool options remain largely upstream defaults.

Relevant defaults include:

- `min_connections = 0`;
- finite idle timeout;
- finite max lifetime;
- `test_before_acquire = true`;
- fair acquisition.

### 9.2 Correction to the earlier audit candidate

The earlier candidate:

```text
min_connections = 2
max_connections = 2
idle_timeout = None
max_lifetime = None
```

is **SUPERSEDED AS A RECOMMENDATION**.

Reasons:

1. infinite connection lifetime can accumulate database-side resources;
2. `min_connections > 0` causes maintenance/replacement work;
3. maintenance uses the retrying connect machinery;
4. background replacement needs its own root reservation/deadline policy;
5. disabling all churn without evidence is not a valid substitute for bounding churn.

### 9.3 Current supported ceiling conclusion

`DERIVED — HIGH CONFIDENCE`:

```text
max_connections <= 2
```

is sufficient after every production semantic DB operation is routed through the sealed two-active-slot executor and no nested independent second connection exists inside one pass.

`2` is **not** proven to be the mathematical minimum.

The following remain `NEEDS_DECISION`:

```text
min_connections
idle_timeout
max_lifetime / explicit retirement policy
background maintenance policy
ready-connection replacement policy
whether active work may trigger a connect
```

### 9.4 Pool control state is real resident backing

`PROVEN`.

SQLx creates a fixed-capacity Crossbeam `ArrayQueue` sized from `max_connections`, plus semaphore/atomic/event/control state. Reducing the ceiling from 4 to 2 reduces real fixed pool backing.

### 9.5 Connection establishment is not globally serialized today

`PROVEN / DERIVED — HIGH CONFIDENCE`.

`PoolInner::new_arc()` starts maintenance tasks during pool construction. `PoolOptions::connect_with()` can separately invoke minimum-connection establishment. SQLx source itself notes the race when the reaper/maintenance path is active.

An individual `try_min_connections()` loop awaits connections sequentially, but multiple callers can overlap. Therefore **one loop being sequential does not prove one TLS handshake at a time**.

A two-connection pool can have up to two permitted connect attempts in flight unless v2 explicitly serializes connection establishment.

### 9.6 Corrected root connection-peak formula

Define:

```text
I = fixed executor/runtime/pool shared resident
R = settled resident cost of one ready connection
T = total cost attributable to one connection while at connect peak,
    including the resident portion it will retain
Q = actual current queue charge
A = actual current active charge
```

Without a new global connect serializer, a conservative two-connection connection component must admit:

```text
steady two ready:
2R

one ready + one connecting:
R + T

cold/two concurrent connects:
2T
```

Therefore the root feasibility condition is at least:

```text
I + max(2R, R + T, 2T) + Q + A <= 12 MiB
```

using **actual current** Q/A charges, not blindly substituting all nominal maxima.

If v2 intentionally serializes all connection establishment across eager/checkout/maintenance/replacement paths, the `2T` term may be narrowed only after that serialization is source-proven and qualified.

If the root cannot fund the next connect, replacement must wait/fail closed; no extra connection budget is invented.

## 10. Connection-resident state audit

### 10.1 Hard source-visible socket-buffer lower bound

`PROVEN`.

Pinned SQLx `BufferedSocket` starts with approximately:

```text
write buffer: 8 KiB
read spare capacity: 8 KiB
```

Source-visible lower bound:

```text
>= 16,384 B per live connection
>= 32,768 B for two live connections
```

before TLS state, socket boxing, reactor registration, statement/status/type caches or allocator overhead.

### 10.2 Statement cache is finite but should not be disabled blindly

`PROVEN`.

PostgreSQL statement-cache default capacity is finite (100). Persistent queries create named prepared statements and normal cache eviction sends `Close::Statement`.

Setting cache capacity to zero while keeping persistent queries can lose the normal client cache/eviction lifecycle for those named statements.

Recommendation:

- keep a finite cache initially;
- inventory exact unique B SQL strings and metadata;
- include maximum reachable statement metadata in `R`;
- if later disabling the cache, make B queries non-persistent/unnamed or separately prove statement cleanup.

### 10.3 Custom type/table cache growth is likely avoidable, not yet proven absent

`DERIVED — REQUIRES FINAL QUERY/TYPE/FEATURE INVENTORY`.

Pinned SQLx has fast paths for built-in OIDs and ordinary runtime execution does not need generic column-origin discovery. The inspected B query corpus uses built-in PostgreSQL types.

Candidate first-slice profile:

```text
built-in PostgreSQL types only
no generic by-name custom type resolution
no table-origin discovery requirement
```

But final reachability must be proven from the complete resolved production graph, not only one dependency edge or a source search.

### 10.4 ParameterStatus is a connection-resident growth surface

`PROVEN`.

`PgStream::recv()` decodes `ParameterStatus` into owned `String` name/value pairs and retains most entries in a connection-lifetime `BTreeMap<String, String>`.

Required v2 disposition: exact allowlist and aggregate bytes/count ceiling, or no retained generic status map in the B profile.

### 10.5 Cache/buffer cleanup is not complete residency proof

`PROVEN`.

`clear_cached_statements()` does not clear every retained PostgreSQL type/table map. `shrink_buffers()` addresses stream buffers, not all connection metadata. `after_release` is called before the final ping.

Therefore final connection residency must be established after all cleanup/protocol work, or `R` must conservatively include what survives it.

### 10.6 Fail-closed return policy

`RECOMMENDATION`.

If final accepted resident state cannot be established within `R`:

```text
close connection
```

rather than returning it idle.

Closing is itself asynchronous and must retain root/operation ownership until finality.

## 11. PostgreSQL receive/decode audit

### 11.1 Peer frame length is still accepted before a budget/profile gate

`PROVEN`.

`PgStream::recv_unchecked()` reads PostgreSQL's peer-provided message length and requests that size from ordinary `BufferedSocket::try_read(expected_len)`.

The minimal seam still requires a checked bound before buffer growth/reserve.

### 11.2 Huge-count/tiny-body DataRow is a concrete hostile family

`PROVEN`.

`DataRow::decode_body()` reads a peer `u16` column count and immediately executes `Vec::with_capacity(count)`.

A DataRow containing 65,535 NULL columns requires only:

```text
2 + 65,535 * 4 = 262,142 B body
```

which is below the accepted 524,288 B aggregate logical SQL-result byte ceiling, yet causes a huge metadata vector allocation.

Byte limits alone are insufficient. This is exactly the original #351 huge-count/tiny-body acceptance family.

### 11.3 RowDescription has the same count/name family

`PROVEN`.

`RowDescription::decode_body()` reads a peer count, reserves a field vector and copies field names into owned strings.

Final qualification needs both byte and count/query-profile controls.

### 11.4 Returned row backing remains operation custody

`PROVEN`.

A `PgRow` holds DataRow storage plus statement metadata; values can retain `Bytes` slices or allocate copies. Active-slot release waits for those descendants to be consumed/dropped or for an explicit charged copy boundary.

## 12. SQL result/query-shape consumer findings

### 12.1 Pending commands

`PROVEN`.

The accepted DFR decision requires one bounded ordered aggregate for the inherited 64 pending commands. Current #335 still has `fetch_all()` paths for child rows.

### 12.2 Variable durable records

`PROVEN`.

Some #335 reads already use good guarded projections. Other `record_json` reads do not.

Required rule: same protected SQL snapshot must validate logical byte limits before transferring variable payload into the driver.

### 12.3 Schema ledger

`PROVEN`.

The migration compatibility read itself uses unbounded `fetch_all` before WorkCustody. Its row count/checksum size must be bounded independently of the permanent admission-history limits.

### 12.4 Positive control

Not every `fetch_all()` is automatically a defect. For example, guarded fixed-cardinality reads that establish an exact upper count/byte transfer before materialization can satisfy the contract. Findings must remain query-specific.

## 13. TLS phase-bound quantitative evidence

### 13.1 Exact ring decode research bound

`PROVEN FOR THE PINNED RESEARCH PROFILE ONLY`.

Pinned `handshake_decode_heap_bound()` derives:

```text
non-ECH decoded-heap bound = 2,557,169 B
ECH decoded-heap bound     = 3,276,650 B
selected bound             = 3,276,650 B
```

The function explicitly states that this is **only a phase term**. Configuration, input buffers, fragment spans, crypto, retained certificates and returned errors are separate.

### 13.2 Known retained-fragment overlap term

Existing WP3 source research records approximately:

```text
327,680 B
```

of retained fragment-span/high-water state that can overlap decoded handshake state in the pinned research profile.

Partial term:

```text
3,276,650 + 327,680 = 3,604,330 B
```

Against one 4 MiB active slot:

```text
4,194,304 - 3,604,330 = 589,974 B
```

This does **not** prove that a 4 MiB slot is impossible. It strongly supports treating connect/TLS as root `CONNECT_TRANSIENT` rather than assuming abundant per-operation headroom.

### 13.3 Feature absence must be proven from the resolved graph

`CORRECTION / PROVEN PRINCIPLE`.

One dependency edge using `default-features = false` and omitting `brotli`/`zlib` does not, by itself, prove those features are absent in the complete Cargo build because features can be unified across edges/contexts.

Revision 3 therefore retracts any unconditional “certificate compression is production-unreachable” claim until the exact target's **resolved production feature graph** is recorded.

The final profile may still choose to disable compression/resumption, but absence must be proven from the resolved graph and runtime configuration.

### 13.4 Measurement is not a universal phase-bound proof

`PROVEN CONTRACTUAL REQUIREMENT`.

A defensible phase reservation must identify:

- admission point;
- every reachable allocation family;
- reallocation/conversion overlap;
- external descendants;
- concurrent re-entry/overlap;
- success exit/transfer;
- every failure/cancellation exit;
- exact finality event.

Independent measurement is corroboration/regression evidence. A sampled maximum alone is not an accepted universal bound.

## 14. Provider process/thread-shared residency

### 14.1 AWS-LC provider shared accounting remains valid evidence

`PROVEN FROM PROTECTED ARCHITECTURE`.

The protected provider-resident decision derives finite process and per-thread provider-shared reservation terms for its exact provider profile.

### 14.2 Thread first-use debt depends on thread cohort/lifetime

`PROVEN / DERIVED`.

The owner-aware AWS-LC model contains process-wide initialization plus a per-thread first-use reservation (1360 B in the protected profile). A lifetime sequence of fresh threads can therefore consume additional accounting even if only a small number of connections are live at one time.

This is an accounting/availability concern, not a claim that native memory leaks by exactly that amount.

A dedicated fixed Durability thread makes this easier to bound, but the final architecture must still prove the accepted crypto-thread cohort and provider lifetime.

## 15. Timeout, cancellation, return and close finality

### 15.1 Submitter timeout is not executor cancellation

`PROVEN / CONTRACTUAL`.

DFR already says timeout never releases outstanding custody. Work must be owned independently from a caller wait handle.

### 15.2 Transaction drop is not rollback completion

`PROVEN`.

Dropping an open SQLx transaction only queues rollback; later asynchronous connection work performs the protocol drain.

### 15.3 after_release is not final return state

`PROVEN`.

The final return ping occurs after the hook. A hook that observes “small enough” state has not automatically observed all later receive/status/error work.

### 15.4 awaited return is not global pool quiescence

`PROVEN / DERIVED`.

Wrapper Drop/minimum-connection maintenance can schedule root work after an explicit return path. Root-funded maintenance must be separated from operation-specific finality.

### 15.5 close_hard remains asynchronous

`PROVEN / source-backed`.

The TLS close path can drive close-notify/I/O before underlying shutdown. Stalled-peer cleanup needs a bounded tested policy; the name `close_hard` is not proof of instant deallocation.

### 15.6 Recommended executor cleanup shape

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
  -> establish operation-clean connection state or decide to close
  -> transfer any legitimate background maintenance to root ownership
  -> publish bounded completion/disposition
  -> only then release active slot
```

The active slot does not need to fund unrelated long-lived pool maintenance after all operation ownership has truthfully transferred to a pre-reserved root owner.

## 16. Production DB/TLS/configuration profile is not frozen

### 16.1 Current SSL mode is URL/default driven

`PROVEN`.

`PgConnectOptions` defaults to `Prefer`; Child B passes the database URL through ordinary pool construction and does not enforce a product SSL mode in `db.rs`.

`Prefer` can fall back to plaintext when TLS is unavailable/refused.

This audit records missing product policy; it does not invent one.

### 16.2 Candidate bounded transport identity split

`RECOMMENDATION / REQUIRES SUPERSEDING ARCHITECTURE`.

Evaluate:

```text
connect_address = literal SocketAddr / IP endpoint
TLS server_name = separate hostname used for SNI/hostname verification
CA/root material = preloaded bounded shared configuration
configuration sources = explicitly frozen/bounded
```

This can remove runtime DNS from the DB connect path without disabling hostname verification.

However the final profile must also decide environment/pgpass/OS-username behavior. Literal transport address alone does not eliminate configuration-time blocking/file/string allocations.

Current SQLx does not provide a complete libpq-like independent `hostaddr + host` seam, so a small SQLx change may be necessary.

No such policy may be implemented as a local workaround under #583 without a reviewed superseding architecture decision.

## 17. Dedicated Durability runtime option

`RECOMMENDATION / REQUIRES MEASUREMENT`.

Candidate:

```text
one dedicated Durability executor thread
    -> Tokio current-thread runtime
    -> DB I/O only
    -> bounded PgPool
```

Benefits:

- reactor allocations become attributable to Durability rather than global gameplay I/O;
- unrelated network registrations cannot inflate DB pending-release state;
- fixed thread cohort helps bound provider per-thread first-use residency;
- literal address + preloaded TLS data can remove the generic blocking-owner path;
- no multi-worker scheduler is required for two I/O-bound active operations.

Unknowns:

- exact OS thread stack/runtime resident bytes;
- exact Tokio current-thread runtime baseline;
- exact registration/pending-release bound under the selected pool;
- continuous driving/fairness behavior under the final workload;
- whether this is smaller/simpler than a narrow shared-runtime reactor seam.

## 18. Minimal SQLx seam — current best candidate

The deeper audit does **not** support keeping a broad “fork every private allocation owner” strategy by default.

### SQLx core candidates

- root/connection budget/profile plumbing usable by ordinary `PgPool`;
- separate transport address from TLS server name if selected;
- one-shot connect mode or a composition preventing retry loops inside active DFR passes;
- prospective `BufferedSocket` read/write growth checks;
- exact cleanup/high-water information only where upstream API is insufficient.

### SQLx PostgreSQL candidates

- validate backend frame length before receive-buffer reserve;
- validate DataRow/RowDescription count/profile before vector allocation;
- bound/allowlist retained `ParameterStatus` state;
- bound schema/configuration/bootstrap intake where SQL-side protection cannot do it;
- prevent unbounded custom type/table cache growth in the B profile;
- expose/enforce the cleanup contract needed before idle return;
- preserve full PostgreSQL semantics inside the frozen profile.

### Prefer not to patch if avoidable

- generic Tokio DNS behavior;
- generic Tokio blocking-worker internals;
- broad rustls private AST/container ownership;
- unrelated Tokio TCP/UDS behavior;
- dependency paths proven unreachable under the exact resolved production graph.

## 19. Updated dependency-fork disposition

| Component | Revision-3 disposition | Reason |
|---|---|---|
| `ResourceBudget` | KEEP | Useful single root admission abstraction. |
| `ResourceReservation` | KEEP | Valid RAII/phase reservation primitive with caller proof obligations. |
| `Charged<T>` | KEEP where exact backing custody is needed | Correct drop-order direction. |
| SQLx PostgreSQL fork | REWORK / NARROW | Frame/count/status/config/cache/transport/cleanup seams remain real. |
| SQLx core fork | REWORK / NARROW | Pool/root/buffer/profile plumbing may remain necessary. |
| broad rustls custody fork | PRESERVE AS EVIDENCE; candidate removal | Valuable census/bound research; may be replaced by finite phase reservation. |
| Tokio blocking-owner fork | PRESERVE AS EVIDENCE; candidate removal | May become unreachable under the final bounded profile/runtime. |
| Tokio reactor fork | NEEDS_DECISION | Could be replaced by finite root-shared DB-runtime registration reservation. |
| runtime-owner/thread-finality research | PRESERVE | Important if any owner-aware runtime path survives. |
| real PG17.6/TLS harness | KEEP / HARDEN | High-value end-to-end evidence. |
| hostile denial/source census | KEEP | Needed regardless of implementation strategy. |
| blanket phase bound based only on one measured peak | REJECT | Does not prove overlap/failure/re-entry/finality. |

No fork should be deleted merely because Revision 3 prefers a smaller architecture. Removal becomes safe only after the replacement consumer is qualified.

## 20. Quantitative status of the four requested v2 bounds

### 20.1 `EXECUTOR_RUNTIME_RESIDENT`

`UNKNOWN — PARTIALLY CONSTRAINED`.

Must include at minimum:

- final executor queue/active bookkeeping;
- pool control/semaphore/idle queue backing;
- selected Tokio runtime attribution;
- shared TLS/provider reservation;
- selected maintenance/reaper task retained state;
- fixed connection-slot metadata not already counted in `R`;
- bounded configuration/bootstrap retained state.

### 20.2 `CONNECTION_RESIDENT`

`UNKNOWN — HARD LOWER BOUND ESTABLISHED`.

Source-visible socket-buffer lower bound:

```text
>= 16,384 B / connection
```

Final `R` additionally includes socket/reactor/TLS settled state, bounded statement/status/type metadata and required control/allocator overhead.

### 20.3 `CONNECT_TRANSIENT_PEAK`

`UNKNOWN — LARGE SOURCE-DERIVED PARTIAL TERM ESTABLISHED`.

For the pinned ring research profile:

```text
handshake decoded heap:      3,276,650 B
known overlapping span term:   327,680 B
partial connect term:         3,604,330 B
```

This is not complete `T`. Final provider graph, configuration, socket/reactor, crypto/KX, retained certs and other overlap remain to be composed.

Because two connects can overlap under ordinary pool behavior, final root proof must either fund up to the selected concurrent connect count or source-prove a global connection-establishment serializer.

### 20.4 `ACTIVE_SQL_PEAK`

`UNKNOWN — CONSUMER REPAIR REQUIRED BEFORE MEASUREMENT`.

Current B still bypasses the real active executor for semantic operations and has unresolved query/error/timeout shapes. Measuring a final active peak now would measure the wrong composition.

Known accepted ceiling remains 4 MiB per active slot, two active slots maximum.

## 21. Root-envelope feasibility test for v2

Define:

```text
I = executor/runtime/pool shared resident
R = settled one-connection resident
T = total one-connection connect-peak attribution
Q = actual current queue charge
A = actual current active charge
```

Without global connect serialization, a two-connection ceiling requires at least:

```text
I + max(2R, R + T, 2T) + Q + A <= 12 MiB
```

The equation uses actual current queue/active charges. It does not assert that full nominal queue+active ceilings can coexist with positive infrastructure residency.

A valid design must publish an admission schedule and at least one funded successful witness at required semantic maxima without double counting.

Completion, ambiguity, cleanup and root-transfer state must already be funded before irreversible COMMIT; they are not post-commit bonus budgets.

## 22. Required WP3-v2 / Child-B-v2 decision packet

One superseding decision should replace further micro-amendment growth and answer these questions in one place.

### Resource ownership

1. executor/root-shared cells;
2. per-connection resident cells;
3. connect-transient cells;
4. active-operation cells;
5. exact release/transfer event for each reservation;
6. double-count prevention between root, connection and active views.

### Consumer executor

7. single sealed production executor API;
8. routing of all fresh/reconnect/guard work through `8 -> 2` custody;
9. one 1s queue deadline;
10. one 2s DB-pass deadline including lock waits;
11. submitter cancellation semantics;
12. definitive disposition + owner acknowledgement slot release;
13. repeated sequential capacity recovery.

### Bootstrap/configuration

14. bounded schema-ledger compatibility inspection;
15. environment input policy;
16. pgpass policy and line-size bound if enabled;
17. diagnostic redaction before library sinks;
18. bootstrap/custody resource owner before WorkCustody exists.

### Pool

19. final `max_connections` ceiling;
20. `min_connections` policy;
21. finite retirement/lifetime policy;
22. idle policy;
23. background maintenance/retry policy;
24. global connect concurrency admission;
25. whether an active pass may trigger a connect;
26. after_release/final-ping/idle-return boundary;
27. close/stalled-I/O finality.

### SQL profile

28. exact static query corpus or allowed family;
29. built-in/custom type policy;
30. statement cache policy;
31. returned row/count/byte policy;
32. ParameterStatus retention policy;
33. bounded error representation;
34. pending-command aggregate shape;
35. same-snapshot oversized-value rejection;
36. migration-ledger bound.

### Transport/TLS/runtime

37. resolved provider/features for the exact production target;
38. TLS version(s);
39. SSL mode;
40. transport address form;
41. TLS server-name/hostname verification form;
42. CA/root/client-auth input form;
43. DNS policy;
44. session resumption/compression policy;
45. exact connect-transient proof;
46. dedicated versus shared Tokio runtime;
47. reactor registration reservation/finality;
48. accepted provider process/thread cohort;
49. whether any blocking-owner/thread-owner seam remains reachable.

### Supersession

50. which protected WP3 amendments remain active authority;
51. which become historical evidence;
52. which #356 code/tests are salvaged;
53. which broad vendor patches are removed only after replacement qualification.

## 23. Required implementation sequence after a protected v2 decision

This audit grants no implementation authority.

### V2-0 — freeze expansion

Hold new broad rustls/Tokio ownership work. Continue read-only census/bounded experiments.

### V2-1 — repair Child B substrate

- real root ledger;
- one production executor;
- seal bypass entrypoints;
- 8 queued / 2 active byte custody;
- absolute queue/pass deadlines;
- no active-pass retry loop;
- bounded migration-ledger inspect;
- pending-command aggregate;
- same-snapshot payload guards;
- bounded errors;
- definitive acknowledgement/release lifecycle.

### V2-2 — freeze exact production configuration/DB/TLS/runtime profile

Decide configuration sources, provider/features, TLS/SSL mode, address/name split, credential/root input form, DNS, resumption/compression, pool policy and runtime topology.

### V2-3 — derive/measure `I`, `R`, `T`, `ACTIVE_SQL_PEAK`

Each phase bound must include all overlapping lifetimes, escaping descendants, re-entry and failure exits. Measurement corroborates source proof; it does not replace it.

### V2-4 — implement only remaining minimal SQLx seams

Prefer upstream rustls/Tokio behavior behind finite root/phase reservations where the proof is complete.

### V2-5 — exact consumer qualification

Minimum matrix:

- exact resolved production Cargo feature graph;
- real PostgreSQL 17.6;
- real hostname-verifying TLS according to the selected policy;
- real `RuntimeBackend/PgPool` path;
- bounded ambient configuration;
- bounded migration compatibility read;
- bootstrap before work admission;
- two active slots;
- queue 8/max+1;
- 1s queue timeout;
- 2s DB pass including lock waits;
- no active-pass automatic retry;
- concurrent connect admission according to selected pool policy;
- connection unavailable/replacement behavior;
- cancellation with retained executor custody;
- rollback/drain finality;
- stalled TLS close;
- ambiguous/lost COMMIT reconciliation;
- more than two sequential completed/acknowledged operations;
- both ambiguous slots occupied and reconciled;
- pending-command bounded aggregate;
- max/max+1 returned rows/count/bytes;
- huge-count/tiny-body protocol denial;
- oversized durable value denial before transfer;
- bounded ParameterStatus behavior;
- retained row/value/error lifetime;
- after_release + final-ping resident proof;
- return-to-pool vs forced close;
- hostile frame/error/status vectors;
- provider cold/warm/thread-cohort behavior;
- root inequality at worst permitted concurrency;
- independent allocation/release-order observation while runtime stays alive;
- strict Rust 1.94 fmt/check/Clippy/tests;
- affected vendored and ordinary-consumer profiles;
- supply-chain/provenance checks;
- genuinely independent exact-head HIGH-risk review;
- canonical CI;
- FULL Merge Queue;
- protected-main readback.

## 24. Verification refinements

The original audit tracked `WP3-Q01`–`WP3-Q46`. Revision 3 retains them and adds the following refinements from the concurrent source review. These are requirements, **not claims of executed new tests**.

| ID | Required observation |
|---|---|
| Q47 | Fund configuration, pool bootstrap and custody restoration before work admission. |
| Q48 | Bound migration-ledger count/checksum transfer before client materialization. |
| Q49 | Bound/authorize PG environment, passfile and username configuration inputs. |
| Q50 | Redact diagnostics before library sinks, including malformed credential records. |
| Q51 | Exercise concurrent eager/maintenance/checkout connect attempts under one admission bound. |
| Q52 | Observe maintenance after explicit return/wrapper Drop under selected pool policy. |
| Q53 | Establish post-ping residency, not only after_release state. |
| Q54 | Close during stalled TLS I/O while retaining truthful ownership. |
| Q55 | Verify all reachable retained connection maps, not only statements/buffers. |
| Q56 | Complete/acknowledge more than two sequential operations without runtime restart. |
| Q57 | Reconcile two occupied slots without a third slot/replacement identity. |
| Q58 | Classify pre-effect initialization failure versus uncertain takeover without blind reset. |
| Q59 | Exercise registered production B construction separately from `LegacyFixture`. |
| Q60 | Observe release order/reuse while runtime remains alive, then audit shutdown separately. |
| Q61 | Record resolved production features and prove excluded-family reachability claims. |
| Q62 | Bind layout/phase evidence to compiler, allocator, target, panic and optimization profile. |
| Q63 | Prove each phase entry, overlap, escaping descendants and failure exits. |
| Q64 | Show simultaneous root/sub-budget fit without double counting. |
| Q65 | Reserve completion/cleanup before COMMIT and retain ambiguity if delivery fails. |
| Q66 | Separate provider cold/warm initialization and lifetime crypto-thread churn. |
| Q67 | Charge old/new configuration/connection overlap during permitted rotation/failover. |
| Q68 | Prove runtime driving/scheduling under crypto/parser load for selected topology. |
| Q69 | Run affected vendored and ordinary-consumer tests in exact supported profiles. |
| Q70 | Retain source identities, test names/skips, independent observations and cleanup outcomes. |

## 25. What must not be inferred

This report does **not** prove or authorize:

- that `max_connections=2` is the mathematical minimum;
- that `min_connections=2` is correct;
- that one connect is always in flight;
- that infinite connection lifetime is acceptable;
- that a dedicated Tokio runtime is required;
- that production must use ring or AWS-LC;
- that compression/resumption are absent until the resolved production graph/config proves it;
- that production must use a specific SSL mode without an owning decision;
- that production may discard hostname semantics to avoid DNS;
- that literal IP + preloaded certs remove all configuration I/O;
- that `after_release` or an awaited return is final pool quiescence;
- that `close_hard` means immediate deallocation;
- that the 3.604 MB partial ring term is the full connect bound;
- that 4 MiB active slots must be enlarged;
- that broad rustls/Tokio forks can already be deleted;
- that a measured peak without source/lifetime proof is an accepted bound;
- that #335 is authorized to mutate before normal dependency/custody gates;
- that #356 can be closed before superseding-decision evidence is retained;
- that green helper/CI evidence proves terminal WP3 readiness.

## 26. Integration impact

### WP2

Do not restart already-protected WP2 work. Preserve its semantic/replay/nonreuse guarantees.

### WP4 / Child B

WP4 remains owner of durable atomicity/reconciliation policy. The real executor/resource/bootstrap/query substrate must be repaired before WP3 can be qualified as a production dependency.

### WP5

Source-owner/bootstrap readiness remains separate. Resource accounting does not manufacture missing producers.

### G0 / Server Seam

No release follows from this audit. G0 and Server Seam require the composed exact state after terminal WP3, repaired/qualified WP4 and required WP5 composition.

## 27. Current unknowns that still block terminal architecture

`UNKNOWN` — exact `EXECUTOR_RUNTIME_RESIDENT` (`I`).

`UNKNOWN` — exact final per-connection resident bound (`R`).

`UNKNOWN` — exact final connect peak (`T`) after provider/config/profile selection.

`UNKNOWN` — exact active SQL peak after the real executor/query-shape repairs.

`UNKNOWN` — exact permitted concurrent-connect count if v2 does not add a serializer.

`UNKNOWN` — shared-vs-dedicated Tokio runtime comparison on intended topology.

`UNKNOWN` — final production DB endpoint/DNS/configuration requirements.

`UNKNOWN` — final statement-cache size after exact SQL inventory.

`UNKNOWN` — exact status/type metadata allowance.

`UNKNOWN` — accepted provider thread cohort/lifetime.

No unknown above authorizes a guessed cap, silent TLS weakening or an additional independent budget.

## 28. Final go/no-go

### Current decision

**NO-GO for integrating #356 as terminal WP3 architecture in its audited state.**

**GO for preserving #356 as the canonical research/evidence lineage while one superseding WP3-v2 architecture decision is prepared and reviewed.**

**NO-GO for further broad per-allocation rustls/Tokio ownership expansion until that decision answers the root/pool/configuration/active boundary.**

### Strongest blockers, ordered

1. real Child B workload bypasses its own accepted executor/custody model;
2. accepted 2-second DB-pass deadline is not enforced;
3. ordinary PgPool acquire/connect contains automatic retry behavior forbidden inside the DFR pass;
4. pool/bootstrap/schema/configuration work exists outside active operations;
5. SQLx pool establishment can overlap through independent eager/maintenance entrypoints unless explicitly serialized/funded;
6. PostgreSQL receive frame/count and ParameterStatus retained growth remain unbounded in the real profile;
7. pending-command, record and schema-ledger query shapes still contain pre-transfer gaps;
8. cancellation/drop does not physically finish rollback/cleanup, and after_release/return are not complete finality boundaries;
9. ambient configuration/pgpass intake is not bounded by the URL limit;
10. production TLS/provider/feature/SSL-mode profile is not frozen and current helper differs from the consumer graph;
11. exact `I/R/T/ACTIVE_SQL_PEAK` and joint root admission schedule are not proven;
12. broad dependency-owner expansion has worse maintenance risk than the now-evidenced root/phase-reservation alternative, but that alternative still requires full qualification.

### Positive work to preserve

- `ResourceBudget` / reservation primitives;
- exact source census/provenance;
- hostile denial vectors;
- real PostgreSQL 17.6/TLS harness;
- provider/native finite-bound research;
- handshake phase-bound research;
- sound backing-finality repairs;
- corrected socket/reactor/DNS decomposition;
- guarded two-slot pending reload where already correctly bounded;
- repository governance/exact-head qualification discipline.

### Final markers

```text
WP3_V2_BOUNDARY_DECISION_REQUIRED
CHILD_B_DFR_EXECUTOR_REPAIR_REQUIRED
CONFIGURATION_BOOTSTRAP_BOUND_REQUIRED
#356 = OPEN / DRAFT / PRESERVE EVIDENCE / NEEDS_DECISION
#335 = OPEN / NON-DRAFT / NOT READY FOR FINAL COMPOSED QUALIFICATION
```

This report is evidence and recommendation only. Protected repository architecture, live #162/#351/#329 authority and normal review/CI/Merge Queue controls remain binding until a separately reviewed/protected decision changes them.
