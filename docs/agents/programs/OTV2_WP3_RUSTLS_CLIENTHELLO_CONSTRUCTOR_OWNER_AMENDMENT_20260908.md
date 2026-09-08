# WP3 rustls ClientHello preconstruction owner amendment

Coordinator: #162. Programme: #364. Existing sole material worker: #351 / #356. Prior rustls ownership allocations: #424, #425, #427.

## State

```yaml
allocation_id: OTV2-WP3-RUSTLS-CLIENTHELLO-PRECONSTRUCTION-OWNER-20260908
repository: Oteryn/Oteryn-Game
allocation_base_main_sha: e3d8a46871a98a309c73b3febaa41a7e6d2ec408
allocation_state: NOT_ACTIVE
preparation_branch: coord/wp3-rustls-clienthello-constructor-owner-351
worker_branch: agent/sqlx-driver-budget-351
risk: HIGH
source_wp3_head: dd0f7e5cb4ad4755b0a8ddcd36cdae95baacfec2
source_wp3_preflight_parent: 8e73bad310edc18c2a11403b49762a573daafcc3
```

This is an allocation-only follow-up for the same #351/#356 worker. It grants no present implementation authority and creates no replacement worker. Material application requires independent exact-head review, canonical exact-head checks, normal FULL Merge Queue, protected-main readback and a fresh Work custody/overlap check.

## Why one combined amendment is required

The protected #427 amendment authorized `vendor/rustls-0.23.43/src/client/hs.rs::emit_client_hello_for_retry`, but the canonical worker stopped before semantic mutation after proving two earlier ownership boundaries.

1. `ClientHelloInput::new` clones `extra_exts.protocols` into `ClientHelloDetails` before `start_handshake` reaches `emit_client_hello_for_retry`. The clone is allocation-bearing and the source backing can coexist with the destination. A reservation made only inside `emit_client_hello_for_retry` is therefore too late to satisfy reservation-before-allocation and overlap custody.
2. SQLx currently receives a constructed `ClientConnection` from `ClientConnection::new`. In pinned rustls, `new` delegates to `new_with_alpn`, which calls `ConnectionCore::for_client`; `for_client` executes `ClientHelloInput::new` and `start_handshake` before construction returns. The existing post-construction `ConnectionCommon::set_deframer_buffer_owner` cannot install the accepted owner early enough for the initial ClientHello.

Exact unchanged source evidence at the worker checkpoint:

- `vendor/rustls-0.23.43/src/client/hs.rs` blob `34fc1ae1687e5978b735f0237d1de5e66b2eb609`;
- `vendor/rustls-0.23.43/src/client/client_conn.rs` blob `77b2dc5ea149ab8c72f40fc31a425ca4b6cc720e`;
- existing public owner interface is the already-protected `DeframerBufferOwner` / `DeframerBufferError` seam and `ConnectionCommon::set_deframer_buffer_owner`; no new parallel ledger or ownership trait is authorized.

A second one-off lease for only `ClientHelloInput::new` would still leave owner installation after first-ClientHello construction. A lease for only `client_conn.rs` would still leave the protocol clone outside the reservation boundary. The smallest structurally complete fix therefore covers the exact preconstruction owner path below in one amendment.

## Exact additional authored lease after protected application

### 1. Existing allocated file: `vendor/rustls-0.23.43/src/client/hs.rs`

New authority is limited to:

- `ClientHelloInput` private owner/custody field(s) strictly required to carry the same #424/#425 owner through initial ClientHello construction and later move into the already-authorized successor chain;
- `ClientHelloInput::new`, only to accept that existing owner, reserve before the configured protocol/ALPN clone, account the actual destination backing/capacity with checked arithmetic, retain source/destination overlap while both backings exist, unwind on constructor/random/session failure, and attach surviving custody to the returned `ClientHelloInput`;
- ordinary owner-free construction must remain semantically identical;
- `ClientHelloInput::start_handshake` is not granted new semantic behavior; ordinary movement of an already-owned `ClientHelloInput` into the already-authorized `emit_client_hello_for_retry` path may be wired only as mechanically required by the new private owner field.

No new authority is granted for unrelated `hs.rs` functions, retry semantics, key exchange, ECH, resumption, verifier logic or other state handlers beyond prior protected allocations.

### 2. New file-level lease: `vendor/rustls-0.23.43/src/client/client_conn.rs`

Authority is limited to these symbols and only for pre-first-ClientHello owner installation:

- `ClientConnection::{new,new_with_alpn}` only as necessary to preserve their existing behavior while introducing a separate fallible owner-aware construction entry point/helper; ordinary public constructors must retain current API/behavior and remain owner-free;
- the minimum new owner-aware `ClientConnection` constructor/helper needed by the already-owned SQLx rustls adapter;
- `ConnectionCore::for_client` only to accept/forward the optional existing owner before `ClientHelloInput::new` / `start_handshake` and to preserve it into the resulting connection state;
- initial ALPN/protocol cloning performed by the owner-aware construction path must itself be reserved before allocation and charged by actual backing/capacity, with source/destination overlap retained until the source clone boundary is genuinely released.

Do not add a global/thread-local owner, hidden static registry, process-global ledger, semantic delay of ClientHello, post-construction catch-up charge, or opaque whole-handshake/whole-ClientHello reservation. Do not modify unbuffered/server constructors or unrelated client APIs.

### 3. Existing SQLx custody remains unchanged

`vendor/sqlx-core-0.9.0/src/net/tls/tls_rustls.rs` is already owned by #351. After protected application, that worker may replace its existing `ClientConnection::new(...)` use with the new owner-aware construction path using the same accepted B `ResourceBudget` adapter/owner identity. No new SQLx path is granted by this amendment.

## Required semantics

The same accepted B operation ledger and the same #424/#425 owner identity must be used from before the first owner-derived ClientHello allocation through decoded-state, transcript, certificate, successor-state and retained-session custody.

The implementation must:

- reserve before every authorized input/config-derived allocation at this newly opened preconstruction boundary;
- charge actual capacity/backing, not requested length or logical length;
- use checked arithmetic and fail closed before allocation on overflow or insufficient balance;
- preserve old/new or source/destination charges while allocations coexist;
- move custody with backing rather than with logical control flow;
- roll back only after failed-construction backing is actually destroyed;
- preserve ordinary owner-free/no-owner behavior and all upstream TLS/wire/security semantics;
- preserve TLS 1.2/1.3, ALPN, ECH, QUIC, SNI, certificate-authority/compression, resumption, cipher-suite, verification and early-data behavior;
- introduce no dependency/version/feature change and no new numeric resource maximum.

## Focused qualification before continuing the decoded-owner matrix

At minimum prove RED then GREEN for:

- owner-aware construction denies before the first configured ALPN/protocol clone when unfunded;
- configured protocol clone charges actual destination capacity and retains source/destination overlap correctly;
- `ClientHelloInput::new` failure paths release exactly once and leave no retained charge;
- owner identity reaches `emit_client_hello_for_retry` before its first allocation and is the same identity later reused by #425 decoded-state custody;
- successful construction transfers retained ClientHello custody into the returned connection/successor state without early release or double release;
- ordinary `ClientConnection::new` and `new_with_alpn` remain behaviorally unchanged and owner-free;
- SQLx owner-aware construction has no fallback to ordinary unowned construction on denial.

Then resume all previously protected #425/#427 ClientHello and decoded-owner RED/GREEN boundaries. Real TLS-positive evidence and configured PostgreSQL 17.6 qualification remain separate mandatory acceptance cells on the actual final #356 candidate under protected #422 control.

## Explicit exclusions

No new authority is granted for:

- any rustls path other than the exact `hs.rs` and `client_conn.rs` surfaces above;
- `client/common.rs` / `ClientHelloDetails` — pass already-owned backing into its existing constructor without changing it unless a future exact proof requires a separate lease;
- `conn.rs`, `lib.rs` or deframer buffer APIs beyond reusing the already-protected owner interface;
- server handshake paths, verifier/crypto provider internals, cache policy, SQLx/PostgreSQL decoder expansion, Game/Foundation, workflows, migrations, registry, Platform or production;
- direct merge, protection/ruleset changes, WP4/WP5/G0/Server Seam release.

If implementation proves another unlisted path/symbol is materially required, stop before mutation with exact `SHARED_LEASE_REQUIRED = path :: symbol :: reason`. Do not silently widen this amendment.

## Integration lifecycle

```text
this allocation-only amendment
-> independent exact-head review
-> canonical exact-head checks
-> normal FULL Merge Queue
-> protected-main readback
-> fresh Work custody/overlap readback
-> apply to SAME #351/#356 worker
-> preconstruction owner RED/GREEN
-> previously protected ClientHello + decoded-state RED/GREEN
-> complete TLS composition + actual TLS-positive evidence
-> actual #356 configured PostgreSQL 17.6 qualification under protected #422 classifier
-> independent high-risk whole-diff review
-> canonical CI + normal FULL Merge Queue
-> protected WP3 readback/release
-> only then WP4 #335 resume
```

`MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`.
