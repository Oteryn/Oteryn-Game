# OTV2-20260824-content-evidence-activation-fence-repair

```yaml
task_id: OTV2-20260824-content-evidence-activation-fence-repair
title: Repair non-production CONTENT activation boundary
mode: IMPLEMENT
status: review_pending
repository: Oteryn/Oteryn-Game
base_branch: main
branch: repair/content-evidence-activation-fence-20260824
issue: 85
allocation_pr: 86
pr: 87
allocation_base_sha: d9d927acfcebe0c61c0e8e826bae170767b12730
worker_base_sha: 19329df11eb5c605e338a472c277ac023a8d7c43
owner: chat-github-20260818-implementation-coordinator
created_at: 2026-08-24T14:10:00+02:00
updated_at: 2026-08-24T14:10:00+02:00
owned_paths_after_allocation_merge:
  - apps/game-server/src/content/mod.rs
  - apps/game-server/src/content/artifact.rs
  - docs/agents/tasks/active/OTV2-20260824-content-evidence-activation-fence-repair.md
shared_paths: []
external_repositories: []
```

## Root cause

Merged PR #58 exposes `ActivationSlot::stage_and_activate` and `ActiveContent` through the production public `content` module even though the only physical artifact profile is explicitly `VSL_BUNDLE_EVIDENCE_PROFILE/v1/non-production` and DUR-04 production activation authority is `NONE`.

The defect was independently reproduced after merge. A TDD `compile_fail` doctest attempting to import `oteryn_game_server::content::ActivationSlot` compiled successfully on the merged tree, proving the production public API boundary was open.

## Required repair

- add the compile-fail regression to `content/mod.rs`;
- keep `ActiveContent`, `ActivationSlot` and their impls available only under `#[cfg(test)]`;
- keep evidence staging/parser/compiler/projection bytes and tests unchanged;
- keep `OrdinaryRelease` rejected and `GameplayAvailability::UnavailableBootstrap` unchanged;
- do not change registries, contracts, workflows, Cargo/workspace topology, permanent format, limits or production activation authority.

## Acceptance

- [x] observed RED on merged-tree production API import;
- [x] GREEN compile-fail doctest after the fence;
- [x] game-server focused tests PASS;
- [x] package strict Clippy PASS;
- [x] full workspace tests and strict Clippy PASS;
- [x] architecture-check, governance and `git diff --check` PASS;
- [ ] mandatory whole-diff self-review PASS on frozen exact head;
- [ ] genuinely independent exact-head review PASS with P0=0/P1=0/P2=0;
- [ ] exact-head repository `game-gate` PASS;
- [ ] squash merge with expected-head fence, Issue #85 closeout and repair task archive.

## Review classification

Independent exact-head review is **REQUIRED** because this repair closes a previously missed P0 in a production/public boundary around content activation. Historical review evidence for PR #58 cannot be reused.

## Context checkpoint

```yaml
last_progress: allocation PR #86 merged as 19329df11eb5c605e338a472c277ac023a8d7c43; RED was reproduced on the merged CONTENT tree; the minimal cfg(test) fence makes the compile-fail regression GREEN; doc tests, game-server 129/129, package/workspace strict Clippy, full workspace tests, architecture-check, governance and diff-check pass locally
status: review_pending
branch: repair/content-evidence-activation-fence-20260824
head_sha: pending_final_freeze
pr: 87
blocker: mandatory whole-diff self-review, genuinely independent exact-head review and exact-head repository CI remain before merge
owner_action_required: null
next_action: commit and push only the two code files plus this repair task, open the repair PR, freeze exact head, review it independently, require exact-head game-gate and merge only if clean
```
