# OTV2-20260913-work-dispatcher-lane-blocker-continuation

```yaml
task_id: OTV2-20260913-work-dispatcher-lane-blocker-continuation
title: Keep Work dispatcher active across lane-local blockers
mode: COORDINATE
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: coord/work-dispatcher-lane-blockers-20260913
owner: OTV2_WORK_DELIVERY_COORDINATOR
created_at: 2026-09-13
updated_at: 2026-09-13
execution_policy: continuous_progress
owned_paths:
  - docs/agents/prompts/OTV2_WORK_DELIVERY_COORDINATOR.md
  - docs/agents/tasks/active/OTV2-20260913-work-dispatcher-lane-blocker-continuation.md
public_contracts: []
depends_on:
  - issue:162
  - issue:364
blocks: []
```

## Outcome

Refine the active Work coordinator dispatch semantics so a blocker on one lane does not terminate the whole programme while other legal useful work exists.

## Required semantics

- distinguish `LANE_BLOCKED` from `PROGRAMME_BLOCKED`;
- after every worker result, blocker or capability failure, refresh live state and recompute the complete programme DAG;
- continue the highest-value legally runnable independent task instead of self-routing the coordinator and stopping;
- park blocked lanes with exact recheck triggers and avoid busy polling;
- permit programme termination only when a fresh full-DAG pass proves zero runnable mutating work, zero useful independent evidence/review work, zero useful bounded read-only preparation and zero coordinator action that can advance a gate;
- preserve all existing authority, path-ownership, Merge Queue, architecture and production restrictions.

## Scope

Documentation/governance prompt semantics only. No runtime, Cargo, vendor, SQL/migration, workflow/ruleset, production, external-repository or merge mutation.

## Validation

Require repository-native exact-head governance/semantic checks on the final candidate. Existing live allocations and integration authority remain unchanged.
