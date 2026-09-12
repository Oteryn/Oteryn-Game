# OTV2 WP3 — Architecture / Complexity Audit and Recovery Decision

- Date: 2026-09-12
- Revision: 4 — consolidated source, consumer, quantitative, pool-topology and concurrent-review pass
- Repository: `Oteryn/Oteryn-Game`
- Scope: WP3 SQLx/TLS resource accounting plus exact Child B consumer/bootstrap/pool lifecycle required to qualify it
- Canonical WP3 Issue: `#351`
- Canonical WP3 Draft PR: `#356`
- WP3 exact head inspected: `fe7891989b1247012e32c89c10cff6a10bacb943`
- Child B / WP4 PR: `#335`
- Child B exact head inspected: `834db1d7118d751e31287715d3eaac7780a0c7b9`
- Protected `main` at Revision-4 authoring readback: `489e3e390a1bce1ce3439c66521ab75f8a826cd8`
- Audit PR: `#588`
- Prior Revision-3 blob: `763e357bb234bc49ccd3d02b22ee928af5496879`
- Evidence class: retained architecture/source audit evidence
- Architecture authority: **none** — this file does not itself accept, activate, supersede or implement architecture

## 1. Executive verdict

```text
WP3 architectural health:
RED

#356:
OPEN / DRAFT / PRESERVE EVIDENCE / NEEDS_DECISION

#335:
OPEN / NON-DRAFT / NOT READY FOR FINAL COMPOSED QUALIFICATION

Recommended next architecture action:
WP3_V2_SUPERSEDING_DECISION

Implementation hold:
HOLD NEW BROAD RUSTLS/TOKIO OWNERSHIP EXPANSION

Highest-confidence first-slice simplification candidate:
ONE PHYSICAL PG CONNECTION + TWO LOGICAL ACTIVE CUSTODY SLOTS

Confidence in source-backed NO-GO:
HIGH

Replacement architecture terminally qualified:
NO
```

The audit no longer treats WP3 as merely a collection of missing reservations. The stronger result is that the current operation-owned connection model does not match the real consumer lifecycle.

Child B owns one process-shared `RuntimeBackend` and `PgPool`. Configuration intake, connection establishment, schema compatibility inspection, custody takeover, idle connection state, reactor state, provider state and pool maintenance can exist outside an individual active B operation. At the same time, the real B semantic APIs are not yet routed through the accepted eight-queued/two-active custody model and do not enforce the accepted 2-second DB-pass deadline.

The current best recovery direction is therefore:

```text
one accepted 12 MiB DFR root ledger
│
├── fixed executor/runtime/pool shared reservation
├── at most one settled production PG connection in the first slice
├── one root-owned connect/reconnect transient reservation
├── two logical active custody slots
├── eight queued custody slots
└── a narrow SQLx seam only where SQL/query shape cannot close the boundary
```

The broad rustls/Tokio custody work remains valuable source research, hostile-test evidence and bound-derivation material. It should not automatically remain in the final product because the work already exists.

## 2. Live state and retained evidence

At the Revision-4 authoring readback:

| Item | State |
|---|---|
| protected `main` | `489e3e390a1bce1ce3439c66521ab75f8a826cd8` |
| WP3 Issue | `#351` |
| WP3 PR | `#356`, OPEN / DRAFT |
| WP3 head | `fe7891989b1247012e32c89c10cff6a10bacb943` |
| WP3 changed files | 928 |
| WP3 diff stats | `+257002 / -24` |
| Child B PR | `#335`, OPEN / NON-DRAFT |
| Child B head | `834db1d7118d751e31287715d3eaac7780a0c7b9` |
| audit PR | `#588` |

The WP3 addition count is dominated by imported/patched vendored crates and must not be described as 257k lines of newly authored Oteryn product code.

Exact-head #356 evidence already retained by the audit includes successful repository-native Merge Gate, Agent Governance and Architecture Semantic Audit, Rust 1.94, PostgreSQL 17.6, workspace build/tests/strict Clippy, Durability PostgreSQL E2E and one owner-aware AWS-LC TLS positive/denial helper. That evidence is valuable for the exercised path; it is not complete production resource proof.

The prior Revision-3 audit head `8e818b7701431957bf97fcf14a178f03c74ae232` also reached SUCCESS on its exact-head Agent Governance, Architecture Semantic Audit and Merge Gate. Those checks qualify the evidence document only, not #356/#335 architecture.

Earlier audit/addendum/Round-3 bytes remain immutable in Git history. Revision 4 is the current consolidated narrative.

## 3. Governing DFR contract

Authority remains:

`docs/architecture/reviews/OTERYN_GAME_DURABLE_FRESH_RESOURCE_ENVELOPE_DECISION_2026-09-06.md`

Key first-slice maxima:

| Resource | Accepted maximum |
|---|---:|
| operation bytes | 65,536 B |
| guard bytes | 8,192 B |
| durable row logical bytes | 131,072 B |
| variable-length SQL columns / durable row | 32 |
| returned SQL payload rows / pass | 32 |
| aggregate logical returned SQL bytes / pass | 524,288 B |
| lock footprint | 64 logical keys / 16 relation classes |
| queue | 8 × 524,288 B = 4,194,304 B |
| active | 2 × 4,194,304 B = 8,388,608 B |
| completion | 131,072 B inside active slot |
| pending checkpoint | 2 × 131,072 logical B |
| queue wait | 1,000 ms |
| one DB execution/reconciliation pass | 2,000 ms including lock waits |

The nominal queue+active ceiling is exactly 12 MiB per logical Durability executor.

Binding rules include:

1. runtime/database-pool overhead retained for B is not free;
2. dependency/runtime residency must be directly charged or covered by an explicit finite reservation inside the same root;
3. inability to derive a defensible bound closes acceptance;
4. timeout never proves backend death/rollback and never releases custody;
5. no automatic retry loop is accepted inside one DB pass;
6. 64 pending commands must be returned as one bounded ordered aggregate, not 64 SQL rows;
7. variable payload limits must be checked in the same protected database snapshot before oversized transfer;
8. pool topology was deliberately **not** frozen by the DFR decision.

The accepted provider-shared precedent also proves that shared residency may consume root capacity and reduce later free queue/active capacity. The maxima are ceilings, not a promise that every ceiling is simultaneously fillable.

## 4. Fundamental composition mismatch

### 4.1 #356 proves an operation-owned direct connection, not the real pool lifecycle

`PROVEN`.

The #356 branch exposes an owner-aware direct `PgConnection` establishment route and explicitly avoids changing ordinary pool behavior.

Child B production composition uses one shared `RuntimeBackend` / `PgPool`.

Therefore direct owner-aware connection qualification is a source experiment/fixture, not sufficient proof for the final production consumer.

### 4.2 Pool/bootstrap work exists before active operation custody

`PROVEN`.

Current registered backend construction is structurally:

```text
schema::connect_runtime(database_url)
    -> PgPoolOptions::connect(...)
    -> schema compatibility inspection
DurabilityCustody::acquire(&pool)
    -> transaction / exclusive custody lock / relation locks / pending reload
WorkCustody::new(&pending)
RuntimeBackend { pool, custody, work, pending }
```

Configuration/TCP/TLS/reactor/socket/schema/custody residency therefore cannot be exclusively owned by a later active-operation slot.

### 4.3 Real semantic APIs still bypass WorkCustody

`PROVEN`.

`WorkCustody` has fixed `queued[8]` and `active[2]` and explicitly says it is not yet the executor's complete queue/active byte budget.

Only the checkpoint prototype is routed through it. Reconnect, fresh-admission and guard APIs still call `backend.begin()` directly.

The production architecture therefore still needs one sealed executor entrypoint that all semantic persistence work must traverse.

## 5. Production custody/finality gaps in Child B

### 5.1 There is no active-slot clear path

`PROVEN`.

Current `WorkCustody` exposes enqueue, queued cancel and promote. There is deliberately no active clear on submitter cancellation — correctly, because cancellation is not definitive outcome or owner acknowledgement.

But no later production path exists yet for:

```text
definitive outcome
+ reconciliation if needed
+ owner acknowledgement
-> clear active slot
```

The current prototype can occupy both active slots and then reject a third promotion for the lifetime of that custody object.

This is a correct fail-closed intermediate state, not terminal executor implementation.

### 5.2 Recovered pending state is duplicated and never cleared in the current branch

`PROVEN`.

`RuntimeBackend` retains:

```text
pending: [Option<DurablePendingCheckpoint>; 2]
```

`WorkCustody::new(&pending)` copies each restored `operation_json` into its active slot. In the entire #335 diff, the only later direct use of `backend.pending` is the read-only `recovered_pending()` getter; no mutation/clear path exists.

For two maximum 65,536-byte restored operations, payload backing alone is therefore at least:

```text
RuntimeBackend.pending original Strings: 2 × 65,536
WorkCustody.active copies:             2 × 65,536
---------------------------------------------------
minimum duplicated payload backing:      262,144 B
```

before object/control/allocator overhead.

For a newly enqueued maximum operation, promotion moves the original queue allocation into active and makes one bounded submission clone, giving at least 131,072 B of payload backing during submission.

Required terminal lifecycle must coordinate durable slot clear, active custody clear and recovered in-memory snapshot retirement after definitive outcome + owner acknowledgement.

### 5.3 Submitter timeout is not executor cancellation

`PROVEN / CONTRACTUAL`.

The executor must own work independently from the waiting caller. A caller timeout can stop waiting; it cannot release operation identity, driver backing, transaction cleanup or ambiguous outcome state.

### 5.4 Transaction drop is not rollback completion

`PROVEN`.

SQLx transaction Drop only queues rollback. Physical protocol drain occurs later when the connection is driven again.

Any v2 slot release rule must survive this fact.

## 6. Deadline and retry violations

### 6.1 Accepted 2-second DB-pass deadline is not enforced end-to-end

`PROVEN`.

Child B configures a five-second SQLx pool acquire timeout. Semantic methods then call `backend.begin().await`, acquire relation/key locks and execute multiple queries without one propagated two-second absolute deadline.

No production `statement_timeout` / `lock_timeout` enforcement was found in the inspected #335 branch.

Required executor contract:

```text
one absolute pass deadline
  -> DB checkout
  -> custody fence
  -> relation/key locks
  -> every SQL await
  -> COMMIT/reconcile
```

A server-side timeout derived from remaining time may help physical cleanup but never releases custody by itself.

### 6.2 Ordinary PgPool connect contains retry/backoff

`PROVEN`.

Pinned SQLx retries refused/transient connect failures with exponential backoff until the acquire deadline.

This is incompatible with an **active** DFR pass if the pass invokes ordinary pool connect/acquire to manufacture a connection.

The new first-slice candidate avoids this by removing connection establishment from active work entirely.

## 7. Strong first-slice pool candidate: one physical connection

This section **supersedes the earlier “max_connections <= 2” recommendation as the preferred first-slice candidate**. A ceiling of two remains safe-looking but no longer appears minimal for the current serialized B design.

### 7.1 Why one connection is semantically plausible

`PROVEN / DERIVED — HIGH CONFIDENCE`.

Current Child B deliberately takes the same strongest relation fence before semantic database work. The fence uses `EXCLUSIVE` table locks across the complete 15-relation ledger and intentionally makes no concurrency claim.

The #335 patch documents that the semantic transaction starts were moved onto the shared backend/custody path and the inspected production starts then acquire relation/domain fencing.

A second database connection therefore does not provide first-slice transaction throughput while this global fence remains: the second writer would wait behind the first.

The DFR contract limits active custody to at most two slots but does **not** require two simultaneous PostgreSQL transactions.

### 7.2 SQLx already exposes the needed active-path primitive

`PROVEN`.

Pinned SQLx exposes public `Pool::try_acquire()` and `Pool::try_begin()`.

`try_acquire()`:

- takes only an already-idle connection;
- returns `None` immediately if no idle connection is available;
- does not open a new connection;
- does not enter the connect retry/backoff loop.

This removes the need for a deep pool-connect patch on the active operation path.

### 7.3 Candidate topology

`RECOMMENDATION / REQUIRES PROTECTED ARCHITECTURE`.

```text
one logical Durability executor
│
├── PgPool
│   ├── max_connections = 1
│   ├── min_connections = 0
│   ├── constructed lazily
│   ├── finite retirement policy retained
│   └── no automatic minimum-connection replacement
│
├── root-owned connection maintenance
│   ├── obtains CONNECT_TRANSIENT reservation before connect
│   ├── explicitly prewarms/reconnects the single connection
│   ├── owns any connect retry/deadline outside active DFR pass
│   └── explicitly returns the ready connection to idle
│
└── active operation
    ├── never opens a connection
    ├── uses try_acquire / equivalent fail-fast ready-only checkout
    ├── begins a transaction borrowing the explicit PoolConnection
    ├── executes one bounded DB pass
    ├── commits/reconciles/cleans
    └── awaits return-to-pool or close before active-slot release
```

Two logical active custody slots remain. They cover ambiguity, retained outcomes and bounded work ownership; they are not a requirement for two physical database connections.

### 7.4 Why `min_connections = 0` matters

`PROVEN`.

`connect_lazy` opens no connection immediately. SQLx maintenance may reap connections according to idle/lifetime policy, but `min_connections = 0` means the minimum-connection maintenance loop has no obligation to open a replacement.

This permits the product to make **all connection creation an explicit root-owned action** rather than an uncontrolled background resource event.

### 7.5 Active operation failure when no ready connection exists

`RECOMMENDATION`.

An active B pass should not wait for SQLx to connect/retry. If ready-only checkout fails:

1. the pass returns/fails closed according to the accepted Durability disposition;
2. root maintenance may independently establish a connection under its own reservation/deadline;
3. any later semantic retry is a new authorized operation/reconciliation action, not an automatic hidden retry inside the original pass.

The exact public error/disposition policy requires owning architecture review.

## 8. Connection close/return and physical tails

### 8.1 Pool permit/size protects logical max=1 through SQLx close

`PROVEN`.

`PoolConnection::close()` / floating close paths retain the pool size/permit guard until the underlying async connection close completes. With `max_connections = 1`, SQLx will not create a second **logical** pooled connection while that guard is still held.

### 8.2 Logical max=1 does not remove Tokio deferred reactor tail

`PROVEN`.

Tokio `RegistrationSet::deregister()` clones the connection's `Arc<ScheduledIo>` into `pending_release`. The I/O driver drops pending registrations at the beginning of a later driver turn.

Therefore a newly permitted SQLx connection can theoretically overlap with a deferred reactor-registration tail from the previous socket after SQLx close has returned.

This is a **root runtime** lifetime problem, not evidence that two PostgreSQL connections are needed.

Revision-4 v2 must either:

- include a source-derived worst-case reactor-retirement tail in `I`; or
- add one tiny reviewed reactor-finality seam if no defensible finite shared bound exists.

Do not call SQLx close “physical finality” without this lower-layer proof.

### 8.3 after_release is not final idle state

`PROVEN`.

SQLx invokes `after_release` before its final ping/protocol drain. A hook that observes bounded state has not automatically observed all later receive/status/error work.

### 8.4 awaited return is not global pool quiescence

`PROVEN / DERIVED`.

Pool return/drop may schedule root maintenance. With the recommended `min_connections = 0`, this cannot create a minimum-connection replacement, which materially simplifies the boundary. The reaper can still close an expired idle connection according to selected lifetime policy.

### 8.5 close_hard remains asynchronous

`PROVEN`.

PostgreSQL/rustls shutdown drives async I/O. Stalled-peer cleanup requires a tested bounded ownership policy; the method name does not imply instant backing destruction.

## 9. Corrected root-envelope equation

Define:

```text
I = fixed executor/runtime/pool/shared residency,
    including the proved worst-case deferred reactor-retirement tail
R = settled resident cost of the one ready connection
T = complete cost attributable to the one connection while at connect peak,
    including the resident portion it will retain
Q = actual current queue charge
A = actual current active custody charge across up to two logical slots
```

Under the Revision-4 single-connection candidate, logical two-connect overlap is removed from the product topology.

Candidate feasibility condition:

```text
I + max(R, T) + Q + A <= 12 MiB
```

This is **not accepted architecture yet**. It is valid only if qualification proves:

- all connection creation is root-owned and serialized;
- active paths cannot call connecting `acquire()`;
- SQLx max=1 is preserved through close/return;
- deferred lower-layer reactor/provider tails are fully included in `I` or `T`;
- there is no alternate direct `PgConnection` production path;
- actual current Q/A charges are used without double counting;
- completion/cleanup/ambiguity custody is already funded before irreversible COMMIT.

This is materially simpler than the Revision-3 two-connection equation `max(2R, R+T, 2T)`, but it must be proven, not merely selected.

## 10. Bootstrap/configuration findings

### 10.1 Schema inspection is unbounded today

`PROVEN`.

Runtime schema compatibility performs:

```sql
SELECT version, checksum, success
FROM _sqlx_migrations
ORDER BY version ASC
```

with `fetch_all()` before `WorkCustody` exists.

### 10.2 Exact current bootstrap bound is tiny

`PROVEN FOR #335@834db1d...`.

There are exactly two embedded migrations at this head. SQLx migration checksum is SHA-384, exactly 48 bytes.

A bounded compatibility shape can therefore read at most `N+1 = 3` rows and require exactly 48 checksum bytes before transfer. The third row is sufficient to prove an unexpected extra migration.

This converts a generic driver/resource problem into a small consumer SQL repair.

The future code should derive `N` from the embedded migrator rather than hard-code `2` forever.

### 10.3 URL length is not the entire configuration boundary

`PROVEN`.

SQLx PostgreSQL options can consume environment values, OS username fallback and pgpass configuration in addition to the explicit URL.

Pinned pgpass reads a line into a growable `String` without a project hard line-size bound and can log the full malformed line.

No actual secret leak is claimed by this audit. The finding is that bounded production configuration requires an explicit source set and redaction policy.

Required v2 decision:

```text
explicit URL/struct only?
selected PG* environment fields?
pgpass enabled or disabled?
OS username fallback enabled?
file-backed or preloaded CA/client credentials?
maximum bytes per enabled source?
redaction before dependency logging?
```

## 11. Relation-lock optimization: 15 round trips -> 1

`PROVEN / RECOMMENDATION`.

Current `lock_admission_relations()` loops over 15 fixed table names and performs 15 separate:

```sql
LOCK TABLE <name> IN EXCLUSIVE MODE
```

awaits.

PostgreSQL 17 documents that:

```sql
LOCK TABLE a, b;
```

is equivalent to separate `LOCK TABLE a; LOCK TABLE b;` commands and locks the tables one-by-one in the specified order.

Therefore the first-slice relation fence can be represented as one static multi-table `LOCK TABLE ... IN EXCLUSIVE MODE` preserving the same lexical lock order and mode.

Benefits:

- 15 -> 1 protocol awaits/round trips per transaction;
- 15 -> 1 relation-lock SQL shapes;
- less statement-cache residency;
- lower latency pressure against the accepted 2-second pass deadline;
- no weakening of the intentionally global first-slice writer serialization.

Official PostgreSQL 17 reference:
`https://www.postgresql.org/docs/17/sql-lock.html`

This is a Child B repair, not a reason to deepen WP3.

## 12. Statement-cache census

`PROVEN / DERIVED CONSERVATIVE UPPER BOUND`.

Exact source census at #335 head gives:

```text
admission_journal.rs        33 sqlx::query* call-sites
mod.rs                      22
fresh_admission.rs          10
admission_authority_guards   4
                              --
base call-site upper bound   69
```

The count is conservative because source files include test-only material and duplicate SQL text.

Additional dynamic-shape families inspected:

- admission journal has one `let sql = if for_update` family -> at most +1 shape beyond its one call-site;
- guards have two `QueryBuilder` families and exactly four closed key/table variants -> at most +8 shapes;
- current `db.rs` relation locks -> 15 shapes plus one advisory-lock shape;
- schema inspect -> one shape.

Conservative current upper bound:

```text
69 + 1 + 8 + 15 + 1 + 1 = 95 SQL shapes
```

Pinned `QueryBuilder::build()` and ordinary `sqlx::query()` are persistent by default, so these shapes are relevant to the prepared-statement cache.

Pinned PostgreSQL statement-cache capacity defaults to 100.

After the 15 -> 1 relation-lock consolidation:

```text
95 - 14 = 81 conservative shapes
```

leaving at least ~19 cache entries of count headroom under this source census.

Revision-4 recommendation:

```text
KEEP upstream statement_cache_capacity = 100 initially
```

Do not disable the cache merely to simplify accounting: persistent named statements with a disabled client cache require separate server-statement cleanup proof.

This closes the **count** question much more strongly, but not the byte footprint. `R` must still bound SQL text/parameter/column metadata for the final exact query corpus.

## 13. Connection-resident state

### 13.1 Hard socket-buffer lower bound

`PROVEN`.

Pinned `BufferedSocket` starts with approximately 8 KiB write capacity plus 8 KiB read spare capacity:

```text
R >= 16,384 B / live connection
```

before TLS state, socket/reactor control state and caches.

With the new max=1 candidate this lower-bound term is no longer multiplied by two.

### 13.2 Retained PostgreSQL maps remain part of R

`PROVEN`.

`PgConnectionInner` retains:

- statement cache;
- type-info / type-OID maps;
- element-to-array map;
- table metadata map;
- stream status state.

`clear_cached_statements()` does not clear every retained type/table map. `shrink_buffers()` only shrinks stream buffers.

### 13.3 ParameterStatus is an unbounded growth surface today

`PROVEN`.

`ParameterStatus` name/value pairs become owned Strings and most are retained in a connection-lifetime BTreeMap.

The B production profile needs an exact allowlist plus count/bytes bound, or a seam that does not retain generic arbitrary status entries.

### 13.4 Built-in-only type profile remains a strong candidate

`DERIVED — REQUIRES FINAL GRAPH/QUERY INVENTORY`.

The inspected B corpus uses built-in PostgreSQL types and ordinary runtime execution does not need generic table-origin discovery.

Candidate first-slice profile:

```text
built-in PostgreSQL types only
no generic by-name custom type resolution
no table-origin discovery requirement
```

Any later custom type/profile widening requires separate bounded evidence.

## 14. Receive/decode seams that remain real

### 14.1 Backend frame length

`PROVEN`.

`PgStream::recv_unchecked()` accepts the peer-provided PostgreSQL message length before ordinary socket-buffer growth. A checked gate before reserve remains necessary unless the exact consumer/protocol layer can prove a tighter invariant earlier.

### 14.2 Huge-count/tiny-body DataRow

`PROVEN`.

`DataRow::decode_body()` reads peer `u16` column count and allocates `Vec::with_capacity(count)`.

65,535 NULL columns require only:

```text
2 + 65,535 * 4 = 262,142 B body
```

which is below the accepted 524,288 B aggregate result-byte ceiling but drives a huge metadata vector.

Byte limits alone are therefore insufficient. A count/query-profile gate is mandatory before allocation.

The DFR 32-variable-column rule is not automatically a universal wire-column limit; the final query/profile decision must state the exact accepted row/column shape.

### 14.3 RowDescription

`PROVEN`.

`RowDescription` has the same peer count -> vector + owned field-name family and needs byte/count/profile control.

### 14.4 Returned rows/values

`PROVEN`.

`PgRow`, `PgValue`, shared `Bytes` and metadata `Arc`s can outlive the immediate executor call. Active custody must remain until descendants are dropped or cross an explicit charged copy boundary.

## 15. Consumer query-shape gaps

### 15.1 Pending commands

`PROVEN`.

#335 still has production `fetch_all()` paths for pending-command child rows and checks vector count only after materialization.

The accepted shape is one bounded ordered aggregate per exact attempt.

### 15.2 Unbounded reconnect record reads

`PROVEN`.

Several `SELECT state, record_json ...` paths transfer variable `record_json` without the same-snapshot byte guard already used correctly elsewhere in Child B.

### 15.3 One additional reconnect binding fetch_all family

`PROVEN`.

`active_committed_binding_is_valid` materializes a row vector and then requires exactly one row. This can be converted to a bounded `LIMIT 2` / exact-cardinality shape, with variable record payload guarded before transfer.

### 15.4 Schema ledger

`PROVEN`.

The migration compatibility read is another `fetch_all` family, but unlike pending commands it has a tiny exact current bound: expected migration count + one sentinel row.

These are primarily Child B query repairs, not reasons for a broad driver fork.

## 16. Error/logging custody

### 16.1 Application error normalization

`PROVEN`.

`DurabilityError::Database(sqlx::Error)` can retain full PostgreSQL `ErrorResponse` / Notice fields and Display them.

Recommendation: normalize the full driver error inside the active slot into a bounded Durability disposition, bounded SQLSTATE/correlation data and no unbounded server prose, then destroy the full driver error before slot release.

### 16.2 SQLx internal logging is a separate sink

`PROVEN`.

Pool paths log `%error` internally for ping/return/after-release failures before application normalization can occur.

A complete v2 policy must therefore cover dependency logging sinks as well as errors returned to Child B. Candidate controls include exact tracing filters or a narrow profile-gated logging seam; silent assumptions are insufficient.

## 17. TLS/connect quantitative evidence

Pinned ring research already derives:

```text
non-ECH decoded heap bound = 2,557,169 B
ECH decoded heap bound     = 3,276,650 B
known overlapping span     =   327,680 B
partial connect term       = 3,604,330 B
```

This is a **partial phase term**, not complete T. Configuration, socket/reactor state, provider/KX state, retained certificate data, error paths and exact final provider graph remain to be composed.

Against one 4 MiB active slot, that partial term leaves only:

```text
4,194,304 - 3,604,330 = 589,974 B
```

This strongly supports keeping connection/TLS establishment outside an active slot.

Under the single-connection candidate only one logical connect transient needs to be admitted at a time. With a full nominal 4 MiB queue, the partial T alone would leave:

```text
12,582,912 - 4,194,304 - 3,604,330 = 4,784,278 B
```

for `I + A`.

The final T may be larger. No acceptance follows until the production provider/configuration graph is frozen and every overlapping lifetime/failure exit is included.

Feature absence claims must be proven from the resolved exact production Cargo graph and runtime configuration; one dependency edge is not sufficient because Cargo features unify.

Measurement corroborates source bounds but does not replace source/lifetime/failure proof.

## 18. Provider/runtime shared residency

Protected AWS-LC architecture already established finite provider process/per-thread reservation terms for its exact qualified profile.

Thread-first-use accounting means thread cohort/lifetime matters. A fixed dedicated Durability thread can simplify that dimension, but this audit does not select the provider or runtime.

Candidate runtime remains:

```text
one dedicated Durability executor thread
-> Tokio current-thread runtime
-> DB I/O only
-> max-one physical PG connection
```

Unknowns requiring measurement/source proof:

- OS thread stack;
- current-thread Tokio baseline;
- registration/pending-release bound;
- scheduling/fairness under crypto/parser load;
- shared-vs-dedicated runtime comparison.

## 19. Minimal SQLx seam — Revision-4 candidate

The stronger max=1/ready-only design removes more fork pressure.

### SQLx core changes that may still be needed

- root/connection budget/profile plumbing if ordinary APIs cannot express the final bound cleanly;
- separate transport address from TLS server name if selected;
- prospective socket-buffer growth checks;
- a stable awaited return/finality helper if product code should not depend on doc-hidden `return_to_pool()`;
- bounded internal logging behavior if tracing policy cannot close the error sink.

### SQLx PostgreSQL changes that remain strongly justified

- backend frame-length gate before receive-buffer reserve;
- DataRow/RowDescription count/profile gate before vector allocation;
- bounded/allowlisted retained ParameterStatus state;
- prevent unbounded custom type/table cache growth in the selected B profile;
- exact transport/TLS-name seam if the profile uses separate address/identity;
- expose only the cleanup/high-water state actually required for R proof.

### Changes that should be avoided if the v2 proof succeeds without them

- generic Tokio DNS redesign;
- generic Tokio blocking-worker ownership;
- broad rustls internal AST/container ownership;
- unrelated Tokio TCP/UDS semantics;
- complete theoretical SQLx feature accounting outside the frozen B profile.

## 20. Fork disposition

| Component | Revision-4 disposition |
|---|---|
| `ResourceBudget` | KEEP |
| `ResourceReservation` | KEEP |
| exact `Charged<T>` backing wrappers | KEEP where still required |
| SQLx core fork | REWORK / NARROW |
| SQLx PostgreSQL fork | REWORK / NARROW |
| broad rustls custody fork | PRESERVE AS EVIDENCE; candidate removal after replacement proof |
| Tokio blocking-owner fork | PRESERVE AS EVIDENCE; candidate removal |
| Tokio reactor fork | NEEDS_DECISION; prefer finite root tail if defensible |
| runtime/thread-finality research | PRESERVE |
| PG17.6/TLS harness | KEEP / HARDEN |
| hostile vectors/source census | KEEP |
| one sampled peak used as universal bound | REJECT |

No existing fork should be deleted before the replacement consumer path is independently qualified.

## 21. Status of requested quantitative bounds

### `EXECUTOR_RUNTIME_RESIDENT` (`I`)

`UNKNOWN — PARTIALLY CONSTRAINED`.

Must include selected runtime baseline, pool fixed structures, queue/semaphore/event state, shared TLS/provider state, maintenance task state, configuration/bootstrap retained state and the proven worst-case deferred reactor retirement tail.

### `CONNECTION_RESIDENT` (`R`)

`UNKNOWN — MULTIPLICITY REDUCED TO ONE IN THE PREFERRED CANDIDATE`.

Hard source-visible lower bound remains:

```text
>= 16,384 B
```

from SQLx read/write buffers alone.

R must additionally include settled TLS/socket/reactor state and bounded statement/status/type/table metadata.

### `CONNECT_TRANSIENT_PEAK` (`T`)

`UNKNOWN — LARGE PARTIAL TERM ESTABLISHED`.

```text
partial source-derived ring research term = 3,604,330 B
```

Only one logical T is needed at a time under the preferred max=1/root-serialized candidate, but lower-layer deferred tails still belong in I/T.

### `ACTIVE_SQL_PEAK`

`UNKNOWN — MEASUREMENT MUST FOLLOW EXECUTOR/QUERY REPAIR`.

Useful current hard payload components:

- fresh promoted maximum operation: active original + submission clone -> at least 131,072 B payload backing;
- two maximum recovered pending operations at startup: backend snapshot + active copies -> at least 262,144 B payload backing before later semantic/reconciliation work.

The current source does not yet provide the terminal active-slot clear/ack lifecycle, so a final active peak measured now would be the wrong architecture.

## 22. Required v2 architecture decision

One decision should replace further micro-amendment growth and answer at minimum:

### Root/custody

1. exact root/shared/connection/connect-transient/active resource classes;
2. double-count prevention;
3. definitive active-slot release + owner acknowledgement;
4. recovered pending snapshot retirement;
5. submitter cancellation semantics.

### Executor

6. single sealed production entrypoint;
7. eight queued / two logical active custody;
8. physical DB execution concurrency (preferred first-slice candidate: one);
9. one 1s queue deadline;
10. one 2s DB-pass deadline;
11. no hidden active-pass retry.

### Pool

12. `max_connections` (preferred first-slice candidate: 1);
13. `min_connections` (preferred candidate: 0);
14. lazy construction;
15. finite max-lifetime/idle retirement policy;
16. explicit root prewarm/reconnect trigger and deadline;
17. ready-only active checkout;
18. awaited return/close boundary;
19. deferred reactor tail accounting.

### Bootstrap/configuration

20. bounded migration-ledger inspection;
21. allowed PG environment sources;
22. pgpass/username policy;
23. credential/root input forms;
24. diagnostic redaction.

### SQL profile

25. exact query corpus/family;
26. relation-lock one-statement form;
27. built-in/custom type policy;
28. statement cache policy;
29. returned row/count/byte policy;
30. ParameterStatus policy;
31. bounded error/logging policy;
32. pending-command aggregate;
33. same-snapshot variable-value guards.

### TLS/runtime

34. exact resolved provider/features;
35. TLS/SSL mode;
36. transport address and TLS server identity;
37. DNS policy;
38. CA/client credential loading;
39. resumption/compression policy;
40. runtime topology;
41. exact I/R/T proof and supported target/toolchain/allocator assumptions.

### Supersession

42. protected WP3 amendments retained as authority;
43. amendments superseded but retained as evidence;
44. #356 code/tests salvaged;
45. broad vendor patches removed only after replacement qualification.

## 23. Recommended implementation sequence after protected v2 decision

This audit grants no implementation authority.

### V2-0 — freeze expansion

Hold new broad rustls/Tokio owner propagation. Continue read-only census and bounded experiments.

### V2-1 — repair Child B substrate

- one real executor;
- seal persistence bypass APIs;
- active definitive release/ack;
- recovered pending snapshot retirement;
- absolute queue/pass deadlines;
- 15 -> 1 relation-lock statement;
- bounded migration inspect;
- pending-command aggregate;
- guarded record reads;
- bounded application/dependency error logging.

### V2-2 — freeze exact product DB/TLS/config/runtime profile

Include pool=1 candidate, configuration sources and resolved Cargo graph.

### V2-3 — derive/measure I/R/T/active peak

Source proof must cover phase entry, overlap, escaping descendants, re-entry, success transfer and every failure/cancellation exit. Measurement is corroboration.

### V2-4 — implement only remaining minimal SQLx seams

Prefer upstream rustls/Tokio behavior behind finite root/phase reservations where proven.

### V2-5 — exact consumer qualification

At minimum prove:

- exact production Cargo feature graph;
- bounded configuration/credential intake;
- PostgreSQL 17.6;
- selected TLS policy with hostname verification where required;
- lazy pool with exactly the selected max/min behavior;
- no connection establishment from active pass;
- root connect maintenance under one T reservation;
- one physical connection and two logical active slots;
- eight queue slots/max+1;
- 1s queue deadline;
- 2s DB pass including lock waits;
- no active-pass retry loop;
- repeated sequential success beyond two operations;
- both logical active slots occupied by uncertain outcomes and later recovered;
- definitive acknowledgement releases capacity;
- recovered pending snapshot retires after closure;
- bounded migration ledger;
- 15 -> 1 relation lock preserves exact lock order;
- pending-command bounded aggregate;
- result row/count/byte max and max+1;
- huge-count/tiny-body rejection;
- bounded ParameterStatus;
- retained row/value/error lifetime;
- final return/ping/close behavior;
- stalled TLS close;
- deferred reactor tail;
- lost/ambiguous COMMIT reconciliation;
- statement cache <= selected finite corpus;
- provider cold/warm/thread-cohort behavior;
- root inequality at worst permitted state;
- independent allocation/release-order observation while runtime remains alive;
- strict Rust 1.94 fmt/check/Clippy/tests;
- affected vendored and ordinary-consumer profiles;
- supply-chain/provenance;
- independent exact-head HIGH-risk review;
- canonical CI / FULL Merge Queue / protected-main readback.

## 24. Verification matrix status

The earlier audit tracks `WP3-Q01..WP3-Q70`. Revision 4 retains them and adds these required cases; they are requirements, **not claims of already executed tests**.

| ID | Required proof |
|---|---|
| Q71 | Active DB work uses ready-only checkout and cannot enter connect/retry. |
| Q72 | `max_connections=1` blocks a second logical pooled connection through close/return. |
| Q73 | Root prewarm/reconnect is the only production connection-establishment owner. |
| Q74 | `min_connections=0` prevents automatic replacement connect after reaping/return failure. |
| Q75 | Two logical active custody slots work correctly with one physical connection. |
| Q76 | More than two sequential acknowledged operations reuse released active capacity. |
| Q77 | Durable pending slot + active custody + recovered in-memory snapshot retire together after ack. |
| Q78 | Migration compatibility reads at most expected+1 rows and exact checksum size. |
| Q79 | One multi-table relation lock preserves current lexical order and lock mode. |
| Q80 | Statement-shape census remains below selected cache capacity on the exact production corpus. |
| Q81 | SQLx internal logging cannot emit unbounded peer/configuration content in the B profile. |
| Q82 | One old deferred reactor registration plus new connection activity fits the root bound. |
| Q83 | Reaper can retire an idle connection without auto-replacement and active work fails closed until root maintenance succeeds. |
| Q84 | Root maintenance connect timeout/retry has its own finite reservation and cannot consume active-slot budget. |

## 25. Explicit corrections/supersessions from earlier audit revisions

1. Earlier `min=2/max=2/idle=None/max_lifetime=None` recommendation is superseded.
2. Earlier `max_connections<=2` remains an upper bound candidate but is no longer the preferred first-slice topology; `max=1` is now the stronger candidate for the current globally serialized B design.
3. Earlier two-connect `max(2R,R+T,2T)` equation remains relevant only if v2 selects a two-connection topology. Preferred max=1 candidate uses `max(R,T)` plus a fully funded shared/deferred tail inside I.
4. Certificate-compression/resumption unreachability cannot be asserted from one dependency edge; exact resolved production graph/configuration is required.
5. `after_release` and SQLx logical close/return are not automatically lower-layer physical finality.
6. Literal transport address does not by itself close ambient PG configuration/pgpass intake.
7. The DFR 32-variable-column rule is not silently promoted into a universal PostgreSQL wire-column cap.

## 26. What must not be inferred

This report does **not** prove or authorize:

- that pool max=1 has acceptable final product throughput;
- that max=1 remains correct after global relation fencing is later relaxed;
- that min=0 is accepted product policy;
- that idle timeout should be disabled or max lifetime changed without evidence;
- that a dedicated Tokio runtime is required;
- that production must use ring or AWS-LC;
- that one sampled memory peak is a universal bound;
- that broad rustls/Tokio forks can already be deleted;
- that SQLx internal logging is already safely bounded;
- that the 95/81 statement-shape census is a byte bound;
- that #335 is authorized to mutate before normal dependency/custody gates;
- that #356 can be closed before evidence migration/supersession is protected;
- that green audit-document CI proves WP3/WP4 readiness.

## 27. Integration impact

### WP2

Do not restart protected WP2 work. Preserve its nonreuse/replay semantics.

### WP4 / Child B

WP4 remains owner of durability atomicity/reconciliation. The executor/deadline/bootstrap/query/finality substrate must be repaired before WP3 can be qualified as its production dependency.

### WP5

Source readiness remains independent. Resource accounting does not manufacture production authority sources.

### G0 / Server Seam

No release follows from this audit. G0/Server Seam require the composed exact state after terminal WP3, qualified WP4 and required WP5 composition.

## 28. Current unknowns

`UNKNOWN` — exact `EXECUTOR_RUNTIME_RESIDENT` I.

`UNKNOWN` — exact final one-connection resident R.

`UNKNOWN` — exact complete connect transient T.

`UNKNOWN` — exact active SQL peak after executor/query repair.

`UNKNOWN` — exact deferred reactor-retirement bound for selected runtime/toolchain.

`UNKNOWN` — final production provider/TLS/configuration policy.

`UNKNOWN` — exact statement metadata byte bound for the <=81 preferred query-shape corpus.

`UNKNOWN` — selected provider thread cohort/lifetime.

`UNKNOWN` — performance/latency of the one-physical-connection first slice under representative load.

No unknown above authorizes a guessed cap, silent TLS weakening or an extra independent budget.

## 29. Final go/no-go

**NO-GO for integrating #356 as terminal WP3 architecture in its audited state.**

**GO for preserving #356 as the canonical research/evidence lineage while a single superseding WP3-v2 decision is prepared and reviewed.**

**NO-GO for further broad per-allocation rustls/Tokio ownership expansion before that decision.**

**GO for evaluating the max-one-physical-connection / two-logical-active-slot candidate as the smallest first-slice architecture, because it removes connect/retry from active work and matches the current globally serialized B lock model.**

Strongest blockers, ordered:

1. real B semantic APIs bypass the accepted executor/custody model;
2. active slots have no definitive release/ack path;
3. recovered pending snapshot is duplicated and has no retirement path;
4. accepted 2-second DB-pass deadline is not enforced;
5. ordinary connecting pool acquire would retry inside a pass;
6. configuration/schema/custody work exists before WorkCustody;
7. pending-command/record/schema SQL transfer shapes still contain gaps;
8. PostgreSQL frame/count/ParameterStatus growth remains unbounded in the final consumer profile;
9. cleanup/return/close and reactor retirement are not one finality event;
10. product TLS/provider/configuration profile is not frozen;
11. I/R/T/active peak and joint root fit are not proven;
12. broad dependency-owner propagation is higher maintenance risk than the now-evidenced root/phase-reservation alternative.

Positive work to preserve:

- `ResourceBudget` / reservation primitives;
- source census/provenance;
- hostile denial vectors;
- real PostgreSQL 17.6/TLS harness;
- provider/native finite-bound research;
- handshake phase-bound research;
- sound backing-finality repairs;
- socket/reactor/DNS source decomposition;
- correctly bounded two-slot pending reload SQL;
- repository governance/exact-head qualification discipline.

Final markers:

```text
WP3_V2_BOUNDARY_DECISION_REQUIRED
CHILD_B_DFR_EXECUTOR_REPAIR_REQUIRED
CONFIGURATION_BOOTSTRAP_BOUND_REQUIRED
SINGLE_CONNECTION_FIRST_SLICE_CANDIDATE
#356 = OPEN / DRAFT / PRESERVE EVIDENCE / NEEDS_DECISION
#335 = OPEN / NON-DRAFT / NOT READY FOR FINAL COMPOSED QUALIFICATION
```

This report is evidence and recommendation only. Protected repository architecture, live #162/#351/#329 authority and normal review/CI/Merge Queue controls remain binding until a separately reviewed/protected decision changes them.
