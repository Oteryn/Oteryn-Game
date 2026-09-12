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
