# WP3 AWS-LC provider-configuration owner amendment

Coordinator: #162. Programme: #364. Existing sole material worker: #351 / Draft PR #356.

## State

```yaml
allocation_id: OTV2-WP3-AWS-LC-PROVIDER-CONFIG-OWNER-20260909
repository: Oteryn/Oteryn-Game
allocation_base_main_sha: b26395edff3dde1ebcc155ab70758520d780884c
allocation_state: NOT_ACTIVE
preparation_branch: coord/wp3-aws-lc-provider-config-owner-351
worker_branch: agent/sqlx-driver-budget-351
risk: HIGH
source_blocker_comment: 5597312030
source_wp3_head: cfc916e1c42e12a0d0b735419711ad2d82aa6337
source_wp3_tree: efc3d3b4db30bc9f795a9a29a2b50d3b76610652
```

This is a prospective allocation-only amendment. It grants no present implementation authority and creates no replacement material worker. Material application is permitted only after independent exact-head review, canonical exact-head checks, normal FULL Merge Queue integration, protected-main readback and a fresh coordinator custody/overlap check. The existing #351/#356 branch remains the sole material lineage.

This amendment also does **not** accept the current #356 claim that the #451 KX cell is already `PROVEN`. Coordinator evidence comment `5597261954` records unresolved lifecycle findings on checkpoint `526485c6c4578bd71e1201db9056435c632fd6c0` (extra owner-wrapper heap and release before returned-secret consumption). Those findings must be repaired independently inside the already-protected #451 lease before final KX acceptance.

## Mandatory decision test

1. **Must decide now?** YES.
2. **Concrete downstream work blocked:** #351/#356 cannot construct the exact owner-aware AWS-LC `CryptoProvider` without allocating two configuration `Vec`s before the caller can reserve their backing. Complete TLS composition, funded TLS-positive qualification, PostgreSQL 17.6 qualification, WP3 release, WP4 and Server Seam remain downstream-blocked.
3. **What becomes harder later:** allowing SQLx-side post-allocation charging or copied private provider constants would establish the wrong ownership boundary and make future provider changes silently under-accounted.
4. **Evidence that would supersede this amendment:** an upstream rustls owner/preallocation API, a provider representation with no dynamic configuration backing, a controlled allocator contract that changes the accepted accounting unit, or a different explicitly qualified rustls/provider/toolchain graph.
5. **Deliberately not decided:** no new public resource maximum, no provider/group/TLS policy change, no FIPS/other-target support, no allocator replacement, and no general redesign of `CryptoProvider`.

## Problem and exact source evidence

At canonical #356 checkpoint `cfc916e1c42e12a0d0b735419711ad2d82aa6337`, the owner-aware SQLx path correctly acquires the protected #451 AWS-LC process/thread residency before provider construction, but then calls `rustls::crypto::aws_lc_rs::default_provider()`.

Pinned rustls 0.23.43 source blob `1ab57dae6c51aae67ab343860f1f4ea22415caec` shows that the exact non-FIPS provider construction performs:

```rust
cipher_suites: DEFAULT_CIPHER_SUITES.to_vec(),
kx_groups: DEFAULT_KX_GROUPS.to_vec(),
```

Those two allocations happen inside `vendor/rustls-0.23.43/src/crypto/aws_lc_rs/mod.rs`, which #451 kept read-only. SQLx cannot observe their actual `Vec` capacities until after allocation, so charging them there would be prohibited catch-up.

The pinned compiler/runtime for this programme is Rust 1.94.0. Exact Rust 1.94 sources close the preallocation question:

- `library/alloc/src/slice.rs`, blob `bf5cbafbac63044d78a22ad8bdd76bce856a7dfe`: slice `to_vec()` calls `Vec::with_capacity_in(s.len(), alloc)` before copying/cloning the elements;
- `library/alloc/src/raw_vec/mod.rs`, blob `1e76710d35364f688403fa355c8f7eab788cc452`: `RawVecInner::try_allocate_in(capacity, ...)` requests `layout_array(capacity, elem_layout)` and stores `cap = capacity`; the source comment explicitly says the current allocator return length matches the requested size;
- `vendor/rustls-0.23.43/src/suites.rs`, blob `92f56d77dc0af20ece35bd500d3bd3d521f4dcc9`: `SupportedCipherSuite` is `Clone + Copy`, so this `to_vec` path has no element-level heap allocation;
- `library/alloc/src/sync.rs`, Rust 1.94 blob `4180fe91cb558bb12044ddeb09c4c96d7202bab5`: `Arc::new(T)` allocates one `Box<ArcInner<T>>`; `ArcInner` is `repr(C, align(2))` with two `AtomicUsize` counters followed by `T`, and its requested layout is source-derived with `Layout::new::<ArcInner<()>>().extend(Layout::new::<T>()).pad_to_align()` semantics.

Therefore, for this exact target/toolchain, all caller-visible Rust capacity requested by an owner-aware default AWS-LC provider can be reserved **before** allocation from private information at the allocation-owning rustls module itself.

## Decision

Authorize one **owner-aware process-shared default-provider constructor** in the pinned `aws_lc_rs` module. It must not alter the ordinary `default_provider()` API or behavior.

The owner-aware path reuses the same `Arc<dyn DeframerBufferOwner>` / caller-supplied `ResourceBudget` root already protected by #451. It constructs at most one process-shared `Arc<CryptoProvider>` for owner-aware WP3 use and retains that provider plus its shared debit for process lifetime. Subsequent owner-aware connections clone the already-existing `Arc`; they do not allocate fresh provider configuration vectors.

This is configuration ownership, not a new TLS/provider policy. The provider contents and order must remain byte/semantics-equivalent to the ordinary exact-target default:

```text
DEFAULT_CIPHER_SUITES
DEFAULT_KX_GROUPS = X25519MLKEM768 -> X25519 -> P-256 -> P-384
```

No additional group becomes qualified merely because it appears in `ALL_KX_GROUPS`.

## Exact shared reservation formula

Before the first owner-aware provider-configuration allocation, compute with checked arithmetic:

```text
CIPHER_VEC = DEFAULT_CIPHER_SUITES.len()
             * size_of::<SupportedCipherSuite>()

KX_VEC     = DEFAULT_KX_GROUPS.len()
             * size_of::<&'static dyn SupportedKxGroup>()

ARC_PROVIDER = exact Rust-1.94 ArcInner<CryptoProvider> requested layout
               (two AtomicUsize counters + CryptoProvider using the pinned
               repr/layout rule, padded to alignment)

PROVIDER_CONFIG_SHARED = CIPHER_VEC + KX_VEC + ARC_PROVIDER
```

These are private exact-target implementation reservation terms, not new registry/public maxima. The implementation should compute them from the private types/slices in the allocation-owning module, not hard-code copied SQLx constants.

The two `Vec` terms are valid preallocation bounds because pinned Rust 1.94 `to_vec()` requests capacity equal to source length and pinned `RawVec` stores that requested capacity. The `Arc` term is the exact source-derived requested layout of the one retained process-shared `Arc<CryptoProvider>`. Allocator-private metadata/RSS remains outside the accepted charged-unit contract exactly as in protected #451.

## Exact authored material lease after protected application

New rustls authored authority is limited to:

- `vendor/rustls-0.23.43/src/crypto/aws_lc_rs/mod.rs`
  - ordinary `default_provider()` and `default_kx_groups()` behavior must remain unchanged;
  - add only the owner-aware exact-target provider-construction/shared-custody path, checked reservation formula, process-shared initialization state and focused tests;
  - the owner-aware function may return an `Arc<CryptoProvider>` directly so SQLx does not perform a second uncharged `Arc::new`;
  - non-exact target/FIPS use must fail closed before provider configuration allocation.

Existing #351 authored SQLx authority may compose the new function only in:

- `vendor/sqlx-core-0.9.0/src/net/tls/tls_rustls.rs`;
- `vendor/sqlx-core-0.9.0/src/net/tls/resource_budget_tests.rs`.

The SQLx change is limited to replacing owner-aware `Arc::new(aws_lc_rs::default_provider())` construction with the reviewed owner-aware provider function and proving same-root behavior. Ordinary SQLx/rustls provider selection remains unchanged.

No new authority is granted for:

- `vendor/rustls-0.23.43/src/crypto/mod.rs` beyond the already-active #451 KX/provider-residency lease;
- `vendor/rustls-0.23.43/src/client/**` or decoded-state paths;
- Cargo/lock, aws-lc-rs/aws-lc-sys, workflows, registries, SQL migrations, B/Foundation, production or external repositories.

Any implementation need outside these exact paths must stop with `SHARED_LEASE_REQUIRED = path :: symbol :: reason` before mutation.

## Required semantics

After protected application, the same #351/#356 worker must:

1. call the already-reviewed `ensure_aws_lc_provider_residency(owner)` before any owner-aware AWS-LC provider/config/randomness work on the current thread;
2. serialize first owner-aware provider configuration construction with non-allocating process-static synchronization;
3. if a process-shared provider already exists, return `Arc::clone` only after current-thread provider residency has been established; no configuration debit or allocation repeats;
4. if no provider exists, calculate `PROVIDER_CONFIG_SHARED` with checked arithmetic and debit it through `try_reserve_provider_shared` **before** either `to_vec()` or `Arc::new`;
5. on debit denial/overflow/target mismatch, return a bounded error with no provider configuration allocation;
6. construct exactly the ordinary non-FIPS default cipher-suite and KX-group vectors, then the one process-shared `Arc<CryptoProvider>`;
7. retain the canonical provider `Arc` and its debit for process lifetime in this first slice; no provider-config shared-release API is introduced;
8. leave ordinary `default_provider()`, ordinary clients, ordinary `SupportedKxGroup::start()`, TLS versions, certificate/hostname verification, PQ preference and randomness unchanged;
9. preserve the one-root invariant from DUR-FRESH / #451; this path must not mint a second allowance or silently convert provider-shared accounting into an active-slot debit.

Prohibited: SQLx copied length/size constants, post-allocation catch-up, allocator hook, alternate provider, provider/group pruning, TLS downgrade, reuse of static ephemeral keys, dynamic provider fallback, or treating the provider-config debit as proof that the separate KX lifecycle findings in `5597261954` are closed.

## Required RED/GREEN

Before this allocation can contribute to WP3 readiness, focused evidence must prove:

- provider-config shared debit one byte short rejects before the first `Vec`/`Arc` provider allocation;
- exact funded first use creates the provider and charges exactly the source-derived `PROVIDER_CONFIG_SHARED` term;
- resulting `cipher_suites.capacity() == DEFAULT_CIPHER_SUITES.len()` and `kx_groups.capacity() == DEFAULT_KX_GROUPS.len()` on pinned Rust 1.94;
- provider contents/order match ordinary exact-target defaults;
- two racing first-use calls establish one provider, one configuration debit and no duplicate configuration allocation;
- repeat calls on the same and new registered threads reuse the same process provider via `Arc::clone` while still satisfying #451 per-thread residency before AWS-LC use;
- non-Linux-x86_64 or FIPS/non-exact qualification fails closed before allocation;
- ordinary `default_provider()` remains unchanged;
- SQLx owner-aware path uses the shared provider and never falls back to ordinary unowned provider construction after denial;
- build/source drift in the private layout/source assumptions fails qualification rather than silently reusing stale numeric constants.

Run affected rustls/SQLx focused tests, Rust 1.94 strict check/Clippy, package/provenance integrity and applicable repository governance. Final material #356 still requires the separately repaired #451 KX lifecycle cell, all protected decoded/ClientHello/session/cache accounting, funded AWS-LC TLS-positive evidence, configured PostgreSQL 17.6 qualification, independent high-risk whole-diff review, exact-head canonical CI, normal FULL Merge Queue and protected-main readback.

## Programme effect

After independent review, canonical checks, FULL Merge Queue integration and protected-main readback, Work may apply this exact amendment to the **same** #351/#356 material worker after a fresh overlap/custody check.

Protected application removes only:

```text
SHARED_LEASE_REQUIRED = vendor/rustls-0.23.43/src/crypto/aws_lc_rs/mod.rs
                        :: default_provider / default_kx_groups
                        :: provider configuration Vec/Arc preallocation + custody
```

It does not by itself mark KX, complete TLS, WP3, WP4, WP5, G0 or Server Seam complete.

`MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`.
