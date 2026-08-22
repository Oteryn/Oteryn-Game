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
updated_at: 2026-08-23T00:16:11+02:00
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
- result: PASS - 46 foundation tests; 0 failed after third-review repair plus final FND-ID self-review hardening

### Component/integration
- command/run: `cargo test -p oteryn-game-server`
- result: PASS - 49 package tests; 0 failed; full workspace PASS; Clippy `-D warnings` PASS

### E2E
- scenario: Tier 1 wire journey only when a real merged production transport seam exists; otherwise `NOT_EVALUATED` with exact blocker.
- result: `NOT_EVALUATED` - no merged production transport listener/client-entry seam exists in the allocated composition; this lane intentionally adds no listener side effects.

### Exact-head CI
- second repaired PR head: `c72bb0ce9cd7e982f0720e47571c173d178b5465`
- superseded evidence: Agent Governance #228, Architecture Semantic Audit #167 and Merge Authority Audit #145 were SUCCESS; Merge Gate #192 was still running when the third material Codex finding invalidated that exact-head evidence
- third repaired code head: `ed3bc851955974b471d3d9544189e0e8bd1f6456`
- final FND-ID code head: `99786226bf988840daa0bcde55d8f90f4d744561`
- classification: high-risk foundation
- result: pending fresh exact-head CI; no prior exact-head success is reused after this material repair

## Self-review

- first independent-review repair cycle: 5 material issues (3 P1, 2 P2) plus one terminal-replay self-review edge repaired
- second independent review on `815fe5cc8da0633b67cab7840b1f60cb2137df78`: 2 P1 repaired (post-payload server-sequence commit; updatable current runtime-ownership fence)
- third independent review on `c72bb0ce9cd7e982f0720e47571c173d178b5465`: 1 P1 - a committed reconnect-attempt result could replay success after a later transport/generation superseded it
- third repair: shared committed-attempt reconciliation now returns success only when both `current_transport` and current `connection_generation` still match the committed attempt; PREPARE and COMMIT use the same helper
- additional same-invariant self-review finding: `mark_unexpected_control_loss` retained the lost physical transport identity, which could still satisfy the replay helper before another winner appeared; RED reproduced this and both unexpected loss and terminalization now clear `current_transport`
- TDD evidence: superseded A->B replay RED returned `Ok(ConnectionGeneration(2))` instead of `StaleConnection`; lost-current-transport RED retained `Some(200)` instead of `None`; both regressions are GREEN after repair
- final self-review findings: raw `[u8; 16]` semantic IDs violated FND-ID strong typing/UUIDv7 trust-boundary validation; successful GameSessionId reuse and failed precommit-candidate reuse were also possible
- final TDD evidence: semantic-ID API tests first failed on missing typed ID constructors; duplicate GameSessionId was observed accepted; failed precommit candidate was observed reusable after incumbent-race rejection
- final repair: distinct `GameSessionId`/`CharacterId`/`WorldId`/`ChannelId` UUIDv7+RFC-variant wrappers, `CommandRef`/session/lease typed APIs, and a non-reusable GameSession candidate set reserved before the incumbent race while GrantNonce remains unconsumed on that losing path
- repaired-tree method: full diff review against FND-02/FND-03/FND-04, focused/package/workspace tests, strict Clippy, fmt check, governance and diff check
- open material findings: 0 locally; fresh independent exact-head review still mandatory
- verdict: PASS for self-review only

## Independent review

- required: YES - protocol/session/admission/fencing high-risk semantics
- first reviewed superseded head `9d5a251adb16076c3b0ebc50ae023677bf571894`: REQUEST_CHANGES - 3 P1 + 2 P2; repaired
- second reviewed superseded head `815fe5cc8da0633b67cab7840b1f60cb2137df78`: REQUEST_CHANGES - 2 P1; repaired
- third reviewed superseded head `c72bb0ce9cd7e982f0720e47571c173d178b5465`: REQUEST_CHANGES - 1 P1; repaired
- third P1 repair: committed-attempt result replay is current-channel/current-generation fenced; superseded transport/generation receives `StaleConnection` rather than a historical success
- owner authorization: explicit and bounded to PR #59 independent review plus autonomous repair/re-review/merge/closeout cycle
- final product-code head: `99786226bf988840daa0bcde55d8f90f4d744561`
- final PR head: pending metadata freeze commit
- verdict: pending fresh genuinely independent exact-head review after this material repair

## Context checkpoint

```yaml
last_progress: third-review reconnect replay repair at ed3bc851 is preserved; final self-review then found and TDD-repaired FND-ID strong-typing/UUIDv7 and GameSession candidate non-reuse gaps. Product code head 99786226 is green: Foundation 46/46, game-server 49/49, workspace and strict Clippy PASS. Final metadata freeze, exact-head CI and genuinely independent final review remain.
status: review_pending
branch: agent/otv2-impl-foundation-runtime-01
head_sha: pending-final-metadata-freeze
pr: 59
blocker: fresh exact-head CI and genuinely independent exact-head review are mandatory after the final FND-ID material repair
owner_action_required: null
next_action: commit and push this metadata checkpoint, verify exact-head CI, then run a fresh independent exact-head review on the unchanged final PR head; merge only with zero material findings.
```
