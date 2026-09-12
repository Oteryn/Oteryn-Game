# Oteryn Game — WP3-v2 superseding architecture decision

- Decision ID: `WP3-V2-ROOT-OWNED-BOUNDED-PGPOOL-V1`
- Revision: **3 — independent-review P1/P2 closure; frozen lifecycle overlap and recovery trigger**
- Date: 2026-09-12
- Status: **CANDIDATE / SUCCESSOR EXACT-HEAD VALIDATION / NOT ACCEPTED**
- Worker: `Oteryn: astra wp3-v2 architecture lead`
- Protected admission: `main@489e3e390a1bce1ce3439c66521ab75f8a826cd8`
- Canonical WP3 lineage: Issue #351 / Draft PR #356 @ `fe7891989b1247012e32c89c10cff6a10bacb943`
- Canonical Child B lineage: Issue #329 / PR #335 @ `834db1d7118d751e31287715d3eaac7780a0c7b9`
- Programme: #162 / #364
- Retained audit: Draft PR #588
- WP3-v2 programme: Draft PR #589
- This candidate: Draft PR #590
- Architecture authority: **none**. This revision repairs only the independent-review P1/P2 findings; A4 receives no write authority from this file.

## 1. Resolution

Select **Option B** for the first safe production slice:

> one process-scoped logical Durability executor, one accepted DFR root ledger, one lazy SQLx `PgPool` used as a **single-ready-connection holder**, root-owned serialized connection establishment outside active DFR work, `Pool::try_begin()` for active ready-only transactions, two logical active custody slots, at most one physical PostgreSQL transaction at a time, and only narrow SQLx/PostgreSQL/rustls seams required by the exact frozen profile.

First-slice topology:

```text
one Durability executor / one DFR root
|
+-- I: fixed executor/runtime/pool/provider/config/shared-tail residency
|
+-- PgPool holder
|   +-- max_connections = 1
|   +-- min_connections = 0
|   +-- lazy construction
|   +-- idle_timeout = 10 minutes
|   +-- max_lifetime = 30 minutes
|   +-- at most one established/retiring PG generation R
|
+-- root connection maintenance
|   +-- only production connect/reconnect owner
|   +-- one connect/failed-retirement generation T at a time
|   +-- 5 second root-maintenance acquire/connect deadline per recovery window
|   +-- bounded SQLx connect backoff allowed only inside that root window
|   +-- awaited return/ping before T -> R readiness transfer
|   +-- no new T while any prior R or T retirement tail is non-final
|
+-- DFR work custody
    +-- queued <= 8
    +-- logical active <= 2
    +-- physical DB pass <= 1
    +-- queue residence <= 1 second
    +-- DB execution/reconciliation pass <= 2 seconds
    +-- active path uses Pool::try_begin(), never connecting acquire/begin
```

Root feasibility invariant:

```text
I + max(R, T) + Q + A <= 12 MiB
```

For this first slice the terms and overlap behind that equation are frozen, not left to A4:

- `I` is root/process backing whose lifetime is independent of one physical connection generation: executor/pool control structures, explicit retained configuration, and genuinely shared runtime/provider backing. Per-connection socket/TLS/driver/reactor descendants do **not** migrate into `I` merely because their cleanup is asynchronous.
- `T` is the complete charge for the single root-owned connection-establishment generation, beginning before the first controlled connect/TLS/auth allocation and ending only by either (a) charged ownership transfer into `R` after successful establishment plus awaited pool return/final ping, or (b) complete finality of every descendant after connect failure, timeout or cancellation. A failed/timed-out attempt remains `T` during its retirement tail.
- `R` is the complete charge for the single established physical-connection generation from the successful `T -> R` transfer through ready/checked-out use and through any fenced, reaper, close, return or reactor retirement tail until every per-connection descendant is final. A logically removed/reaped connection remains `R` until that finality point.
- `R` and `T` are mutually exclusive connection generations. Root maintenance must not begin a new `T` while any prior `R` or `T` retirement tail remains non-final. Successful establishment is an ownership transfer `T -> R`, never a double-charged overlap.

Therefore there is no hidden third retirement term outside `max(R,T)`: every connection-attributable tail remains inside the generation that created it, while genuinely process-shared residency is in `I`. `Q` and `A` are the actual current queue/active charges under `DUR-FRESH-RESOURCE-ENVELOPE-V1`; they are not duplicated per connection. Exact final byte values for `I`, complete lifecycle `R`, complete lifecycle `T` and active SQL peak remain implementation-qualification obligations. They are deliberately `UNKNOWN` here rather than fabricated architecture numbers. If the exact implementation cannot enforce the frozen non-overlap/finality boundary above, the equation is invalid and A4 must escalate rather than choosing a different overlap model.

## 2. Mandatory decision test

### Must decide now?

**YES.**

### Concrete downstream work blocked

- WP3-v2 Gate 1 cannot release A4 without one superseding architecture.
- #356 cannot truthfully continue broad dependency ownership expansion while the real consumer is a process-shared backend/pool/executor lifecycle.
- #335 cannot be the terminal WP3/WP4 consumer while semantic persistence bypasses the accepted executor, active custody has no definitive release/ack path, and ordinary DB passes lack one absolute two-second deadline.
- WP4/WP5/fresh G0/Server Seam remain downstream of a terminally qualified WP3 consumer.

### What becomes harder later?

- retaining generic rustls/Tokio forks creates recurring upgrade/provenance/review cost;
- a connection-actor rewrite would create a second work-ownership API beside the accepted DFR executor;
- two physical connections enlarge overlap/finality proof before product evidence shows first-slice benefit;
- ambiguous cleanup/retry semantics become durable compatibility debt once consumers depend on them.

### Evidence that justifies later supersession

Reopen this decision when one of these is proven:

- representative mixed-operation latency/throughput shows one physical connection violates an accepted product SLO;
- required correctness locks are safely narrowed and useful parallel PostgreSQL work becomes possible;
- exact source proof shows holder-style `PgPool` finality requires a broader maintained fork than one explicit connection actor;
- another provider/profile materially reduces the complete proved root cost or maintenance burden;
- changed availability/failover requirements cannot be met by this first-slice topology.

Cross-repository finding R21 remains binding: **one physical connection is not declared sufficient solely because current relation locks serialize writes**. It is selected because it is the smallest first-slice topology; performance remains a measured supersession criterion.

### Deliberately not decided

- later multi-connection throughput topology;
- later relaxation of the global relation fence;
- multi-node DB failover;
- durable-history retention/compaction;
- Platform source deployment;
- gameplay Server Seam TLS profile;
- any new public resource maximum not already accepted by DFR.

## 3. Exact evidence pins used by Revision 3

### Protected / live state

- Game protected main: `489e3e390a1bce1ce3439c66521ab75f8a826cd8`.
- #356 remains OPEN/DRAFT at `fe7891989b1247012e32c89c10cff6a10bacb943`.
- #335 remains OPEN at `834db1d7118d751e31287715d3eaac7780a0c7b9`.
- PR #451 is merged and protects the exact non-FIPS Linux x86-64 GNU AWS-LC finite provider-residency/KX architecture.
- PR #453 is merged and protects the owner-aware provider-config allocation.
- PR #458 is merged and protects the SQLx AWS-LC `prefer-post-quantum` feature-edge allocation.
- PR #455 was closed without merge; its proposed `208`-byte root-graph correction is **not protected architecture authority**.

### Exact Child B production source

At #335 the production durability module consists of:

- `apps/game-server/src/durability/db.rs`;
- `apps/game-server/src/durability/schema.rs`;
- `apps/game-server/src/durability/admission_journal.rs`;
- `apps/game-server/src/durability/fresh_admission.rs`;
- `apps/game-server/src/durability/admission_authority_guards.rs`;
- `apps/game-server/src/durability/mod.rs`.

The read-only source pass classified the reachable SQL/lock/config/pool/startup families needed to choose the architecture. This is evidence collection, not an independent final architecture review.

## 4. Options and disposition

### A — broad SQLx/rustls/Tokio ownership instrumentation

**REJECT as terminal architecture; retain exact useful evidence.**

Useful work exists: `ResourceBudget`/reservations, hostile vectors, lower-layer lifetime research, real PG/TLS harnesses, provider accounting and source census. But broad generic instrumentation does not solve the real consumer's executor bypass, pool bootstrap, active finality, config authority or deadline model, and creates a large dependency-maintenance surface.

### B — finite root + bounded holder-style PgPool + minimal seams

**SELECT.**

Why:

- Child B already uses `PgPool` and SQLx transactions across its semantic implementation.
- SQLx 0.9.0 exposes `Pool::try_begin()`, which first calls `try_acquire()` and returns an owned `Transaction<'static, DB>` only from an existing idle `PoolConnection`; it cannot create a connection.
- This preserves transaction-centric Child B APIs while removing connect/retry from active work.
- `max_connections=1` removes logical two-connect overlap from the first-slice topology.
- `min_connections=0` ensures reaping/return failure does not trigger minimum replacement.
- most current query gaps are repairable at the consumer/query layer without generic driver redesign.

### C — explicit PostgreSQL connection actor(s)

**REJECT for first slice; preserve as supersession option.**

A direct actor would make connection ownership explicit, but today it would require replacing the existing `PgPool`/transaction boundary with an actor mailbox or shared-connection execution API across the consumer. That duplicates or rewrites the accepted DFR work-custody model. Two actors also reintroduce multi-connection `R/T` overlap before evidence demonstrates benefit.

Reconsider C only if the narrow owner-aware pool seam below proves broader than an actor rewrite, or measured product evidence requires additional physical DB concurrency.

## 5. Root/executor ownership

Exactly one production Durability executor exists per process in this first slice and exactly one accepted DFR root ledger funds all attributable retained work.

The root owns:

- fixed executor/runtime structures;
- pool structures and maintenance task state;
- explicit bounded DB configuration and credential backing;
- one established-or-retiring physical connection generation `R`;
- the one serialized connect-or-failed-retirement generation `T`;
- selected TLS/provider shared residency;
- lower-layer socket/reactor/provider tails according to the frozen `I/R/T` classification above;
- queue/active sub-reservations and their descendants until transfer/finality.

A connection, request, account, task, retry, reconnect or TLS provider use does **not** mint another 12 MiB budget.

No second owner identity may represent the same charged backing. Shared backing is charged once only with explicit ownership/finality; deep copies are separate charges.

## 6. Queue/active custody and definitive release

### Queue

- maximum eight not-yet-submitted operations;
- reserve before retaining/copying the sealed operation;
- one-second queue-residence ceiling;
- queued cancellation releases only work not yet promoted.

### Logical active

- maximum two active operation identities;
- promotion moves original custody before DB awaits and funds required submission/completion/reconciliation state before irreversible effects;
- only one active identity may own the physical DB pass at a time;
- the second logical active slot may retain an ambiguous/reconciliation obligation while another slot progresses when the accepted state machine permits it;
- caller timeout/cancellation stops waiting only; it never proves rollback/non-commit/cleanup/finality.

### Release rule

An active slot clears only after all are true:

1. exact original operation has a definitive durable disposition;
2. ambiguous COMMIT has been reconciled by original immutable identity when necessary;
3. transaction/protocol cleanup has completed or the connection has been fenced/retired under root custody;
4. bounded completion reached the owning consumer;
5. owner acknowledged the completion;
6. matching durable pending checkpoint is cleared/finalized;
7. recovered in-memory pending duplicate is retired;
8. no descendant charged to the slot remains except through an explicit charged ownership transfer.

`Transaction::drop`, caller timeout, future cancellation, `after_release`, wrapper drop, SQLx logical close or a timeout around close is not finality by itself.

## 7. Exact pool policy

Freeze the first-slice pool policy to:

```text
max_connections = 1
min_connections = 0
construction = connect_lazy_with(explicit bounded PgConnectOptions)
idle_timeout = 10 minutes
max_lifetime = 30 minutes
root acquire/connect timeout = 5 seconds per recovery window
active checkout = Pool::try_begin()
recovery trigger = coalesced root_ready_demand event; no periodic/polling retry loop
```

The 10-minute/30-minute values are the pinned SQLx 0.9.0 defaults and are intentionally retained rather than inventing a new policy. They may be superseded later by measurement.

### Root maintenance

Only root maintenance may create a connection. It may use SQLx's existing connect/backoff loop inside one five-second root recovery window because that work is outside active DFR custody and funded by `T`.

The root owns one coalescing `root_ready_demand` latch. Exactly these first-slice events may set it:

1. **startup/takeover demand:** entry into a lifecycle step that must obtain the ready physical connection before that step can progress;
2. **ready miss:** an otherwise authorized active DB pass calls `Pool::try_begin()` (or the exact ready-only equivalent) and receives `None`; that pass still fails closed/unavailable, while the miss signals root maintenance;
3. **root-owned retirement:** the root explicitly fences/retires a connection and knows readiness has become absent.

A silent SQLx reaper close does not require a new generic reaper callback: `min_connections=0` permits the holder to become empty, and the next ready-only `try_begin()==None` is the exact demand trigger. The reaper itself never opens a replacement.

Trigger handling is frozen as follows:

- triggers are idempotently coalesced; they do not allocate one retry object per caller;
- one latched demand authorizes at most one five-second recovery window;
- a recovery window may begin only after every prior `R` or `T` retirement tail is proven final, preserving the frozen `max(R,T)` non-overlap;
- at window start, the currently latched demand is consumed; any new demand arriving while that window or its retirement tail is in progress may set the latch once for a later successor window;
- on successful connect, root maintenance explicitly awaits the pool return path through its final return ping, performs the charged `T -> R` transfer, publishes readiness, and does not run another window unless a later demand occurs;
- on failure/timeout/cancellation, readiness remains absent, the failed generation remains charged as `T` until all descendants are final, and there is **no autonomous periodic or immediate retry loop**. Only a new/coalesced demand event authorizes a later recovery window.

This freezes recovery policy rather than delegating it to A4. A4 may implement the narrow signalling/finality observability needed by this state machine, but may not substitute timer polling, unbounded reconnect spinning, a reaper-created minimum connection or active-path connecting acquire without architecture supersession.

### Active pass

Active work calls `Pool::try_begin()` or an exact equivalent that uses only `try_acquire()`. `None` means fail closed/unavailable for that pass and emits the coalesced ready-miss demand above; it never creates a connection, waits for root recovery inside the pass, or enters connect backoff.

One absolute two-second pass deadline covers:

- ready-only checkout/transaction begin;
- executor/custody fence;
- relation/advisory locks;
- every SQL await;
- COMMIT or reconciliation work belonging to that pass.

Server-side `lock_timeout`/`statement_timeout` may be derived from remaining time, but timeout is a cleanup aid, not rollback/finality proof.

## 8. Narrow owner-aware PgPool seam

Current SQLx pool creation calls ordinary `ConnectOptions::connect()`; the existing #356 owner-aware direct `PgConnection::establish_with_resource_budget()` does not automatically fund pool-created connections.

A4 may therefore implement **one narrow PostgreSQL-specific root-owner seam**:

- a dedicated/internal `PgConnectOptions` profile carries the same root `ResourceBudget` identity;
- when that exact profile is present, `PgConnection` establishment delegates to the existing owner-aware connection/TLS path;
- ordinary SQLx `PgConnectOptions` and ordinary pool behavior remain unchanged;
- the seam must reserve before the controlled connection/TLS allocations, not in `after_connect`;
- the owner carried by pool connection creation is the process/root owner, not an active-slot owner;
- no generic new SQLx-wide budget API or second ledger is authorized.

This is the main reason B remains maintainable: it preserves the existing consumer transaction model while adding only the root ownership capability that ordinary pooling lacks.

## 9. Explicit bounded configuration profile

Production WP3 must not parse a generic ambient PostgreSQL URL directly into ordinary `PgConnectOptions`.

Exact source proves ordinary SQLx options may read:

- `PGPORT`;
- `PGHOSTADDR` / `PGHOST`;
- `PGUSER` or OS username fallback;
- `PGDATABASE`;
- `PGPASSWORD`;
- `PGSSLROOTCERT`, `PGSSLCERT`, `PGSSLKEY`, `PGSSLMODE`;
- `PGAPPNAME`, `PGOPTIONS`;
- `PGPASSFILE` or default `.pgpass`.

The pinned `.pgpass` reader grows a `String` with `read_line()` without a hard line-size bound and logs the entire malformed line. This ambient profile is incompatible with the accepted finite/redacted boundary.

First-slice production configuration is therefore an **Oteryn-owned explicit struct**, not libpq ambient discovery. It contains only:

- literal TCP transport address (`IpAddr` + port);
- separate DNS-format TLS server identity;
- database name;
- username;
- password/secret material;
- explicit bounded in-memory root CA PEM;
- fixed statement-cache/profile options selected by this decision.

No production `PG*` inheritance, OS username fallback, `.pgpass`, arbitrary `options`, arbitrary application name, UDS discovery, client certificate/key or implicit root-file discovery is admitted in the first slice.

All retained configuration bytes/capacities are reserved against the same root before copy and stay charged for their actual lifetime. No independent config allowance is created.

A minimal no-ambient `PgConnectOptions` constructor/profile is authorized if the pinned public API cannot express this without first consulting ambient sources.

## 10. Transport, TLS and provider profile

Freeze the first-slice PostgreSQL transport/security profile to:

```text
transport: TCP to explicit literal IP address
DNS resolution: not used by the DB transport path
TLS identity: separate explicit DNS server name
PgSslMode: VerifyFull only
TLS protocol: TLS 1.3 only
provider: rustls 0.23.43 + aws-lc-rs 1.18.0 + aws-lc-sys 0.44.0
provider mode: default non-FIPS
qualified target: x86_64-unknown-linux-gnu
Rust: 1.94
SQLx: 0.9.0
Tokio: 1.53.1
KX policy: protected PQ-first AWS-LC profile from #451/#453/#458
resumption: disabled for the first slice unless separately source-bounded
certificate compression: not admitted unless exact reachability/bounds are separately proven
client certificate authentication: not admitted in first slice
```

### Why AWS-LC supersedes the current root ring profile

Current root `Cargo.toml` selects `tls-rustls-ring-webpki`. Exact #356 source shows its owner-aware TLS function explicitly fails when the AWS-LC SQLx feature is absent: the qualified owner-aware provider path is AWS-LC-specific.

Protected PR #451 provides finite same-root provider/KX architecture for the exact AWS-LC target. #453 protects owner-aware provider-config allocation. #458 protects the `prefer-post-quantum` feature-edge allocation. The exact current #356 vendored SQLx manifest already contains that feature edge in its AWS-LC profile.

Therefore the first-slice WP3-v2 candidate selects the **AWS-LC SQLx TLS feature profile** for the final Game Server consumer rather than extending a second ring ownership design.

This is a profile selection, not a claim that the current protected Game Server already uses AWS-LC. A4 must change the exact production feature graph only after Gate 1 allocation and prove that graph on the final candidate.

### TLS1.3-only seam

Pinned rustls is compiled with TLS1.2 support and ordinary `with_safe_default_protocol_versions()` would leave TLS1.2 reachable. To keep the selected proof surface exact, A4 may make the owner-aware PostgreSQL TLS profile build its `ClientConfig` with TLS1.3 only. Ordinary rustls/SQLx behavior stays unchanged.

The existing real AWS-LC helper already demonstrates PostgreSQL 17.6 + `VerifyFull` + negotiated TLS1.3; this is retained evidence, not final product qualification.

### #455 disposition

PR #455 is closed/unmerged, so its proposed `208`-byte correction is historical evidence only. Exact current #356 source compiles the shared KX source for both provider instances, asserts `200 | 208`, and the owner-aware AWS-LC KX path separately requires the 200-byte AWS-LC instance before applying the protected #451 bounds.

A4 must nevertheless re-run exact root-graph layout assertions on the final production feature graph. Any proof that the owner-aware AWS-LC instance itself no longer matches protected #451 layout invalidates the bound and requires architecture escalation; do not silently substitute #455 numbers.

## 11. PostgreSQL startup/authentication profile

Pinned SQLx source supports these authentication branches:

- `AuthenticationOk`;
- CleartextPassword;
- MD5Password;
- SASL;
- all other methods are rejected.

Pinned SASL code implements the non-channel-binding `SCRAM-SHA-256` exchange. It recognizes the `SCRAM-SHA-256-PLUS` name but sends `plus: false`; therefore PLUS/channel binding is **not** qualified and must not be claimed.

First-slice production authentication is frozen to:

```text
SCRAM-SHA-256 only
no trust/passwordless AuthenticationOk path
no CleartextPassword
no MD5Password
no SCRAM-SHA-256-PLUS claim
```

A narrow owner-aware PostgreSQL auth-profile seam may reject every non-SCRAM challenge and require successful SCRAM before `ReadyForQuery`. Ordinary SQLx behavior remains unchanged.

The SCRAM work stays under the root connect deadline. Server-selected excessive work cannot consume an active DFR slot; the five-second root connect deadline remains authoritative for the attempt.

`BackendKeyData`, startup/status/error messages and all retained authentication backing remain part of the owning `T` or `R` generation until their actual finality.

## 12. SQL corpus closure

Exact call-site census at #335:

| File | `sqlx::query*` call-sites | Disposition |
|---|---:|---|
| `admission_journal.rs` | 33 | retain fixed/exact families; repair three unbounded collection/value families below |
| `mod.rs` | 22 | retain fixed/exact families; repair one unguarded `record_json` family; custody/takeover moves under ready-only/root model |
| `fresh_admission.rs` | 10 | retain; several variable payload reads already use same-snapshot `octet_length` guards |
| `admission_authority_guards.rs` | 4 | retain; bounded codec + exact history/current writes |
| **base total** | **69** | complete production call-site census for these files |

Additional source-shape families:

- one dynamic advisory-lock SQL family;
- two guard `QueryBuilder` families over four closed variants (conservative +8 shapes);
- current 15 relation-lock statement shapes;
- one schema-inspection family.

Conservative source upper bound remains 95 statement shapes. Consolidating relation locks from 15 statements to one static multi-table statement reduces this to 81, below the pinned statement-cache capacity 100. This is a **count bound**, not a metadata-byte bound.

### Required consumer repairs

1. **Schema migration ledger:** replace unbounded `fetch_all()` with embedded migration count `N + 1` sentinel coverage and exact checksum-length validation before client materialization.
2. **Pending commands:** both child-row `fetch_all()` families become one bounded ordered aggregate per exact attempt, preserving accepted maximum 64 commands and full identity/disposition comparison.
3. **Active committed binding:** replace vector materialization followed by `len()==1` with `LIMIT 2` / exact-cardinality bounded transfer.
4. **Reconnect `record_json`:** every unguarded `SELECT state, record_json` family, including the V2 terminal-state read, gets same-snapshot logical-byte guarding before transfer.
5. **Errors:** normalize full `sqlx::Error` inside active custody into bounded Durability classification/correlation data; do not retain or log unbounded peer/config prose after release.
6. **Dependency logging:** configure/fix only the exact SQLx logging sinks that can emit unbounded peer/config content; no generic logging fork.
7. **Relation locks:** replace 15 separate fixed `LOCK TABLE ... IN EXCLUSIVE MODE` awaits with one static multi-table statement preserving the same lexical relation order and mode.
8. **Statement/type profile:** retain statement cache capacity 100 initially; first slice admits built-in PostgreSQL types only and no generic custom-type/table-origin discovery unless separately bounded.

All other inspected production query calls are fixed-cardinality scalar/optional reads or writes/updates whose collection cardinality is not an unbounded `fetch_all` family. They remain subject to ordinary result-byte, metadata, deadline and error-lifetime qualification.

## 13. Exact lock footprint

Current common relation fence covers exactly **15 relation classes** in lexical order, all `EXCLUSIVE`.

Current V1/V2 domain lock helper may add these logical advisory roots:

- account;
- character;
- session;
- transport;
- attempt;
- epoch;
- runtime scope;
- optional recovery nonce.

Maximum domain roots there: **8**.

Every normal registered backend transaction first obtains the shared executor-custody advisory fence. Therefore the maximum exact current advisory-root footprint for that path is:

```text
1 executor shared fence + 8 domain roots = 9 logical advisory roots
```

Executor takeover uses:

```text
1 executor exclusive advisory root + 15 relation classes
```

Fresh-admission/lifecycle paths use the shared executor fence plus the same 15-relation fence and do not add the V1/V2 domain set in the inspected first-slice path.

This fits the accepted DFR maxima `64 logical keys / 16 relation classes` without inventing capacity.

Lock order remains:

```text
executor custody fence
-> 15 relations in current lexical order (one static SQL statement after repair)
-> sorted/deduplicated domain advisory roots where the operation requires them
-> exact row/FK/unique/index work under the accepted transaction protocol
```

Any new relation or independently required lock root must be added to the exact inventory and requalified before use.

## 14. PostgreSQL receive/resident minimal seams

Consumer/query repairs do not close every peer-controlled driver allocation. Minimal PostgreSQL seams remain justified for the frozen profile:

- backend frame-length gate before receive-buffer reserve;
- DataRow count/profile gate before vector allocation;
- RowDescription count/name/profile gate before allocation;
- bounded/allowlisted retained `ParameterStatus` count/bytes;
- finite selected statement/type/table cache behavior;
- prospective socket-buffer growth checks where root reservation cannot prove them externally;
- exact transport-address vs TLS-server-name separation;
- only the cleanup/high-water observability needed to prove `R`/`T` retirement finality and the non-overlap gate.

The exact source-visible settled connection lower bound remains at least 16 KiB from initial SQLx read/write socket backing before TLS/reactor/cache state. That is a lower bound, not complete lifecycle `R`.

## 15. Cancellation, rollback and ambiguous COMMIT

- no automatic retry inside an active pass;
- transaction drop only queues rollback and preserves custody;
- lost response after COMMIT is `AMBIGUOUS`, never inferred rollback;
- reconciliation uses the original immutable operation/replay identity;
- completion/cleanup/ambiguity capacity is funded before irreversible COMMIT;
- unusable connection is fenced/retired under root ownership and remains `R` until finality;
- root maintenance may later restore readiness only through the frozen demand-triggered recovery state machine and a new bounded `T` generation after prior tails are final;
- any later semantic retry is a new authorized operation/reconciliation action, never hidden dependency retry.

## 16. Restart/takeover and predecessor fencing

The executor keeps one durable generation/fence and two durable pending custody slots.

On startup/takeover:

1. emit the startup/takeover `root_ready_demand` and obtain the ready physical connection through root maintenance;
2. acquire the exclusive executor custody advisory fence;
3. acquire the same 15-relation fence;
4. validate/increment exact executor generation;
5. load exactly the two bounded pending slots under existing same-snapshot payload guards;
6. reconstruct at most two logical active identities without creating new capacity;
7. reconcile predecessor work by original operation identity before replacement effect;
8. fence stale predecessor generation/process work before accepting new semantic work;
9. retire durable pending + active + recovered in-memory copies only through the definitive release/ack protocol.

The current permanent fail-closed `RuntimeRegistration::Starting` behavior after uncertain initialization is safe as an intermediate state but is not terminal operational recovery. A4 must implement the frozen root-demand transition above plus one reviewed takeover/recovery transition without blind reset, duplicate capacity or an autonomous reconnect loop.

## 17. Resource terms and proof responsibility

Architecture freezes ownership **and overlap**, not guessed numbers.

### PROVEN / accepted architecture semantics

- `Q <= 4 MiB` under DFR queue rules;
- `A <= 8 MiB` under two logical active slots;
- at most one established-or-retiring `R` generation;
- at most one connect-or-failed-retiring `T` generation;
- `R` and `T` never overlap: a new `T` is gated on full finality of every prior `R`/`T` descendant, and successful establishment transfers ownership `T -> R`;
- connection-attributable retirement tails remain in their owning `R`/`T` generation; genuinely shared process-lifetime runtime/provider/config backing is in `I`;
- AWS-LC protected provider-residency/KX accounting model for the exact target;
- source-visible SQLx socket backing lower bound `R >= 16,384 B` before TLS/reactor/cache state.

### UNKNOWN until exact A4 candidate

- complete `I` byte value;
- complete lifecycle `R` byte value, including all established-connection retirement descendants;
- complete lifecycle `T` byte value for the selected TLS1.3/SCRAM/config profile, including failed/timed-out attempt retirement descendants;
- exact active SQL peak after query/executor repairs;
- exact bytes of any connection-attributable deferred Tokio reactor retirement inside the owning `R`/`T` generation;
- exact statement/type/status metadata bytes for the final corpus.

A4 must prove phase entry, the frozen non-overlap gate, escaping descendants, re-entry, `T -> R` success transfer and every failure/cancellation/retirement exit. It may measure the unknown byte values, but it may not decide a different overlap or tail-classification model. Measurement corroborates source/lifetime proof; it does not replace it.

If the exact candidate cannot satisfy:

```text
I + max(R, T) + Q + A <= 12 MiB
```

under those frozen semantics, it is a hard architecture/resource failure, not permission to overlap generations, add a retirement exemption, add another budget or weaken security/DFR semantics.

## 18. Q01-Q75 disposition

The canonical `WP3-Q01..WP3-Q75` matrix from PR #588 remains binding and is not replaced by this decision.

The final exact consumer must terminally classify every applicable cell. Revision-4 Q76-Q84 refinements remain additional obligations where applicable but do not renumber or replace Q01-Q75.

Selected-profile consequences include:

- DNS and UDS production cells become exact-unreachable claims that require graph/config proof, not `N/A` by assertion;
- TLS1.2 cells become exact-unreachable only after the TLS1.3-only owner-aware config is proven on the final graph;
- AWS-LC provider cold/warm/thread-cohort and PQ-first group behavior remain applicable;
- Q47-Q54 pool/bootstrap/finality cells are exercised on max1/min0/lazy/finite-retirement policy, including `R/T` tail non-overlap and demand-triggered recovery;
- Q56/Q57 prove sequential active-slot reuse and two simultaneous logical ambiguous obligations despite one physical DB pass;
- Q71/Q72 close exact SQL corpus and lock inventory;
- Q73-Q75 prove producer-to-completion custody, restart/takeover and retained startup/config lifetime.

`Q01-Q75 green` means exact terminal evidence on the final composed consumer, not document completion, local-only PASS or skipped tests.

## 19. #356 retention/supersession map

#356 remains the canonical research/evidence lineage until replacement proof is integrated.

### RETAIN

- `ResourceBudget` / `ResourceReservation` primitives;
- exact backing/finality primitives still needed by the selected profile;
- protected AWS-LC provider/KX work and applicable #356 implementation/tests;
- PostgreSQL 17.6 / real TLS harness;
- hostile frame/count/denial vectors;
- provenance/source census;
- valid socket/reactor/runtime finality research;
- owner-aware direct establishment primitives reused by the root-owned pool seam.

### REWORK / NARROW

- SQLx core/postgres forks to the exact root-owner, receive/count/status/cache/finality seams above;
- root Game Server TLS feature from current ring profile to the accepted AWS-LC first-slice profile after Gate 1 allocation;
- pool/bootstrap path to lazy max1/min0 + root maintenance + active `try_begin` plus the frozen coalesced root-demand signal/finality gate;
- config/auth/TLS setup to no-ambient + VerifyFull/TLS1.3/SCRAM-only profile.

### HISTORICAL EVIDENCE ONLY / REMOVE AFTER REPLACEMENT PROOF

- generic Tokio blocking-owner propagation not required by the frozen profile;
- broad rustls container ownership beyond retained AWS-LC exact seams;
- generic DNS/UDS ownership work excluded by the literal-TCP first slice;
- per-operation direct-connect architecture assumptions superseded by root ownership;
- closed/unmerged PR #455 correction as authority (retain only as historical investigation).

### SUPERSEDED

- #356 broad operation-owned direct connection as terminal WP3 architecture;
- earlier `min=2/max=2` or generic two-connection first-slice recommendation;
- any assumption that caller timeout, transaction drop, `after_release`, pool wrapper drop or SQLx close alone proves finality;
- any implementation choice that overlaps a new connect transient with a prior connection/attempt retirement tail under the unchanged root equation;
- timer/polling/unbounded reconnect policy substituted for the frozen demand-triggered recovery model;
- current production ring profile as the final WP3-v2 first-slice TLS provider choice, once this decision is accepted and A4 is allocated.

No vendor patch is deleted before the replacement consumer is independently qualified.

## 20. Cross-repository/security boundaries

This decision changes no Platform authority.

R01-R21 remain qualification constraints, including:

- PostgreSQL TLS, gameplay TLS, Platform native-source mTLS and client HTTPS are separate profiles;
- source freshness is checked at final authority use after queue/transport/DB waiting;
- possible DFR/native-source wait cycles require an explicit resource/wait graph and saturation test;
- one DB connection is a first-slice architecture choice, not a throughput conclusion;
- final S3/G0 evidence binds exact compatible Game + Platform heads and real authenticated transport.

## 21. Candidate acceptance state

Revision 3 changes only the two independent-review findings against the prior exact candidate: P1 freezes complete `I/R/T` overlap/retirement-tail semantics so the root equation is complete, and P2 freezes the exact demand-triggered root-maintenance recovery state machine after reaper absence or a failed connect window. No runtime or broader architecture scope is added.

Current terminal worker marker:

```text
WP3_V2_ARCHITECTURE_SUCCESSOR_VALIDATING
ARCHITECTURE_ACCEPTED = NO
IMPLEMENTATION_AUTHORITY = NONE
```

Still required before material A4 work:

1. fresh exact-head repository/governance checks on the Revision-3 successor head;
2. genuinely independent exact-head HIGH-risk architecture/resource/security re-review of that successor, because the accepted P1 repair supersedes the reviewed generation;
3. normal repository architecture acceptance/protected integration/readback only after that successor is clean;
4. fresh #162/#364 allocation identifying canonical A4 lineage and exact owned paths/custody.

The remaining `I/R/T/active` **byte values** are implementation qualification obligations. Their lifecycle ownership, retirement classification, non-overlap and recovery trigger are no longer A4 architecture choices. If the measured exact candidate fails the frozen root equation, A4 must stop and escalate rather than changing those semantics or widening the envelope.

This file performs no runtime, Cargo, vendor, SQL/migration, workflow, Platform, production, secret, merge or Merge Queue mutation.