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
protected_main_sha: 256aa3b152c944cb8451906effe1f0090c5b798d
owner: ChatGPT Work Delivery Coordinator
created_at: 2026-08-25T23:13:10+02:00
updated_at: 2026-09-21
execution_policy: continuous_progress
owned_paths:
  - docs/agents/programs/OTERYN_V2_IMPLEMENTATION_LIVE_ALLOCATIONS.md
  - docs/agents/tasks/active/OTV2-20260825-work-delivery-coordinator.md
public_contracts: []
depends_on: []
blocks:
  - WP3_PROTECTED_TERMINAL_READBACK_FOR_WP4_WP5_SERVER_SEAM
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories: []
```

## Current authority

Issue #162 remains the unique programme allocation/control-plane lifecycle. The active mutating
profile is `OTV2_WORK_DELIVERY_COORDINATOR`; reusable Terra or other coordinator aliases do not
receive concurrent mutation authority.

Live GitHub state outranks this packet. This file is a **single current checkpoint**, not a cumulative
programme ledger. Historical coordinator checkpoints are preserved at
`docs/agents/evidence/OTV2-20260921-work-delivery-coordinator-history.md`.

## Current checkpoint

- Protected Game base observed for this cleanup: `256aa3b152c944cb8451906effe1f0090c5b798d`.
- PR #713 is protected-integrated, so bounded API-native authoring is available under current root governance.
- WP3 / Issue #351 / PR #673 is the critical path. Current live head:
  `cacb03c3863d92c62ff5c12e58b0739e17f4fe70`.
- PR #673 comment `5761030944` reports the minimal owner-scoped `CommitOutcomeUnknown`
  repair qualified; comment `5761200059` requests the single final exact-head review.
- Owner direction in #162 comment `5760534132` keeps maintenance-burst and panic-cleanup
  hardening deferred/non-blocking for the first playable slice. Do not re-open them absent new
  first-slice evidence.
- WP4 Child B (#329 / PR #335), WP5 source composition (#319) and Server Seam (#247) remain
  held for protected WP3 terminal readback plus fresh overlap/custody reconciliation.
- Content/World PR #719 is a path-disjoint draft 64-item native-binding batch and may qualify
  independently within its exact four-path scope.
- Physical spatial Content remains blocked by the Reference spatial evidence binding gate.
- CONTENT-QUEST-01 PR #709 remains a stable qualified candidate; do not manufacture head movement
  merely to regenerate integration evidence.

## Ownership discipline

The coordinator owns only the shared coordination surfaces named in `owned_paths`.
Child tasks own their exact runtime/source/test paths. Before any new allocation:

1. fresh-read protected `main`;
2. fresh-read the target Issue/task/PR/head;
3. detect actual overlapping active writers/leases;
4. allocate the smallest exact path set;
5. preserve existing canonical branch/PR history.

Historical task text, closed PRs and archived allocations never create live custody.

## Current dependency shape

```text
ACTIVE / QUALIFY:
  WP3 #673
  Content #719 (path-disjoint)

HELD FOR WP3 PROTECTED:
  WP4 #329 / PR #335
  WP5 #319 material lanes
  Server Seam #247

BLOCKED EXTERNAL EVIDENCE:
  physical Content spatial slice

STABLE QUALIFIED CANDIDATE:
  CONTENT-QUEST-01 #709
```

## Validation / closeout rule

Do not declare downstream release from PR-head CI alone. Required terminal evidence remains:

- exact-head qualification/review where applicable;
- governed Merge Queue integration;
- real `merge_group` aggregate `game-gate`;
- protected-main readback;
- task/archive ownership release.

## Context checkpoint

```yaml
last_progress: >-
  Context/lifecycle cleanup #714 reduced this active coordinator packet to current state only;
  historical checkpoint prose moved to evidence without changing programme authority.
status: implementing
programme_state: ACTIVE
active_control_plane_profile: OTV2_WORK_DELIVERY_COORDINATOR
protected_main_sha: 256aa3b152c944cb8451906effe1f0090c5b798d
wp3_issue: 351
wp3_pr: 673
wp3_head_sha: cacb03c3863d92c62ff5c12e58b0739e17f4fe70
wp3_state: QUALIFICATION_REVIEW_PENDING
wp4_issue: 329
wp4_pr: 335
wp4_state: HELD_UNTIL_WP3_PROTECTED
wp5_issue: 319
wp5_state: HELD_UNTIL_WP3_PROTECTED
server_seam_issue: 247
server_seam_state: WAITING_DEPENDENCY
content_native_item_pr: 719
content_native_item_state: ACTIVE_DRAFT_PATH_DISJOINT
owner_action_required: null
next_action: >-
  consume PR #673 final review; if current-gate clean integrate through the governed route,
  then fresh-reconcile #329/#319/#247 before releasing downstream material work
```
