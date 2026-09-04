# Production Gameplay Server Seam Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the smallest accepted production TCP/TLS gameplay entry seam that consumes canonical Foundation/Durability authority, remains fail-closed for unallocated gameplay, enforces registered resource limits, and proves a real local Tier 1 bootstrap/resume boundary.

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

---

## File Structure

- `apps/game-server/src/foundation/protocol.rs` — Foundation-owned crate-internal typed extraction for already-registered bootstrap/resume messages and crate-internal outbound encoding for already-registered server Foundation messages.
- `apps/game-server/src/gameplay_transport/tcp_tls.rs` — TCP accept/TLS 1.3/ALPN/profile-1 handshake plus bounded big-endian-u32 frame I/O; no admission semantics.
- `apps/game-server/src/gameplay_transport/connection.rs` — one connection state machine that maps validated Foundation messages to existing FND-04/GameSession/Durability consumers and owns connection-local backpressure/generation state only.
- `apps/game-server/src/gameplay_transport/mod.rs` — configuration/lifecycle composition, registered semaphores/budgets, listener/drain orchestration and testable start/stop API.
- `apps/game-server/src/lib.rs` — compose the transport module and preserve existing bootstrap/foundation regression modules.
- `apps/game-server/src/main.rs` — configuration-driven startup entry; preserve `--smoke`; no hard-coded production endpoint or TLS material.
- `apps/game-server/Cargo.toml`, `Cargo.toml`, `Cargo.lock` — only exact TLS/transport dependency/features needed by the seam; no unrelated dependency updates.
- `apps/game-server/tests/gameplay_server_seam.rs` — real loopback TCP/TLS Tier 1 plus malformed/limit/authority/backpressure/shutdown integration evidence.

The Foundation bridge MUST stay `pub(crate)` or narrower unless an already-canonical consumer requires wider visibility. If a new externally public API/schema is required, stop before that mutation with `ARCHITECTURE_ESCALATION_REQUIRED`.

---

### Task 1: Foundation Typed Consumer Bridge

**Files:**
- Modify: `apps/game-server/src/foundation/protocol.rs`
- Test: existing `protocol.rs` unit-test module and `apps/game-server/tests/gameplay_server_seam.rs`

**Interfaces:**
- Consumes: existing `WireEnvelopeView<'a>`, `MessageType`, `decode_wire_envelope`, existing protocol constants/identifier decoders and current FND-04 material formats.
- Produces: crate-internal typed views for validated `ClientBootstrap`/`ClientResume`; crate-internal encoders for already-registered `ServerAccepted`, `ServerResumeAccepted` and `ProtocolError`; no new wire IDs.

- [ ] **Step 1: Write focused failing tests for typed extraction**

Add tests that decode canonical bootstrap/resume envelopes, call the new crate-internal typed extraction, and assert exact validated fields are available without reparsing raw bytes in transport code. Add negative cases for wrong message type, missing required field, duplicate singular field, oversized material/build ID, invalid UUID/capability shape and bootstrap-phase generation misuse.

The tests must make the intended API crate-internal, for example:

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

- [ ] **Step 2: Run the focused RED**

Run:

```bash
cargo +1.94.0 test --locked -p oteryn-game-server foundation::protocol -- --nocapture
```

Expected: FAIL because the crate-internal typed extraction methods/types do not exist yet. Existing protocol validation tests must continue compiling/passing up to the new failures.

- [ ] **Step 3: Implement the minimal crate-internal typed extraction**

Parse fields once inside Foundation using the same bounded protobuf helpers already used for validation. Return borrowed views where possible; do not allocate untrusted payload copies merely to expose validated fields. Do not export these types outside the game-server crate.

The transport consumer should be able to use an interface equivalent to:

```rust
pub(crate) struct ClientBootstrapView<'a> { /* validated borrowed fields */ }
pub(crate) struct ClientResumeView<'a> { /* validated borrowed fields */ }

impl<'a> WireEnvelopeView<'a> {
    pub(crate) fn client_bootstrap(&self) -> Result<ClientBootstrapView<'a>, FoundationProtocolError>;
    pub(crate) fn client_resume(&self) -> Result<ClientResumeView<'a>, FoundationProtocolError>;
}
```

Exact field storage may follow existing parser helpers, but the semantic result must not re-interpret or weaken current validator rules.

- [ ] **Step 4: Write failing outbound-encoder tests**

Add exact round-trip/wire tests for already-registered server messages. The encoder interface should remain crate-internal and accept typed existing identifiers/values rather than raw unvalidated field maps:

```rust
let encoded = encode_server_accepted(&accepted)?;
let decoded = decode_wire_envelope(&encoded)?;
assert_eq!(decoded.message_type(), MessageType::ServerAccepted);
```

Cover `ServerAccepted`, `ServerResumeAccepted`, `ProtocolError`, connection generation/server-sequence legality and the global frame/message limits. Assert no unregistered message ID can be supplied through a generic numeric encoder.

- [ ] **Step 5: Run outbound RED, implement minimal registered-only encoders, rerun GREEN**

Run the same focused command. Implement only the three accepted server Foundation message families using existing protobuf/wire rules and checked lengths; do not add a general caller-selected message-ID API.

Expected after implementation: focused protocol tests PASS.

- [ ] **Step 6: Commit Task 1**

```bash
git add apps/game-server/src/foundation/protocol.rs
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
- Produces: an authenticated TLS stream wrapper plus `read_frame`/`write_frame` operations that deal only in bounded encoded Foundation frames.

- [ ] **Step 1: Write RED tests for raw frame boundaries before adding TLS dependencies**

In `gameplay_server_seam.rs`, define loopback tests that require a transport frame reader to:

```rust
assert_eq!(read_frame(prefix_for(0), &mut reader).await, Err(TransportError::InvalidFrameLength));
assert_eq!(read_frame(prefix_for(1_048_577), &mut reader).await, Err(TransportError::FrameTooLarge));
```

Also require exact 1,048,576 acceptance when the body exists, truncated prefix/body rejection and checked conversion/accounting. Do not reserve a peer-declared body before the hard maximum is verified.

- [ ] **Step 2: Run RED**

```bash
cargo +1.94.0 test --locked -p oteryn-game-server --test gameplay_server_seam frame_ -- --nocapture
```

Expected: FAIL because `gameplay_transport`/framing does not exist.

- [ ] **Step 3: Add only the exact TLS dependencies/features required by the accepted profile**

Inspect current compatible Rust TLS crates at implementation time and add exact-pinned workspace dependencies following existing repository policy. Enable only features needed for Tokio TCP + TLS 1.3 + ALPN and test certificate loading. Do not absorb #259/#260/#261 dependency upgrades unless the exact dependency is independently required by this seam.

Before editing Cargo files, re-read open PR ownership. If a non-Dependabot active writer has acquired `Cargo.toml`/`Cargo.lock`, stop and return `SHARED_LEASE_REQUIRED` rather than editing concurrently.

- [ ] **Step 4: Implement bounded frame I/O and TLS profile enforcement**

`tcp_tls.rs` must:

```text
TCP accepted
  -> registered pre-admission slot already held by caller
  -> TLS 1.3 handshake under handshake slot
  -> ALPN must equal oteryn-game/1
  -> release handshake slot
  -> read 4-byte big-endian length
  -> validate 1..=1_048_576 before body allocation/read
  -> read exact body
  -> Foundation decode/connection state machine
```

Write paths use the same checked maximum and never permit a frame that exceeds FND-02. TLS/profile errors close the connection before admission mutation.

- [ ] **Step 5: Add TLS/ALPN negative tests and rerun GREEN**

Use non-shipping loopback certificate/key fixtures generated or embedded under test-only code. Require:

- TLS 1.3 succeeds with ALPN `oteryn-game/1`;
- missing/wrong ALPN fails before admission;
- plaintext bytes never reach Foundation admission;
- malformed/truncated/oversized frame closes fail-closed.

Run:

```bash
cargo +1.94.0 test --locked -p oteryn-game-server --test gameplay_server_seam frame_ -- --nocapture
cargo +1.94.0 test --locked -p oteryn-game-server --test gameplay_server_seam tls_ -- --nocapture
```

Expected: PASS.

- [ ] **Step 6: Commit Task 2**

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

- [ ] **Step 1: Write RED for pre-admission message legality**

Require a new connection to accept only bootstrap/resume/protocol-error handling allowed by FND-02 phase rules. Examples:

```rust
assert_eq!(connection.handle(client_command_before_admission).await, Err(ConnectionError::Protocol(_)));
assert!(!authority_probe.any_session_mutation());
```

Unknown message types, server-to-client types received from the client, non-zero pre-admission generation, and malformed payloads must fail before FND-04 or durable mutation.

- [ ] **Step 2: Write RED for fresh admission authority-before-mutation**

Use current test-support evidence builders/verifier seams. Prove separately that missing/invalid authentication evidence, mismatched character/world/lease facts and stale authority facts do not create an admitted session or transport generation.

The positive case must route:

```text
validated ClientBootstrap
 -> FND-04 evidence consumer
 -> canonical FreshAdmissionFacts/current authority
 -> AdmissionAuthority commit
 -> attach admitted transport generation
 -> ServerAccepted
```

Transport-local state must not mark the connection admitted before canonical commit succeeds.

- [ ] **Step 3: Run the admission RED**

```bash
cargo +1.94.0 test --locked -p oteryn-game-server --test gameplay_server_seam admission_ -- --nocapture
```

Expected: FAIL because the connection state machine is absent.

- [ ] **Step 4: Implement the minimal fresh-admission path**

Keep connection state explicit, equivalent to:

```rust
enum ConnectionPhase {
    PreAdmission,
    Admitted { generation: ConnectionGeneration },
    Closing,
}
```

The exact internal type may include references/handles needed by current Foundation/Durability APIs, but it cannot duplicate GameSession/CharacterLease authority or cache supposedly current authority beyond the boundary where it is valid.

- [ ] **Step 5: Write RED for reconnect/resume authority family**

Cover one invariant per case across:

- expected GameSession mismatch;
- account/character/world mismatch;
- runtime-scope mismatch within the same world;
- stale/mismatched predecessor connection generation;
- stale ownership generation/authority fence;
- stale/missing control-loss epoch/liveness fact;
- candidate transport/binding mismatch;
- persisted-record provenance substituted for independently current facts;
- PREPARED/reconciled path and final COMMIT revalidation;
- replay of terminal outcomes without reacquiring current controller authority;
- concurrent replacement/stale attempt distinction.

Each negative case keeps unrelated facts semantically valid and changes exactly one applicable invariant.

- [ ] **Step 6: Implement reconnect by consuming the current Foundation/Durability APIs, then rerun GREEN**

Do not recreate record-derived current authority in transport. Resolve current facts through the same canonical Foundation/current-authority sources required by protected-main governance and pass them to the durable flow at the exact consumer boundary.

Run:

```bash
cargo +1.94.0 test --locked -p oteryn-game-server --test gameplay_server_seam admission_ -- --nocapture
cargo +1.94.0 test --locked -p oteryn-game-server --test gameplay_server_seam reconnect_ -- --nocapture
```

Expected: PASS.

- [ ] **Step 7: Commit Task 3**

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

**Interfaces:**
- Consumes: Task 2 TLS accept/frame I/O, Task 3 connection state machine, current cancellation token pattern, registered NET03 hard maxima.
- Produces: listener lifecycle with bounded semaphores/queues/write slots/drain batches and deterministic shutdown.

- [ ] **Step 1: Write RED for each registered Server Seam limit**

Use test configuration fixed at the canonical hard maxima; do not make a configurable value exceed them. For each resource, prove max accepted and max+1 rejected before partial mutation:

```text
pre-admission connections: 256 / 257th rejected
handshake/auth units: 64 / 65th waits or rejects under bounded policy without spawning unbounded work
outbound queue entries: 64 / 65th rejected/backpressured
outbound queue bytes: 1,048,576 / +1 rejected with checked arithmetic
pending writes: 8 / 9th rejected/backpressured
drain tasks per batch: 256 / excess split/rejected without unbounded join set
```

Also add integer-overflow/accounting tests where arithmetic can overflow.

- [ ] **Step 2: Run resource RED**

```bash
cargo +1.94.0 test --locked -p oteryn-game-server --test gameplay_server_seam resource_ -- --nocapture
```

Expected: FAIL because listener/resource orchestration is absent.

- [ ] **Step 3: Implement exact bounded lifecycle**

Use bounded primitives only. The module API should require caller-provided bind/TLS/admission dependencies, conceptually:

```rust
pub struct GameplayServerConfig { /* caller-supplied endpoint/TLS + values bounded by canonical maxima */ }
pub struct GameplayServer { /* listener, cancellation, canonical bounded budgets */ }

impl GameplayServer {
    pub async fn bind(config: GameplayServerConfig, dependencies: GameplayDependencies<'_>) -> Result<Self, GameplayServerError>;
    pub async fn run_until_shutdown(&self) -> Result<(), GameplayServerError>;
    pub fn request_shutdown(&self);
}
```

Keep these crate/module visibility-scoped unless existing product composition requires wider visibility. No runtime option may disable the absolute hard maxima.

- [ ] **Step 4: Implement slow-client backpressure and deterministic drain**

One slow client must consume only its own registered queue/write budget. No hidden retry-until-success loop, unbounded channel, unbounded spawned writer task or cross-session authority wait. Shutdown cancels accepts, stops new admission, closes/drains bounded connection-local work and releases permits.

- [ ] **Step 5: Run resource/backpressure/shutdown GREEN**

```bash
cargo +1.94.0 test --locked -p oteryn-game-server --test gameplay_server_seam resource_ -- --nocapture
cargo +1.94.0 test --locked -p oteryn-game-server --test gameplay_server_seam backpressure_ -- --nocapture
cargo +1.94.0 test --locked -p oteryn-game-server --test gameplay_server_seam shutdown_ -- --nocapture
```

Expected: PASS.

- [ ] **Step 6: Commit Task 4**

```bash
git add apps/game-server/src/gameplay_transport/mod.rs apps/game-server/src/gameplay_transport/connection.rs apps/game-server/tests/gameplay_server_seam.rs
git commit -m "feat(server): bound gameplay transport lifecycle"
```

---

### Task 5: Compose Library and Configuration-Driven Executable Startup

**Files:**
- Modify: `apps/game-server/src/lib.rs`
- Modify: `apps/game-server/src/main.rs`
- Modify: `apps/game-server/tests/gameplay_server_seam.rs`

**Interfaces:**
- Consumes: complete `gameplay_transport` module and existing `GameServerBootstrap`/`bootstrap_smoke` behavior.
- Produces: one composed Server Seam; normal executable startup remains fail-closed unless explicit valid caller configuration is supplied.

- [ ] **Step 1: Write RED for composition invariants**

Require:

- `--smoke` still succeeds using bootstrap smoke without binding gameplay;
- absent gameplay configuration does not open a listener and exits/reports fail-closed;
- invalid/missing TLS material does not fall back to plaintext;
- caller-supplied loopback endpoint/TLS config can start the exact production transport module in integration tests;
- no gameplay command becomes accepted merely because the transport is listening.

- [ ] **Step 2: Run composition RED**

```bash
cargo +1.94.0 test --locked -p oteryn-game-server --test gameplay_server_seam composition_ -- --nocapture
cargo +1.94.0 run --locked -p oteryn-game-server -- --smoke
```

Expected: new composition tests FAIL before wiring; existing smoke remains successful.

- [ ] **Step 3: Wire `lib.rs` without changing authority ownership**

Add the transport module and a composition entry that takes already-constructed configuration/dependencies. Preserve existing Foundation/Durability regression modules and the fail-closed bootstrap semantics used by tests.

- [ ] **Step 4: Wire `main.rs` only to explicit configuration input**

Do not add a default production address, port, certificate path, secret name or deployment topology. If the repository has no accepted product configuration source for a required runtime value, normal invocation must remain fail-closed for that missing value; the physical Tier 1 test uses the library composition with caller-supplied loopback/test TLS material.

If a new product-wide configuration contract is required to make `main` production-operable, classify that precise need `ARCHITECTURE_ESCALATION_REQUIRED` rather than inventing it here.

- [ ] **Step 5: Run composition GREEN**

```bash
cargo +1.94.0 test --locked -p oteryn-game-server --test gameplay_server_seam composition_ -- --nocapture
cargo +1.94.0 run --locked -p oteryn-game-server -- --smoke
```

Expected: PASS; missing production configuration remains fail-closed.

- [ ] **Step 6: Commit Task 5**

```bash
git add apps/game-server/src/lib.rs apps/game-server/src/main.rs apps/game-server/tests/gameplay_server_seam.rs
git commit -m "feat(server): compose gameplay server seam"
```

---

### Task 6: Real Local TCP/TLS Tier 1 Journey

**Files:**
- Modify: `apps/game-server/tests/gameplay_server_seam.rs`

**Interfaces:**
- Consumes: exact production listener/TLS/framing/connection/FND-04/Durability path created in Tasks 1-5.
- Produces: physical Tier 1 evidence for supported bootstrap/admission and resume/reconnect paths on the exact candidate.

- [ ] **Step 1: Write the full bootstrap/admission Tier 1 test**

Start the real transport on `127.0.0.1:0` with caller-supplied non-shipping TLS material, connect using a real TLS client configured for TLS 1.3 + ALPN `oteryn-game/1`, send a correctly framed `ClientBootstrap`, and assert the server returns a correctly framed `ServerAccepted` only after canonical admission authority commits.

The test must observe a canonical authoritative result, not merely “socket stayed open”.

- [ ] **Step 2: Run bootstrap Tier 1 RED/GREEN**

```bash
cargo +1.94.0 test --locked -p oteryn-game-server --test gameplay_server_seam tier1_bootstrap_ -- --nocapture
```

Repair only within allocated paths until PASS.

- [ ] **Step 3: Write the resume/reconnect Tier 1 test**

Drive a real admitted session through transport loss/control-loss setup supported by current Foundation fixtures, reconnect on a fresh TLS connection, send `ClientResume`, exercise current durable PREPARE/reconcile/final revalidation as required, and assert `ServerResumeAccepted` is emitted only for the current candidate generation/authority.

Add a stale-generation sibling that receives a protocol/typed rejection and never reacquires controller authority.

- [ ] **Step 4: Run reconnect Tier 1 RED/GREEN**

```bash
cargo +1.94.0 test --locked -p oteryn-game-server --test gameplay_server_seam tier1_reconnect_ -- --nocapture
```

Expected: PASS on the real socket/TLS path.

- [ ] **Step 5: Run the whole integration target**

```bash
cargo +1.94.0 test --locked -p oteryn-game-server --test gameplay_server_seam -- --nocapture
```

Expected: all physical and negative Server Seam tests PASS with no ignored test standing in for required evidence.

- [ ] **Step 6: Commit Task 6**

```bash
git add apps/game-server/tests/gameplay_server_seam.rs
git commit -m "test(server): prove gameplay seam tier1 boundary"
```

---

### Task 7: Affected Validation and Authority Finding-Family Sweep

**Files:**
- Modify only if an accepted finding requires a repair inside the current allowlist.
- Update: `docs/agents/tasks/active/OTV2-20260904-gameplay-server-seam.md` before material freeze with exact evidence/dispositions that are known before the final commit.

**Interfaces:**
- Consumes: coherent Server Seam candidate from Tasks 1-6.
- Produces: deterministic affected validation, whole-diff adversarial review and a stable review candidate.

- [ ] **Step 1: Run formatting and focused package validation**

```bash
cargo +1.94.0 fmt --all -- --check
cargo +1.94.0 test --locked -p oteryn-game-server --test gameplay_server_seam
cargo +1.94.0 test --locked -p oteryn-game-server
cargo +1.94.0 clippy --locked -p oteryn-game-server --all-targets -- -D warnings
```

All must PASS.

- [ ] **Step 2: Run affected whole-workspace validation**

```bash
cargo +1.94.0 build --locked --workspace --all-targets
cargo +1.94.0 clippy --locked --workspace --all-targets -- -D warnings
cargo +1.94.0 test --locked --workspace
cargo +1.94.0 run --locked -p oteryn-synthetic-client-harness
```

All applicable commands must PASS. Do not report historical parent results as current-head evidence.

- [ ] **Step 3: Perform the finding-family sweep before material freeze**

Review the exact diff against every applicable cell:

```text
Authority invariants
  x fresh admission / direct resume / reconciled resume / final commit / admitted I/O / drain
  x missing / stale / mismatch / temporal / provenance-substitution / replay-concurrency operators
```

Explicitly inspect sibling APIs, protocol v1 paths, direct/reconciled flows, fenced durable writes, restart/retry/replay/concurrent replacement and PostgreSQL reload implications. If the Server Seam diff invalidates existing Durability physical evidence and proving it requires an unowned test path, stop with the smallest `SHARED_LEASE_REQUIRED` or architecture escalation rather than silently assuming coverage.

- [ ] **Step 4: Adversarial whole-diff self-review**

Check for:

- duplicate protocol/admission/session authority;
- record-derived “current” facts;
- allocation after unchecked peer length;
- unbounded channel/task/retry growth;
- TLS downgrade/plaintext fallback;
- wrong ALPN/profile or new wire IDs;
- queue byte/count accounting divergence;
- stale generation able to write/receive authority;
- shutdown retaining current controller authority incorrectly;
- accidental production endpoint/secret defaults;
- unrelated dependency upgrades or path expansion.

Every P0/P1 report must be verified on the exact head and either accepted+repaired test-first or rejected with exact evidence. Every P2 is `fixed`, `accepted` or `deferred` explicitly.

- [ ] **Step 5: Prepare task metadata before freeze and commit**

Record known validation/self-review/family-sweep evidence in the worker task without attempting to store the commit's own SHA. Then commit the metadata together with any final pre-freeze repair.

```bash
git add docs/agents/tasks/active/OTV2-20260904-gameplay-server-seam.md
git commit -m "docs(agents): qualify gameplay server seam candidate"
```

---

### Task 8: Exact-Head CI, Independent Review and Handoff

**Files:**
- No tracked-file change unless an accepted finding requires a test-first repair inside the allocation.

**Interfaces:**
- Consumes: stable exact PR head.
- Produces: immutable exact-head CI/review evidence and the canonical SERVER_SEAM handoff to Work.

- [ ] **Step 1: Publish the stable candidate without force/rebase/reset**

Push normal branch history and open/update one Draft PR for the lane. Verify the remote head SHA and exact changed-file allowlist from GitHub.

- [ ] **Step 2: Run/observe exact-head repository gates**

Require the current protected-main path-triggered gates for the exact head, including governance/repository policy, Rust Linux/Windows/supply-chain and canonical `game-gate` composition as current policy requires. If `main` advances and repository policy requires current-head refresh, merge `main` normally, rerun every invalidated validation layer and treat the resulting SHA as a new candidate generation.

- [ ] **Step 3: Obtain one genuinely independent exact-head deep review**

The review must explicitly cover protocol/session/admission/reconnect/fencing and transport/resource risks. This implementing session cannot count its own self-review as independent. External AI review remains advisory; repository gates and protected integration remain authoritative.

- [ ] **Step 4: Reconcile findings correctly**

For each material report, first verify applicability/correctness against the reviewed SHA. Accepted P0/P1 -> fresh RED -> minimal GREEN -> family sweep -> affected revalidation -> a new representative independent review if the material repair invalidated the prior one. Verified rejection with exact evidence preserves the candidate. P2 receives explicit disposition.

Do not create a no-op commit merely to retrigger CI/review.

- [ ] **Step 5: Return the canonical handoff without self-merging**

Return exactly current live evidence:

```yaml
lane: SERVER_SEAM
issue: 247
task_id: OTV2-20260904-gameplay-server-seam
admission_main_sha: <allocation merge SHA used to create worker branch>
integration_main_sha: <fresh main SHA used for integration preflight>
branch: agent/otv2-gameplay-server-seam-01
pr: <worker PR number>
final_head_sha: <exact remote PR head>
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
e2e: <real Tier 1 evidence>
self_review: <exact-head verdict>
independent_review: <exact-head independent verdict>
architecture_escalation: null
unresolved_findings: []
recommended_control_plane_action: integrate
next_action: Work independently verifies exact head/main relation/checks/review/threads/paths and integrates through protected controls if every predicate remains proven
```

If any predicate is not proven, use the corresponding truthful non-ready state rather than `READY_FOR_INTEGRATION`.
