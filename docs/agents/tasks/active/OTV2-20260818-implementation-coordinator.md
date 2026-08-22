# OTV2-20260818-implementation-coordinator

```yaml
task_id: OTV2-20260818-implementation-coordinator
title: Coordinate first native implementation wave
mode: COORDINATE
status: wave1_exact_base_bind_pending
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/otv2-coordinator-bind-wave1-base
pr: 46
base_sha: 33cec30b8075c73290d7d76e9f59df4701771650
head_sha: null
owner: chat-github-20260818-implementation-coordinator
created_at: 2026-08-18T16:10:00+02:00
updated_at: 2026-08-22T18:09:00+02:00
execution_budget_minutes: 60
owned_paths:
  - docs/agents/tasks/active/OTV2-20260818-implementation-coordinator.md
  - docs/agents/programs/OTERYN_V2_IMPLEMENTATION_LIVE_ALLOCATIONS.md
public_contracts:
  - docs/agents/programs/OTERYN_V2_IMPLEMENTATION_EXECUTOR_DAG.md
  - docs/agents/prompts/OTV2_POST_SIM_WAVE1_PARALLEL_LAUNCH.md
  - docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md
depends_on:
  - OTV2-20260805-foundation-preimplementation-contracts
  - Oteryn-Game#44
  - Oteryn-Game#45
blocks:
  - OTV2-20260822-impl-foundation-runtime
  - OTV2-20260822-impl-domain-core
  - OTV2-20260822-impl-vsl-content
  - OTV2-20260822-impl-qa-e2e
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
```

## Outcome

Coordinate the explicitly invoked `Oteryn: implementation coordinator` programme on canonical `Oteryn/Oteryn-Game`, releasing only dependency-ready bounded worker lanes under exact-base, path-ownership, serialized-shared-mutation and review discipline.

## Current proven state

- `PROVEN`: Bootstrap lifecycle is completed and archived.
- `PROVEN`: SIM lifecycle is completed and archived; its production deterministic simulation crate is consumed by `apps/game-server`.
- `PROVEN`: post-SIM Wave 1 launch pack PR #44 merged as `7694c8a5e1ebc1dbffa937adf6b5cb775f7745f2`.
- `PROVEN`: Wave 1 allocation PR #45 passed exact-head Agent governance, Architecture semantic audit, Merge authority audit and aggregate Merge gate, then squash-merged as `33cec30b8075c73290d7d76e9f59df4701771650`.
- `PROVEN`: PR #45 reserves four pairwise non-overlapping primary paths: `apps/game-server/src/foundation/**`, `apps/game-server/src/domain/**`, `apps/game-server/src/content/**`, and `apps/game-server/tests/**`.
- `PROVEN`: historical SIM PR #13 bound `worker_base_sha` to the exact allocation merge SHA before implementation; Wave 1 exact-base PR #46 follows the same lifecycle.
- `PROVEN`: current server package is `apps/game-server`; `crates/protocol-oteryn` is not present on the allocation baseline, so no speculative new crate topology is introduced.
- `PROVEN`: shared composition/workspace paths remain coordinator-serialized, with FOUNDATION first after bind.
- `PROVEN`: stable contract/registry/CI/policy/new-crate mutations remain outside the Wave 1 allocation until separately granted.
- `PROVEN`: Foundation retains mandatory genuinely independent exact-head review for protocol/session/admission/fencing delivery.
- `PROVEN`: no production/protected/live-data/Platform/external-repository authority is granted.

## Exact-base bind

```yaml
allocation_pr: 45
allocation_merge_sha: 33cec30b8075c73290d7d76e9f59df4701771650
bind_branch: agent/otv2-coordinator-bind-wave1-base
bind_pr: 46
bind_write_authority_before_merge: false
worker_base_sha: 33cec30b8075c73290d7d76e9f59df4701771650
lanes:
  - OTV2-IMPL-FOUNDATION
  - OTV2-IMPL-DOMAIN
  - OTV2-IMPL-CONTENT
  - OTV2-IMPL-QA
serialized_shared_paths:
  - apps/game-server/src/lib.rs
  - apps/game-server/Cargo.toml
  - Cargo.toml
  - Cargo.lock
  - workspace-boundaries.toml
initial_shared_lease_owner_after_bind: OTV2-IMPL-FOUNDATION
```

## Merge discipline

Exact-base PR #46 is coordination-only. While it is unmerged, all four worker lanes remain read-only because canonical `main` still contains their `allocated_pending_exact_base` state. After lawful exact-head merge, create every worker branch/task from the resulting post-bind `main`; the live record keeps `worker_base_sha` equal to exact allocation merge `33cec30b8075c73290d7d76e9f59df4701771650`, matching the established SIM #12 → #13 convention.

Wave 1 workers may then develop their primary paths in parallel. Shared composition/workspace mutations remain serialized, starting with FOUNDATION. Any stable registry/contract/CI/policy/new-crate mutation requires an explicit coordinator reallocation before write.

## Context checkpoint

```yaml
last_progress: Wave 1 allocation PR #45 passed exact-head gates and merged as 33cec30b8075c73290d7d76e9f59df4701771650; exact-base PR #46 binds all four workers to that exact allocation merge.
status: wave1_exact_base_bind_pending
branch: agent/otv2-coordinator-bind-wave1-base
head_sha: null
pr: 46
blocker: exact-base PR #46 must pass exact-head governance/merge checks and merge before any worker write
owner_action_required: null
next_action: validate PR #46 at frozen head; after lawful merge, create four worker branches/tasks from resulting main and release non-overlapping implementation writes.
```
