# WP3 TLS1.3 ServerHello transcript caller propagation amendment

Coordinator: #162. Programme: #364. Existing sole material worker: #351 / Draft PR #356.

## State

```yaml
allocation_id: OTV2-WP3-RUSTLS-TLS13-SERVER-HELLO-TRANSCRIPT-20260910
repository: Oteryn/Oteryn-Game
initial_allocation_base_main_sha: 7144c0b9ec8691e481df058c85d890ac88d32461
reconciled_main_sha: 5025be6cf3f5140cf94708f8e6ddc9ab3f40d99f
allocation_state: NOT_ACTIVE
preparation_branch: coord/wp3-rustls-tls13-server-hello-transcript-351-20260910
worker_branch: agent/sqlx-driver-budget-351
source_wp3_head: aa8281dbec97de4ecf63949f7f7896afcb3ee15e
source_wp3_tree: 370624395c910ee3a33268257bd9aecf1810c4e4
risk: HIGH
```

This is an allocation-only control-plane amendment. It grants no present runtime mutation authority and creates no replacement worker, branch, or material PR. The preparation branch was normally reconciled with protected `main@5025be6cf3f5140cf94708f8e6ddc9ab3f40d99f` after upstream advanced during PR creation. Material application is permitted only after independent exact-head review, canonical exact-head checks, normal FULL Merge Queue, protected-main readback, fresh custody/overlap reconciliation, and explicit application to the SAME #351/#356 worker.

## Verified necessity

Protected #550 enabled owner-aware transcript start/growth at `vendor/rustls-0.23.43/src/client/hs.rs :: ExpectServerHello::handle`. The canonical #356 worker has consumed that authority and source head `aa8281dbec97de4ecf63949f7f7896afcb3ee15e` was reconciled with its protected main before this boundary was re-read.

On that exact worker source, `client::tls13::handle_server_hello` receives the resulting `HandshakeHash` and later calls ordinary `transcript.current_hash()` before deriving client handshake secrets. The accepted-ECH branch also calls ordinary `accepted.transcript.add_message(server_hello_msg)`.

The owner-aware transcript implementation in `hash_hs.rs` deliberately fails closed on those ordinary APIs: `HandshakeHash::current_hash()` requires owner-free state and directs owner-aware callers to `try_current_hash()`, while ordinary `add_message()` likewise requires owner-free state and has `try_add_message()` as the fallible owner-aware API. Those primitives and their accounting formulas already exist; this amendment does not re-authorize or redesign them.

Protected #425 allocated selected later TLS1.3 state handlers (`ExpectEncryptedExtensions::handle`, compressed/certificate/certificate-verify and ticket handling) but did not allocate `client::tls13::handle_server_hello`. Therefore the newly reachable callsite is a real exact-symbol authority boundary rather than an implementation choice that the #356 worker may infer from file proximity.

## Exact new authored runtime lease after protected application

Only this additional existing symbol may receive semantic changes under this amendment:

`vendor/rustls-0.23.43/src/client/tls13.rs :: handle_server_hello`

No whole-file authority is granted.

Within that symbol, the SAME accepted owner/ledger identity may be propagated only as necessary to use existing fallible transcript primitives:

- on the owner-aware normal TLS1.3 ServerHello path, obtain the transcript hash through existing `HandshakeHash::try_current_hash()` and propagate owner-denial before client handshake-secret derivation or successor-state construction;
- preserve the ordinary owner-free path and its existing `current_hash()` behavior;
- if the accepted-ECH transcript reaching this same symbol is owner-aware, use existing `HandshakeHash::try_add_message()` and propagate failure before further accepted-ECH state is committed, but only if this is achievable entirely within `handle_server_hello` and existing protected ECH/transcript authority;
- do not alter transcript accounting formulas, provider sizing, key-schedule semantics, cipher selection, verification, ECH protocol behavior, wire behavior, TLS versions, resumption policy, or owner-free behavior.

The implementation must remain fail-closed. No post-allocation catch-up, global/thread-local owner, hidden registry, magic whole-handshake reservation, semantic cap, plaintext/unowned fallback, dependency/version change, or architecture redesign is permitted.

## Existing authority retained, not duplicated

This amendment does not re-grant any protected #425/#429/#430/#432/#451/#453/#458/#501/#518/#535/#538/#542 symbol. In particular:

- `client/hs.rs :: ExpectServerHello::handle` remains governed by protected #550 and its explicit application;
- `client/hs.rs :: ExpectServerHelloOrHelloRetryRequest::handle_hello_retry_request` remains governed by prior protected authority;
- `hash_hs.rs` owner-aware transcript primitives remain existing protected implementation and are consumers here, not newly allocated source;
- the selected later TLS1.3 handlers listed by #425 remain under #425;
- TLS1.2 work is not widened by this amendment;
- PSK-binder authority from #493 remains `NOT_ACTIVE_CONDITIONAL` and is not activated or implied here.

## Explicitly outside this amendment

No new authority is granted for:

- any other symbol in `vendor/rustls-0.23.43/src/client/tls13.rs`;
- `vendor/rustls-0.23.43/src/client/tls12.rs` or any additional `client/hs.rs` symbol;
- `vendor/rustls-0.23.43/src/hash_hs.rs` formulas or provider sizing;
- TLS key schedule, KX/provider internals, verifier/certificate-verifier internals, session cache/store internals, ECH internals, generic send/dequeue custody, server handshake paths, or PSK binder work;
- SQLx/PostgreSQL/Game/Foundation/Durability runtime expansion, workflows, registry, production data, PKI, secrets, credentials, branch protection, or required checks;
- WP4/WP5/G0/Server Seam activation.

If the same-symbol implementation proves that accepted-ECH handling or the next reachable caller requires another unlisted symbol/path, the material worker must stop before that mutation and return exactly:

`SHARED_LEASE_REQUIRED = <exact path> :: <exact symbol/resource> :: <reason>`

## Required focused qualification

Before this exact unit may be considered proven on #356, execute evidence on the final candidate for at least:

1. owner-aware normal TLS1.3 ServerHello success through `try_current_hash()`;
2. owner-aware fork/finish underfunding denial, demonstrating failure before key-schedule derivation/successor work and correct unwind;
3. accepted-ECH owner-aware transcript append success and denial if that path is reachable without widening this exact symbol authority; otherwise report the exact next boundary rather than claiming it covered;
4. ordinary owner-free TLS1.3 ServerHello behavior unchanged;
5. existing owner-aware HRR/KX/transcript regression coverage remains green.

Use the existing protected Rust 1.94 / rustls 0.23.43 / AWS-LC graph and existing owned test/provenance surfaces. Compile-only, skipped tests, plaintext PostgreSQL, or an unrelated old-head result do not prove this unit.

## Integration and application lifecycle

```text
this allocation-only amendment
-> independent exact-head review
-> canonical exact-head checks
-> normal FULL Merge Queue
-> protected-main readback
-> fresh #356 head/source/custody reconciliation
-> explicit ACTIVE application to SAME #351/#356 worker only
-> exact-symbol RED/GREEN implementation
-> return next exact boundary or continue existing protected WP3 cells
```

Queue admission, CI success, or amendment merge is not WP3 completion. Complete TLS composition, funded SQLx TLS-positive evidence, PostgreSQL 17.6 positive plus hostile/denial qualification, independent whole-diff review, exact-head canonical CI, FULL Merge Queue, and protected WP3 readback remain mandatory before WP3 release.

`MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`
