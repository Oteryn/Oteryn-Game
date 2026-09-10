# WP3 rustls transcript caller propagation amendment — 2026-09-10

Coordinator: #162. Programme: #364. Existing sole material worker: #351 / Draft PR #356. Existing transcript-owner allocation: #501. Existing ClientHello caller allocation: #427.

## State

```yaml
allocation_id: OTV2-WP3-RUSTLS-TRANSCRIPT-CALLER-PROPAGATION-20260910
repository: Oteryn/Oteryn-Game
coordinator_issue: 162
worker_issue: 351
worker_pr: 356
allocation_base_main_sha: 149e1e5cc3dd09b4bbf53e9213b92901233da210
source_wp3_head: 507483d4986c4299587b9e01e932b3fc189b8a83
source_wp3_tree: 9fdcc60746dad163df2f7cd57accb7f338cdab26
source_client_hs_blob: 12203e3fd0fbc3f5476256847fa45b9c9dd73c9d
worker_branch: agent/sqlx-driver-budget-351
allocation_state: NOT_ACTIVE_CONDITIONAL
risk: HIGH
material_worker_authority: UNCHANGED_UNTIL_PROTECTED_AND_ACTIVATED
replacement_worker_authority: NONE
```

This is a docs-only control-plane amendment for the SAME canonical WP3 #351/#356 worker. It creates no replacement worker, branch or material PR and grants no present runtime mutation authority.

The current owner directive authorizes the coordinator to prepare, qualify, integrate and activate this bounded unblock transaction subject to the active repository/META review, Merge Queue and protected-readback gates. It does not authorize bypass of those gates.

## Fresh live blocker

At protected `main@149e1e5cc3dd09b4bbf53e9213b92901233da210`, canonical WP3 is Draft PR #356 on `agent/sqlx-driver-budget-351@507483d4986c4299587b9e01e932b3fc189b8a83`, tree `9fdcc60746dad163df2f7cd57accb7f338cdab26`. The worker is synchronized with this protected main and its exact-head repository checks are green.

The already-protected #501 transcript seam is materially present on the WP3 branch. `vendor/rustls-0.23.43/src/hash_hs.rs` already provides owner-aware `try_start_hash`, `try_add_message`, `try_into_hrr_buffer`, `try_hash_given`, `try_current_hash` and related custody support.

The first still-unallocated reachable caller boundary is in `vendor/rustls-0.23.43/src/client/hs.rs`:

1. `ExpectServerHello::handle` still calls ordinary `HandshakeHashBuffer::start_hash(...)` and ordinary `HandshakeHash::add_message(...)` before dispatching into TLS 1.2 or TLS 1.3 handling.
2. `ExpectServerHelloOrHelloRetryRequest::handle_hello_retry_request` still calls ordinary `start_hash(...)`, `into_hrr_buffer()` and `add_message(...)` before the second ClientHello path.

On an owner-aware SQLx client path, the ordinary transcript methods deliberately reject owner-bearing transcript state. The existing #427 grant covers only `emit_client_hello_for_retry`; it does not authorize authored changes in these two handlers. The protected #501 grant owns the transcript-local primitives, not these caller control-flow sites.

Therefore the exact current classification is:

```text
SHARED_LEASE_REQUIRED =
vendor/rustls-0.23.43/src/client/hs.rs ::
ExpectServerHello::handle +
ExpectServerHelloOrHelloRetryRequest::handle_hello_retry_request
:: propagate the existing SAME resource owner through #501 fallible transcript start/growth/HRR transition before later client-handshake work; #427 covers only emit_client_hello_for_retry
```

This is an implementation ownership boundary inside accepted resource-accounting semantics. It does not require a new numeric resource limit, transport/security choice, public API, schema or persistence decision and therefore is not an architecture escalation by itself.

## Conditional future material authority

Only after this amendment is independently qualified, protected-integrated, read back from protected `main`, freshly reconciled against the then-current #356 head, and explicitly activated by Work for the SAME worker, the material branch may modify exactly these additional symbols:

### `vendor/rustls-0.23.43/src/client/hs.rs`

- `ExpectServerHello::handle`;
- `ExpectServerHelloOrHelloRetryRequest::handle_hello_retry_request`.

The permitted authored delta is limited to the minimum control-flow required to consume the already-existing #501 owner-aware transcript methods and propagate their existing fail-closed error type. This includes only mechanically necessary local binding/branching needed to select the owner-aware path while preserving the ordinary owner-free path.

No whole-file authority follows. No other `client/hs.rs` symbol is added by this amendment. Existing #427 authority for `emit_client_hello_for_retry` remains unchanged.

## Required semantics

The activated implementation must:

- reuse the SAME `DeframerBufferOwner` / #351 resource ledger already carried by `ClientHelloInput.resource_owner`; never create a second budget or owner plane;
- in `ExpectServerHello::handle`, use the #501 fallible owner-aware transcript start and message-add path when the accepted resource owner is present, and fail closed before any later TLS 1.2/TLS 1.3 key-schedule or successor-state work if reservation fails;
- in `ExpectServerHelloOrHelloRetryRequest::handle_hello_retry_request`, use the #501 fallible owner-aware `start_hash -> into_hrr_buffer -> add_message` sequence when the resource owner is present;
- preserve transcript/context and HRR-successor custody through their real backing lifetime as implemented by #501;
- propagate denial before the second ClientHello and before later HRR successor work that would rely on a successfully established transcript;
- preserve the exact successful transcript bytes, cipher-suite selection, HRR semantics, TLS 1.2/TLS 1.3 dispatch, certificate verification, wire behavior and owner-free behavior;
- introduce no new wire/count cap, magic reserve, post-allocation catch-up, plaintext fallback, feature/version change, dependency change, global/thread-local owner, test suppression or security downgrade.

If exact-current source proves one of these calls already uses an equivalent owner-aware path before mutation, do not duplicate it; shrink the material delta accordingly.

## Focused qualification after activation

The SAME #356 lineage must provide executable Rust 1.94 evidence for this exact caller unit before moving on:

1. funded owner-aware normal ServerHello path reaches both the existing TLS 1.2 and TLS 1.3 successor paths with the SAME owner and transcript semantics;
2. underfunded owner-aware transcript-context admission fails before provider-context allocation and before later handshake successor work;
3. owner-aware ServerHello transcript growth uses `try_add_message` and releases/transfers backing only through #501 custody;
4. a budget that can fund transcript/provider-context creation but cannot fund the received ServerHello transcript growth must fail specifically at the propagated `try_add_message` boundary before TLS 1.2/TLS 1.3 dispatch or successor-state work;
5. funded HRR path performs `try_start_hash -> try_into_hrr_buffer -> try_add_message` and can continue to the existing `emit_client_hello_for_retry` seam;
6. underfunded HRR context or `try_into_hrr_buffer` successor-buffer admission fails before second-ClientHello material work;
7. a budget that can fund `try_start_hash` and `try_into_hrr_buffer` but cannot fund the subsequent HRR `try_add_message` growth must fail at that propagated add boundary before second-ClientHello material work;
8. owner-free TLS 1.2, TLS 1.3 and HRR controls remain behavior-equivalent;
9. focused check/Clippy/tests required by the current exact dependency graph remain green.

Exact-head repository CI remains required after the material checkpoint.

## Known downstream census — not granted here

Fresh source inspection confirms additional owner-aware composition work remains after these two callers. Examples include ordinary transcript consumers in client TLS 1.2/TLS 1.3, `HandshakeHash::take_handshake_buf`, client-auth/helper temporaries, non-KX handshake-flight send custody, session-store/control backing and retained key-schedule/cipher backing.

Those findings are evidence for subsequent exact bounded cells, not authority to edit them now. The worker must not infer whole-file ownership from this census. After completing this caller unit, it must recompute the first reachable unallocated boundary and either continue inside already-protected grants or return one exact `SHARED_LEASE_REQUIRED` before excluded mutation.

#493 PSK binder remains `NOT_ACTIVE_CONDITIONAL` and is not activated by this amendment. ECH transcript `rollup_for_hrr` remains outside this exact SQLx fresh-config caller cell unless separately proven necessary and allocated.

## Existing protected grants retained

This amendment does not replace or broaden already-protected allocations, including #425/#427/#501/#518/#535/#538/#542 and their exact symbol/path boundaries. Material already completed on #356, including outbound queue/dequeue custody and TLS1.2 ClientKeyExchange fallible propagation, must not be redone.

## Explicit exclusions

No authority is granted for:

- any other `client/hs.rs` symbol;
- `hash_hs.rs` symbols beyond already-protected #425/#501 authority;
- broad TLS1.2/TLS1.3 handler conversion;
- #493 PSK binder activation;
- ECH-specific transcript/accounting;
- crypto-provider source or dependency changes;
- `HandshakeHash::take_handshake_buf`;
- generic `HandshakeFlight` / non-KX outbound custody;
- session-cache/control backing changes;
- retained key-schedule/cipher backing changes;
- SQLx/PostgreSQL source beyond existing #351 authority;
- Cargo/lock/workspace changes beyond already-protected active leases;
- WP4/#329/#335, WP5, Server Seam/#247, Movement, Platform, Atlas or META mutation;
- production, PKI, credentials, secrets, protected-environment or live-data mutation;
- workflow, branch protection, required-status or Merge Queue weakening.

## Control-plane validation and integration lifecycle

This is a HIGH-risk docs-only allocation candidate. Runtime E2E is `NOT_APPLICABLE` for this control-plane document because it changes no executable code.

Before it can grant material authority:

1. confirm the effective PR diff is exactly this one new documentation file;
2. run/obtain the repository-selected governance validation and all exact-head required PR checks;
3. obtain one genuinely independent exact-head HIGH-risk review with no unresolved actionable P0/P1/P2 finding and zero unresolved review threads/requested changes;
4. confirm the current task retains explicit human-owner authorization to integrate this bounded control-plane change under active ADR 0005; do not recreate the superseded comment-ledger/fingerprint/attestation machinery;
5. immediately before submission, re-read repository, PR, `base=main`, exact head, authorization and eligibility;
6. integrate only through the bound META 3.1 native exact-head Merge Queue route: REST `merge-async` with the exact qualified `sha` and explicit `merge_action="merge_queue"`;
7. treat HTTP 202 as admission only and perform the required causal receipt/readback checks;
8. require a real `merge_group` aggregate `game-gate` SUCCESS;
9. re-read protected `main` and prove this exact amendment is present;
10. fresh-read #356 current head/source/overlap and confirm the blocker remains necessary;
11. post one explicit Work activation for this amendment to the SAME #351/#356 worker and only then allow material mutation.

Direct merge, generic auto-merge, bypass, force/rebase/reset, default merge action, no-op/retrigger commits and protection weakening are forbidden substitutes. If the repository-native `merge-async` capability is unavailable to the active execution surface, preserve the qualified candidate and report `BLOCKED_CAPABILITY_UNAVAILABLE`; do not substitute another merge primitive.

## Resume contract

After protected activation, resume only `agent/sqlx-driver-budget-351` / Draft PR #356 from the then-current exact head. Do not create a replacement worker, branch or material PR. Complete only this exact caller unit first, publish one coherent checkpoint, run focused validation, and recompute the next legal WP3 cell from fresh source.

This amendment alone does not prove complete TLS accounting, PostgreSQL 17.6 qualification, WP3 completion, WP4 readiness, G0 readiness or Server Seam readiness.

`MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`
`MATERIAL_ACTIVATION: SAME_WP3_WORKER_ONLY_AFTER_PROTECTED_READBACK`
