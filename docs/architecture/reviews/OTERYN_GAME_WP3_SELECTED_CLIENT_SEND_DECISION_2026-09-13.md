# WP3 selected TLS client send custody decision

- Status: CANDIDATE / NOT_ACCEPTED / NOT_ACTIVE
- Coordinator: #162; same implementation: #351 / #356
- Protected basis: `448309c4042e3fb783d1ab9d7bb324f7ee977420`
- Source inspection: `a83af56e6269e288aade597e093eee086cf3ddcb`; the relevant
  rustls/SQLx TLS blobs are unchanged in canonical `f10cab5b7d1d388fb20acf0beec4a5a62aef49fc`.
- Governing decision: accepted WP3 Revision 3, #162/5652184410.
- Frozen inventory: #162/5655890678; disposition: #162/5656633925.

## Problem and constraints

The selected TCP TLS1.3 client sends initial and HRR ClientHello messages through
an ordinary send/queue path. Existing ClientHello grants cover caller allocation;
existing queue grants cover TLS1.2 KX and the normal TLS1.3 final flight. Neither
grants a new initial-handshake, compatibility-CCS or fatal-alert send purpose.

At the inspected source, `client/hs.rs::emit_client_hello_for_retry` constructs
`MessagePayload::handshake` and invokes ordinary `CommonState::send_msg`.
`msgs/message/mod.rs` allocates the encoded payload and a subsequent PlainMessage
copy; `common_state.rs` fragments/encodes and retains queue backing. Caller-local
funding would end before queued records die. Retaining every message debit until
connection destruction would violate the existing per-chunk release contract.

The same selected path reaches `client/tls13.rs::emit_fake_ccs` through
`handle_server_hello`, and verification failure reaches `send_fatal_alert`.
Those outputs require the same prospective reservation and retained-queue
custody, including when the original failure leaves insufficient alert funding.

Preserve the accepted owner, ledger, wire bytes, crypto selection, sequence
ordering, primary failure, and owner-free/no_std behavior. This proposal does not
change Revision 3, choose new numeric limits, or prove the complete resource bound.

## Options, recommendation and trade-offs

Use one narrow fallible owner-aware send bridge for the three named purposes,
reusing the existing outbound custody, fragmenter, encrypted-length inspection
and queue/dequeue primitives. Existing admitted callers precharge typed and
encoded construction before invoking ordinary codecs. This avoids a generic
encoder, state, record-layer or provider rewrite and preserves release at actual
record destruction. The cost is exact source-derived overlap accounting and
fallible caller propagation, verified independently before activation.

A caller-only temporary debit cannot cover retained queued backing. A broad
generic send rewrite captures unrelated application-data/key-update/QUIC paths
without necessity and is rejected for this bounded amendment.

## Other known M03 boundaries

These scope dispositions retain explicit qualification obligations:

- Fixed successor-state Boxes can be funded by an enclosing existing SQLx TLS
  reservation covering old state, successor and concurrent ownership-conversion
  replacement. Hold it outside ClientConnection through actual destruction.
- Selected AWS-LC HKDF/record-key residency can use that admitted enclosing route,
  including temporary expanders and old/new encrypter/decrypter overlap. No
  key_schedule, record_layer or provider mutation is established as necessary.
- Admitted certificate-verification callers can reserve input-derived built-in
  verifier scratch before invocation. A separate enclosing reserve must cover
  saved/returned error clones and complete_io's boxed error. A handler-local
  guard is insufficient. At the private owned SQLx socket boundary, normalize
  opaque TLS io::Error inside custody, preserving IO kind and verification/alert
  outcome, destroy the allocating source, then return the bounded form. Detailed
  source payload is intentionally absent on that private owned boundary;
  ordinary owner-free source chains remain unchanged. Initial SQLx escaping
  TLS/Configuration errors remain the separate #603 boundary.
- Selected ALPN is not an additional allocation: SQLx offers no ALPN;
  validate_encrypted_extensions rejects unsolicited ALPN before
  process_alpn_protocol can copy it. Successful selected execution passes None.

These are permitted implementation approaches, not accepted numerical formulas
or terminal proofs. Each actual allocation, replacement and escape needs a
checked source bound and the final root equation. An observed insufficiency
requires a concrete successor disposition, not pre-emptive wider authority.

Current production D still uses ordinary URL pool construction. The helper's
selected-profile exclusions become production proof only after M05 wiring.
Retain ordinary compatibility; do not activate TLS1.2, resumption, compression,
nonempty client authentication, DNS/UDS or custom providers to close this scope.

## Risks and proof

Incorrect overlap/queue accounting could release funding early. Alert denial
could obscure the primary error; CCS retry could duplicate bytes or flags.
Require denial before allocation/encryption/queue growth, checked complete
lengths, source-plus-destination overlap, actual chunk destruction, partial
writes/cancellation, one-CCS semantics, and primary-failure preservation.
Independent wire-equivalence fixtures and ordinary/no_std controls are required.
No sampled peak or guessed whole-handshake allowance substitutes for source proof.

## Decision timing and future impact

Must decide now: YES, before the selected M03 send bridge is mutated. This is an
already-frozen scope boundary, not a new material cell. Caller/config/state/key
and harness work under existing grants can proceed independently.

Deferring the custody transfer choice would couple new callers to an unowned
queue path. The narrow bridge preserves later extension without promising any
generic send API. Reopen only for exact source evidence that current primitives
cannot preserve the named wire/finality invariants or a smaller safe route exists.
Application data, key update, QUIC, provider redesign, public error APIs, numeric
maxima and production topology are deliberately undecided here.

Independent HIGH-risk review, exact-head CI, authorized native Merge Queue,
real merge_group success, protected readback, decision acceptance and explicit
Work activation are required. This candidate supplies none of those grants.
