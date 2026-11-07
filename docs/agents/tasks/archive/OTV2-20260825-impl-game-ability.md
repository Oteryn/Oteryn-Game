# OTV2-20260825-impl-game-ability

```yaml
task_id: OTV2-20260825-impl-game-ability
title: Implement bounded ability occurrence and effect-plan engine
mode: IMPLEMENT
status: completed_released
repository: Oteryn/Oteryn-Game
base_branch: main
branch: impl/game-ability-engine
issue: 166
pr: 171
base_sha: 7ac06bd84a1a31fc9a3ea2560de8ae20cea96741
head_sha: f9a359282701cd385a6bd0252105bc11d35f8832
final_head_sha: f9a359282701cd385a6bd0252105bc11d35f8832
final_head_frozen_at: null
owner: Oteryn: impl ability
created_at: 2026-08-25T23:24:03+02:00
updated_at: 2026-08-26T00:15:00+02:00
execution_budget_minutes: 120
large_budget_reason: revision-bound idempotent effect pipeline with deterministic boundary proof
owned_paths:
  - apps/game-server/src/ability/mod.rs
  - apps/game-server/src/ability/occurrence.rs
  - apps/game-server/src/ability/intent.rs
  - apps/game-server/src/ability/plan.rs
  - apps/game-server/src/ability/commit.rs
  - apps/game-server/src/ability/effects.rs
  - apps/game-server/src/ability/tests.rs
  - apps/game-server/tests/ability_engine.rs
  - docs/agents/tasks/archive/OTV2-20260825-impl-game-ability.md
public_contracts:
  - GAME-ABILITY-01 whole-gate acceptance
depends_on:
  - issue:162 allocation merge
  - issue:166
  - main@c57ddb5253cdfec126a768232d53f8a9bb292e3f or recorded successor
blocks:
  - future Combat readiness only after serial integration prerequisites
external_repositories: []
delivery_pr: 171
delivery_merge_sha: 2faa280b406a313d02ee1330c65651bc36e215a9
historical_pre_merge_independent_review: NOT_PROVEN
post_merge_independent_review: PASS_POST_MERGE_RECONCILIATION
independent_review_packet_sha256: fccb4e4d8ffa1406e4221edc3869ba7bd2607a1c1fe6c2f044ecc9ecc9babde2
independent_review_response_sha256: 1b8cd28726c5a4f9a8b37b77da9a9b13d3e35a21c9446c88e466d2e17f0305fd
ownership_release: true
```

## Outcome

Revision-bound, deterministic, idempotent occurrence/intent/plan/commit pipeline with immediate stateless typed damage/heal fixtures.

## Architecture and source of truth

- `PROVEN`: limits are 2 target candidates, 2 resolved targets, 2 effect-plan entries, 4096 effect-plan bytes and 8 calculation stages.
- `PROVEN`: fixture behavior is structural evidence, not Reference parity; unregistered mechanics remain excluded.
- `DERIVED`: `apps/game-server/src/lib.rs` composition is a coordinator-only serialized lease, so this task uses its allocated standalone test harness before post-delivery composition.

## Acceptance criteria

- [x] Occurrence/revision and retry no-double-commit proof passes.
- [x] Partial/sequential groups and deterministic calculation/effect order pass.
- [x] Each allocated limit has max/max+1 proof before effects.
- [x] Proposal-only/direct-mutation negatives, exact-head CI/review/merge/archive lifecycle pass.

## Excluded scope

No formula/geometry/retarget/multihit/channel/timer/cooldown/condition/reaction invention; no durability/value, Movement, protocol/UI/persistence/production Content, Cargo/workspace/registry mutation.

## Validation

### Focused

- command/run: PATH=/tmp/oteryn-rust/rustup/toolchains/1.94.0-x86_64-unknown-linux-gnu/bin:$PATH cargo test --locked -p oteryn-game-server --test ability_engine
- result: PASS — 8 passed, 0 failed

### Component/integration

- command/run: PATH=/tmp/oteryn-rust/rustup/toolchains/1.94.0-x86_64-unknown-linux-gnu/bin:$PATH cargo clippy --locked -p oteryn-game-server --test ability_engine -- -D warnings
- result: PASS — focused integration target compiles cleanly with `-D warnings`

### E2E

- scenario: `NOT_APPLICABLE` until real listener/client boundary exists
- result: `NOT_APPLICABLE`

## Context checkpoint

```yaml
last_progress: PR #171 merged; exact-head CI passed; post-merge independent reconciliation PASS 0/0/0; ownership released
status: completed_released
branch: impl/game-ability-engine
base_sha: 7ac06bd84a1a31fc9a3ea2560de8ae20cea96741
head_sha: f9a359282701cd385a6bd0252105bc11d35f8832
pr: 171
final_head_sha: f9a359282701cd385a6bd0252105bc11d35f8832
owner_action_required: null
blocker: null
next_action: none_terminal
```

## Terminal post-merge reconciliation

Historical pre-merge independent-review evidence remains `NOT_PROVEN`; the fresh non-authoring exact-tree review is recorded only as `PASS_POST_MERGE_RECONCILIATION`. The merged implementation, exact-head CI, post-merge review and ownership release are terminal.
