> Lifecycle closeout: **COMPLETED / ARCHIVED / OWNERSHIP RELEASED**. Delivery PR #720 exact candidate `86b03128cb1f662ebdea202b12525db04832e7bb` passed real Merge Queue run `35615586460` with aggregate `game-gate` job `106388104993` SUCCESS and integrated as `39fde6956c582e0757740de87f845bfb08005f50`. Protected `main@5a28b53108b5a9e6c570e6dd3dbb2e9bad0a9555` contains the accepted cleanup chain. Any nonterminal/checkpoint wording below is historical provenance only.

# OTV2-20260921-agent-lifecycle-cleanup-714

```yaml
task_id: OTV2-20260921-agent-lifecycle-cleanup-714
title: Reconcile stale active tasks and live allocation state
mode: GOVERNANCE
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: docs/agent-lifecycle-cleanup-714
issue: 714
pr: 720
base_sha: 256aa3b152c944cb8451906effe1f0090c5b798d
head_sha: 86b03128cb1f662ebdea202b12525db04832e7bb
final_head_sha: 86b03128cb1f662ebdea202b12525db04832e7bb
final_head_frozen_at: null
owner: ChatGPT
created_at: 2026-09-21T12:23:17Z
updated_at: 2026-09-21T13:40:00Z
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/OTV2-20260827-multichannel-world-architecture-continuation.md
  - docs/agents/tasks/active/OTV2-20260904-gameplay-server-seam-allocation.md
  - docs/agents/tasks/active/OTV2-20260907-pg-target-large-diff-420.md
  - docs/agents/tasks/active/OTV2-20260910-reference-investigation-prompts-486.md
  - docs/agents/tasks/active/OTV2-20260910-reference-world-corridor-census-511.md
  - docs/agents/tasks/active/OTV2-20260910-runtime-actor-local-generation-539.md
  - docs/agents/tasks/active/OTV2-20260911-native-ui-prompt-activation.md
  - docs/agents/tasks/active/OTV2-20260911-runtime-actor-carrier-preproduction-530.md
  - docs/agents/tasks/active/OTV2-20260912-neutral-doc-lane-drift.md
  - docs/agents/tasks/active/OTV2-20260912-ref-combat-death-corpse-loot-evidence.md
  - docs/agents/tasks/active/OTV2-20260912-wp3-v2-architecture-decision.md
  - docs/agents/tasks/active/OTV2-20260912-wp3-v2-multi-agent-programme-registration.md
  - docs/agents/tasks/active/OTV2-20260913-wp3-v2-gate1-acceptance-allocation.md
  - docs/agents/tasks/active/OTV2-20260914-large-pr-enumeration-620.md
  - docs/agents/tasks/active/OTV2-20260914-neutral-doc-consumer-baseline-refresh.md
  - docs/agents/tasks/active/OTV2-20260915-defect-discovery-prompts.md
  - docs/agents/tasks/active/OTV2-20260916-upstream-first-dependency-doctrine.md
  - docs/agents/tasks/active/OTV2-20260916-wp3-playable-first-q-triage.md
  - docs/agents/tasks/active/OTV2-20260917-content-world-execution-prompts.md
  - docs/agents/tasks/active/OTV2-20260918-content-world-cw4-local-object-runtime-504.md
  - docs/agents/tasks/active/OTV2-20260918-content-world-d3-real-batch-bundle-measurement.md
  - docs/agents/tasks/active/OTV2-20260918-content-world-fresh-crystal-source-profile-504.md
  - docs/agents/tasks/active/OTV2-20260919-content-world-cw2-b3-loot-item-bindings-504.md
  - docs/agents/tasks/active/OTV2-20260919-content-world-cw3-project-publication-recovery-504.md
  - docs/agents/tasks/active/OTV2-20260919-wp3-v2-retained-config-limits.md
  - docs/agents/tasks/active/OTV2-20260919-wp3-v2-tokio-maintenance-task-layout-amendment.md
  - docs/agents/tasks/active/OTV2-20260920-content-world-cw2-b1-vase-project-binding-504.md
  - docs/agents/tasks/active/OTV2-20260921-agent-context-economy.md
  - docs/agents/tasks/archive/**
  - docs/agents/programs/OTERYN_V2_IMPLEMENTATION_LIVE_ALLOCATIONS.md
  - docs/agents/tasks/active/OTV2-20260825-work-delivery-coordinator.md
  - tools/agents/validate_governance.py
  - docs/agents/tasks/active/OTV2-20260921-agent-lifecycle-cleanup-714.md
public_contracts:
  - agent task lifecycle and current allocation-state routing
depends_on:
  - pr:713
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Archive terminal task records that remain in the hot-path active directory, release stale task ownership, and reduce current allocation/coordinator surfaces to current operational state while preserving historical provenance outside the hot path.

## Source of truth

- PROVEN: PR #713 merged as protected `256aa3b152c944cb8451906effe1f0090c5b798d`, restoring bounded API-native authoring.
- PROVEN: Issue #714 records the terminal-task and allocation-monolith census.
- PROVEN: `DELIVERY_COMPLETENESS_AND_CLOSEOUT.md` requires post-merge task archival and ownership release.
- PROVEN: live GitHub PR/Issue state outranks task prose.

## High-risk authority/recovery qualification

NOT_APPLICABLE — documentation/governance lifecycle cleanup only.

## Acceptance criteria

- [x] 31 terminal task packets were archived with live merged-PR evidence or equivalent terminal delivery evidence.
- [x] No task was archived solely because an Issue is old/open; terminality was live-reconciled.
- [x] `tasks/active` reduced from 61 to 31 files including this cleanup task.
- [x] `LIVE_ALLOCATIONS` reduced from 73,679 chars / 1,120 lines to 4,361 chars / 64 lines; full prior ledger preserved in evidence.
- [x] Active Work coordinator packet reduced from 39,916 chars / 375 lines to 4,816 chars / 130 lines with exactly one current checkpoint; full prior ledger preserved in evidence.
- [x] Governance validation now rejects duplicate active/archive task packets and regrowth of the two current-state ledgers.
- [ ] Exact-head governance/semantic/merge-gate validation passes.

## Excluded scope

No runtime/gameplay/protocol/persistence/production/external-repository mutation. No reusable-prompt retirement in this task.

## Context checkpoint

```yaml
last_progress: >-
  archived 31 terminal task packets; compacted LIVE_ALLOCATIONS and the active Work coordinator
  packet; preserved both complete historical ledgers under docs/agents/evidence; added hygiene guards
status: validating
branch: docs/agent-lifecycle-cleanup-714
head_sha: null
pr: null
next_action: freeze exact remote head, open PR #714 delivery candidate, and require exact-head governance/semantic/merge-gate validation
```


## Terminal closeout

- issue: #714
- delivery PR: #720
- frozen exact head: `86b03128cb1f662ebdea202b12525db04832e7bb`
- real Merge Queue run: `35615586460`
- merge-group aggregate `game-gate`: job `106388104993` — SUCCESS
- integrated commit: `39fde6956c582e0757740de87f845bfb08005f50`
- final protected-main readback for this cleanup chain: `5a28b53108b5a9e6c570e6dd3dbb2e9bad0a9555`
- ownership: released
