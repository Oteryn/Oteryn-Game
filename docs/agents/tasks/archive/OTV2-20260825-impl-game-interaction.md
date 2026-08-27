# OTV2-20260825-impl-game-interaction

```yaml
task_id: OTV2-20260825-impl-game-interaction
title: Implement bounded deterministic interaction lifecycle
mode: IMPLEMENT
status: completed_released
repository: Oteryn/Oteryn-Game
base_branch: main
branch: impl/game-interaction-lifecycle
issue: 165
pr: 172
base_sha: 7ac06bd84a1a31fc9a3ea2560de8ae20cea96741
head_sha: 14572daedfca2207cd024a022613ce42c2539169
final_head_sha: 14572daedfca2207cd024a022613ce42c2539169
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
  - docs/agents/tasks/archive/OTV2-20260825-impl-game-interaction.md
public_contracts:
  - GAME-INTERACTION-01 successor-child identity/retry acceptance
depends_on:
  - issue:162 allocation merge
  - issue:165
  - main@c57ddb5253cdfec126a768232d53f8a9bb292e3f or recorded successor
blocks:
  - future Movement readiness only after merged interaction and independent prerequisites
external_repositories: []
delivery_pr: 172
delivery_merge_sha: 73f82e4864aa15ece50625bda8bac7868f779ba3
historical_pre_merge_independent_review: NOT_PROVEN
post_merge_independent_review: PASS_POST_MERGE_RECONCILIATION
independent_review_packet_sha256: 987fe186d3dde5d209e800a61adba41bafcbd2fc2a68a66b8288ab6787f5eb16
independent_review_response_sha256: 1b8cd28726c5a4f9a8b37b77da9a9b13d3e35a21c9446c88e466d2e17f0305fd
ownership_release: true
```

## Outcome

Bounded deterministic interaction lifecycle with truthful retry/reconciliation and no foreign-owner mutation.

## Architecture and source of truth

- `PROVEN`: only registry limits cascade depth 2, child fan-out 8, root work 8, trigger candidates 16 and retained child lifecycles 8 are authorized.
- `PROVEN`: Movement, Ability, DUR-03 and Foundation retain their respective state authority.
- `DERIVED`: `apps/game-server/src/lib.rs` composition is a coordinator-only serialized lease, so this task must use its allocated standalone test harness until post-delivery composition.

## Acceptance criteria

- [x] Stable identity/retry/reconciliation and deterministic order/RNG have focused proof.
- [x] All five limits have max/max+1 rejection proof before effects.
- [x] Duplicate, stale/cancel and foreign-owner direct-write negatives pass.
- [x] Exact-head CI/review/merge/archive lifecycle is complete.

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
last_progress: PR #172 merged; exact-head CI passed; post-merge independent reconciliation PASS 0/0/0; ownership released
status: completed_released
branch: impl/game-interaction-lifecycle
head_sha: 14572daedfca2207cd024a022613ce42c2539169
pr: 172
final_head_sha: 14572daedfca2207cd024a022613ce42c2539169
blocker: null
next_action: none_terminal
```

## Terminal post-merge reconciliation

Historical pre-merge independent-review evidence remains `NOT_PROVEN`; the fresh non-authoring exact-tree review is recorded only as `PASS_POST_MERGE_RECONCILIATION`. The merged implementation, exact-head CI, post-merge review and ownership release are terminal.
