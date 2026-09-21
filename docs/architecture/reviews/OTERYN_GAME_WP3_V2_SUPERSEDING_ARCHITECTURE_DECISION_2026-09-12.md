# Oteryn Game — WP3-v2 superseding architecture decision

> **Accounting supersession:** Owner decision #162 comment `5759734485`, recorded
> in
> [`OTERYN_GAME_WP3_TLS_EXTERNAL_SUBRESOURCE_ACCOUNTING_AMENDMENT_2026-09-21.md`](OTERYN_GAME_WP3_TLS_EXTERNAL_SUBRESOURCE_ACCOUNTING_AMENDMENT_2026-09-21.md),
> supersedes only this decision's dependency-internal byte-perfect TLS/socket
> accounting clauses. Read the 12 MiB equation as applying to the controlled
> graph defined by that amendment; all other topology, lifetime, security,
> finality, hostile-input, and deadline requirements remain in force.

- Decision ID: `WP3-V2-ROOT-OWNED-BOUNDED-PGPOOL-V1`
- Revision: **5 — P1-A finite dedicated Tokio topology and complete root-runtime backing closure**
- Date: 2026-09-12
- Revision-4 amendment date: 2026-09-19
- Revision-5 amendment date: 2026-09-20
- Status: **CANDIDATE AMENDMENT / EXACT-HEAD VALIDATION / NOT YET PROTECTED**
- Worker: `Oteryn: astra wp3-v2 architecture lead`
- Protected admission: `main@489e3e390a1bce1ce3439c66521ab75f8a826cd8`
- Canonical WP3 lineage: Issue #351 / Draft PR #356 @ `fe7891989b1247012e32c89c10cff6a10bacb943`
- Canonical Child B lineage: Issue #329 / PR #335 @ `834db1d7118d751e31287715d3eaac7780a0c7b9`
- Programme: #162 / #364
- Retained audit: Draft PR #588
- WP3-v2 programme: Draft PR #589
- This candidate: Draft PR #590
- Revision-4 amendment authority: #162 comment `5745227522`, superseding the insufficient layout-only release `5744804452` and closing the source-proof gaps recorded in `5744812736`.
- Revision-5 finite-topology owner authority: #162 comment `5748461271`, consuming architecture qualification `5748151621`, host evidence `5748332099`, and control-plane packet `5748378407`.
- Revision-4/5 amendment branch: `agent/wp3-v2-tokio-maintenance-task-layout-amendment-351`; Revision 5 continues the same PR #681 lineage and does not create a competing decision.
- Implementation authority: **none from this file or amendment branch**. PR #673 remains paused until protected amendment integration/readback and a fresh coordinator source/vendor/Cargo lease.

## 1. Resolution

Select **Option B** for the first safe production slice:

> one process-scoped logical Durability executor, one accepted DFR root ledger, one **WP3-owned dedicated Tokio 1.53.1 MultiThread runtime with an exact finite 1-worker / 1-blocking-thread / 2-MiB-stack topology**, one lazy SQLx `PgPool` used as a **single-ready-connection holder**, root-owned serialized connection establishment outside active DFR work, `Pool::try_begin()` for active ready-only transactions, two logical active custody slots, at most one physical PostgreSQL transaction at a time, and only the narrow SQLx/PostgreSQL/rustls/Tokio representation seams required to prove the frozen same-root envelope.

First-slice topology:

```text
one Durability executor / one DFR root
|
+-- I: fixed executor/runtime/pool/provider/config/shared-tail residency
|   +-- WP3-owned Tokio 1.53.1 MultiThread runtime
|   +-- worker_threads = 1
|   +-- max_blocking_threads = 1
|   +-- thread_stack_size = 2 MiB
|   +-- only the I/O/time capabilities required by the frozen literal-IP PostgreSQL slice
|   +-- arbitrary ambient Handle/runtime qualification forbidden
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

- `I` is root/process backing whose lifetime is independent of one physical connection generation: executor/pool control structures, explicit retained configuration, genuinely shared runtime/provider backing, and all source-derived dedicated-runtime plus SQLx root-maintenance task/shared-tail backing admitted by Revisions 4 and 5. Per-connection socket/TLS/driver/reactor descendants do **not** migrate into `I` merely because their cleanup is asynchronous.
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

- fixed executor structures plus the complete attributable backing of the dedicated WP3 Tokio runtime selected by Revision 5, including its one-worker scheduler topology and every reachable retained runtime allocation that must exist for the accepted first slice;
- the explicit `2 MiB` worker-stack setting is a finite topology contract, not permission to equate process RSS with DFR accounting; exact implementation qualification must conservatively account the applicable stack/runtime backing under the accepted same-root `I` rules and may prove an unreachable blocking-worker stack only through exact reachability evidence, never by assumption;
- pool structures and maintenance task state, including the source-derived optional boxed-future/task-cell backing for the exact SQLx root-maintenance task;
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

## 8A. Revision-5 P1-A finite dedicated-runtime amendment

### Problem and timing

**Must decide now? YES.** Revision 4 closed the pre-spawn
`CloseEvent/EventListener` allocation and current-thread queue-growth gaps, but the
independent HIGH review on PR #681 head
`9b5c4717b59955b8a340d8bbf9d8ec707ddf5d66` found one remaining material P1:
`RuntimeFlavor::MultiThread` bounds scheduler kind, not worker count, worker stacks,
or the complete scheduler/runtime backing consumed through an ambient
`Handle`.

Pinned Tokio `1.53.1` source proves that an unspecified MultiThread builder derives
worker count from ambient host/environment state, leaves thread stack size
unspecified, and carries a separately configurable blocking-thread cap. The
owner-authorized host probe in #162 comment `5748332099` independently observed
that host/config dependence and showed the proposed bounded builder remains at one
worker even under `TOKIO_WORKER_THREADS=16`. That probe is supporting evidence,
not the final root-byte proof.

Revision 5 therefore consumes the explicit owner decision in #162 comment
`5748461271`. It does **not** reopen Options A/B/C and does not select a Game-wide
Tokio topology.

### Exact production topology

The first-slice WP3 Durability root owns one dedicated Tokio `1.53.1` runtime with
this exact topology:

```text
RuntimeFlavor = MultiThread
worker_threads = 1
max_blocking_threads = 1
thread_stack_size = 2 MiB
runtime owner = WP3 Durability root
ambient Handle qualification = forbidden
```

Only the I/O/time capabilities already required by the frozen literal-IP
PostgreSQL slice may be enabled. This is a WP3-local ownership decision, not a
general FND-03/GameNode worker-count decision.

Production root acceptance must fail closed when:
- the dedicated runtime cannot be created with that exact topology;
- runtime flavor is unavailable or is not `MultiThread`;
- production code attempts to substitute an arbitrary ambient Tokio `Handle`;
- exact prospective resource proof cannot fit the existing same-root envelope.

`CurrentThread` remains test-only/non-production.

### Dedicated-runtime custody

All production WP3 root-owned spawned work that is part of the accepted Durability
root lifecycle must be tied to the explicit root-owned runtime/handle. An ambient
`tokio::spawn`, `Handle::current()`, or `Handle::try_current()` may not silently
select production scheduler ownership for that work.

SQLx pool construction/maintenance for the accepted root must therefore occur
under the explicit dedicated-root runtime context/handle through the smallest
implementation seam that preserves upstream Tokio semantics. This requirement
does not authorize a global runtime wrapper, generic owner propagation, or a second
work-ownership API.

### Exact pinned-Tokio representation/accounting seam

Pinned Tokio `1.53.1` may expose/read exact source-derived representation facts
needed to prove the **selected dedicated runtime plus the exact SQLx root-maintenance
task**, and nothing broader.

The proof surface is limited to backing attributable to the exact accepted
configuration, including as applicable:

1. the one-worker MultiThread scheduler structures and fixed worker-local run queue;
2. exact retained per-worker/shared scheduler structures, remotes/metrics/control
   backing and runtime-owned driver/control structures that exist for the enabled
   first-slice feature set;
3. the existing `BOX_FUTURE_THRESHOLD` optional future box for the exact SQLx
   maintenance future;
4. the exact MultiThread task-cell representation allocated for that future;
5. the configured worker stack bound and any blocking-pool backing actually
   reachable by the selected first-slice graph.

The representation seam supplies facts only. It creates no allowance, allocator
interception, owner identity, generic task/scheduler API, second ledger, retry
policy or lifetime policy.

No literal scheduler/task byte constant may replace source-derived proof. Any
candidate-specific size conversion must use the existing accepted
allocator/backing-charge rule.

### Blocking-thread and stack reachability

`max_blocking_threads = 1` is the smallest finite accepted cap and prevents an
ambient 512-thread default from remaining part of the topology.

The first slice does not gain permission to call `spawn_blocking` merely because a
cap exists. Qualification must prove either:
- no blocking worker is reachable in the exact WP3 root graph, in which case no
  blocking-worker stack may be silently materialized or charged as active backing;
  or
- one blocking worker is genuinely reachable/required, in which case its complete
  attributable backing must be included in the same-root `I` proof before root
  acceptance.

The explicit `thread_stack_size = 2 MiB` removes `RUST_MIN_STACK`/platform
selection from the WP3 contract. Virtual stack reservation and resident/committed
bytes must remain distinguished in evidence; process RSS/working-set measurement
is not a substitute for the frozen DFR accounting model.

### Root-specific SQLx maintenance construction

SQLx `0.9.0` retains the Revision-4 authority for one root-specific maintenance
construction that removes the heap-allocating `CloseEvent/EventListener`
dependency from this WP3 maintenance future before Tokio task allocation.

It must preserve:

- `idle_timeout = 10 minutes`;
- `max_lifetime = 30 minutes`;
- `max_connections = 1`;
- `min_connections = 0`;
- ready-only active work;
- no reaper-created replacement connection;
- the existing coalesced demand/recovery/finality state machine.

### Same-root `I` reservation and finality

Before production root acceptance and before any selected runtime/task backing is
created, Game must derive the finite prospective charge for all attributable
dedicated-runtime and maintenance backing admitted above and reserve it against the
**same existing root `I` ledger**.

The accepted maximum remains unchanged:

```text
DFR-TOTAL-RESIDENT-BYTES = 12,582,912
I + max(R,T) + Q + A <= 12 MiB
```

There is:
- no second runtime/task budget;
- no resource-registry increase;
- no early release merely because work is cancelled, a pool becomes empty, or a
  maintenance future is logically complete while retained descendants remain;
- no transfer of scheduler backing outside `I` to make the equation fit.

If exact qualification cannot fit the frozen equation, root creation/acceptance
fails closed and WP3 returns to architecture escalation. Implementation may not
raise the budget.

### Required successor proof

A later coordinator-issued PR #673 implementation lease must prove on one exact
candidate:

1. the production root creates/owns exactly the accepted
   `1 worker / 1 blocking / 2 MiB` MultiThread topology and rejects ambient
   substitution;
2. all production WP3 root-owned spawning uses the explicit root runtime/handle;
3. the exact dedicated-runtime scheduler/control backing and exact SQLx maintenance
   future/task backing are source-derived and prospectively reserved in the same
   root `I` ledger before acceptance/allocation;
4. blocking-thread reachability is explicitly proven rather than assumed;
5. the root-specific SQLx maintenance path does not allocate
   `CloseEvent/EventListener` backing for this task;
6. max/max+1 same-root reservation boundaries reject before root/maintenance
   allocation;
7. reservation lifetime covers complete runtime/task/shared-tail finality;
8. 10-minute idle / 30-minute max-lifetime reaping, max1/min0, ready-only activity,
   strict R/T non-overlap and root demand/recovery/finality semantics remain
   unchanged;
9. configured PostgreSQL, caller-cancellation/finality, focused resource tests,
   workspace tests, strict Clippy, governance, exact-head CI and the required
   independent HIGH whole-diff review requalify the final successor.

### Explicit exclusions

Revision 5 does not authorize:
- a Game-wide Tokio/FND-03 worker topology;
- generic Tokio allocator/task/scheduler/resource-owner instrumentation;
- a broad Tokio fork;
- a second allocator/runtime budget;
- rustls widening;
- WP4/WP5/Server-Seam or `fresh_admission.rs` custody;
- any increase to the accepted DFR maximum.

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

### 9.1 Retained-config hard limits — P1-2 owner amendment (2026-09-19)

This amendment closes only the retained-configuration authority gap identified by the
independent HIGH review of WP3-A PR #673. It does not change the selected root/pool
architecture, add a second allowance, or increase the accepted DFR total.

The first-slice production profile fixes these **input-byte** hard maxima:

| retained field | hard maximum | unit / grammar | owner evidence |
| --- | ---: | --- | --- |
| `tls_server_name` | **253** | ASCII bytes in DNS presentation form; each label 1–63 octets, no trailing root dot | RFC 1035 §2.3.4 caps a DNS wire name at 255 octets; the uncompressed presentation form is two octets shorter because dots replace label-length octets and the terminal root byte is omitted |
| `database` | **63** | UTF-8 bytes | PostgreSQL 17 default `max_identifier_length=63` bytes; the selected first-slice target is PostgreSQL 17.6 and does not rely on a recompiled larger `NAMEDATALEN` |
| `username` | **63** | UTF-8 bytes | same PostgreSQL identifier bound; role names are not allowed to rely on silent server truncation |
| `password` | **1,024** | UTF-8 bytes, non-empty | first-slice Oteryn product ceiling. PostgreSQL 17 SCRAM does not publish a smaller password-length contract; protected Oteryn Platform password ingress already applies a 1,024-unit string ceiling (for example `Oteryn/Oteryn-Platform@623435ec1b907d6d9770b767806c90300252a71c:app/Http/Requests/Identity/LoginIdentityRequest.php`). This decision deliberately adopts the stricter allocation unit **UTF-8 bytes** for the DB secret; it does not inherit Platform authority or claim a PostgreSQL universal limit |
| `root_ca_pem` | **22,768** | ASCII bytes; 1–4 RFC-7468 `CERTIFICATE` blocks, LF or CRLF only, no unrelated text; each decoded certificate <=4,096 DER bytes and aggregate DER <=16,384 bytes | adopts, by this owner decision, the already-protected Oteryn conservative PKI envelope `NSRC-TLS-CERTS` (4 roots, 4,096 DER bytes each) for this separate PostgreSQL trust input. RFC 7468's 64-character generated base64 lines give 5,464 base64 characters and 86 lines for 4,096 DER bytes; worst-case CRLF canonical text is 5,692 bytes per certificate, hence 4 × 5,692 = 22,768 bytes |

The five variable fields therefore have one checked aggregate input ceiling:

```text
253 + 63 + 63 + 1,024 + 22,768 = 24,171 bytes
```

`DFR-PG-RETAINED-CONFIG-BYTES = 24,171` is a **conjunctive input bound**,
not another memory budget. The fixed `IpAddr`/port/profile fields, struct backing,
allocator capacity/metadata and every retained clone still count in `I` through
their actual qualified resident charge.

The implementation boundary is fail-closed:

1. inspect length/grammar from borrowed or otherwise already-bounded source bytes;
2. reject any field above its hard maximum and use checked addition to reject an
   aggregate above 24,171 **before** an owned `String`/`Vec`, PEM parse, or
   retained copy is created for this configuration;
3. only after those checks, reserve the actual retained capacity against the same
   root `I` ledger and create the owned backing; allocated capacity may not exceed
   what was reserved;
4. for `root_ca_pem`, enforce the raw 22,768-byte bound before decoding, then
   enforce certificate count, per-certificate DER and aggregate DER bounds inside
   that already-bounded input;
5. retain the charge through the last configuration-dependent owner and release it
   only when the backing is actually final.

A constructor that first accepts arbitrary already-owned `String`/`Vec<u8>`
values and only then checks their lengths does **not** satisfy this boundary.

Exceeding any of these internal configuration bounds maps to the existing DFR
failure category `UNAVAILABLE` and is not client-visible. There is no truncation,
fallback to ambient/libpq configuration, secret logging, or partial root creation.

Qualification must include, for every field, an exact-maximum and first-byte-above
case, plus checked aggregate 24,171 / 24,172 cases and arithmetic-overflow denial.
The CA tests additionally cover fifth-root, 4,097-byte decoded certificate,
16,385-byte aggregate DER, malformed/noncanonical text and both LF/CRLF accepted
canonical encodings. Where a semantic fixture cannot realize an exact byte size,
the generic pre-copy length comparison still tests the exact maximum/max+1 and
the largest valid representable value at or below the maximum is tested
semantically.

These maxima do **not** change `DFR-TOTAL-RESIDENT-BYTES = 12,582,912`, the
`I + max(R,T) + Q + A <= 12 MiB` equation, or the strict R/T non-overlap rule.
If exact implementation evidence cannot reserve/retain the selected representation
inside that existing root equation, A4 must escalate; it may not enlarge these
limits or mint another allowance.

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
- ParameterDescription count/profile gate before SmallVec allocation;
- RowDescription count/name/profile gate before allocation;
- bounded/allowlisted retained `ParameterStatus` count/bytes;
- finite selected statement/type/table cache behavior;
- prospective socket-buffer growth checks where root reservation cannot prove them externally;
- exact transport-address vs TLS-server-name separation;
- only the cleanup/high-water observability needed to prove `R`/`T` retirement finality and the non-overlap gate.

The exact source-visible settled connection lower bound remains at least 16 KiB from initial SQLx read/write socket backing before TLS/reactor/cache state. That is a lower bound, not complete lifecycle `R`.


### 14.1 PostgreSQL first-slice receive/resident hard profile — owner amendment (2026-09-20)

This amendment closes the missing externally-controlled profile authority for
`RC-WP3-002-POSTGRES-FIRST-SLICE-RECEIVE-RESIDENT-PROFILE`. It is one
indivisible first-slice contract. It does not create another DFR budget, increase
`DFR-TOTAL-RESIDENT-BYTES = 12,582,912`, or change
`I + max(R,T) + Q + A <= 12 MiB`.

The production root profile fixes these hard boundaries:

| boundary | hard rule | first-slice meaning |
| --- | ---: | --- |
| backend PostgreSQL message | **131,207 total wire bytes** | includes the one-byte backend type, four-byte PostgreSQL length field and body; therefore the announced length field is <= **131,206** and body is <= **131,202** |
| DataRow fields | **32** | reject count 33 before vector allocation |
| ParameterDescription parameters | **32** | reject count 33 before SmallVec allocation |
| RowDescription fields | **32** | reject count 33 before vector or field-name allocation |
| RowDescription single field name | **63 UTF-8 bytes** excluding NUL | selected stock PostgreSQL 17.6 compatibility ceiling, not a protocol-wide invariant |
| RowDescription aggregate field-name bytes | **2,016** excluding NULs | derived as 32 x 63; reject 2,017 before owned field-name allocation |
| retained raw ParameterStatus entries | **0** | no heap-backed raw key/value map; only normalized `server_version_num: Option<u32>` may survive |
| retained raw ParameterStatus variable bytes | **0** | unknown/unneeded statuses are consumed from the bounded message and not retained |
| reusable socket read scratch | **8,192 bytes** | no retained growth on the production root profile; larger admitted messages use separately owned, same-root-charged backing |

The backend-message ceiling is a PostgreSQL-specific first-slice safety and
compatibility cap. It is not `DFR-SQL-RESULT-BYTES` and does not reuse
`FND02-WIRE-FRAME-BYTES`. For the admitted row-bearing representation its
maximum is derived as:

```text
1 type + 4 length + 2 DataRow count + (32 * 4 field-length prefixes)
+ 131,072 admitted row-value bytes
= 131,207 total wire bytes
```

The five-byte backend header is inspected first. A total size of 131,208 bytes
(or an announced length above 131,206) fails closed before any reserve, growth
or read target derived from the peer length. Structural truncation, checked
arithmetic overflow, or a body inconsistent with its message-specific profile
also fails closed.

#### Row-bearing execution profile

```text
POSTGRES_FIRST_SLICE_ROW_RESULTS = PREPARED_BINARY_ONLY
```

Every production root-profile operation that can produce `DataRow` uses the
extended-query Prepare/Bind path with binary result format. A simple/unprepared
text query is admitted only where the production path cannot produce row data.
If `DataRow` is observed on a simple/text root-profile path, the connection
fails closed before row decoding or retention.

This rule is part of the 131,207-byte decision: it prevents the binary-row
derivation from being silently applied to arbitrary text-wire representations.
Qualification must prove the current WP3/WP4 first-slice row-bearing corpus has
no text-result bypass.

#### Decoder and retained-state rules

Before allocating decoded peer-driven collections:

- `DataRow` validates count <=32 and the complete body before
  `Vec::with_capacity`;
- `ParameterDescription` validates count <=32 and exact
  `2 + 4 * count` body structure before `SmallVec::with_capacity`;
- `RowDescription` validates count <=32, each UTF-8 name <=63 bytes,
  aggregate name bytes <=2,016 and complete body structure before vector or
  owned-String allocation.

The 63-byte field-name ceiling belongs specifically to the selected stock
PostgreSQL 17.6 profile. A server built with a larger `NAMEDATALEN` /
`max_identifier_length > 63` is outside this first slice and cannot silently
widen the boundary; supporting it requires a successor owner decision.

`ParameterStatus` keeps no raw heap-backed key/value entries. The existing
`server_version` status may be parsed into `server_version_num: Option<u32>`;
all raw status bytes are then released with the received message. Other
unneeded statuses are ignored after bounded parsing, not accumulated.

The reusable socket read scratch remains at exactly 8,192 retained bytes on the
root profile. An admitted message larger than the scratch is read into separate
backing only after the header gate and after reserving the actual allocation
against the same owning R/T generation. Releasing that message must not leave
the reusable scratch enlarged.

#### Statement/type/table profile

The statement-cache capacity remains **100**. This amendment does not create a
new cache allowance.

The production root profile accepts built-in PostgreSQL OIDs only. Every
parameter or result OID must resolve through the built-in type table; an unknown
OID fails closed before generic type resolution, catalog queries or custom
type-cache insertion. Generic domain/enum/composite/range discovery is not
authorized.

Table-origin discovery is disabled on the production root profile.
`ColumnResolver` catalog lookup is not entered and the generic custom
type/table maps remain empty on this path.

Actual receive-message backing, decoded vectors/strings, statement metadata,
cache backing and socket backing remain conjunctively charged to the existing
same-root R/T accounting before allocation/growth and through final backing
destruction. These hard maxima are validity ceilings, not additional resident
allowances. An otherwise valid maximum still fails closed when the existing
12 MiB root cannot fund its actual representation.

Qualification covers max/max+1 and malformed cases for every boundary above,
configured stock PostgreSQL 17.6 startup/status behavior, built-in-only OID
reachability, no table-origin/cache growth, statement-cache capacity 100,
socket scratch high-water/finality, exact same-root charge lifetime, and the
prepared/binary-only row-bearing corpus.

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

- generic Tokio blocking-owner propagation and generic Tokio owner/allocator/scheduler instrumentation remain historical/not required; only the Revision-5 read-only representation seam for the exact dedicated WP3 runtime plus exact root-maintenance future/task is retained;
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

Revision 5 preserves all Revision-3/4 overlap, finality, demand-recovery and maintenance semantics while closing independent-review P1 `4054801878` through the owner-accepted finite dedicated runtime in #162 comment `5748461271`. The production WP3 root now owns one Tokio 1.53.1 MultiThread runtime with `worker_threads=1`, `max_blocking_threads=1` and `thread_stack_size=2 MiB`; arbitrary ambient runtime ownership is forbidden. The narrow read-only representation seam covers only the exact dedicated runtime backing plus the exact SQLx maintenance future/task required for same-root `I` proof. No second budget, Game-wide topology or implementation authority is created by this amendment.

Current amendment worker marker:

```text
WP3_V2_P1A_FINITE_RUNTIME_AMENDMENT_VALIDATING
BASE_REVISION_3_4_AUTHORITY = PRESERVED
REVISION_5_AMENDMENT_PROTECTED = NO
OWNER_RUNTIME_DECISION = WP3_DEDICATED_BOUNDED_TOKIO_RUNTIME_V1
IMPLEMENTATION_AUTHORITY = NONE
PR673_IMPLEMENTATION = PAUSED
```

Still required before PR #673 P1-A implementation may resume:

1. exact three-path readback for this Revision-5 docs-only amendment;
2. Agent Governance, Architecture Semantic Audit and FULL Merge Gate / aggregate `game-gate` SUCCESS on the stable amendment head;
3. one genuinely independent exact-head HIGH whole-diff architecture/resource/security review of this amendment;
4. protected integration/readback through the coordinator-controlled governed route;
5. fresh #162 preflight granting the smallest exact source/vendor/Cargo successor lease to the PR #673 implementation writer.

The remaining `I/R/T/active` **byte values** are implementation qualification obligations. Their lifecycle ownership, retirement classification, non-overlap and recovery trigger are no longer A4 architecture choices. If the measured exact candidate fails the frozen root equation, A4 must stop and escalate rather than changing those semantics or widening the envelope.

This file performs no runtime, Cargo, vendor, SQL/migration, workflow, Platform, production, secret, merge or Merge Queue mutation.
