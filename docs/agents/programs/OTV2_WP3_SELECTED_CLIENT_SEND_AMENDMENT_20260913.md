# WP3 selected client-handshake send-purpose amendment

```yaml
allocation_id: OTV2-WP3-SELECTED-CLIENT-SEND-20260913
allocation_state: NOT_ACTIVE
decision_state: CANDIDATE_NOT_ACCEPTED
repository: Oteryn/Oteryn-Game
allocation_base_main_sha: 448309c4042e3fb783d1ab9d7bb324f7ee977420
source_wp3_head: f10cab5b7d1d388fb20acf0beec4a5a62aef49fc
coordinator: 162
issue: 351
pr: 356
worker_branch: agent/sqlx-driver-budget-351
risk: HIGH
```

This prospective allocation implements only the companion
`docs/architecture/reviews/OTERYN_GAME_WP3_SELECTED_CLIENT_SEND_DECISION_2026-09-13.md`.
It preserves frozen inventory #162/5655890678 and existing grants mapped by
#162/5656338852. It creates no replacement material branch, PR, owner or ledger.

## Exact additional authored scope after protected activation

### vendor/rustls-0.23.43/src/common_state.rs

- One narrow fallible owner-aware send helper and checked source-derived
  length/reservation helper for selected TCP TLS1.3 initial/HRR ClientHello,
  compatibility CCS and fatal alerts only.
- Minimum private optional same-owner storage/installation for those alert sends,
  initialized without changing ordinary behavior or minting an owner.
- CommonState::send_fatal_alert only to route selected owned alerts through the
  bridge. Preserve primary failure and fail closed if alert funding is denied;
  do not allocate a replacement diagnostic or fall back to unowned output.
- Reuse existing fragmentation, record encrypted-length inspection,
  OutboundTlsCustody, prepare_owned_append, RetainedChunk and queue/dequeue
  custody without expanding their authored scope.

No generic send_msg/Codec/fragmenter/record-layer rewrite, application-data,
key-update or QUIC authority is granted.

### vendor/rustls-0.23.43/src/client/tls13.rs

- One fallible owner-aware sibling of emit_fake_ccs, preserving the existing
  one-CCS flag and bytes; ordinary emit_fake_ccs behavior stays unchanged.
- handle_server_hello only for the owned CCS call and funding-failure propagation
  before dependent progression. Existing transcript authority stays separate.

No derive_early_traffic_secret, generic key schedule, verification/provider or
other TLS1.3 helper authority is added by this amendment.

Existing client/hs.rs::emit_client_hello_for_retry authority supplies caller
precharge and the bridge call. Existing #429 early owner installation supplies
same-owner plumbing; neither is a new owner or a blanket file lease.

## Required implementation and qualification

Reserve complete typed/encoded/plaintext/fragment/record/queue-control overlap
before controlled allocation. Use checked arithmetic and actual requested
capacities. Retain old backing during replacement; transfer each output record
to existing per-chunk custody without reserve-again. Release at actual backing
destruction, including partial writes, WouldBlock, cancellation and drop. Keep
queue-control capacity funded through its own lifetime. Preserve pre-encrypt and
sequence advancement ordering and primary error semantics.

Qualify initial and HRR wire equivalence, one CCS, certificate/signature failure
alerts, funded and one-byte-short denial before encoding/encryption/queue growth,
retained output and finality, ordinary owner-free and no_std behavior. Retain
existing final-flight/provider component proofs. No new arbitrary allowance,
post-allocation catch-up, secret/provider change or production activation.

The companion decision explains why fixed states/selected keys/verifier scratch
can be implemented under admitted caller/enclosing reservation scope. Their
numeric bounds remain unproven until implementation qualification. No selected
ALPN representation change is required. Core sqlx-core/src/error.rs remains
separate #603; this package cannot activate it.

## Activation gate

Require independent exact-head HIGH-risk review, required repository CI,
currently authorized native META3.1 exact-head Merge Queue, real merge_group
game-gate success and protected-main readback. Then require decision acceptance,
fresh overlap/custody reconciliation and explicit #162 activation for the SAME
#351/#356 worker. An unmerged proposal is never a shared lease.

Until activation, execute substantial admitted M03 caller/config/state/key and
focused harness work when frozen order permits. Stop only at an exact missing
symbol boundary; do not hold that legal work on this amendment's integration.
