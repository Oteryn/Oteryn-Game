# OTV2-20260825-impl-game-interaction

```yaml
task_id: OTV2-20260825-impl-game-interaction
title: Implement bounded deterministic interaction lifecycle
mode: IMPLEMENT
status: done_with_concerns
repository: Oteryn/Oteryn-Game
base_branch: main
branch: impl/game-interaction-lifecycle
issue: 165
pr: null
base_sha: 7ac06bd84a1a31fc9a3ea2560de8ae20cea96741
head_sha: d0059c5a11d0b8bd3b8c159a871436e34f1c9937
final_head_sha: null
final_head_frozen_at: null
owner: Oteryn: impl interaction
created_at: 2026-08-25T23:24:03+02:00
updated_at: 2026-08-26T00:40:00+02:00
execution_budget_minutes: 120
large_budget_reason: bounded authoritative lifecycle implementation with retry/reconciliation and deterministic limit proof
owned_paths:
  - apps/game-server/src/interaction/mod.rs
  - apps/game-server/src/interaction/identity.rs
  - apps/game-server/src/interaction/plan.rs
  - apps/game-server/src/interaction/lifecycle.rs
  - apps/game-server/src/interaction/dispatch.rs
  - apps/game-server/src/interaction/tests.rs
  - apps/game-server/tests/interaction_workflow.rs
  - docs/agents/tasks/active/OTV2-20260825-impl-game-interaction.md
public_contracts:
  - GAME-INTERACTION-01 successor-child identity/retry acceptance
depends_on:
  - issue:162 allocation merge
  - issue:165
  - main@c57ddb5253cdfec126a768232d53f8a9bb292e3f or recorded successor
blocks:
  - future Movement readiness only after merged interaction and independent prerequisites
external_repositories: []
```

## Outcome

Bounded deterministic interaction lifecycle with truthful retry/reconciliation and no foreign-owner mutation.

## Architecture and source of truth

- `PROVEN`: only registry limits cascade depth 2, child fan-out 8, root work 8, trigger candidates 16 and retained child lifecycles 8 are authorized.
- `PROVEN`: Movement, Ability, DUR-03 and Foundation retain their respective state authority.
- `DERIVED`: `apps/game-server/src/lib.rs` composition is a coordinator-only serialized lease, so this task must use its allocated standalone test harness until post-delivery composition.

## Acceptance criteria

- [ ] Stable identity/retry/reconciliation and deterministic order/RNG have focused proof.
- [ ] All five limits have max/max+1 rejection proof before effects.
- [ ] Duplicate, stale/cancel and foreign-owner direct-write negatives pass.
- [ ] Exact-head CI/review/merge/archive lifecycle is complete.

## Excluded scope

No Movement/handoff, Ability effects, value durability, protocol/events, transport/client/persistence, Cargo/workspace/registry or unregistered limits.

## Validation

### Focused

- command/run: `cargo test -p oteryn-game-server --test interaction_workflow stable_child_identity_is_reconstructed_from_the_same_semantic_tuple`
- result: `NOT_EXECUTED_LOCAL`: `/bin/bash: cargo: command not found`; neither `cargo` nor `rustc` is available on `PATH` or in the checked standard tool locations. The test-first harness proves 16 typed trigger candidates with one selected child, rejects 17 candidates, proves all remaining max/max+1 limits from selected work or typed retention, and covers stable ancestry/revision identity, opaque domain-owned operation types, canonical ordering/RNG identity, duplicate dispatch, stale/cancel, stale authority generation/revision and adapter-owner negatives.

### Component/integration

- command/run: `cargo test -p oteryn-game-server --test interaction_workflow`
- result: `NOT_EXECUTED_LOCAL`: same absent Rust toolchain; exact-head GitHub CI is required before composition or merge.

### E2E

- scenario: `NOT_APPLICABLE` until real listener/client boundary exists
- result: `NOT_APPLICABLE`

## Context checkpoint

```yaml
last_progress: CI correction aligns the standalone harness plan construction with the exact rustfmt layout reported at the remote head
status: done_with_concerns
branch: impl/game-interaction-lifecycle
head_sha: d0059c5a11d0b8bd3b8c159a871436e34f1c9937
pr: null
final_head_sha: null
blocker: local Rust toolchain unavailable; static review only until exact-head GitHub CI
next_action: coordinator evaluates the exact candidate in GitHub CI and serializes the separate lib.rs composition lease only after worker review
```
