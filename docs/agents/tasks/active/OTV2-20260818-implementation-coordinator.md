# OTV2-20260818-implementation-coordinator

```yaml
task_id: OTV2-20260818-implementation-coordinator
title: Coordinate first native implementation wave
mode: COORDINATE
status: wave1_allocation_pending_exact_base
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/otv2-coordinator-allocate-wave1
pr: 45
base_sha: 7694c8a5e1ebc1dbffa937adf6b5cb775f7745f2
head_sha: null
owner: chat-github-20260818-implementation-coordinator
created_at: 2026-08-18T16:10:00+02:00
updated_at: 2026-08-22T18:03:00+02:00
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
- `PROVEN`: SIM allocation PR #12 merged as `2fc59dd83a3d13e7de8954d4dbcce5415e346389`; exact-base PR #13 merged as `977e98b05738076744540a123d4e35c32cd94c2c`.
- `PROVEN`: SIM final frozen head `7a0d71bbabdd00c54951aa8e0084d62f3dce748b` passed mandatory self-review and exact-head Agent governance, Architecture semantic audit, Merge authority audit, aggregate Merge gate, Linux full workspace, Windows production-client checks, supply chain and exact-head Windows SIM golden validation.
- `PROVEN`: SIM delivery PR #14 auto/squash-merged as `66619daf5837f31f7c54676e9f8351ed4ae220b0`.
- `PROVEN`: SIM archive/ownership-release PR #15 passed exact-head lifecycle CI and auto/squash-merged as terminal SIM lifecycle baseline `d178fee47bb42856a719aaf8c4a4fced64278df0`.
- `PROVEN`: post-SIM Wave 1 launch pack PR #44 merged to live `main` as `7694c8a5e1ebc1dbffa937adf6b5cb775f7745f2`.
- `PROVEN`: live `main` was re-resolved immediately before this allocation and still equals `7694c8a5e1ebc1dbffa937adf6b5cb775f7745f2`.
- `PROVEN`: current server package is `apps/game-server`; `apps/game-server/AGENTS.md` explicitly reserves later coordinator allocation for Foundation protocol/runtime/admission work.
- `PROVEN`: `crates/protocol-oteryn` is not present on the resolved allocation source and is not a current workspace member; this allocation therefore does not invent a new crate topology.
- `PROVEN`: allocation PR #45 reserves Foundation/Domain/Content/QA as four non-overlapping module/test families, but all remain `allocated_pending_exact_base` and therefore read-only.
- `PROVEN`: shared composition/workspace paths are coordinator-serialized and no stable registry, contract, CI or architecture-policy mutation is granted by this reservation.
- `PROVEN`: Foundation retains the mandatory genuinely independent exact-head review requirement for protocol/session/admission/fencing semantics; self-review must never be relabeled as independent.
- `PROVEN`: no production/protected/live-data/Platform/external-repository authority is granted.

## Wave 1 reservation

```yaml
allocation_source_main_sha: 7694c8a5e1ebc1dbffa937adf6b5cb775f7745f2
allocation_branch: agent/otv2-coordinator-allocate-wave1
allocation_pr: 45
worker_write_authority: false
reserved_lanes:
  - lane_id: OTV2-IMPL-FOUNDATION
    task_id: OTV2-20260822-impl-foundation-runtime
    branch: agent/otv2-impl-foundation-runtime-01
    owned_paths:
      - apps/game-server/src/foundation/**
      - docs/agents/tasks/active/OTV2-20260822-impl-foundation-runtime.md
  - lane_id: OTV2-IMPL-DOMAIN
    task_id: OTV2-20260822-impl-domain-core
    branch: agent/otv2-impl-domain-core-01
    owned_paths:
      - apps/game-server/src/domain/**
      - docs/agents/tasks/active/OTV2-20260822-impl-domain-core.md
  - lane_id: OTV2-IMPL-CONTENT
    task_id: OTV2-20260822-impl-vsl-content
    branch: agent/otv2-impl-vsl-content-01
    owned_paths:
      - apps/game-server/src/content/**
      - docs/agents/tasks/active/OTV2-20260822-impl-vsl-content.md
  - lane_id: OTV2-IMPL-QA
    task_id: OTV2-20260822-impl-qa-e2e
    branch: agent/otv2-impl-qa-e2e-01
    owned_paths:
      - apps/game-server/tests/**
      - docs/agents/tasks/active/OTV2-20260822-impl-qa-e2e.md
serialized_shared_paths:
  - apps/game-server/src/lib.rs
  - apps/game-server/Cargo.toml
  - Cargo.toml
  - Cargo.lock
  - workspace-boundaries.toml
shared_lease_order:
  - OTV2-IMPL-FOUNDATION
  - OTV2-IMPL-DOMAIN
  - OTV2-IMPL-CONTENT
  - OTV2-IMPL-QA
```

## Merge discipline

Allocation PR #45 must merge before any worker can write. After its exact merge SHA is known, the coordinator must create a separate exact-base bind PR that sets every reserved lane's `worker_base_sha` to that exact allocation merge SHA and changes each lane from `allocated_pending_exact_base` to `allocated`. Only after the bind itself merges may worker task files/branches be created and used for implementation writes.

Wave 1 code paths may then develop in parallel, but shared composition/workspace paths remain one-writer-at-a-time under the coordinator lease. Contract/registry/stable-ID/CI/policy/new-crate mutations are outside this allocation until separately and explicitly granted. FOUNDATION shared composition is first in the lease order and its protocol/session/admission/fencing delivery remains subject to mandatory independent exact-head review.

## Context checkpoint

```yaml
last_progress: allocation PR #45 is open from live main 7694c8a5e1ebc1dbffa937adf6b5cb775f7745f2 with four non-overlapping Wave 1 reservations and serialized shared-path ownership.
status: wave1_allocation_pending_exact_base
branch: agent/otv2-coordinator-allocate-wave1
head_sha: null
pr: 45
blocker: allocation PR #45 must pass exact-head governance/merge checks and merge before exact-base binding
owner_action_required: null
next_action: validate PR #45 at its frozen head; after lawful merge, bind all four workers to the exact allocation merge SHA in a separate coordinator PR before any worker write.
```
