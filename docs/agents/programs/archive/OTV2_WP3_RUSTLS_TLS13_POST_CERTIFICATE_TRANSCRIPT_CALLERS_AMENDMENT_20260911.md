# WP3 TLS1.3 post-certificate transcript caller propagation amendment

Coordinator: #162. Programme: #364. Existing sole material worker: #351 / Draft PR #356.

## State

```yaml
allocation_id: OTV2-WP3-RUSTLS-TLS13-POST-CERTIFICATE-TRANSCRIPT-CALLERS-20260911
repository: Oteryn/Oteryn-Game
allocation_base_main_sha: 1f2781c9c52e4231c9638553bcdd6014291e109a
allocation_state: NOT_ACTIVE
preparation_branch: coord/wp3-rustls-tls13-finished-callers-351-20260911
worker_branch: agent/sqlx-driver-budget-351
source_wp3_head: c7437fd325f9b4c8d0f72aa1ba65e78ae1ac9b2f
risk: HIGH
```

This is an allocation-only control-plane amendment. It grants no present runtime mutation authority and creates no replacement worker, material branch or material PR. Material application is permitted only after independent exact-head review, canonical exact-head checks, normal FULL Merge Queue, protected-main readback, fresh overlap/custody reconciliation and explicit application to the SAME #351/#356 worker.

## Verified necessity

The canonical #356 lineage has already materialized the protected #425 TLS1.3 decoded-owner cells and protected #501 transcript/hash primitives, plus the protected #553 `handle_server_hello` caller propagation. Fresh caller-sensitive readback at `c7437fd325f9b4c8d0f72aa1ba65e78ae1ac9b2f` shows the next supported TLS1.3 client paths still consume an owner-aware `HandshakeHash` through ordinary infallible APIs:

- `ExpectCertificateRequest::handle` appends the server CertificateRequest through ordinary `self.transcript.add_message(&m)` before client-auth successor construction;
- `ExpectFinished::handle` obtains transcript hashes through ordinary `current_hash()` and appends Finished through ordinary `add_message()` before traffic-key transition/successor construction.

These caller symbols are not in the exact TLS1.3 symbol list of protected #425. Protected #501 supplies the already-materialized fallible transcript/hash primitives; it does not by itself grant new caller mutation authority. Fresh open-PR search found no separate material PR claiming these exact caller symbols.

The branch is currently one protected-main commit behind only because `main` advanced after the last material publication. The SAME #356 worker must normally merge current protected main before consuming any activated authority; no rebase/reset/force-push is authorized.

## Exact new authored runtime lease after protected application

Only these two existing symbols may receive semantic changes under this amendment:

1. `vendor/rustls-0.23.43/src/client/tls13.rs :: ExpectCertificateRequest::handle`
2. `vendor/rustls-0.23.43/src/client/tls13.rs :: ExpectFinished::handle`

No whole-file authority is granted.

Within those symbols only, the SAME accepted resource owner/ledger may be propagated as necessary to use already-protected fallible transcript/hash operations. Required behavior:

- owner-aware paths must use fallible transcript append/hash operations and propagate denial before successor-state/key-schedule/traffic work that depends on the denied operation;
- ordinary owner-free/no_std behavior must remain unchanged;
- source/destination lifetime and existing decoded/session/certificate custody must remain intact;
- no transcript accounting formula, provider sizing, key-schedule semantics, cipher selection, verifier behavior, certificate semantics, cache/session policy, wire behavior or TLS version behavior may change.

If implementing either caller proves that another helper/symbol is necessarily modified (including outbound client-auth/early-data helpers), stop before that mutation and return exactly:

`SHARED_LEASE_REQUIRED = <exact path> :: <exact symbol/resource> :: <reason>`

## Existing authority retained, not duplicated

This amendment does not re-grant protected #425/#501/#550/#553 or any other existing lease. In particular:

- #425 retains its exact decoded-owner TLS1.3 handlers and retained-state custody;
- #501 retains its already-materialized transcript/hash primitive authority;
- #553 retains `client/tls13.rs::handle_server_hello`;
- `ExpectEncryptedExtensions::handle`, `ExpectCompressedCertificate::handle`, `ExpectCertificate::handle`, `ExpectCertificateVerify::{handle,into_owned}` and `ExpectTraffic::handle_new_ticket_impl` remain governed by #425;
- PSK-binder allocation #493 remains `NOT_ACTIVE_CONDITIONAL` and is neither activated nor implied here.

## Explicitly outside this amendment

No new authority is granted for:

- any other symbol in `vendor/rustls-0.23.43/src/client/tls13.rs`, including `emit_end_of_early_data_tls13`, `emit_certverify_tls13`, PSK binder/resumption helpers or ECH internals;
- TLS1.2 callers or server handshake paths;
- `hash_hs.rs` accounting formulas/provider sizing beyond already-protected #501 primitives;
- key schedule/KX/provider internals, verifier/certificate-verifier internals, session store/cache internals or generic send/dequeue custody;
- SQLx/PostgreSQL/Game/Foundation runtime expansion, workflows, registry, production data, PKI, secrets, credentials, branch protection or required checks;
- WP4/WP5/G0/Server Seam activation.

## Required focused qualification

Before this exact unit may be considered proven on #356, execute evidence on the final candidate for at least:

1. funded owner-aware TLS1.3 full-handshake path through `ExpectFinished::handle` using fallible transcript hash/append operations;
2. owner-aware hash-fork/finish denial in `ExpectFinished::handle`, failing before dependent key-schedule/traffic transition;
3. owner-aware Finished transcript-growth denial, with correct unwind and no owner-free fallback;
4. funded owner-aware TLS1.3 CertificateRequest path through `ExpectCertificateRequest::handle`;
5. CertificateRequest transcript-growth denial before client-auth successor work;
6. ordinary owner-free/no_std controls unchanged;
7. regression of already-proven #425/#501/#553 TLS1.3 cells.

Compile-only, skipped tests, plaintext PostgreSQL or unrelated old-head CI do not prove this unit.

## Integration and application lifecycle

```text
this allocation-only amendment
-> independent exact-head review
-> canonical exact-head checks
-> normal FULL Merge Queue
-> protected-main readback
-> fresh #356 head/source/overlap reconciliation
-> explicit ACTIVE application to SAME #351/#356 worker only
-> normal non-force merge-up of protected main into canonical worker branch
-> exact-symbol RED/GREEN implementation
-> caller-sensitive rescan for the next exact boundary
```

Queue admission, CI success or amendment merge is not WP3 completion. Complete TLS composition, funded SQLx TLS-positive evidence, PostgreSQL 17.6 positive plus hostile/denial qualification, independent whole-diff review, exact-head canonical CI, FULL Merge Queue and protected WP3 readback remain mandatory before WP3 release.

`MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`
