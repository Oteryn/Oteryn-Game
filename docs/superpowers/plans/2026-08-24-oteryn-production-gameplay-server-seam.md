# Production Gameplay Server Seam Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the smallest accepted production TCP/TLS gameplay entry seam that consumes canonical Foundation/Durability authority, remains fail-closed for unallocated gameplay, enforces registered resource limits, and proves real local production-path bootstrap/resume integration without claiming formal ADR-0007 QA Tier 1/Tier 2.

**Architecture:** Keep Foundation as the sole owner of protocol parsing/registered wire semantics and admission/reconnect authority. Add one `gameplay_transport` module whose TCP/TLS I/O layer performs bounded framing only, then hands validated envelopes to one connection state machine that invokes existing FND-04/GameSession/Durability consumers. Configuration and TLS material are caller-supplied; this plan does not choose production endpoint or secret topology.

**Tech Stack:** Rust 1.94, Tokio, current Foundation/Durability modules, SQLx-backed reconnect durability where already consumed, exact-pinned Rust TLS dependencies selected inside the allocated Cargo lease, repository-native tests and GitHub merge gates.

**Spec:** `docs/architecture/reviews/OTERYN_GAME_PRODUCTION_GAMEPLAY_SERVER_SEAM_PLAN_2026-08-24.md`

## Global Constraints

- Execute only after `OTV2-20260904-gameplay-server-seam-allocation` has merged and protected-main readback supplies the exact worker base SHA.
- Owned runtime paths are exactly those in `docs/agents/tasks/active/OTV2-20260904-gameplay-server-seam.md`; no implicit path expansion.
- Preserve FND-02 protocol major `1`, transport profile `1` and ALPN `oteryn-game/1`.
- Transport profile 1 is **TLS 1.3 only**. A client constrained to TLS 1.2, even with the exact ALPN, must be rejected during TLS negotiation before any Foundation frame handoff or admission work.
- Preserve the FND-02 wire-frame hard maximum of 1,048,576 bytes and every smaller message-specific bound in the current registry/code.
- Preserve registered Server Seam hard maxima: 256 pre-admission connections, 64 concurrent handshake/auth units, 64 outbound queue entries/session, 1,048,576 outbound queued bytes/session, 8 pending writes/session and 256 drain tasks/batch.
- No new protocol/event/state/capability/stable numeric ID, no Reference formula/value and no permanent Content-format decision.
- No second admission/GameSession/CharacterLease/reconnect/durability authority; transport is a consumer only.
- No production bind address/port/certificate/private-key/secret/deployment choice. Tests use loopback/ephemeral endpoints and non-shipping test TLS material.
- `workspace-boundaries.toml`, workflows, rulesets and architecture/contracts are not allocated.
- High-risk authority work follows `AuthorityInvariant × ConsumerBoundary × MutationOperator`, one invariant per negative case, current facts independent of immutable records, full finding-family sweep before freeze.
- Every implementation task follows fresh RED -> minimal GREEN. Do not weaken an assertion, fail-closed path, limit or authority boundary to make a test pass.
- Tasks 1-4 may use in-crate unit tests to exercise private implementation seams during sequential RED -> GREEN. Those tests are intermediate TDD evidence only and do **not** satisfy the physical Server Seam claim. Tasks 5-6 must project every applicable mandatory transport/authority/resource negative family through the actual production listener/composition path before final qualification.
- Do not make `gameplay_transport`, the Foundation bridge or any test hook externally public merely so `apps/game-server/tests/gameplay_server_seam.rs` can reach private implementation. If the production composition cannot be exercised through an already-canonical shipped surface without adding a new external public API/schema, stop with `ARCHITECTURE_ESCALATION_REQUIRED`.
- Real local production-path tests in this plan qualify the Server Seam boundary only. They MUST NOT be reported as formal ADR-0007 QA Tier 1/Tier 2; QA owns that later evidence after the seam is merged.

---

## File Structure

- `apps/game-server/src/foundation/protocol.rs` — Foundation-owned crate-internal typed extraction for already-registered bootstrap/resume messages and crate-internal outbound encoding for already-registered server Foundation messages.
- `apps/game-server/src/gameplay_transport/tcp_tls.rs` — TCP accept/TLS 1.3/ALPN/profile-1 handshake plus bounded big-endian-u32 frame I/O; no admission semantics.
- `apps/game-server/src/gameplay_transport/connection.rs` — one connection state machine that maps validated Foundation messages to existing FND-04/GameSession/Durability consumers and owns connection-local backpressure/generation state only.
- `apps/game-server/src/gameplay_transport/mod.rs` — registered privately in-crate before connection/lifecycle TDD; later extended with configuration/lifecycle composition, registered semaphores/budgets, listener/drain orchestration and the production start/stop composition.
- `apps/game-server/src/lib.rs` — register the private transport module early enough for in-crate TDD, then compose exactly one production transport path while preserving existing bootstrap/foundation regression modules.
- `apps/game-server/src/main.rs` — configuration-driven startup entry; preserve `--smoke`; no hard-coded production endpoint or TLS material.
- `apps/game-server/Cargo.toml`, `Cargo.toml`, `Cargo.lock` — only exact TLS/transport dependency/features needed by the seam; no unrelated dependency updates.
- `apps/game-server/tests/gameplay_server_seam.rs` — created only once the production composition entry exists; real loopback production-path TCP/TLS bootstrap/resume integration plus physical malformed/limit/authority/backpressure/shutdown evidence; not formal QA Tier evidence.

The Foundation bridge MUST stay `pub(crate)` or narrower unless an already-canonical consumer requires wider visibility. If a new externally public API/schema is required, stop before that mutation with `ARCHITECTURE_ESCALATION_REQUIRED`.

---

### Task 1: Foundation Typed Consumer Bridge

**Files:**
- Modify: `apps/game-server/src/foundation/protocol.rs`
- Test: existing `protocol.rs` in-crate unit-test module

**Interfaces:**
- Consumes: existing `WireEnvelopeView<'a>`, `MessageType`, `decode_wire_envelope`, existing protocol constants/identifier decoders and current FND-04 material formats.
- Produces: crate-internal typed views for validated `ClientBootstrap`/`ClientResume`; crate-internal encoders for already-registered `ServerAccepted`, `ServerResumeAccepted` and `ProtocolError`; no new wire IDs.

- [ ] **1.1 RED — typed extraction**

Add in-crate tests that decode canonical bootstrap/resume envelopes, call the new crate-internal typed extraction, and assert exact validated fields are available without reparsing raw bytes in transport code. Negative cases include wrong message type, missing required field, duplicate singular field, oversized material/build ID, invalid UUID/capability shape, wrong `protocol_major`, wrong `transport_profile` and bootstrap-phase generation misuse.

The intended API stays crate-internal, for example:

```rust
let envelope = decode_wire_envelope(&encoded_bootstrap)?;
let bootstrap = envelope.client_bootstrap()?;
assert_eq!(bootstrap.protocol_major(), PROTOCOL_MAJOR_V1);
assert_eq!(bootstrap.transport_profile(), TRANSPORT_PROFILE_TCP_TLS13_V1);
assert_eq!(bootstrap.admission_material(), expected_material.as_slice());
```

For resume:

```rust
let envelope = decode_wire_envelope(&encoded_resume)?;
let resume = envelope.client_resume()?;
assert_eq!(resume.game_session_id(), expected_session);
assert_eq!(resume.connection_generation(), expected_generation);
assert_eq!(resume.reconnect_material(), expected_material.as_slice());
```

- [ ] **1.2 Run focused RED**

```bash
cargo +1.94.0 test --locked -p oteryn-game-server foundation::protocol -- --nocapture
```

Expected FAIL because typed extraction does not exist yet.

- [ ] **1.3 Minimal typed extraction GREEN**

Parse fields once inside Foundation using the bounded helpers already used for validation. Return borrowed views where possible; do not copy untrusted payload merely to expose validated values. Do not export outside the game-server crate.

Equivalent shape:

```rust
pub(crate) struct ClientBootstrapView<'a> { /* validated borrowed fields */ }
pub(crate) struct ClientResumeView<'a> { /* validated borrowed fields */ }

impl<'a> WireEnvelopeView<'a> {
    pub(crate) fn client_bootstrap(&self) -> Result<ClientBootstrapView<'a>, FoundationProtocolError>;
    pub(crate) fn client_resume(&self) -> Result<ClientResumeView<'a>, FoundationProtocolError>;
}
```

- [ ] **1.4 RED — registered outbound encoders with independent oracle**

Add encoder tests for `ServerAccepted`, `ServerResumeAccepted` and `ProtocolError` that use **fixed canonical/golden byte vectors or another independent cross-oracle derived from the registered `foundation.proto` contract**. A same-implementation `encode -> decode_wire_envelope` round trip is useful secondary evidence but MUST NOT be the sole oracle.

For every family require:

1. exact encoded bytes equal the independent canonical vector;
2. decoding those canonical bytes yields the expected registered type/fields;
3. encoding the typed value yields those same canonical bytes;
4. illegal generation/server-sequence values and over-limit payloads fail;
5. no caller-selected generic numeric message-ID encoder exists.

Example secondary round trip:

```rust
let encoded = encode_server_accepted(&accepted)?;
assert_eq!(encoded, CANONICAL_SERVER_ACCEPTED_BYTES);
let decoded = decode_wire_envelope(CANONICAL_SERVER_ACCEPTED_BYTES)?;
assert_eq!(decoded.message_type(), MessageType::ServerAccepted);
```

- [ ] **1.5 GREEN — minimal registered-only encoders**

Implement only the three accepted server Foundation message families using existing protobuf/wire rules and checked lengths; no generic message-ID API. Run focused protocol tests and independent golden/cross-oracle cases to GREEN.

- [ ] **1.6 Commit Task 1**

```bash
git add apps/game-server/src/foundation/protocol.rs
git commit -m "feat(foundation): expose server seam protocol bridge"
```

---

### Task 2: Private In-Crate Bounded TCP/TLS Framing Boundary

**Files:**
- Create: `apps/game-server/src/gameplay_transport/mod.rs`
- Create: `apps/game-server/src/gameplay_transport/tcp_tls.rs`
- Modify: `apps/game-server/src/lib.rs`
- Modify: `apps/game-server/Cargo.toml`
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Test: `#[cfg(test)]` unit tests inside `gameplay_transport/tcp_tls.rs`

**Interfaces:**
- Consumes: Tokio async TCP/I/O; TLS implementation pinned under workspace policy; `ALPN_OTERYN_GAME_V1`; `MAX_WIRE_FRAME_BYTES`/current FND limits.
- Produces: private in-crate authenticated TLS stream wrapper plus bounded encoded Foundation frame read/write operations. No externally public transport API is introduced in this task.

- [ ] **2.1 RED — register only the private module shell and raw frame tests**

Create the minimum compile-time module topology needed to test the real source in-crate:

```text
lib.rs: private `mod gameplay_transport;`
gameplay_transport/mod.rs: private `mod tcp_tls;`
tcp_tls.rs: RED unit tests plus only the minimum declarations needed for the failing target
```

Do not compose startup/listener lifecycle yet. In `tcp_tls.rs` unit tests, on a real loopback transport reader require 0, 1,048,577, truncated prefix/body and checked accounting failures before peer-sized allocation; exact 1,048,576 is accepted when a complete body exists and message-specific validation permits it.

- [ ] **2.2 Run frame RED**

```bash
cargo +1.94.0 test --locked -p oteryn-game-server gameplay_transport::tcp_tls::tests::frame_ -- --nocapture
```

Expected FAIL because bounded gameplay transport behavior is not implemented yet. The RED must exercise the crate's real private `tcp_tls.rs`, not a `#[path]` copy in an integration-test crate.

- [ ] **2.3 Add only required TLS dependencies**

Inspect compatible Rust TLS crates at implementation time and add exact-pinned workspace dependencies/features required for Tokio TCP + TLS 1.3 + ALPN + non-shipping test certificate material. Do not absorb Dependabot #259/#260/#261 upgrades as convenience.

Before any Cargo edit, re-read active ownership. A new non-Dependabot Cargo writer is `SHARED_LEASE_REQUIRED`.

- [ ] **2.4 Minimal bounded TLS/frame GREEN**

```text
registered pre-admission slot held
-> TCP accept
-> bounded handshake/auth slot
-> TLS 1.3 only
-> exact ALPN oteryn-game/1
-> release handshake slot
-> read 4-byte BE length
-> validate 1..=1_048_576 before body allocation/read
-> read exact body
-> return one bounded frame to the caller
```

Write-side framing obeys the same hard maximum. This task owns transport framing only; Foundation decode, the connection state machine and production composition are integrated by later tasks. TLS/ALPN/framing failure closes before any admission boundary can be reached.

- [ ] **2.5 RED/GREEN — transport-level TLS version/ALPN/plaintext failures**

Using the real private in-crate loopback `tcp_tls.rs` boundary with non-shipping test TLS material, prove:

- TLS 1.3 + exact ALPN succeeds to one bounded frame handoff;
- a client explicitly constrained to TLS 1.2 **with the exact ALPN** fails TLS negotiation before any frame handoff; name this case `transport_tls12_rejected_before_frame_handoff` and configure the client so a TLS 1.3 retry/fallback cannot mask the result;
- missing/wrong ALPN fails before frame handoff;
- plaintext never yields an authenticated gameplay transport stream;
- malformed/truncated/oversized frames fail closed before peer-sized allocation or any higher-layer handoff.

Name these transport-only cases under a `transport_tls_` selector. Do **not** claim production-listener version/profile evidence here: `protocol_major` and `transport_profile` are Foundation-envelope semantics, and all physical profile/version negatives are re-exercised through the actual composed listener in Task 5.

Run:

```bash
cargo +1.94.0 test --locked -p oteryn-game-server gameplay_transport::tcp_tls::tests::frame_ -- --nocapture
cargo +1.94.0 test --locked -p oteryn-game-server gameplay_transport::tcp_tls::tests::transport_tls_ -- --nocapture
```

Expected PASS at the private transport boundary only. This is intermediate TDD evidence and does not yet claim FND-04/admission or production-composition evidence.

- [ ] **2.6 Commit Task 2**

```bash
git add Cargo.toml Cargo.lock apps/game-server/Cargo.toml apps/game-server/src/lib.rs apps/game-server/src/gameplay_transport/mod.rs apps/game-server/src/gameplay_transport/tcp_tls.rs
git commit -m "feat(server): add bounded TLS transport framing"
```

---

### Task 3: Private In-Crate Authority-Safe Connection State Machine

**Files:**
- Create: `apps/game-server/src/gameplay_transport/connection.rs`
- Modify: `apps/game-server/src/gameplay_transport/mod.rs`
- Test: `#[cfg(test)]` unit tests inside `gameplay_transport/connection.rs`

**Interfaces:**
- Consumes: bounded decoded Foundation envelope, typed bridge from Task 1, canonical FND-04 verifier/consumer, `AdmissionAuthority`, current reconnect/Durability flow and current-authority facts.
- Produces: one private connection-local state transition result and encoded registered Foundation response; no gameplay mutation interface and no externally public test hook.

- [ ] **3.1 RED — wire the private module before exercising it**

Add private `mod connection;` to `gameplay_transport/mod.rs`, then add in-crate RED tests inside `connection.rs`. Because these tests compile inside `oteryn-game-server`, they may consume the Task 1 `pub(crate)` Foundation bridge without widening visibility.

Require a new connection to reject client commands/server-direction types/unknown messages/non-zero pre-admission generation/malformed payload before FND-04 or durable mutation.

- [ ] **3.2 RED — fresh admission authority before mutation**

Use current verifier/evidence builders. One invariant per negative case, with unrelated facts valid, including:

- missing evidence;
- invalid signature/authentication;
- expired evidence;
- already-consumed/replayed grant/nonce;
- wrong transport/candidate binding;
- account/character/world mismatch;
- current CharacterLease/runtime authority mismatch;
- concurrent use of the **same valid grant/material** by two fresh-admission attempts.

For the replay/concurrent cases assert exactly one canonical `GameSession`/controller can become authoritative and the losing path fails/reconciles according to current FND-04/Foundation semantics; transport-local state cannot mark itself admitted first.

Positive route:

```text
validated ClientBootstrap
-> FND-04 verifier/consumer
-> independently current FreshAdmissionFacts/current authority
-> canonical AdmissionAuthority commit
-> bind admitted transport generation
-> ServerAccepted
```

- [ ] **3.3 Run fresh-admission RED**

```bash
cargo +1.94.0 test --locked -p oteryn-game-server gameplay_transport::connection::tests::admission_ -- --nocapture
```

Expected FAIL because the connection-state behavior is not implemented yet, **not** because the source file is unreachable from the crate or because a `pub(crate)` Foundation bridge is inaccessible from an external test crate.

- [ ] **3.4 Minimal fresh-admission GREEN**

Keep connection state local (`PreAdmission`, `Admitted { generation }`, `Closing` or equivalent) and do not duplicate GameSession/CharacterLease authority or cache a supposedly current fact beyond the boundary where it is valid.

- [ ] **3.5 RED — reconnect authority family**

One independent invariant per case across expected GameSession, account presence, character/world eligibility, runtime scope, predecessor generation, ownership generation/fence, control-loss/liveness, candidate transport binding, record provenance substitution, direct/reconciled PREPARED path, final COMMIT revalidation, terminal replay and concurrent replacement/stale-attempt distinction.

- [ ] **3.6 Minimal reconnect GREEN**

Consume current Foundation/Durability APIs. Resolve live facts through canonical current-authority sources; do not rebuild current authority from the persisted record. Direct and reconciled terminal outcomes remain historical unless current controller authority is independently reacquired.

Run the real in-crate source tests to GREEN:

```bash
cargo +1.94.0 test --locked -p oteryn-game-server gameplay_transport::connection::tests::admission_ -- --nocapture
cargo +1.94.0 test --locked -p oteryn-game-server gameplay_transport::connection::tests::reconnect_ -- --nocapture
```

These unit tests are intermediate authority TDD only. Task 6 must re-exercise the applicable admission/reconnect negative families through the actual production listener/composition path.

- [ ] **3.7 Commit Task 3**

```bash
git add apps/game-server/src/gameplay_transport/mod.rs apps/game-server/src/gameplay_transport/connection.rs
git commit -m "feat(server): bind transport to admission authority"
```

---

### Task 4: Private Lifecycle, Registered Resource Budgets, Backpressure and Drain

**Files:**
- Modify: `apps/game-server/src/gameplay_transport/mod.rs`
- Modify: `apps/game-server/src/gameplay_transport/connection.rs`
- Test: in-crate unit tests under `gameplay_transport`

- [ ] **4.1 RED — every registered Server Seam limit**

In the private in-crate lifecycle tests, prove max accepted, max+1 rejected/backpressured before partial mutation, and checked overflow where applicable:

```text
pre-admission connections: 256 / 257
handshake/auth units: 64 / 65
outbound queue entries: 64 / 65
outbound queue bytes: 1,048,576 / +1
pending writes: 8 / 9
drain tasks: max 256 per batch
```

- [ ] **4.2 Implement bounded lifecycle GREEN**

Extend the already-registered private `gameplay_transport/mod.rs`; do not add a duplicate module or a public test-only façade. Use bounded primitives only. Configuration supplies endpoint/TLS/admission dependencies; configurable values can reduce but never exceed/disable hard maxima. No unbounded channel/task/retry loop.

- [ ] **4.3 RED/GREEN — slow client and authoritative shutdown**

Prove one slow client consumes only its own queue/write budget. Then exercise at least these distinct shutdown states:

1. pre-authority connection-local work may be cancelled/released;
2. already-authoritative reserved work is either completed before shutdown returns **or** durably/reconcilably preserved by the owning Foundation/Durability contract;
3. a reconnect/controller transfer in progress cannot leave a stale transport authoritative after drain.

The shutdown assertion MUST observe the authoritative work's terminal/preserved state, not merely that tasks/permits disappeared. Silently dropping already-authoritative reserved work is a test failure.

Run the in-crate lifecycle targets:

```bash
cargo +1.94.0 test --locked -p oteryn-game-server gameplay_transport::tests::resource_ -- --nocapture
cargo +1.94.0 test --locked -p oteryn-game-server gameplay_transport::tests::backpressure_ -- --nocapture
cargo +1.94.0 test --locked -p oteryn-game-server gameplay_transport::tests::shutdown_ -- --nocapture
```

These are intermediate lifecycle TDD only. Task 6 must project applicable saturation/backpressure/shutdown preservation cases through the actual production composition before the seam can qualify.

- [ ] **4.4 Commit Task 4**

```bash
git add apps/game-server/src/gameplay_transport/mod.rs apps/game-server/src/gameplay_transport/connection.rs
git commit -m "feat(server): bound gameplay transport lifecycle"
```

---

### Task 5: Compose the Production Library/Startup Path and Create Physical Integration Tests

**Files:**
- Modify: `apps/game-server/src/lib.rs`
- Modify: `apps/game-server/src/main.rs`
- Create: `apps/game-server/tests/gameplay_server_seam.rs`

The private `gameplay_transport` module already exists and is compiled from Tasks 2-4. This task must compose that **same** module into the one production path; it must not create a second listener implementation, `#[path]` test copy or public test-only façade.

- [ ] **5.1 RED — production composition invariants and physical TLS/version/profile negatives**

Before adding the production composition/startup behavior, create `apps/game-server/tests/gameplay_server_seam.rs` against the intended **shipped** composition surface. The RED must be caused by missing production composition/startup behavior, not because tests cannot reach private source and not by widening private Foundation/transport visibility.

Require:

- `--smoke` to remain green without gameplay bind;
- missing config/TLS to fail closed;
- explicit caller-supplied loopback endpoint/TLS to compose the exact production transport in tests;
- no default production address/port/certificate/secret;
- no gameplay command to become accepted merely because a listener exists;
- a TLS 1.2-only client using the exact ALPN to fail at the real production listener before any frame/Foundation/admission handoff (`tls_legacy_12_rejected_before_frame_handoff`);
- missing/wrong ALPN and plaintext to fail at the real listener before frame/Foundation/admission handoff;
- a correctly framed `ClientBootstrap` carrying the wrong `protocol_major` to fail before FND-04 verification, `GameSession` creation or any admission mutation;
- a correctly framed `ClientBootstrap` carrying the wrong `transport_profile` to fail at that same real listener boundary before FND-04/admission mutation.

The version/profile cases MUST traverse TCP -> TLS -> BE32 frame read -> Foundation decode through the actual composed listener. Direct decoder-unit rejection or the Task 2 private transport harness is insufficient. Name them under the `tls_` selector as `tls_wrong_protocol_major_rejected_before_admission` and `tls_wrong_transport_profile_rejected_before_admission` so the focused production-composition command cannot omit them accidentally.

If exercising the shipped composition from an integration test would require a new externally public API/schema solely for test reachability, stop with `ARCHITECTURE_ESCALATION_REQUIRED` rather than adding that API. Use the existing/canonical production composition or executable surface.

Run the new composition/listener target before implementation and record RED caused by the missing production composition path.

- [ ] **5.2 Minimal production composition GREEN**

Promote the already-private `gameplay_transport` module from compile-time/in-crate TDD wiring into exactly one production composition path in `lib.rs`; do not redeclare or duplicate it. `main.rs` consumes only explicit configuration. If a product-wide configuration contract or canonical shipped composition surface is materially missing, stop with `ARCHITECTURE_ESCALATION_REQUIRED` rather than inventing it.

Run:

```bash
cargo +1.94.0 test --locked -p oteryn-game-server --test gameplay_server_seam tls_ -- --nocapture
cargo +1.94.0 run --locked -p oteryn-game-server -- --smoke
```

Expected PASS with TLS 1.2/ALPN/plaintext and wrong-version/profile cases rejected through the real composed listener before the relevant frame/Foundation/FND-04/admission boundary, while `--smoke` remains fail-closed from gameplay serving.

- [ ] **5.3 Commit Task 5**

```bash
git add apps/game-server/src/lib.rs apps/game-server/src/main.rs apps/game-server/tests/gameplay_server_seam.rs
git commit -m "feat(server): compose gameplay server seam"
```

---

### Task 6: Real Local Production-Path Authority, Resource and Resume Integration

**Files:**
- Modify: `apps/game-server/tests/gameplay_server_seam.rs`

This task demonstrates the Server Seam itself. It is **not** the later ADR-0007 QA Tier 1/Tier 2 evidence envelope. Unit tests from Tasks 1-4 remain supporting TDD only; this task must re-exercise the applicable mandatory negative families through the actual production listener/composition path.

- [ ] **6.1 Bootstrap/admission path**

Start the actual transport on `127.0.0.1:0` with caller-supplied non-shipping TLS material, connect with TLS 1.3 + exact ALPN, send framed `ClientBootstrap`, and require `ServerAccepted` only after canonical FND-04 verification and Foundation admission authority commits.

Also prove unsupported post-admission `ClientCommand`/gameplay state fails closed with zero domain/gameplay mutation; use an already registered applicable Foundation error/close semantic and never allocate a new gameplay ID.

- [ ] **6.2 Resume/reconnect path**

Drive a real admitted session through supported transport/control-loss setup, reconnect on fresh TLS, send `ClientResume`, exercise current durable PREPARE/direct-or-reconciled outcome/final current-authority revalidation as applicable, and require `ServerResumeAccepted` only for current candidate authority. A stale-generation sibling must remain fenced and cannot regain controller authority.

- [ ] **6.3 Physical negative-family projection**

Through the same actual composed listener (not a direct-domain helper or test-only listener), require representative physical evidence for every mandatory family whose inner invariant was established in Tasks 1-4, including at minimum:

- TLS 1.2-only + exact ALPN rejection, wrong/missing ALPN, plaintext and malformed/truncated/oversized framing before higher-layer authority;
- wrong protocol major/profile and malformed/direction/phase-invalid Foundation input before FND-04/admission mutation;
- invalid, expired, replayed and wrong-binding fresh-admission material plus concurrent use of the same valid grant/material, with at most one canonical GameSession/controller becoming authoritative;
- stale generation/reconnect candidate/authority mismatches and direct/reconciled reconnect outcomes with final current-authority revalidation and stale-transport fencing;
- registered connection/handshake/outbound/pending-write/drain saturation, including slow-client isolation, at their accepted maxima/max+1 boundaries where physically applicable;
- shutdown/drain after authority has been granted, observing completion or durable/reconcilable preservation of already-authoritative reserved work rather than merely permit/task disappearance;
- unsupported gameplay command/state after admission with zero domain mutation.

Where a complete max+1 physical setup would make the test itself unbounded or non-deterministic, keep the exact boundary proof in the private unit suite and add a bounded production-path projection proving that the same registered limiter is the one reached by the listener. Do not substitute a duplicate test-only limiter.

- [ ] **6.4 Whole production-path target**

```bash
cargo +1.94.0 test --locked -p oteryn-game-server --test gameplay_server_seam -- --nocapture
```

All physical and negative Server Seam tests PASS. Record exact candidate SHA, loopback topology, TLS version/ALPN, bounded framing, canonical admission/reconnect outcome, registered limiter identity for projected resource cases and explicit statement `FORMAL_ADR0007_QA_TIER1_TIER2=NOT_EVALUATED`.

- [ ] **6.5 Commit Task 6**

```bash
git add apps/game-server/tests/gameplay_server_seam.rs
git commit -m "test(server): prove gameplay seam production boundary"
```

---

### Task 7: Validation, Finding-Family Sweep and Stable Candidate

- [ ] **7.1 Focused/package validation**

```bash
cargo +1.94.0 fmt --all -- --check
cargo +1.94.0 test --locked -p oteryn-game-server --test gameplay_server_seam
cargo +1.94.0 test --locked -p oteryn-game-server
cargo +1.94.0 clippy --locked -p oteryn-game-server --all-targets -- -D warnings
```

- [ ] **7.2 Affected workspace validation**

```bash
cargo +1.94.0 build --locked --workspace --all-targets
cargo +1.94.0 clippy --locked --workspace --all-targets -- -D warnings
cargo +1.94.0 test --locked --workspace
cargo +1.94.0 run --locked -p oteryn-synthetic-client-harness
```

- [ ] **7.3 Authority finding-family sweep before material freeze**

Review every applicable cell:

```text
AuthorityInvariant
  x fresh admission / direct resume / reconciled resume / final commit / admitted I/O / drain
  x missing / stale / mismatch / temporal / provenance-substitution / replay-concurrency
```

Inspect sibling APIs, protocol v1, TLS 1.3-only/no-legacy behavior, direct/reconciled paths, fenced durable writes, restart/retry/replay/concurrent replacement and PostgreSQL reload implications. Verify that every private Task 1-4 TDD family that contributes to the accepted physical seam claim has a production-listener projection in Tasks 5-6. Current #302/#303 tests are reusable evidence only where the Server Seam diff leaves the underlying boundary representative; otherwise add owned-path physical proof or report the smallest `SHARED_LEASE_REQUIRED`.

- [ ] **7.4 Whole-diff self-review**

Check duplicate protocol/admission/session authority; record-derived current facts; unchecked allocation; unbounded queues/tasks/retries; TLS 1.2/legacy downgrade or plaintext; wrong ALPN/version/profile; test-only duplicate composition/public visibility widening; new IDs; accounting divergence; stale generation; authoritative-work shutdown; production defaults; unrelated dependency upgrades/path expansion; accidental QA Tier overclaim.

Verify every P0/P1 finding against the exact head and accept+repair test-first or reject with exact evidence. Every P2 gets explicit disposition.

- [ ] **7.5 Pre-freeze task metadata**

Record known validation/family-sweep/finding dispositions in the worker task before material freeze without trying to write the commit's own SHA into the same commit. Commit only real evidence/repair metadata, not a no-op retrigger.

---

### Task 8: Exact-Head CI, Independent Review and Handoff

- [ ] **8.1 Publish stable candidate normally**

No force/rebase/reset. Verify remote head and exact allowlist on GitHub.

- [ ] **8.2 Exact-head repository qualification**

Require all current protected-base selected gates for the exact head, including Linux/real PostgreSQL, Windows/SIM, supply chain and canonical `game-gate` whenever current trusted policy classifies them applicable. If protected `main` advances, reconcile normally and rerun invalidated evidence.

- [ ] **8.3 One genuinely independent exact-head deep review**

Review protocol/session/admission/reconnect/fencing/transport/resource risks, TLS 1.3-only enforcement, sequential private-module TDD and production-path negative projection. The implementing session's self-review cannot qualify as independent. External AI remains advisory/non-mutating.

- [ ] **8.4 Finding reconciliation**

Accepted material finding -> fresh RED -> minimal GREEN -> affected family sweep/validation -> new representative review when required. Verified rejection with exact evidence preserves the candidate. No no-op commit just to retrigger checks/review.

- [ ] **8.5 Handoff without self-merge**

```yaml
lane: SERVER_SEAM
issue: 247
task_id: OTV2-20260904-gameplay-server-seam
admission_main_sha: <allocation merge SHA used to create worker branch>
integration_main_sha: <fresh main SHA at integration preflight>
branch: agent/otv2-gameplay-server-seam-01
pr: <worker PR>
final_head_sha: <exact remote head>
changed_paths: []
shared_lease_used:
  - apps/game-server/src/foundation/protocol.rs
  - apps/game-server/src/lib.rs
  - apps/game-server/src/main.rs
  - apps/game-server/Cargo.toml
  - Cargo.toml
  - Cargo.lock
state: READY_FOR_INTEGRATION
focused_validation: []
component_validation: []
server_seam_physical_integration: <exact local production-path evidence>
formal_adr0007_qa_tier1_tier2: NOT_EVALUATED
self_review: <exact-head verdict>
independent_review: <exact-head verdict>
architecture_escalation: null
unresolved_findings: []
recommended_control_plane_action: integrate
next_action: Work independently verifies exact head/main relation/checks/review/threads/paths and integrates through protected controls if every predicate remains proven
```

If any predicate is not proven, use a truthful non-ready state rather than `READY_FOR_INTEGRATION`.