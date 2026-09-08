# OTV2-20260908-wp3-aws-lc-provider-resident-amendment-439

```yaml
task_id: OTV2-20260908-wp3-aws-lc-provider-resident-amendment-439
title: Resolve WP3 AWS-LC provider-resident lifetime and finite reservation boundary
mode: CONTRACT
status: READY_FOR_REVIEW
repository: Oteryn/Oteryn-Game
base_branch: main
allocation_observed_main_sha: c9cec0f746e549ff96151bcf1e3582522dfea0ee
publication_base_sha: b6411e9bd280a1b48a8c356492a332d084ac7672
branch: arch/wp3-aws-lc-provider-resident-boundary-439
issue: 439
prior_decision_pr: 441
canonical_material_pr: 356
canonical_material_branch: agent/sqlx-driver-budget-351
canonical_material_head_observed: 8a97ed5e0bdfd42b934295ecf3f98584bb49a00f
owner: WP3 architecture amendment lane
allocation_comment: 5588086089
updated_at: 2026-09-08T23:20:00+02:00
execution_policy: continuous_progress
classification: ARCHITECTURE_RESOLUTION
architecture_disposition: B_PROVIDER_FINITE_RESERVATION
activation: architecture-only; material authority begins only after protected integration and fresh bounded application to existing #356
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_WP3_AWS_LC_PROVIDER_RESIDENT_ACCOUNTING_AMENDMENT_2026-09-08.md
  - docs/agents/tasks/active/OTV2-20260908-wp3-aws-lc-provider-resident-amendment-439.md
public_contracts:
  - DUR-FRESH-RESOURCE-ENVELOPE-V1
blocks:
  - WP3 KX continuation in #351/#356 until protected integration
  - dependent WP4 / Server Seam readiness
```

## Outcome

Publish a two-document architecture amendment that removes `BLOCKED_MISSING_PROVIDER_BOUNDARY` only for the exact Linux x86-64 GNU, non-FIPS production graph. It selects **B — `PROVIDER_FINITE_RESERVATION`** and separates:

- operation-scaled KX **full-lifetime** custody; and
- provider-resident process/thread custody.

Both debit the same accepted executor-root ceiling. Provider residency is not free overhead and no second budget/ledger is created. This architecture task itself mutates no runtime source; sole material worker remains Draft PR #356.

## Exact provider graph

- rustls `0.23.43`;
- aws-lc-rs `1.18.0`, checksum `ce2b2dcc879c3bae0d371e77c99f2238400ef24ec001394befa67b6e543add9e`;
- aws-lc-sys `0.44.0`, checksum `f09fae7be8bb3174e05c6afdb34199e6dc0c7c04ba9fa237b1967adfbde27483`;
- aws-lc-rs VCS `f464440d1fd3983ce9fb023e9eaf1698530919a2`;
- AWS-LC `991e67ff4cf04df4dd89e407f8b920c6936cb56a`;
- target `x86_64-unknown-linux-gnu`, default non-FIPS aws-lc-sys;
- reachable groups `X25519MLKEM768`, X25519, P-256, P-384.

The durable decision is `docs/architecture/reviews/OTERYN_GAME_WP3_AWS_LC_PROVIDER_RESIDENT_ACCOUNTING_AMENDMENT_2026-09-08.md`.

## Resolved accounting boundary

### Charged unit

Under the existing non-RSS charged resident-work envelope:

- AWS-LC `OPENSSL_malloc(n)` charges exact `n + 8` at this pin;
- direct libc allocations charge source-requested capacity;
- Rust boxes charge exact target layout; collections charge actual capacity;
- mmap charges page-rounded requested mapping capacity;
- allocator-private RSS/arena metadata is not the charged unit.

### Same-root provider custody

The existing caller-supplied `ResourceBudget` remains the sole budget capability. Provider-shared residency debits the same 12 MiB executor root but is not attached to one 4 MiB active slot. It reduces later root availability and never mints capacity.

Read-only preflight of canonical #356 proved one additional exact future material seam is required to make this architecture implementable without violating the already-protected “no parallel owner trait” rule:

- current rustls owner-aware construction carries `Arc<dyn DeframerBufferOwner>`;
- current `DeframerBufferOwner` exposes only ordinary `try_reserve/release`;
- therefore provider-shared root reservation must be an **additive default-fail method on that existing trait**, not a new KX owner interface.

The first material slice retains all process/thread shared debits until process teardown. It does not need and is not authorized to add a provider-shared release method or directly call `AWSLC_thread_local_clear()`.

### Process provider bound

```text
JITTER_FIRST_USE_PEAK = 140208
PROCESS_PROVIDER_RESIDENT(page_size) = 140208 + 2 * page_size
```

The health-test collector and 8 KiB GCD history overlap and are freed before the smaller persistent collector. Two page terms cover Linux fork UBE plus optional SysGenID UBE mapping.

### Per-thread provider bound

```text
THREAD_PROVIDER_RESIDENT = 1360 bytes
```

The 64-byte AWS-LC pointer table is direct libc `malloc` in pinned `thread_pthread.c`, so it is `L(64)=64`, not `P(64)=72`. The remaining terms cover frontend RAND state, entropy source, thread tree-Jitter DRBG and fixed ERR_STATE. Registration bookkeeping is separately precharged if it allocates. Every successful thread debit is conservatively retained until process teardown in this first slice.

### Full-lifetime operation KX bounds

| group | bytes reserved before `start()` and retained through `complete()`/error/drop |
|---|---:|
| X25519 | **554** |
| P-256 | **1625** |
| P-384 | **1705** |
| ML-KEM-768 child | **6264** |
| X25519MLKEM768 | **7881** |

The classical numbers include peer parsing, derive contexts, ECDH validation/BIGNUM temporaries and shared-secret overlap. ML-KEM/hybrid remain dominated by their source-derived start peaks. No post-start top-up is allowed.

HRR reserves `held_initial_full_bound + replacement_full_bound` before retry `start()` and releases old custody only after old backing destruction.

## Review findings / repairs

1. First candidate `ef7979f40...` had start-only classical bounds. Self-review found that `complete()` could exceed them. Current architecture uses full-lifetime `554/1625/1705/6264/7881` bounds.
2. Old Codex P1 claimed the pthread pointer table used `OPENSSL_calloc`; exact `crypto/thread_pthread.c` disproved it because OPENSSL allocation is prohibited there and direct libc `malloc/free` is used.
3. Read-only material preflight on #356 found a real **scope P1**: current future path set could not express provider-shared root reservation through the same `Arc<dyn DeframerBufferOwner>`. The architecture is repaired before MQ by leasing exactly `msgs/deframer/buffers.rs::DeframerBufferOwner` for one additive default-fail shared-root method. No new owner trait is permitted.
4. Direct thread-exit release was removed from the first material slice. `AWSLC_thread_local_clear()` remains evidence only, avoiding destructor-order and direct aws-lc-sys/Cargo scope.

All review/CI evidence from any superseded exact head is stale for integration.

## Acceptance criteria

- [x] Fresh protected main and canonical #356 read back before allocation.
- [x] Exact architecture branch absence proved before creation.
- [x] Architecture-only allocation recorded in #439 comment `5588086089`.
- [x] No #356 source mutation performed by this task.
- [x] Exact non-FIPS provider graph and target pinned.
- [x] Native charged unit reconciled with `DUR-FRESH-RESOURCE-ENVELOPE-V1`.
- [x] Jitter first-use and process provider-resident peak source-closed.
- [x] Per-thread provider-resident bound and direct-libc TLS table source-closed.
- [x] Shared residency debits same executor root without holding/minting active-slot capacity.
- [x] Non-FIPS self-test/service-indicator, error-state and ML-KEM workspace unknowns closed.
- [x] Exact full-lifetime operation KX bounds and HRR overlap defined.
- [x] Ordinary owner-free behavior and custom-provider fail-closed rule preserved.
- [x] Material preflight closed same-owner shared-root scope gap with one exact existing-trait surface.
- [x] First material slice avoids provider-shared release/aws-lc-sys/Cargo expansion.
- [ ] Independent exact-head review of final repaired docs candidate.
- [ ] Required canonical exact-head CI.
- [ ] Normal Merge Queue integration.
- [ ] Protected-main readback.
- [ ] Fresh material application to existing #356.

## Future material path set after protected integration only

Existing #351-owned SQLx surfaces:

- `vendor/sqlx-core-0.9.0/src/net/resource_budget.rs`
- `vendor/sqlx-core-0.9.0/src/net/tls/tls_rustls.rs`
- `vendor/sqlx-core-0.9.0/src/net/tls/resource_budget_tests.rs`

Existing rustls owner-interface surface, newly authorized only for same-owner provider-shared root reservation:

- `vendor/rustls-0.23.43/src/msgs/deframer/buffers.rs :: DeframerBufferOwner`

Allowed change on this symbol is limited to one additive object-safe provider-shared reservation method with **default unsupported/error** semantics, minimum bounded error/support plumbing on the same surface, and focused tests. Existing ordinary `try_reserve/release` behavior must remain unchanged. No new owner trait, deframer policy redesign or numeric limit is authorized.

New KX crypto-cell surfaces:

- `vendor/rustls-0.23.43/src/client/hs.rs`
- `vendor/rustls-0.23.43/src/client/tls13.rs`
- `vendor/rustls-0.23.43/src/crypto/mod.rs`
- `vendor/rustls-0.23.43/src/crypto/ring/kx.rs`
- `vendor/rustls-0.23.43/src/crypto/aws_lc_rs/pq/hybrid.rs`
- `vendor/rustls-0.23.43/src/crypto/aws_lc_rs/pq/mlkem.rs`

`vendor/rustls-0.23.43/src/lib.rs` remains byte-identical because it already re-exports `DeframerBufferOwner`. `crypto/aws_lc_rs/mod.rs`, Cargo/lock, aws-lc-rs/aws-lc-sys and every other path remain read-only. Another required path must stop with exact `SHARED_LEASE_REQUIRED = path :: symbol :: reason` before mutation.

## Required future RED/GREEN

The same #356 worker must prove at minimum:

1. ordinary `DeframerBufferOwner` without shared-root support fails before any AWS-LC call;
2. supporting SQLx owner delegates shared reservation to the **same** caller-supplied `ResourceBudget` identity;
3. ordinary deframer `try_reserve/release` semantics and owner-free clients remain unchanged;
4. process shared bound max/max+1, same-root debit and active-slot reuse while process debit persists;
5. concurrent first use cannot mint/double-debit process residency;
6. new-thread shared debit/bookkeeping denies before provider call and repeat KX on one thread does not duplicate charge;
7. root exhaustion under thread churn denies before provider use and all successful thread debits remain process-lifetime charged;
8. X25519 `553/554`, P-256 `1624/1625`, P-384 `1704/1705`, ML-KEM `6263/6264`, hybrid `7880/7881` boundaries deny/succeed before start;
9. classical completion peer parse/derive/ECDH validation/BIGNUM/secret overlap fits held reservation with no post-start growth;
10. HRR old-full + replacement-full overlap and no early old release;
11. hybrid child failure/unwind without leak or early release;
12. completion/error/cancellation/drop releases operation custody only after backing destruction while provider-shared debits persist;
13. owner-aware unsupported custom provider fails before ordinary start;
14. exact-target internal size/capacity assertions;
15. actual AWS-LC positive TLS handshake under budgeted SQLx path;
16. configured PostgreSQL 17.6 qualification and all remaining #356 acceptance cells.

## Explicit exclusions

This docs task grants no present mutation to #356, rustls/sqlx/Tokio/Game source, Cargo/lock, aws-lc-rs/aws-lc-sys, workflows, AGENTS, registry, rulesets/protection, production/external systems, or WP4/WP5/G0/Server Seam readiness. It grants no FIPS/cross-target proof and no direct `AWSLC_thread_local_clear()` implementation in the first material slice.

## Validation and integration

The architecture PR must remain exactly the two allocated documentation paths. Independent review is mandatory because the disposition changes D→B and because the final authority includes a high-risk resource-owner interface amendment. Canonical exact-head checks, zero unresolved review threads, normal FULL Merge Queue and protected-main readback are mandatory. No direct main update, bypass, protection weakening or self-approval is allowed.

`MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`.

## Context checkpoint

```yaml
last_progress: full-lifetime bounds source-closed; material preflight found and repaired missing same-owner shared-root trait lease
status: READY_FOR_REVIEW
branch: arch/wp3-aws-lc-provider-resident-boundary-439
pr: 451
canonical_material_pr: 356
canonical_material_modified: false
blocker: independent review + canonical CI + protected integration of final repaired architecture amendment before material KX lease
next_action: exact diff readback; request fresh exact-head independent review; require fresh CI; normal Merge Queue; protected readback; then apply exact bounded KX lease to existing #356
```
