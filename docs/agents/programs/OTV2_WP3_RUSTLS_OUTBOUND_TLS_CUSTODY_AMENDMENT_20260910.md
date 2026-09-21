# WP3 rustls outbound TLS queue custody conditional amendment

Coordinator: #162. Programme: #364. Existing sole material worker: #351 / Draft PR #356. Parent client-handshake amendment: #518.

## State

```yaml
allocation_id: OTV2-WP3-RUSTLS-OUTBOUND-TLS-CUSTODY-20260910
repository: Oteryn/Oteryn-Game
allocation_base_main_sha: 49560d344939488ec01f4fea41b04272a97e10f2
allocation_state: NOT_ACTIVE_CONDITIONAL
preparation_branch: coord/wp3-rustls-outbound-custody-351
worker_branch: agent/sqlx-driver-budget-351
source_wp3_head: 933ccef1d37b2f0f31b3d88dd3576b5d804b284a
source_common_state_blob: 6a9a85cb4cead0c80bf9e13e6056908cad5ef0fb
source_vecbuf_blob: c050a3018c75941cfe0341159f977305b3bbce37
source_evidence_comment: 5618687023
risk: HIGH
material_worker_status_at_allocation: STOPPED_SHARED_LEASE_REQUIRED
```

This is one minimal allocation-only control-plane amendment for the outbound TLS custody boundary proven while exhausting active #518. It is `NOT_ACTIVE_CONDITIONAL`: it creates no replacement worker, branch or PR and grants no present rustls, SQLx, PostgreSQL, runtime, deployment or integration authority. It does not resume #356.

## Fresh live and source evidence

Immediately before preparation, GitHub LIVE and exact-source readback established:

- protected `main` = `49560d344939488ec01f4fea41b04272a97e10f2`;
- canonical material branch = `agent/sqlx-driver-budget-351@933ccef1d37b2f0f31b3d88dd3576b5d804b284a`;
- Draft PR #356 remains the sole canonical material lineage; the branch is unchanged by the failed GitHub-Mention retries;
- local isolated same-lineage execution on the merged-up source completed read-only inspection with no tracked-file mutation and returned the exact boundary recorded in comment `5618687023`;
- no new numeric budget, TLS policy or downstream scope is requested.
### Exact boundary

At source head `933ccef...`, `CommonState::queue_tls_message` encodes the final `OutboundOpaqueMessage` and appends the resulting `Vec<u8>` to `sendable_tls`. `ChunkVecBuffer::append` moves that vector into its internal `VecDeque`; later `consume`/`write_to` remove a chunk only after bytes are actually written. The final backing can therefore survive the `emit_client_kx` stack frame and a partial or `WouldBlock` write.

Active #518 can pre-reserve the TLS1.2 KX source/destination and outbound allowance in `emit_client_kx`, but its caller-local token cannot be released at the actual queued backing destruction without a queue-level transfer. Releasing at function return is premature; retaining to connection drop is stale overcharge after earlier transmission.

The minimum lawful bridge is an optional owner/custody handoff through the existing TLS-send path plus per-retained-chunk custody in `ChunkVecBuffer`. Ordinary unowned traffic must remain unchanged.

## Exact future material authority

Only after protected integration, protected-main readback, fresh same-lineage custody verification and explicit Work activation for the SAME #351/#356 worker, this amendment MAY authorize exactly:

### `vendor/rustls-0.23.43/src/common_state.rs`

- `CommonState::send_msg` only as required to carry an already-reserved outbound TLS custody token from the authorized TLS1.2 KX caller;
- `CommonState::send_msg_encrypt` and `CommonState::send_single_fragment` only where required to preserve that token across fragmentation/encryption;
- `CommonState::queue_tls_message` only to bind the already-reserved token to the final encoded queued backing;
- minimum private owner-aware handoff/helper state inseparable from those exact calls.

No general application-data accounting, new buffering policy, key-update redesign, record-layer policy or unrelated `CommonState` mutation is granted.

### `vendor/rustls-0.23.43/src/vecbuf.rs`

- `ChunkVecBuffer` private representation only for optional per-retained-chunk custody paired with a queued `Vec<u8>` and for exact custody of queue/control backing allocated by the owner-aware outbound path;
- `append`, `pop`, `consume`, `read`/`read_buf`, and `write_to` only as strictly necessary to transfer/release chunk custody at the same point the corresponding backing is moved or destroyed;
- minimum private reservation/growth helper needed to reserve `VecDeque` queue/control backing before owner-aware insertion can allocate or grow it, accounting actual element layout/capacity and old/new overlap;
- minimum private Drop/replacement/shrink plumbing required to retain the queue/control debit while its capacity remains allocated and to release it only after that backing is actually destroyed.

No new public API, generic allocator interception, changed buffer limit, changed read/write ordering or unrelated `ChunkVecBuffer` behavior is granted.
## Required semantics

Any activated implementation must:

- reuse the SAME accepted #351/#356 operation owner and ledger; no second allowance, global owner, thread-local budget or new resource maximum;
- reserve in the already-authorized caller before every resulting allocation/growth covered by that reservation; this amendment is custody plumbing, not permission for post-allocation catch-up;
- verify resulting actual capacity/backing is within the pre-reserved source-derived bound before releasing any unused over-reservation;
- preserve all simultaneous source, typed-message, encoded plaintext, encrypted-record and queued-send backing until each is actually destroyed or custody is explicitly transferred;
- attach custody to the exact final `Vec<u8>` retained in `sendable_tls` and release it only after the corresponding backing is popped/destroyed, including partial writes, `WouldBlock`, error, cancellation and connection drop;
- reserve queue/control backing before any owner-aware `VecDeque` allocation/growth, retain that separate debit after the queue becomes logically empty if capacity is still allocated, and release it only on actual replacement/shrink/drop of that backing;
- preserve old/new queue-control capacity overlap during any replacement growth and fail closed before allocation if the same ledger cannot fund it;
- avoid double-charge when ownership moves between TLS1.2 KX, encryption and queue stages;
- preserve TLS1.2 wire bytes, cipher/record semantics, ordering, alerts, randomness, transcript behavior and security;
- preserve ordinary owner-free std/no_std behavior and existing application-data behavior;
- preserve the existing `DEFAULT_BUFFER_LIMIT`; it is not an accounting proof and must not be changed or repurposed as a hidden resource allowance;
- add no arbitrary cap, copied allocation schedule, whole-handshake magic reserve, plaintext fallback, unowned retry, or bypass after denial.

If exact implementation proves that an additional path/symbol must allocate or retain covered backing before the authorized handoff can exist, stop before mutation and return a new exact `SHARED_LEASE_REQUIRED` rather than widening by proximity.

## Focused future qualification

Before this seam can be treated as proven, the SAME #356 lineage must provide executable exact-graph evidence for:

- exact source-derived funded success for both ECDHE and DHE ClientKeyExchange paths;
- max-minus-one denial before the first covered destination allocation/state admission;
- final encoded/encrypted queue backing charged on the same owner without reserve-again;
- partial write and `WouldBlock` retaining charge while the chunk remains live;
- full write/pop releasing only after backing destruction;
- connection error/cancellation/drop releasing all still-retained outbound custody exactly once;
- multiple queued chunks preserving one-to-one backing/custody association and destruction order;
- repeated enqueue/drain churn proving `VecDeque` queue/control capacity is either still charged while retained or released only after actual deallocation, with growth denial before allocation and old/new overlap accounted;
- ordinary unowned `ChunkVecBuffer` behavior and existing buffer limits unchanged;
- Rust 1.94 focused rustls tests/check/Clippy and the affected SQLx integration control needed to prove the same owner reaches the queue.
Focused proof does not substitute for #501 transcript/hash completion, the still-gated #493 binder condition, complete TLS composition, real funded SQLx AWS-LC TLS-positive evidence, PostgreSQL 17.6 qualification, final review, canonical CI, Merge Queue or protected-main delivery.

## Explicit exclusions

This amendment grants no authority for:

- broad `common_state.rs`, `vecbuf.rs`, `message.rs`, record-layer, fragmenter or generic codec changes;
- application-data buffering/accounting redesign or changes to `received_plaintext`;
- TLS1.2 KX semantics outside already-active #518 `emit_client_kx` and its exact private handoff;
- TLS1.3 semantic changes, key-update policy, certificate/hostname/signature verification, crypto-provider selection or cipher changes;
- #493 binder paths, which remain governed by their separate conditional activation;
- #501 transcript/hash paths beyond their separately protected activated scope;
- Cargo/lock/version/feature changes;
- SQLx/PostgreSQL source, SQL/migrations, B/#329/#335, WP4, WP5, Server Seam/#247, Platform, Atlas, META, production, secrets, PKI or live data;
- direct merge, generic auto-merge, protection weakening, force/rebase/reset or no-op/retrigger commits.

All earlier protected grants remain unchanged within their exact boundaries.

## Integration and activation gates

This HIGH-risk control-plane candidate cannot authorize or integrate itself. It remains `NOT_ACTIVE_CONDITIONAL` until:

1. the exact one-document candidate diff/head is frozen and producer self-review reports no P0/P1/P2;
2. all applicable deterministic exact-head repository checks are terminal GREEN;
3. a genuinely independent exact-head HIGH-risk/deep review reports P0/P1/P2=0 and required review threads/requested-changes are clear;
4. the human owner explicitly authorizes the exact reviewed candidate head for protected integration;
5. fresh target-bound preflight verifies repository, PR, `base=main`, exact head, authorization and eligibility;
6. integration uses only the bound META 3.1 native exact-head REST `merge-async` route with exact `sha` and `merge_action="merge_queue"`;
7. accepted submission is reconciled by UUID and strictly later executor sequence, then real `merge_group` aggregate `game-gate` must succeed;
8. protected `main` readback proves this exact amendment is integrated;
9. Work freshly verifies #351/#356 ownership/head and explicitly activates this amendment for the SAME canonical worker.

If the native exact-head Merge Queue operation is unavailable, preserve the candidate and return `BLOCKED_CAPABILITY_UNAVAILABLE`; no alternate merge primitive is authorized.
## Source anchors

At canonical source head `933ccef...`:

- `common_state.rs:440-442` is `CommonState::queue_tls_message`, where `m.encode()` creates the final vector and `sendable_tls.append(...)` retains it;
- `common_state.rs:452+` is the existing `CommonState::send_msg` handoff used by the TLS1.2 caller;
- `vecbuf.rs:16-27` defines `ChunkVecBuffer` and its retained `VecDeque<Vec<u8>>`;
- `vecbuf.rs:75-87` moves a vector into that queue;
- `vecbuf.rs:92-102` pops a retained chunk;
- `vecbuf.rs:172-191` destroys fully consumed front chunks;
- `vecbuf.rs:194+` performs vectored writes and calls the same destruction path after actual bytes are written.

These anchors are evidence for minimum custody placement, not permission to alter neighboring symbols.

## Lifecycle

```text
allocation-only NOT_ACTIVE_CONDITIONAL candidate
-> one-document exact diff + producer self-review
-> deterministic exact-head repository checks
-> independent HIGH-risk exact-head review P0/P1/P2=0
-> explicit human-owner authorization of the exact reviewed head
-> fresh target/head/eligibility preflight
-> META 3.1 native exact-head merge-async merge_queue
-> same-UUID later-sequence reconciliation
-> real merge_group game-gate SUCCESS
-> protected-main readback
-> explicit Work activation for SAME #351/#356 lineage
-> focused #518 outbound custody implementation
-> continue remaining legal WP3 cells
```

Protection of this document alone does not prove TLS, SQLx, PostgreSQL, WP3, WP4, WP5, G0 or Server Seam readiness.

`MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`
