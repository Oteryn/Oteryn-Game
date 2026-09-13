# OTV2-20260913-work-dispatcher-lane-blocker-continuation

```yaml
task_id: OTV2-20260913-work-dispatcher-lane-blocker-continuation
title: Harden Work dispatcher continuation and context efficiency
mode: COORDINATE
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: coord/work-dispatcher-lane-blockers-20260913
issue: 162
pr: 596
owner: OTV2_WORK_DELIVERY_COORDINATOR
created_at: 2026-09-13
updated_at: 2026-09-13
execution_policy: continuous_progress
owned_paths:
  - docs/agents/prompts/OTV2_WORK_DELIVERY_COORDINATOR.md
  - docs/agents/tasks/active/OTV2-20260913-work-dispatcher-lane-blocker-continuation.md
  - docs/agents/PROMPT_LIFECYCLE.json
public_contracts: []
depends_on:
  - issue:162
  - issue:364
blocks: []
```

## Outcome

Make the active Work coordinator a thin dispatcher: lane-local blockers do not terminate the programme, unchanged evidence is reused, and each worker receives only the bounded context required for one task.

## Required semantics

- `LANE_BLOCKED` is distinct from `PROGRAMME_BLOCKED`;
- recompute the full live DAG after every terminal worker/blocker/integration result;
- schedule other legal work instead of self-routing the coordinator and stopping;
- use minimal worker context packets with optional `lazy_refs`;
- cache evidence by exact main/head/review/check/allocation generation;
- use anti-loop fingerprints for blocked/retryable actions;
- allow at most two retries on an unchanged fingerprint unless new diagnostics appear;
- rank work by critical-path value, blocker reduction, readiness, overlap risk and context cost;
- require one machine-readable terminal state from every worker;
- reserve `DONE` for non-mutating terminal work or post-protected-integration/readback/closeout of a mutating lane;
- require mutating pre-integration candidates to return `READY_FOR_INTEGRATION`, never `DONE`;
- stop the programme only after a full-DAG scan proves no legal useful work remains;
- preserve current authority, path ownership, review and protected integration rules.

## Prompt-debt reduction

The Work prompt is intentionally a compact execution profile over `OTV2_IMPLEMENTATION_COORDINATOR`. It does not restate historical wave sequencing; current programme/allocation records remain authoritative for live order.

## Corrective custody reconciliation

PR #596 reached protected `main` before the final review P1 about lifecycle-registry ownership was repaired. This corrective successor records the full path set actually required by that delivery. `docs/agents/PROMPT_LIFECYCLE.json` is therefore explicitly part of this task's serialized governance custody.

At corrective admission on protected `main@5d67ebb88b1775af665452b1b4ea1696ecfe6a98`, the live open PR path scan for the other current documentation/control-plane candidates checked (#592, #588 and #585) shows no overlapping write to `docs/agents/PROMPT_LIFECYCLE.json`. This corrective task changes only this task record and does not reopen or mutate the lifecycle registry itself.

## Scope

Documentation/governance only. No product/runtime implementation or integration action is performed by this task.

## Validation

Require exact-head Agent Governance, Architecture Semantic Audit and Merge Gate plus one fresh independent review on the final corrective head. Existing allocations and authority remain unchanged.
