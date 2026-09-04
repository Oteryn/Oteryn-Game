# Production Gameplay Server Seam Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans task-by-task. Every implementation task below is fresh RED -> minimal GREEN.

**Goal:** Add the smallest accepted production TCP/TLS gameplay entry seam that consumes canonical Foundation/Durability authority, remains fail-closed for unallocated gameplay, enforces registered hard limits, and proves the seam itself through a real local production-path TCP/TLS integration journey.

**Evidence boundary:** The Server Seam must make the later ADR-0007 physical Tier 1 journey executable, but it does **not** own or declare QA Tier 1 `PROVEN`. Formal Tier 1/Tier 2 evidence remains `NOT_EVALUATED` until a separately allocated QA lane runs after the Server Seam is merged. The local socket/TLS tests in this plan are Server Seam implementation/integration evidence only.

**Architecture:** Foundation remains the sole owner of protocol parsing, registered wire semantics, admission/GameSession/reconnect/fencing authority. `gameplay_transport` performs bounded TCP/TLS/framing and delegates authority decisions to the current Foundation/FND-04/Durability consumers. Configuration and TLS material are caller-supplied; this lane selects no production endpoint, certificate, key, secret or deployment topology.

**Spec:** `docs/architecture/reviews/OTERYN_GAME_PRODUCTION_GAMEPLAY_SERVER_SEAM_PLAN_2026-08-24.md`

## Verified prerequisites at allocation admission

- Issue #115 production FND-04 verifier/consumer is completed; implementation PR #151 merged as `2d0e951ce37c2e28773c22966bb816c00bebaa0a` and the archived task is `completed_released`.
- Issue #116 is closed and the current Resource Limits Registry contains the accepted Server Seam NET03 ceilings.
- The Durability prerequisite is terminal on protected `main`; current coordinator checkpoints record PR #252 plus archival PR #290 and released ownership.
- Server Seam preparation #96 / PR #117 is merged and defines the exact conditional topology and shared paths used here.
- These facts MUST be re-read from protected `main` when the worker branch is actually released. A changed or conflicting prerequisite is a control-plane blocker, not an assumption.

## Global constraints

- Execute only after `OTV2-20260904-gameplay-server-seam-allocation` merges and Work reads the exact merge SHA from protected `main`.
- Create `agent/otv2-gameplay-server-seam-01` from exactly that allocation merge SHA.
- Owned paths are exactly those in `docs/agents/tasks/active/OTV2-20260904-gameplay-server-seam.md`; no implicit path expansion.
- Preserve protocol major `1`, transport profile `1`, TLS 1.3 and ALPN `oteryn-game/1`.
- Preserve BE32 framing, the 1,048,576-byte FND-02 frame maximum and every smaller message-specific hard limit.
- Preserve registered Server Seam maxima: 256 pre-admission connections, 64 handshake/auth work units, 64 outbound queue entries/session, 1,048,576 outbound queued bytes/session, 8 pending writes/session and 256 drain tasks/batch.
- No new protocol/event/state/capability/stable numeric ID, gameplay semantics, Reference fact, permanent Content format or production deployment choice.
- No second admission/GameSession/CharacterLease/reconnect/durability authority.
- `workspace-boundaries.toml`, workflows, rulesets, stable registries and architecture/contracts are not allocated.
- High-risk authority work follows `AuthorityInvariant × ConsumerBoundary × MutationOperator`, uses independently current facts at each authority-consuming boundary, and changes one invariant per negative case.
- Open Dependabot #259/#260/#261 are non-owning Cargo candidates; re-read them before the first Cargo write and before integration. Do not absorb unrelated upgrades.

## File structure

- `apps/game-server/src/foundation/protocol.rs` — minimum Foundation-owned crate-internal typed bootstrap/resume extraction and registered-only server encoders.
- `apps/game-server/src/gameplay_transport/tcp_tls.rs` — TCP/TLS 1.3/ALPN/profile-1 handshake and bounded BE32 frame I/O only.
- `apps/game-server/src/gameplay_transport/connection.rs` — connection-local state machine delegating admission/reconnect to canonical Foundation/Durability authority.
- `apps/game-server/src/gameplay_transport/mod.rs` — configuration/lifecycle composition and registered resource budgets.
- `apps/game-server/src/lib.rs` — compose exactly one transport seam without moving gameplay authority.
- `apps/game-server/src/main.rs` — explicit configuration-driven startup; preserve `--smoke`; no hard-coded production endpoint/TLS material.
- `apps/game-server/Cargo.toml`, `Cargo.toml`, `Cargo.lock` — only minimum direct TLS/transport dependency/features required by the accepted profile.
- `apps/game-server/tests/gameplay_server_seam.rs` — real loopback production-path seam integration plus negative/resource/authority/backpressure/shutdown evidence. It is not the QA Tier 1 evidence envelope.

The Foundation bridge MUST remain `pub(crate)` or narrower unless an already-canonical current consumer proves wider visibility is required. A new externally public API/schema/wire semantic is `ARCHITECTURE_ESCALATION_REQUIRED` before mutation.

---

## Task 1 — Foundation typed consumer bridge and registered server encoding

**Files**
- Modify: `apps/game-server/src/foundation/protocol.rs`
- Test: existing protocol unit tests and `apps/game-server/tests/gameplay_server_seam.rs`

- [ ] **1.1 RED — typed bootstrap/resume extraction**

Add focused tests that decode canonical `ClientBootstrap`/`ClientResume` envelopes and require crate-internal typed views without transport-side raw protobuf reinterpretation.

Negative cases must include, independently where applicable:

- wrong message type;
- missing required field;
- duplicate singular field;
- malformed/unknown field behavior required by current FND-02 semantics;
- oversized admission/reconnect/build material;
- invalid UUID/capability shape;
- wrong protocol major;
- wrong transport profile;
- illegal direction/phase/generation.

Run:

```bash
cargo +1.94.0 test --locked -p oteryn-game-server foundation::protocol -- --nocapture
```

Expected RED: the typed consumer bridge does not yet exist.

- [ ] **1.2 GREEN — minimum borrowed typed views**

Implement only bounded crate-internal extraction using the existing Foundation parser/validator helpers. Return borrowed data where possible. Do not add a second parser/schema or expose caller-selected numeric message IDs.

- [ ] **1.3 RED — server encoder golden/cross-oracle evidence**

Require registered-only encoding for `ServerAccepted`, `ServerResumeAccepted` and `ProtocolError`.

Tests MUST include:

- canonical/golden bytes derived from the accepted `foundation.proto` semantics;
- an independent cross-oracle or otherwise non-self-referential wire check, so encode->the-same-decoder alone is not sufficient;
- direction/phase/generation/server-sequence legality;
- message/global limits;
- proof that no generic unregistered numeric message ID can be supplied.

- [ ] **1.4 GREEN — registered-only encoders**

Implement only those three existing Foundation server message families with checked lengths and existing typed IDs/values.

Run the focused protocol tests again; expected PASS.

- [ ] **1.5 Commit Task 1**

```bash
git add apps/game-server/src/foundation/protocol.rs
git commit -m "feat(foundation): expose server seam protocol bridge"
```

---

## Task 2 — Bounded TCP/TLS framing boundary

**Files**
- Create: `apps/game-server/src/gameplay_transport/tcp_tls.rs`
- Modify: `apps/game-server/tests/gameplay_server_seam.rs`
- Modify under serialized lease: `apps/game-server/Cargo.toml`, `Cargo.toml`, `Cargo.lock`

- [ ] **2.1 Re-read shared Cargo ownership before mutation**

If any active non-Dependabot writer now owns a shared Cargo path, stop with `SHARED_LEASE_REQUIRED`. Dependabot candidates do not gain lane ownership, but their current state must be recorded for later reconciliation.

- [ ] **2.2 RED — frame boundaries before peer-sized allocation**

Require:

- length `0` rejected;
- `1_048_576` accepted only when the complete body exists;
- `1_048_577` rejected before body allocation/read;
- truncated prefix/body rejected deterministically;
- checked conversion/accounting, including relevant overflow cases.

Run:

```bash
cargo +1.94.0 test --locked -p oteryn-game-server --test gameplay_server_seam frame_ -- --nocapture
```

Expected RED: transport framing is absent.

- [ ] **2.3 Add minimum exact-pinned TLS dependencies**

Select only Rust-1.94-compatible dependencies/features required for Tokio TCP + TLS 1.3 + ALPN and non-shipping test certificate loading. Follow workspace exact-pinning/supply-chain policy. Do not take unrelated bot upgrades.

- [ ] **2.4 GREEN — bounded TLS/profile-1 transport**

Required order:

```text
pre-admission slot
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

- [ ] **2.5 RED/GREEN — TLS/ALPN/plaintext failures**

Using non-shipping loopback TLS material, prove:

- TLS 1.3 + exact ALPN succeeds;
- missing/wrong ALPN fails before admission;
- plaintext never reaches Foundation admission;
- malformed/truncated/oversized frames fail closed.

Run:

```bash
cargo +1.94.0 test --locked -p oteryn-game-server --test gameplay_server_seam frame_ -- --nocapture
cargo +1.94.0 test --locked -p oteryn-game-server --test gameplay_server_seam tls_ -- --nocapture
```

Expected PASS.

- [ ] **2.6 Commit Task 2**

```bash
git add Cargo.toml Cargo.lock apps/game-server/Cargo.toml apps/game-server/src/gameplay_transport/tcp_tls.rs apps/game-server/tests/gameplay_server_seam.rs
git commit -m "feat(server): add bounded TLS transport framing"
```

---

## Task 3 — Authority-safe connection state machine

**Files**
- Create: `apps/game-server/src/gameplay_transport/connection.rs`
- Modify: `apps/game-server/tests/gameplay_server_seam.rs`

- [ ] **3.1 RED — pre-admission legality**

Unknown, malformed, server-to-client, phase-invalid and `ClientCommand`-before-admission messages must fail before FND-04, GameSession or durable mutation.

- [ ] **3.2 RED — fresh-admission authority before mutation**

Use the current production FND-04 verifier/consumer seam and current-authority evidence. Prove one invariant per negative case for at least:

- malformed/invalid authentication material;
- expired material;
- replayed material;
- wrong binding;
- stale/missing current authoritative evidence;
- account/character/world/lease mismatch;
- concurrent/replayed fresh admission cannot create two sessions.

The positive order is fixed:

```text
validated ClientBootstrap
-> production FND-04 verifier/consumer
-> independently current authoritative facts
-> canonical AdmissionAuthority commit
-> resulting GameSession/current ConnectionGeneration
-> attach transport
-> ServerAccepted
```

Transport-local state must not become admitted before canonical commit succeeds.

Run:

```bash
cargo +1.94.0 test --locked -p oteryn-game-server --test gameplay_server_seam admission_ -- --nocapture
```

Expected RED before implementation, then PASS after minimal GREEN.

- [ ] **3.3 RED — reconnect/resume authority family**

Cover independently:

- expected GameSession mismatch;
- account/character/world mismatch;
- same-world runtime-scope mismatch;
- stale/mismatched predecessor connection generation;
- stale ownership generation/authority fence;
- stale/missing control-loss epoch/liveness fact;
- candidate transport/binding mismatch;
- persisted-record provenance substituted for independently current facts;
- PREPARED/reconciled path and final COMMIT revalidation;
- replay of terminal outcomes without reacquiring current controller authority;
- concurrent replacement/stale attempt distinction.

Each negative case changes exactly one applicable invariant while unrelated facts remain semantically valid.

- [ ] **3.4 GREEN — consume current Foundation/Durability replacement flow**

Never derive supposedly current session/lease/runtime/generation/ownership facts from immutable reconnect records. Resolve current facts through canonical Foundation sources at the exact consuming boundary and pass them to the current durable flow.

Run:

```bash
cargo +1.94.0 test --locked -p oteryn-game-server --test gameplay_server_seam admission_ -- --nocapture
cargo +1.94.0 test --locked -p oteryn-game-server --test gameplay_server_seam reconnect_ -- --nocapture
```

Expected PASS.

- [ ] **3.5 Commit Task 3**

```bash
git add apps/game-server/src/gameplay_transport/connection.rs apps/game-server/tests/gameplay_server_seam.rs
git commit -m "feat(server): bind transport to admission authority"
```

---

## Task 4 — Registered resource budgets, backpressure and drain

**Files**
- Create: `apps/game-server/src/gameplay_transport/mod.rs`
- Modify: `apps/game-server/src/gameplay_transport/connection.rs`
- Modify: `apps/game-server/tests/gameplay_server_seam.rs`

- [ ] **4.1 RED — exact hard-limit boundaries**

Prove max accepted, max+1 rejected/backpressured before partial mutation, plus relevant checked overflow for:

```text
pre-admission connections: 256 / 257
handshake/auth work: 64 / 65
outbound queue entries/session: 64 / 65
outbound queue bytes/session: 1,048,576 / +1
pending writes/session: 8 / 9
drain tasks/batch: 256 / excess bounded into later work
```

Run:

```bash
cargo +1.94.0 test --locked -p oteryn-game-server --test gameplay_server_seam resource_ -- --nocapture
```

Expected RED before lifecycle orchestration exists.

- [ ] **4.2 GREEN — bounded primitives only**

Use bounded semaphores/queues/write slots/drain batches. No option may raise or disable an absolute registered maximum. One slow client may consume only its own bounded queue/write budget; no unbounded spawn/channel/retry-until-success path.

- [ ] **4.3 RED/GREEN — shutdown/drain authority preservation**

Prove shutdown:

- stops accepts/new admission;
- drains or explicitly resolves already-authoritative reserved transport work according to canonical Foundation lifecycle semantics rather than silently dropping it;
- releases bounded transport-local work/permits;
- never transfers or resurrects stale controller/session authority;
- never turns drain into unbounded tasks.

Run:

```bash
cargo +1.94.0 test --locked -p oteryn-game-server --test gameplay_server_seam backpressure_ -- --nocapture
cargo +1.94.0 test --locked -p oteryn-game-server --test gameplay_server_seam shutdown_ -- --nocapture
```

Expected PASS.

- [ ] **4.4 Commit Task 4**

```bash
git add apps/game-server/src/gameplay_transport/mod.rs apps/game-server/src/gameplay_transport/connection.rs apps/game-server/tests/gameplay_server_seam.rs
git commit -m "feat(server): bound gameplay transport lifecycle"
```

---

## Task 5 — Library/executable composition and fail-closed unsupported gameplay

**Files**
- Modify: `apps/game-server/src/lib.rs`
- Modify: `apps/game-server/src/main.rs`
- Modify: `apps/game-server/tests/gameplay_server_seam.rs`

- [ ] **5.1 RED — composition invariants**

Require:

- `--smoke` still succeeds without binding gameplay;
- absent configuration opens no listener and remains fail-closed;
- invalid/missing TLS material never falls back to plaintext;
- caller-supplied loopback endpoint/test TLS material starts the exact production transport module in integration tests;
- after successful admission, an unsupported/unregistered `ClientCommand` fails closed with zero command reservation/domain mutation and no invented message/capability ID.

Run:

```bash
cargo +1.94.0 test --locked -p oteryn-game-server --test gameplay_server_seam composition_ -- --nocapture
cargo +1.94.0 run --locked -p oteryn-game-server -- --smoke
```

- [ ] **5.2 GREEN — configuration-driven composition only**

Compose exactly one transport seam in `lib.rs`. `main.rs` may enter serving only from explicit valid configuration input. Do not add a default production address, port, certificate path, private key, secret name or deployment topology.

If making `main` production-operable requires a new product-wide configuration contract, return `ARCHITECTURE_ESCALATION_REQUIRED`; the physical integration test may use library composition with caller-supplied loopback/test material.

Run the composition tests and smoke command again; expected PASS.

- [ ] **5.3 Commit Task 5**

```bash
git add apps/game-server/src/lib.rs apps/game-server/src/main.rs apps/game-server/tests/gameplay_server_seam.rs
git commit -m "feat(server): compose gameplay server seam"
```

---

## Task 6 — Real local production-path seam integration

**Files**
- Modify: `apps/game-server/tests/gameplay_server_seam.rs`

**Evidence rule:** These tests traverse the actual Server Seam implementation and are required implementation/integration evidence. They do **not** mark ADR-0007 QA Tier 1/Tier 2 `PROVEN`.

- [ ] **6.1 RED/GREEN — bootstrap/admission physical seam test**

Start the real transport on `127.0.0.1:0` with caller-supplied non-shipping TLS material. Connect via a real TLS client using TLS 1.3 + ALPN `oteryn-game/1`, send a correctly framed `ClientBootstrap`, and require a correctly framed `ServerAccepted` only after canonical admission authority commits.

The assertion must observe the authoritative GameSession/current-generation outcome, not merely an open socket.

Run:

```bash
cargo +1.94.0 test --locked -p oteryn-game-server --test gameplay_server_seam physical_bootstrap_ -- --nocapture
```

- [ ] **6.2 RED/GREEN — resume/reconnect physical seam test**

Drive supported transport-loss/control-loss setup, reconnect over a fresh real TLS connection, send `ClientResume`, exercise current durable PREPARE/reconcile/final-current-authority validation, and require `ServerResumeAccepted` only for the current candidate generation. A stale-generation sibling must fail and never reacquire controller authority.

Run:

```bash
cargo +1.94.0 test --locked -p oteryn-game-server --test gameplay_server_seam physical_reconnect_ -- --nocapture
```

- [ ] **6.3 Run complete Server Seam integration target**

```bash
cargo +1.94.0 test --locked -p oteryn-game-server --test gameplay_server_seam -- --nocapture
```

Expected: all required physical/negative/resource tests PASS with no ignored substitute for required evidence.

- [ ] **6.4 Record truthful QA state**

The worker handoff MUST state:

```text
Server Seam physical integration: PROVEN for the exact candidate
ADR-0007 QA Tier 1: NOT_EVALUATED (separately allocated QA after merge)
ADR-0007 QA Tier 2: NOT_EVALUATED unless a later owning QA allocation proves it
```

- [ ] **6.5 Commit Task 6**

```bash
git add apps/game-server/tests/gameplay_server_seam.rs
git commit -m "test(server): prove gameplay seam physical boundary"
```

---

## Task 7 — Affected validation and authority finding-family sweep

**Files**
- Modify only allocated paths if an accepted finding requires repair.
- Update `docs/agents/tasks/active/OTV2-20260904-gameplay-server-seam.md` before material freeze with evidence/dispositions known before the final metadata commit.

- [ ] **7.1 Focused/package validation**

```bash
cargo +1.94.0 fmt --all -- --check
cargo +1.94.0 test --locked -p oteryn-game-server --test gameplay_server_seam
cargo +1.94.0 test --locked -p oteryn-game-server
cargo +1.94.0 clippy --locked -p oteryn-game-server --all-targets -- -D warnings
```

All must PASS.

- [ ] **7.2 Affected workspace validation**

```bash
cargo +1.94.0 build --locked --workspace --all-targets
cargo +1.94.0 clippy --locked --workspace --all-targets -- -D warnings
cargo +1.94.0 test --locked --workspace
cargo +1.94.0 run --locked -p oteryn-synthetic-client-harness
```

All applicable commands must PASS on the coherent candidate.

- [ ] **7.3 Finding-family sweep**

Review every applicable cell:

```text
Authority invariant
x fresh admission / direct resume / reconciled resume / final COMMIT / admitted I/O / drain
x missing / stale / mismatch / temporal / provenance-substitution / replay-concurrency operator
```

Explicitly inspect sibling APIs, protocol v1 paths, direct/reconciled paths, fenced durable writes, restart/retry/replay/concurrent replacement and PostgreSQL reload implications. Existing Durability tests may be reused only when the Server Seam diff has not invalidated their claim. A required unowned test path is `SHARED_LEASE_REQUIRED`, not implicit expansion.

- [ ] **7.4 Whole-diff adversarial self-review**

Check specifically for duplicate protocol/session authority, record-derived current facts, unchecked peer allocation, unbounded work, TLS downgrade/plaintext fallback, wrong ALPN/profile, new IDs, queue accounting drift, stale-generation authority, silent shutdown loss of authoritative reserved work, accidental production defaults, unrelated dependency upgrades and path expansion.

Every reported P0/P1 must be verified and either repaired test-first or rejected with exact evidence. Every P2 gets an explicit disposition.

- [ ] **7.5 Commit pre-freeze task evidence**

```bash
git add docs/agents/tasks/active/OTV2-20260904-gameplay-server-seam.md
git commit -m "docs(agents): qualify gameplay server seam candidate"
```

Do not create a commit solely to record that commit's own SHA.

---

## Task 8 — Exact-head CI, independent review and handoff

- [ ] **8.1 Publish one stable Draft PR without force/rebase/reset**

Verify remote head and exact changed-file allowlist from GitHub.

- [ ] **8.2 Require exact-head repository gates**

Require current path-triggered governance/repository, Rust Linux/Windows/supply-chain and canonical `game-gate` as applicable. If protected `main` advances and current policy requires refresh, integrate it with normal history-preserving Git, then rerun every invalidated layer on the new head.

- [ ] **8.3 Obtain one genuinely independent exact-head deep review**

The review must cover protocol/session/admission/reconnect/fencing, TLS/framing, resource accounting, shutdown and evidence ownership. The implementing session's self-review is not independent; green CI is not independent review.

- [ ] **8.4 Reconcile findings**

Accepted material P0/P1 -> fresh RED -> minimal GREEN -> family sweep -> affected validation -> new exact-head review where the risk-bearing repair invalidated prior evidence. Do not make no-op retrigger commits.

- [ ] **8.5 Return canonical handoff without self-merging**

```yaml
lane: SERVER_SEAM
issue: 247
task_id: OTV2-20260904-gameplay-server-seam
admission_main_sha: <allocation merge SHA used to create worker branch>
integration_main_sha: <fresh protected main used for integration preflight>
branch: agent/otv2-gameplay-server-seam-01
pr: <worker PR>
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
server_seam_physical_integration: <exact-head real TCP/TLS bootstrap+resume evidence>
qa_tier1: NOT_EVALUATED
qa_tier2: NOT_EVALUATED
self_review: <exact-head verdict>
independent_review: <exact-head independent verdict>
architecture_escalation: null
unresolved_findings: []
recommended_control_plane_action: integrate
next_action: Work independently verifies exact head/main relation/checks/review/threads/paths and integrates through protected controls if every predicate remains proven
```

Use `READY_FOR_INTEGRATION` only when every predicate is actually proven. The worker never merges its own PR.