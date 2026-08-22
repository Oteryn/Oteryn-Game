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
updated_at: 2026-08-22T22:08:00+02:00
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

- [ ] TDD: every production behavior is preceded by a focused test observed failing for the intended reason.
- [ ] Bounded protocol frame envelope rejects truncated, oversized and unknown foundation message classes before peer-sized allocation.
- [ ] Command/session connection-generation state rejects duplicate gaps, stale generations and invalid progression deterministically.
- [ ] Runtime ownership primitives enforce one current generation and reject stale owner work/timers/results.
- [ ] Admission/reconnect primitives model fresh admission, GameSession/CharacterLease binding and fail-closed stale/replay cases without live Platform/DB authority.
- [ ] Foundation error vocabulary maps failures without leaking secrets or inventing gameplay semantics.
- [ ] `apps/game-server` composition consumes the foundation module while preserving fail-closed gameplay availability until owning gameplay registrations exist.
- [ ] Focused tests, full workspace tests, format and Clippy pass at final head.
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
- result: PASS — 30 foundation tests; 0 failed

### Component/integration
- command/run: `cargo test -p oteryn-game-server`
- result: PASS — 33 package tests; 0 failed; full-workspace Clippy `-D warnings` PASS

### E2E
- scenario: Tier 1 wire journey only when a real merged production transport seam exists; otherwise `NOT_EVALUATED` with exact blocker.
- result: `NOT_EVALUATED` - no merged production transport listener/client-entry seam exists in the allocated composition; this lane intentionally adds no listener side effects.

### Exact-head CI
- final head: pending final task-metadata checkpoint; frozen code head `7f214607e24f666b8569af3ce0a6222fbec51d00`
- trigger source: pending
- workflow/run/job: pending
- runner assignment: pending
- classification: high-risk foundation
- result: pending

## Self-review

- exact head: `7f214607e24f666b8569af3ce0a6222fbec51d00` (code head before task-metadata-only checkpoint)
- method/reviewer: implementing/coordinating agent full-diff review against Issue #53, FND-02/FND-03/FND-04 and registered limits/errors
- material findings: 7 repaired before freeze; open material findings: 0
- verdict: PASS

## Independent review

- required: YES — protocol/session/admission/fencing high-risk semantics
- exact head: pending
- method/auditor: genuinely independent exact-head reviewer
- material findings: pending
- verdict: pending

## Context checkpoint

```yaml
last_progress: code head 7f214607 is frozen and locally green after synchronization with main; self-review repaired CommandRef identity, snapshot zero-length semantics/assembly, additive protobuf compatibility, bootstrap generation phase rules, snapshot limit disposition and reconnect-attempt preservation; open material findings 0.
status: review_pending
branch: agent/otv2-impl-foundation-runtime-01
head_sha: 7f214607e24f666b8569af3ce0a6222fbec51d00
pr: 59
blocker: mandatory genuinely independent exact-head review and protected exact-head CI remain before merge
owner_action_required: null
next_action: commit this task-metadata-only checkpoint, then run exact-head governance/CI and obtain genuinely independent review on the unchanged final PR head.
```
