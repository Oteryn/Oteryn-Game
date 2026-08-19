# OTV2-20260805-foundation-preimplementation-contracts

```yaml
task_id: OTV2-20260805-foundation-preimplementation-contracts
title: Coordinate Oteryn v2 foundation contracts and staged implementation gates
mode: COORDINATE
status: ready_for_owner_initiated_implementation
repository: blakinio/Oteryn-v2
base_branch: main
branch: null
pr: null
owner: unassigned
created_at: 2026-08-05T08:49:00+02:00
updated_at: 2026-08-17
owned_paths: []
public_contracts:
  - docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md
  - docs/architecture/GLOBAL_ARCHITECTURE_DECISION_REGISTER.md
  - docs/architecture/README.md
  - docs/agents/programs/OTERYN_V2_IMPLEMENTATION_EXECUTOR_DAG.md
continuation_prompt: docs/agents/prompts/OTV2_GLOBAL_ARCHITECTURE_DECISION_COORDINATOR.md
implementation_prompt: docs/agents/prompts/OTV2_IMPLEMENTATION_COORDINATOR.md
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
```

## Outcome

Maintain a truthful **non-owning** programme checkpoint. This record owns no architecture or implementation path. Material future work uses bounded tasks/branches/PRs and live coordinator allocations.

## Accepted architecture state

```yaml
GAME-ABILITY-01: ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED
GAME-INTERACTION-01: ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED
ALPHA-CLIENT-01: ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED
GAME-AI-01: ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED
ANL-02: ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED
ANL-03: ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED
VSL-MOVE-01: ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED
VSL-COMBAT-01: ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED
VSL-CONTENT-01: ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED
```

Stage-C acceptance merged through PR #311 / `e0ea9ef87c01dec720a22e8df6d54bfd669cb62c`; a fresh separate-session independent review found zero material findings on exact head `c5d9f839abd8998d42f4f37b203882f03bb51ce0`; lifecycle/status closeout merged through PR #318 / `a6a5180d98cf7791e40d9e1d08b25a5c8b4eff96`.

Reference evidence remains 0/4 promoted and fail closed. Architecture acceptance does not imply parity.

## Final implementation handoff

PR #314 merged as `88f4fb754b5ae11243afd38a9e0b6a8e3b0a5815` and delivered the formally evaluated implementation coordinator/worker DAG:

```text
17/17 execution prompts: PASS
10/10 prompt-evaluation gates: PASS
open material prompt findings: 0
```

The merge released the coordinator programme for explicit invocation but started no implementation worker.

Normal owner entry point:

```text
Oteryn: implementation coordinator
```

Direct worker aliases remain read-only without a live coordinator allocation.

## Canonical implementation order

```text
BOOTSTRAP [serial]
  -> FOUNDATION + SIM + DOMAIN + CONTENT + QA
  -> DURABILITY after Foundation/Domain
  -> ABILITY + INTERACTION + AI after Foundation/SIM/Domain/Content
  -> CLIENT after compatible Foundation seam
  -> MOVE after Foundation/SIM/Domain/Content/Interaction/Client/QA
  -> COMBAT only after merged MOVE + Foundation/SIM/Domain/Content/Ability/Interaction/Durability/Client/QA

CHANNEL = later after Foundation/Domain/Durability
CONTENT-FORMAT-SPIKE = evidence only
ANALYTICS = later after concrete producer event families exist
```

Stable workspace/registry/stable-ID mutations remain serialized.

## Lane-specific holds preserved

- `PROD-ENTITLEMENTS-01` remains unaccepted on the Oteryn-v2 consumer/enforcement side; entitlement implementation/activation stays blocked.
- Exact Reference formulas/mechanics/values remain evidence-gated; test fixtures cannot establish parity.
- Permanent World Project/Bundle physical encoding remains behind the DUR-04 format spike plus later owner decision.
- Missing required resource ceilings fail closed.
- High-risk protocol/session/admission/persistence/item/loot/value/multichannel/fencing implementation changes require genuinely independent exact-head review.
- QA-E2E real-boundary evidence remains required for terminal vertical-slice proof.
- Production/protected-environment/live data/session/account, Platform/external-repository and owner-funded AI authority remain separately governed.

## Executor state

```text
EXECUTOR_PROGRAMME: RELEASED
DEFAULT_ENTRYPOINT: Oteryn: implementation coordinator
DIRECT_WORKERS: ALLOCATION_GATED
IMPLEMENTATION_WORKERS_STARTED: NO
IMPLEMENTATION_AUTHORITY_OUTSIDE_LIVE_COORDINATOR_ALLOCATION: NONE
```

## Context checkpoint

```yaml
last_progress: first-wave and Stage-C architecture accepted/lifecycle-closed; final 17-prompt executor DAG merged through #314 and coordinator programme released
status: ready_for_owner_initiated_implementation
owner_action_required: explicit invocation of `Oteryn: implementation coordinator` when implementation should start
blocker: none for starting the bounded coordinator programme; lane-specific evidence/authority/resource holds remain binding
authorized_implementation_started: false
next_action: owner invokes `Oteryn: implementation coordinator`; coordinator verifies live main and creates the serial Bootstrap allocation
executor_programme: RELEASED
```

`PRODUCTION_AUTHORITY: NONE`
