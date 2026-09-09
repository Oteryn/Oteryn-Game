# WP3 rustls Message custody constructor amendment

Coordinator: #162. Programme: #364. Existing sole material worker: #351 / PR #356. Protected prerequisites: #425, #429, #430, #432, #451, #453, #458.

## State

```yaml
allocation_id: OTV2-WP3-RUSTLS-MESSAGE-CUSTODY-CONSTRUCTORS-20260909
repository: Oteryn/Oteryn-Game
allocation_base_main_sha: 466d16abc0d10a018b2c4b5019bf0e0bcb59da46
allocation_state: NOT_ACTIVE
preparation_branch: coord/wp3-rustls-message-custody-351
worker_branch: agent/sqlx-driver-budget-351
source_wp3_head: 9852a07994c9f44004a044b1b1c15acab97f5f74
source_wp3_tree: 503cc1cbf70069b4fe146e3e24259eff15ecd38c
source_blocker_commit: 6ae4944353d8ee263e43f8e849e1cec1796e344b
source_blocker: SHARED_LEASE_REQUIRED
risk: HIGH
```

This is an allocation-only control-plane amendment. It creates no replacement material worker and grants no present rustls, SQLx, PostgreSQL, Game, server-runtime, deployment or production mutation authority.

Material application is permitted only after independent exact-head review, canonical exact-head repository checks, normal FULL Merge Queue, protected-main readback, a fresh custody/overlap readback, and explicit application to the SAME #351/#356 worker.

## Current blocker

The canonical #351/#356 worker proved that protected #425 cannot close backing-bound decoded `Message` custody while preserving source compatibility using only the already-authorized client decoded paths.

A concrete preflight placed a private non-allocating custody field after `Message::payload`, so payload backing is destroyed before custody release. Rust then required every existing `Message { ... }` construction site to initialize the new field. Three such files are outside #425:

- `vendor/rustls-0.23.43/src/client/ech.rs`;
- `vendor/rustls-0.23.43/src/server/tls12.rs`;
- `vendor/rustls-0.23.43/src/server/tls13.rs`.

`vendor/rustls-0.23.43/src/common_state.rs` was also named by the preflight, but it is already an authored path of protected #425. It therefore requires **no new grant** and must not be used to justify widening this amendment.

Keeping the current bare decoded `Vec<T>` result with connection-aggregate accounting is insufficient because successful transient backing can die while its debit remains retained until connection drop. An address registry, second owner model, post-allocation catch-up, or opaque whole-handshake reservation is forbidden. The smallest source-compatible representation therefore requires the existing ordinary `Message` construction sites to initialize or route through the new custody-compatible representation.

Exact source blobs at the blocker head:

```text
vendor/rustls-0.23.43/src/client/ech.rs
  5675e71d0924b3f83f322246659f1c32645fa0c8

vendor/rustls-0.23.43/src/server/tls12.rs
  d3dfa5c83dd8d2395a106dc78cdbdd3095b4ed25

vendor/rustls-0.23.43/src/server/tls13.rs
  04fdf27c949966f1176acee0e1b85a93586d8606
```

## Protected authority already available

### #425 decoded owner

Protected #425 already owns the `Message` representation/composition path in `src/msgs/message/mod.rs`, `common_state.rs`, and the rest of the client decoded-state matrix. It may introduce the minimum private custody representation and backing-bound transfer mechanics there. This amendment does not re-grant those paths.

### #429 / #430 / #432 / #451

Existing protected grants continue to own client pre-ClientHello owner installation, SQLx/PostgreSQL operation-owner propagation, retained-session retrieval/custody, and AWS-LC KX/provider accounting respectively. This amendment changes none of those contracts and supplies no new ECH, server-handshake, crypto, verifier, SQLx or PostgreSQL semantics.

## Exact additional authored lease after protected application

Only the three files below receive new authored authority. Every grant is **mechanical constructor compatibility only** for the backing-bound `Message` custody representation introduced under existing #425 authority.

### 1. `vendor/rustls-0.23.43/src/client/ech.rs`

Authority is limited to existing `Message` construction sites and the minimum adjacent function-local factoring required to construct a `Message` after #425 adds private custody state.

Permitted changes:

- initialize the new private custody field as ordinary/unowned (`None` or equivalent) where ECH constructs messages that do not originate from the owner-aware decoded-message path;
- or route those existing literals through a crate-private, non-allocating ordinary `Message` constructor introduced under #425 in `msgs/message/mod.rs`;
- preserve field values, wire encoding, transcript inputs and control flow byte-for-byte/semantically except for the required representation initialization.

Not granted:

- ECH configuration/resource accounting;
- HPKE allocation/accounting;
- ECH transcript ownership changes;
- ECH ClientHello clone/copy accounting;
- ECH retry/security semantics;
- new public API;
- a decoded owner on ordinary ECH-generated messages.

If backing-bound decoded custody later proves to require any actual ECH allocation/ownership mutation beyond constructor compatibility, stop before that mutation with a new exact `SHARED_LEASE_REQUIRED`.

### 2. `vendor/rustls-0.23.43/src/server/tls12.rs`

Authority is limited to existing `Message` construction sites and minimum adjacent function-local factoring needed to initialize/use the #425 custody-compatible `Message` representation.

The server TLS1.2 path remains ordinary owner-free behavior. Existing message fields, protocol bytes, transcript behavior, state transitions, certificate behavior, session behavior and cryptography must remain unchanged.

No server decoded-owner installation, server resource accounting, server handshake redesign, crypto/verifier change, or public API change is granted.

### 3. `vendor/rustls-0.23.43/src/server/tls13.rs`

Authority is limited identically to TLS1.3 existing `Message` construction sites and minimum adjacent function-local factoring needed to initialize/use the #425 custody-compatible representation.

The server TLS1.3 path remains ordinary owner-free behavior. Existing message fields, HRR behavior, certificate/compression behavior, early-data behavior, transcript behavior, state transitions, session/ticket behavior and cryptography must remain unchanged.

No server decoded-owner installation, server resource accounting, server handshake redesign, crypto/verifier change, or public API change is granted.

## Representation and lifetime contract

The material implementation under #425 plus this amendment must preserve these invariants:

1. A decoded owner-aware `Message` may carry one private non-allocating custody token/aggregate whose fields are ordered so charged backing is destroyed before its debit is released.
2. Ordinary unowned/ECH/server-created `Message` values carry no decoded allocation charge and require no owner lookup or allocation merely to initialize the representation.
3. Moving a `Message` transfers custody without a second debit and without early release.
4. Deep ownership/clone allocates only after a separate destination reservation while the source custody remains live.
5. Partial parse/error paths destroy partial backing before releasing its exact charge.
6. Reallocation holds old plus prospective-new charges through allocation, verifies actual capacity, destroys old backing, then releases old custody.
7. Retained successor/peer/session descendants receive transferred or separately charged custody that survives parser/message scope until actual final backing/control-block destruction.
8. No successful transient backing remains charged merely until connection drop after its allocation is destroyed.
9. The existing #424 wire deframer and #451 provider/KX charges remain separate and must not be double-charged.
10. Ordinary owner-free `std` and `no_std` behavior remains compatible with upstream rustls 0.23.43.

The amendment does not select a new general-purpose ownership architecture. It authorizes only the smallest mechanical source-compatibility surface proven necessary for the already-selected #425 backing-bound representation.

## Explicit exclusions

No new authority is granted for:

- any other `vendor/rustls-0.23.43/**` file;
- `server/hs.rs`, `server/server_conn.rs`, server session/cache internals, server certificate/verifier or server crypto paths;
- `client/ech.rs` allocations beyond mechanical `Message` construction compatibility;
- SQLx/PostgreSQL/Game source, Cargo/lock, workflows, migration, registry, Platform or external repositories;
- runtime/server deployment or live environment changes;
- changing TLS versions, ciphers, PQ ordering, certificate/hostname verification, ALPN, ECH policy, SNI, session/cache policy, early data, QUIC or protocol limits;
- arbitrary numeric caps, opaque whole-message/whole-handshake reservations, address registries, second ledgers, thread-local/global ownership, post-allocation charging or unowned fallback.

`common_state.rs` remains governed only by existing #425 authority; this amendment grants nothing new there.

## Required qualification after activation

Before P1 `3966866700` or umbrella `3954831069` may be resolved, the SAME #351/#356 worker must demonstrate on the final exact head:

- ordinary ECH, TLS1.2 server and TLS1.3 server construction still compile and preserve owner-free semantics after the `Message` representation change;
- owner-aware decoded client `Message` backing releases after backing destruction rather than connection drop;
- move transfers custody without a new charge;
- deep clone/source+destination overlap is separately charged before destination allocation;
- list/payload reallocation retains old+new peak and releases old only after destruction;
- nested/later-element parse failure destroys partial backing before rollback;
- payload U8/U16 and borrowed-to-owned/message parsed+encoded paths preserve exact custody;
- retained successor/peer/session descendants remain charged through their real lifetime;
- max-minus-one denial occurs before allocation;
- ordinary owner-free `std` and `--no-default-features` rustls builds retain normal amortized behavior.

Run focused rustls tests, exact SQLx owner-aware regressions affected by the representation, Rust 1.94 format/check/strict Clippy, governance and applicable repository gates. The three new files should receive no semantic test credit beyond constructor compatibility; complete TLS still requires the remaining #425 matrix and full composition witness.

## Acceptance boundary

Even after this amendment is protected and activated, WP3 remains incomplete until the SAME #356 lineage closes:

- backing-bound decoded custody and all remaining #425 payload/message/handshake/transcript/certificate/successor/compressed-certificate/session cells;
- `complete_tls_accounting = PROVEN` with a source-derived simultaneous-live composition witness and underfunded fail-before-first-unauthorized-allocation proof;
- actual funded owner-aware AWS-LC SQLx TLS-positive with real certificate + hostname verification on the pinned final graph;
- configured PostgreSQL 17.6 positive + hostile/denial/recovery qualification through the protected include-only durability target;
- independent high-risk whole-diff exact-head review;
- canonical exact-head CI;
- normal FULL Merge Queue;
- protected-main readback.

No WP4, WP5 material activation, G0 or Server Seam release follows merely from this allocation.

`MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`
