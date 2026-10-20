# OTV2-20261011-b6-evidence-bind-1

```yaml
task_id: OTV2-20261011-b6-evidence-bind-1
title: B6-EVIDENCE-BIND-1 bind the offline B6 evidence check to the checkout
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/b6-evidence-bind-1-20261011
pr: 1996
base_sha: d538d5d97
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: B6-EVIDENCE-BIND-1 writer (control plane session_01CwP6d84eCPvpgoEuyci8Tx)
created_at: 2026-10-11T00:00:00Z
updated_at: 2026-10-11T00:00:00Z
execution_policy: continuous_progress
owned_paths:
  - .github/workflows/content-world-b6-interaction-bindings.yml
  - tools/reference-world-corridor-census/README.md
  - docs/agents/tasks/OTV2-20261011-b6-evidence-bind-1.md
  - docs/agents/tasks/archive/OTV2-20261011-b6-evidence-bind-1.md
public_contracts: []
depends_on:
  - OTV2-20261011-decanary-game-2a
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

This task resolves Codex P1 4242674790 and P2 4242479561 from #1986, which merged on a803e412
before the fix could land there.

The offline check in `Content World B6 interaction bindings` (step `Verify tracked B6 evidence
closure offline`) is now bound to the checkout:

- Every recorded `game_inputs` path is hashed from the checkout, both the HEAD blob and the
  worktree bytes, and the check fails on a mismatch.
- Every family and shard `tracked_path` must equal the full path the generator writes.
- The git-tracked and on-disk file sets must equal the referenced set.
- Every record must re-normalize unchanged with the checked-out mapper.
- The index, family and shard bytes must equal an offline rebuild with the checked-out generator.

Expected values come from the checked-out generator, not from a second hand-kept copy in the
workflow.

## Architecture and source of truth

- `interaction_binding_catalog.py` is the source of truth for the pins, paths and payload layout.
  The check imports it and reuses `normalize_observation`, `build_catalog` and `storage_payloads`.
- Audit of recorded values that are still compared with a constant: the external source and
  parser pins. CI no longer fetches those repositories (GAME-DECANARY-2a), so they can only be
  compared with the generator constants, through the byte-identical rebuild. The pinned logical
  product digest stays in the workflow.

## Acceptance criteria

- [x] A committed edit to a recorded Game input fails the check.
- [x] A `tracked_path` that keeps its basename under another directory fails the check.
- [x] No recorded Game input is compared only with a hand-kept constant.

## Excluded scope

- The tracked evidence and the generator are unchanged.
- The external source and parser cannot be checked offline.
- Changes to `collect_real_observations` stay uncovered until a source rebuild.

## Implementation / findings

- Replaced the inline closure script in the workflow and updated its header comment.
- Documented the check in `tools/reference-world-corridor-census/README.md`.

## Validation

### Focused

- B6 offline closure check, extracted from the workflow YAML: pass (13 files, 6932 occurrences); it now also asserts
  evidence paths against literals matching the workflow triggers, and the no-promotion invariant (index counters,
  `closure`, `authority` block and per-record authority fields)
- Negative cases, all rejected:
  - `producer.py` edited, both worktree-only and committed
  - a changed recorded `game_inputs` blob
  - a `bogus/` index `tracked_path` that keeps the basename
  - a tampered shard record
  - an unreferenced extra file
  - a changed parser pin
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass (60 tests, OK)
- `git diff --check`: clean
- `python tools/repository/validate_repository_policy.py`: pass (23 files, 63 workflows)
- `tools/repository/test_*.py` (run as scripts): 10 of 12 pass; `test_validate_merge_group_pg_sim.py` and
  `test_validate_pr_gate_pg_sim.py` BLOCKED locally (pwsh not installed, required for WP1 native
  failure-propagation qualification); CI covers them

### Component/integration

- NOT_APPLICABLE: CI-only check; repository-policy results are under Focused.

### E2E

- NOT_APPLICABLE: CI-only check.

### Exact-head CI

- final head: see PR
- result: pending

## Self-review

- exact head: see PR
- method/reviewer: implementing agent
- verdict: no findings

## Independent review

- required: control plane decides

## PR and closeout

- PR: 1996
- ownership release: after merge

## Context checkpoint

```yaml
last_progress: check bound to the checkout, negative cases verified
status: completed
branch: claude/b6-evidence-bind-1-20261011
head_sha: null
pr: 1996
final_head_sha: null
next_action: control plane review and freeze
```
