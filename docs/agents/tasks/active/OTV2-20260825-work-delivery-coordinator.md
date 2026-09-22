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
protected_main_sha: 40e9d723392b8fc1be652bdbb4b6d31b6729867b
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

Issue #162 remains the unique programme allocation/control-plane lifecycle. The active mutating
profile is `OTV2_WORK_DELIVERY_COORDINATOR`. Live GitHub state outranks this packet.

This file is a **single current checkpoint**. Historical coordinator checkpoints remain at
`docs/agents/evidence/OTV2-20260921-work-delivery-coordinator-history.md`.

## Current checkpoint

- Protected Game main: `40e9d723392b8fc1be652bdbb4b6d31b6729867b`.
- WP3 successor PR #673 is protected-complete as `3a384864d84560eb6ce76136afeb038576dc2976`.
- WP4 Child B PR #335 is protected-complete as `f02beb42523af6db1bb0c71d2840961e3fa5fcd0`; its stale active task packet is being archived by #740.
- WP5 Issue #319 is the current material critical path. S1 PR #735 is protected at
  `c59d8b25f9e3017d013d578a5a4bc9d93fa49e1d`; S2 and material #416 routing are active on their existing canonical branches.
- WP5 G0/source-composition readiness is **not yet proven**. Do not resume Server Seam early.
- Server Seam Issue #247 remains preserved at
  `agent/otv2-gameplay-server-seam-01@9370b254c6ac4f6529e069c1968ae6bfa1e1750e`.
- Content Item identity PR #737 remains open/draft at frozen head
  `0e643dbb5981599702b3407256e8ed03f1cad943`. Its scope is identity/resource closure only.
  D6-M1 Item schema-readiness starts only after #737 protects and fresh custody is computed.
- Governance cleanup #740 is path-disjoint from product/runtime lanes and must not broaden into them.

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
PROTECTED_COMPLETE:
  WP3 #673
  WP4 #335
  WP5 S1 #735

ACTIVE / QUALIFY:
  WP5 #319 S2 + material #416 routing
  Content Item #737
  Governance hygiene #740 (path-disjoint)

HELD:
  Server Seam #247 -> WP5_G0_READINESS_PROVEN
  downstream Client/QA -> Server Seam
  Movement -> Client/QA
  Combat -> Movement
```

## Validation / closeout rule

A downstream release or terminal closeout requires its actual accepted gate, not stale prose:
exact-head qualification/review where applicable, governed Merge Queue, real `merge_group`
aggregate `game-gate`, protected-main readback and lifecycle/ownership release.

## Context checkpoint

```yaml
last_progress: >-
  WP3 #673 and WP4 #335 are protected-complete; WP5 #319 has active S2 plus material
  #416 routing, while Server Seam remains frozen pending actual WP5 G0 readiness.
status: implementing
programme_state: ACTIVE
active_control_plane_profile: OTV2_WORK_DELIVERY_COORDINATOR
protected_main_sha: 40e9d723392b8fc1be652bdbb4b6d31b6729867b
wp3_issue: 351
wp3_pr: 673
wp3_state: PROTECTED_COMPLETE
wp4_issue: 329
wp4_pr: 335
wp4_state: PROTECTED_COMPLETE
wp5_issue: 319
wp5_state: ACTIVE_S2_AND_ROUTING
server_seam_issue: 247
server_seam_state: WAITING_WP5_G0
content_item_pr: 737
content_item_state: QUALIFYING_DRAFT
governance_cleanup_issue: 740
governance_cleanup_state: IMPLEMENTING_PATH_DISJOINT
owner_action_required: null
next_action: >-
  continue the existing WP5 S2/routing/composition sequence; prove WP5 G0 before resuming the
  preserved Server Seam, while qualifying #737 and #740 independently inside their exact scopes
```
