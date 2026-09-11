# WP3 rustls TLS1.2 KX caller propagation conditional amendment

Coordinator: #162. Programme: #364. Existing sole material worker: #351 / Draft PR #356. Parent client-handshake authority: #518. Protected outbound-custody amendment: #535.

## State

```yaml
allocation_id: OTV2-WP3-RUSTLS-TLS12-KX-CALLER-PROPAGATION-20260910
repository: Oteryn/Oteryn-Game
allocation_base_main_sha: f921d182a4f2b435dcd8bdc13fc7ccb147860010
allocation_state: NOT_ACTIVE_CONDITIONAL
preparation_branch: coord/wp3-rustls-kx-caller-351
worker_branch: agent/sqlx-driver-budget-351
source_wp3_head: d90950237e1da5b8e0710f746417b2a603b7337e
source_tls12_blob: 3e17c9b9d778ae5336bc473a637b74810622dd4d
source_evidence_review: 5167948988
risk: HIGH
material_worker_status_at_allocation: STOPPED_SHARED_LEASE_REQUIRED
```

This is one minimal allocation-only control-plane amendment for the exact TLS1.2 caller propagation boundary found after protected #535 activation. It creates no replacement worker, branch or PR and grants no present material authority until protected integration/readback and explicit Work activation for the SAME #351/#356 lineage.

## Fresh source boundary

At current canonical WP3 source, `emit_client_kx` is called from `ExpectServerDone::handle` and returns `()`. Immediately after that call the caller computes the EMS seed and proceeds through later handshake work. A resource denial added inside `emit_client_kx` therefore cannot safely terminate the enclosing handshake unless the result is propagated by this caller.

The focused Astra source pass stopped before mutation and returned:

```text
SHARED_LEASE_REQUIRED = vendor/rustls-0.23.43/src/client/tls12.rs :: ExpectServerDone::handle, emit_client_kx call site :: propagate fallible KX reservation/transcript/send admission errors before EMS hashing, secret derivation, and successor-state admission
```

The source blob remains unchanged across the canonical `main` merge-ups since `933ccef...`; no competing WP3 material writer was introduced.

## Exact future material authority

Only after this document is protected-integrated, read back from `main`, and explicitly activated by Work for the SAME #351/#356 worker, authorize exactly:

### `vendor/rustls-0.23.43/src/client/tls12.rs`

- `ExpectServerDone::handle` **only at the existing `emit_client_kx` call site**;
- the minimum expression/control-flow change required to propagate the already-authorized fallible `emit_client_kx` result;
- no other statement, branch, state field, cryptographic operation or ordering change in `ExpectServerDone::handle`.

The existing #518 authority for `emit_client_kx` and the protected #535 authority for outbound queue custody remain separate and unchanged. This amendment does not widen either one.
## Required semantics

Any activated implementation must:

- make only the already-authorized `emit_client_kx` path fallible for resource admission/custody failures;
- propagate that failure at the existing call site before EMS seed calculation and before all later TLS1.2 client-flight work;
- ensure denial cannot continue into secret derivation, client CertificateVerify, ChangeCipherSpec, Finished, successor-state admission or outbound traffic;
- preserve the existing successful ordering exactly: client KX transcript/send work completes before EMS seed observation and later handshake work;
- preserve existing TLS alerts/error mapping unless a narrowly required existing resource-denial representation is already selected by #518/#535; do not invent a new protocol semantic;
- preserve TLS1.2 wire bytes, key exchange, randomness, EMS semantics, transcript semantics, certificate verification and cipher behavior on funded paths;
- preserve owner-free std/no_std behavior and all unrelated TLS1.2 states;
- add no new resource maximum, magic reservation, fallback, retry, suppression or post-allocation catch-up.

If a correct fallible call requires changing any other caller/callee symbol beyond the already-active #518/#535 surfaces and this single call site, stop before mutation with a new exact `SHARED_LEASE_REQUIRED`.

## Focused future qualification

The SAME #356 lineage must prove on the exact patched graph:

- max-minus-one owner denial from `emit_client_kx` is observed by `ExpectServerDone::handle` as an error;
- denial occurs before EMS seed capture and before later client-flight admission;
- no ClientKeyExchange/CCS/Finished bytes are accepted as successfully queued after the denied path;
- funded ECDHE and DHE continue with byte/order-equivalent ClientKeyExchange and normal later handshake behavior;
- existing owner-free control remains unchanged;
- focused Rust 1.94 tests/check/Clippy/fmt pass for the affected rustls surface.

This proof is only the missing call-site propagation prerequisite. It does not by itself prove #535 queue custody, complete TLS accounting, TLS-positive SQLx, PostgreSQL 17.6, WP3, B, G0 or Server Seam readiness.

## Explicit exclusions

No authority is granted for:

- any other statement or branch inside `ExpectServerDone::handle`;
- `ExpectServerKx::handle` beyond existing #518 authority;
- generic TLS message/codec/record-layer/fragmenter paths;
- #493 binder, #501 transcript/hash, verifier/provider, TLS1.3 or server TLS work;
- SQLx/PostgreSQL, Cargo/lock/features, workflows, registry, B/WP4/WP5/Server Seam, Platform/Atlas/META, production, secrets or live data;
- broad cleanup/refactoring, API redesign, direct merge, generic auto-merge, protection weakening, rebase/reset/force-push or no-op retrigger.

All prior protected grants remain authoritative only inside their exact scopes.
## Integration and activation gates

This HIGH-risk control-plane candidate cannot authorize or integrate itself. It remains `NOT_ACTIVE_CONDITIONAL` until:

1. the exact one-document candidate head/tree/diff is frozen and producer self-review has no unresolved P0/P1/P2;
2. all applicable deterministic exact-head repository checks are terminal GREEN;
3. a genuinely independent exact-head HIGH-risk/deep review reports P0/P1/P2=0, with zero unresolved review threads and zero outstanding requested-changes reviews;
4. the human owner explicitly authorizes the exact reviewed candidate head for protected integration;
5. fresh target-bound preflight verifies repository, PR, `base=main`, exact head, authorization and eligibility;
6. only META 3.1 native REST `PUT /repos/{owner}/{repo}/pulls/{pull_number}/merge-async` is used with exact `sha` and `merge_action="merge_queue"`;
7. the returned UUID is bound to an executor-owned receipt sequence and a strictly later live PR readback for the same repo/PR/base/head;
8. the real `merge_group` aggregate `game-gate` succeeds;
9. protected `main` is read back and proves this exact amendment is present;
10. Work freshly verifies SAME #351/#356 custody and explicitly activates this amendment for that lineage.

If the native exact-head Merge Queue primitive is unavailable, preserve the candidate and return `BLOCKED_CAPABILITY_UNAVAILABLE`. Direct/immediate merge, generic auto-merge, GraphQL enqueue, bypass, force/rebase/reset and no-op/retrigger commits are forbidden substitutes.

## Lifecycle

```text
allocation-only NOT_ACTIVE_CONDITIONAL
-> exact-head self-review + CI
-> independent HIGH-risk exact-head review
-> exact-candidate owner authorization
-> META 3.1 native merge-async Merge Queue
-> real merge_group game-gate SUCCESS
-> protected-main readback
-> explicit Work activation for SAME #351/#356
-> focused call-site RED/GREEN with active #518 + #535
-> continue complete TLS qualification
```

This allocation proves no TLS, PostgreSQL, WP3, WP4, WP5, G0 or Server Seam readiness.

`MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`
