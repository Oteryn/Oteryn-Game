# OTV2 WP3 — Architecture / Complexity Audit Addendum 01

- Date: 2026-09-12
- Repository: `Oteryn/Oteryn-Game`
- Parent audit: `docs/agents/evidence/OTV2_WP3_ARCHITECTURE_COMPLEXITY_AUDIT_20260912.md`
- Parent audit branch: `agent/wp3-architecture-audit-20260912`
- Protected `main` at addendum readback: `489e3e390a1bce1ce3439c66521ab75f8a826cd8`
- WP3 Issue: `#351`
- WP3 Draft PR: `#356`
- WP3 PR HEAD at addendum readback: `fe7891989b1247012e32c89c10cff6a10bacb943`
- Child B / WP4 Draft PR inspected: `#335`
- Evidence class: retained architecture audit evidence
- Architecture authority: **none** — this addendum does not activate, accept, supersede or implement architecture by itself

## 1. Purpose

This addendum records findings obtained after the parent WP3 architecture audit was saved. It must be read together with the parent report.

The new evidence strengthens the parent's `WP3_BOUNDARY_REDESIGN_REQUIRED` verdict. The most important new result is that the current operation-owned connection model does not match the lifecycle of the accepted `PgPool`-based Durability consumer.

The corrected question is no longer only:

```text
How far down must one operation owner be propagated?
```

It is now:

```text
Which resource classes are genuinely operation-owned,
which are executor/pool-shared,
and which dependency-private peaks are best covered by
source-derived finite phase reservations from the same root ledger?
```

## 2. Executive delta

```text
Parent verdict:
WP3_BOUNDARY_REDESIGN_REQUIRED

Addendum verdict:
CONFIRMED_AND_NARROWED

Current #356 classification:
NEEDS_DECISION

Recommended implementation direction:
ROOT/POOL-SHARED RESIDENCY
+ ACTIVE-SLOT OPERATION RESERVATION
+ SOURCE-DERIVED PHASE ENVELOPES
+ MINIMAL SQLx SEAM

Do not continue broad Tokio/rustls custody expansion
until the superseding boundary decision is protected.
```

## 3. New finding — the production PgPool exists before an active operation slot

### Classification

`PROVEN`

### Evidence

Child B / PR #335 uses a shared `RuntimeBackend` containing a `PgPool`.

Its runtime registration sequence is structurally:

```text
registered_backend(database_url)
  -> schema::connect_runtime(database_url)
     -> PgPoolOptions::connect(...)
  -> DurabilityCustody::acquire(&pool)
     -> real PostgreSQL transaction / queries
  -> WorkCustody::new(&pending)
  -> RuntimeBackend { pool, custody, work, ... }
```

The work-custody object containing the fixed eight queued and two active semantic slots is therefore created only **after** pool creation and an initial custody transaction.

SQLx `PgPoolOptions::connect()` also establishes at least one connection during pool creation. This means TCP/TLS/socket/reactor/connection baseline work exists before any concrete active operation slot can own it.

### Consequence

The current #356/#583 direction of treating connection establishment as one same-operation owner from DNS through TLS cannot be the only production ownership model.

At minimum the architecture needs an **executor/root-shared resource owner or reservation** capable of funding bootstrap and idle pool residency independently of a later active operation.

Creating a fictitious active slot solely to pay for bootstrap would misrepresent DFR semantics. Replacing the accepted pool topology with one direct connection per operation would also be a material topology change.

## 4. New finding — the current operation-owned direct connection is not the production consumer path

### Classification

`PROVEN`

The WP3 branch exposes an owner-aware direct PostgreSQL connection establishment route intended to avoid changing ordinary pool behavior.

Child B / #335, however, uses `PgPool` through one shared `RuntimeBackend`.

### Derived conclusion

`DERIVED — HIGH CONFIDENCE`

The direct owner-aware connection is useful as a qualification fixture and source experiment, but it is not sufficient proof for the real production composition.

The final WP3 qualification must exercise the exact consumer lifecycle:

```text
RuntimeBackend
  -> PgPool
  -> checkout
  -> transaction/query work
  -> cleanup
  -> return/close
  -> idle/reconnect lifecycle
```

## 5. New finding — the DFR owner and driver owner currently exist as disconnected halves

### Classification

`PROVEN`

Child B / #335 already has:

```text
WorkCustody
  queued[8]
  active[2]
```

but explicitly documents that this is not yet the executor's complete queue/active-operation byte budget.

The semantic APIs are also not yet fully routed through that queue/active lifecycle.

WP3 / #356 contains a sophisticated `ResourceBudget` / `ResourceReservation` driver mechanism, but it is attached to an owner-aware direct connection path rather than the final `RuntimeBackend`/pool lifecycle.

### Consequence

The two halves must be composed before terminal qualification:

```text
B semantic custody + real byte ledger
              ↓
        RuntimeBackend root
              ↓
          PgPool/shared state
              ↓
       active-slot budget view
              ↓
          SQL operation
```

A driver proof in isolation cannot substitute for this composition.

## 6. Recommended corrected ownership model

### Classification

`RECOMMENDATION`

One root ledger remains authoritative. No new resource budget or new numeric maximum is proposed.

```text
DUR-FRESH executor root ledger
│
├── executor/runtime shared reservation
│   ├── runtime bookkeeping attributable to the B executor
│   ├── shared TLS configuration
│   └── PgPool control state
│
├── connection resident reservation × bounded connection slots
│   ├── socket baseline buffers
│   ├── TLS connection resident state
│   ├── Tokio registration state attributable to DB sockets
│   ├── bounded statement/type/status metadata
│   └── retained connection caches that are explicitly permitted
│
├── connect transient reservation
│   ├── handshake parser/transcript peak
│   ├── temporary certificate/verifier state
│   ├── socket construction overlap
│   └── other source-derived bounded connection-establishment peaks
│
└── active slot × 2
    ├── immutable operation/correlation state
    ├── SQL arguments
    ├── operation-specific send/receive growth
    ├── bounded SQL results / row/value backing
    ├── parser/error temporaries
    ├── completion
    └── reconciliation / ambiguity custody
```

This model still satisfies the accepted rule that fixed runtime/database overhead is not free: shared state remains charged to the same root envelope. It only changes **where the lifetime is represented**.

## 7. Source-derived phase reservation is compatible with the accepted contract

### Classification

`PROVEN / DERIVED`

The accepted DFR decision allows dependency/runtime allocation to be either charged directly or covered by an explicit finite reservation inside the existing envelope.

The historical WP3 plan also contemplated source-derived phase reservation before entering rustls.

### Consequence

WP3 does **not** inherently require one token attached to every private `Vec`, `Box`, `Arc`, transcript object and provider-internal value.

A defensible architecture can instead prove, for a tightly frozen production profile:

```text
CONNECT_TRANSIENT_BOUND
CONNECTION_RESIDENT_BOUND
ACTIVE_SQL_PEAK
```

reserve them before entering the corresponding phase, and release/convert them at truthful lifecycle boundaries.

This is valid only when each bound is source-derived or measured/proven for the exact profile. A magic whole-slot reserve remains forbidden.

## 8. New finding — production SQLx profile and current owner-aware qualification profile diverge

### Classification

`PROVEN`

The workspace production SQLx feature graph still uses:

```text
tls-rustls-ring-webpki
```

The current owner-aware TLS path has been developed and positively qualified around the AWS-LC profile, and the owner-aware handshake rejects a build that lacks the qualified AWS-LC feature.

### Consequence

The current helper proves an isolated supported profile, not the frozen production Game profile.

This must be resolved explicitly. Two lawful options are:

1. keep production `ring/webpki` and qualify that exact profile; or
2. separately decide to change the production provider/profile and then qualify the new production graph.

Changing the whole project to AWS-LC merely to make #356 pass is not authorized by this audit.

## 9. New finding — several rustls accounting families are unreachable in the current production feature graph

### Classification

`PROVEN`

The current SQLx rustls dependency uses `default-features = false` and does not enable rustls `brotli` or `zlib` certificate-compression features.

Without configured decompressors, compressed-certificate payloads may still need safe parsing/rejection, but the large certificate decompression destination backing and second decoded certificate pass are not production-reachable allocation families in this graph.

Rustls session resumption is also configurable through public API and can be disabled for a first bounded DB profile.

### Recommendation

For the first production DB profile, prefer eliminating optional retained allocation families before inventing custody for them:

- keep certificate compression disabled;
- consider `Resumption::disabled()` for DB connections pending measured reconnect cost;
- do not carry production fork code solely for unreachable optional paths.

This is a profile decision, not an implementation authorization.

## 10. New finding — Tokio blocking-owner expansion is largely induced by DNS/file-loader choices

### Classification

`PROVEN / DERIVED`

Tokio 1.53.1 `ToSocketAddrs` resolves a literal `SocketAddr` / `(IpAddr, u16)` synchronously through a ready future. Hostname strings use the blocking resolver path.

SQLx file-backed certificate inputs also use blocking file work; already-loaded/in-memory certificate material does not require the same file-loader path.

### Recommendation

Evaluate a bounded first-slice connection profile with:

```text
connect_address: SocketAddr
TLS server_name: hostname retained separately for SNI/VerifyFull
CA/root material: preloaded bounded configuration
```

This is not equivalent to disabling hostname verification. Transport routing and TLS identity are separate concerns.

SQLx 0.9 currently does not provide full libpq-like separate `hostaddr + host` semantics because the parsed fields converge on the same host representation. A narrow SQLx seam separating transport address from TLS server name may therefore be required.

If this profile is accepted, the generic Tokio DNS blocking-owner/std-thread cluster can disappear from the production WP3 path instead of being fully forked.

## 11. Candidate simplification — dedicated Durability runtime

### Classification

`RECOMMENDATION / REQUIRES MEASUREMENT`

The accepted Durability topology requires an asynchronous Durability worker / PgPool, but does not require the worker to share the main gameplay Tokio runtime.

A dedicated current-thread Tokio runtime for the B executor is therefore a candidate simplification:

```text
one B executor thread
  -> current-thread Tokio runtime
  -> DB I/O only
  -> PgPool max/min bounded to the accepted active model
```

Advantages:

- reactor allocation attribution becomes B-local instead of process-global;
- pending-release state cannot be inflated by unrelated gameplay/network I/O;
- only the DB socket registration family needs to fit the B root reservation;
- no multi-worker scheduler is required for two concurrent I/O-bound active futures.

Cost / unknown:

- the runtime/thread bookkeeping itself must be measured and charged inside the same root envelope;
- this is a topology refinement and requires architecture review before implementation.

## 12. New finding — `PgPool max_connections(4)` is not justified by the current first-slice concurrency model

### Classification

`PROVEN + DERIVED`

Child B uses `schema::connect_runtime()` with `max_connections(4)`.

The accepted DFR executor has two active slots. The Child B plan intentionally makes no broad concurrency claim and serializes the strongest relation/domain locking protocol.

The V1/V2 shared `RuntimeBackend` repair also avoids the previously observed duplicate-pool topology.

### Recommendation

`PgPool max_connections = 2` is the smallest currently evidenced first-slice ceiling, subject to final runtime tests proving no operation requires a nested second independent connection.

A deterministic first-slice pool candidate is:

```text
min_connections = 2
max_connections = 2
idle_timeout = None
max_lifetime = None
```

SQLx minimum-connection maintenance opens missing connections sequentially. This gives a simpler bootstrap/transient peak and avoids background idle/lifetime churn.

This is a recommendation, not an accepted value change.

## 13. New finding — active-slot release must include pool cleanup/return lifecycle

### Classification

`PROVEN`

A `Transaction` obtained from `Pool::begin()` owns a pooled connection. `commit(self)` consumes the transaction. When the pooled connection is dropped, SQLx asynchronously spawns `return_to_pool()`; `after_release`, ping and final idle release happen later.

### Consequence

The active byte slot cannot truthfully be released merely because `commit().await` returned if operation-owned buffers/results/error/custody can still be involved in asynchronous connection return/cleanup.

### Recommended lifecycle

Prefer explicit checkout and awaited cleanup:

```text
acquire active slot
  -> pool.acquire()
  -> transaction borrowing that PoolConnection
  -> operation
  -> commit/reconcile
  -> clear/shrink bounded connection state where applicable
  -> await return-to-pool or close
  -> prove no operation-owned backing remains
  -> release active slot
```

SQLx exposes the required connection lifecycle primitives, including buffer shrink and cached-statement cleanup; exact API use must be qualified against the pinned version.

Cancellation preserves the same rule. Existing Child B semantics already deliberately retain active custody after submitter cancellation until definitive disposition/owner acknowledgement. The byte ledger should follow that same custody state.

## 14. New finding — Child B itself still violates accepted SQL-result resource policy

### Classification

`PROVEN`

The accepted DFR decision states that the inherited 64 pending commands must not be treated as 64 returned SQL payload rows. They must be represented as one bounded ordered aggregate per exact attempt while preserving every command identity/disposition.

Current #335 still contains `fetch_all()` paths that retrieve pending command child rows individually and check the returned vector length only afterward.

Current #335 also contains several direct `SELECT state, record_json ...` reads without the accepted SQL-side `octet_length` / bounded projection pattern before transferring the variable-size payload into the driver.

### Consequence

Even a perfect WP3 dependency fork cannot make Child B DFR-compliant while these consumer queries bypass the accepted row/result transfer rules.

### Required repair before final WP3/WP4 qualification

- replace individual pending-command row fetches with the accepted bounded ordered aggregate shape;
- guard variable-size `record_json` and analogous payload columns in the same protected SQL snapshot before driver materialization;
- retain exact semantic equality checks after bounded decode;
- exercise max/max+1, corrupted stored state and concurrent substitution negatives on real PostgreSQL 17.6.

These are primarily Child B query-shape repairs, not reasons to deepen the Tokio/rustls fork.

## 15. New finding — raw `sqlx::Error` is an unbounded/long-lived completion risk

### Classification

`PROVEN`

Child B `DurabilityError` contains `Database(sqlx::Error)` and formats the wrapped error.

`PgDatabaseError` retains the decoded PostgreSQL `ErrorResponse` / `Notice`, including variable human-readable message/detail/hint/context/object fields. The full driver error can therefore retain peer-originated backing beyond the immediate receive path and may be formatted into logs or completions.

### Recommendation

Normalize SQLx/PostgreSQL failures **inside the active slot** into a small bounded Durability error representation, for example:

```text
resource/unavailable
integrity/conflict class
schema/migration class
bounded SQLSTATE
bounded correlation identifier
```

Destroy the full driver error before backend cleanup and active-slot release. Do not persist or log unlimited PostgreSQL human-readable fields.

The exact public error contract remains governed by existing Durability semantics; this audit does not invent a new client-visible enum.

## 16. New finding — connection-retained PostgreSQL state requires an explicit policy

### Classification

`PROVEN`

`PgConnection` retains more than the socket:

- statement cache;
- `ParameterStatus` values;
- type/OID/table/array caches;
- row/metadata structures depending on exercised queries.

Some caches are bounded by configuration (statement cache default 100), while others require reachability or explicit policy proof.

Child B currently uses built-in PostgreSQL types and a finite query corpus; current source does not use the general custom `PgTypeInfo::with_name`, `derive(sqlx::Type)` or broad offline type-resolution profile.

### Recommendation

Do not qualify all theoretical SQLx PostgreSQL features.

Freeze a **B production SQL profile** and prove only its reachable connection-retained state. Keep the default statement cache only if an exact unique-SQL inventory and finite memory bound fit the two resident connection reservations; otherwise reduce/disable it with explicit prepared-statement lifecycle qualification.

Do not use a fixed resident reservation until all retained maps in the accepted profile are either bounded, unreachable, periodically reset, or cause the connection to be closed instead of returned idle.

## 17. Pool release hook as a fail-closed simplification

### Classification

`PROVEN / RECOMMENDATION`

SQLx 0.9 pool hooks allow an `after_release` decision to reject a connection and close it instead of returning it to idle state.

This offers a simple first-slice fail-closed policy:

```text
if connection residual state cannot be proven within the
accepted resident envelope after operation cleanup:
    close connection
else:
    return to idle pool
```

This can avoid proving perfect shrink semantics for every cache in the first implementation.

It does not remove the need to account the connection while it exists.

## 18. Updated disposition of current dependency forks

### ResourceBudget / ResourceReservation

`KEEP`

The RAII primitive remains useful for exact dynamic boundaries and for phase/root reservations.

### SQLx PostgreSQL fork

`REWORK / NARROW`

Retain only seams required to:

- bind pool/root and active-slot reservations;
- bound receive/write transfer before peer-controlled growth;
- enforce exact PostgreSQL length/count/result policies that cannot be guaranteed by the B SQL query shape alone;
- separate transport address from TLS server identity if the bounded connection profile is accepted;
- expose required cleanup/high-water information if no upstream API suffices.

### rustls fork

`PRESERVE AS EVIDENCE, REMOVE FROM FINAL PRODUCT DIFF IF PHASE-BOUND PROOF SUCCEEDS`

The extensive custody work is valuable as source census, hostile vectors and phase-bound evidence. It should not be retained as production complexity merely because effort has already been spent.

### Tokio blocking-owner fork

`PRESERVE AS EVIDENCE, CANDIDATE FOR REMOVAL`

If the production DB profile uses literal `SocketAddr`, preloaded TLS material and a dedicated bounded Durability runtime, the current DNS/file-loader/thread-owner expansion may no longer be production-reachable.

### Tokio reactor fork

`NEEDS_DECISION`

A dedicated Durability runtime plus finite root-shared registration reservation may remove the need for per-`ScheduledIo` operation ownership. The exact source-derived reactor bound and runtime footprint still need proof.

## 19. Updated proposed WP3-v2 implementation sequence

This sequence does not authorize implementation. It is the recommended shape of the next protected architecture decision.

### V2-0 — freeze new micro-fix expansion

Do not activate further E1/E2/E3 or rustls/Tokio private ownership amendments merely to continue the current direction until the superseding boundary analysis is decided.

Safe read-only census/tests may continue.

### V2-1 — superseding architecture decision

Decide explicitly:

1. executor/root-shared versus active-operation resource classes;
2. production SQLx/rustls provider/features;
3. DB connection profile: TCP/UDS, transport address, TLS server identity, sslmode, CA/client-auth input forms;
4. whether Durability uses a dedicated Tokio runtime;
5. pool topology and connection ceiling;
6. connect transient reservation formula;
7. connection resident reservation formula;
8. active SQL peak formula;
9. timeout/cancellation/return-to-pool finality;
10. which existing WP3 amendments become superseded, retained evidence or still required.

### V2-2 — repair the real consumer substrate

On Child B / the final shared RuntimeBackend composition:

- route all affected semantic DB work through the accepted queue/active lifecycle;
- install the real root byte ledger;
- add two active-slot budget views;
- repair pending-command aggregate reads;
- repair SQL-side oversized payload guards;
- normalize driver errors inside the slot;
- make connection cleanup/return part of slot finality.

### V2-3 — narrow SQLx seams

Implement only the SQLx changes still required after V2-2 and the frozen profile.

Prefer ordinary upstream Tokio/rustls behavior under pre-reserved phase envelopes where proof is sufficient.

### V2-4 — measure/prove exact bounds

Required values remain `UNKNOWN` until source-derived/measured:

```text
EXECUTOR_RUNTIME_RESIDENT
CONNECTION_RESIDENT × connection_count
CONNECT_TRANSIENT_PEAK
ACTIVE_SQL_PEAK per slot
```

All must compose inside the already accepted root envelope. Do not invent a new maximum to make them fit.

### V2-5 — complete real qualification

At minimum:

- exact production provider/profile;
- real PostgreSQL 17.6;
- real TLS with hostname verification;
- exact RuntimeBackend/PgPool path, not only direct `PgConnection` fixture;
- two simultaneous active slots;
- bootstrap before work admission;
- connection failure/reconnect;
- bounded result maxima and max+1 denial before transfer;
- cancellation after submission;
- ambiguous commit/reconcile;
- connection cleanup then awaited return/close before slot release;
- retained row/value/error lifetime tests;
- hostile PostgreSQL length/count/status/error vectors;
- no product patch to Tokio/rustls if phase-bound proof succeeds;
- full exact-head Rust 1.94 fmt/check/strict Clippy/tests/governance/security/supply-chain review;
- genuinely independent whole-diff HIGH-risk review;
- canonical CI + FULL Merge Queue + protected-main readback.

## 20. Current unknowns

`UNKNOWN` — exact numerical `EXECUTOR_RUNTIME_RESIDENT` bound.

`UNKNOWN` — exact two-connection resident bound under the final B SQL profile.

`UNKNOWN` — exact connect transient bound after optional/unreachable rustls families are removed.

`UNKNOWN` — whether dedicated current-thread runtime footprint plus reactor state is smaller/simpler than retaining a narrow Tokio owner seam; this requires measurement.

`UNKNOWN` — final deployment connection profile, especially whether production may require runtime DNS, file-backed roots/client certificates, UDS or multiple address failover.

`UNKNOWN` — whether current statement-cache default 100 is desirable once exact unique query inventory and memory profile are measured.

No unknown above authorizes inventing a cap or silently weakening TLS/PostgreSQL semantics.

## 21. Final addendum verdict

The later source and consumer audit does not contradict the existing protected WP3 reports. It explains why those reports keep discovering successively earlier/lower allocation seams.

The existing reports correctly identify real defects in:

```text
BufferedSocket boundary
Tokio reactor registration
DNS blocking work
runtime owner finality
thread metadata finality
```

The new evidence adds the missing higher-level fact:

```text
A pooled PostgreSQL connection is executor infrastructure whose
bootstrap and idle residency can exist outside any individual
active B operation.
```

Therefore continuing to propagate one operation-owned capability through every private dependency layer is not demonstrated to be the minimum or correct architecture.

Recommended current state:

```text
#356:
OPEN / DRAFT / PRESERVE EVIDENCE

architecture disposition:
NEEDS_DECISION

material implementation:
HOLD NEW BROAD DEPENDENCY-OWNERSHIP EXPANSION

next architecture action:
WP3_V2_SUPERSEDING_DECISION

preserve:
ResourceBudget primitives
source census
hostile tests
real PG17.6/TLS harness
sound exact lifetime repairs

reconsider/remove from final product if no longer required:
broad rustls custody fork
Tokio blocking-owner fork
operation-owned DNS/reactor/thread metadata propagation
```

This addendum records evidence and recommendation only. Existing protected architecture and live #162/#351 authority remain binding until a separately reviewed/protected decision changes them.
