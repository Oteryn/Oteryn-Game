# Oteryn Game — WP3 AWS-LC provider-resident accounting amendment

- Date: 2026-09-08
- Issue: #439
- Prior protected decision: PR #441
- Owner continuation allocation: #439 comment `5588086089`
- Allocation observation: protected `main@c9cec0f746e549ff96151bcf1e3582522dfea0ee`
- Publication base: protected `main@b6411e9bd280a1b48a8c356492a332d084ac7672`
- Governing contract: `DUR-FRESH-RESOURCE-ENVELOPE-V1`
- Canonical material worker: existing Draft PR #356, branch `agent/sqlx-driver-budget-351`
- Qualified target: `x86_64-unknown-linux-gnu`, exact default non-FIPS AWS-LC graph

```yaml
classification: ARCHITECTURE_RESOLUTION
repository: Oteryn/Oteryn-Game
issue: 439
prior_decision_pr: 441
publication_base_main_sha: b6411e9bd280a1b48a8c356492a332d084ac7672
canonical_material_pr: 356
canonical_material_branch: agent/sqlx-driver-budget-351
material_head_observed: 8a97ed5e0bdfd42b934295ecf3f98584bb49a00f
decision: B_PROVIDER_FINITE_RESERVATION
implementation_ready_after_protected_integration: true
resource_numeric_values_changed: false
production_authority_changed: false
target_scope: x86_64-unknown-linux-gnu_non_fips_default_aws_lc
```

## 1. Decision

Choose **B — `PROVIDER_FINITE_RESERVATION`** for the exact target above.

Post-#441 source evidence closes both parts that were previously mixed together:

1. operation-scaled AWS-LC/rustls key-exchange allocation has a finite exact-target **full-lifetime** reservation bound from `start()` through `complete()`/error/drop; and
2. AWS-LC RNG/error/UBE state that outlives one KX operation has a finite provider-resident bound and a different custody lifetime.

The second class is **not excluded** from accounting. It is charged through a registered provider-resident shared reservation backed by the same executor-root resource ceiling. The first class remains charged to the admitted operation. There is one ceiling and no hidden provider allowance.

This supersedes only #441's `D_BLOCKED_MISSING_PROVIDER_BOUNDARY` disposition for this exact target. It does not itself mutate runtime code or make WP3 complete. Material application is permitted only after this amendment is independently reviewed, protected-integrated and freshly applied to the same #351/#356 worker.

## 2. Exact evidence pins and invalidation rule

| Component | Exact pin |
|---|---|
| rustls | `0.23.43` |
| aws-lc-rs | `1.18.0`, checksum `ce2b2dcc879c3bae0d371e77c99f2238400ef24ec001394befa67b6e543add9e` |
| aws-lc-sys | `0.44.0`, checksum `f09fae7be8bb3174e05c6afdb34199e6dc0c7c04ba9fa237b1967adfbde27483` |
| aws-lc-rs VCS | `f464440d1fd3983ce9fb023e9eaf1698530919a2` |
| AWS-LC submodule | `991e67ff4cf04df4dd89e407f8b920c6936cb56a` |
| provider mode | default non-FIPS `aws-lc-sys` |
| target | Linux x86-64 GNU |
| reachable KX groups | `X25519MLKEM768`, X25519, P-256, P-384 |

Evidence lineage is #439 comments `5586413904`, `5586624216`, `5586628094`, `5587745874`, plus owner allocation `5588086089` and exact pinned provider/rustls sources.

A provider version, target ABI, FIPS mode, group set, AWS-LC prefix model, rustls private layout or provider feature change invalidates this proof until independently requalified. This is not blanket `ALL_KX_GROUPS` authority.

## 3. Charged native accounting unit

`DUR-FRESH-RESOURCE-ENVELOPE-V1` defines the 12 MiB executor envelope as a **charged resident-work ceiling, not a process RSS guarantee**. It requires reservation before size-controlled allocation and requires dependency/runtime backing retained on behalf of B work to be charged or explicitly finitely reserved.

At AWS-LC `991e67ff...`, `crypto/mem.c` defines an 8-byte `OPENSSL_MALLOC_PREFIX`; default `OPENSSL_malloc(n)` requests exactly `n + 8` from the system allocator.

For this target the accepted charged unit is source-requested/owned capacity:

```text
AWS-LC OPENSSL heap request: P(n) = n + 8
Direct libc heap request:    L(n) = n
Rust Box allocation:         exact target size_of::<T>()
Rust Vec/String backing:     actual allocated capacity in bytes
mmap backing:                page-rounded requested mapping capacity
```

Allocator-private headers, bins/arenas/page reuse, pthread implementation bookkeeping and kernel VMA metadata that have no caller-visible/source-requested byte capacity are deployment/runtime implementation overhead, not a second hidden KX byte term. That follows the existing contract's non-RSS model; it is analogous to the contract not claiming physical PostgreSQL page/index/WAL overhead from a logical row bound. Source-visible dependency allocations and mappings are still charged in full.

If a future contract requires full physical allocator/RSS backing, this decision is invalid and a controlled allocator or supported provider preallocation interface is required.

`CRYPTO_set_mem_functions` remains rejected as production ownership: it is one-time, process-global, debug-oriented and untagged, and its callbacks have severe locking/reentrancy constraints.

## 4. One root ceiling, two custody lifetimes

The existing caller-supplied `ResourceBudget` remains the only accounting capability. The material implementation may extend that existing capability with a **provider-shared reservation operation**; it must not define a second budget, allowance or independent numeric ceiling.

Required semantics:

- ordinary operation reservations debit both their active-slot accounting and the same executor-root aggregate ceiling;
- a provider-shared reservation debits the **same executor-root aggregate ceiling** but is not owned by one active slot;
- while a provider-shared reservation remains held, it reduces root capacity available to later queued/active work;
- `DFR-QUEUED-WORK`, `DFR-ACTIVE-WORK` and the 12 MiB total remain maxima, not promises that all individual maxima can be simultaneously filled after provider residency consumes root capacity;
- no root debit may be minted by creating a connection/client/thread;
- unsupported `ResourceBudget` implementations return a bounded error before any AWS-LC call rather than falling back to an operation-only or unowned provider path;
- a root-backed shared reservation token must not keep an active slot occupied merely because the token outlives the operation that first caused provider initialization.

This is the provider-specific registered shared owner required by the already-protected blocking-runtime ownership architecture: genuinely process/runtime-shared backing is held by a legitimate shared owner whose capacity is already charged, never silently attached to a completed operation.

## 5. Process-resident provider reservation

Before the first owner-aware AWS-LC KX/provider call in the process, acquire one root-shared reservation for the following conservative first-use peak:

```text
P(sizeof(tree_jitter_drbg_t))       =     344
Jitter health-test collector:
  P(sizeof(rand_data))              =     216
  P(JENT_MEMORY_SIZE=131072)        =  131080
  P(sizeof(jent_sha3_ctx))          =     368
Jitter GCD history:
  P(1024 * sizeof(uint64_t))        =    8200
                                      -------
JITTER_FIRST_USE_PEAK               =  140208

PROCESS_PROVIDER_RESIDENT(page_size) = 140208 + 2 * page_size
```

This is a peak formula, not a false claim that two Jitter collectors coexist.

Pinned `jitterentropy-base.c::jent_time_entropy_init` allocates the 8192-byte GCD history and a temporary health-test collector concurrently, then frees the GCD history and that collector before returning. The persistent Jitter collector is created afterward. Persistent global DRBG + persistent collector is smaller (`344 + 131664 = 132008`) than the charged first-use peak, so retaining the complete `140208` charge for process lifetime is conservative.

The smaller `jent_gcd_selftest()` allocation occurs before the large first-use peak and does not exceed it. The exact build does not enable the Jitter internal timer-thread feature. Jitter health-failure recovery frees the old collector before allocating its replacement, so it does not require two full collectors concurrently.

Two page-sized mapping terms are reserved because, on Linux:

- fork UBE lazily requests one page-sized mapping; and
- VM SysGenID, when present, requests a separate mapping whose 4-byte source request occupies one page-rounded mapping capacity.

`page_size` must be obtained and checked before provider use. Overflow, invalid page size or unavailable shared root capacity fails closed before KX/provider initialization.

The process reservation is retained to process/provider teardown. There is no AWS-LC production API that releases the global tree-Jitter object during normal static-library lifetime; a logical connection/KX completion therefore cannot release this charge.

## 6. Per-thread provider reservation

Before owner-aware KX invokes AWS-LC on a thread not yet registered by this provider owner, reserve from the same root-shared capability:

```text
64       // direct-libc AWS-LC pthread TLS pointer table: 8 pointers on x86-64
400      // P(sizeof(rand_thread_local_state)=392)
24       // P(sizeof(entropy_source_t)=16)
344      // P(sizeof(tree_jitter_drbg_t)=336)
528      // fixed ERR_STATE direct libc allocation
----
1360 bytes
```

The 64-byte TLS pointer-table term is intentionally **not** `P(64)`. At the exact pin, `crypto/thread_pthread.c` defines `_BORINGSSL_PROHIBIT_OPENSSL_MALLOC` and `CRYPTO_set_thread_local` allocates `sizeof(void *) * NUM_OPENSSL_THREAD_LOCALS` with direct libc `malloc`. Therefore the charged source request is `L(64)=64` on x86-64 GNU.

The tree-Jitter and error terms are conservative even when a successful call selects another entropy method or never initializes the error queue. The shared TLS pointer table is charged once per registered thread, not once per AWS-LC TLS key.

The minimum safe implementation may conservatively retain each 1360-byte registration charge until process teardown. That can only reduce future admissions; root exhaustion fails before registering another KX thread, so sequential thread churn cannot create unbounded uncharged provider work.

An implementation may instead release a thread charge at actual thread teardown only if it proves a free-before-release boundary. The exact `aws-lc-sys 0.44.0` x86-64 GNU binding exposes `AWSLC_thread_local_clear()` (prefixed link symbol `aws_lc_0_44_0_AWSLC_thread_local_clear`), and AWS-LC documents that function as destructing AWS-LC-related TLS data for the current thread. Such an optimization is optional; if destructor ordering or call reachability is not proven, retain the conservative process-lifetime charge.

Provider-owner registration bookkeeping must itself be precharged from the shared root before allocation. An unbounded uncharged map/list/registry is forbidden.

## 7. Previously open provider cells

### RNG/DRBG/Jitter — CLOSED

Pinned `rand.c` lazily creates one frontend `rand_thread_local_state`; `entropy_sources.c` creates one `entropy_source_t`. `tree_drbg_jitter_entropy.c` states that per-thread seed DRBG lifetime follows frontend TLS, while the global seed DRBG and Jitter collector live for the duration AWS-LC is loaded. Sections 5–6 charge those lifetimes rather than pretending they end with one KX.

### UBE — CLOSED

Pinned Linux fork UBE requests one page mapping. VM SysGenID may request a second page-rounded mapping. Both are process-shared and are pre-funded by section 5.

### Non-FIPS initialization/self-test/service indicator — CLOSED

The exact production graph uses non-FIPS `aws-lc-sys`; rustls `fips` is not a default feature. For this exact build:

- `CRYPTO_library_init()` performs CPU capability setup, not FIPS KAT allocation;
- `boringssl_ensure_ml_kem_self_test()` is an inline no-op;
- service-indicator functions are inline no-ops.

FIPS mode is outside this decision.

### Error queue — CLOSED for reachable KX

Pinned `ERR_STATE` is a fixed per-thread 16-entry ring and is included in the 1360-byte shared-thread reservation. Variable `ERR_add_error_data*` backing is not reached by the fixed supported KX paths. The EC raw-point parser's unsuccessful SPKI probe may emit a fixed error before falling back to SEC1 parsing, but it allocates no variable diagnostic string. A future reachable variable diagnostic invalidates the exact bound until a finite pre-reserved term is added.

### ML-KEM implementation workspace — CLOSED

Pinned `mlkem_native_config.h` does not enable `MLK_CONFIG_CUSTOM_ALLOC_FREE`. The exact `mlkem_native.h` states that default algorithm workspace allocation is on the stack and publishes fixed ML-KEM-768 parameter-set workspace constants (`10176` keypair without PCT, `13248` encapsulation, `14336` decapsulation). Therefore the exact build has no hidden target-selected dynamic heap workspace at the KX boundary. A future custom allocator or provider feature invalidates this classification.

## 8. Operation-scaled KX full-lifetime reservations

After required process/thread shared residency is funded, the admitted operation reserves its selected group's complete **full-lifetime** bound before `SupportedKxGroup::start()` or any equivalent provider/randomness allocation. The reservation is retained through `complete()`/error/drop; it is not merely a start allowance.

Private rustls concrete sizes must be asserted inside the vendored implementation on this exact target. A mismatch fails qualification; a wrapper `size_of` is never used as a proxy for native child backing.

### X25519

Start evidence remains:

```text
native start = P(EVP_PKEY_CTX=72) + P(EVP_PKEY=24) + P(X25519_KEY=65)
             = 185
rustls KeyExchange Box = 200
X25519_START = 385
```

`complete()` is larger. The original active exchange retains `200 + P(24) + P(65) = 305`. `ParsedPublicKey` copies the 32-byte peer share and retains a peer `EVP_PKEY + X25519_KEY`, adding `32 + 32 + 73 = 137`. The derive phase then overlaps a generic `EVP_PKEY_CTX` (`80`) and aws-lc-rs 32-byte secret Vec:

```text
X25519_COMPLETE_PEAK = 305 + 137 + 80 + 32 = 554
KX_FULL_X25519 = max(385, 554) = 554
```

The later rustls `SharedSecret` copy overlaps the 32-byte aws-lc-rs secret only after the derive context is gone and is below this peak.

### P-256

Start evidence remains:

```text
retained active = rustls KeyExchange Box 200
                + P(EVP_PKEY=24) 32
                + P(EC_KEY=64) 72
                + P(EC_WRAPPED_SCALAR=96) 104
                + P(EC_POINT=224) 232
                = 640
start-only contexts = P(EVP_PKEY_CTX=72) 80 + P(EC_PKEY_CTX=16) 24
P256_START = 744
```

For TLS's uncompressed raw peer point, the failed SPKI probe rejects before allocating an `EVP_PKEY`; SEC1 parsing then retains peer bytes 65, peer `EVP_PKEY` 32, `EC_KEY` 72 and duplicated `EC_POINT` 232 = `401`. The parser's first temporary `EC_POINT` can coexist during construction, but `640 + 401 + 232 = 1273`, below the derive peak.

During derive, while active and retained parsed peer are live:

- generic derive `EVP_PKEY_CTX + EC_PKEY_CTX` = `80 + 24 = 104`;
- aws-lc-rs shared-secret Vec = `32`;
- `ECDH_compute_shared_secret` creates another temporary `EC_KEY` and copied `EC_POINT` = `72 + 232 = 304`;
- `EC_KEY_check_fips` runs even in this non-FIPS ECDH path and converts affine x/y to BIGNUMs. For P-256 each coordinate is `P(BIGNUM=24)=32 + P(4 limbs * 8)=40 = 72`, so x+y = `144`.

Thus:

```text
P256_COMPLETE_PEAK = 640 + 401 + 104 + 32 + 304 + 144 = 1625
KX_FULL_P256 = max(744, 1625) = 1625
```

### P-384

The retained active/start object shapes are the same as P-256, so `P384_START = 744`.

The peer share is 97 bytes, making retained peer parse `97 + 32 + 72 + 232 = 433`. The derive secret is 48 bytes. P-384 BIGNUM coordinate limbs are six 64-bit words: each coordinate is `32 + P(48)=32+56=88`; x+y = `176`.

```text
P384_COMPLETE_PEAK = 640 + 433 + 104 + 48 + 304 + 176 = 1705
KX_FULL_P384 = max(744, 1705) = 1705
```

### ML-KEM-768 child

The start peak remains dominant:

```text
AWS-LC heap start =
    P(EVP_PKEY_CTX=72)
  + P(KEM_PKEY_CTX=8)
  + P(EVP_PKEY=24)
  + P(KEM_KEY=32)
  + P(public=1184)
  + P(secret=2400)
  + P(seed=64)
  = 3840

Rust retained/temporary start backing =
    DecapsulationKey Box 16
  + Active Box 40
  + public Vec 1184
  + temporary public encoding 1184
  = 2424

MLKEM768_START = 6264
```

The retained active object is `40 + 16 + 1184 + 3744 = 4984`. Decapsulation adds a 32-byte aws-lc-rs secret Vec plus a generic/KEM context pair `80 + 16 = 96`; mlkem-native decapsulation workspace is stack-only for this exact config:

```text
MLKEM768_COMPLETE_PEAK = 4984 + 32 + 96 = 5112
KX_FULL_MLKEM768 = max(6264, 5112) = 6264
```

The generated `DecapsulationKey` path retains the 64-byte seed.

### Hybrid `X25519MLKEM768`

A conservative source-derived start reservation remains:

```text
X25519 retained complete = 305
ML-KEM complete start    = 6264
combined Vec capacity    = 1216
ActiveHybrid Box         = 96
HYBRID_START_RESERVATION = 7881
```

This is deliberately a **conservative reservation bound**, not a claim that every term is simultaneously allocated at one instant; it safely covers the sequential child-start and later combine phases without under-reservation.

Hybrid `complete()` does not exceed it. Rustls completes the classical child first, then the PQ child, so their complete temporaries do not overlap:

```text
classical-complete phase = ActiveHybrid 96 + combined Vec 1216
                         + retained PQ child 4984 + X25519 complete 554
                         = 6850

PQ-complete phase        = ActiveHybrid 96 + combined Vec 1216
                         + classical SharedSecret 32 + ML-KEM complete 5112
                         = 6456

KX_FULL_X25519MLKEM768 = max(7881, 6850, 6456) = 7881
```

### Full-lifetime reservation table

| Reachable group | Operation reservation acquired before `start()` and retained through `complete()`/error/drop |
|---|---:|
| X25519 | **554** |
| P-256 | **1625** |
| P-384 | **1705** |
| ML-KEM-768 child | **6264** |
| X25519MLKEM768 | **7881** |

These are exact-target implementation reservation bounds, not new public resource-policy maxima.

## 9. HRR and release boundary

For initial KX, reserve and hold the complete full-lifetime group bound from section 8.

For HelloRetryRequest:

1. retain the complete initial full-lifetime group reservation;
2. reserve the complete replacement **full-lifetime** group bound from the same operation ledger;
3. only after the replacement full bound is reserved may replacement `start()` execute;
4. on success, destroy the old active exchange before releasing its old reservation;
5. on replacement failure, destroy/unwind any new backing before releasing the replacement reservation; the old reservation remains until its enclosing active state is actually destroyed.

Conservative admission is:

```text
HRR_RESERVATION = held_initial_full_bound + replacement_full_bound
```

A timeout, state transition, future cancellation or logical replacement is not release evidence.

## 10. Exact future material lease after protected integration

Only after this amendment is independently reviewed, normal-Merge-Queue integrated and read back on protected `main`, Work may apply this exact crypto-cell lease to the **existing #351/#356 worker**.

Existing #351-owned SQLx surfaces needed to extend the same accounting capability and compose the provider owner:

- `vendor/sqlx-core-0.9.0/src/net/resource_budget.rs`
- `vendor/sqlx-core-0.9.0/src/net/tls/tls_rustls.rs`
- `vendor/sqlx-core-0.9.0/src/net/tls/resource_budget_tests.rs`

New rustls KX surfaces, limited to owner-aware KX reservation/custody and exact-target assertions:

- `vendor/rustls-0.23.43/src/client/hs.rs`
- `vendor/rustls-0.23.43/src/client/tls13.rs`
- `vendor/rustls-0.23.43/src/crypto/mod.rs`
- `vendor/rustls-0.23.43/src/crypto/ring/kx.rs`
- `vendor/rustls-0.23.43/src/crypto/aws_lc_rs/pq/hybrid.rs`
- `vendor/rustls-0.23.43/src/crypto/aws_lc_rs/pq/mlkem.rs`

`vendor/rustls-0.23.43/src/crypto/aws_lc_rs/mod.rs`, Cargo/lock, aws-lc-rs/aws-lc-sys source and all other paths remain read-only. If another path becomes materially required, stop before mutation with exact `SHARED_LEASE_REQUIRED = path :: symbol :: reason`.

The existing operation-owner propagation remains intact: the caller supplies one existing `ResourceBudget` identity. Provider-shared reservation must be an extension of that same capability and route to the same executor root; no second owner trait/ledger is authorized.

## 11. Required material semantics

The implementation must:

- preserve the existing sole B resource ceiling;
- acquire the process provider-shared reservation before first owner-aware AWS-LC provider use;
- acquire one provider-shared thread reservation before first owner-aware provider use on each unregistered thread;
- precharge provider-owner bookkeeping;
- reserve the selected **full-lifetime** KX bound from the operation ledger before initial/HRR `start()`;
- retain complete old+replacement full-bound overlap during HRR;
- retain the operation KX reservation through `complete()` and release only after every corresponding active/temporary/returned-overlap backing covered by that reservation is destroyed or custody has moved to a separately charged successor;
- retain process shared custody to process/provider teardown and thread custody at least until a proven free boundary;
- use checked arithmetic and fail closed before provider/randomness/key generation on overflow/exhaustion/unsupported shared-owner capability;
- leave ordinary owner-free `SupportedKxGroup::start()` and ordinary clients/providers unchanged;
- make the dedicated owner-aware custom-provider default `unsupported` before ordinary `start()`, never a silent unowned fallback;
- preserve default provider/group ordering, PQ preference, TLS semantics, cryptographic randomness and ephemeral-key behavior.

Prohibited: process-global debug allocator ownership, post-allocation catch-up, magic whole-handshake/whole-slot reservation, PQ/group/TLS weakening, provider swap, static/cached ephemeral keys, early shared-owner release, or widening this proof to another target/FIPS/provider pin.

## 12. RED/GREEN acceptance matrix

| Boundary | Required negative proof | Required positive proof |
|---|---|---|
| Shared-root semantics | A `ResourceBudget` with no provider-shared capability rejects before AWS-LC; process reservation short by 1 rejects before provider use. | Shared reservation debits the same 12 MiB root, not a new allowance; an active slot can be released/reused while process shared custody remains and root usage stays charged. |
| Concurrent first use | Two operations racing first provider use cannot double-initialize/mint provider capacity. | Exactly one process shared reservation becomes canonical; loser observes/reuses it without another root debit. |
| Thread first use | Thread reservation/bookkeeping short by 1 rejects before provider call on an unregistered thread. | One thread registration is charged once; repeat KX on that thread does not duplicate shared charge. |
| Thread churn | Root exhaustion rejects a new registration before AWS-LC; no uncharged registry growth. | Conservative retained registrations remain within root ceiling; optional free path releases only after `AWSLC_thread_local_clear`/actual TLS destruction. |
| X25519 full lifecycle | 553 bytes available rejects before `start()`. | 554-byte reservation covers start, peer parse, derive and returned-secret overlap through release. |
| P-256 full lifecycle | 1624 bytes available rejects before `start()`. | 1625-byte reservation covers start, SEC1 peer parse, ECDH validation/BIGNUM temporaries, derive and returned-secret overlap. |
| P-384 full lifecycle | 1704 bytes available rejects before `start()`. | 1705-byte reservation covers the analogous P-384 lifecycle. |
| ML-KEM-768 full lifecycle | 6263 bytes available rejects before `start()`. | 6264-byte reservation covers start and the smaller decapsulation complete phase. |
| Initial hybrid | 7880 bytes available rejects before either child starts. | 7881-byte conservative reservation covers child starts, combined share, both sequential complete phases and final drop. |
| HRR | Capacity only for the initial full bound rejects before retry start while initial stays owned. | Initial full bound + replacement full bound coexist; old release follows old destruction. |
| Failure/drop/cancel | Child/provider/error/cancel path cannot release before backing destruction or shared owner lifetime. | Unwind destroys partial operation backing before charge release; process shared charge remains. |
| Ordinary control | Ordinary constructors/groups acquire no WP3 budget requirement. | Existing owner-free behavior is unchanged. |
| Custom provider | Unsupported owner-aware group fails before ordinary `start()`. | A supporting custom provider requires its own reviewed equivalent bound. |
| Exact build drift | Size/capacity/pin/target mismatch fails qualification. | Internal exact-target assertions match reviewed values. |

Final WP3 qualification still requires the actual AWS-LC budgeted SQLx TLS-positive path, existing session/configuration/ClientHello cells, configured PostgreSQL 17.6 evidence, independent high-risk whole-diff review, canonical exact-head CI, normal FULL Merge Queue and protected-main readback.

## 13. Programme effect

After protected integration:

- #439 is architecture-resolved for the exact qualified target;
- `BLOCKED_MISSING_PROVIDER_BOUNDARY` is removed for that target;
- Work may apply section 10 to the same #356 worker after a fresh custody/overlap check;
- #356 remains Draft until all existing WP3 acceptance cells are actually closed;
- WP4 #329/#335 and Server Seam #247 are not released merely by this architecture merge.

For any target/build/provider outside this exact proof, disposition D remains the fail-closed fallback.
