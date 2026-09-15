# WP3 TLS1.3 normal final-flight custody amendment

Coordinator: #162. Programme: #364. Existing sole material worker: #351 / Draft PR #356.

## State

```yaml
allocation_id: OTV2-WP3-RUSTLS-TLS13-NORMAL-FINAL-FLIGHT-CUSTODY-20260911
repository: Oteryn/Oteryn-Game
allocation_base_main_sha: 663bd35a5196a925fc6eb0318381ad0b97f4cc2c
allocation_state: NOT_ACTIVE
preparation_branch: coord/wp3-rustls-tls13-final-flight-351-20260911
worker_branch: agent/sqlx-driver-budget-351
source_wp3_head: ed7b540b6ccd432168ef4859c5d7048b7fa6d260
risk: HIGH
```

This is an allocation-only control-plane amendment. It grants no present runtime mutation authority, does not resume #356, and creates no replacement worker, material branch or material PR. Material use is permitted only after independent exact-head review, canonical exact-head checks, normal FULL Merge Queue, protected-main readback, fresh overlap/custody reconciliation and explicit application to the SAME #351/#356 worker.

## Verified next boundary

Protected #559 grants only `client/tls13.rs::ExpectCertificateRequest::handle` and `ExpectFinished::handle`. The SAME worker has now legally materialized that caller unit at canonical #356 head `ed7b540b6ccd432168ef4859c5d7048b7fa6d260`: std paths use `try_add_message` / `try_current_hash`, while owner-free/no_std calls remain unchanged. Fresh exact-source readback on that successor shows the next normal TLS1.3 full-handshake path necessarily crosses an unallocated outbound-flight boundary before it can finish truthfully.

At canonical #356 head `ed7b540b6ccd432168ef4859c5d7048b7fa6d260`:

- `common_state.rs` blob `eccccc47f5807823eca00667f25540ed5655d9f1` is unchanged and defines `HandshakeFlight` with an uncharged `Vec<u8>` body; `HandshakeFlight::add` performs `hs.encode(&mut self.body)` and then calls ordinary `HandshakeHash::add`;
- `hash_hs.rs` blob `46c63263bd6ea43530593e34a843efc41a47d84d` is unchanged; ordinary `HandshakeHash::add/add_raw` rejects owner-aware use and protected fallible transcript primitives exist, but no crate-visible operation-owner accessor exists for flight-body reservation;
- `client/tls13.rs` blob is now `5f0ac4c46f7928c0980fec81557c83d1559c04cc`; the #559 caller changes are present, while `ExpectFinished::handle` still unconditionally reaches ordinary `emit_finished_tls13 -> flight.add` and `flight.finish` on the normal no-client-auth/no-early-data path;
- `record_layer.rs` blob `8c2ea6b9b6849a444e055332d4e735cff91e7980` remains unchanged and already exposes `RecordLayer::encrypted_len(payload_len)`, so no record-layer authored change is required to pre-reserve encrypted record backing;
- `vecbuf.rs` blob `75b0834be851794766a1a1407d3e89ba2e0ea2f9` remains unchanged and already contains protected #535 `OutboundTlsCustody`, `RetainedChunk`, queue-control pre-reservation and retained-chunk custody primitives.

Protected #425 does not grant `HandshakeFlight::{new,add,finish}`. Protected #550 explicitly excluded generic `HandshakeFlight` / non-KX outbound custody. Protected #535 materialized queue-custody machinery for its TLS1.2 KX seam and explicitly did not grant TLS1.3 semantic changes. Fresh open-PR searches found no competing allocation for `HandshakeFlight` or `emit_finished_tls13`.

Therefore the normal SQLx TLS1.3 positive path still cannot truthfully complete after the legal #559 caller commit: owner-aware transcript append would next hit ordinary `HandshakeHash::add`, the final-flight body can allocate/grow before reservation, and its encrypted queued record backing must be reserved while the source flight body remains live.

## Exact prospective material authority

Only after protected application, this amendment may add the following exact, bounded semantics.

### `vendor/rustls-0.23.43/src/hash_hs.rs`

- one minimum crate-private read-only operation-owner accessor on `HandshakeHash`, used only to obtain the SAME already-installed `DeframerBufferOwner` for outbound final-flight reservation;
- no change to transcript bytes, hashing, provider sizing, context allocation, HRR, clone or binder semantics;
- existing #425/#501 fallible transcript methods remain authoritative and are reused rather than duplicated.

No other `hash_hs.rs` authored symbol is granted.

### `vendor/rustls-0.23.43/src/common_state.rs`

`HandshakeFlight` only:

- private representation only as required to retain optional SAME-owner custody for its `body: Vec<u8>`;
- `HandshakeFlight::new` only to initialize that private optional custody without changing owner-free behavior;
- one new/private fallible exact-length append operation for owner-aware use; it must reserve prospective replacement capacity before allocation, preserve old+new overlap, encode into exact-capacity backing, verify encoded-length/capacity invariants, append the exact newly encoded bytes through the already-protected fallible transcript operation, and commit the replacement only after both body and transcript operations succeed;
- one new/private owner-aware TLS1.3 TCP finalization operation that keeps source flight-body custody live while it pre-reserves queue-control and encrypted-record backing, preserves current pre-encrypt/sequence behavior, then encrypts/fragments and transfers each retained record to the already-materialized #535 queue custody primitives;
- minimum private helper state inseparable from those exact operations.

The existing ordinary `HandshakeFlight::add` and `finish` remain behaviorally unchanged for owner-free/no_std and unrelated callers. No broad `CommonState::send_msg`, application-data, key-update or generic send redesign is granted.

The owner-aware finalization may use existing read-only `RecordLayer::encrypted_len`, existing fragmentation, `DirectDecodedCustody`/`OutboundTlsCustody`, `ChunkVecBuffer::prepare_owned_append`, `RetainedChunk::with_custody` and `append_with_custody` without modifying those existing symbols. Any current key-update queue interaction must preserve existing ordering or fail closed before owner-aware final-flight allocation; this amendment does not authorize changing key-update semantics.

### `vendor/rustls-0.23.43/src/client/tls13.rs`

- `emit_finished_tls13` only, to become fallible on the owner-aware path and pass a checked source-derived exact Finished handshake encoding length into the new `HandshakeFlight` operation;
- the already-materialized #559 `ExpectFinished::handle` may propagate that failure and call the new owner-aware finalization operation; #559 remains the authority for edits inside that caller.

No other TLS1.3 helper is granted here.

## Normal-path scope fence

This amendment intentionally proves only the normal TCP TLS1.3 final flight with:

```text
early_traffic = false
client_auth = None
ECH behavior unchanged
TCP transport
```

It does not silently widen into optional paths. If current execution requires any of the following, stop before mutation and return a new exact boundary:

- `emit_end_of_early_data_tls13` / 0-RTT EndOfEarlyData;
- `emit_certificate_tls13`;
- `emit_compressed_certificate_tls13`;
- `emit_certverify_tls13`;
- client-auth signer/certificate/compression output custody;
- QUIC handshake queue custody;
- PSK binder/resumption #493 seams;
- any record-layer implementation change;
- any generic `Codec`, `MessagePayload`, `Message -> PlainMessage`, fragmenter or crypto-provider mutation.

#493 remains `NOT_ACTIVE_CONDITIONAL`.

## Required semantics

Any activated implementation must:

- reuse the SAME accepted #351/#356 operation owner/ledger; no new owner plane or allowance;
- reserve before every owner-aware final-flight body allocation/growth;
- account actual `Vec<u8>` capacity, not logical length or requested delta;
- retain old and prospective-new flight-body charges simultaneously during replacement growth until old backing is actually destroyed;
- on append failure, destroy prospective backing before releasing its reservation and leave prior flight body/transcript state coherent;
- use an exact source-derived Finished encoded length and fail closed if encoding produces a different length or capacity than qualified;
- keep plaintext flight-body charge live while encrypted record backing is allocated, because source and destination coexist during encryption;
- use checked arithmetic for fragment count, TLS header and `RecordLayer::encrypted_len` composition and verify resulting record capacity against the pre-reserved amount before custody transfer;
- reserve send-queue control backing before encrypted record allocation and reuse existing #535 per-record retained custody without reserve-again;
- preserve `next_pre_encrypt_action` / sequence advancement ordering and reject before encryption if the current state cannot be represented without an excluded helper change;
- release source flight-body charge only after its backing is actually destroyed; release each encrypted-record charge only after its retained queue chunk is actually destroyed/drained; preserve partial-write/WouldBlock/error/drop behavior already owned by #535/#542;
- preserve record bytes, encryption, TLS versions, verification, key schedule, cache/session, PQ/provider, randomness and all owner-free/no_std behavior;
- introduce no arbitrary cap, magic whole-handshake reserve, post-allocation catch-up, plaintext/unowned fallback, dependency/version change or semantic shrink.

## Focused qualification

Before this unit may be considered proven on #356, the final candidate must provide executable evidence for at least:

1. funded normal owner-aware TLS1.3 final flight through server Finished -> client Finished -> traffic transition;
2. max-minus-one/body-growth denial before final-flight backing allocation;
3. transcript-growth denial after flight-body reservation, with prospective body backing destroyed/released and no dependent key-schedule/send transition;
4. exact source flight-body + encrypted-record coexistence on the SAME owner during encryption;
5. encrypted-record max-minus-one denial before encryption output allocation, with allocated record capacity matching/contained by its pre-reservation;
6. queue/control growth denial before encrypted record allocation;
7. current pre-encrypt/key-update ordering unchanged on the supported normal path;
8. partial write/WouldBlock retains record custody until backing destruction and full drain releases exactly once via existing #535/#542 semantics;
9. connection error/cancellation/drop releases all still-live final-flight/queued custody exactly once;
10. ordinary owner-free/no_std final-flight behavior unchanged;
11. regressions for protected #501/#535/#542/#559 normal TLS1.3/TLS queue behavior.

Compile-only, plaintext PostgreSQL, rustls-only handshake on a different dependency graph, skipped tests or old-head CI do not prove the final SQLx TLS-positive gate.

## Explicit exclusions

No new authority is granted for:

- broad `common_state.rs`, `hash_hs.rs`, `client/tls13.rs`, `vecbuf.rs`, `record_layer.rs` or message/codec files;
- early data, client auth, ECH expansion or QUIC;
- TLS1.2 semantics or the existing #535 KX caller;
- verifier/certificate-verifier, key-schedule or crypto-provider implementation;
- session store/cache or PSK binder #493;
- SQLx/PostgreSQL/Game/Foundation runtime expansion, Cargo/lock/version/features, workflows, registry, production, deployment, PKI, secrets, credentials or protection;
- WP4/WP5/G0/Server Seam activation.

All earlier protected grants retain their exact authority and are not duplicated.

## Lifecycle

```text
this docs-only NOT_ACTIVE amendment
-> independent exact-head HIGH-risk review
-> canonical exact-head checks
-> normal FULL Merge Queue
-> real merge_group game-gate SUCCESS
-> protected-main readback
-> fresh #356 head/source/overlap reconciliation
-> explicit ACTIVE application to SAME #351/#356 worker only
-> normal final-flight RED/GREEN from the already-materialized #559 caller state
-> fresh caller/custody rescan
-> remaining complete TLS composition
-> real funded SQLx AWS-LC TLS-positive
-> configured PostgreSQL 17.6 positive + hostile/denial qualification
-> final independent whole-diff review + exact-head CI
-> FULL Merge Queue + protected WP3 readback
```

Queue admission or protection of this document alone does not prove TLS, PostgreSQL, WP3, WP4, WP5, G0 or Server Seam readiness.

`MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`
