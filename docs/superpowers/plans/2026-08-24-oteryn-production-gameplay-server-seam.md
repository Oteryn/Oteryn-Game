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
- Preserve the FND-02 wire-frame hard maximum of 1,048,576 bytes and every smaller message-specific bound in the current registry/code.
- Preserve registered Server Seam hard maxima: 256 pre-admission connections, 64 concurrent handshake/auth units, 64 outbound queue entries/session, 1,048,576 outbound queued bytes/session, 8 pending writes/session and 256 drain tasks/batch.
- No new protocol/event/state/capability/stable numeric ID, no Reference formula/value and no permanent Content-format decision.
- No second admission/GameSession/CharacterLease/reconnect/durability authority; transport is a consumer only.
- No production bind address/port/certificate/private-key/secret/deployment choice. Tests use loopback/ephemeral endpoints and non-shipping test TLS material.
- `workspace-boundaries.toml`, workflows, rulesets and architecture/contracts are not allocated.
- High-risk authority work follows `AuthorityInvariant × ConsumerBoundary × MutationOperator`, one invariant per negative case, current facts independent of immutable records, full finding-family sweep before freeze.
- Every implementation task follows fresh RED -> minimal GREEN. Do not weaken an assertion, fail-closed path, limit or authority boundary to make a test pass.
- Real local production-path tests in this plan qualify the Server Seam boundary only. They MUST NOT be reported as formal ADR-0007 QA Tier 1/Tier 2; QA owns that later evidence after the seam is merged.

---

## File Structure

- `apps/game-server/src/foundation/protocol.rs` — Foundation-owned crate-internal typed extraction for already-registered bootstrap/resume messages and crate-internal outbound encoding for already-registered server Foundation messages.
- `apps/game-server/src/gameplay_transport/tcp_tls.rs` — TCP accept/TLS 1.3/ALPN/profile-1 handshake plus bounded big-endian-u32 frame I/O; no admission semantics.
- `apps/game-server/src/gameplay_transport/connection.rs` — one connection state machine that maps validated Foundation messages to existing FND-04/GameSession/Durability consumers and owns connection-local backpressure/generation state only.
- `apps/game-server/src/gameplay_transport/mod.rs` — configuration/lifecycle composition, registered semaphores/budgets, listener/drain orchestration and testable start/stop API.
- `apps/game-server/src/lib.rs` — compose the transport module and preserve existing bootstrap/foundation regression modules.
- `apps/game-server/src/main.rs` — configuration-driven startup entry; preserve `--smoke`; no hard-coded production endpoint or TLS material.
- `apps/game-server/Cargo.toml`, `Cargo.toml`, `Cargo.lock` — only exact TLS/transport dependency/features needed by the seam; no unrelated dependency updates.
- `apps/game-server/tests/gameplay_server_seam.rs` — real loopback production-path TCP/TLS bootstrap/resume integration plus malformed/limit/authority/backpressure/shutdown evidence; not formal QA Tier evidence.

The Foundation bridge MUST stay `pub(crate)` or narrower unless an already-canonical consumer requires wider visibility. If a new externally public API/schema is required, stop before that mutation with `ARCHITECTURE_ESCALATION_REQUIRED`.

---

### Task 1: Foundation Typed Consumer Bridge

**Files:**
- Modify: `apps/game-server/src/foundation/protocol.rs`
- Test: existing `protocol.rs` unit-test module and `apps/game-server/tests/gameplay_server_seam.rs`

**Interfaces:**
- Consumes: existing `WireEnvelopeView<'a>`, `MessageType`, `decode_wire_envelope`, existing protocol constants/identifier decoders and current FND-04 material formats.
- Produces: crate-internal typed views for validated `ClientBootstrap`/`ClientResume`; crate-internal encoders for already-registered `ServerAccepted`, `ServerResumeAccepted` and `ProtocolError`; no new wire IDs.

- [ ] **1.1 RED — typed extraction**

Add tests that decode canonical bootstrap/resume envelopes, call the new crate-internal typed extraction, and assert exact validated fields are available without reparsing raw bytes in transport code. Negative cases include wrong message type, missing required field, duplicate singular field, oversized material/build ID, invalid UUID/capability shape, wrong `protocol_major`, wrong `transport_profile` and bootstrap-phase generation misuse.

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
git add apps/game-server/src/foundation/protocol.rs apps/game-server/tests/gameplay_server_seam.rs
git commit -m "feat(foundation): expose server seam protocol bridge"
```

---

### Task 2: Bounded TCP/TLS Framing Boundary

**Files:**
- Create: `apps/game-server/src/gameplay_transport/tcp_tls.rs`
- Create/Modify: `apps/game-server/tests/gameplay_server_seam.rs`
- Modify: `apps/game-server/Cargo.toml`
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`

**Interfaces:**
- Consumes: Tokio async TCP/I/O; TLS implementation pinned under workspace policy; `ALPN_OTERYN_GAME_V1`; `MAX_WIRE_FRAME_BYTES`/current FND limits.
- Produces: authenticated TLS stream wrapper plus bounded encoded Foundation frame read/write operations.

- [ ] **2.1 RED — raw frame boundary**

On a real loopback transport reader require 0, 1,048,577, truncated prefix/body and checked accounting failures before peer-sized allocation; exact 1,048,576 is accepted when a complete body exists and message-specific validation permits it.

- [ ] **2.2 Run frame RED**

```bash
cargo +1.94.0 test --locked -p oteryn-game-server --test gameplay_server_seam frame_ -- --nocapture
```

Expected FAIL because gameplay transport does not exist.

- [ ] **2.3 Add only required TLS dependencies**

Inspect compatible Rust TLS crates at implementation time and add exact-pinned workspace dependencies/features required for Tokio TCP + TLS 1.3 + ALPN + non-shipping test certificate material. Do not absorb Dependabot #259/#260/#261 upgrades as convenience.

Before any Cargo edit, re-read active ownership. A new non-Dependabot Cargo writer is `SHARED_LEASE_REQUIRED`.

- [ ] **2.4 Minimal bounded TLS/frame GREEN**

```text
registered pre-admission slot held
-> TCP accept
-> bounded handshake/auth slot
-> TLS 1.3
-> exact ALPN oteryn-game/1
-> release handshake slot
-> read 4-byte BE length
-> validate 1..=1_048_576 before body allocation/read
-> read exact body
-> Foundation decode/connection state machine
```

Write-side framing obeys the same hard maximum. TLS/profile failure closes before admission mutation.

- [ ] **2.5 RED/GREEN — TLS/ALPN/version/profile/plaintext failures through the composed listener**

Using the actual local production listener/composition path with non-shipping loopback TLS material, prove:

- TLS 1.3 + exact ALPN succeeds;
- missing/wrong ALPN fails before admission;
- a correctly framed `ClientBootstrap` carrying the wrong `protocol_major` fails before FND-04 verification, `GameSession` creation or any admission mutation;
- a correctly framed `ClientBootstrap` carrying the wrong `transport_profile` fails at that same real listener boundary before FND-04/admission mutation;
- both wrong-version/profile cases traverse TCP -> TLS -> BE32 frame read -> Foundation decode through the composed listener; direct decoder-unit rejection alone does **not** satisfy these two cases;
- plaintext never reaches Foundation admission;
- malformed/truncated/oversized frames fail closed.

Name the two production-path cases under the `tls_` selector (for example `tls_wrong_protocol_major_rejected_before_admission` and `tls_wrong_transport_profile_rejected_before_admission`) so the focused command below cannot omit them accidentally.

Run:

```bash
cargo +1.94.0 test --locked -p oteryn-game-server --test gameplay_server_seam frame_ -- --nocapture
cargo +1.94.0 test --locked -p oteryn-game-server --test gameplay_server_seam tls_ -- --nocapture
```

Expected PASS with explicit evidence that neither invalid envelope reaches the FND-04/admission mutation boundary.

- [ ] **2.6 Commit Task 2**

```bash
git add Cargo.toml Cargo.lock apps/game-server/Cargo.toml apps/game-server/src/gameplay_transport/tcp_tls.rs apps/game-server/tests/gameplay_server_seam.rs
git commit -m "feat(server): add bounded TLS transport framing"
```

---

### Task 3: Authority-Safe Connection State Machine

**Files:**
- Create: `apps/game-server/src/gameplay_transport/connection.rs`
- Modify: `apps/game-server/tests/gameplay_server_seam.rs`

**Interfaces:**
- Consumes: bounded decoded Foundation envelope, typed bridge from Task 1, canonical FND-04 verifier/consumer, `AdmissionAuthority`, current reconnect/Durability flow and current-authority facts.
- Produces: one connection-local state transition result and encoded registered Foundation response; no gameplay mutation interface.

- [ ] **3.1 RED — pre-admission legality**

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
cargo +1.94.0 test --locked -p oteryn-game-server --test gameplay_server_seam admission_ -- --nocapture
```

Expected FAIL because connection state machine is absent.

- [ ] **3.4 Minimal fresh-admission GREEN**

Keep connection state local (`PreAdmission`, `Admitted { generation }`, `Closing` or equivalent) and do not duplicate GameSession/CharacterLease authority or cache a supposedly current fact beyond the boundary where it is valid.

- [ ] **3.5 RED — reconnect authority family**

One independent invariant per case across expected GameSession, account presence, character/world eligibility, runtime scope, predecessor generation, ownership generation/fence, control-loss/liveness, candidate transport binding, record provenance substitution, direct/reconciled PREPARED path, final COMMIT revalidation, terminal replay and concurrent replacement/stale-attempt distinction.

- [ ] **3.6 Minimal reconnect GREEN**

Consume current Foundation/Durability APIs. Resolve live facts through canonical current-authority sources; do not rebuild current authority from the persisted record. Direct and reconciled terminal outcomes remain historical unless current controller authority is independently reacquired.

Run `admission_` and `reconnect_` focused suites to GREEN.

- [ ] **3.7 Commit Task 3**

```bash
git add apps/game-server/src/gameplay_transport/connection.rs apps/game-server/tests/gameplay_server_seam.rs
git commit -m "feat(server): bind transport to admission authority"
```

---

### Task 4: Registered Resource Budgets, Backpressure and Drain

**Files:**
- Create: `apps/game-server/src/gameplay_transport/mod.rs`
- Modify: `apps/game-server/src/gameplay_transport/connection.rs`
- Modify: `apps/game-server/tests/gameplay_server_seam.rs`

- [ ] **4.1 RED — every registered Server Seam limit**

Prove max accepted, max+1 rejected/backpressured before partial mutation, and checked overflow where applicable:

```text
pre-admission connections: 256 / 257
handshake/auth units: 64 / 65
outbound queue entries: 64 / 65
outbound queue bytes: 1,048,576 / +1
pending writes: 8 / 9
drain tasks: max 256 per batch
```

- [ ] **4.2 Implement bounded lifecycle GREEN**

Use bounded primitives only. Configuration supplies endpoint/TLS/admission dependencies; configurable values can reduce but never exceed/disable hard maxima. No unbounded channel/task/retry loop.

- [ ] **4.3 RED/GREEN — slow client and authoritative shutdown**

Prove one slow client consumes only its own queue/write budget. Then exercise at least these distinct shutdown states:

1. pre-authority connection-local work may be cancelled/released;
2. already-authoritative reserved work is either completed before shutdown returns **or** durably/reconcilably preserved by the owning Foundation/Durability contract;
3. a reconnect/controller transfer in progress cannot leave a stale transport authoritative after drain.

The shutdown assertion MUST observe the authoritative work's terminal/preserved state, not merely that tasks/permits disappeared. Silently dropping already-authoritative reserved work is a test failure.

Run:

```bash
cargo +1.94.0 test --locked -p oteryn-game-server --test gameplay_server_seam resource_ -- --nocapture
cargo +1.94.0 test --locked -p oteryn-game-server --test gameplay_server_seam backpressure_ -- --nocapture
cargo +1.94.0 test --locked -p oteryn-game-server --test gameplay_server_seam shutdown_ -- --nocapture
```

- [ ] **4.4 Commit Task 4**

```bash
git add apps/game-server/src/gameplay_transport/mod.rs apps/game-server/src/gameplay_transport/connection.rs apps/game-server/tests/gameplay_server_seam.rs
git commit -m "feat(server): bound gameplay transport lifecycle"
```

---

### Task 5: Compose Library and Configuration-Driven Startup

**Files:**
- Modify: `apps/game-server/src/lib.rs`
- Modify: `apps/game-server/src/main.rs`
- Modify: `apps/game-server/tests/gameplay_server_seam.rs`

- [ ] **5.1 RED — composition invariants**

Require `--smoke` to remain green without gameplay bind; missing config/TLS to fail closed; explicit caller-supplied loopback endpoint/TLS to compose the exact production transport in tests; no default production address/port/certificate/secret; no gameplay command becomes accepted merely because a listener exists.

- [ ] **5.2 Minimal composition GREEN**

Wire exactly one transport module into `lib.rs`. `main.rs` consumes only explicit configuration; if a product-wide configuration contract is materially missing, stop with `ARCHITECTURE_ESCALATION_REQUIRED` rather than inventing it.

Run composition suite + `cargo +1.94.0 run --locked -p oteryn-game-server -- --smoke`.

- [ ] **5.3 Commit Task 5**

```bash
git add apps/game-server/src/lib.rs apps/game-server/src/main.rs apps/game-server/tests/gameplay_server_seam.rs
git commit -m "feat(server): compose gameplay server seam"
```

---

### Task 6: Real Local Production-Path Bootstrap and Resume Integration

**Files:**
- Modify: `apps/game-server/tests/gameplay_server_seam.rs`

This task demonstrates the Server Seam itself. It is **not** the later ADR-0007 QA Tier 1/Tier 2 evidence envelope.

- [ ] **6.1 Bootstrap/admission path**

Start the actual transport on `127.0.0.1:0` with caller-supplied non-shipping TLS material, connect with TLS 1.3 + exact ALPN, send framed `ClientBootstrap`, and require `ServerAccepted` only after canonical FND-04 verification and Foundation admission authority commits.

Also prove unsupported post-admission `ClientCommand`/gameplay state fails closed with zero domain/gameplay mutation; use an already registered applicable Foundation error/close semantic and never allocate a new gameplay ID.

- [ ] **6.2 Resume/reconnect path**

Drive a real admitted session through supported transport/control-loss setup, reconnect on fresh TLS, send `ClientResume`, exercise current durable PREPARE/direct-or-reconciled outcome/final current-authority revalidation as applicable, and require `ServerResumeAccepted` only for current candidate authority. A stale-generation sibling must remain fenced and cannot regain controller authority.

- [ ] **6.3 Whole production-path target**

```bash
cargo +1.94.0 test --locked -p oteryn-game-server --test gameplay_server_seam -- --nocapture
```

All physical and negative Server Seam tests PASS. Record exact candidate SHA, loopback topology, TLS/ALPN, bounded framing, canonical admission/reconnect outcome and explicit statement `FORMAL_ADR0007_QA_TIER1_TIER2=NOT_EVALUATED`.

- [ ] **6.4 Commit Task 6**

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

Inspect sibling APIs, protocol v1, direct/reconciled paths, fenced durable writes, restart/retry/replay/concurrent replacement and PostgreSQL reload implications. Current #302/#303 tests are reusable evidence only where the Server Seam diff leaves the underlying boundary representative; otherwise add owned-path physical proof or report the smallest `SHARED_LEASE_REQUIRED`.

- [ ] **7.4 Whole-diff self-review**

Check duplicate protocol/admission/session authority; record-derived current facts; unchecked allocation; unbounded queues/tasks/retries; TLS downgrade/plaintext; wrong ALPN/version/profile; new IDs; accounting divergence; stale generation; authoritative-work shutdown; production defaults; unrelated dependency upgrades/path expansion; accidental QA Tier overclaim.

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

Review protocol/session/admission/reconnect/fencing/transport/resource risks. The implementing session's self-review cannot qualify as independent. External AI remains advisory/non-mutating.

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
