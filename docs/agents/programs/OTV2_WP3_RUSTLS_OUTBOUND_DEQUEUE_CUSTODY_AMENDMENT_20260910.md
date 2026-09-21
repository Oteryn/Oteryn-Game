# WP3 rustls outbound dequeue custody conditional amendment

Coordinator: #162. Programme: #364. Existing sole material worker: #351 / Draft PR #356. Parent protected amendments: #535 and #538.

## State

```yaml
allocation_id: OTV2-WP3-RUSTLS-OUTBOUND-DEQUEUE-CUSTODY-20260910
repository: Oteryn/Oteryn-Game
allocation_base_main_sha: 72c997b3f71c180437f6b40e2c5e2abbe0f45273
allocation_state: NOT_ACTIVE_CONDITIONAL
preparation_branch: coord/wp3-rustls-outbound-dequeue-351
worker_branch: agent/sqlx-driver-budget-351
source_wp3_head: d90950237e1da5b8e0710f746417b2a603b7337e
source_common_state_blob: 6a9a85cb4cead0c80bf9e13e6056908cad5ef0fb
source_unbuffered_blob: 77927b5587ff46511e288dd33dad07d0506194b9
source_vecbuf_blob: c050a3018c75941cfe0341159f977305b3bbce37
risk: HIGH
material_worker_status_at_allocation: STOPPED_SHARED_LEASE_REQUIRED
```

This is one allocation-only control-plane amendment for the downstream lifetime boundary discovered after protected #535/#538 activation. It creates no replacement worker, branch or material PR and grants no present runtime authority. It does not resume #356 until protected integration/readback and explicit Work activation.
## Exact source boundary

Protected #535 authorizes custody while owner-aware encrypted chunks remain in `CommonState::sendable_tls` / `ChunkVecBuffer`. Source census proves two routes can remove such a chunk while its `Vec<u8>` backing remains alive:

- `CommonState::write_fragments` pops a queued encrypted chunk, copies it to a caller-owned output slice, and destroys the source vector only after the copy completes;
- `UnbufferedConnectionCommon::process_tls_records_common` pops a queued encrypted chunk and transfers it into `EncodeTlsData`, which can retain it across an insufficient-output-buffer retry.

Releasing chunk custody at `ChunkVecBuffer::pop` is therefore premature. Keeping the charge only inside the queue after pop would instead lose a destruction-bound owner for the moved backing.

Buffered `ConnectionCommon::write_tls` does not require this amendment: it uses already-authorized `ChunkVecBuffer::write_to`, where failed/WouldBlock writes do not consume the chunk and successful consumption destroys fully consumed backing in the queue.

`sendable_plaintext` and `received_plaintext` pop sites do not carry the outbound TLS custody described by #535 and are excluded.

## Exact future material authority

Only after protected integration/readback and explicit Work activation for the SAME #351/#356 worker, this amendment MAY authorize exactly the following additional symbols.
### `vendor/rustls-0.23.43/src/common_state.rs`

- `CommonState::write_fragments` only at the existing `sendable_tls` dequeue/copy loop, solely to receive the already-charged retained-chunk custody from `ChunkVecBuffer`, keep it through copying, and release after the popped source backing is actually destroyed.
- No authority is granted to the subsequent fragment encryption loop, sizing policy, application-data behavior, plaintext queues, record-layer semantics or unrelated `CommonState` code.

### `vendor/rustls-0.23.43/src/conn/unbuffered.rs`

- `UnbufferedConnectionCommon<Data>::process_tls_records_common` only at the existing `sendable_tls.pop()` to `EncodeTlsData::new` transfer;
- `EncodeTlsData` private retained-chunk representation, `EncodeTlsData::new`, and `EncodeTlsData::encode` only to carry the already-reserved chunk custody with the exact `Vec<u8>` backing;
- minimum private destruction plumbing for `EncodeTlsData` only if required to ensure an unencoded/failed-encode adapter releases custody after its retained source backing is destroyed.

The public client/server `process_tls_records` wrappers, `TransmitTlsData`, application-data APIs and unrelated unbuffered states remain excluded.

### Existing #535 authority, unchanged

`ChunkVecBuffer` per-chunk and queue/control backing custody remains governed by protected #535. This amendment does not widen its allocation formula or add another ledger.
## Required semantics

Any activated implementation must:

- reuse the SAME accepted #351/#356 `DeframerBufferOwner` ledger and the same already-reserved outbound chunk custody; no re-reserve, second allowance or generic global registry;
- transfer custody allocation-free when the exact queued `Vec<u8>` moves out of `ChunkVecBuffer`;
- in `write_fragments`, retain custody until after the source vector has been copied and destroyed, then release exactly once;
- in the unbuffered path, retain custody across `process_tls_records_common` return and for as long as `EncodeTlsData::chunk` owns the exact source vector;
- on insufficient output space, restore both the same `Vec` and its same custody to `EncodeTlsData` for retry without release/re-reserve;
- on successful `EncodeTlsData::encode`, copy bytes, destroy the source vector, then release its custody before returning success;
- on dropping an unencoded or failed-encode adapter, preserve current drop semantics while destroying the retained vector before releasing its custody;
- keep queue/control backing charge independent from popped-chunk custody and retain that queue charge while allocated even when the logical queue is empty;
- preserve buffered partial/WouldBlock behavior, unbuffered output bytes, `AlreadyEncoded`, TLS wire/security/order, and ordinary owner-free std/no_std behavior.

If another path or symbol is required to retain or release the original outbound chunk backing, stop before mutation with exact `SHARED_LEASE_REQUIRED` rather than broadening by proximity.
## Focused future qualification

Before this downstream seam can be `PROVEN_FOCUSED`, the SAME #356 lineage must prove:

- buffered `write_fragments` dequeue retains the debit through source-copy lifetime and releases after destruction;
- unbuffered transfer preserves one-to-one owner identity and charge without reserve-again;
- insufficient output buffer retains both backing and charge across retry;
- successful retry and first-attempt success release only after source destruction;
- drop before encode and drop after an insufficient-size result release exactly once after backing destruction;
- multiple queued chunks preserve independent destruction order;
- ordinary unowned buffered/unbuffered output remains byte- and behavior-equivalent;
- queue/control backing remains separately charged across dequeue and logical-empty states;
- focused Rust 1.94 tests/check/Clippy/fmt pass on the pinned rustls graph.

This qualification does not prove complete TLS, #501/#493, SQLx TLS-positive, PostgreSQL 17.6 or WP3 completion.

## Explicit exclusions

No generic `Message`, codec, record-layer, fragmenter, server handshake, `received_plaintext`, `sendable_plaintext`, application-data accounting redesign, buffer-limit change, TLS policy/version/security change, SQLx/PostgreSQL, Cargo/lock, workflow, B/WP4/WP5/Server Seam, production or external-repository authority is granted.
## Integration and activation gates

This HIGH-risk control-plane candidate cannot authorize or integrate itself. It remains `NOT_ACTIVE_CONDITIONAL` until:

1. exact effective one-document diff/head is frozen and producer self-review has P0/P1/P2=0;
2. all applicable deterministic exact-head repository checks are terminal GREEN;
3. genuinely independent exact-head HIGH-risk review has P0/P1/P2=0 and review debt is clear;
4. current human-owner authorization is bound to the exact reviewed candidate within the already-authorized WP3 completion programme;
5. fresh target-bound preflight verifies repository, PR, `base=main`, exact head and eligibility;
6. integration uses only META 3.1 native REST `merge-async` with exact `sha` and `merge_action="merge_queue"`;
7. HTTP 202 receipt is bound to UUID plus later executor-sequence readback, and the real `merge_group` aggregate `game-gate` succeeds;
8. protected `main` readback proves the exact amendment integrated;
9. Work freshly verifies the SAME #351/#356 lineage and explicitly activates this amendment before material mutation.

Direct merge, generic auto-merge, bypass, force/rebase/reset, protection changes and no-op retriggers are forbidden. If native exact-head Merge Queue is unavailable, preserve the candidate and return `BLOCKED_CAPABILITY_UNAVAILABLE`.

After activation, this amendment is only a downstream lifetime bridge for the already-authorized #535/#538 outbound path. It grants no independent feature or worker.

`MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`