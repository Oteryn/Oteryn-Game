# OTV2-20260825-work-delivery-coordinator

```yaml
task_id: OTV2-20260825-work-delivery-coordinator
title: Coordinate the post-blocker gameplay vertical slice
mode: COORDINATE
status: implementing
programme_state: ACTIVE
active_control_plane_profile: OTV2_WORK_DELIVERY_COORDINATOR
repository: Oteryn/Oteryn-Game
base_branch: main
branch: null
issue: 162
pr: null
protected_main_sha: 86e25ab6c830159d9cb32aee1c3c8ff7726cdcd1
owner: ChatGPT Work Delivery Coordinator
created_at: 2026-08-25T23:13:10+02:00
updated_at: 2026-09-22
execution_policy: continuous_progress
owned_paths:
  - docs/agents/programs/OTERYN_V2_IMPLEMENTATION_LIVE_ALLOCATIONS.md
  - docs/agents/tasks/active/OTV2-20260825-work-delivery-coordinator.md
public_contracts: []
depends_on: []
blocks:
  - WP5_G0_READINESS_FOR_SERVER_SEAM
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories: []
```

## Current authority

Issue #162 remains the unique programme allocation/control-plane lifecycle. The active mutating profile is `OTV2_WORK_DELIVERY_COORDINATOR`. Live GitHub state outranks this packet.

This file is a **single current checkpoint**. Historical coordinator checkpoints remain at `docs/agents/evidence/OTV2-20260921-work-delivery-coordinator-history.md`.

## Current checkpoint

- Admission protected Game main for this checkpoint: `86e25ab6c830159d9cb32aee1c3c8ff7726cdcd1`.
- WP5 Issue #319 is the material Server-Seam critical path. Material #416 routing PR #739 is protected as `cf5c5f35476559450b6bbaf87dce519f7eead9d0`.
- S2 qualification/repair continues on canonical PR #757; S3-A real-interoperability remains a separately gated successor. WP5 G0/source-composition readiness is **not yet proven**.
- Server Seam Issue #247 remains preserved at `agent/otv2-gameplay-server-seam-01@9370b254c6ac4f6529e069c1968ae6bfa1e1750e` and must not resume before fresh `WP5_G0_READINESS_PROVEN`.
- Content D6-M1 Item Schema Readiness continues on canonical PR #749. The measured artifact resource profile is accepted; production schema/codec qualification is the active successor, not a new source import.
- Governance Issue #745 Phase 1 is protected. Phase 2 is waiting for a fresh bounded allocation; no Phase-2 branch/PR is implied by this checkpoint.
- Agent hygiene Issue #740 is in final closeout only; completed lifecycle records are no longer live writer custody.

## Ownership discipline

Before every new allocation or resumed writer:

1. fresh-read protected `main`;
2. fresh-read the exact target Issue/task/PR/branch/head;
3. detect overlapping active writers/leases;
4. allocate the smallest exact path set;
5. preserve canonical branch/PR history;
6. reject historical task/programme prose as live custody unless current authority explicitly renews it.

## Current dependency shape

```text
ACTIVE / QUALIFY:
  WP5 #319 -> S2 #757 + S3/source-composition successors
  Content D6-M1 #749
  Governance #745 Phase 2 -> fresh allocation required

HELD:
  Server Seam #247 -> WP5_G0_READINESS_PROVEN
  downstream Client/QA -> Server Seam
  Movement -> Client/QA
  Combat -> Movement
```

## Validation / closeout rule

A downstream release or terminal closeout requires its actual accepted gate, not stale prose: exact-head qualification/review where applicable, governed Merge Queue, real `merge_group` aggregate `game-gate`, protected-main readback and lifecycle/ownership release.

## Context checkpoint

```yaml
last_progress: lifecycle hygiene removed terminal task packets; #739 and #748 are protected while active work is WP5, D6-M1 and future #745 Phase 2
status: implementing
programme_state: ACTIVE
active_control_plane_profile: OTV2_WORK_DELIVERY_COORDINATOR
protected_main_sha: 86e25ab6c830159d9cb32aee1c3c8ff7726cdcd1
wp5_issue: 319
wp5_state: ACTIVE_S2_S3_COMPOSITION
server_seam_issue: 247
server_seam_state: WAITING_WP5_G0
content_item_pr: 749
content_item_state: ACTIVE_SCHEMA_READINESS
governance_simplification_issue: 745
governance_simplification_state: WAITING_PHASE2_ALLOCATION
owner_action_required: null
next_action: continue canonical WP5 and D6-M1 lineages; keep Server Seam held until G0; allocate #745 Phase 2 only after fresh overlap/custody readback
```
