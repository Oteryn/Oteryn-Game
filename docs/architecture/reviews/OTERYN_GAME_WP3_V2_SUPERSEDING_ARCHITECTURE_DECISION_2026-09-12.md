# Oteryn Game — WP3-v2 superseding architecture decision

- Decision ID: `WP3-V2-ROOT-OWNED-BOUNDED-PGPOOL-V1`
- Date: 2026-09-12
- Status: **CANDIDATE / NOT ACCEPTED / EVIDENCE-BLOCKED**
- Worker: `Oteryn: astra wp3-v2 architecture lead`
- Protected admission: `main@489e3e390a1bce1ce3439c66521ab75f8a826cd8`
- Canonical implementation lineage: Issue #351 / Draft PR #356
- Child B consumer lineage: Issue #329 / PR #335
- Programme: #162 / #364
- Retained architecture evidence: Draft PR #588
- Cross-repository evidence: `docs/agents/evidence/OTV2_WP3_GAME_PLATFORM_CROSS_REPO_AUDIT_R01_R21_20260912.md` on the WP3-v2 programme branch
- Architecture authority: **none**. This candidate does not activate A4, mutate runtime, accept itself, close #356, or bypass protected review/integration.

## 1. Resolution

Select **Option B** for the first production slice:

> one logical Durability executor and one accepted DFR root ledger, a narrowly configured SQLx `PgPool` used as a **single-ready-connection holder** (`max_connections = 1`, `min_connections = 0`, lazy construction), root-owned and serialized connection establishment outside active DFR work, ready-only checkout for active DB passes, and only the minimal SQLx/PostgreSQL seams that the exact final consumer cannot close at the query/profile layer.

This decision intentionally keeps **two logical active custody slots** while allowing **at most one physical PostgreSQL DB pass at a time**. Active slots represent bounded custody/ambiguity ownership; they are not a promise of two simultaneous PostgreSQL transactions.

The selected first-slice root model is:

```text
one process-scoped Durability executor / one accepted DFR root ledger
|
+-- fixed shared executor/runtime/pool/provider/config reservation (I)
|
+-- PgPool holder
|   +-- max_connections = 1
|   +-- min_connections = 0
|   +-- lazy construction
|   +-- at most one settled ready connection (R)
|   +-- no minimum-connection replacement
|   +-- idle retirement may close only under the frozen finite policy
|
+-- root maintenance
|   +-- the only production connection-establishment owner
|   +-- at most one connect/reconnect transient at a time (T)
|   +-- finite deadline/retry policy outside active DB passes
|   +-- returns exactly one qualified ready connection to the pool
|
+-- executor custody
    +-- queued <= 8
    +-- logical active <= 2
    +-- physical DB execution concurrency <= 1
    +-- one absolute 1 s queue-residence ceiling
    +-- one absolute 2 s DB execution/reconciliation-pass ceiling
```

Candidate feasibility invariant:

```text
I + max(R, T) + Q + A <= 12 MiB
```

where `Q` and `A` are actual current queue/active charges under `DUR-FRESH-RESOURCE-ENVELOPE-V1`, not additional per-connection budgets. Exact `I`, complete `R`, complete `T`, active peak and lower-layer retirement tails remain evidence obligations; this candidate does not fabricate their values.

## 2. Why this must be decided now

**Must decide now: YES.**

Blocked downstream work:

1. Gate 1 of the WP3-v2 programme cannot release A4 without one superseding architecture.
2. #356 cannot truthfully continue broad SQLx/rustls/Tokio ownership expansion while the final consumer uses a shared `RuntimeBackend`/`PgPool` lifecycle that exists before active-operation custody.
3. Child B #335 cannot become the final composed WP3/WP4 consumer while semantic persistence bypasses the accepted `8 queued -> 2 active` executor, lacks definitive active-slot release, and does not enforce the accepted 2-second pass deadline.
4. WP4/WP5/G0/Server Seam remain downstream of a terminally qualified WP3 consumer.

What becomes harder later if the wrong topology is frozen:

- broad dependency forks create continuing SQLx/rustls/Tokio upgrade and supply-chain burden;
- actor-specific mailbox/lifecycle APIs would couple Child B to a second work-ownership model if added beside the already accepted DFR executor;
- a multi-connection topology increases root overlap/finality proof before evidence shows first-slice value;
- ambiguous cleanup/retry behavior becomes a durable compatibility problem once persistence APIs depend on it.

Evidence that can justify supersession later:

- representative mixed-operation latency/throughput showing max-one physical connection materially violates an accepted product SLO while required correctness locks remain;
- source or exact-head proof that SQLx pool background/finality behavior cannot be bounded without a larger fork than an explicit connection actor;
- a later lock architecture that permits useful parallel DB execution and a complete two-connection root proof;
- PostgreSQL/TLS/provider/resource evidence showing a different topology materially reduces risk or resident/transient cost;
- changed availability/failover requirements that cannot be met by the first-slice holder policy.

Deliberately not decided here:

- final production throughput target;
- later removal/relaxation of the global relation fence;
- multi-node DB availability/failover topology;
- database history retention/compaction;
- Platform source implementation/deployment;
- gameplay Server Seam TLS profile;
- final values for any resource quantity not already accepted by `DUR-FRESH-RESOURCE-ENVELOPE-V1`.

## 3. Evidence basis

### PROVEN

- The accepted DFR envelope provides one logical executor, queue `8`, active `2`, queue wait `1,000 ms`, DB pass `2,000 ms`, and a nominal queue+active resident ceiling of 12 MiB. Runtime/database-pool overhead retained for B is not free.
- #335 production composition owns one process-shared `RuntimeBackend` and `PgPool`; pool/bootstrap/schema/custody work exists before active `WorkCustody`.
- Current semantic persistence APIs still call the shared backend transaction path directly and are not all routed through the prototype work queue.
- Current `WorkCustody` has no terminal active clear/ack path, and recovered pending state is duplicated into active custody without a retirement path.
- Ordinary SQLx pool connect includes retry/backoff; an active DFR pass must not manufacture a connection through that path.
- SQLx provides ready-only pool acquisition primitives that do not create a new connection.
- `max_connections = 1` preserves the logical pool permit through SQLx close, but lower Tokio reactor retirement can outlive SQLx-level close.
- Current Child B relation fencing serializes protected admission writes strongly; this does not by itself prove one connection meets product latency requirements.
- Current schema inspection, pending-command materialization, variable `record_json` reads, retained `ParameterStatus`, frame/count decoding and error/logging paths include boundedness gaps.
- #356 contains valuable resource primitives, hostile tests, PostgreSQL/TLS evidence and source research, but its broad operation-owned connection model is not sufficient proof for the real pooled Child B lifecycle.

### UNKNOWN / evidence obligations

- exact `EXECUTOR_RUNTIME_RESIDENT` (`I`);
- exact complete one-connection resident (`R`);
- exact complete connect transient (`T`);
- exact active SQL peak after executor/query repair;
- exact lower-layer deferred reactor-retirement term;
- exact final resolved TLS provider/features and runtime profile;
- exact accepted PostgreSQL startup/authentication mechanism allowlist for the production profile;
- exact final SQL corpus and statement-metadata byte bound;
- representative mixed-operation latency/throughput for the one-physical-connection first slice.

The missing values above are not permission to invent a cap, add a second budget, weaken TLS/hostname verification, or add hidden connection capacity.

## 4. Alternatives

### Option A — broad SQLx/rustls/Tokio ownership instrumentation

**Disposition: REJECT AS TERMINAL PRODUCT ARCHITECTURE; RETAIN SELECTIVE EVIDENCE.**

Benefits:

- strongest existing low-level source investigation;
- hostile denial/finality cases already expose real lower-layer lifetime hazards;
- some generic owner-aware primitives may remain useful if the final minimal seam needs them.

Costs/risks:

- audited #356 spans a very large vendored surface and its operation-owned direct-connection proof does not match the real shared pool/bootstrap lifecycle;
- generic dependency ownership creates high upgrade/provenance/review burden;
- it does not by itself solve executor bypass, active-slot finality, recovered pending retirement, root bootstrap/configuration or end-to-end deadline ownership;
- continuing broad expansion before the root topology is frozen risks optimizing the wrong boundary.

### Option B — minimal SQLx seam + one finite root/phase budget around a bounded PgPool

**Disposition: SELECTED.**

Benefits:

- reuses Child B's current `PgPool` composition and SQLx transaction/query model;
- removes connection establishment/retry from active work while preserving a narrow ready-only checkout path;
- requires one physical connection in the first slice, reducing multiplicity in the root proof;
- keeps SQLx pooling as a holder/return mechanism while disabling minimum-connection replacement through `min_connections = 0`;
- permits query/profile repairs to close many gaps without generic Tokio/rustls redesign;
- smallest migration from the current Child B source while preserving an upstream-compatible path for most of the stack.

Risks:

- pool reaper/return/ping/close and Tokio reactor tails still need explicit finality accounting;
- SQLx internal logging and retained connection maps may still require a narrow profile seam;
- one physical connection is not yet performance-qualified;
- exact provider/configuration/authentication profile remains to be frozen from evidence.

### Option C — one or two explicit bounded PostgreSQL connection actors

**Disposition: REJECT FOR FIRST SLICE; PRESERVE AS SUPERSESSION OPTION.**

A single connection actor could make connection ownership explicit and remove general pool maintenance from the proof. Two actors could later support real parallel DB work.

It is not selected now because:

1. Child B already has an accepted logical queue/active custody model. A second actor mailbox/ownership layer would duplicate custody unless the executor and actor are collapsed into one new API.
2. Current Child B semantic code is written around `PgPool`/pooled transactions. Moving all reachable persistence to actor-owned `PgConnection` would be a broader consumer rewrite than Option B.
3. Explicit actors do not remove TLS/provider/socket/reactor/configuration finality obligations; they only move their owner.
4. Two connection actors reintroduce concurrent `R/T` overlap and a stronger root equation before product evidence demonstrates first-slice benefit.
5. No current evidence shows Option B's holder-only pool policy is less maintainable or less provable than a custom actor once connection creation is root-owned and active checkout is ready-only.

Reconsider C if exact source/finality evidence proves a bounded holder-style pool still needs broader invasive changes than a single actor, or if representative performance evidence establishes a real need for additional physical DB concurrency.

## 5. Root and executor ownership

There is exactly one production Durability executor per process for this first slice and exactly one accepted DFR root ledger for its request-derived and retained shared resources.

The root owns:

- fixed executor/runtime structures;
- bounded configuration and credential material retained for DB operation;
- the pool object and one settled connection reservation;
- provider/runtime shared residency attributable to this Durability profile;
- the serialized connect/reconnect transient reservation;
- any deferred reactor/finality tail that can outlive SQLx logical return;
- the bounded work queue and two logical active custody slots through their accepted sub-reservations.

A connection, request, account, task, retry or reconnect does **not** mint another 12 MiB ledger.

No second runtime-owner identity may represent the same charged backing. Shared backing is counted once only with explicit transfer/reference finality; deep copies are charged separately.

## 6. Queue, active custody and release/finality

### Queue

- maximum eight not-yet-submitted operations;
- reserve capacity before retaining/copying the sealed operation;
- queue residence maximum remains 1,000 ms;
- queued cancellation may release only work that has not promoted to active.

### Logical active custody

- maximum two active operation identities;
- promotion moves original custody before any DB await and funds required submission/completion/reconciliation state before irreversible effects;
- physical DB execution is serialized to at most one pass at a time in this first slice;
- submitter timeout/cancellation ends waiting only. It never means rollback, non-commit, cleanup completion or slot release.

### Definitive release

An active slot may clear only after all of the following are true:

1. the exact original operation has a definitive durable disposition, including reconciliation where COMMIT outcome was ambiguous;
2. transaction/protocol cleanup required for that disposition has completed or the connection has been fenced/closed under root custody;
3. the bounded completion has been delivered to the owning consumer;
4. the owner has acknowledged that completion;
5. the matching durable pending checkpoint is cleared/finalized under the accepted protocol;
6. recovered in-memory pending duplicates for that operation are retired;
7. no descendant object charged to the active slot remains outside an explicit charged transfer.

Transaction `Drop`, caller timeout, future cancellation, `after_release`, pool wrapper drop, or SQLx logical close alone is never sufficient evidence of active-slot or root-resource finality.

## 7. PgPool and connection lifecycle

### Construction

- pool construction is lazy;
- `max_connections = 1`;
- `min_connections = 0`;
- production code must not perform an eager/minimum background connect at executor creation;
- one process-scoped root owns the pool for the lifetime of the registered Durability backend.

### Establishment / replacement

Only root maintenance may establish or replace the physical PostgreSQL connection.

- at most one establishment transient may be live at a time;
- the complete transient is covered by root reservation `T` before connect begins;
- connect retry/backoff, if retained at all, is bounded by a separately frozen root-maintenance deadline and remains outside active DB-pass time/custody;
- active work never calls a connecting `acquire()` path;
- no minimum-connection background replacement is allowed;
- failed root establishment leaves active semantic work fail-closed/unavailable rather than creating hidden capacity or retrying inside the original pass.

### Active checkout

Active DB work uses `try_acquire()` or an equivalent proven ready-only primitive. `None`/not-ready means unavailable for that pass; it does not trigger connection creation.

The transaction borrows the checked-out connection explicitly so connection return/close can be observed and qualified.

### Retirement / return / close

- idle/max-lifetime policy must be explicitly frozen to finite values before Gate 1 acceptance; **exact durations are currently `BLOCKING_EVIDENCE_GAP`** because the current A2 exact-source package is not yet present;
- reaping may close the one idle connection but `min_connections = 0` must prevent automatic replacement;
- root maintenance is the only path that can later recreate readiness;
- `after_release` is pre-final-ping and cannot be used as finality;
- SQLx close/return must retain the pool permit until its own async close is complete;
- deferred Tokio reactor/provider/socket tail after SQLx logical return is included in `I/T` or closed by one narrowly reviewed finality seam;
- stalled TLS close remains root-owned until its bounded cleanup/fencing policy is terminal.

## 8. Bounded configuration and credential sources

The first-slice production profile admits **one explicit Oteryn-owned bounded configuration object** that is converted into SQLx PostgreSQL options. Ambient dependency configuration is not production authority.

Required policy:

- database endpoint/address, database name, username and explicit auth material come only from the bounded Oteryn configuration source;
- arbitrary `PG*` environment inheritance, `.pgpass`/passfile discovery and OS-username fallback are forbidden in the qualified production profile unless individually re-admitted with finite bounds and exact tests;
- CA roots/client credential material, when enabled, must come from explicit bounded in-memory/preloaded inputs or explicitly bounded files with lifetime and rotation accounting;
- malformed credential/configuration content is redacted before dependency/application logging;
- old/new configuration and connection overlap during any permitted rotation is charged and covered by the root equation;
- no secret content may be retained in unbounded errors, logs or historical work records.

If pinned SQLx cannot express this profile without consulting ambient sources, the allowed change is a **minimal profile seam**, not a generic configuration subsystem fork.

## 9. PostgreSQL startup/authentication profile

The production connection profile is closed-world: every message/mechanism reachable from connection start through `ReadyForQuery` must be listed and funded before Gate 1 acceptance.

Current architecture requirements are:

- PostgreSQL 17.6 qualification target remains binding for the current programme;
- TLS/hostname verification must not be weakened to make accounting easier;
- startup/authentication state, `BackendKeyData`, server parameter/status state, errors/notices and all reachable auth buffers remain inside the root/connect transient budget until their actual finality;
- unlisted startup/auth mechanisms are fail-closed/unreachable, not silently accepted;
- CleartextPassword, MD5Password, SASL/SCRAM and channel-binding families are **not** assumed reachable or unreachable without exact A2 resolved-profile evidence.

`BLOCKING_EVIDENCE_GAP`: the exact production authentication allowlist is not frozen by the retained audit. A2 must provide the exact reachable mechanism set. This decision may not be marked architecture-ready for protected acceptance until that allowlist is recorded without weakening current PostgreSQL security semantics.

## 10. Frozen TLS/runtime/provider profile requirement

The topology decision is Option B, but architecture acceptance still requires a closed exact profile appendix containing:

- resolved production Cargo feature graph;
- selected rustls/provider implementation and version;
- accepted PostgreSQL SSL mode(s) for this first slice;
- hostname/server-name verification rule;
- TLS protocol/version/resumption/compression reachability;
- DNS vs literal-address policy and transport-address/TLS-identity mapping;
- CA/client credential source/lifetime;
- executor runtime topology, target/toolchain/allocator/panic/optimization assumptions needed for layout/phase bounds.

Current retained evidence is insufficient to choose these details without guessing. Until A2 supplies exact reachability and source-backed terms, these fields are `BLOCKING_EVIDENCE_GAP`.

This gap blocks **architecture acceptance**, not the Option B recommendation itself.

## 11. SQL/query profile and minimal seam

Consumer-first repairs are required before broad dependency changes:

1. route every production semantic persistence operation through one sealed executor entrypoint;
2. replace repeated relation-lock awaits with one static multi-table `LOCK TABLE ... IN EXCLUSIVE MODE` preserving the exact required order/mode;
3. bound migration compatibility inspection to embedded migration count + one sentinel and exact checksum size;
4. replace pending-command child `fetch_all()` materialization with the accepted bounded ordered aggregate per exact attempt;
5. protect variable `record_json`/other variable values in the same DB snapshot before transfer;
6. bound exact-cardinality reads with bounded sentinel shapes rather than unbounded materialization;
7. normalize full SQLx errors inside active custody into bounded Durability dispositions and destroy unbounded driver error state before active release;
8. preserve the selected finite statement cache only after the exact production query corpus and metadata byte footprint are classified.

Minimal SQLx/PostgreSQL seams remain allowed only where the exact consumer/profile cannot close the boundary:

- backend frame length before receive-buffer reserve;
- DataRow/RowDescription count/profile gate before vector allocation;
- bounded/allowlisted retained `ParameterStatus`;
- bounded selected statement/type/table metadata behavior;
- prospective socket-buffer growth checks if consumer/root reservation cannot prove them externally;
- exact transport/TLS-name/profile seam if required;
- bounded internal logging if tracing/profile controls cannot close the sink;
- one stable awaited return/finality observation seam only if source proof cannot otherwise close the root lifetime.

Generic Tokio DNS/blocking-worker redesign, broad rustls internal container ownership and unrelated Tokio TCP/UDS semantic changes are outside the selected product architecture unless A2/A4 produces a new exact blocking proof.

## 12. Cancellation, rollback, COMMIT ambiguity and reconciliation

- one absolute 2-second DB pass deadline covers ready-only checkout, custody fence, required locks, every SQL await, COMMIT/reconciliation work in that pass;
- server-side `lock_timeout`/`statement_timeout` may be derived from remaining pass time as a cleanup aid, but never as proof of rollback/finality;
- no hidden automatic retry occurs inside an active pass;
- transaction drop only queues rollback and therefore preserves custody;
- lost response after COMMIT is `AMBIGUOUS`, never inferred rollback;
- reconciliation uses the original immutable operation/replay identity and accepted current fences; it may not mint a replacement identity;
- completion/cleanup capacity is reserved before irreversible COMMIT;
- if the connection becomes unusable, root ownership retains/fences/retire-closes it and root maintenance later restores readiness under a new bounded connect transient;
- a later semantic retry is a new authorized operation/reconciliation action under accepted authority, not dependency retry/backoff hidden in the original pass.

## 13. Restart/takeover and predecessor-work fencing

The executor has one durable generation/fence and two durable pending custody slots.

On process restart/takeover:

1. load/validate the exact executor generation before admitting new work;
2. load at most the two accepted pending slots under bounded row/value rules;
3. reconstruct logical active custody without creating extra active capacity;
4. reconcile predecessor work by original operation identity and exact current DB authority before any replacement semantic effect;
5. never blind-reset an uncertain `Starting`/takeover state;
6. fence stale predecessor process/generation work before accepting a new executor generation;
7. clear durable pending + active + recovered in-memory copies only through the definitive release/ack protocol.

Any current branch behavior that leaves `RuntimeRegistration::Starting` permanently fail-closed after uncertain initialization is acceptable as an intermediate safety state, not a complete operational recovery design. A4 must implement a reviewed takeover/recovery path without permitting duplicate capacity.

## 14. Q01-Q75 qualification disposition

The canonical `WP3-Q01..WP3-Q75` matrix remains binding and is not replaced by this decision.

For the selected Option B, A4 must produce exact composed-consumer evidence that terminally classifies every applicable Q01-Q75 cell. In particular:

- Q01/Q61: exact resolved production features/provider;
- Q08/Q18/Q60/Q63/Q64: one root identity, lower-layer tails and release order;
- Q17/Q29/Q30/Q32: DNS, credentials, hostname/TLS and complete startup/auth profile;
- Q31-Q35/Q55: PostgreSQL counts/buffers/status/caches/retained rows;
- Q38/Q65: lost COMMIT response and pre-funded ambiguity/completion;
- Q39/Q47-Q54: simultaneous root fit, bootstrap/pool/return/close;
- Q49/Q50/Q67: bounded configuration, redaction and rotation overlap;
- Q56-Q59/Q71-Q75: sequential reuse, two occupied logical slots, production construction, exact SQL/lock corpus, custody transfer, restart/takeover and retained configuration lifetime.

The Revision-4 audit also records later candidate refinements Q76-Q84. Those are carried forward as additional qualification obligations where applicable, but they do not silently renumber or replace the canonical Q01-Q75 package established by PR #588.

`Q01-Q75 green` means exact terminal evidence for the final composed consumer, not document completion, skipped tests or local-only PASS.

## 15. #356 retention/supersession map

PR #356 stays open as retained evidence until the replacement path is protected and its salvage/removal is reviewed.

### RETAIN

- `ResourceBudget` / `ResourceReservation` ownership primitives where the selected root/minimal seams still use them;
- exact charged backing/finality primitives that remain necessary after consumer-first narrowing;
- PostgreSQL 17.6/TLS harness and hostile vectors;
- source census, provenance/supply-chain records and bound derivations;
- valid runtime/thread/socket/TLS finality research;
- tests that remain applicable to the selected exact production profile.

### REWORK / NARROW

- SQLx core fork to only exact holder/profile/finality seams that cannot be closed without it;
- SQLx PostgreSQL fork to frame/count/status/cache/profile seams required by the final query/auth profile;
- current direct owner-aware connection experiment into root-owned serialized establishment if its primitives remain useful.

### HISTORICAL_EVIDENCE_ONLY / CANDIDATE REMOVAL AFTER REPLACEMENT PROOF

- generic Tokio blocking-owner expansion not reachable/required by the frozen product profile;
- broad rustls internal ownership instrumentation that the finite root/connect reservation can replace;
- generic DNS/TCP/UDS dependency changes outside the accepted production profile;
- amendments whose only purpose was to make per-operation direct connection ownership match a lifecycle the final consumer no longer uses.

### SUPERSEDED

- the idea that #356's broad operation-owned direct-connection model is itself terminal WP3 architecture;
- earlier `min_connections = 2 / max_connections = 2` or generic `max_connections <= 2` first-slice recommendation;
- any architecture assumption that submitter timeout, transaction drop, `after_release`, pool wrapper drop or SQLx close alone proves full resource finality.

No vendor patch is deleted merely because this candidate says it is removable. Removal happens only in A4 after replacement evidence proves the exact final consumer and preserves required tests/provenance.

## 16. Security and cross-repository composition

This decision is Game WP3 architecture only.

It does not change Platform authority, source provenance, witness semantics, source age, native-source transport or mTLS rules. Cross-repository R01-R21 remain qualification constraints.

Especially:

- PostgreSQL, gameplay TLS, Platform native-source mTLS and client HTTPS are separate profiles and must not share proof by analogy;
- source freshness must be checked at final authority use after queue/transport/DB waiting;
- any DFR/native-source resource wait cycle must be tested rather than solved by silently adding slots/executors;
- one physical DB connection is selected for first-slice simplicity, **not** claimed sufficient solely because current relation locks serialize writes;
- final Game+Platform S3/G0 evidence must bind exact compatible Game and Platform heads with real authenticated transport.

## 17. Acceptance gates for this architecture candidate

This candidate may advance to repository architecture acceptance only after all of the following are attached to the exact decision head:

1. A2 read-only exact-source evidence package covering the complete reachable SQL corpus and exact lock footprint with no silently unclassified production query.
2. Exact production PostgreSQL startup/authentication allowlist from configuration through `ReadyForQuery`.
3. Exact resolved Cargo/TLS/provider/runtime profile and reachability classifications.
4. Explicit finite pool retirement/maintenance policy, including exact idle/max-lifetime behavior and proof that `min_connections = 0` cannot create automatic replacement.
5. A source-backed plan for lower-layer reactor/provider finality: finite term in `I/T` or one narrowly justified seam.
6. The Option B-vs-C comparison is independently reviewed with R21 respected; no claim that max-one is performance-qualified before measurement.
7. #356 retention/supersession map is independently reviewed for evidence loss and upgrade/supply-chain consequences.
8. Exact-head repository/governance checks and required independent high-risk architecture review are green.
9. Normal protected integration/readback occurs before A4 receives material implementation authority.

Until items 1-7 are closed, terminal marker is:

```text
WP3_V2_ARCHITECTURE_CANDIDATE_OPTION_B
BLOCKING_EVIDENCE_GAP = A2 exact profile/corpus/finality evidence
ARCHITECTURE_ACCEPTED = NO
IMPLEMENTATION_AUTHORITY = NONE
```

Once those evidence gaps are closed without changing the selected invariants, the intended worker terminal outcome becomes:

```text
WP3_V2_ARCHITECTURE_READY_FOR_ACCEPTANCE
```

## 18. Implementation impact after protected acceptance

Only after Gate 1 acceptance/allocation:

- A4 becomes the sole WP3 material writer;
- repair the executor/consumer boundary first;
- freeze exact config/TLS/provider/query profile;
- derive and measure `I/R/T/A` on that exact consumer;
- implement only the remaining minimal dependency seams;
- qualify canonical Q01-Q75 plus applicable carried-forward refinements;
- preserve #356 evidence until replacement proof is complete;
- integrate only through current protected review/CI/Merge Queue controls.

This architecture candidate itself performs no runtime, Cargo, vendor, SQL, workflow, Platform, production, secret, merge or Merge Queue mutation.
