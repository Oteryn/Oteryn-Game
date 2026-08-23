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
updated_at: 2026-08-23T22:26:54+02:00
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
  - docs/architecture/FND-04B_RECONNECT_RECOVERY_CONTINUITY_CONTRACT.md
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

Deliver the smallest real server foundation module and composition seam implementing accepted protocol framing/state, authoritative runtime fencing and admission/reconnect primitives without gameplay command/state registrations, invented recovery timing, persistence schema or production side effects.

## Authority and current source of truth

- `PROVEN`: canonical repository is `Oteryn/Oteryn-Game`; current implementation PR is #59 on `agent/otv2-impl-foundation-runtime-01`.
- `PROVEN`: Foundation owns `apps/game-server/src/foundation/**` and this task record; the allocated shared composition lease permits the existing `apps/game-server/src/lib.rs` integration.
- `PROVEN`: gameplay command/state registries, workflow/policy, contracts and new crate topology remain outside this worker allocation.
- `PROVEN`: `main@099e147031ce9320586602b98c62df1c4311bbe8` is the current integration base incorporated into the branch before the recovery repair.
- `PROVEN`: Git history and PR #59 discussion retain the detailed superseded repair/review chronology that previously made this active record large; this file now records only the live candidate state.

## Acceptance criteria

- [x] Bounded FND-02 framing/envelope/ingress rejects malformed, truncated, oversized and unknown input before peer-controlled allocation.
- [x] Typed foundation IDs, stable error dispositions, CommandRef/CommandId ordering/dedup and connection-generation fencing are implemented and negatively tested.
- [x] Server sequence, state revision, resync and snapshot barriers/assembly fail closed on gaps, rollback, replay and size violations.
- [x] FND-03 runtime ownership/ordinal primitives reject stale generations and prevent duplicate public construction/copy/clone of the ordinal issuer.
- [x] FND-04 fresh admission, GameSession, CharacterLease, control-loss and reconnect PREPARE/COMMIT semantics are bound to exact transport/generation/lease/runtime authority.
- [x] Durable/lost-response seams expose stable fresh-grant and reconnect-attempt reconciliation identities without inventing physical persistence policy.
- [x] Process replacement can rehydrate only a proven current GameSession binding; missing/terminal/stale lease/runtime evidence fails closed.
- [x] Focused regressions, full Rust workspace build/test, strict Clippy, formatting, policy/metadata, CodeQL, supply-chain, synthetic harness and server smoke pass on the product-code candidate.
- [ ] Mandatory genuinely independent exact-head review reports zero material P0/P1/P2 findings before merge.

## Final recovery repair

The owner-requested takeover resumed from PR #59 exact head `a58ba99b56eebaeb7eeb118406a12447b235e89d`, whose protected CI was green but whose fresh exact-head review left two unresolved P1 findings.

### P1 — reconnect COMMIT process recovery

`PRRT_kwDOT8SzxM6bgUjE`: after durable reconnect COMMIT + lost response + process crash, a new process had no local `current` GameSession and returned `Terminal` before it could consult the durable `(GameSessionId, ReconnectAttemptRef)` disposition.

- RED exact head: `ba3de01e5f0a481d48345e9c757bff8dc8fc9474`.
- RED evidence: Merge Gate run `32662981761`, Linux job `97251887783`, compile failure exactly `no method named rehydrate_session` from the new recovery regression.
- GREEN: public Foundation `ReconnectAttemptJournal::load_session(GameSessionId)` supplies one trusted `GameSessionAuthoritySnapshot`; `AdmissionAuthority::rehydrate_session` reconstructs the current projection before exact attempt reconciliation.
- Regression: durable generation-2 COMMIT survives process replacement, wrong transport remains `StaleConnection`, exact current candidate transport replays the same committed generation without a second authority switch.

### P1 — stale runtime generation resurrection

`PRRT_kwDOT8SzxM6bgUjG`: fresh-grant reconciliation rebuilt `ScopeRuntimeFence` from immutable admission-time scope generation instead of current externally fenced RuntimeScope ownership.

- RED exact head: `c00b8485e1b2b70bc6eefb511fdb0d67db7d26c9`.
- RED evidence: Merge Gate run `32662466581`, Linux job `97250663885`; build and strict Clippy passed, then the focused recovery assertion failed `left: 11`, `right: 12`.
- GREEN: `GameSessionAuthoritySnapshot` carries current typed `ScopeOwnershipGeneration`; the public trusted seam requires process-replacement runtime generation to come from the externally established FND-03 recovery ownership grant, never from a local increment or stale admission value.
- Same-class regression advances the trusted runtime ownership again after process replacement and proves recovery adopts only that newer external generation.

### Same-class CharacterLease fence

Self-review found that accepting `current_character_lease.generation >= admission_generation` would let an old GameSession adopt a newer CharacterLease instead of being fenced by it.

- RED exact head: `ee456f44dd14a783f5d2a243c1161bdaffcc84ad`.
- RED evidence: Merge Gate run `32664134768`, Linux job `97254837444`; build and strict Clippy passed, then exactly one of 85 tests failed: `foundation_recovery_tests::rehydrate_rejects_advanced_character_lease_for_same_session`.
- GREEN product-code head: `a3a36c14b7d6114ba5b3b6cfbb0ba244a066da71`.
- Repair: same-GameSession rehydration now requires exact CharacterLease identity + generation equality. Both rollback and advance are `StaleLease`; a different lease generation is a superseding character-authority fence, not adoptable state.

## Product-code validation

Exact product-code head: `a3a36c14b7d6114ba5b3b6cfbb0ba244a066da71`.

Protected Merge Gate run `32664262967` on that exact head proves:

- Rust Linux workspace build: PASS.
- strict workspace Clippy `-D warnings`: PASS.
- locked full workspace tests: PASS.
- `oteryn-game-server`: **85 passed / 0 failed**.
- recovery regressions: committed reconnect rehydration, current runtime authority, missing session, terminal session, advanced/rolled-back lease and rolled-back runtime generation all PASS.
- public fresh replay-key doctest: PASS.
- all three `ScopeRuntimeFence` compile-fail doctests: PASS.
- synthetic client harness: PASS (`synthetic-ok revision=1 entities=2 asset=checker actions=1`).
- native game-server bootstrap smoke: PASS.
- Rust policy/metadata/format/workspace-boundaries/production-closure checks: PASS.
- CodeQL actions/python: PASS.
- dependency review and supply-chain checks: PASS.

The Windows client job for this product-only SHA may complete independently, but this task-record commit intentionally creates a newer exact PR head and therefore all protected gates must qualify the final head again; no product-only CI result is reused as final merge authority.

## Scope and self-review

Current changed paths remain bounded to:

- `apps/game-server/src/foundation/admission.rs` — pre-existing Foundation implementation;
- `apps/game-server/src/foundation/protocol.rs` — pre-existing Foundation protocol implementation;
- `apps/game-server/src/foundation/mod.rs` — module composition/runtime primitives;
- `apps/game-server/src/foundation/admission_recovery_inner.rs` — current-authority validation and process projection reconstruction;
- `apps/game-server/src/foundation/admission_facade.rs` — public trusted recovery seam/facade;
- `apps/game-server/src/foundation/recovery_tests.rs` — cross-process/fail-closed recovery regressions;
- `apps/game-server/src/lib.rs` — allocated shared composition/test integration;
- this task record.

Self-review result for the takeover diff: PASS with zero known local material findings. No production `unsafe`, panic/unwrap/expect/TODO recovery escape hatch was introduced. `GameSessionId` remains routing/correlation identity rather than bearer authority. Rehydration never invents a connection, lease or runtime generation: it reconstructs only the current trusted authority snapshot, rejects terminal/missing/inconsistent state and retains exact transport/generation replay fencing. Runtime ownership replacement must be supplied by the external FND-03 authority grant.

## E2E

`NOT_EVALUATED`: no merged production gameplay transport listener/client-entry seam exists, and this allocation intentionally adds no listener side effects. Synthetic harness and server bootstrap smoke are evidence for the available boundary only; they are not represented as Tier-1 physical gameplay wire proof.

## Independent review

- required: YES — high-risk protocol/session/admission/fencing semantics.
- every independent verdict on a superseded SHA is non-authoritative for this candidate.
- current blocker: fresh genuinely independent review of the final exact PR head with zero material P0/P1/P2 findings.
- Remote Desktop/local Qwen was unavailable at takeover checkpoint time; no stale Qwen result is reused and no owner-funded Codex invocation is silently triggered.

## Context checkpoint

```yaml
last_progress: Owner-requested takeover repaired both outstanding exact-head P1 recovery findings plus one same-class CharacterLease fence issue using independent RED regressions. Product code at a3a36c14b7d6114ba5b3b6cfbb0ba244a066da71 has protected Linux build/strict Clippy/full workspace test GREEN, game-server 85/85, recovery regressions GREEN, compile-fail runtime-fence doctests GREEN, synthetic harness/server smoke GREEN and policy/security/supply-chain gates GREEN. This checkpoint commit intentionally moves the PR head, so only fresh protected checks and fresh independent review of the resulting final SHA may authorize merge.
status: review_pending
branch: agent/otv2-impl-foundation-runtime-01
product_code_head: a3a36c14b7d6114ba5b3b6cfbb0ba244a066da71
pr: 59
blocker: final exact-head protected CI and genuinely independent exact-head review must both be clean; the two repaired P1 review threads remain unresolved until that evidence exists
owner_action_required: null
next_action: qualify this checkpointed exact PR head with protected CI, then obtain one genuinely independent exact-head review; if both are clean, resolve the two repaired P1 threads, squash-merge PR #59, verify merged main, archive the task and release Foundation ownership.
```
