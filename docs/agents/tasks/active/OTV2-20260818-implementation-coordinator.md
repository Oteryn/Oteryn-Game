# OTV2-20260818-implementation-coordinator

```yaml
task_id: OTV2-20260818-implementation-coordinator
title: Coordinate first native implementation wave
mode: COORDINATE
status: content_post_merge_p0_repair_allocation_pending
repository: Oteryn/Oteryn-Game
base_branch: main
branch: coord/content-activation-fence-repair-allocation-20260824
pr: 86
base_sha: d9d927acfcebe0c61c0e8e826bae170767b12730
head_sha: null
owner: chat-github-20260818-implementation-coordinator
created_at: 2026-08-18T16:10:00+02:00
updated_at: 2026-08-24T14:10:00+02:00
execution_budget_minutes: 60
owned_paths:
  - docs/agents/tasks/active/OTV2-20260818-implementation-coordinator.md
  - docs/agents/programs/OTERYN_V2_IMPLEMENTATION_LIVE_ALLOCATIONS.md
  - docs/agents/tasks/active/OTV2-20260822-impl-vsl-content.md
  - docs/agents/tasks/active/OTV2-20260824-content-evidence-activation-fence-repair.md
public_contracts:
  - docs/agents/programs/OTERYN_V2_IMPLEMENTATION_EXECUTOR_DAG.md
  - docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md
depends_on:
  - Oteryn-Game#82
  - Oteryn-Game#58
  - Oteryn-Game#84
  - Oteryn-Game#85
blocks: []
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
```

## Outcome

Coordinate the active native implementation programme, serialize shared mutations and keep merged-vs-blocked state durable without converting missing product decisions into implementation guesses.

## Current proven state

- `PROVEN`: FOUNDATION and DOMAIN are merged and lifecycle-released.
- `PROVEN`: CONTENT PR #58 merged as `8f99f25d0b1b3472d40504cd54b463cf752ebe7a`; PR #84 then released CONTENT write ownership and retained Issue #54 as a production-only blocker.
- `PROVEN`: a later genuinely independent review of the exact merged CONTENT tree reproduced one P0: production public API exports `ActivationSlot::stage_and_activate` for the explicitly non-production evidence profile.
- `PROVEN`: local TDD reproduction on the merged tree produced the expected RED compile-fail failure because `content::ActivationSlot` was publicly importable; a minimal two-file `cfg(test)` fence turns that regression GREEN without changing parser/compiler/staging behavior.
- `PROVEN`: Issue #85 tracks the repair; current canonical allocation has no CONTENT write authority, so code mutation must wait for a fresh merged allocation.
- `PROVEN`: Issue #54 remains separately blocked by missing accepted DUR-04/VSL production hard maxima and production activation authority.
- `PROVEN`: QA still has an evidence shell only; real Tier 1/Tier 2 gameplay journeys remain `NOT_EVALUATED`.
- `DERIVED`: DURABILITY remains dependency-ready; ABILITY/INTERACTION/AI and the content-format spike are held until the CONTENT P0 repair is terminal.

## Coordinator decision

Issue #85 is the only newly allocated repair candidate. This allocation becomes effective only after its coordinator PR merges. It grants exactly:

```yaml
lane_id: OTV2-REPAIR-CONTENT-ACTIVATION-FENCE
owned_paths:
  - apps/game-server/src/content/mod.rs
  - apps/game-server/src/content/artifact.rs
  - docs/agents/tasks/active/OTV2-20260824-content-evidence-activation-fence-repair.md
shared_paths: []
registry_contract_workflow_cargo_authority: none
```

The implementation must preserve the already-merged evidence parser/compiler/staging seam and only remove test-only activation publication from production API. A fresh genuinely independent exact-head review is mandatory because this is a repair of a previously missed P0.

DURABILITY remains separately dependency-ready but unallocated. CONTENT-dependent lanes remain on hold until #85 is terminal. No owner decision is needed for this repair because it restores an already accepted fail-closed boundary; the unrelated production CONTENT limits/activation decision in Issue #54 remains owner-gated.

## Validation / merge posture

This reconciliation is lifecycle/status documentation only. Runtime/component/E2E is `NOT_APPLICABLE`. Required before merge:

- exact four-file changed-path review;
- `python tools/agents/validate_governance.py`;
- `git diff --check`;
- repository-required exact-head `game-gate`/governance checks;
- mandatory whole-diff self-review;
- zero unresolved review threads.

Independent implementation review is `NOT_REQUIRED` because this change introduces no runtime, parser, item/value, protocol, persistence, security or production authority.

## Context checkpoint

```yaml
last_progress: post-merge independent review of CONTENT found a reproducible activation-boundary P0 on merged PR #58 tree; Issue #85 created; coordinator allocation candidate grants only the exact two code paths plus repair task after merge
status: content_post_merge_p0_repair_allocation_pending
branch: coord/content-activation-fence-repair-allocation-20260824
head_sha: null
pr: 86
blocker: repair code authority is not active until this allocation PR merges
owner_action_required: null
next_action: merge this bounded allocation after exact-head governance/self-review, then implement #85 via TDD, independent review and exact-head CI before releasing CONTENT-dependent lanes
```
