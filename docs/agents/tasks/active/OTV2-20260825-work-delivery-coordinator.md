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
protected_main_sha: 961c74ab573d87807ed24cbd414a46d8072f72d3
owner: ChatGPT Work Delivery Coordinator
created_at: 2026-08-25T23:13:10+02:00
updated_at: 2026-09-27
execution_policy: continuous_progress
owned_paths:
  - docs/agents/programs/OTERYN_V2_IMPLEMENTATION_LIVE_ALLOCATIONS.md
  - docs/agents/tasks/active/OTV2-20260825-work-delivery-coordinator.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories: []
```

## Current authority

Issue #162 remains the unique programme allocation/control-plane lifecycle. The active mutating profile is `OTV2_WORK_DELIVERY_COORDINATOR`. Live GitHub state outranks this packet.

This file is a **single current checkpoint**. Historical coordinator checkpoints remain at `docs/agents/evidence/OTV2-20260921-work-delivery-coordinator-history.md`.

## Current checkpoint

- Admission protected Game main for this checkpoint: `961c74ab573d87807ed24cbd414a46d8072f72d3`.
- WP5 G0 is met (`WP5_G0_READY`, #319 comment `5809797683`). Server Seam #247 is closed (PR #823); ClientResume #822, Item schema readiness #749 and governance #745 are complete.
- The current vertical is "click -> Use / backpack / door -> client result". Its lane register and gates are in #162 comment `5860494545` and the live allocations snapshot.
- Content/World allocations are issued as #162 comments (latest `5860394705`); released tasks keep their own active packets.
- Architecture package A1-A3 (GAME-INTERACTION-01 successor + D37/D38 acceptance, the next control-wire owner decision, the scope progression input owner) is with the Supervising Architect and blocks no other lane.

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
WAITING:
  CW4 door execution -> CW3 local-object state model
  impl interaction -> architecture A1
  seam command dispatch + native client entry -> architecture A2
  pickup / inventory -> #513 B3 allocation (DUR-03 lineage)
  UI input -> L-CARGO release + UI-P1
```

## Validation / closeout rule

A downstream release or terminal closeout requires its actual accepted gate, not stale prose: exact-head qualification/review where applicable, governed Merge Queue, real `merge_group` aggregate `game-gate`, protected-main readback and lifecycle/ownership release.

## Context checkpoint

```yaml
last_progress: refreshed stale G0/Server Seam routing; recorded the click-Use-backpack-door lane register and architecture package A1-A3 on #162
status: implementing
programme_state: ACTIVE
active_control_plane_profile: OTV2_WORK_DELIVERY_COORDINATOR
protected_main_sha: 961c74ab573d87807ed24cbd414a46d8072f72d3
wp5_g0_state: READY
server_seam_issue: 247
server_seam_state: DONE
vertical_click_use_state: WAITING_ARCHITECTURE_AND_CUSTODY
owner_action_required: null
next_action: integrate released Content/World tasks; resume each vertical lane when its #162-recorded gate is met
```
