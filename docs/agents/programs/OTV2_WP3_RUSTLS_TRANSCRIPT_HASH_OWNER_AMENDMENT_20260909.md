# WP3 rustls transcript/hash owner seam conditional amendment

Coordinator: #162. Programme: #364. Existing sole material worker: #351 / PR #356. Parent decoded-state allocation: #425. Existing ClientHello caller allocation: #427.

## State

```yaml
allocation_id: OTV2-WP3-RUSTLS-TRANSCRIPT-HASH-OWNER-20260909
repository: Oteryn/Oteryn-Game
allocation_base_main_sha: fce21fda538e4a9cd6e8c1c1386b6b9f6a3edc89
meta_policy_binding: Oteryn/Oteryn@ed6c8c98605a7fbfea858e0ef616f89baa617262
meta_policy_version: 3.1.0
allocation_state: NOT_ACTIVE_CONDITIONAL
preparation_branch: coord/wp3-rustls-transcript-hash-owner-351
worker_branch: agent/sqlx-driver-budget-351
source_wp3_head: a538aeb1e0a01172db3cd87e4313c6498fcdf156
source_wp3_tree: fad2df6d37b3329eae4fe3ae17449d184e0744df
risk: HIGH
material_worker_dispatch_at_preparation: 5607037576
material_worker_authority: UNCHANGED
```

This is a docs-only conditional control-plane candidate for the SAME canonical WP3 worker. It creates no replacement worker and grants no present rustls, crypto-provider, SQLx, PostgreSQL, Game runtime, deployment or integration authority.

Material use is permitted only after this candidate itself is independently qualified, protected and explicitly activated by Work for the same #351/#356 lineage, and only if exact-current source/build evidence still proves the existing #425/#427 caller surfaces cannot carry the required transcript/hash allocation custody truthfully.

If a smaller already-authorized implementation is proven sufficient, this allocation remains inactive and unused.

## Exact source evidence

Read-only evidence is bound to canonical #356 head `a538aeb1e0a01172db3cd87e4313c6498fcdf156`, tree `fad2df6d37b3329eae4fe3ae17449d184e0744df`.

Relevant exact source:

- `vendor/rustls-0.23.43/src/hash_hs.rs` blob `8b4edc68fb2bd3f93191dc47b8f8a165c958b62c`;
- `vendor/rustls-0.23.43/src/client/hs.rs` current branch blob is already covered at the caller by protected #427 for `emit_client_hello_for_retry`;
- crypto-provider implementations are read-only evidence and remain excluded from authored authority.

Protected #425 grants in `hash_hs.rs` only:

- `HandshakeHashBuffer::{add_message,add_raw}`;
- `HandshakeHash::{add_message,add_raw,clone}`.

Those symbols can account transcript/client-auth vector growth and clone behavior, but they do not own the persistent/transient hash-context allocation boundaries below.

### Persistent start

`HandshakeHashBuffer::start_hash` calls `provider.start()` and returns a `HandshakeHash` containing `Box<dyn hash::Context>`. The provider context survives the caller and is moved through later handshake states.

A caller-side temporary guard cannot release after the returned context's actual destruction, and post-allocation accounting is forbidden. Keeping a caller guard alive separately through every later state would duplicate private `HandshakeHash` lifetime semantics and create a second custody plane.

### Normal HRR successor

`HandshakeHash::into_hrr_buffer` consumes the current context, obtains the old hash, constructs a synthetic handshake-hash message, and allocates its encoded `Vec` via `get_encoding()` before returning a new `HandshakeHashBuffer` that outlives the call.

The returned buffer must carry the exact encoded backing custody. A caller cannot attach exact backing custody after the allocation without violating reservation-before-allocation.

### Transient forks

`HandshakeHashBuffer::hash_given`, `HandshakeHash::hash_given`, and `HandshakeHash::current_hash` create provider contexts through `start`, `fork`, or `fork_finish`. Their backing is transient but must still be reserved before the hidden allocation and released only after the transient context is destroyed.

### Clone

`HandshakeHash::clone` is already inside #425, but its provider `ctx.fork()` allocation cannot be made truthful without the owner/provider-layout plumbing shared with the newly bounded context helpers below. Existing #425 clone authority remains authoritative; this amendment does not replace it.

### ECH exclusion

`HandshakeHash::rollup_for_hrr` is used by the ECH transcript path and is not required for the exact SQLx TCP acceptance graph. It remains excluded from this amendment. If an accepted future target requires ECH accounting, that is a separate source/capability decision.

## Why caller-only custody is insufficient

The accepted owner-aware path must reserve before allocation and bind custody to actual backing lifetime. For `start_hash` and `into_hrr_buffer`, the backing becomes part of the returned transcript object. The current authorized callers do not have a private field or returned token seam that can follow those objects without either:

- modifying the unallocated transcript symbols;
- duplicating `HandshakeHash`/`HandshakeHashBuffer` lifetime state outside `hash_hs.rs`; or
- using an opaque/magic whole-handshake reservation.

The latter two are forbidden by #425. Therefore, if the then-current worker still reaches this representation, the expected exact blocker is:

```text
SHARED_LEASE_REQUIRED =
vendor/rustls-0.23.43/src/hash_hs.rs ::
HandshakeHashBuffer::{start_hash,hash_given} +
HandshakeHash::{hash_given,into_hrr_buffer,current_hash}
(+ minimum private owner/layout/custody plumbing inseparable from those types)
:: provider-context and HRR-successor backing allocations outlive or are hidden from the existing caller-only ownership surfaces; exact reservation-before-allocation and destruction-bound release require transcript-local custody
```

This document pre-allocates only that bounded seam. It does not assert that activation has already occurred.

## Conditional future material authority

Only after protected integration, fresh overlap/custody readback and explicit Work activation for the SAME #351/#356 worker may this amendment add authored authority in:

### `vendor/rustls-0.23.43/src/hash_hs.rs`

New exact symbols:

- `HandshakeHashBuffer::start_hash`;
- `HandshakeHashBuffer::hash_given`;
- `HandshakeHash::hash_given`;
- `HandshakeHash::into_hrr_buffer`;
- `HandshakeHash::current_hash`;
- one minimum private owner-installation/layout/custody helper or private field set required by those exact symbols.

Existing #425 authority remains unchanged for:

- `HandshakeHashBuffer::{add_message,add_raw}`;
- `HandshakeHash::{add_message,add_raw,clone}`.

No other `hash_hs.rs` symbol is granted. In particular, `rollup_for_hrr` remains excluded.

### Existing caller use only

Protected #427 already owns `client/hs.rs::emit_client_hello_for_retry`. That caller may install the already-accepted resource owner into the transcript through the minimum newly authorized private `hash_hs.rs` seam before the first owner-aware transcript growth. This amendment adds no new `client/hs.rs` symbol authority.

Normal TLS 1.2/TLS 1.3 call sites may consume the owner-aware transcript methods without otherwise changing their semantics. Any additional authored caller symbol not already covered by #425/#427 requires a new exact blocker before mutation.

## Exact provider qualification requirements

This amendment authorizes accounting at the transcript call boundary, not crypto-provider implementation changes.

The owner-aware path must fail closed unless the exact provider/context allocation graph is qualified. The pinned final proof must remain bound to the repository's exact accepted graph, including:

- rustls `0.23.43`;
- aws-lc-rs `1.18.0`;
- aws-lc-sys `0.44.0`;
- pinned AWS-LC source used by protected #451;
- Rust `1.94.0` on the accepted Linux x86_64 qualification target.

Do not qualify by `HashAlgorithm` alone. Exact AWS-LC static hash-provider identity is required for an owner-aware hard-coded/provider-specific layout rule.

Where a Rust allocation layout is derivable from a public exact dependency type, production code must derive it from `Layout`/`size_of`/`align_of` rather than a guessed byte literal. C-side allocator charges must use the already-protected allocator policy from #451 and exact pinned source evidence. Any unqualified/custom provider under an owner-aware resource budget fails closed rather than allocating unaccounted context backing.

No mutation of `crypto/ring/hash.rs`, `crypto/aws_lc_rs/**`, aws-lc-rs, aws-lc-sys or AWS-LC source is authorized by this document.

## Required semantics

Any activated implementation must:

- reuse the SAME accepted #351/#356 `DeframerBufferOwner` ledger; no second owner plane;
- install the owner before the first owner-aware transcript/context allocation;
- reserve exact checked prospective Rust and C backing before `start`, `fork`, `fork_finish` or HRR successor encoding allocation;
- bind persistent start/fork custody to the actual `HandshakeHash` object and release only after the actual context backing is destroyed;
- keep old and new context charges simultaneously live across clone/HRR transition where both allocations coexist;
- separately account `HandshakeHashBuffer.buffer`, `HandshakeHash.client_auth`, and HRR synthetic encoding vector capacity using existing #425 growth/clone authority;
- release transient `hash_given`/`current_hash` context debit only after the transient context is destroyed;
- preserve ordinary owner-free and no_std behavior exactly;
- preserve TLS transcript bytes, hash selection, HRR semantics, PSK binder input, certificate verification and all wire/security behavior;
- introduce no new arbitrary semantic maximum, opaque whole-handshake reserve, global/thread-local owner, post-allocation catch-up, plaintext fallback or dependency/version change.

## Focused qualification

Before the conditional seam can be called proven, the SAME #356 lineage must provide executable evidence for at least:

- funded owner-aware `start_hash` with exact persistent provider-context backing charge;
- max-minus-one/underfunded denial before the first provider allocation;
- `HandshakeHash::clone` source+destination context overlap and independent final releases;
- `hash_given` and `current_hash` transient context charge/release;
- normal non-ECH HRR `start_hash -> into_hrr_buffer` transition with old context destruction, exact synthetic encoding backing and successor buffer custody;
- client-auth transcript vector growth/clone overlap where enabled;
- cancellation/error/connection drop with backing destruction before debit release;
- unqualified/custom provider owner-aware fail-closed control;
- ordinary owner-free/no_std controls unchanged.

The proof must be run on the exact patched repository graph used for final SQLx TLS-positive qualification. A standalone registry-selected or different dependency graph is not evidence.

## Explicit exclusions

This amendment grants no authority for:

- ECH transcript/accounting or `HandshakeHash::rollup_for_hrr`;
- generic `Codec::get_encoding` mutation;
- PSK binder helper symbols allocated separately, conditionally, by PR #493;
- crypto-provider implementation files or dependency source;
- verifier/certificate-verifier/key-schedule semantics;
- server-specific handshake implementation changes;
- Cargo/lock/version/feature mutation;
- SQLx/PostgreSQL source beyond existing #351/#430 authority;
- Foundation, WP4/#335, WP5/#319, Server Seam/#247, Platform, deployment, production or live credential changes.

Existing grants #425/#427/#429/#430/#432/#451/#453/#458/#466 remain unchanged.

## Integration and activation gates

This is a HIGH-risk material control-plane allocation candidate. It cannot authorize or integrate itself.

It remains `NOT_ACTIVE_CONDITIONAL` until all of the following are true:

1. exact-head deterministic/canonical checks are terminal GREEN;
2. one genuinely independent exact-head HIGH-risk/deep review reports no unresolved actionable finding;
3. a human owner explicitly authorizes repository integration of the then-current exact candidate; a generic instruction to continue useful work is not exact-candidate approval;
4. immediately before submission, fresh target-bound live preflight verifies repository, this PR, `base=main`, exact qualified/frozen head, authorization and Merge Queue eligibility;
5. submission follows the protected META 3.1 native route only: REST `merge-async`, exact qualified `sha`, explicit `merge_action="merge_queue"`;
6. HTTP `202` is acceptance only: bind the exact returned UUID to a positive executor-owned monotonic receipt sequence, then require immediate same-target readback carrying the same UUID at a strictly greater executor sequence; reconcile HTTP `200`/`409` from live state;
7. if the native operation is unavailable, integration state is exactly `BLOCKED_CAPABILITY_UNAVAILABLE`; preserve the candidate and use no forbidden substitute;
8. a real `merge_group` aggregate `game-gate` succeeds and protected-main readback proves this allocation present;
9. Work fresh-reads current #356 head, current protected main and all overlapping rustls custody;
10. the SAME #351/#356 worker or independent exact-current source proof still establishes the transcript-local seam as necessary;
11. Work explicitly activates this exact amendment for that SAME worker.

Direct/immediate merge, generic auto-merge, bypass, force, default merge action, no-op/retrigger commit, protection weakening and ambiguous dequeue are forbidden substitutes.

No activation is implied merely by protecting this document.

## Lifecycle

```text
conditional docs-only allocation candidate
-> exact-head checks
-> independent HIGH-risk/deep review
-> explicit human owner approval for exact candidate
-> governed META 3.1 Merge Queue submission/readback
-> real merge_group game-gate
-> protected-main readback
-> fresh #356/current-main/overlap proof
-> exact-current necessity still true
-> explicit Work activation
-> SAME #356 focused transcript/hash RED/GREEN
-> complete TLS composition
-> real SQLx AWS-LC TLS-positive
-> configured PostgreSQL 17.6 qualification
-> final independent whole-diff review + exact-head CI
-> governed Merge Queue + protected WP3 readback
-> only then WP4 resume
```

This allocation alone proves no TLS, PostgreSQL, WP3, WP4, WP5, G0 or Server Seam readiness.

`MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`
