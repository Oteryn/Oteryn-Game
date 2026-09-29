# WP3 rustls decoded ownership amendment

Coordinator: #162. Programme: #364. Existing sole worker: #351 / #356. Prior rustls deframer owner allocation: #424.

## State

```yaml
allocation_id: OTV2-WP3-RUSTLS-DECODED-OWNER-20260908
repository: Oteryn/Oteryn-Game
allocation_base_main_sha: 8ec8fedd23e7cd6b0acb3c0848baf4f7b629919f
allocation_state: NOT_ACTIVE
preparation_branch: coord/wp3-rustls-decoded-owner-351
worker_branch: agent/sqlx-driver-budget-351
risk: HIGH
source_scope_result_comment: 5580281914
source_wp3_head: bd5b4b2e62b37b77c8fd0d93ed54926e5c10a785
source_wp3_tree: 3c230b23f0ef900544e21c8c17bfe7d680ea1d1b
```

This is a prospective allocation-only amendment. It grants no present implementation authority and creates no replacement worker. Material application is permitted only after independent exact-head review, canonical exact-head checks, normal FULL Merge Queue, protected-main readback and a fresh Work custody/overlap check. WP4 remains frozen until protected WP3 release.

## Verified necessity

The canonical #356 worker stopped truthfully at `SHARED_LEASE_REQUIRED` after proving the existing #424 deframer owner hook. Independent read-only exact-head analysis in comment `5580281914` returned:

`SCOPE_RESULT = MULTI_PATH_REQUIRED`.

`vendor/rustls-0.23.43/src/conn.rs` alone cannot enforce reservation-before-allocation and exact lifetime custody for decoded TLS state. The call site cannot observe private element layouts, actual `Vec` capacity transitions, explicit payload/`Box`/DNS allocations, nested parse cleanup, or which allocations are destroyed versus transferred into successor handshake, transcript, peer-chain or retained session state. A call-site-only workaround would require duplicating private parsing/layout semantics or an opaque whole-handshake reservation; both are forbidden by the accepted resource-accounting invariants.

The existing #424 owner interface remains authoritative for the private wire deframer buffer only. This amendment reuses that owner/ledger identity but extends authored custody to the minimum decoded-state and retention boundaries proven below.

## Exact authored rustls lease after protected application

Only the following existing rustls 0.23.43 paths may receive authored semantic changes for this amendment. All other package files remain copy-only unless separately allocated.

1. `vendor/rustls-0.23.43/src/conn.rs`
   - exact source blob at scope proof: `bb61d37ed118cd464d885b26acfc7a9ea7fb5270`;
   - symbols: `ConnectionCore::{deframe,process_msg,process_new_packets}`, `ConnectionState::first_handshake_message`, decoded-owner installation/wiring;
   - purpose: open/close per-message decoded custody, pass the same ledger through decode, and order commit/split/transfer/rollback against parse failure, cancellation, connection error and drop.

2. `vendor/rustls-0.23.43/src/msgs/deframer/handshake.rs`
   - blob: `0d91f0eac82464933f543f190c58668dbc88b980`;
   - symbols: `HandshakeDeframer::{default,input_message,coalesce,coalesce_one}`, `HandshakeIter::drop`;
   - purpose: reserve actual `Vec<FragmentSpan>` capacity before initial allocation/growth, retain old/new overlap through reallocation, and release only on real shrink/drop rather than iterator drain.

3. `vendor/rustls-0.23.43/src/msgs/codec.rs`
   - blob: `bd62cd3a6de2fff93b5b2daa598eb53e96ebd886`;
   - symbols: `Reader::{init,sub}`, generic `Codec for Vec<T>::read`, `TlsListIter`;
   - purpose: propagate decoded ownership through nested readers; pre-reserve checked prospective `capacity * size_of::<T>()`; retain old/new backing overlap; reconcile actual capacity; unwind partially decoded collections on later errors.

4. `vendor/rustls-0.23.43/src/msgs/base.rs`
   - blob: `4204714d891012485d63aa5358d105e0d3ae0510`;
   - symbols: `Payload::{into_owned,into_vec}`, `PayloadU8::read`, `PayloadU16::read`;
   - purpose: reserve before `to_vec`/length-prefixed byte-vector allocation, bind actual returned capacity to custody, transfer with owned payloads and release only after backing destruction.

5. `vendor/rustls-0.23.43/src/msgs/message/mod.rs`
   - blob: `d5cecc94892c5c24c3dfb572e5c0b2422e257847`;
   - symbols: `MessagePayload::{new,into_owned}`, `Message::into_owned`, both `Message::try_from` implementations;
   - purpose: thread one RAII decoded custody through parsing and deep ownership conversion; split parsed/encoded charges when both coexist; preserve owner identity across moves and errors.

6. `vendor/rustls-0.23.43/src/msgs/handshake.rs`
   - blob: `0139953fbedd5c53442694de15bd83b524b03c5a`;
   - symbols include `HandshakeMessagePayload::{read_version,into_owned}`, `ClientHelloPayload::read`, `ServerHelloPayload::read`, `HandshakePayload::into_owned`, `CertificateChain::into_owned`, `CertificatePayloadTls13::into_owned`, `DuplicateExtensionChecker`;
   - purpose: account exact explicit `Box`, DNS/String, payload copy, nested/generic vectors, certificate collections, ECH/QUIC ownership and duplicate-extension backing, including parse-failure unwind.

7. `vendor/rustls-0.23.43/src/hash_hs.rs`
   - blob: `8b4edc68fb2bd3f93191dc47b8f8a165c958b62c`;
   - symbols: `HandshakeHashBuffer::{add_message,add_raw}`, `HandshakeHash::{add_message,add_raw,clone}`;
   - purpose: reserve transcript/client-auth vector growth and transcript clones before allocation and retain custody while transcript state outlives a decoded message.

8. `vendor/rustls-0.23.43/src/common_state.rs`
   - blob: `8b1d499a6ffcc3e165d9d74d8295084c50bddef0`;
   - symbols: `CommonState::process_main_protocol`, `State::{handle,into_owned}`, `CommonState::peer_certificates`;
   - purpose: transfer charged descendants into boxed successor states and retained peer certificates, releasing only after replacement/error/drop actually destroys the backing.

9. `vendor/rustls-0.23.43/src/client/common.rs`
   - blob: `1bf0d8c7758e6fef35ec6bbca1b55badf24d9fce`;
   - symbols: `ServerCertDetails::{new,into_owned}`;
   - purpose: preserve independent certificate-chain and OCSP charges through verifier/successor-state ownership conversions.

10. `vendor/rustls-0.23.43/src/client/hs.rs`
    - blob: `34fc1ae1687e5978b735f0237d1de5e66b2eb609`;
    - symbols: `ExpectServerHelloOrHelloRetryRequest::{handle,handle_hello_retry_request,into_expect_server_hello}` and successor-state construction;
    - purpose: reserve cookie/extension clones and successor `Box` allocations before allocation, retain source/destination overlap and transfer custody into returned state.

11. `vendor/rustls-0.23.43/src/client/tls12.rs`
    - blob: `3366021a16f652d855d6bc9cfd6bda3dfb145ee6`;
    - symbols: `ExpectCertificate::handle`, certificate-bearing `State::into_owned`, `ExpectServerDone::handle`, peer-certificate/session assignments;
    - purpose: carry certificate/OCSP custody through borrowed/owned boxed states, peer-chain retention and TLS 1.2 session copies.

12. `vendor/rustls-0.23.43/src/client/tls13.rs`
    - blob: `5f15f36c7b7f741a4e36e27ef03635ace58ad473`;
    - symbols: `ExpectEncryptedExtensions::handle`, `ExpectCompressedCertificate::handle`, `ExpectCertificate::handle`, `ExpectCertificateVerify::{handle,into_owned}`, `ExpectTraffic::handle_new_ticket_impl`;
    - purpose: account ECH/QUIC clones, certificate decompression buffer plus second decode, OCSP/certificate deep ownership, peer-chain retention, resumed-session chain clones and pre-cache clones, including overlap and error rollback.

13. `vendor/rustls-0.23.43/src/msgs/persist.rs`
    - blob: `9b8f19e6a1947bf24db827b856c3a2da472e2889`;
    - symbols: `ClientSessionCommon::new`, `Tls12ClientSessionValue::{new,clone}`, `Tls13ClientSessionValue::{new,clone}`;
    - purpose: carry transferred peer-chain custody into opaque retained sessions, reserve secret-vector and `Arc` control/backing allocation before construction, charge every deep clone, and release only on final backing/control-block destruction.

`vendor/rustls-0.23.43/OTERYN_PROVENANCE.md` remains within the existing #351/#424 package provenance custody and may be updated only to record this protected amendment, exact original blob identities, authored delta manifest and validation evidence. Existing #351 task/plan records remain owned by that worker and do not require a new coordinator path lease.

## Explicitly NOT required / not granted

The exact source analysis excludes these paths from the new authored lease:

- `vendor/rustls-0.23.43/src/msgs/message/inbound.rs` — borrow/slice only;
- `vendor/rustls-0.23.43/src/msgs/deframer/mod.rs` — orchestration only;
- `vendor/rustls-0.23.43/src/msgs/deframer/buffers.rs` — remains solely under #424 wire-buffer accounting;
- `vendor/rustls-0.23.43/src/msgs/macros.rs` — generated reads can inherit owner-aware `Reader`;
- `vendor/rustls-0.23.43/src/lib.rs` — existing #424 public owner interface must be reused; no new export is required;
- `vendor/rustls-0.23.43/src/error.rs` — reuse a preconstructed bounded owner-denial error;
- `vendor/rustls-0.23.43/src/client/handy.rs` and `vendor/rustls-0.23.43/src/limited_cache.rs` — stores may retain an already charged opaque session value without learning internals;
- all server handshake paths — not reached by the SQLx-owned `ClientConnection` installation;
- crypto-provider, verifier and certificate-verifier paths — separate configuration/crypto proof cells, not decoded-state allocation remedies;
- SQLx, PostgreSQL, Game, workflow, registry and SQL migration paths — no new authority is created here.

Existing root Cargo/rustls patch and SQLx/Game include-only custody are unchanged. This allocation neither widens nor resets any prior #351/#356 counter, RED, checkpoint or worker lineage.

## Required semantics

After protected application, the sole #356 worker must use the same accepted B resource ledger and existing #424 owner identity. It must:

- deny before every unauthorized input-derived allocation or growth;
- account actual capacity/backing, not logical length, requested delta or an opaque whole-handshake estimate;
- use checked arithmetic for prospective layout/capacity accounting;
- retain old and prospective-new charges through reallocation overlap until old backing is actually destroyed;
- carry charge with every move, clone and deep-ownership conversion;
- split custody when a source partly drops and partly transfers into successor state;
- release only after actual backing destruction on parse error, handshake error, cancellation, replacement, connection drop or final session-store drop;
- preserve ordinary unowned/no-owner behavior, no_std compatibility, TLS versions, wire/security semantics, certificate/hostname verification, cache behavior and protocol limits;
- introduce no new arbitrary semantic cap, magic whole-slot reservation, plaintext/unowned fallback, dependency/version change or architecture redesign.

## Qualification before any readiness claim

Material implementation must first demonstrate focused RED/GREEN for each newly owned boundary, including at minimum:

- generic list/payload reservation-before-allocation and actual-capacity accounting;
- nested parse failure rollback;
- `Message::try_from` / `into_owned` parsed+encoded overlap;
- handshake deframer span growth/high-water retention;
- transcript growth and clone custody;
- ClientHello/ServerHello extension and DNS/payload ownership;
- certificate/OCSP deep ownership and verifier-error rollback;
- TLS 1.2 and TLS 1.3 successor-state/peer-chain transfer;
- compressed-certificate double-buffer/second-decode overlap;
- retained session construction/clone/final-drop custody;
- connection cancellation/drop and ordinary no-owner controls.

Run affected rustls tests, the existing SQLx resource-budget/TLS tests, strict Rust 1.94 checks/Clippy/format/provenance/delta verification and applicable repository governance. The implementation must preserve archive/package identity and every upstream byte outside authorized authored paths.

This decoded-state amendment proves only the newly allocated client rustls ownership boundary. It does not by itself prove complete TLS configuration/crypto, real TLS-positive qualification, configured PostgreSQL 17.6 qualification, whole-WP3 review, WP3 readiness, WP4 release, WP5 composition, G0 or Server Seam readiness.

## Integration lifecycle

```text
this allocation-only amendment
-> independent exact-head review
-> canonical exact-head checks
-> normal FULL Merge Queue
-> protected-main readback
-> fresh Work custody/overlap readback
-> explicit application to the SAME #351/#356 writer
-> material RED/GREEN implementation within the exact authored lease
-> real TLS-positive + configured PostgreSQL 17.6 qualification
-> independent high-risk whole-diff review
-> canonical CI + normal FULL Merge Queue
-> protected WP3 readback/release
-> only then resume WP4
```

`MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`.
