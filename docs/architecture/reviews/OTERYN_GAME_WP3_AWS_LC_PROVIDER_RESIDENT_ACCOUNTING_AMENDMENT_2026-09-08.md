# Oteryn Game — WP3 AWS-LC provider-resident accounting amendment

- Date: 2026-09-08
- Issue: #439
- Prior protected decision: PR #441
- Owner continuation allocation: #439 comment `5588086089`
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

Choose **B — `PROVIDER_FINITE_RESERVATION`** for this exact target.

Post-#441 evidence closes both previously mixed ownership classes:

1. operation-scaled rustls/AWS-LC key-exchange backing has a finite exact-target **full-lifetime** reservation from `start()` through `complete()`/error/drop; and
2. AWS-LC RNG/error/UBE backing that outlives one KX operation has a finite provider-resident process/thread bound with a different custody lifetime.

Neither class is exempt from accounting. Operation backing remains charged to the admitted active operation. Provider-resident backing is charged to the **same executor-root ceiling** through the already-existing owner identity; it does not create a second budget or a provider allowance.

This supersedes only #441's `D_BLOCKED_MISSING_PROVIDER_BOUNDARY` for the exact target below. It does not itself mutate runtime code or make WP3 complete. Material application is allowed only after independent review, normal Merge Queue integration, protected-main readback and a fresh bounded lease to the same #351/#356 worker.

## 2. Exact evidence pins and invalidation

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

Evidence lineage is #439 comments `5586413904`, `5586624216`, `5586628094`, `5587745874`, allocation `5588086089`, and exact pinned provider/rustls source.

Any provider version, target ABI, FIPS mode, reachable group set, AWS-LC allocation-prefix model, rustls private layout or provider feature change invalidates this proof until requalified. This is not blanket `ALL_KX_GROUPS` authority.

## 3. Charged native accounting unit

`DUR-FRESH-RESOURCE-ENVELOPE-V1` defines the 12 MiB executor envelope as a **charged resident-work ceiling, not a process RSS guarantee**. Dependency/runtime backing retained on behalf of B work must be charged or explicitly finitely reserved before allocation.

At AWS-LC `991e67ff...`, default `OPENSSL_malloc(n)` requests `n + 8` bytes because `crypto/mem.c` prepends an exact 8-byte AWS-LC allocation prefix.

For this target the charged unit is source-requested/owned capacity:

```text
AWS-LC OPENSSL heap request: P(n) = n + 8
Direct libc heap request:    L(n) = n
Rust Box allocation:         exact target size_of::<T>()
Rust Vec/String backing:     actual allocated capacity in bytes
mmap backing:                page-rounded requested mapping capacity
```

Allocator-private chunk headers, arenas, page reuse, pthread implementation bookkeeping and kernel VMA metadata that have no caller-visible/source-requested capacity are deployment/runtime implementation overhead under this non-RSS contract. This is not an AWS-LC exemption: every source-visible AWS-LC/libc/Rust/mmap allocation in the reachable graph is charged.

If policy later requires physical allocator/RSS backing, this B decision is invalid and a controlled allocator or supported provider preallocation interface becomes required.

`CRYPTO_set_mem_functions` remains rejected as production ownership: it is process-global, one-time, debug-oriented, untagged by operation and constrained by AWS-LC locking/reentrancy rules.

## 4. One root ceiling, two custody lifetimes

The existing caller-supplied `ResourceBudget` remains the only accounting capability. Material implementation may add a **provider-shared reservation operation to that same capability**, but must not define another budget, ledger, allowance or parallel owner trait.

The current #356 owner-aware rustls path carries `Arc<dyn DeframerBufferOwner>` from SQLx through `ClientConnection::new_with_resource_owner`. Therefore the exact implementation seam is an additive method on the existing `DeframerBufferOwner` trait, not a new KX owner interface.

Required semantics:

- ordinary operation reservations continue to debit the active-slot subledger and the same executor root;
- provider-shared reservation debits that same executor root but is not owned by one 4 MiB active slot;
- shared provider custody reduces root capacity available to later queue/active work;
- the 8 queued / 2 active / 12 MiB values remain maxima, not promises that every maximum remains simultaneously fillable after shared residency consumes root capacity;
- creating a connection/client/thread never mints capacity;
- owner implementations that cannot supply provider-shared root accounting fail closed **before any AWS-LC call**;
- the first implementation slice retains provider-shared debits for process lifetime, so no shared-release API is required or authorized;
- no provider-shared debit may be represented by ordinary `try_reserve` if doing so would keep an active slot occupied after the initiating operation is otherwise releasable.

### Exact same-owner seam

After protected integration, the existing rustls trait in
`vendor/rustls-0.23.43/src/msgs/deframer/buffers.rs` may gain one additive object-safe capability with semantics equivalent to:

```text
DeframerBufferOwner::try_reserve_provider_shared(bytes) -> Result<(), bounded error>
```

Requirements:

- existing `try_reserve(bytes)` / `release(bytes)` semantics remain unchanged;
- the new method defaults to **unsupported/error**, never to ordinary `try_reserve` and never to unowned success;
- no provider-shared release method is needed in this first slice because process/thread charges are conservatively retained to process teardown;
- SQLx's existing `DeframerBudgetOwner` delegates the new method to the **same** caller-supplied `ResourceBudget` identity;
- `ResourceBudget` may gain the corresponding root-shared operation, defaulting fail-closed for implementations without root support;
- no second rustls/SQLx owner trait is introduced.

This is the minimal provider-specific registered shared owner contemplated by the already-protected blocking-runtime ownership architecture.

## 5. Process-resident provider reservation

Before first owner-aware AWS-LC KX/provider use in the process, acquire one provider-shared root debit:

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

This is a peak, not a claim that two collectors coexist. `jent_time_entropy_init` allocates the 8192-byte GCD history and temporary health-test collector concurrently, then frees both before the persistent collector is created. Persistent global DRBG + persistent collector is smaller than the first-use peak. Jitter health-failure recovery frees the old collector before allocating its replacement. The exact build does not enable the Jitter internal timer thread.

The two page terms cover Linux fork UBE and optional VM SysGenID UBE mappings. `page_size` is obtained and checked before provider use. Overflow, invalid page size or unavailable root capacity fails closed before provider initialization.

The complete process debit is retained to process/provider teardown.

## 6. Per-thread provider reservation

Before owner-aware KX invokes AWS-LC on an unregistered thread, acquire another provider-shared root debit:

```text
64       // direct-libc AWS-LC pthread TLS pointer table: 8 pointers on x86-64
400      // P(sizeof(rand_thread_local_state)=392)
24       // P(sizeof(entropy_source_t)=16)
344      // P(sizeof(tree_jitter_drbg_t)=336)
528      // fixed ERR_STATE direct libc allocation
----
1360 bytes
```

The 64-byte pointer table is intentionally `L(64)`, not `P(64)`: pinned `crypto/thread_pthread.c` defines `_BORINGSSL_PROHIBIT_OPENSSL_MALLOC`, allocates `sizeof(void *) * NUM_OPENSSL_THREAD_LOCALS` with direct libc `malloc`, and frees it with direct `free`.

The tree-Jitter and error terms remain conservatively charged even when a successful KX does not instantiate every one of them. Registration bookkeeping must itself be precharged if it allocates.

The first material slice **retains every successful 1360-byte thread debit until process teardown**. Root exhaustion rejects another thread before AWS-LC use, so sequential thread churn cannot create uncharged provider work. Although the pinned aws-lc-sys binding exposes `AWSLC_thread_local_clear()`, direct thread-exit release is evidence for a possible future optimization only and is **not authorized in this first material slice**. This avoids destructor-order ambiguity and any need for a new aws-lc-sys/Cargo surface.

## 7. Previously open provider cells

### RNG/DRBG/Jitter — CLOSED

Pinned `rand.c` lazily creates frontend `rand_thread_local_state`; `entropy_sources.c` creates one `entropy_source_t`; `tree_drbg_jitter_entropy.c` explicitly gives per-thread seed DRBGs TLS lifetime and the global seed DRBG/Jitter collector process lifetime. Sections 5–6 charge those actual lifetimes.

### UBE — CLOSED

Linux fork UBE requests one page mapping; VM SysGenID may request a second page-rounded mapping. Both are covered before provider use.

### Non-FIPS initialization/self-test/service indicator — CLOSED

For this exact non-FIPS graph, `CRYPTO_library_init()` performs CPU capability setup rather than FIPS KAT allocation; `boringssl_ensure_ml_kem_self_test()` and service-indicator functions are non-FIPS no-ops. FIPS mode remains outside this proof.

### Error queue — CLOSED for reachable KX

Pinned `ERR_STATE` is a fixed per-thread 16-entry ring included in the 1360-byte debit. Variable `ERR_add_error_data*` backing is not reachable from the fixed supported KX start/complete paths. Any future reachable variable diagnostic invalidates the bound until a finite pre-reserved term is added.

### ML-KEM implementation workspace — CLOSED

Pinned `mlkem_native_config.h` does not enable `MLK_CONFIG_CUSTOM_ALLOC_FREE`. Default mlkem-native algorithm workspace is stack-backed and publishes fixed ML-KEM-768 workspace constants; there is no hidden target-selected dynamic heap workspace in this exact build.

## 8. Operation-scaled full-lifetime KX reservations

After required process/thread shared residency is funded, the admitted operation reserves the selected group's entire **full-lifetime** bound before `SupportedKxGroup::start()` or equivalent provider/randomness allocation. There is no later top-up. The reservation remains through `complete()`/error/drop.

Private rustls sizes/capacities are asserted inside the exact vendored implementation; build/layout mismatch fails qualification.

### X25519

```text
X25519_START = P(EVP_PKEY_CTX=72) + P(EVP_PKEY=24) + P(X25519_KEY=65)
             + rustls KeyExchange Box 200
             = 385

active retained = 200 + P(24) + P(65) = 305
parsed peer     = peer bytes 32 + P(EVP_PKEY=24) 32 + P(X25519_KEY=65) 73 = 137
derive          = P(EVP_PKEY_CTX=72) 80 + aws-lc-rs secret Vec 32

X25519_COMPLETE_PEAK = 305 + 137 + 80 + 32 = 554
KX_FULL_X25519 = 554
```

The later rustls `SharedSecret` copy occurs after the derive context is gone and does not exceed this peak.

### P-256

```text
active retained = KeyExchange Box 200
                + P(EVP_PKEY=24) 32
                + P(EC_KEY=64) 72
                + P(EC_WRAPPED_SCALAR=96) 104
                + P(EC_POINT=224) 232
                = 640
P256_START = 640 + P(EVP_PKEY_CTX=72) 80 + P(EC_PKEY_CTX=16) 24 = 744
```

TLS accepts the uncompressed SEC1 peer representation. Retained peer parse is `65 + 32 + 72 + 232 = 401`; compressed-point BN_CTX allocation is not reached by this TLS path. During derive, while active + peer remain live:

- generic + EC derive contexts = `104`;
- aws-lc-rs secret Vec = `32`;
- temporary ECDH validation EC key + point = `304`;
- x/y coordinate BIGNUM backing = `144`.

```text
P256_COMPLETE_PEAK = 640 + 401 + 104 + 32 + 304 + 144 = 1625
KX_FULL_P256 = 1625
```

### P-384

The active/start shapes are the same as P-256, so `P384_START = 744`. Retained peer parse is `97 + 32 + 72 + 232 = 433`; derive secret is 48 bytes; x/y coordinate BIGNUM backing is `176`.

```text
P384_COMPLETE_PEAK = 640 + 433 + 104 + 48 + 304 + 176 = 1705
KX_FULL_P384 = 1705
```

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

Rust retained/temporary start =
    DecapsulationKey Box 16
  + Active Box 40
  + public Vec 1184
  + temporary public encoding 1184
  = 2424

MLKEM768_START = 6264
```

Retained active backing is 4984; complete adds a 32-byte aws-lc-rs secret Vec and generic/KEM contexts `96`:

```text
MLKEM768_COMPLETE_PEAK = 5112
KX_FULL_MLKEM768 = max(6264, 5112) = 6264
```

### Hybrid `X25519MLKEM768`

```text
X25519 retained complete = 305
ML-KEM complete start    = 6264
combined Vec capacity    = 1216
ActiveHybrid Box         = 96
HYBRID_START_RESERVATION = 7881
```

Hybrid completion is sequential:

```text
classical-complete phase = 96 + 1216 + retained PQ child 4984 + X25519 complete 554 = 6850
PQ-complete phase        = 96 + 1216 + classical SharedSecret 32 + ML-KEM complete 5112 = 6456
KX_FULL_X25519MLKEM768   = max(7881, 6850, 6456) = 7881
```

### Full-lifetime table

| Reachable group | Reservation acquired before `start()` and retained through `complete()`/error/drop |
|---|---:|
| X25519 | **554** |
| P-256 | **1625** |
| P-384 | **1705** |
| ML-KEM-768 child | **6264** |
| X25519MLKEM768 | **7881** |

These are exact-target implementation reservation bounds, not new public policy maxima.

## 9. HRR and release boundary

For initial KX, hold the complete full-lifetime group reservation. For HelloRetryRequest:

1. keep the initial full-lifetime reservation;
2. reserve the complete replacement full-lifetime bound from the same operation ledger;
3. only then execute retry `start()`;
4. on success, destroy old active backing before releasing its reservation;
5. on failure, destroy/unwind new backing before releasing the replacement reservation while the old one remains held.

```text
HRR_RESERVATION = held_initial_full_bound + replacement_full_bound
```

Timeout, cancellation, logical state transition or replacement intent is not release evidence.

## 10. Exact future material lease after protected integration

Only after independent review, normal Merge Queue integration and protected-main readback may Work apply this lease to the **existing #351/#356 worker**.

Existing #351-owned SQLx surfaces needed to extend the same capability and compose the owner:

- `vendor/sqlx-core-0.9.0/src/net/resource_budget.rs`
- `vendor/sqlx-core-0.9.0/src/net/tls/tls_rustls.rs`
- `vendor/sqlx-core-0.9.0/src/net/tls/resource_budget_tests.rs`

Existing rustls owner-interface surface, newly authorized **only** for the additive same-owner shared-root capability described in section 4:

- `vendor/rustls-0.23.43/src/msgs/deframer/buffers.rs :: DeframerBufferOwner`

This exact symbol authority permits only:

- adding the object-safe provider-shared reservation method with default fail-closed behavior;
- the minimum bounded error/support plumbing required by that method on the same surface;
- tests on that surface proving ordinary `try_reserve/release` behavior is unchanged and unsupported shared reservation fails closed.

It does **not** authorize deframer buffer policy redesign, new numeric limits, a second owner trait, a hidden static registry, or changing ordinary deframer allocation semantics.

New rustls KX surfaces, limited to owner-aware KX reservation/custody and exact-target assertions:

- `vendor/rustls-0.23.43/src/client/hs.rs`
- `vendor/rustls-0.23.43/src/client/tls13.rs`
- `vendor/rustls-0.23.43/src/crypto/mod.rs`
- `vendor/rustls-0.23.43/src/crypto/ring/kx.rs`
- `vendor/rustls-0.23.43/src/crypto/aws_lc_rs/pq/hybrid.rs`
- `vendor/rustls-0.23.43/src/crypto/aws_lc_rs/pq/mlkem.rs`

`vendor/rustls-0.23.43/src/lib.rs` needs no mutation: it already re-exports the existing `DeframerBufferOwner` trait. `vendor/rustls-0.23.43/src/crypto/aws_lc_rs/mod.rs`, Cargo/lock, aws-lc-rs/aws-lc-sys source and all other paths remain read-only. Any further required path must stop with exact `SHARED_LEASE_REQUIRED = path :: symbol :: reason` before mutation.

The caller still supplies one existing `ResourceBudget` identity. The SQLx adapter and rustls shared method must route to that same executor root. No second owner trait/ledger is authorized.

## 11. Required material semantics

The implementation must:

- preserve the sole B resource ceiling;
- keep ordinary `DeframerBufferOwner::try_reserve/release` semantics unchanged;
- make provider-shared reservation on `DeframerBufferOwner` default unsupported/fail-closed;
- delegate the supporting SQLx owner to the same caller-supplied `ResourceBudget` root-shared capability;
- acquire process provider-shared residency before first owner-aware AWS-LC use;
- acquire and permanently retain one provider-shared 1360-byte debit before first owner-aware AWS-LC use on each newly registered thread;
- precharge any provider registration bookkeeping that allocates;
- reserve the selected full-lifetime KX bound before initial/HRR `start()`;
- retain old + replacement full-bound overlap during HRR;
- retain operation KX custody through complete/error/drop and release only after covered backing is destroyed or moved to separately charged successor custody;
- use checked arithmetic and fail closed before provider/randomness/key generation on overflow, exhaustion or unsupported shared-owner capability;
- leave ordinary owner-free `SupportedKxGroup::start()` and ordinary clients/providers unchanged;
- make dedicated owner-aware custom-provider default unsupported before ordinary `start()`, never an unowned fallback;
- preserve default provider/group ordering, PQ preference, TLS semantics, cryptographic randomness and ephemeral keys.

Prohibited: `CRYPTO_set_mem_functions` production ownership, post-allocation catch-up, magic whole-handshake/whole-slot reserve, PQ/group/TLS weakening, provider swap, static/cached ephemeral keys, direct aws-lc-sys thread-clear use in this first slice, or widening this proof to another target/FIPS/provider pin.

## 12. RED/GREEN acceptance matrix

| Boundary | Required negative proof | Required positive proof |
|---|---|---|
| Existing owner seam | Default `DeframerBufferOwner` shared-reserve capability rejects before AWS-LC; ordinary owner-only implementation cannot silently fall back. | Supporting SQLx adapter delegates shared debit to the same `ResourceBudget` identity; ordinary deframer `try_reserve/release` behavior remains unchanged. |
| Shared-root semantics | Root-shared process reservation short by 1 rejects before provider use. | Shared debit reduces the same 12 MiB root without pinning the initiating active slot or creating another allowance. |
| Concurrent first use | Two operations racing first use cannot mint/double-debit process residency. | Exactly one canonical process shared debit is established; loser reuses observed initialized state without another debit. |
| Thread first use | Thread reservation/bookkeeping short by 1 rejects before provider call. | First use on one thread debits once; repeat KX on that thread does not duplicate charge. |
| Thread churn | Root exhaustion rejects another registration before AWS-LC; no uncharged registry growth. | Conservative process-lifetime retained registrations remain bounded by root exhaustion. |
| X25519 | 553 bytes rejects before start. | 554 covers start, peer parse, derive and returned-secret overlap. |
| P-256 | 1624 rejects before start. | 1625 covers full lifecycle. |
| P-384 | 1704 rejects before start. | 1705 covers full lifecycle. |
| ML-KEM-768 | 6263 rejects before start. | 6264 covers start and smaller complete phase. |
| Hybrid | 7880 rejects before either child starts. | 7881 covers child construction, combined share and both sequential complete phases. |
| HRR | Capacity only for initial full bound rejects before retry start. | Initial + replacement full bounds coexist; old release follows old destruction. |
| Failure/drop/cancel | Error/cancel cannot release before backing destruction or shared process lifetime. | Partial operation backing unwinds before operation charge release; provider-shared charges remain. |
| Ordinary control | Ordinary clients/groups acquire no WP3 resource requirement. | Existing owner-free behavior is unchanged. |
| Custom provider | Unsupported owner-aware custom group fails before ordinary start. | Supporting provider requires its own reviewed equivalent finite bound. |
| Build drift | Pin/target/private-size mismatch fails qualification. | Internal assertions match exact reviewed target values. |

Final WP3 qualification still requires the actual AWS-LC budgeted SQLx TLS-positive path, all existing session/configuration/ClientHello cells, configured PostgreSQL 17.6 evidence, independent high-risk whole-diff review, exact-head canonical CI, normal FULL Merge Queue and protected-main readback.

## 13. Programme effect

After protected integration:

- #439 is architecture-resolved for the exact qualified target;
- `BLOCKED_MISSING_PROVIDER_BOUNDARY` is removed for that target;
- Work may apply section 10 to the same #356 worker after a fresh custody/overlap check;
- #356 remains Draft until all existing WP3 acceptance cells are closed;
- WP4 #329/#335 and Server Seam #247 are not released merely by this architecture merge.

For any target/build/provider outside this exact proof, disposition D remains the fail-closed fallback.
