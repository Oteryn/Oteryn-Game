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

The protected #427 amendment authorized `vendor/rustls-0.23.43/src/client/hs.rs::emit_client_hello_for_retry`, but the canonical worker stopped before semantic mutation after proving that the accepted owner must be installed earlier. A further read-only closure check found the complete **owner-installation plus ALPN/protocol-custody seam** that precedes the already-protected emit function:

```text
ClientConnection::new
-> owner-aware equivalent of new/new_with_alpn
-> ClientExtensionsInput::from_alpn
-> ConnectionCore::for_client
-> ClientHelloInput::new
-> ClientHelloInput::start_handshake
-> emit_client_hello_for_retry
```

Three coupled ALPN/protocol allocation boundaries must share one owner before the already-protected emit function can be correct:

1. Existing `ClientConnection::new` clones `config.alpn_protocols`; any owner-aware equivalent preserving that behavior must reserve before every outer/inner protocol clone it performs and retain source/destination overlap.
2. `ClientExtensionsInput::from_alpn` consumes the owned protocol byte vectors but performs a new `.collect::<Vec<_>>()`, allocating the outer `Vec<ProtocolName>` before `ClientHelloInput::new`. Mapping `ProtocolName::from` moves each inner `Vec<u8>` backing through `PayloadU8::new`; the inner backing is not deep-copied, while the outer collection is a distinct allocation whose custody must survive the conversion.
3. `ClientHelloInput::new` then clones `extra_exts.protocols` into `ClientHelloDetails` before `start_handshake` reaches `emit_client_hello_for_retry`. The source and destination protocol backings can coexist, so reservation inside the already-authorized emit symbol is too late.

Separately, SQLx receives a constructed `ClientConnection` only after the initial handshake path has begun. The existing post-construction `ConnectionCommon::set_deframer_buffer_owner` therefore cannot install the accepted owner early enough.

Exact unchanged source evidence at the worker checkpoint:

- `vendor/rustls-0.23.43/src/client/hs.rs` blob `34fc1ae1687e5978b735f0237d1de5e66b2eb609`;
- `vendor/rustls-0.23.43/src/client/client_conn.rs` blob `77b2dc5ea149ab8c72f40fc31a425ca4b6cc720e`;
- `vendor/rustls-0.23.43/src/msgs/handshake.rs` blob `0139953fbedd5c53442694de15bd83b524b03c5a`;
- `vendor/rustls-0.23.43/src/msgs/base.rs` blob `4204714d891012485d63aa5358d105e0d3ae0510` confirms `PayloadU8::new(Vec<u8>)` stores the supplied vector without copying it;
- existing public owner interface is the already-protected `DeframerBufferOwner` / `DeframerBufferError` seam and `ConnectionCommon::set_deframer_buffer_owner`; no new parallel ledger or ownership trait is authorized.

One-off leases for any single ALPN call site would leave another earlier allocation outside the reservation boundary. The smallest structurally complete fix therefore covers the exact owner-installation and ALPN/protocol-custody surfaces below in one amendment.

## Known OPEN pre-emit cells outside this amendment

This amendment deliberately does **not** claim that every allocation executed before `emit_client_hello_for_retry` is closed. The canonical #351 task already keeps `configuration/crypto/session-cache` as separate OPEN acceptance cells.

Fresh source readback confirms:

- `ClientHelloInput::new` calls `ClientSessionValue::retrieve(...)` before its protocol clone. Any session-store retrieval/clone custody not already covered by protected #425 remains part of the existing session/cache acceptance cell; this amendment adds no new session-store symbol authority.
- `ClientHelloInput::start_handshake` may call `tls13::initial_key_share(...)`, which resolves to the configured key-exchange group and `group.start() -> Box<dyn ActiveKeyExchange>`. That allocation is part of the explicitly separate configuration/crypto cell and is not authorized here.
- `ClientHelloInput::start_handshake` may create ECH state through `EchConfig::state(...)`. ECH/configuration ownership likewise remains OPEN unless already covered by a prior exact protected lease.

The new owner-aware construction path must make the same accepted owner available early enough for those later cells to be composed truthfully, but #429 does not authorize or claim their accounting. After the ALPN/protocol seam is GREEN, the worker must continue only through already-protected #425/#427 surfaces and stop at any still-unallocated session/crypto/config path with an exact `SHARED_LEASE_REQUIRED` rather than treating #429 as blanket pre-handshake authority.

## Exact additional authored lease after protected application

### 1. New exact surface: `vendor/rustls-0.23.43/src/client/client_conn.rs`

Authority is limited to these symbols and only for pre-first-ClientHello owner installation:

- `ClientConnection::{new,new_with_alpn}` only as necessary to preserve their existing behavior while introducing a separate fallible owner-aware construction entry point/helper; ordinary public constructors must retain current API/behavior and remain owner-free;
- the minimum new owner-aware `ClientConnection` constructor/helper needed by the already-owned SQLx rustls adapter;
- `ConnectionCore::for_client` only through a separate owner-aware helper/path that accepts and forwards the optional existing owner before `ClientExtensionsInput` / `ClientHelloInput` construction and preserves it into the resulting connection state; ordinary callers, including the unbuffered path, must remain owner-free and need no semantic change;
- any `config.alpn_protocols` or equivalent protocol-vector clone performed by the owner-aware path must reserve before allocation, account actual outer and nested backing/capacity with checked arithmetic, retain source/destination overlap, and unwind only after failed-construction backing is actually destroyed.

Do not add a global/thread-local owner, hidden static registry, process-global ledger, semantic delay of ClientHello, post-construction catch-up charge, or opaque whole-handshake/whole-ClientHello reservation. Do not modify server constructors, unbuffered semantics or unrelated client APIs.

### 2. Existing rustls file: `vendor/rustls-0.23.43/src/msgs/handshake.rs`

New authority is limited to the `ClientExtensionsInput` ALPN construction seam:

- `ClientExtensionsInput::from_alpn` must remain ordinary owner-free behavior for existing callers;
- a minimum adjacent private/crate-private fallible owner-aware sibling helper may be added solely for the same semantics under the accepted owner;
- the owner-aware path must reserve before the `.collect::<Vec<ProtocolName>>()` outer-vector allocation, charge actual resulting capacity/backing, and preserve custody after the input `Vec<Vec<u8>>` is consumed;
- inner `Vec<u8>` backings moved through `ProtocolName::from` / `PayloadU8::new` must transfer existing custody rather than be charged as new deep copies;
- the minimum private/crate-private RAII custody helper needed to carry this exact allocation from the ALPN conversion into `ClientHelloInput` may be added on this surface. It must release only after the charged backing is destroyed and must not create a second owner interface or ledger;
- existing `ClientExtensionsInput` transport/QUIC semantics, ordinary `Clone`, `into_owned` and unrelated handshake encoding/decoding behavior are not granted semantic redesign. If owner custody cannot be propagated without materially changing one of those unlisted operations, stop with a fresh exact shared-lease request rather than silently widening.

No other `msgs/handshake.rs` symbols are newly authorized by this amendment beyond prior protected allocations.

### 3. Existing allocated file: `vendor/rustls-0.23.43/src/client/hs.rs`

New authority is limited to:

- `ClientHelloInput` private owner/custody field(s) and the minimum private RAII helper(s) strictly required to receive the same #424/#425 owner plus already-reserved ALPN/preconstruction custody, carry it through initial ClientHello construction and later move it into the already-authorized successor chain;
- `ClientHelloInput::new`, or a separate owner-aware sibling constructor preserving ordinary `new`, only to accept that existing owner/custody, reserve before the configured protocol/ALPN clone into `ClientHelloDetails`, account actual destination backing/capacity with checked arithmetic, retain source/destination overlap while both backings exist, unwind on constructor/random/session failure, and attach surviving custody to the returned `ClientHelloInput`;
- ordinary owner-free construction must remain semantically identical;
- `ClientHelloInput::start_handshake` is not granted new session, crypto, ECH or TLS semantics; ordinary movement of already-owned custody and owner identity into the protected `emit_client_hello_for_retry` path may be wired only as mechanically required.

No new authority is granted for session retrieval, key-exchange creation, ECH-state construction, retry semantics, verifier logic or other state handlers beyond prior protected allocations.

### 4. Existing SQLx custody remains unchanged

`vendor/sqlx-core-0.9.0/src/net/tls/tls_rustls.rs` is already owned by #351. After protected application, that worker may replace its existing `ClientConnection::new(...)` use with the new owner-aware construction path using the same accepted B `ResourceBudget` adapter/owner identity. No new SQLx path is granted by this amendment.

## Required semantics

The same accepted B operation ledger and the same #424/#425 owner identity must be installed before the first newly authorized ALPN/protocol allocation and remain available for later independently authorized TLS custody cells.

For the exact surfaces above, the implementation must:

- reserve before every newly authorized input/config-derived ALPN/protocol allocation;
- charge actual capacity/backing, not requested length or logical length;
- use checked arithmetic and fail closed before allocation on overflow or insufficient balance;
- preserve old/new or source/destination charges while allocations coexist;
- distinguish moves of already-owned backing from actual new allocations so backing is neither double-charged nor released early;
- move custody with backing rather than with logical control flow;
- roll back only after failed-construction backing is actually destroyed;
- preserve ordinary owner-free/no-owner behavior and all upstream TLS/wire/security semantics;
- preserve TLS 1.2/1.3, ALPN, ECH, QUIC, SNI, certificate-authority/compression, resumption, cipher-suite, verification and early-data behavior;
- introduce no dependency/version/feature change and no new numeric resource maximum.

## Focused qualification before continuing the decoded-owner matrix

At minimum prove RED then GREEN for:

- owner-aware construction denies before the first newly authorized `config.alpn_protocols` clone/allocation when unfunded;
- `ClientExtensionsInput` owner-aware ALPN conversion reserves before the outer `Vec<ProtocolName>` collect and transfers, rather than duplicates, custody for moved inner protocol byte vectors;
- configured protocol clone into `ClientHelloDetails` charges actual destination capacity and retains source/destination overlap correctly;
- every newly authorized ALPN/preconstruction failure path releases exactly once after its charged backing is destroyed and leaves no retained charge;
- owner identity reaches `emit_client_hello_for_retry` before its first newly authorized allocation and is the same identity later reused by #425 decoded-state custody;
- successful construction transfers retained ALPN/ClientHello custody into the returned connection/successor state without early release or double release;
- ordinary `ClientConnection::new`, `new_with_alpn` and `ClientExtensionsInput::from_alpn` remain behaviorally unchanged and owner-free;
- SQLx owner-aware construction has no fallback to ordinary unowned construction on denial.

Then resume all previously protected #425/#427 ClientHello and decoded-owner RED/GREEN boundaries. Session/cache and configuration/crypto composition remain explicitly OPEN unless already covered by protected authority. Real TLS-positive evidence and configured PostgreSQL 17.6 qualification remain separate mandatory acceptance cells on the actual final #356 candidate under protected #422 control.

## Explicit exclusions

No new authority is granted for:

- any rustls path other than the exact `client/client_conn.rs`, `msgs/handshake.rs`, and `client/hs.rs` surfaces above;
- `client/common.rs` / `ClientHelloDetails` — pass already-owned backing into its existing constructor without changing it unless a future exact proof requires a separate lease;
- `client/tls13.rs::initial_key_share`, configured key-exchange provider internals, ECH state construction or other configuration/crypto cells;
- session-store/cache retrieval/clone paths not already covered by prior protected allocations;
- `conn.rs`, `lib.rs` or deframer buffer APIs beyond reusing the already-protected owner interface;
- server/unbuffered handshake semantics, verifier internals, SQLx/PostgreSQL decoder expansion, Game/Foundation, workflows, migrations, registry, Platform or production;
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
-> complete owner-installation + ALPN/protocol RED/GREEN
-> previously protected ClientHello + decoded-state RED/GREEN
-> separately close any still-open session/cache + configuration/crypto cells under exact authority
-> complete TLS composition + actual TLS-positive evidence
-> actual #356 configured PostgreSQL 17.6 qualification under protected #422 classifier
-> independent high-risk whole-diff review
-> canonical CI + normal FULL Merge Queue
-> protected WP3 readback/release
-> only then WP4 #335 resume
```

`MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`.
