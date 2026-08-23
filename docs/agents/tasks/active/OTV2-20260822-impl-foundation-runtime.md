# OTV2-20260822-impl-foundation-runtime

```yaml
task_id: OTV2-20260822-impl-foundation-runtime
title: Implement native protocol runtime admission foundation
mode: IMPLEMENT
status: review_pending
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/otv2-impl-foundation-runtime-01
pr: 59
base_sha: fd39c6aa026e82062a8b29af24811d467c115f19
allocation_merge_sha: 33cec30b8075c73290d7d76e9f59df4701771650
owner: chat-github-20260822-foundation-runtime
created_at: 2026-08-22T18:11:00+02:00
updated_at: 2026-08-23T17:12:00+02:00
execution_budget_minutes: 120
large_budget_reason: XHigh protocol/session/admission/fencing lane with mandatory independent exact-head review
owned_paths:
  - apps/game-server/src/foundation/**
  - docs/agents/tasks/active/OTV2-20260822-impl-foundation-runtime.md
shared_lease_paths:
  - apps/game-server/src/lib.rs
  - apps/game-server/Cargo.toml
  - Cargo.toml
  - Cargo.lock
  - workspace-boundaries.toml
public_contracts:
  - docs/architecture/FND-02_PROTOCOL_OTERYN_V1_CONTRACT.md
  - docs/architecture/FND-03_RUNTIME_EXECUTION_CONTRACT.md
  - docs/architecture/FND-04_IDENTITY_GAME_SESSION_ADMISSION_CHARACTER_LEASE_CONTRACT.md
  - docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json
  - docs/contracts/PROTOCOL_OTERYN_TRANSPORT_POLICY.json
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - docs/contracts/FOUNDATION_ERROR_VOCABULARY.md
  - docs/contracts/FOUNDATION_FAILURE_SCENARIOS.md
depends_on:
  - Oteryn-Game#45
  - Oteryn-Game#46
  - OTV2-20260818-impl-simulation
blocks:
  - OTV2-IMPL-CLIENT
  - OTV2-IMPL-DURABILITY
  - OTV2-IMPL-ABILITY
  - OTV2-IMPL-INTERACTION
  - OTV2-IMPL-AI
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories: []
```

## Outcome

Deliver the smallest real server foundation module and composition seam implementing accepted protocol framing/state, authoritative runtime fencing and admission/reconnect primitives without gameplay command/state registrations or persistence semantics.
## Architecture and source of truth

- `PROVEN`: exact allocation merge is `33cec30b8075c73290d7d76e9f59df4701771650`; worker branch starts from post-bind `main@fd39c6aa026e82062a8b29af24811d467c115f19`.
- `PROVEN`: `apps/game-server` is the existing production composition root; this lane must not invent a new crate topology.
- `PROVEN`: gameplay command/state ID registries remain outside this lane.
- `PROVEN`: shared composition/workspace lease belongs to FOUNDATION first.
- `UNKNOWN` values or semantics not frozen by accepted registries/contracts fail closed and are not guessed.

## Acceptance criteria

- [x] TDD: every production behavior is preceded by a focused test observed failing for the intended reason.
- [x] Bounded protocol frame envelope rejects truncated, oversized and unknown foundation message classes before peer-sized allocation.
- [x] Command/session connection-generation state rejects duplicate gaps, stale generations and invalid progression deterministically.
- [x] Runtime ownership primitives enforce one current generation and reject stale owner work/timers/results.
- [x] Admission/reconnect primitives model fresh admission, GameSession/CharacterLease binding and fail-closed stale/replay cases without live Platform/DB authority.
- [x] Foundation error vocabulary maps failures without leaking secrets or inventing gameplay semantics.
- [x] `apps/game-server` composition consumes the foundation module while preserving fail-closed gameplay availability until owning gameplay registrations exist.
- [x] Focused tests, full workspace tests, format and Clippy pass at final head.
- [ ] Mandatory genuinely independent exact-head review has zero unresolved material findings before merge.

## Excluded scope

No movement/combat/inventory/chat/content IDs; no PostgreSQL schema; no production traffic/deployment; no Canary compatibility; no final post-15s recovery policy; no permanent resource values not already registered.
## Implementation plan

1. Read exact protocol/transport/resource registries and existing server composition; name only implementation decisions already permitted by accepted contracts.
2. RED: add focused module tests for bounded framing, command ordering/generation fencing, runtime ownership generation and admission/reconnect state transitions.
3. GREEN: implement minimal `apps/game-server/src/foundation/**` types/state machines needed to pass those tests with no network listener side effects.
4. Integrate only required dependencies/composition through the active shared lease; if canonical protobuf codegen requires an unallocated path such as `apps/game-server/build.rs`, stop that mutation and update coordinator allocation first.
5. Add negative/golden coverage and run full workspace validation; then freeze head, self-review, independent review and exact-head CI.

## Validation

### Focused
- command/run: `cargo test -p oteryn-game-server foundation`
- result: PASS - 70 foundation tests; 0 failed after the full-session reconnect COMMIT and non-copyable runtime-ordinal issuer repairs

### Component/integration
- command/run: `cargo test -p oteryn-game-server`
- result: PASS - 73 package tests; 0 failed; `cargo +1.94.0 test --locked --workspace` PASS; locked strict workspace Clippy PASS; changed-file `rustfmt --check` PASS

### E2E
- scenario: Tier 1 wire journey only when a real merged production transport seam exists; otherwise `NOT_EVALUATED` with exact blocker.
- result: `NOT_EVALUATED` - no merged production transport listener/client-entry seam exists in the allocated composition; this lane intentionally adds no listener side effects.

### Exact-head CI
- third repaired code head: `ed3bc851955974b471d3d9544189e0e8bd1f6456`
- superseded FND-ID PR head: `1ab4cd321a551597b8cdda20845af216790654c9`
- superseded exact-head evidence on `1ab4cd3...`: Merge Gate #196 / run `32602950356`, Architecture Semantic Audit #170 / `32602950354`, Agent Governance #232 / `32602950353`, Merge Authority Audit #148 / `32602950360` - SUCCESS before the issuer-boundary self-review repair
- superseded issuer-boundary PR head: `959d083cee9a03a71ac8d5eca54fc897ed372189`
- superseded exact-head evidence on `959d083...`: Merge Gate #204 / run `32626162871` SUCCESS plus Architecture Semantic Audit #176 / `32626124677`, Agent Governance #240/#241, Merge Authority Audit #153; later #203/#205 were cancelled lifecycle reruns
- superseded ingress-matrix PR head: `bfd43c6e24e75534cc574df8b7161df736939f4d`
- superseded exact-head evidence on `bfd43c6...`: Merge Gate #212 / run `32627380787` SUCCESS including Windows/Linux/CodeQL/supply-chain/final validate; Agent Governance #248, Architecture Semantic Audit #179 and Merge Authority Audit #156 SUCCESS
- superseded independent review on `bfd43c6...`: separate non-authoring Qwen session PASS / 0 material findings, but owner-authorized Codex subsequently returned REQUEST_CHANGES with 1 P1 + 2 P2, so the head is not merge evidence
- superseded Codex-repair PR head: `89cfd20d2e6fe02556db7777ea0e6e895bc15701`
- superseded exact-head evidence on `89cfd20...`: Agent Governance #252/#253, Architecture Semantic Audit #181 and Merge Authority Audit #158 were SUCCESS; Merge Gate #216 had green scope/governance/dependency/policy/CodeQL/supply-chain/Linux with Windows still running when the material final P2 invalidated the head
- final pending-PREPARE reconciliation tree: pending freeze commit
- classification: high-risk foundation
- result: pending fresh exact-head CI; all `89cfd20...` CI/review evidence is superseded by the material final P2 repair

## Self-review

- first independent-review repair cycle: 5 material issues (3 P1, 2 P2) plus one terminal-replay self-review edge repaired
- second independent review on `815fe5cc8da0633b67cab7840b1f60cb2137df78`: 2 P1 repaired (post-payload server-sequence commit; updatable current runtime-ownership fence)
- third independent review on `c72bb0ce9cd7e982f0720e47571c173d178b5465`: 1 P1 - a committed reconnect-attempt result could replay success after a later transport/generation superseded it
- third repair: shared committed-attempt reconciliation now returns success only when both `current_transport` and current `connection_generation` still match the committed attempt; PREPARE and COMMIT use the same helper
- additional same-invariant self-review finding: `mark_unexpected_control_loss` retained the lost physical transport identity, which could still satisfy the replay helper before another winner appeared; RED reproduced this and both unexpected loss and terminalization now clear `current_transport`
- TDD evidence: superseded A->B replay RED returned `Ok(ConnectionGeneration(2))` instead of `StaleConnection`; lost-current-transport RED retained `Some(200)` instead of `None`; both regressions are GREEN after repair
- FND-ID self-review findings: raw `[u8; 16]` semantic IDs violated strong typing/UUIDv7 trust-boundary validation; successful GameSessionId reuse was also possible
- FND-ID repair: distinct `GameSessionId`/`CharacterId`/`WorldId`/`ChannelId` UUIDv7+RFC-variant wrappers and typed `CommandRef`/session/lease APIs
- final issuer-boundary self-review finding: `FreshAdmissionFacts` still carried a GameSessionId and the precommit candidate set reserved it before incumbent arbitration, contradicting FND-04A and the owner-accepted GameSessionId issuer baseline: Platform/grant material is not GameSessionId and the canonical ID is game-domain issued only at successful final admission
- final TDD evidence: desired `commit_fresh(..., issue_game_session_id)` test was observed RED because the issuer seam did not exist; after repair admission is 18/18 and issuer call count remains zero for incumbent/replay rejection while the same unconsumed grant can retry after the blocker clears
- issuer-boundary repair: GameSessionId was removed from `FreshAdmissionFacts`; `commit_fresh` invokes a trusted game-domain atomic identity seam only after local incumbent/runtime preconditions; durable GrantNonce consume/replay and never-reused GameSessionId reservation remain outside process-local session history
- fourth independent review on `959d083cee9a03a71ac8d5eca54fc897ed372189`: 1 P1 - nested `ClientBootstrap.admission_material` / `ClientResume.reconnect_material` exceeded the registered 16,384-byte limit while the 65,536-byte outer bootstrap limit still passed
- P1 TDD evidence: exact nested-material regression was observed RED with 16,385 bytes returning `Ok(WireEnvelopeView)` instead of `BootstrapLimitExceeded`; the zero-copy repair is GREEN
- same-class resource-matrix self-review then found three additional server-ingress gaps before the next freeze: client build-id 128/UTF-8 and capability count/sorted-unique, ClientCommand payload/expected revisions, and ResyncRequest domain count/uniqueness; all three focused regressions were observed RED before production changes and are GREEN after one bounded scanner repair
- final FND-02 repair: a zero-copy ingress validator enforces outer bootstrap 65,536 plus nested material 16,384, build-id 128/UTF-8, capability 128 sorted/unique, command payload 65,536, command expected revisions 64 unique, and resync domains 256 unique before deep parse/materialization; bounded domain sets never exceed the registered count
- matrix classification: server-receive repeated fields have specific ceilings lower than `FND02-ORDINARY-REPEATED-ENTRIES=4096`; parsed nested `StateRevision` is non-recursive and therefore below depth 32; server-outbound CommandResult/StateDelta/Snapshot limits remain covered by their existing producer/barrier primitives rather than peer-ingress allocation paths
- final Codex review on `bfd43c6e24e75534cc574df8b7161df736939f4d` found 1 P1 + 2 P2: protobuf field numbers above the legal 29-bit range could wrap through `as u32`; build-id >128 was incorrectly classified as `CAPACITY_EXCEEDED`; committed reconnect outcomes grew without a bound for a long-lived GameSession
- exact RED evidence: oversized field number `(2^32 + 1)` decoded as a valid bootstrap; build-id 129 returned `BootstrapLimitExceeded` instead of an INVALID_INPUT code; three reconnect commits retained 3 map entries; a prepared attempt invalidated by runtime-owner change could be reused
- superseded repair on `89cfd20...`/`af7b946...` used a numeric reconnect-attempt high-watermark to keep process-local history O(1); later review proved that compression invalid because FND-04B defines `ReconnectAttemptRef` as opaque operation identity rather than a monotonic generation
- current repair removes all numeric ordering/high-watermark semantics from reconnect attempt identity; production `ReconnectAttemptRef` is equality/hash only and historical idempotency is delegated to a trusted `ReconnectAttemptJournal` keyed by exact `(GameSessionId, ReconnectAttemptRef)`
- the journal seam atomically claims exact PREPARE identities, terminalizes unseen losing candidates, transitions the exact prepared operation to committed, and records supersession of invalidated prepared candidates; no production historical map/set or invented retention count/deadline is added to the Foundation kernel
- if an authoritative historical PREPARE cannot be reconstructed by the current process, the kernel fails closed as `ReconciliationUnavailable` instead of minting a second candidate; committed replay remains exact-current-transport/current-generation fenced
- the in-memory `HashMap` implementation exists only under `#[cfg(test)]` as a model of the external/durable authority seam, analogous to the existing fresh-identity test ledger; physical retention/rate/storage limits remain owned by the deferred DUR/OPS evidence rather than guessed here
- same-class memory self-review also removed unbounded process-local GrantNonce and historical GameSessionId sets from `AdmissionAuthority`; replay consume and never-reused GameSessionId reservation now belong to the trusted atomic game-domain fresh-identity commit seam, modeled by a test ledger without adding live DB authority
- fresh-history TDD evidence: the desired zero retained session-authority history was observed RED as a missing behavior; replay/no-reuse tests now exercise the trusted ledger seam while local incumbent checks still prevent calling it on rejected admission
- final Codex review on `89cfd20d2e6fe02556db7777ea0e6e895bc15701` found 1 P2: while newer attempt B was pending in PREPARE, retries of older terminally-superseded A were rejected early as generic `AttemptMismatch` before the high-watermark reconciliation path could return A's stable terminal disposition
- final P2 RED evidence: both `prepare_reconnect(A, ...)` and `commit_reconnect(A, ...)` during pending B returned `AttemptMismatch` instead of `StaleConnection`; the two focused regressions were observed failing independently
- superseded P2 repair: exact-current prepared retries still require the bound candidate transport while older known refs reconcile through stable historical disposition; that behavior is retained, but numeric high-watermark classification is removed
- Codex review of `11d1f89f62d5b94f63577287acf86934c3ab0318` found a new P1 still applicable after the merge-only `dbe7b50...`: a competing arbitrary `ReconnectAttemptRef(u64::MAX)` could poison the numeric high-watermark and permanently reject all later lower-valued opaque refs
- exact RED evidence: `opaque_reconnect_ref_value_cannot_poison_future_attempts` failed with `StaleConnection` when a low-valued new attempt followed the losing `u64::MAX` contender after the prepared winner committed and control was lost
- subsequent exact-head Codex review of `dbe7b50...` found another P1: `mark_unexpected_control_loss()` carried no observed transport/generation, so a delayed predecessor close could demote a healthy reconnected controller to `Reconnectable`
- exact RED evidence: `delayed_predecessor_loss_cannot_drop_reconnected_controller` reproduced a predecessor callback clearing `current_transport=Some(200)` to `None` after generation 2 was already current
- control-loss repair: every loss observation now carries exact transport identity plus `ConnectionGeneration`; only the current pair may mutate session state, while stale callbacks return `ControlLossDisposition::StaleIgnored` without touching authority; the exact-current loss still returns `Applied` and transitions to `Reconnectable`
- current GREEN evidence after both material repairs: Foundation 61/61 PASS; full game-server 64/64 PASS; `cargo test --workspace` PASS; strict workspace Clippy PASS; scoped fmt PASS; governance PASS; semantic scan finds no numeric high-watermark/no-arg loss callback remnants; diff-check PASS
- cross-process self-review additionally strengthened `ReconnectAttemptJournal::claim_prepared`: across one GameSession it atomically permits at most one distinct PREPARED attempt and terminalizes a competing distinct claim without disturbing the incumbent candidate
- exact-head Codex review of `a408d39502fd4df0eda8806b91d3bf7413c0b6ab` found 1 P1 + 1 P2 after the prior repair freeze: the fresh-admission durable seam committed only identity before local session activation, and `ReconnectAttemptRef` exposed no stable durable encoding for journal keys
- fresh-admission P1 RED evidence: `fresh_admission_lost_commit_response_can_reconstruct_the_same_session` simulated a durable consume/session-ID commit whose response was lost before local activation; retry failed as `GrantReplayed` while no local GameSession existed
- fresh-admission P1 repair: `FreshAdmissionCommit` is now the trusted atomic authority receipt containing the complete initial logical binding (`GameSessionId`, Character/World/Channel, lease/scope generations and connection generation 1); the trusted seam must return the same receipt after lost response/recovery, and the kernel validates it against current admission facts before reconstructing local ACTIVE state
- reconnect-ref P2 RED evidence: the focused stable-encoding test failed to compile because the public opaque ref exposed neither durable bytes nor decode; this proved an external journal adapter could not form a stable collision-free recovery key from the public API
- reconnect-ref P2 repair: exact refs now encode/decode as fixed 8-byte big-endian values; the API documentation explicitly states byte order is encoding only and grants no ordering/recency semantics
- current GREEN evidence after the `a408d395...` findings: Foundation 63/63 PASS; full game-server 66/66 PASS; `cargo test --workspace` PASS; strict workspace Clippy PASS; scoped fmt PASS; governance PASS; diff-check PASS; repair danger scan clean
- open material findings: both `a408d395...` findings are repaired locally; fresh independent exact-head review is still mandatory before claiming zero material findings
- verdict: PASS for self-review only; ready to freeze the final repair head

## Independent review

- required: YES - protocol/session/admission/fencing high-risk semantics
- first reviewed superseded head `9d5a251adb16076c3b0ebc50ae023677bf571894`: REQUEST_CHANGES - 3 P1 + 2 P2; repaired
- second reviewed superseded head `815fe5cc8da0633b67cab7840b1f60cb2137df78`: REQUEST_CHANGES - 2 P1; repaired
- third reviewed superseded head `c72bb0ce9cd7e982f0720e47571c173d178b5465`: REQUEST_CHANGES - 1 P1; repaired
- third P1 repair: committed-attempt result replay is current-channel/current-generation fenced; superseded transport/generation receives `StaleConnection` rather than a historical success
- owner authorization: explicit and bounded to PR #59 independent review plus autonomous repair/re-review/merge/closeout cycle
- superseded FND-ID product-code head: `99786226bf988840daa0bcde55d8f90f4d744561`
- superseded issuer-boundary PR head: `959d083cee9a03a71ac8d5eca54fc897ed372189` - REQUEST_CHANGES, 1 P1 nested ingress limit; repaired
- superseded ingress-matrix PR head: `bfd43c6e24e75534cc574df8b7161df736939f4d` - Qwen independent PASS 0 findings, then Codex REQUEST_CHANGES with 1 P1 + 2 P2; repaired
- superseded Codex-repair PR head: `89cfd20d2e6fe02556db7777ea0e6e895bc15701` - Codex REQUEST_CHANGES with 1 P2 pending-PREPARE reconciliation finding; repaired
- superseded concurrent-loser repair product head: `11d1f89f62d5b94f63577287acf86934c3ab0318`; Codex REQUEST_CHANGES with 1 P1 against numeric ordering of opaque `ReconnectAttemptRef`
- merge-only PR head before current local repair: `dbe7b50c7f5dad627c2a569b726c7e6a53f86b00` on `main@c4c407d096a3252fd2850abbd616944c97297ce6`; review reported two material P1s applicable to its admission code: numeric ordering of opaque reconnect refs and unfenced stale control-loss callbacks
- superseded repair product head: `7abe6fd5e737ce78d7d9fefa7ef69c230144de7d` (`fix(server): harden reconnect authority fencing`)
- reviewed exact head `a408d39502fd4df0eda8806b91d3bf7413c0b6ab`: REQUEST_CHANGES - 1 P1 atomic fresh-admission authority boundary + 1 P2 stable reconnect-ref encoding; both repaired locally with RED/GREEN evidence
- final product-code head: pending freeze commit containing the two `a408d395...` review repairs
- final PR head: pending same repair/evidence freeze commit; no documentation-only descendant will be added after qualification
- verdict: repaired tree locally validated; pending fresh protected exact-head CI and genuinely independent exact-head review

## Context checkpoint

```yaml
last_progress: Final exact-head Codex review of a408d395 found 1 P1 and 1 P2. P1: durable fresh admission committed only nonce/session identity before local ACTIVE binding, so a crash/lost response could strand a consumed grant with no reconstructable session; RED retry returned GrantReplayed. P2: opaque ReconnectAttemptRef had no stable public durable encoding; RED compile evidence showed no to_be_bytes/decode API. Local repair adds reconstructable FreshAdmissionCommit receipts and fixed 8-byte equality-key encoding with no ordering semantics. Final local gate: Foundation 63/63, game-server 66/66, workspace, strict Clippy, fmt, governance and diff-check PASS.
status: final_review_repair_ready_to_freeze
branch: agent/otv2-impl-foundation-runtime-01
head_sha: a408d39502fd4df0eda8806b91d3bf7413c0b6ab-plus-uncommitted-final-review-repair
pr: 59
blocker: repair must be committed/pushed as the new exact head, then protected exact-head CI and genuinely independent exact-head review must both be clean
owner_action_required: null
next_action: freeze/push the P1+P2 repair on the authorized branch, update PR #59 without moving the head again, resolve superseded review threads, request fresh exact-head independent review, and squash merge only after every exact-head gate is green.
```

## Handoff checkpoint — 2026-08-23T13:36:00+02:00

This checkpoint supersedes the preceding stale `Context checkpoint` for continuation purposes; GitHub live state remains authoritative.

- `PROVEN`: `main` is `c4c407d096a3252fd2850abbd616944c97297ce6`.
- `PROVEN`: Issue #53 is OPEN.
- `PROVEN`: PR #59 is OPEN, mergeable, not merged; branch `agent/otv2-impl-foundation-runtime-01` head is `c231b45d102e05807430f0a0b1d1f41ca9d2d4e6`.
- `PROVEN`: exact-head workflows are green: Agent Governance #267, Merge Gate #229, Merge Authority Audit #165 and Architecture Semantic Audit #188. Merge Gate #228 is cancelled/superseded.
- `BLOCKER P1` `PRRT_kwDOT8SzxM6be0V4`: a consumed fresh-admission grant can reconstruct its initial receipt as ACTIVE after process recovery even when that GameSession later became terminal. Durable authority must expose/revalidate current lifecycle state before reconstruction so terminal sessions cannot revive.
- `BLOCKER P1` `PRRT_kwDOT8SzxM6be0V5`: reconnect COMMIT does not atomically revalidate current CharacterLease and RuntimeScope ownership generations inside the trusted commit seam. An external fence advance between PREPARE and COMMIT can therefore allow a stale authority switch.
- `MERGE`: prohibited until both P1s are repaired, fresh exact-head CI is green and a genuinely independent exact-head review reports zero material findings.
- `LOCAL CACHE NOTE`: an older dirty Foundation worktree was observed at `7d0a493...` with 24 uncommitted test lines against the obsolete pre-fenced loss API. Do not fold that local diff into the current branch without re-deriving it against `c231b45...`.
- `CONTINUATION`: start from exact remote head `c231b45...`; add focused RED regressions for both P1s, repair the trusted fresh-admission and reconnect-commit authority seams, run the full Foundation/game-server/workspace/Clippy/fmt/governance/diff gate, commit and push a new exact head, then rerun protected CI and genuinely independent exact-head review.


## Continuation checkpoint — 2026-08-23T15:00:28+02:00

This checkpoint supersedes the 13:36 handoff for continuation. Historical review/CI evidence above remains intentionally retained as superseded evidence.

- `PROVEN`: work resumed from remote PR #59 head `b94afb5fbc0446e659b4f0937ec7b9d086b9de1a` in isolated worktree `C:\Users\barte\oteryn-impl-foundation`; no shared/main worktree mutation was used.
- `RED/P1 fresh terminal recovery`: after durable fresh admission, terminalization and process restart, retrying the consumed grant returned `Ok(GameSession { state: Active, generation: 1, ... })` instead of `Err(GrantReplayed)`.
- `GREEN/P1 fresh terminal recovery`: `FreshAdmissionAuthoritySnapshot` now requires the trusted fresh seam to return the committed binding plus current authoritative lifecycle state; non-`Active` durable state cannot reconstruct the old session.
- `RED/P1 reconnect lease fence`: an authoritative CharacterLease advance after PREPARE returned `Ok(ConnectionGeneration(2))` instead of `Err(StaleLease)`.
- `RED/P1 reconnect runtime fence`: an authoritative RuntimeScope ownership-generation advance after PREPARE returned `Ok(ConnectionGeneration(2))` instead of `Err(StaleRuntime)`.
- `GREEN/P1 reconnect fences`: `ReconnectAttemptJournal::commit_prepared` now receives expected typed CharacterLease and scope ownership generation and must compare them at the same linearization point as PREPARED -> COMMITTED; mismatch terminally supersedes the candidate. The test journal models journal state and authoritative fences in one shared authority cell.
- `SELF-REVIEW RED`: lifecycle state alone was insufficient after a successful reconnect: process recovery could replay the original fresh grant and roll an `Active` session from connection generation 2 back to generation 1. The regression reproduced exactly that rollback.
- `SELF-REVIEW GREEN`: the durable fresh snapshot also carries current authoritative connection generation and reconstruction is allowed only when it still equals the original committed generation. The generation-rollback regression is GREEN.
- `PROVEN focused`: `cargo test -p oteryn-game-server foundation` -> 67 passed / 0 failed; `cargo test -p oteryn-game-server` -> 70 passed / 0 failed.
- `PROVEN canonical Rust gates`: Rust/Cargo 1.94.0; `metadata --locked`, architecture workspace check, `build --locked --workspace --all-targets`, strict `clippy --locked --workspace --all-targets -- -D warnings`, `test --locked --workspace`, synthetic client harness and changed-file `rustfmt --check` all PASS.
- `PROVEN governance`: `python tools/agents/validate_governance.py` PASS after this task-record update.
- `BASELINE/ENVIRONMENT`: workspace-wide `cargo +1.94.0 fmt --all --check` and `validate_repository_policy.py` are not usable as Windows-checkout evidence because untouched files are checked out with CRLF; repository-policy validation fails on untouched `LICENSE`. `git diff origin/main -- LICENSE` is empty and the same policy failure reproduces in the clean/base worktree. Earlier workspace fmt check passed while Rust sources were temporarily normalized to LF. Exact-head Linux merge-gate CI remains authoritative for these two baseline-sensitive checks.
- `PROVEN diff`: before task-record update, `git diff --check` PASS and the only product-code diff is `apps/game-server/src/foundation/admission.rs`.
- `MERGE`: still prohibited. Next steps are freeze commit + push, exact-head protected CI, resolution/re-review of the two P1 threads, then genuinely independent exact-head review with zero material findings. No documentation-only descendant may be added after qualification.

## Final exact-head repair checkpoint — 2026-08-23T16:10:24+02:00

This checkpoint supersedes the preceding continuation checkpoint for the current repair tree. Historical SHA/review/CI evidence above is retained only as superseded provenance.

- `PROVEN`: repair started from exact remote PR #59 head `681f9a924b899ff47f70f5073fd38b6cf12691e4` in isolated worktree `C:\Users\barte\Documents\ChatGPT\oteryn-governance-exec-20260819\Oteryn-Game-foundation-final`; baseline Foundation was 67/67 PASS before mutation.
- `INDEPENDENT REVIEW / SUPERSEDED HEAD`: owner-authorized Codex exact-head review of `681f9a9...` found two material issues: P1 reconnect COMMIT could publish `COMMITTED` after authoritative lifecycle/controller/generation changed between process-local checks and the journal linearization point; P2 `ScopeRuntimeFence: Clone + Copy` allowed duplicate `RuntimeExecutionOrdinal` issuance from copied fence state.
- `RED/P1`: focused regression changed authoritative durable session/controller state after PREPARE but before COMMIT; the old seam returned `Ok(ConnectionGeneration(2))` instead of `Err(StaleConnection)`.
- `GREEN/P1`: `ReconnectAttemptJournal<T>` now receives an exact `ReconnectCommitBinding<T>` at PREPARE/COMMIT containing predecessor generation, candidate generation, candidate transport, CharacterLease and RuntimeScope generation. The trusted COMMIT seam is required to revalidate reconnectable lifecycle/no-current-controller, exact predecessor/candidate/transport and lease/runtime fences at the PREPARED -> COMMITTED linearization point and atomically publish the candidate generation/transport authority. Process-local state becomes only a projection after that commit.
- `SAME-CLASS P1`: separate regressions prove an authoritative healthy-controller recovery without generation change fails as `StaleConnection`, and an authoritative terminal transition inside the COMMIT race fails as `Terminal`; neither advances the process-local connection generation.
- `RED/P2`: a compile-fail doctest copied `ScopeRuntimeFence` and successfully issued from both copies, so the test failed because the invalid program compiled.
- `GREEN/P2`: `ScopeRuntimeFence` is no longer `Clone` or `Copy`; `GameSession` is likewise non-Clone because it owns that issuer. The compile-fail doctest now passes, preventing duplicated ordinal-issuer state within one ownership generation.
- `PROVEN focused`: `cargo test -p oteryn-game-server foundation` -> 70 passed / 0 failed; `cargo test -p oteryn-game-server` -> 73 passed / 0 failed; both `ScopeRuntimeFence` compile-fail doctests (move/Copy and explicit `clone()`) PASS.
- `PROVEN canonical gates`: `cargo +1.94.0 metadata --locked --format-version 1`, architecture workspace check, `build --locked --workspace --all-targets`, strict `clippy --locked --workspace --all-targets -- -D warnings`, `test --locked --workspace`, and `oteryn-synthetic-client-harness` all PASS.
- `PROVEN formatting/diff`: changed-file `rustfmt +1.94.0 --check apps/game-server/src/foundation/admission.rs` PASS; `mod.rs` PASS with `skip_children=true` to avoid the known untouched Windows CRLF child-module baseline; `git diff --check` PASS. Product-code diff is limited to `apps/game-server/src/foundation/admission.rs` and `apps/game-server/src/foundation/mod.rs`.
- `PROVEN governance`: `python tools/agents/validate_governance.py` PASS after the final task-record mutation; `git diff --check` PASS.
- `E2E`: `NOT_EVALUATED` remains correct because no merged production gameplay transport listener/client-entry seam exists and this allocation intentionally adds no listener side effects.
- `MERGE`: still prohibited until this repair is frozen/pushed as a new exact PR head, protected exact-head CI is green, every material thread is resolved on that head, and a genuinely independent exact-head review reports zero material findings.
- `FINAL SELF-REVIEW`: PASS on the complete repair diff; zero open material findings. Same-class review added separate healthy-controller/terminal COMMIT races and explicit `Clone` prevention in addition to the original `Copy` regression.
- `NEXT`: freeze/commit/push once, then do not move the head while fresh CI/review qualify it.

## Main governance drift integration checkpoint ? 2026-08-23T16:25:00+02:00

- `PROVEN`: frozen Foundation repair head `d729f3bdb461ad9171aac2b30e9bb9af2de0f49a` passed the complete local product gate and self-review, but exact-head Merge Authority Audit #172 failed before product inspection because its inherited repository policy still declared the superseded required status instead of `game-gate`.
- `PROVEN`: canonical `main` advanced from `c4c407d096a3252fd2850abbd616944c97297ce6` to `099e147031ce9320586602b98c62df1c4311bbe8` via merged PR #69 (`ci(governance): promote stable game-gate`), which owns exactly that control-plane transition.
- `AUTHORITY`: FOUNDATION does not hand-edit `.github/**` or repository policy. Current `main@099e147...` is merged into this branch as authoritative upstream state; the merge is conflict-free and imports only the PR #69 governance/workflow/policy files.
- `QUALIFICATION`: all exact-head evidence on `d729f3b...` is now superseded by this necessary main-sync head. Product repair content is unchanged; full local gate, protected exact-head CI, thread verification and genuinely independent exact-head review must be rerun on the resulting merge commit.
- `LOCAL POLICY BASELINE`: `validate_repository_policy.py` on this Windows checkout still fails only on untouched `LICENSE` newline normalization; `git diff origin/main -- LICENSE` is empty. Exact-head Linux Merge Authority/merge-gate validation is authoritative for this baseline-sensitive check.
- `MERGE`: prohibited until the new exact head passes those gates.

## Unified GameSession authority repair checkpoint - 2026-08-23T17:10:00+02:00

This checkpoint supersedes all preceding readiness/final-head checkpoints. Earlier SHA/review/CI entries remain historical evidence only.

- `SUPERSEDED EXACT HEAD`: `349430043addf628ecb3c0689226b499ccef5910` had green exact-head governance/architecture/merge-authority checks and independent Qwen PASS, but owner-authorized Codex exact-head review found 2 P1 + 1 P2; it is not merge evidence.
- `RED/P1 fresh transport`: two `AdmissionAuthority` processes sharing one consumed fresh grant could both become ACTIVE at generation 1 on different authenticated transports. The focused regression expected `GrantReplayed` for transport 200 but observed a second `Ok(GameSession { ... })`.
- `GREEN/P1 fresh transport`: `FreshAdmissionCommit<T>` now binds the exact initial transport and `FreshAdmissionAuthoritySnapshot<T>` carries current authoritative transport. More importantly, fresh admission is committed/reconciled through the same trusted `ReconnectAttemptJournal<T>` GameSession authority seam that owns lifecycle/reconnect state; local session/transport fields are projection only.
- `RED/P1 control loss`: after exact control loss, PREPARE succeeded locally but COMMIT failed `StaleConnection` because durable authority still saw ACTIVE/current controller. The old tests hid this with a test-only resynchronization helper.
- `GREEN/P1 control loss`: `mark_control_loss` is now an atomic authority operation keyed by exact GameSessionId + observed transport + observed connection generation. `Applied` durably publishes `Reconnectable` with no current controller before the local projection changes. The normal loss -> PREPARE -> COMMIT regression passes without any manual synchronization.
- `SAME-CLASS AUTHORITY REPAIR`: terminalization and RuntimeScope ownership-generation advance also route through the same trusted authority before local projection. All normal-path `sync_authoritative_fences` and the separate `TestFreshIdentityLedger` were removed; only direct test-only setters used to inject adversarial concurrent authority changes remain.
- `RED/P2 issuer reconstruction`: a compile-fail doctest constructed two `ScopeRuntimeFence` values from the same public `ScopeOwnershipGeneration`; the invalid program compiled, proving duplicate ordinal issuers remained possible despite removal of Clone/Copy.
- `GREEN/P2 issuer reconstruction`: `ScopeRuntimeFence::from_external_grant` is private to the Foundation owner implementation path. External callers cannot construct/reconstruct the issuer; production has one construction site in fresh GameSession projection and later ownership changes mutate the existing fence. The duplicate-constructor compile-fail test now passes, alongside Copy and Clone compile-fail tests.
- `PROVEN focused`: Foundation 72/72 PASS; full game-server 75/75 PASS; no manual authority synchronization helper remains; all three new material-finding regressions are GREEN; three `ScopeRuntimeFence` compile-fail doctests PASS.
- `PROVEN full local gate`: locked metadata PASS; architecture workspace check PASS; workspace all-targets build PASS; strict workspace Clippy `-D warnings` PASS; locked full workspace tests PASS; synthetic client harness PASS; changed-file Rust 2024 rustfmt PASS; governance PASS; `git diff --check` PASS.
- `PROVEN scope`: product changes remain limited to `apps/game-server/src/foundation/admission.rs` and `apps/game-server/src/foundation/mod.rs`; task evidence is this owned task record. No workflow/policy/registry/contract/new-crate mutation is authored by FOUNDATION.
- `SELF-REVIEW`: danger scan found no production TODO/FIXME/unsafe/panic/unwrap/expect/unreachable residue; the only `unwrap()` hits are compile-fail doctest examples. Production has exactly one `ScopeRuntimeFence::from_external_grant` construction call.
- `E2E`: `NOT_EVALUATED` remains accurate because no merged production gameplay listener/client-entry seam exists; this allocation introduces no listener side effects.
- `MERGE`: prohibited until this repair is committed/pushed as a new frozen exact head, all protected exact-head checks are green, all review threads are resolved, and a fresh genuinely independent exact-head review reports zero material findings.
- `FINAL SELF-REVIEW`: PASS on the complete current `main...working-tree` Foundation delivery diff; zero open local material findings. Public authority surface, all authority mutation entry points, constructor reachability, test-only mutation hooks and full changed-file set were re-inspected. Normal lifecycle paths contain no manual resynchronization escape hatch.
- `NEXT`: freeze one commit, push, then qualify only that exact SHA with protected CI and fresh independent review.
