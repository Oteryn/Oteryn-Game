# Oteryn Game WP3 AWS-LC KX root-graph layout correction — 2026-09-09

Refs #162, #351, #356, #364, #439, #451.

## Status

```yaml
decision_id: OTV2-WP3-AWS-LC-KX-ROOT-GRAPH-LAYOUT-CORRECTION-20260909
repository: Oteryn/Oteryn-Game
publication_base_main_sha: 85ba329ccbdf51338a672fcac8ca0836726dc71b
status: PROSPECTIVE_CORRECTION_NOT_ACTIVE
supersedes: only the classical-rustls private-layout terms and dependent KX reservation numbers in protected #451
preserves: all other #451 provider/native lifetime and fail-closed semantics
exact_target: x86_64-unknown-linux-gnu / non-FIPS / Rust 1.94.0 / rustls 0.23.43 / aws-lc-rs 1.18.0 / aws-lc-sys 0.44.0 / AWS-LC 991e67ff4cf04df4dd89e407f8b920c6936cb56a
```

This document is an additive architecture correction. It does not rewrite historical #451 evidence, grant present material mutation, or make Draft PR #356 ready. It becomes material authority only after independent exact-head review, canonical checks, normal FULL Merge Queue integration, protected-main readback and an explicit fresh application to the existing #351/#356 worker.

## Mandatory decision test

1. **Must decide now?** YES.
2. **Concrete downstream work blocked:** canonical #356 cannot compile its exact-target layout assertion or truthfully use the protected classical KX reservation numbers. Provider-config ownership, the remaining #451 matrix, complete TLS, PostgreSQL 17.6 qualification, WP3 release, WP4 and Server Seam remain downstream.
3. **Why deferral is unsafe:** deleting the assertion or changing only test constants would hide an under-reservation in the exact production dependency graph.
4. **Evidence that could supersede this correction:** a different explicitly qualified dependency graph/toolchain, a changed rustls KX representation, or a new independently reviewed complete native/Rust bound.
5. **Deliberately not decided:** no public resource maximum changes, no TLS/group/PQ/provider policy changes, no FIPS/other-target support and no new material path lease.

## Canonical evidence that invalidates the old private layout

Protected #451 intended to qualify the exact root dependency graph:

- Rust `1.94.0`;
- rustls `0.23.43`;
- aws-lc-rs `1.18.0`;
- aws-lc-sys `0.44.0`;
- Linux x86-64 GNU, non-FIPS.

Root `Cargo.lock` at protected main pins:

```text
aws-lc-rs 1.18.0
checksum ce2b2dcc879c3bae0d371e77c99f2238400ef24ec001394befa67b6e543add9e
aws-lc-sys 0.44.0
```

Canonical exact-head Merge gate run `34323039456`, Linux workspace job `102373930802`, checked out #356 head `356abc3a37bae03e1e85b963947ffb1a21f278af`, installed Rust 1.94.0, downloaded `aws-lc-rs v1.18.0` and `aws-lc-sys v0.44.0`, then built:

```text
cargo +1.94.0 build --locked --workspace --all-targets
```

The exact-root compile failed closed at the newly added private-layout assertion:

```text
error[E0308]: mismatched types
  --> vendor/rustls-0.23.43/src/crypto/ring/kx.rs:158:22
158 | const _: [(); 200] = [(); size_of::<KeyExchange>()];
    |               ---    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ expected an array with a size of 200, found one with a size of 208
```

Therefore, on the actual qualified root graph:

```text
size_of::<rustls crypto/ring/kx.rs::KeyExchange>() = 208
```

The prior focused rustls-manifest validation is not authoritative for this private layout because `vendor/rustls-0.23.43/Cargo.lock` pins a different dependency graph:

```text
aws-lc-rs 1.17.3
aws-lc-sys 0.43.0
```

That standalone vendor graph can remain useful for upstream-package tests, but it cannot qualify exact-target memory layouts whose representation depends on aws-lc-rs/aws-lc-sys. Future exact private-size evidence for WP3 must be bound to the root `--locked` graph or an explicitly equivalent lock/patch set.

## Scope of the correction

The failed assertion changes only the rustls classical `KeyExchange` Box payload term from **200** to **208** bytes. No source evidence in this correction changes the already-reviewed AWS-LC native object sizes or lifetimes.

The same canonical root compilation contained the sibling exact assertions for:

```text
ML-KEM Active = 40
ActiveHybrid = 96
```

and emitted no mismatch for them before failing the crate on `KeyExchange`. They remain the expected exact-target layouts, but final material qualification MUST compile all three assertions under the root `--locked` graph. Any mismatch is a new fail-closed architecture blocker, not permission to silently update another number.

## Corrected classical full-lifetime bounds

All arithmetic below is the protected #451 formula with only the classical rustls `KeyExchange` Box term corrected from 200 to 208.

### X25519

Protected #451 used:

```text
X25519_START = P(EVP_PKEY_CTX=72) + P(EVP_PKEY=24) + P(X25519_KEY=65)
             + rustls KeyExchange Box 200
             = 385

active retained = 200 + P(24) + P(65) = 305
parsed peer     = peer bytes 32 + P(EVP_PKEY=24) 32 + P(X25519_KEY=65) 73 = 137
derive          = P(EVP_PKEY_CTX=72) 80 + aws-lc-rs secret Vec 32
X25519_COMPLETE_PEAK = 554
```

Corrected exact-root arithmetic:

```text
X25519_START = 80 + 32 + 73 + 208 = 393
active retained = 208 + 32 + 73 = 313
X25519_COMPLETE_PEAK = 313 + 137 + 80 + 32 = 562
KX_FULL_X25519 = 562
```

### P-256

Protected active retained was 640, including the 200-byte `KeyExchange` Box. Corrected:

```text
active retained = 648
P256_START = 648 + 80 + 24 = 752
P256_COMPLETE_PEAK = 648 + 401 + 104 + 32 + 304 + 144 = 1633
KX_FULL_P256 = 1633
```

### P-384

Corrected:

```text
active retained = 648
P384_START = 752
P384_COMPLETE_PEAK = 648 + 433 + 104 + 48 + 304 + 176 = 1713
KX_FULL_P384 = 1713
```

### ML-KEM-768 child

No classical `KeyExchange` term is present in the protected ML-KEM child formula. Subject to the mandatory root-graph `Active=40` assertion:

```text
KX_FULL_MLKEM768 = 6264
```

is unchanged.

### X25519MLKEM768

Protected hybrid start included the retained X25519 child at 305 bytes. Correct it to 313:

```text
X25519 retained active = 313
ML-KEM start            = 6264
combined Vec capacity   = 1216
ActiveHybrid Box        = 96
HYBRID_START_RESERVATION = 7889
```

Completion phases become:

```text
classical-complete phase = 96 + 1216 + 4984 + 562 = 6858
PQ-complete phase        = 96 + 1216 + 32 + 5112 = 6456
KX_FULL_X25519MLKEM768   = max(7889, 6858, 6456) = 7889
```

## Corrected exact-target reservation table

| Reachable group | Protected #451 | Corrected exact-root full-lifetime reservation |
|---|---:|---:|
| X25519 | 554 | **562** |
| P-256 | 1625 | **1633** |
| P-384 | 1705 | **1713** |
| ML-KEM-768 child | 6264 | **6264** |
| X25519MLKEM768 | 7881 | **7889** |

Corresponding fail-before-start negative thresholds are 561, 1632, 1712, 6263 and 7888 bytes.

These remain private exact-target implementation reservations inside the existing accepted B root ceiling. They are not new public policy maxima and do not enlarge the 4 MiB/12 MiB resource envelope.

## HRR and lifetime semantics remain unchanged

This correction changes numbers only. All protected #451 lifetime rules remain authoritative:

- reserve the corrected full bound before initial `start()`;
- for HRR keep the initial corrected full bound, reserve the corrected replacement full bound, then call replacement `start()`;
- release only after covered provider backing or returned-secret successor backing is actually destroyed;
- timeout, cancellation, logical KX state transition and replacement intent are not release evidence;
- provider process/thread shared residency remains separately charged to the same executor root;
- no post-allocation top-up, allocator hook, whole-handshake magic reserve, PQ/group/TLS weakening, provider swap or static/reused ephemeral key.

The published #356 repair at `5fdea54f0454259fcfdba6e9c4bb3db4e2d9d762` correctly moved reservation control outside the covered provider Box allocations and retained charge through synchronous returned-secret consumption. That structural repair remains useful; only its classical-dependent numeric reservations/assertions require correction.

## Material authority after protected integration

After this document is independently reviewed, passes canonical checks, integrates through FULL Merge Queue and is read back from protected main, Work may apply the correction to the SAME Draft #351/#356 worker.

No new authored source path is added. Existing protected #451 KX paths may only replace the stale exact-target private constants/assertions/tests with:

```text
562 / 1633 / 1713 / 6264 / 7889
```

and the corresponding 208-byte classical layout assertion/formula evidence.

The newly protected #453 provider-config owner amendment remains separately ACTIVE authority for `vendor/rustls-0.23.43/src/crypto/aws_lc_rs/mod.rs`, but material implementation should not claim an exact-target KX GREEN until this numeric correction is protected and the root-locked KX matrix passes.

Coordinator evidence `5597817573` also remains an independent required repair: process registration must not retain an uncharged per-connection `Arc<DeframerBufferOwner>` wrapper to process lifetime. This correction does not exempt or numerically absorb that bookkeeping.

## Required final RED/GREEN after application

The existing #356 worker must prove on the actual root `Cargo.lock` graph:

1. compile-time exact layouts: `KeyExchange=208`, `ML-KEM Active=40`, `ActiveHybrid=96`;
2. max-minus-one denial before provider start at 561 / 1632 / 1712 / 6263 / 7888;
3. exact admission and full returned-secret lifetime at 562 / 1633 / 1713 / 6264 / 7889;
4. hybrid whole and classical-component completion;
5. actual HRR initial + replacement overlap with corrected bounds;
6. cancellation/error/drop ordering;
7. concurrent process first use, thread first/repeat/churn and provider-registration bookkeeping;
8. ordinary owner-free controls and unsupported/non-exact fail-before-start behavior;
9. `cargo +1.94.0 build --locked --workspace --all-targets` and affected strict Clippy/tests on the same root graph.

A standalone vendored-rustls lock using aws-lc-rs 1.17.3 / aws-lc-sys 0.43.0 cannot qualify these exact-target layouts.

## Programme effect

After protected integration and application:

- the exact-target numerical under-reservation introduced by stale standalone dependency evidence is corrected;
- #451 provider/native ownership architecture otherwise remains in force;
- #356 must still finish the provider-config owner implementation, registration-bookkeeping repair, remaining #451 matrix, complete TLS accounting, funded AWS-LC TLS-positive handshake and configured PostgreSQL 17.6 qualification;
- WP3/WP4/WP5/G0/Server Seam are not complete or released merely by this correction.

`MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`.
