# WP3 SQLx AWS-LC PQ-first qualification-profile amendment

Coordinator: #162. Programme: #364. Existing sole material worker: #351 / Draft PR #356. Parent provider-config allocation: #453.

## State

```yaml
allocation_id: OTV2-WP3-SQLX-AWS-LC-PQ-PROFILE-20260909
repository: Oteryn/Oteryn-Game
allocation_base_main_sha: 11fe2548f1d76af49f5149d9deac9a916a5cf8b3
allocation_state: NOT_ACTIVE
preparation_branch: coord/wp3-sqlx-aws-lc-pq-profile-351
worker_branch: agent/sqlx-driver-budget-351
risk: HIGH
source_wp3_head: 0b4dcdc9436b3d50282a5fedf2e83df97eba92d6
source_worker_return: 5598449748
parent_provider_config_activation: 5597855051
```

This is an allocation-only amendment. It grants no present Cargo/runtime mutation and creates no replacement material worker. Material application is allowed only after independent exact-head review, canonical checks, normal FULL Merge Queue integration, protected-main readback and a fresh coordinator overlap/custody check. The existing #351/#356 branch remains the sole material lineage.

## Why this must be decided now

Protected #451/#453 qualify the owner-aware AWS-LC TLS path with the reachable default KX order:

```text
X25519MLKEM768 -> X25519 -> P-256 -> P-384
```

The canonical #356 worker applied protected #453 at `0b4dcdc9436b3d50282a5fedf2e83df97eba92d6` and validated that its owner-aware provider matches the ordinary rustls AWS-LC default under the exact selected feature set. However the exact successful SQLx qualification command was:

```text
cargo +1.94.0 test --manifest-path vendor/sqlx-core-0.9.0/Cargo.toml \
  --config 'patch.crates-io.tokio.path=".../vendor/tokio-1.53.1"' \
  --config 'patch.crates-io.rustls.path=".../vendor/rustls-0.23.43"' \
  --locked --lib --features _rt-tokio,_tls-rustls-aws-lc-rs
```

Fresh exact-source readback establishes:

1. `vendor/sqlx-core-0.9.0/Cargo.toml` declares rustls with `default-features = false` and dependency features only `std,tls12`.
2. SQLx `_tls-rustls-aws-lc-rs` currently expands to `_tls-rustls`, `rustls/aws-lc-rs`, and `webpki-roots`. It does **not** enable `rustls/prefer-post-quantum`.
3. Pinned rustls 0.23.43 `crypto/aws_lc_rs/mod.rs` defines `DEFAULT_KX_GROUPS` conditionally: with `prefer-post-quantum`, `X25519MLKEM768` is first; without it, the same hybrid group is appended after X25519/P-256/P-384.
4. Therefore a test that only compares the owner-aware provider to `DEFAULT_KX_GROUPS` under the same feature closure is self-consistent but does not prove the protected PQ-first contract.
5. The root workspace currently selects SQLx's ring TLS profile; root workspace feature unification is not a substitute for the separately required owner-aware AWS-LC qualification profile.

The current exact AWS qualification profile therefore cannot truthfully satisfy protected #451/#453 PQ-first ordering without an explicit feature-contract change.

## Mandatory decision test

1. **Must decide now?** YES.
2. **Concrete downstream work blocked:** provider-config acceptance, full #451 KX/provider matrix, funded AWS-LC TLS-positive proof, PostgreSQL 17.6 final qualification, terminal WP3, then WP4/Server Seam.
3. **What becomes harder later:** manually reordering provider groups in source or relying on accidental Cargo feature unification would make the qualified provider depend on hidden build context and could silently regress PQ-first behavior.
4. **Evidence that would supersede this amendment:** an independently proven exact AWS qualification profile that already enables `prefer-post-quantum`, or a newer protected provider policy explicitly changing the required default KX order.
5. **Deliberately not decided:** no new KX group, no group pruning, no TLS-version change, no provider swap, no Cargo version/pin change, no FIPS support, no public resource maximum change.

## Decision

Authorize exactly one feature-edge change in the vendored SQLx-core manifest:

```text
vendor/sqlx-core-0.9.0/Cargo.toml
  feature: _tls-rustls-aws-lc-rs
  add dependency feature edge: rustls/prefer-post-quantum
```

Equivalent intended shape:

```toml
_tls-rustls-aws-lc-rs = [
    "_tls-rustls",
    "rustls/aws-lc-rs",
    "rustls/prefer-post-quantum",
    "webpki-roots",
]
```

No dependency version, source, checksum or package pin changes. No root `Cargo.toml` change. Cargo features are not a package-version lock rewrite; `Cargo.lock` must remain byte-identical unless implementation proves otherwise, in which case stop before lock mutation and request a separate exact lease rather than widening this allocation.

The feature edge does not hand-build or replace a provider. It causes the unchanged pinned rustls `aws_lc_rs::default_provider()` / `DEFAULT_KX_GROUPS` implementation to compile in the already-protected PQ-first mode. Ordinary rustls source code remains unchanged. The owner-aware #453 provider must continue to be source-equivalent to that ordinary exact-profile default.

## Exact material lease after protected application

New authored authority is limited to:

- `vendor/sqlx-core-0.9.0/Cargo.toml`
  - only the single `rustls/prefer-post-quantum` feature edge inside `_tls-rustls-aws-lc-rs`;
  - no dependency version/source/default-feature changes;
  - no other SQLx feature changes.

Existing #351 authored authority may update only its already-owned evidence/test paths as mechanically required to record/verify this protected feature edge:

- `vendor/sqlx-core-0.9.0/OTERYN_PROVENANCE.md`;
- `vendor/sqlx-core-0.9.0/src/net/tls/resource_budget_tests.rs`;
- existing task/plan documents.

No new authority is granted for root Cargo/lock, rustls source, aws-lc-rs/aws-lc-sys, SQLx PostgreSQL source, workflows, registries, B/Foundation, production, Platform, Atlas or external repositories.

## Required semantics and RED/GREEN

After protected application, the same #351/#356 worker must prove on the exact AWS-LC qualification profile:

- `cargo tree -e features` or equivalent exact metadata shows `rustls/prefer-post-quantum` enabled because `_tls-rustls-aws-lc-rs` requests it, not because of an unrelated workspace consumer;
- the ordinary pinned AWS-LC provider and the owner-aware #453 provider both expose exactly four reachable default groups in this order:
  `X25519MLKEM768`, `X25519`, `secp256r1`, `secp384r1`;
- the test asserts this explicit four-name sequence, not merely owner-aware equality to ordinary default;
- no extra `ALL_KX_GROUPS` entry becomes qualified;
- protected full-lifetime bounds remain `554/1625/1705/6264/7881` and are applied to the same five admitted group cases already governed by #451;
- ordinary AWS-LC provider construction remains ordinary rustls behavior under the selected feature; owner-aware #453 remains source-equivalent and does not manually reorder/copy the list;
- ring-only and non-AWS SQLx TLS profiles do not acquire this feature edge unless they independently select `_tls-rustls-aws-lc-rs`;
- no TLS version, certificate/hostname verification, randomness or provider semantics are weakened;
- exact Rust 1.94 + rustls 0.23.43 + aws-lc-rs 1.18.0 + aws-lc-sys 0.44.0 qualification is retained.

Run the working SQLx AWS-LC test profile and strict checks. The separate #453 allocation-free provider-validation/racing-first-use repairs and the remaining #451 HRR/thread-churn/cancellation matrix are still required; this amendment does not mark them GREEN by itself.

## Programme effect

After independent review, canonical checks, FULL Merge Queue integration and protected-main readback, Work may activate this exact feature-edge lease for the **same** #351/#356 worker. It removes only the PQ feature-contract blocker. WP3 remains open until all resource/TLS/PostgreSQL/review/CI/MQ/readback cells are truthfully complete.

`MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`.
