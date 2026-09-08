# WP3 rustls ClientHello owner symbol amendment

Coordinator: #162. Programme: #364. Existing sole worker: #351 / #356. Parent decoded-owner allocation: #425.

## State

```yaml
allocation_id: OTV2-WP3-RUSTLS-CLIENTHELLO-OWNER-SYMBOL-20260908
repository: Oteryn/Oteryn-Game
allocation_base_main_sha: dfc0fd3a9148cb85b7c75e7cad3b15a1fe70d2eb
allocation_state: NOT_ACTIVE
preparation_branch: coord/wp3-rustls-clienthello-owner-351
worker_branch: agent/sqlx-driver-budget-351
source_wp3_head: 8e73bad310edc18c2a11403b49762a573daafcc3
source_wp3_tree: 07b67b89bbd5e119857f505e90c134413eba2fa2
risk: HIGH
```

This is an allocation-only symbol amendment for the existing WP3 worker. It grants no present implementation authority and creates no replacement worker. Material application is permitted only after independent exact-head review, canonical checks, normal FULL Merge Queue, protected-main readback and fresh Work custody verification. WP4 remains frozen.

## Exact blocker and necessity

After normal merge-up of protected `main@dfc0fd3a9148cb85b7c75e7cad3b15a1fe70d2eb`, the canonical #356 worker performed the protected #425 decoded-owner preflight and stopped before semantic source mutation at:

`SHARED_LEASE_REQUIRED = vendor/rustls-0.23.43/src/client/hs.rs :: emit_client_hello_for_retry`.

The #425 allocation already grants authored custody of `vendor/rustls-0.23.43/src/client/hs.rs`, but only for `ExpectServerHelloOrHelloRetryRequest::{handle,handle_hello_retry_request,into_expect_server_hello}` and successor-state construction. It does not grant `emit_client_hello_for_retry`.

Exact #356 source blob remains `vendor/rustls-0.23.43/src/client/hs.rs@34fc1ae1687e5978b735f0237d1de5e66b2eb609`. Fresh source readback proves `emit_client_hello_for_retry` allocates or duplicates ClientHello-owned backing before the already-authorized downstream decoded/state boundaries can reserve it, including input-derived extension collections, certificate-compression algorithm collection, cipher-suite collection, protocol/QUIC/ECH-related clones, certificate-authority ownership, ClientHello payload vectors and final successor-state boxing. A downstream-only hook cannot truthfully guarantee reservation-before-allocation for those objects.

## Exact new authority

After protected application, extend the existing #425 authored lease in exactly one existing file and one function boundary:

- path: `vendor/rustls-0.23.43/src/client/hs.rs`;
- source blob at blocker: `34fc1ae1687e5978b735f0237d1de5e66b2eb609`;
- newly authorized symbol: `emit_client_hello_for_retry` only, including only the local ClientHello construction/allocation operations necessary to enforce the resource-owner contract before their allocation and to transfer their custody into already-authorized successor state.

All other symbols in `client/hs.rs` remain governed by #425 or remain unallocated. This amendment does not authorize a file-wide rewrite.

## Required semantics

The sole #356 worker must reuse the SAME accepted B resource ledger and existing #424/#425 owner identity. Within `emit_client_hello_for_retry` it may introduce only the minimum owner-aware reservation/custody plumbing needed to:

- reserve before input/config-derived allocation, collection, clone or `Box` creation;
- account actual backing/capacity with checked arithmetic, not logical length or a fixed whole-ClientHello/whole-handshake estimate;
- preserve old and new backing charges through clone/reallocation overlap until the source backing actually dies;
- roll back reservations only after construction/error paths destroy the corresponding backing;
- transfer surviving charges with ClientHello payloads/extensions and successor state into the already-authorized #425 custody chain;
- preserve existing TLS 1.2/1.3, ECH, QUIC, ALPN, certificate-compression, certificate-authority, resumption, cipher-suite, verification, wire and no-owner behavior exactly;
- introduce no new semantic resource maximum and no fallback to unowned allocation.

## Focused qualification

Before this symbol can be considered proven, add focused evidence within already-owned #351/#425 test/provenance surfaces for at least:

- extension and cipher-suite collection denial before allocation;
- actual-capacity accounting for collected vectors;
- ALPN/QUIC/ECH/cookie or equivalent configured clone overlap exercised where enabled by the pinned build;
- certificate-compression and certificate-authority ownership where enabled;
- construction failure rollback with no leaked or premature owner credit;
- successful transfer into the already-authorized successor state without double release;
- ordinary no-owner control preserving upstream behavior.

Do not invent new test/product paths merely for convenience. If the exact implementation requires another unlisted rustls symbol or path, stop before mutation with a new exact `SHARED_LEASE_REQUIRED`.

## Explicit exclusions

No new authority is granted for:

- any additional rustls path;
- any other `client/hs.rs` symbol beyond the #425 lease plus `emit_client_hello_for_retry`;
- server handshake, crypto-provider, verifier/certificate-verifier, cache/store internals beyond #425;
- SQLx, PostgreSQL, Game product, workflow, registry, Foundation, SQL migration, WP4/WP5 or external repository changes;
- dependency/version/feature changes.

Existing #351/#412/#417/#424/#425 authorities, task history, counters, REDs and include-only PostgreSQL target lease remain unchanged.

## Integration lifecycle

```text
this allocation-only symbol amendment
-> independent exact-head review
-> canonical exact-head checks
-> normal FULL Merge Queue
-> protected-main readback
-> fresh Work custody verification
-> explicit application to the SAME #351/#356 worker
-> focused RED/GREEN within this symbol and existing protected surfaces
-> continue the already-authorized #425 decoded-owner boundaries
-> complete TLS ownership + real TLS-positive + configured PostgreSQL 17.6
-> independent high-risk whole-diff review
-> canonical CI + normal FULL Merge Queue
-> protected WP3 readback/release
-> only then resume WP4
```

This amendment alone proves no TLS, PostgreSQL, WP3, WP4, WP5, G0 or Server Seam readiness.

`MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`.
