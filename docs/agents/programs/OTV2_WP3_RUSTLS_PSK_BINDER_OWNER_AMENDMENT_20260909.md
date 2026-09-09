# WP3 rustls PSK binder owner seam conditional amendment

Coordinator: #162. Programme: #364. Existing sole material worker: #351 / #356. Parent decoded-owner allocation: #425. Existing ClientHello caller amendment: #427.

## State

```yaml
allocation_id: OTV2-WP3-RUSTLS-PSK-BINDER-OWNER-20260909
repository: Oteryn/Oteryn-Game
allocation_base_main_sha: ece300c384aa1e53f208975f25e94635c2fcc7ce
governance_reconciliation_main_sha: fce21fda538e4a9cd6e8c1c1386b6b9f6a3edc89
allocation_state: NOT_ACTIVE_CONDITIONAL
preparation_branch: coord/wp3-rustls-psk-binder-owner-351
worker_branch: agent/sqlx-driver-budget-351
source_wp3_head: a538aeb1e0a01172db3cd87e4313c6498fcdf156
source_wp3_tree: fad2df6d37b3329eae4fe3ae17449d184e0744df
risk: HIGH
material_worker_status_at_allocation: BLOCKED_CODEX_USAGE_LIMIT
```

This is an allocation-only, conditional control-plane amendment for the SAME canonical WP3 worker. It creates no replacement worker and grants no present rustls, SQLx, PostgreSQL, Game runtime or deployment authority.

Material application is permitted only after all repository lifecycle gates below are protected and a recovered #351/#356 worker proves from the then-current exact source/build that the already-active #427 caller-only authority cannot enforce reservation-before-allocation for the binder-signing temporary backing. If caller-only implementation is proven sufficient, this allocation MUST remain inactive and unused.

## Exact source evidence

Fresh read-only inspection is bound to canonical #356 head `a538aeb1e0a01172db3cd87e4313c6498fcdf156` and tree `fad2df6d37b3329eae4fe3ae17449d184e0744df`.

Relevant exact blobs:

- `vendor/rustls-0.23.43/src/client/hs.rs` — `4ac00c3dede88082591c4b09fce36aa7ad5be8dc`;
- `vendor/rustls-0.23.43/src/client/tls13.rs` — `664f1047f29a16c63b303d8c7df31813cceb60b5`;
- `vendor/rustls-0.23.43/src/msgs/handshake.rs` — `04c84e49e9e235c2a011ddf02c9c0a41387346ee`;
- read-only generic `vendor/rustls-0.23.43/src/msgs/codec.rs` — `41a726bca89da2ef3dce5999035126e2288f060e`.

Protected #427 grants only:

`vendor/rustls-0.23.43/src/client/hs.rs :: emit_client_hello_for_retry`.

The exact current resumption path inside that authorized caller invokes:

`tls13::fill_in_psk_binder(&tls13_session, &transcript_buffer, &mut chp)`.

The callee currently exposes four allocation points relevant to the accepted owner accounting:

1. `HandshakeMessagePayload::encoding_for_binder_signing()` calls generic `Codec::get_encoding()`;
2. generic `Codec::get_encoding()` creates `Vec::new()` and calls `encode(&mut Vec<u8>)`, so the allocation/growth occurs inside an unallocated generic helper with no owner or caller-provided preallocation seam;
3. `HandshakeMessagePayload::total_binder_length()` creates a second `Vec::new()` named `binders_encoding` and encodes the binder list into it;
4. `fill_in_psk_binder()` later allocates the replacement binder through `real_binder.as_ref().to_vec()` while the dummy binder exists.

The dummy binder, ticket identity and replacement binder lengths are source-known and remain expected to be handled under existing #427 caller authority where exact prospective accounting is possible. The blocker candidate is narrower: the two internal binder-signing encoding vectors have allocation/capacity behavior hidden behind unallocated helpers, and their actual backing cannot be charged after the allocation without violating the accepted reservation-before-allocation contract.

A fixed whole-ClientHello reserve, guessed capacity, post-allocation catch-up, duplicate opaque estimate, hidden second ledger or broad generic `Codec` rewrite is forbidden.

## Conditional blocker

The expected minimum blocker, if the recovered material worker confirms caller-only #427 cannot satisfy the contract, is:

```text
SHARED_LEASE_REQUIRED =
vendor/rustls-0.23.43/src/client/tls13.rs :: fill_in_psk_binder
+ vendor/rustls-0.23.43/src/msgs/handshake.rs :: HandshakeMessagePayload::{encoding_for_binder_signing,total_binder_length}
(+ only the minimum adjacent private ClientHello binder-length/preallocation helper if exact proof requires it)
:: internal PSK binder-signing Vec allocations have no caller-visible exact preallocation/capacity seam; #427 authorizes only emit_client_hello_for_retry
```

This document pre-allocates only that bounded seam. It does not assert that material activation is already necessary.

## Exact future material authority

Only after protected integration, fresh custody readback and explicit Work activation for the SAME #351/#356 worker, the amendment MAY add the following authority:

### `vendor/rustls-0.23.43/src/client/tls13.rs`

- `fill_in_psk_binder` only;
- only the minimum additive owner-aware/fallible sibling or private plumbing inseparable from that function if needed to receive the existing accepted owner and return the existing key-schedule result without changing wire semantics.

### `vendor/rustls-0.23.43/src/msgs/handshake.rs`

- `HandshakeMessagePayload::encoding_for_binder_signing` only;
- `HandshakeMessagePayload::total_binder_length` only;
- only the minimum adjacent private `ClientHello`/PSK binder encoded-length or exact-preallocation helper required to make those two functions allocation-aware without touching generic `Codec`.

No other symbol in either file is granted by this amendment.

## Required semantics

Any activated implementation must:

- reuse the SAME accepted #351/#356 resource owner and ledger; no second owner plane;
- reserve checked exact prospective backing before each binder-signing allocation or growth;
- use source-derived/non-allocating size calculation where exact preallocation is required;
- construct destination vectors with the proven exact capacity, verify the resulting capacity, and fail closed if the allocator/result cannot satisfy the proven representation;
- account simultaneous-live source/destination and dummy/replacement overlap truthfully;
- retain reservation custody through the actual backing lifetime and release only after backing destruction on success, error, cancellation or unwind;
- avoid reserve-again when already-charged backing is transferred rather than copied;
- preserve ordinary owner-free TLS behavior, PSK binder bytes, transcript input, cipher-suite/hash selection, ticket identity, HRR behavior and all verification/security semantics;
- preserve no_std behavior where the owner capability is unavailable;
- add no new semantic resource maximum and no fallback to unowned allocation after an owner-aware denial.

The implementation MUST NOT change the generic `Codec::get_encoding()` contract merely for convenience. If generic codec authority is genuinely unavoidable, stop again with a new exact evidence-backed blocker instead of widening this amendment implicitly.

## Focused qualification

Before this conditional seam can be considered proven, the SAME #356 lineage must provide executable evidence within already-authorized test/provenance surfaces for at least:

- binder-signing full ClientHello temporary backing reserved before allocation;
- binder-list temporary backing reserved before allocation;
- dummy-binder and real-binder replacement overlap;
- exact-capacity or exact-request validation for every newly owner-aware temporary;
- max-minus-one/underfunded denial before the first unauthorized allocation;
- failure/unwind after the first temporary but before binder replacement with no leaked or early credit;
- successful binder generation producing the same wire/transcript semantics as the ordinary path;
- HRR/resumption case where the same binder path is exercised again, when reachable in the pinned graph;
- ordinary owner-free control unchanged.

This focused proof does not substitute for the remaining complete-TLS matrix, real funded SQLx AWS-LC TLS-positive evidence or configured PostgreSQL 17.6 qualification.

## Explicit exclusions

This amendment grants no authority for:

- generic `vendor/rustls-0.23.43/src/msgs/codec.rs` mutation;
- `prepare_resumption`, unless a future separately protected amendment proves caller-side precharge cannot cover its source-known dummy binder/ticket allocation;
- any ECH state/configuration accounting;
- server-side TLS paths;
- crypto provider, verifier, certificate verifier, key schedule, hash-provider implementation or dependency source changes;
- any other `client/tls13.rs`, `msgs/handshake.rs` or `client/hs.rs` symbol;
- Cargo/lock/version/feature mutation;
- SQLx/PostgreSQL source, migrations, Foundation, WP4/#335, WP5/#319, Server Seam/#247, Platform, production or deployment changes.

All existing grants #425/#427/#429/#430/#432/#451/#453/#458/#466 remain unchanged and authoritative within their exact scopes.

## Integration and activation gates

The historical creation provenance remains `allocation_base_main_sha` `ece300c384aa1e53f208975f25e94635c2fcc7ce`. This repair reconciles governance against protected `main` `fce21fda538e4a9cd6e8c1c1386b6b9f6a3edc89`, whose `docs/agents/META_AGENT_POLICY_BINDING.json` binds `OTERYN_ORGANIZATION_AGENT_POLICY` v3.1.0 at `Oteryn/Oteryn@ed6c8c98605a7fbfea858e0ef616f89baa617262`. Future integration inherits the then-current protected binding rather than freezing this document to a superseded integration algorithm.

Historical exact head `2de8e9415f2d452d0589a97b0cbd8832e85c982e` carried the superseded v3.0 rule requiring one queue mutation to atomically fence the exact head and expected `main` base/queue. Independent exact-head HIGH-risk review found that active requirement to be a P1 after protected #497 adopted META 3.1. This repair removes that superseded algorithm as active authority while retaining the historical fact.

This is a material HIGH-risk control-plane allocation. A candidate-controlled mechanism cannot be the sole authority for integrating its own control-plane authority change. Therefore this allocation remains `NOT_ACTIVE_CONDITIONAL` until all of the following are true:

1. this exact allocation candidate receives genuinely independent exact-head HIGH-risk/deep review with no unresolved actionable finding;
2. exact-head deterministic/canonical repository checks are terminal GREEN;
3. a human owner explicitly authorizes repository integration of the then-current exact #493 candidate; a general instruction to continue useful work MUST NOT be reinterpreted as specific candidate approval;
4. immediately before submission, fresh target-bound live preflight verifies the exact repository, PR #493, `base=main`, exact qualified/frozen head, authorization and queue eligibility;
5. submission uses only REST `PUT /repos/{owner}/{repo}/pulls/{pull_number}/merge-async` with the exact qualified head in `sha` and explicit `merge_action="merge_queue"`; default merge action, direct/immediate merge, generic `enablePullRequestAutoMerge`, bypass, force, protection changes, no-op/retrigger commits and ambiguous dequeue are forbidden substitutes;
6. HTTP `202` is acceptance only: capture the exact returned async UUID and a positive executor-owned monotonic receipt sequence, then immediately obtain same-target live readback that carries the same UUID at an executor sequence strictly greater than the receipt sequence and still binds the same repository, PR number, `base=main` and exact head. Reconcile HTTP `200` or `409` and the async request/status from live state; none is terminal integration proof by itself;
7. if the selected native exact-head operation is unavailable, integration is exactly `BLOCKED_CAPABILITY_UNAVAILABLE`, with the qualified candidate preserved and no forbidden substitute;
8. actual Merge Queue admission is non-terminal: a real `merge_group` aggregate `game-gate` must succeed and protected-main readback must prove this exact allocation is present;
9. Work freshly re-reads the canonical #356 branch, then-current protected main and all overlapping rustls custody;
10. no replacement/competing rustls material worker exists;
11. the recovered SAME #351/#356 worker provides exact source/build evidence that #427 caller-only implementation is insufficient;
12. Work explicitly activates this amendment for that SAME worker.

No activation is implied merely by merging this document.

## Integration lifecycle

```text
conditional allocation-only amendment
-> deterministic exact-head validation
-> one independent exact-head HIGH-risk/deep review
-> explicit human-owner authorization for the exact candidate
-> fresh target-bound repo/PR/base=main/exact-head/auth/eligibility preflight
-> REST merge-async with exact sha + explicit merge_action="merge_queue"
   OR BLOCKED_CAPABILITY_UNAVAILABLE with the qualified candidate preserved
-> accepted UUID + positive executor receipt sequence
-> immediate same-target same-UUID readback at a strictly greater executor sequence
-> reconcile HTTP 200/409 and async status without treating admission as terminal
-> real merge_group aggregate game-gate success
-> protected-main readback
-> WAIT for recovered SAME #351/#356 worker
-> prove caller-only #427 insufficient on then-current exact source/build
-> fresh custody/overlap readback
-> explicit Work activation of this exact seam
-> SAME #356 worker implements focused RED/GREEN
-> continue complete TLS accounting
-> real funded SQLx AWS-LC TLS-positive
-> configured PostgreSQL 17.6 positive/hostile/denial/recovery
-> final independent whole-diff exact-head review
-> canonical exact-head CI + governed normal FULL Merge Queue
-> protected WP3 readback and explicit custody release
-> only then resume WP4/#335
```

This allocation alone proves no TLS, PostgreSQL, WP3, WP4, WP5, G0 or Server Seam readiness.

`MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`
