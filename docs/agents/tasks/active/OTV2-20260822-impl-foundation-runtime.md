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
updated_at: 2026-08-23T10:53:59+02:00
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
- result: PASS - 57 foundation tests; 0 failed after final pending-PREPARE reconciliation repair

### Component/integration
- command/run: `cargo test -p oteryn-game-server`
- result: PASS - 60 package tests; 0 failed; full workspace PASS; Clippy `-D warnings` PASS

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
- open material findings: both currently known P1 findings are repaired locally; fresh independent exact-head review is still mandatory before claiming zero material findings
- verdict: PASS for self-review only; ready to freeze the repair head

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
- final product-code head: pending opaque-ID journal + control-loss fencing repair freeze commit
- final PR head: pending freeze commit
- verdict: pending full validation and fresh genuinely independent exact-head review after this material repair

## Context checkpoint

```yaml
last_progress: Two applicable P1s are locally repaired on top of dbe7b50: numeric ordering of opaque ReconnectAttemptRef has been replaced by exact-key trusted ReconnectAttemptJournal reconciliation, and unexpected control-loss observations are now fenced to exact current transport plus ConnectionGeneration so stale predecessor callbacks are ignored. Cross-process journal claiming also serializes one distinct PREPARED attempt per GameSession. TDD RED/GREEN evidence exists for u64::MAX poisoning and delayed predecessor loss. Final local gate: Foundation 61/61, game-server 64/64, workspace, strict Clippy, fmt, governance, semantic scan and diff-check all PASS.
status: ready_for_repair_freeze
branch: agent/otv2-impl-foundation-runtime-01
head_sha: dbe7b50c7f5dad627c2a569b726c7e6a53f86b00-plus-uncommitted-final-p1-repairs
pr: 59
blocker: repair must be committed/pushed and then receive fresh protected exact-head CI plus genuinely independent exact-head review with zero material findings
owner_action_required: null
next_action: freeze/commit/push the final P1 repairs, verify remote head, update PR #59 and review threads, request fresh exact-head Codex review, and merge only when all exact-head gates are green with zero unresolved material findings.
```
