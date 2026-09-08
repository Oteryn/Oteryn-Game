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

1. operation-scaled AWS-LC/rustls key-exchange allocations have finite exact-target start bounds; and
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

Evidence lineage is #439 comments `5586413904`, `5586624216`, `5586628094`, `5587745874`, plus the owner allocation `5588086089` and the exact source paths cited below.

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

The required semantics are:

- ordinary operation reservations debit both their active-slot accounting and the same executor-root aggregate ceiling;
- a provider-shared reservation debits the **same executor-root aggregate ceiling** but is not owned by one active slot;
- while a provider-shared reservation remains held, it reduces root capacity available to later queued/active work;
- `DFR-QUEUED-WORK`, `DFR-ACTIVE-WORK` and the 12 MiB total remain maxima. They are not promises that all individual maxima can be simultaneously filled after shared provider residency has consumed root capacity;
- no root debit may be minted by creating a connection/client/thread;
- unsupported `ResourceBudget` implementations return a bounded error before any AWS-LC call rather than falling back to an operation-only or unowned provider path;
- a root-backed shared reservation token must not keep an active slot occupied merely because the token outlives the operation that first caused provider initialization.

This rule is the provider-specific registered shared owner required by the already-protected blocking-runtime ownership architecture: genuinely process/runtime-shared backing must be held by a legitimate shared owner whose capacity is already charged, not silently attached to a completed operation.

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

Exact pinned `jitterentropy-base.c::jent_time_entropy_init` allocates the 8192-byte GCD history and a temporary health-test collector concurrently, then frees the GCD history and that collector before returning. The persistent Jitter collector is created afterward. Persistent global DRBG + persistent collector is smaller (`344 + 131664 = 132008`) than the charged first-use peak, so retaining the complete `140208` charge is conservative for process lifetime.

The smaller `jent_gcd_selftest()` allocation occurs before the large first-use peak and does not exceed it. The exact build does not enable the Jitter internal timer-thread feature. Jitter health-failure recovery frees the old collector before allocating its replacement, so it does not require two full collectors concurrently.

Two page-sized mapping terms are reserved because, on Linux:

- fork UBE lazily requests one page-sized mapping; and
- VM SysGenID, when present, requests a separate mapping whose 4-byte source request occupies one page-rounded mapping capacity.

`page_size` must be obtained and checked before provider use. Overflow, invalid page size or unavailable shared root capacity fails closed before KX/provider initialization.

The process reservation is retained to process/provider teardown. There is no AWS-LC production API that releases the global tree-Jitter object during normal static-library lifetime; a logical connection/KX completion therefore cannot release this charge.

## 6. Per-thread provider reservation

Before owner-aware KX invokes AWS-LC on a thread that is not yet registered by this provider owner, reserve from the same root-shared capability:

```text
64       // AWS-LC pthread TLS pointer table: 8 pointers on x86-64
400      // P(sizeof(rand_thread_local_state)=392)
24       // P(sizeof(entropy_source_t)=16)
344      // P(sizeof(tree_jitter_drbg_t)=336)
528      // fixed ERR_STATE direct libc allocation
----
1360 bytes
```

The tree-Jitter and error terms are conservative even when a successful call selects another entropy method or never initializes the error queue. The shared TLS pointer table is charged once per registered thread, not once per AWS-LC TLS key.

The minimum safe implementation may conservatively retain each 1360-byte registration charge until process teardown. That can only reduce future admissions; root exhaustion fails before registering another KX thread, so sequential thread churn cannot create unbounded uncharged provider work.

An implementation may instead release a thread charge at actual thread teardown only if it proves a free-before-release boundary. The exact `aws-lc-sys 0.44.0` x86-64 GNU binding exposes `AWSLC_thread_local_clear()` (prefixed link symbol `aws_lc_0_44_0_AWSLC_thread_local_clear`), and AWS-LC documents that function as destructing AWS-LC-related TLS data for the current thread. Such an optimization is optional; if destructor ordering or call reachability is not proven, retain the conservative process-lifetime charge.

Provider-owner registration bookkeeping must itself be precharged from the shared root before allocation. An unbounded uncharged map/list/registry is forbidden.

## 7. Previously open provider cells

### RNG/DRBG/Jitter — CLOSED

Pinned `rand.c` lazily creates one frontend `rand_thread_local_state`; `entropy_sources.c` creates one `entropy_source_t`. `tree_drbg_jitter_entropy.c` explicitly states that per-thread seed DRBG lifetime follows frontend TLS, while the global seed DRBG and Jitter collector live for the duration AWS-LC is loaded. Sections 5–6 charge those lifetimes rather than pretending they end with one KX.

### UBE — CLOSED

Pinned Linux fork UBE requests one page mapping. VM SysGenID may request a second page-rounded mapping. Both are process-shared and are pre-funded by section 5.

### Non-FIPS initialization/self-test/service indicator — CLOSED

The exact production graph uses non-FIPS `aws-lc-sys`; rustls `fips` is not a default feature. For this exact build:

- `CRYPTO_library_init()` performs CPU capability setup, not FIPS KAT allocation;
- `boringssl_ensure_ml_kem_self_test()` is an inline no-op;
- service-indicator functions are inline no-ops.

FIPS mode is outside this decision.

### Error queue — CLOSED for reachable KX

Pinned `ERR_STATE` is a fixed per-thread 16-entry ring and is included in the 1360-byte shared-thread reservation. Variable `ERR_add_error_data*` backing is not reached by the fixed supported KX start paths; identified dynamic EVP diagnostic formatting is on unsupported algorithm-ID handling. A future reachable variable diagnostic invalidates the exact bound until a finite pre-reserved term is added.

### ML-KEM implementation workspace — CLOSED

Pinned `mlkem_native_config.h` does not enable `MLK_CONFIG_CUSTOM_ALLOC_FREE`. The exact `mlkem_native.h` states that default algorithm workspace allocation is on the stack and publishes fixed ML-KEM-768 parameter-set workspace constants (`10176` keypair without PCT, `13248` encapsulation, `14336` decapsulation). Therefore the exact build has no hidden target-selected dynamic heap workspace at the KX boundary. A future custom allocator or provider feature invalidates this classification.

## 8. Operation-scaled KX start reservations

After required process/thread shared residency is funded, the admitted operation reserves its selected group's complete start bound **before** `SupportedKxGroup::start()` or any equivalent provider/randomness allocation.

Private rustls concrete sizes must be asserted inside the vendored implementation on this exact target. A mismatch fails qualification; a wrapper `size_of` is never used as a proxy for native child backing.

### X25519

```text
native start = P(EVP_PKEY_CTX=72) + P(EVP_PKEY=24) + P(X25519_KEY=65)
             = 185
rustls KeyExchange Box = 200
KX_START_X25519 = 385
```

The 32-byte public representation is inline. The implementation may conservatively hold all 385 bytes until the active KX object is destroyed.

### P-256 / P-384

Both reachable named groups use static built-in `EC_GROUP` objects and the same fixed heap shapes:

```text
native start =
    P(EVP_PKEY_CTX=72)
  + P(EC_PKEY_CTX=16)
  + P(EVP_PKEY=24)
  + P(EC_KEY=64)
  + P(EC_WRAPPED_SCALAR=96)
  + P(EC_POINT=224)
  = 544

KX_START_P256 = 200 + 544 = 744
KX_START_P384 = 200 + 544 = 744
```

Private scalar limbs and public point coordinates are inline in their fixed EC objects; public serialization is inline in aws-lc-rs.

### ML-KEM-768 child

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

KX_START_MLKEM768 = 6264
```

The generated DecapsulationKey path retains the 64-byte seed.

### Hybrid `X25519MLKEM768`

Construction retains X25519 while ML-KEM starts, then allocates the exact 1216-byte combined share and 96-byte `ActiveHybrid` box:

```text
X25519 retained complete = 200 + 105 = 305
ML-KEM complete start    = 6264
combined Vec capacity    = 1216
ActiveHybrid Box         = 96
KX_START_X25519MLKEM768  = 7881
```

Reservation table:

| Reachable group | Pre-start operation reservation |
|---|---:|
| X25519 | 385 |
| P-256 | 744 |
| P-384 | 744 |
| ML-KEM-768 child | 6264 |
| X25519MLKEM768 | 7881 |

These are exact-target implementation bounds, not new public resource-policy maxima.

## 9. HRR and release boundary

For initial KX, hold the complete group reservation while the active exchange exists.

For HelloRetryRequest:

1. retain the complete initial group reservation;
2. reserve the complete replacement-group start bound from the same operation ledger;
3. only after reservation succeeds may replacement `start()` execute;
4. on success, destroy the old active exchange before releasing its old reservation;
5. on replacement failure, destroy/unwind any new backing before releasing the retry reservation; the old reservation remains until its enclosing active state is actually destroyed.

Conservative admission is:

```text
HRR_PEAK = held_initial_group_bound + retry_group_start_bound
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
- reserve the selected KX start bound from the operation ledger before initial/HRR `start()`;
- retain complete old+retry overlap;
- release operation KX custody only after active backing destruction;
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
| Initial X25519/P-256/P-384 | Group bound short by 1 rejects before start/randomness. | Exact selected bound succeeds with unchanged share and handshake semantics. |
| Initial hybrid | 7880 bytes of operation capacity rejects before either child starts. | 7881-byte operation reservation covers classical + ML-KEM + combine peak. |
| HRR | Capacity only for initial share rejects before retry start while initial stays owned. | Initial + complete retry reservation coexist; old release follows old destruction. |
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