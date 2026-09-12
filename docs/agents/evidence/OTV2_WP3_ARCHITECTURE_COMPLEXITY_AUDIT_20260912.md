# OTV2 WP3 — Architecture / Complexity Audit and Recovery Decision

- Date: 2026-09-12
- Repository: `Oteryn/Oteryn-Game`
- Scope: WP3 SQLx/TLS resource accounting
- Canonical issue at audit/save readback: `#351`
- Canonical pull request at audit/save readback: `#356`
- Protected `main` at save readback: `489e3e390a1bce1ce3439c66521ab75f8a826cd8`
- WP3 PR HEAD at save readback: `fe7891989b1247012e32c89c10cff6a10bacb943`
- Evidence class: retained architecture audit evidence
- Architecture authority: **none** — this file does not itself accept or supersede architecture

## 1. Executive verdict

```text
WP3 architectural health:
RED

Recommended WP3 path:
REDESIGN_BOUNDARY

Confidence:
HIGH
```

The audit concludes that WP3 is not merely missing a small number of reservations. The implementation has expanded across SQLx Core, SQLx PostgreSQL, rustls, Tokio blocking runtime internals, networking/reactor lifetime, thread metadata and DNS while several required allocation/finality boundaries still have no complete executable proof.

Passing CI is real and valuable evidence, but it is not equivalent to complete resource-accounting proof. The current positive PostgreSQL/TLS qualification demonstrates one important supported path; it does not prove that every allocation, every accepted TLS mode, every retained result, every cancellation/shutdown path or every opaque runtime allocation is prospectively bounded and held through physical backing finality.

The recommendation is therefore **not** to throw away the whole WP3 implementation. Preserve the narrow `ResourceBudget`/reservation foundation, sound custody/finality repairs and the real PostgreSQL/TLS tests. Stop growing the design by local micro-fixes until the operation-versus-infrastructure boundary and physical backing finality model are explicitly resolved.

The primary decision is:

```text
WP3_BOUNDARY_REDESIGN_REQUIRED
```

## 2. Verified live state

### 2.1 Repository state

At the final audit/save readback:

| Item | State |
|---|---|
| Repository | `Oteryn/Oteryn-Game` |
| Protected `main` | `489e3e390a1bce1ce3439c66521ab75f8a826cd8` |
| WP3 Issue | `#351` |
| WP3 PR | `#356` |
| PR state | `OPEN`, `DRAFT`, `UNMERGED` |
| PR branch | `agent/sqlx-driver-budget-351` |
| PR HEAD | `fe7891989b1247012e32c89c10cff6a10bacb943` |
| PR commits | 215 |
| Changed files | 928 |
| GitHub diff stats | `+257002 / -24` |

The large addition count is dominated by imported/patched vendored dependencies and must not be described as 257k lines of newly authored Oteryn production code.

The earlier deep source inspection was performed primarily at `e88c7cf175e068e798d5aaed1556ac0b8951558a`. Comparison to `fe7891989b1247012e32c89c10cff6a10bacb943` showed only non-vendor changes in the intervening commits, so the inspected vendor findings remain applicable to the final audited HEAD.

### 2.2 Applicable repository rules

PROVEN:

- root `AGENTS.md` makes `Oteryn/Oteryn-Game` the current Game write authority;
- live GitHub state governs task lifecycle;
- repository-native GitHub APIs/CI are preferred;
- `docs/agents/` is appropriate for routed procedures, task records and retained evidence;
- accepted architecture belongs under `docs/architecture/`;
- evidence must distinguish `PROVEN`, `DERIVED`, `UNKNOWN` and `CONFLICT`;
- passing tests may not be used as a substitute for architecture quality or required proof.

This report is therefore retained under `docs/agents/evidence/` and is **not** written as an accepted architecture decision.

### 2.3 CI evidence

PROVEN from exact-head GitHub Actions evidence for `fe7891989...`:

- Merge gate run `34699293933`: success;
- Agent governance run `34699293903`: success;
- Architecture semantic audit run `34699293887`: success;
- Linux workspace job `103568134958`: success;
- exact HEAD checked out in CI;
- Rust 1.94 used;
- PostgreSQL 17.6 service used;
- build, strict workspace Clippy, workspace tests, Durability PostgreSQL E2E, synthetic harness and server smoke passed;
- `durability_postgres` completed `125 passed; 0 failed` in the explicit PostgreSQL step;
- `oteryn_resource_budget::owner_aware_aws_lc_tls_positive_and_denial_qualification` passed.

DERIVED:

This proves the specific exercised PostgreSQL/TLS qualification path works and that the current integration does not fail the repository's selected CI. It does **not** prove completeness of accounting hooks or correctness of unexercised accepted paths.

## 3. Real WP3 responsibility

> WP3 exists to make the accepted PostgreSQL/TLS operation path prospectively account for operation-retained resources before allocation, keep the debit until the real backing memory is gone, preserve transport/security/database semantics, and expose a narrow resource result to downstream durability work without leaking dependency internals.

Required properties:

1. reserve before size-controlled allocation;
2. use checked actual capacity and account overlapping old/new allocations during replacement;
3. retain custody through clones, caches, returned rows/values, errors, cancellation, close and asynchronous task ownership;
4. release only after the actual backing is destroyed;
5. preserve PostgreSQL semantics, TLS certificate/hostname verification and accepted transport behaviour;
6. maintain one accepted root resource envelope rather than multiplying per-connection allowances;
7. distinguish timeout/cancellation from actual physical completion;
8. fail closed when a dependency allocation cannot be bounded or finalized defensibly.

The accepted resource-envelope decision establishes a first-slice charged resident-work ceiling of 12 MiB per logical executor from queue plus active work, not a 12 MiB process-RSS guarantee and not an extra 12 MiB entitlement for each connection.

## 4. Current architecture

The current branch patches four upstream crates through root `[patch.crates-io]`:

- `vendor/sqlx-core-0.9.0`;
- `vendor/sqlx-postgres-0.9.0`;
- `vendor/tokio-1.53.1`;
- `vendor/rustls-0.23.43`.

The architecture currently consists of:

1. `ResourceBudget` / `ResourceReservation` / `Charged<T>` in SQLx Core;
2. `BudgetOwner` / `BlockingJobOwner` for resource-funded blocking execution;
3. owner-aware Tokio blocking queues/workers;
4. extensive rustls custody/accounting additions for handshake decode, retained chunks, certificate/session backing and provider-owned allocations;
5. SQLx PostgreSQL plumbing that carries a resource budget into connection/TLS establishment;
6. a real PostgreSQL 17.6 + TLS1.3 test harness with positive and denial qualification.

The critical end-to-end lifecycle is larger than TLS handshake alone:

```text
operation admission
  -> hostname/DNS
  -> TCP or UDS connect
  -> Tokio reactor registration
  -> optional PostgreSQL TLS negotiation
  -> rustls handshake / roots / certs / crypto / tasks
  -> final socket boxing
  -> BufferedSocket read/write backing
  -> PostgreSQL startup/authentication/status
  -> query encoding / write buffering
  -> PostgreSQL message decoding
  -> row/type/statement metadata
  -> returned Bytes/PgValue/PgRow descendants
  -> cache/error/cancellation/close
  -> final physical backing destruction
  -> debit release
```

This full lifecycle is the standard the implementation must satisfy.

## 5. Architecture problems

### WP3-A01 — HIGH — test and production ownership differ inside rustls

PROVEN:

`vendor/rustls-0.23.43/src/msgs/message/mod.rs` compiles `Message::decoded_custody` only under `std && !test`. Under `cfg(test)`, `decoded_owner()` returns `None`, and decoded-copy owner propagation is similarly bypassed.

Impact:

Library tests compiled with rustls's own `cfg(test)` do not exercise the exact ownership representation used by production. Cross-crate tests can still exercise production compilation, so this does not invalidate every existing test, but it weakens internal qualification.

Required closure:

Use one ownership representation and semantics in test and production builds; fix fixtures/tests rather than compiling the mechanism away.

### WP3-A02 — HIGH — accepted TLS verifier wrappers lack owner-aware supported-scheme allocation

PROVEN:

SQLx's `DummyTlsVerifier` and `NoHostnameTlsVerifier` implement ordinary `supported_verify_schemes()` but not the owner-aware variant. rustls's owner-aware default fails closed rather than invoking the ordinary allocating method.

DERIVED with high confidence:

Budget-aware TLS paths using those wrappers can fail even with sufficient budget when the owner-aware client-hello path requests supported verification schemes. This requires runtime confirmation for each affected `PgSslMode`, but the source chain is concrete.

Required closure:

Qualify every accepted `PgSslMode` with owner-aware verifier behaviour, without weakening certificate or hostname verification semantics.

### WP3-A03 — HIGH — the positive helper does not prove the actual consumer feature profile

PROVEN:

The workspace SQLx dependency is declared with `tls-rustls-ring-webpki`, while the resource-owned SQLx TLS path is explicitly qualified around the AWS-LC owner-aware provider. The qualification helper builds a separate AWS-LC-oriented manifest/profile.

Impact:

A successful helper proves the patched crates can compose in that helper profile, not that the exact future production Game consumer feature graph has been selected and qualified.

Required closure:

Freeze the supported consumer feature/profile and test the real Game composition.

### WP3-A04 — HIGH — PostgreSQL decoding/metadata/cache accounting remains incomplete

PROVEN examples:

- `DataRow::decode_body` builds `Vec<Option<Range<u32>>>` from peer-controlled field count without owner-aware reservation in that function;
- `RowDescription` and connection establishment contain additional retained collection/string allocations;
- SASL, statement/type metadata and connection status/cache surfaces add further allocation families.

Important qualification:

Some of these functions are unchanged upstream code. The defect is that WP3's promised complete resource-accounting boundary does not yet include them; this is not necessarily a regression authored by the worker who imported them.

Required closure:

Publish and satisfy one complete PostgreSQL allocation/lifetime matrix covering establishment, authentication, decode, metadata, cache, errors and returned values.

### WP3-A05 — HIGH — returned values can outlive the query and connection

PROVEN:

`PgValue` owns `Bytes`; `PgValueRef::to_owned` may retain a shared slice of a row backing or allocate a new copy. `PgRow` retains row data and shared metadata.

Impact:

A reservation owned only by `PgStream`/socket/query cannot truthfully be released when a returned value still shares the backing. Conversely, a new deep copy requires prospective admission before allocation even though existing upstream APIs may be infallible.

Required closure:

Tie the debit to physical shared backing or introduce a bounded/fallible copy boundary where required. Test values/rows surviving connection drop.

### WP3-A06 — HIGH — owner-aware TLS buffering coexists with ordinary/unowned buffering

PROVEN:

`ChunkVecBuffer` contains ordinary append behaviour and an `unowned_tail` in addition to owner-aware preparation/custody.

Impact:

Ordinary upstream operation is legitimate outside the budgeted profile, but the budgeted profile must prove that no reachable write path silently falls back to unowned storage.

Required closure:

Make owner-aware and owner-free regimes structurally distinct or exhaustively prove every reachable budgeted insertion path.

### WP3-A07 — HIGH — BudgetOwner Arc control-block finality is not proven by the current representation

PROVEN:

`BudgetOwner` stores the reservation that accounts for the allocation containing the `BudgetOwner` itself. That owner lives inside `Arc`.

Problem:

Dropping the inner value and its reservation is not the same event as deallocating the `Arc` control block. A debit stored inside the same allocation it accounts for can be released before the physical control allocation is gone.

The protected runtime-owner finality decision already selected a safer protocol based around controlled handles and `Arc::into_inner`/final-owner semantics, but the audited branch still requires complete implementation/qualification of that protocol across all strong-owner paths.

Required closure:

No raw strong-owner bypass, no Weak/raw escape that invalidates finality, and exact release after control-block finality.

### WP3-A08 — HIGH — worker/thread lifecycle still has unresolved exceptional finality

PROVEN:

The owner-aware Tokio worker carries `worker_charge` through the worker closure and returns it through the normal join path. Timeout shutdown also has fail-closed retention behaviour rather than a completed reclaim protocol.

DERIVED:

Normal return is much stronger than the original design, but exceptional lifecycle paths still require proof: callback panic/unwind, shutdown timeout, failed spawn, runtime drop and any thread metadata that can escape the join boundary.

Required closure:

One owner protocol covering success, error/unwind and timeout/reaper behaviour. Physical thread capacity and memory debit cannot become available before the actual backing is gone.

### WP3-A09 — HIGH — DNS, reactor registration and std thread metadata remain architecture boundaries

PROVEN:

The corrected connect/socket owner amendment separates:

- E1: final socket Box + BufferedSocket/shared Bytes custody;
- E2: Tokio reactor registration for TCP/UDS;
- E3: DNS hostname resolution.

The amendment explicitly preserves architecture blocking when resolver preallocation cannot be defensibly bounded. A separate unresolved architecture seam remains for owned thread metadata/spawn finality.

Impact:

Continuing to add SQLx/rustls local reservations does not solve allocation/finality that occurs in opaque or longer-lived infrastructure below those libraries.

Required closure:

Explicitly decide whether these resources are operation-owned or finite shared infrastructure charged inside the accepted root envelope; then prove prospective cost and finality.

### WP3-A10 — HIGH — adding budget failure changes error/rollback assumptions

PROVEN:

Existing SQLx paths contain operations that assume valid protocol-control writes are infallible, including rollback queuing that uses `expect`.

Impact:

If WP3 merely makes the buffer fallible without redesigning the surrounding control flow, resource exhaustion can turn a formerly infallible cleanup path into panic or ambiguity. A resource failure after bytes may have been sent also cannot be reported as proof of non-commit.

Required closure:

Design encoding/control/rollback/close and ambiguous-commit behaviour as one failure contract.

### WP3-A11 — HIGH — PostgreSQL TLS harness target binding needs hardening

PROVEN:

The TLS test connects using an admin URL and executes `ALTER SYSTEM`; container discovery by published port is separate from the database URL identity.

Impact:

The current CI uses an isolated PostgreSQL service, so there is no claim that it modified an unintended database in the observed run. The harness nevertheless needs a fail-before-connect guard that proves the configured URL is the exact intended isolated test instance before privileged mutation.

Required closure:

Bind URL, test identity and container instance before `ALTER SYSTEM`, and guarantee cleanup after partial failure.

### WP3-A12 — MEDIUM — a clean ledger does not prove hook completeness

PROVEN:

The positive/denial helper validates the budget hooks that it exercises and checks post-drop ledger state.

DERIVED:

An allocation site that never calls the hook is invisible to the ledger. Therefore `used == 0` after drop is necessary but not sufficient evidence that all operation-retained allocations were covered.

Required closure:

Combine source census, stage-by-stage forced denial and at least one independent memory/allocation observation strategy.

## 6. Essential vs accidental complexity

### Essential complexity

The following complexity is inherent in the accepted requirement:

- accounting old/new allocation overlap during growth/replacement;
- keeping debit through shared backing descendants;
- distinguishing timeout from physical completion;
- handling cancellation/error without losing custody;
- preserving TLS verification and PostgreSQL semantics;
- coordinating connection, TLS, driver and runtime lifetimes;
- maintaining one finite root envelope across queued and active work.

### Accidental complexity

The following complexity comes primarily from the present implementation boundary:

- repeatedly adding local reservations because ownership begins too late or ends too early;
- multiple phase-specific owner constructs that risk becoming distinct execution identities;
- test-only removal of production ownership semantics;
- coexistence of owned/unowned paths without structural separation;
- trying to map short operation lifetime directly onto long-lived/opaque runtime internals;
- micro-fixes that move the finality seam deeper into Tokio/std rather than closing it once.

The core concern is therefore not code size. It is that the present boundary makes every newly discovered allocation family another bespoke integration problem.

## 7. Minimum professional architecture

The smallest credible target should have four explicit layers.

### 7.1 Root budget authority

One accepted root/ledger defines the complete first-slice resource envelope. No dependency-local hidden second budget and no implicit free pool.

### 7.2 One operation execution identity

Create one operation/execution identity before any operation-owned connect work and preserve it across DNS/connect/registration/TLS/blocking/decode. Do not construct separate identities that multiply queue/worker semantics.

### 7.3 Physical-backing custody

Every charged allocation has one custody object or equivalent finality protocol tied to the actual backing. Shared descendants retain one charge until the final backing owner disappears. Replacement charges destination before allocation and holds both old/new debit during overlap.

### 7.4 Explicit shared infrastructure, only if architecture accepts it

If reactor/thread/resolver internals cannot honestly have operation lifetime, define a finite shared infrastructure profile funded inside the accepted root envelope. It must have:

- finite prospective maximum;
- explicit owner;
- exact lifecycle/finality rule;
- proof that operations cannot multiply it beyond the accepted envelope;
- verification independent of convenience wrappers.

This is a new architecture decision, not permission to label runtime overhead free.

## 8. Strategy comparison

| Strategy | Reuse | Implementation risk | Regression risk | Long-term maintenance | Test burden | Relative effort |
|---|---|---|---|---|---|---|
| Continue current architecture | Highest | High and unpredictable | High | Highest debt risk | Continuously expanding | High/unbounded |
| Salvage + simplify | High | Medium | Medium | Better, but opaque seams remain | Medium-high | Medium-high |
| Partial rewrite after boundary decision | Preserve contracts/tests/core; replace owner/lifecycle center | Medium-high but bounded | Medium | Best likely maintainability | High but finite | High |
| Clean rewrite | Lowest | Highest | Highest | Unknown; same dependency constraints remain | Highest requalification | Highest |

Primary recommendation remains `REDESIGN_BOUNDARY`. After that decision, the likely implementation strategy is selective partial rewrite/salvage, not indiscriminate clean rewrite.

## 9. Keep / simplify / rewrite / delete matrix

| Component | Classification | Reason |
|---|---|---|
| `ResourceBudget` | KEEP | Narrow, useful single admission abstraction. |
| `ResourceReservation` | KEEP | Correct RAII direction; supports checked transfer/growth. |
| `Charged<T>` | KEEP | Field/drop ordering explicitly ties backing destruction before reservation release. |
| `BlockingJobOwner` / runtime owner finality | REWRITE | Current Arc/control-block lifetime requires the protected finality protocol. |
| Owner-specific blocking queue identity | KEEP_BUT_SIMPLIFY | Useful only if exactly one operation identity flows end-to-end. |
| Normal external worker join ownership | KEEP_BUT_SIMPLIFY | Preserve correct normal finality; add exceptional/timeout proof. |
| Permanent `mem::forget` timeout retention as final architecture | DELETE_AFTER_REPLACEMENT | Safe as fail-closed temporary behaviour, but not an acceptable reclaim design. |
| rustls decoded/retained custody machinery | KEEP_BUT_SIMPLIFY | Several sound fixes exist; remove test/prod and owned/unowned ambiguity. |
| `RetainedCertificateChain` Arc finality repair | KEEP | Uses final-owner/`Arc::into_inner` direction rather than strong-count snapshot. |
| PgStream/BufferedSocket owner path | KEEP_BUT_SIMPLIFY | Must be completed through final Box, growth, shared Bytes and result lifetime. |
| PostgreSQL decoders/metadata/cache integration | REWRITE/COMPLETE | Core required area remains outside complete accounting. |
| Real PostgreSQL/TLS qualification | KEEP | High-value integration evidence; expand profile/matrix and harden harness. |
| std thread metadata/spawn accounting | UNCERTAIN | Requires architecture decision/feasibility proof. |
| DNS resolver accounting | UNCERTAIN | Must remain blocked if opaque preallocation cannot be bounded. |

## 10. Recommended target architecture

```text
Accepted DFR root budget / ledger
│
├── finite shared infrastructure owner (only if explicitly accepted)
│   ├── reactor registration backing
│   ├── worker/thread infrastructure backing
│   └── resolver infrastructure backing
│
└── operation slot owner
    └── one execution identity
        ├── hostname / connect
        ├── TCP or UDS registration
        ├── TLS handshake / loaders / crypto
        ├── socket Box + BufferedSocket
        ├── PostgreSQL auth / message decode / metadata
        └── returned row/value/shared Bytes custody
            └── final physical backing owner releases debit

error / cancel / timeout
    -> stop creating new operation-owned work
    -> preserve ambiguous durable identity when required
    -> prove actual task/socket/thread/backing finality
    -> only then release debit
```

This boundary should be understandable without reconstructing the history of 200+ commits.

## 11. Exact next steps

### S1 — architecture decision for resource boundary and supported consumer profile

Decide now:

- exact Game SQLx/TLS provider/features/profile;
- operation-owned versus finite shared-infrastructure resource classes;
- std thread metadata strategy;
- DNS strategy;
- reactor-registration strategy;
- timeout finalization strategy.

Acceptance:

Every mandatory resource class has a prospective finite cost, an owner, a finality event and a verification method. Any unresolved class remains explicitly blocked rather than assigned a guessed constant.

### S2 — prove the hardest seams before expanding code

Run focused feasibility proofs for:

- `std::thread::Thread` clones surviving `JoinHandle::join`;
- private std thread-spawn allocations;
- DNS resolver preallocation/finality;
- Tokio reactor-held registrations;
- shared `Bytes` descendants surviving query/connection drop.

Acceptance:

A failed feasibility proof rejects the proposed architecture instead of spawning another chain of local patches.

### S3 — implement protected runtime-owner finality

Implement the selected handle/final-owner protocol across all owned strong references.

Acceptance:

No raw/Weak bypass; exact one-time release after control allocation finality; forced failure and drop tests.

### S4 — complete active E1 in the canonical writer

Scope:

- final socket Box precharge;
- BufferedSocket initial read/write backing;
- growth/replacement/shrink overlap;
- shared `Bytes` finality;
- reuse the same pre-existing operation owner; do not construct a second execution owner.

Acceptance:

Every E1 allocation occurs after reservation and charge survives until final backing release.

### S5 — activate/resolve E2 only with proper shared-path authority

Cover TCP and UDS reactor registration, pending-release driver references, deregister/failure/cancel/shutdown paths.

Acceptance:

Dropping the SQLx wrapper does not release debit while the reactor still retains backing.

### S6 — resolve E3 or preserve architecture block

Hostname resolution must preserve normal hostname semantics. No IP-only shortcut and no guessed magic cap.

Acceptance:

Either prospective resolver accounting is proven, or `ARCHITECTURE_BLOCKED_DNS_RESOLVER_PREALLOCATION` remains truthful.

### S7 — complete TLS and PostgreSQL allocation census

TLS minimum matrix:

- accepted TLS versions;
- all accepted `PgSslMode` values;
- VerifyFull/VerifyCa/Require/Prefer semantics;
- root and client cert/key loading;
- verifier supported schemes;
- transcript/handshake state transitions;
- ALPN where reachable;
- HRR/resumption/binder paths where accepted;
- retained cert/ticket/session backing;
- outbound buffers and errors.

PostgreSQL minimum matrix:

- startup/auth/SASL;
- parameter/status/error/notice/notification;
- `DataRow` and `RowDescription`;
- statement/type metadata;
- caches;
- query write buffers;
- returned `PgRow`/`PgValue`/`Bytes` descendants;
- rollback/close after resource failure.

Acceptance:

Every allocation family is classified as operation charged, finite shared charged, owner-free/not reachable in the accepted profile, or blocked. No unexplained fifth category.

### S8 — strengthen qualification

Required verification cases include:

- successful exact consumer composition;
- resource denial at multiple later reservation points, not only root-zero denial;
- old/new buffer overlap;
- returned values surviving connection drop;
- cancellation at each phase;
- timeout while work continues;
- forced worker spawn failure;
- worker callback panic/unwind where applicable;
- reactor/deregister failure;
- TLS verification failures;
- invalid cert/hostname;
- rollback after memory pressure;
- ambiguous/lost COMMIT response;
- two active slots plus full queue against one root budget;
- accepted semantic maxima without hidden truncation;
- independent observation capable of detecting an unhooked allocation;
- safe harness target binding;
- performance/churn comparison against the comparable upstream profile.

### S9 — final exact-head whole-diff qualification

Before integration:

1. exact HEAD frozen;
2. complete authored-delta versus upstream package census;
3. independent whole-diff review;
4. exact consumer PostgreSQL/TLS qualification;
5. applicable workspace/vendor tests;
6. required CI/gates;
7. Merge Queue only through repository policy;
8. protected-main readback after queue integration.

## 12. Integration impact

### WP2

WP2's already integrated PREPARE/COMMIT/reconnect work should not be restarted because of WP3. WP3 must preserve its durability/ambiguity semantics.

### WP4

WP4 must remain owner of durable atomicity/reconciliation policy. WP3 should expose resource/failure facts, not force WP4 to understand rustls/Tokio internal owners.

### WP5

WP5 source readiness and real owning-source qualification remain separate. Completing driver resource accounting does not manufacture missing production authority sources.

### G0

Fresh G0 requires the composed exact state, not a historical sum of individually green pieces.

### Server Seam

Server Seam remains blocked from a truthful final readiness claim until WP3, WP4, WP5 and the required composition/qualification are simultaneously satisfied. A green WP3 helper or E1 alone is not Server Seam readiness.

## 13. Verification matrix summary

The audit tracks 46 closure requirements (`WP3-Q01` … `WP3-Q46`) across:

- root budget/ledger;
- operation identity;
- owner finality;
- thread lifecycle;
- DNS;
- TCP/UDS reactor registration;
- socket Box and BufferedSocket;
- shared Bytes;
- TLS modes/versions/states/certificates;
- PostgreSQL decode/auth/status/cache/results;
- rollback/error/ambiguous COMMIT;
- concurrency/envelope limits;
- harness safety;
- portability/regression;
- performance;
- supply-chain/upstream delta;
- final exact-head integration.

These are closure requirements, not claims that 46 new tests already exist or passed.

## 14. Known audit limitations

UNKNOWN / not fully proven by this audit:

- exact authored production LOC versus imported vendored LOC;
- exact authored test LOC;
- complete normalized delta against every upstream crate archive;
- full own-code unsafe/FFI census;
- new Loom/Miri or equivalent deterministic concurrency proof;
- independent allocation profiler/allocator-instrumented completeness proof;
- soak/p99/CPU/memory performance comparison;
- second independent final auditor of the final future implementation.

The repository connector/API could read exact selected source files, review state and Actions evidence, but the audit environment could not obtain a complete local Git checkout because of network/DNS restrictions. Therefore the report intentionally does not claim a literal every-line local audit.

These limitations do **not** weaken the primary architectural finding: several already-identified mandatory seams are unresolved and sufficient to make current architecture integration a NO-GO.

## 15. Final go/no-go

### Decision

**NO-GO for integrating WP3 as a completed architecture in its audited state.**

**GO for continuing the canonical WP3 lane only under a boundary-redesign/recovery programme that preserves correct work and stops adding unbounded micro-fixes.**

The strongest reasons are:

1. incomplete PostgreSQL decode/result lifetime accounting;
2. unresolved runtime owner/control-block finality;
3. unresolved thread metadata/spawn finality;
4. DNS and reactor-registration allocation boundaries;
5. test/production ownership divergence inside rustls;
6. unqualified accepted TLS verifier/mode paths;
7. helper-versus-real-consumer feature/profile mismatch;
8. insufficient proof that all allocation sites are hooked;
9. failure/rollback APIs that must be redesigned coherently when allocation becomes fallible;
10. a system boundary that currently encourages repeated local repair instead of one stable ownership model.

Positive evidence to preserve:

- one root budget/reservation primitive;
- sound RAII/custody ideas;
- selected final-owner repairs such as certificate-chain finality;
- normal owned-worker join improvements;
- real PostgreSQL 17.6/TLS1.3 qualification;
- repository CI/governance discipline;
- corrected connect/socket owner decomposition.

Final marker:

```text
WP3_BOUNDARY_REDESIGN_REQUIRED
```

---

## 16. Consumer-lifecycle reconciliation — merged audit extension

- Addendum readback date: 2026-09-12
- Protected `main` at addendum readback: `489e3e390a1bce1ce3439c66521ab75f8a826cd8`
- WP3 Issue: `#351`
- WP3 Draft PR: `#356`
- WP3 PR HEAD at addendum readback: `fe7891989b1247012e32c89c10cff6a10bacb943`
- Child B / WP4 Draft PR inspected: `#335`
- Evidence class: retained architecture audit evidence
- Architecture authority: **none** — this extension does not activate, accept, supersede or implement architecture by itself

### 16.1 Purpose

This section records findings obtained after the first part of the WP3 architecture audit was saved. The new evidence strengthens the `WP3_BOUNDARY_REDESIGN_REQUIRED` verdict. The most important result is that the current operation-owned connection model does not match the lifecycle of the accepted `PgPool`-based Durability consumer.

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

### 16.2 Executive delta

```text
Parent verdict:
WP3_BOUNDARY_REDESIGN_REQUIRED

Extended verdict:
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

### 16.3 New finding — the production PgPool exists before an active operation slot

#### Classification

`PROVEN`

#### Evidence

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

#### Consequence

The current #356/#583 direction of treating connection establishment as one same-operation owner from DNS through TLS cannot be the only production ownership model.

At minimum the architecture needs an **executor/root-shared resource owner or reservation** capable of funding bootstrap and idle pool residency independently of a later active operation.

Creating a fictitious active slot solely to pay for bootstrap would misrepresent DFR semantics. Replacing the accepted pool topology with one direct connection per operation would also be a material topology change.

### 16.4 New finding — the current operation-owned direct connection is not the production consumer path

#### Classification

`PROVEN`

The WP3 branch exposes an owner-aware direct PostgreSQL connection establishment route intended to avoid changing ordinary pool behavior.

Child B / #335, however, uses `PgPool` through one shared `RuntimeBackend`.

#### Derived conclusion

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

### 16.5 New finding — the DFR owner and driver owner currently exist as disconnected halves

#### Classification

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

#### Consequence

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

### 16.6 Recommended corrected ownership model

#### Classification

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

### 16.7 Source-derived phase reservation is compatible with the accepted contract

#### Classification

`PROVEN / DERIVED`

The accepted DFR decision allows dependency/runtime allocation to be either charged directly or covered by an explicit finite reservation inside the existing envelope.

The historical WP3 plan also contemplated source-derived phase reservation before entering rustls.

#### Consequence

WP3 does **not** inherently require one token attached to every private `Vec`, `Box`, `Arc`, transcript object and provider-internal value.

A defensible architecture can instead prove, for a tightly frozen production profile:

```text
CONNECT_TRANSIENT_BOUND
CONNECTION_RESIDENT_BOUND
ACTIVE_SQL_PEAK
```

reserve them before entering the corresponding phase, and release/convert them at truthful lifecycle boundaries.

This is valid only when each bound is source-derived or measured/proven for the exact profile. A magic whole-slot reserve remains forbidden.

### 16.8 New finding — production SQLx profile and current owner-aware qualification profile diverge

#### Classification

`PROVEN`

The workspace production SQLx feature graph still uses:

```text
tls-rustls-ring-webpki
```

The current owner-aware TLS path has been developed and positively qualified around the AWS-LC profile, and the owner-aware handshake rejects a build that lacks the qualified AWS-LC feature.

#### Consequence

The current helper proves an isolated supported profile, not the frozen production Game profile.

This must be resolved explicitly. Two lawful options are:

1. keep production `ring/webpki` and qualify that exact profile; or
2. separately decide to change the production provider/profile and then qualify the new production graph.

Changing the whole project to AWS-LC merely to make #356 pass is not authorized by this audit.

### 16.9 New finding — several rustls accounting families are unreachable in the current production feature graph

#### Classification

`PROVEN`

The current SQLx rustls dependency uses `default-features = false` and does not enable rustls `brotli` or `zlib` certificate-compression features.

Without configured decompressors, compressed-certificate payloads may still need safe parsing/rejection, but the large certificate decompression destination backing and second decoded certificate pass are not production-reachable allocation families in this graph.

Rustls session resumption is also configurable through public API and can be disabled for a first bounded DB profile.

#### Recommendation

For the first production DB profile, prefer eliminating optional retained allocation families before inventing custody for them:

- keep certificate compression disabled;
- consider `Resumption::disabled()` for DB connections pending measured reconnect cost;
- do not carry production fork code solely for unreachable optional paths.

This is a profile decision, not an implementation authorization.

### 16.10 New finding — Tokio blocking-owner expansion is largely induced by DNS/file-loader choices

#### Classification

`PROVEN / DERIVED`

Tokio 1.53.1 `ToSocketAddrs` resolves a literal `SocketAddr` / `(IpAddr, u16)` synchronously through a ready future. Hostname strings use the blocking resolver path.

SQLx file-backed certificate inputs also use blocking file work; already-loaded/in-memory certificate material does not require the same file-loader path.

#### Recommendation

Evaluate a bounded first-slice connection profile with:

```text
connect_address: SocketAddr
TLS server_name: hostname retained separately for SNI/VerifyFull
CA/root material: preloaded bounded configuration
```

This is not equivalent to disabling hostname verification. Transport routing and TLS identity are separate concerns.

SQLx 0.9 currently does not provide full libpq-like separate `hostaddr + host` semantics because the parsed fields converge on the same host representation. A narrow SQLx seam separating transport address from TLS server name may therefore be required.

If this profile is accepted, the generic Tokio DNS blocking-owner/std-thread cluster can disappear from the production WP3 path instead of being fully forked.

### 16.11 Candidate simplification — dedicated Durability runtime

#### Classification

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

### 16.12 New finding — `PgPool max_connections(4)` is not justified by the current first-slice concurrency model

#### Classification

`PROVEN + DERIVED`

Child B uses `schema::connect_runtime()` with `max_connections(4)`.

The accepted DFR executor has two active slots. The Child B plan intentionally makes no broad concurrency claim and serializes the strongest relation/domain locking protocol.

The V1/V2 shared `RuntimeBackend` repair also avoids the previously observed duplicate-pool topology.

#### Recommendation

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

### 16.13 New finding — active-slot release must include pool cleanup/return lifecycle

#### Classification

`PROVEN`

A `Transaction` obtained from `Pool::begin()` owns a pooled connection. `commit(self)` consumes the transaction. When the pooled connection is dropped, SQLx asynchronously spawns `return_to_pool()`; `after_release`, ping and final idle release happen later.

#### Consequence

The active byte slot cannot truthfully be released merely because `commit().await` returned if operation-owned buffers/results/error/custody can still be involved in asynchronous connection return/cleanup.

#### Recommended lifecycle

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

### 16.14 New finding — Child B itself still violates accepted SQL-result resource policy

#### Classification

`PROVEN`

The accepted DFR decision states that the inherited 64 pending commands must not be treated as 64 returned SQL payload rows. They must be represented as one bounded ordered aggregate per exact attempt while preserving every command identity/disposition.

Current #335 still contains `fetch_all()` paths that retrieve pending command child rows individually and check the returned vector length only afterward.

Current #335 also contains several direct `SELECT state, record_json ...` reads without the accepted SQL-side `octet_length` / bounded projection pattern before transferring the variable-size payload into the driver.

#### Consequence

Even a perfect WP3 dependency fork cannot make Child B DFR-compliant while these consumer queries bypass the accepted row/result transfer rules.

#### Required repair before final WP3/WP4 qualification

- replace individual pending-command row fetches with the accepted bounded ordered aggregate shape;
- guard variable-size `record_json` and analogous payload columns in the same protected SQL snapshot before driver materialization;
- retain exact semantic equality checks after bounded decode;
- exercise max/max+1, corrupted stored state and concurrent substitution negatives on real PostgreSQL 17.6.

These are primarily Child B query-shape repairs, not reasons to deepen the Tokio/rustls fork.

### 16.15 New finding — raw `sqlx::Error` is an unbounded/long-lived completion risk

#### Classification

`PROVEN`

Child B `DurabilityError` contains `Database(sqlx::Error)` and formats the wrapped error.

`PgDatabaseError` retains the decoded PostgreSQL `ErrorResponse` / `Notice`, including variable human-readable message/detail/hint/context/object fields. The full driver error can therefore retain peer-originated backing beyond the immediate receive path and may be formatted into logs or completions.

#### Recommendation

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

### 16.16 New finding — connection-retained PostgreSQL state requires an explicit policy

#### Classification

`PROVEN`

`PgConnection` retains more than the socket:

- statement cache;
- `ParameterStatus` values;
- type/OID/table/array caches;
- row/metadata structures depending on exercised queries.

Some caches are bounded by configuration (statement cache default 100), while others require reachability or explicit policy proof.

Child B currently uses built-in PostgreSQL types and a finite query corpus; current source does not use the general custom `PgTypeInfo::with_name`, `derive(sqlx::Type)` or broad offline type-resolution profile.

#### Recommendation

Do not qualify all theoretical SQLx PostgreSQL features.

Freeze a **B production SQL profile** and prove only its reachable connection-retained state. Keep the default statement cache only if an exact unique-SQL inventory and finite memory bound fit the two resident connection reservations; otherwise reduce/disable it with explicit prepared-statement lifecycle qualification.

Do not use a fixed resident reservation until all retained maps in the accepted profile are either bounded, unreachable, periodically reset, or cause the connection to be closed instead of returned idle.

### 16.17 Pool release hook as a fail-closed simplification

#### Classification

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

### 16.18 Updated disposition of current dependency forks

#### ResourceBudget / ResourceReservation

`KEEP`

The RAII primitive remains useful for exact dynamic boundaries and for phase/root reservations.

#### SQLx PostgreSQL fork

`REWORK / NARROW`

Retain only seams required to:

- bind pool/root and active-slot reservations;
- bound receive/write transfer before peer-controlled growth;
- enforce exact PostgreSQL length/count/result policies that cannot be guaranteed by the B SQL query shape alone;
- separate transport address from TLS server identity if the bounded connection profile is accepted;
- expose required cleanup/high-water information if no upstream API suffices.

#### rustls fork

`PRESERVE AS EVIDENCE, REMOVE FROM FINAL PRODUCT DIFF IF PHASE-BOUND PROOF SUCCEEDS`

The extensive custody work is valuable as source census, hostile vectors and phase-bound evidence. It should not be retained as production complexity merely because effort has already been spent.

#### Tokio blocking-owner fork

`PRESERVE AS EVIDENCE, CANDIDATE FOR REMOVAL`

If the production DB profile uses literal `SocketAddr`, preloaded TLS material and a dedicated bounded Durability runtime, the current DNS/file-loader/thread-owner expansion may no longer be production-reachable.

#### Tokio reactor fork

`NEEDS_DECISION`

A dedicated Durability runtime plus finite root-shared registration reservation may remove the need for per-`ScheduledIo` operation ownership. The exact source-derived reactor bound and runtime footprint still need proof.

### 16.19 Updated proposed WP3-v2 implementation sequence

This sequence does not authorize implementation. It is the recommended shape of the next protected architecture decision.

#### V2-0 — freeze new micro-fix expansion

Do not activate further E1/E2/E3 or rustls/Tokio private ownership amendments merely to continue the current direction until the superseding boundary analysis is decided.

Safe read-only census/tests may continue.

#### V2-1 — superseding architecture decision

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

#### V2-2 — repair the real consumer substrate

On Child B / the final shared RuntimeBackend composition:

- route all affected semantic DB work through the accepted queue/active lifecycle;
- install the real root byte ledger;
- add two active-slot budget views;
- repair pending-command aggregate reads;
- repair SQL-side oversized payload guards;
- normalize driver errors inside the slot;
- make connection cleanup/return part of slot finality.

#### V2-3 — narrow SQLx seams

Implement only the SQLx changes still required after V2-2 and the frozen profile.

Prefer ordinary upstream Tokio/rustls behavior under pre-reserved phase envelopes where proof is sufficient.

#### V2-4 — measure/prove exact bounds

Required values remain `UNKNOWN` until source-derived/measured:

```text
EXECUTOR_RUNTIME_RESIDENT
CONNECTION_RESIDENT × connection_count
CONNECT_TRANSIENT_PEAK
ACTIVE_SQL_PEAK per slot
```

All must compose inside the already accepted root envelope. Do not invent a new maximum to make them fit.

#### V2-5 — complete real qualification

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

### 16.20 Current unknowns

`UNKNOWN` — exact numerical `EXECUTOR_RUNTIME_RESIDENT` bound.

`UNKNOWN` — exact two-connection resident bound under the final B SQL profile.

`UNKNOWN` — exact connect transient bound after optional/unreachable rustls families are removed.

`UNKNOWN` — whether dedicated current-thread runtime footprint plus reactor state is smaller/simpler than retaining a narrow Tokio owner seam; this requires measurement.

`UNKNOWN` — final deployment connection profile, especially whether production may require runtime DNS, file-backed roots/client certificates, UDS or multiple address failover.

`UNKNOWN` — whether current statement-cache default 100 is desirable once exact unique query inventory and memory profile are measured.

No unknown above authorizes inventing a cap or silently weakening TLS/PostgreSQL semantics.

### 16.21 Final reconciled verdict

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

This reconciled section records evidence and recommendation only. Existing protected architecture and live #162/#351 authority remain binding until a separately reviewed/protected decision changes them.
