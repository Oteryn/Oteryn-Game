# WP3 rustls client handshake allocation seams conditional amendment

Coordinator: #162. Programme: #364. Existing sole material worker: #351 / #356. Parent decoded-owner allocation: #425. Existing ClientHello caller amendment: #427.

## State

```yaml
allocation_id: OTV2-WP3-RUSTLS-CLIENT-HANDSHAKE-ALLOCATION-SEAMS-20260909
repository: Oteryn/Oteryn-Game
allocation_base_main_sha: 247818666bbbcb13fa27e24ec90fee24c793a416
allocation_state: NOT_ACTIVE_CONDITIONAL
preparation_branch: coord/wp3-rustls-client-lease-seams-351
worker_branch: agent/sqlx-driver-budget-351
source_wp3_head: f64123394435256391203b45a8d63e058428a0c2
source_wp3_tree: 382df34032ab2e6a0edf68421b5d09a0b0c1cdcd
terminal_return_comment: 5608896310
risk: HIGH
material_worker_status_at_allocation: STOPPED_SHARED_LEASE_REQUIRED
```

This is one minimal allocation-only control-plane amendment for two source-proven client-handshake allocation seams. It is `NOT_ACTIVE_CONDITIONAL`: it creates no replacement material worker and grants no present rustls, SQLx, PostgreSQL, Game runtime, deployment or integration authority. It does not resume #356.

The two seams may safely share one amendment because both are bounded client-handshake admission/custody gaps on the same accepted operation owner, neither changes the other's interface, and the exact union below does not require file-wide or subsystem-wide authority. Activation and implementation may still proceed boundary-by-boundary and must fail closed independently at either seam.

## Fresh live and source evidence

Immediately before allocation mutation, GitHub LIVE readback established:

- protected `main` = `247818666bbbcb13fa27e24ec90fee24c793a416`;
- canonical Draft PR #356 head = `f64123394435256391203b45a8d63e058428a0c2`, tree `382df34032ab2e6a0edf68421b5d09a0b0c1cdcd`;
- canonical branch = `agent/sqlx-driver-budget-351` at the same head;
- terminal material return = comment `5608896310` at that head;
- no active replacement material #356 worker or separate allocation PR for these two seams was found;
- #493 remains protected allocation-only `NOT_ACTIVE_CONDITIONAL`, and #501 remains a separate conditional transcript/hash candidate. Neither is activated or duplicated here.

Fresh read-only inspection is bound to that exact #356 head/tree. Relevant blobs are:

- `vendor/rustls-0.23.43/src/client/tls12.rs` — `f2d0da472547352489f8d11fce8fe4f865743092`;
- `vendor/rustls-0.23.43/src/verify.rs` — `46731548ad74be1860d669df320774afabf302e0`;
- `vendor/rustls-0.23.43/src/webpki/server_verifier.rs` — `5aa9235054e8615f5cade08a505a78e21c5ce566`;
- read-only caller `vendor/rustls-0.23.43/src/client/hs.rs` — `4ac00c3dede88082591c4b09fce36aa7ad5be8dc`.

### TLS 1.2 retained/outbound backing seam

`ExpectServerKx::handle` creates `kx_params` with `Vec::new()` plus `kx.params.encode(&mut kx_params)` and retains that signed-parameter backing in `ServerKxDetails` through later signature verification. The first allocation and growth therefore occur inside this omitted symbol; charging in the already-authorized downstream `ExpectServerDone::handle` would be post-allocation.

`emit_client_kx` separately creates a public-key destination with `pub_key.to_vec()` and an encoded ClientKeyExchange buffer with `Vec::new()` plus `encode`. Both first allocate inside this omitted symbol. Caller-side logical-length charging cannot establish the encoded buffer's actual capacity or keep exact custody through transcript/send ownership.

These are two distinct symbols but one TLS 1.2 client KX backing seam. The minimum authority is the two symbols plus only inseparable private owner-aware custody carried by their existing `ServerKxDetails`/message handoff. No broad `client/tls12.rs` authority is needed.

### Custom verifier supported-scheme seam

`ServerCertVerifier::supported_verify_schemes(&self) -> Vec<SignatureScheme>` allocates the returned vector inside an arbitrary trait implementation. The #427 caller in `emit_client_hello_for_retry` receives the vector only after this allocation, so caller-visible preflight is too late.

The built-in `WebPkiServerVerifier` delegates to a static supported-algorithm mapping, but the caller has `Arc<dyn ServerCertVerifier>` and cannot soundly downcast, replace, or infer a custom implementation's result. Preserving custom-verifier semantics therefore requires an additive owner-aware trait dispatch with a fail-closed default, and a built-in implementation that reserves before constructing its ordinary-equivalent result. No verifier policy, verification method, provider selection, or certificate semantics change is required.

## Exact future material authority

Only after protected integration, protected-main readback, fresh necessity/custody verification and explicit Work activation for the SAME canonical #351/#356 worker, this amendment MAY authorize exactly:

### `vendor/rustls-0.23.43/src/client/tls12.rs`

- `ExpectServerKx::handle` only;
- `emit_client_kx` only;
- the minimum private owner/custody field or helper inseparable from `ServerKxDetails` and the existing message/transcript/send handoff, only where exact source/build proves it necessary to carry the already-reserved backing through its real lifetime.

The implementation may add fallible owner-aware siblings or minimum factoring while preserving the ordinary implementations byte-for-byte and behaviorally equivalent when no owner is supplied.

### `vendor/rustls-0.23.43/src/verify.rs`

- `ServerCertVerifier` only for one additive object-safe owner-aware supported-scheme extraction method;
- its default must fail closed before invoking ordinary `supported_verify_schemes` and before any verifier-owned result allocation;
- only the minimum private bounded error/plumbing needed by that method, reusing an existing non-allocating owner-denial representation where possible.

The ordinary `supported_verify_schemes` method and all verification methods remain unchanged.

### `vendor/rustls-0.23.43/src/webpki/server_verifier.rs`

- `WebPkiServerVerifier` implementation of the additive owner-aware supported-scheme method only;
- only minimum private factoring required to reserve from the static mapping before constructing the same ordered result.

### Existing authority, not a new grant

The already-protected #427 `vendor/rustls-0.23.43/src/client/hs.rs :: emit_client_hello_for_retry` caller may invoke the new verifier seam and carry returned custody using its existing exact authority. This amendment does not expand that file or symbol.

No other rustls path or symbol is granted.

## Required semantics

Any activated implementation must:

- reuse the SAME accepted #351/#356 operation owner and ledger; no second allowance, owner model, registry, global or thread-local state;
- deny before every unauthorized allocation or growth with checked arithmetic;
- charge actual `Vec` capacity/backing and exact private control backing, not logical length, requested delta or a copied allocation schedule;
- preserve old/new and source/destination overlap until old/source backing is actually destroyed or custody is explicitly transferred;
- carry custody with retained `ServerKxDetails`, transcript/send message backing and verifier result until the actual backing is destroyed;
- roll back only after any partially constructed backing has been destroyed on encode, verification, cancellation, connection error or drop;
- make the custom-verifier owner-aware default fail closed before calling the ordinary allocating method; never allocate ordinarily and charge afterward;
- make the built-in WebPki owner-aware result identical in order and contents to `supported_verify_schemes()`;
- preserve arbitrary custom verifier semantics by opt-in implementation, rather than replacing its advertised schemes or downcasting it;
- preserve TLS 1.2 and TLS 1.3 wire/security behavior, certificate and hostname verification, signature preference, key exchange, randomness, transcript and alert semantics;
- preserve ordinary owner-free and `no_std` behavior and API compatibility;
- add no arbitrary count/field cap, new numeric policy, magic whole-message/whole-handshake/whole-slot reservation, plaintext fallback or unowned retry after denial.

If exact implementation requires another rustls path/symbol, it must stop before mutation with a new exact `SHARED_LEASE_REQUIRED`; proximity to an authorized symbol does not grant authority.

## Focused future qualification

Before either seam can be treated as proven, the SAME #356 lineage must provide executable exact-graph evidence for:

### TLS 1.2 KX backing

- retained signed-parameter backing reserved before the first allocation/growth in `ExpectServerKx::handle`;
- actual capacity and checked overflow qualification;
- max-minus-one denial before allocation and before state admission;
- custody through successor states and signature verification, including verifier error, cancellation and connection drop;
- public-key copy and encoded ClientKeyExchange buffer reserved before allocation in `emit_client_kx`;
- simultaneous public-key/source, encoded message, transcript and outbound-send overlap;
- exact release after each backing's actual destruction or proved charged transfer;
- successful TLS 1.2 handshake with unchanged ClientKeyExchange bytes;
- ordinary owner-free/no_std control unchanged.

### Verifier supported schemes

- unsupported custom verifier fails before its ordinary allocating method is called;
- an opted-in custom verifier preserves its exact arbitrary ordered result and owner identity;
- built-in WebPki result is byte/order equivalent to the ordinary method;
- pre-reservation covers actual returned capacity, with max-minus-one denial before allocation;
- result custody transfers into ClientHello extension ownership without re-reserve or early release;
- error, cancellation and connection drop release only after backing destruction;
- TLS wire, certificate, hostname and signature-verification behavior remain unchanged;
- ordinary owner-free custom and built-in verifier controls remain unchanged.

Focused proof does not substitute for #493/#501 conditions, complete TLS composition, real funded SQLx AWS-LC TLS-positive evidence, PostgreSQL 17.6 qualification or final whole-diff review.

## Explicit exclusions

This amendment grants no authority for:

- broad `client/tls12.rs` changes or any other TLS 1.2 state/symbol;
- the `ServerCertVerifier` verification methods, `ClientCertVerifier`, verifier policy, provider selection or custom-verifier redesign;
- any other `verify.rs` or `webpki/server_verifier.rs` symbol;
- crypto providers, KX providers, `Codec`, generic message/handshake helpers, server TLS, ECH, transcript/hash implementation or binder internals;
- #493 PSK binder scope or #501 transcript/hash scope, both of which remain separate and `NOT_ACTIVE_CONDITIONAL`;
- Cargo/lock/version/feature changes;
- SQLx/PostgreSQL source, SQL/PG decoding, migrations, B/Foundation, WP4/#335, WP5, Server Seam/#247, Platform, production, secrets or live data.

All prior protected grants remain unchanged within their exact scopes.

## Integration and activation gates

This is a HIGH-risk control-plane allocation candidate and cannot authorize or integrate itself. It remains `NOT_ACTIVE_CONDITIONAL` until all of the following complete:

1. freeze the exact effective diff and exact candidate head/tree, then record a producer exact-head full-diff self-review with no unresolved producer P0/P1/P2 findings;
2. run deterministic exact-head repository checks and require all applicable checks terminal GREEN;
3. obtain genuinely independent exact-head HIGH-risk/deep review with no unresolved P0/P1/P2, and require zero unresolved review threads and zero outstanding requested-changes reviews before owner authorization or integration preflight;
4. obtain explicit human-owner authorization for that exact candidate, not a generic continuation instruction;
5. immediately before integration, fresh target-bound live preflight verifies repository, PR, `base=main`, exact qualified head, authorization, eligibility and intended protected base/queue state;
6. submit only through the META 3.1 native exact-head REST `merge-async` route with exact `sha` and explicit `merge_action="merge_queue"`;
7. capture HTTP `202` UUID and a positive executor receipt sequence, then immediately read back the same target and UUID at a strictly later executor sequence; reconcile `200`/`409` without treating acceptance as merge proof;
8. forbid direct/immediate merge, generic auto-merge, GraphQL enqueue, bypass, force/rebase/reset, protection changes, default merge action and no-op/retrigger commits;
9. require the real Merge Queue `merge_group` aggregate `game-gate` to succeed;
10. read protected `main` back and prove this exact allocation is present;
11. Work returns exact protected evidence to the canonical worker, freshly verifies #351/#356 head and custody, and confirms #493/#501 remain separate;
12. Work explicitly activates this exact amendment for the SAME canonical #351/#356 worker.

If the native exact-head Merge Queue operation is unavailable, preserve the qualified candidate and report `BLOCKED_CAPABILITY_UNAVAILABLE`; do not substitute another merge primitive.

Protection of this document alone does not activate it. Even after protection, material #356 MUST NOT resume until #501 is also protected and Work explicitly activates the required protected amendments for that SAME canonical worker.

## Lifecycle

```text
allocation-only NOT_ACTIVE_CONDITIONAL candidate
-> exact effective diff + producer exact-head full-diff self-review
-> deterministic exact-head checks
-> independent HIGH-risk exact-head review with P0/P1/P2=0
-> zero unresolved review threads + zero outstanding requested-changes reviews
-> explicit owner authorization for exact candidate
-> fresh target-bound exact-head/base/auth/eligibility preflight
-> native REST merge-async exact sha + merge_action=merge_queue
-> same-UUID later-sequence readback
-> real merge_group game-gate SUCCESS
-> protected-main readback
-> return exact protected evidence to Work
-> WAIT for #501 protected readback
-> fresh SAME #351/#356 custody/necessity verification
-> explicit Work activation for SAME canonical worker
-> focused boundary-by-boundary RED/GREEN
-> complete TLS accounting and qualification lifecycle
```

This allocation proves no TLS, PostgreSQL, WP3, WP4, WP5, G0 or Server Seam readiness.

`MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`
