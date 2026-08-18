# OTV2-20260818-implementation-coordinator

```yaml
task_id: OTV2-20260818-implementation-coordinator
title: Coordinate first native implementation wave
mode: COORDINATE
status: sim_allocation_pending_exact_base
repository: Oteryn/Oteryn-Game
base_branch: main
branch: docs/otv2-20260818-sim-allocation
pr: pending
base_sha: ed84415f4a55d8c16f703b7c1a130c0e43a1c1a1
owner: chat-github-20260818-implementation-coordinator
created_at: 2026-08-18T16:10:00+02:00
updated_at: 2026-08-18T17:23:00+02:00
execution_budget_minutes: 60
owned_paths:
  - docs/agents/tasks/active/OTV2-20260818-implementation-coordinator.md
  - docs/agents/programs/OTERYN_V2_IMPLEMENTATION_LIVE_ALLOCATIONS.md
public_contracts:
  - docs/agents/programs/OTERYN_V2_IMPLEMENTATION_EXECUTOR_DAG.md
  - docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md
depends_on:
  - OTV2-20260805-foundation-preimplementation-contracts
blocks:
  - OTV2-IMPL-SIM writes until exact allocation base is merged and bound
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
```

## Outcome

Coordinate the explicitly invoked `Oteryn: implementation coordinator` programme on canonical `Oteryn/Oteryn-Game`, beginning with serial Bootstrap and then releasing dependency-ready bounded worker lanes under exact-base and path-ownership discipline.

## Current proven state

- `PROVEN`: Bootstrap PR #10 merged as `0809004252db228e8f3fac3cdb6638c3c2a7fbda` and PR #11 archived/released Bootstrap as current `main@231d063ff877b41f01a8032018284fc2f910161e`.
- `PROVEN`: Bootstrap ownership is empty and the real `apps/game-server` production root exists.
- `PROVEN`: Wave 1 permits Foundation/SIM/Domain/Content/QA after Bootstrap as paths permit.
- `PROVEN`: root Cargo/workspace/lock/policy/CI mutations must remain serialized; therefore this coordinator releases SIM alone first rather than allowing concurrent root mutations.
- `PROVEN`: no existing stable SIM profile/revision registry or implementation was found on live main; accepted SIM architecture intentionally leaves concrete implementation artifact/profile selection to implementation evidence.
- `PROVEN`: SIM is allocated a real immediate consumer in `apps/game-server`; no speculative unconsumed crate is authorized.
- `PROVEN`: SIM may use existing workspace `sha2` without adding third-party dependency graph, but it receives no security-randomness/seed-secrecy authority.
- `PROVEN`: Foundation remains dependency-ready later but its protocol/session/admission/fencing implementation requires genuinely independent exact-head review. The current historical semantic-audit workflow does not satisfy that gate for arbitrary Foundation code because its deterministic script returns `NOT_APPLICABLE` outside two named historical doc profiles.
- `PROVEN`: no production/protected/live-data/Platform/external-repository authority is granted.

## Merge discipline

The live allocation is currently `allocated_pending_exact_base`. No SIM implementation write may occur until the allocation PR is merged and a follow-up coordinator reconciliation binds `worker_base_sha` to that exact merge SHA on `main`. Other Wave 1 workers remain read-only while SIM owns serialized workspace paths.

## Context checkpoint

```yaml
last_progress: Bootstrap lifecycle is fully closed on main@231d063ff877b41f01a8032018284fc2f910161e; a bounded SIM allocation is prepared with real game-server consumption and serialized workspace ownership.
status: sim_allocation_pending_exact_base
branch: docs/otv2-20260818-sim-allocation
head_sha: pending_final_freeze
pr: pending
blocker: null
owner_action_required: null
next_action: Merge the SIM allocation through exact-head governance CI, bind worker_base_sha to that allocation merge, then implement OTV2-IMPL-SIM.
```
