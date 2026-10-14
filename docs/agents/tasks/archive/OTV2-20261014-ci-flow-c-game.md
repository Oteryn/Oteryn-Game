# OTV2-20261014-ci-flow-c-game

```yaml
task_id: OTV2-20261014-ci-flow-c-game
title: "CI-FLOW-C-GAME: scope the Quest docs check and run cheap gates first"
mode: IMPLEMENTATION
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/ci-flow-c-game
pr: 2102
head_sha: "exact frozen head in the report to the control plane"
final_head_sha: "exact frozen head in the report to the control plane"
owner: claude-code worker for control plane session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-14
updated_at: 2026-10-14
execution_policy: continuous_progress
owned_paths:
  - .github/workflows/quest-authoring-schema.yml
  - tools/repository/classify_quest_authoring_scope.py
  - tools/repository/test_classify_quest_authoring_scope.py
  - tools/content-schema/quest-authoring/run_checks.py
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/tasks/archive/OTV2-20261014-ci-flow-c-game.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories:
  - Oteryn/Oteryn-Platform
```

## Outcome

- Accepted package-C items of the #2100 review (owner: "wprowadz te optymalizacje"). A PR that only adds or modifies unreferenced `.md` files under the Quest tool directory skips `run_checks.py`; `run_checks.py` runs its slowest step last. The Merge gate job-ordering change is not part of this PR: merge-gate.yml is blob-pinned by the merge-authority audit (`EXPECTED_MERGE_GATE_BLOB`), whose rotation needs a separate owner-authorized PR. Platform acceptance coverage map is a separate docs PR (Oteryn-Platform #1612).

## Architecture and source of truth

- PROVEN: `quest-authoring-schema.yml` is not part of `game-gate`; `game-gate` and `validate` are unchanged and still always report.
- PROVEN: README pins live in per-directory manifests (`donor_sources/maps/qualification.json`, three `handoff.json`), which the classifier detects by relative path and content hash.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: CI routing only. No ruleset, branch protection, merge queue, deployment or production change.

## Acceptance criteria

- [x] No blanket `*.md` ignore; pinned README, mixed diff, rename/delete, workflow, build, dependency and unknown inputs take the full path.
- [x] Tests prove each case class.
- [x] merge-gate.yml and the audit pins are untouched.
- [x] Deferred and rejected items are not implemented.

## Excluded scope

- Fast-PR/full-library-in-MQ split, World materializer narrowing, UI routing, PG16/17 reuse, any ruleset or merge queue change.

## Validation

### Focused

- `python -m unittest test_classify_quest_authoring_scope` (tools/repository): OK, 16 tests
- `python tools/repository/validate_repository_policy.py`: pass
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: OK
- `git diff --check`: pass

### Component/integration

- `NOT_APPLICABLE`: workflow routing only.

### E2E

- `NOT_APPLICABLE`: workflow routing only.

### Exact-head CI

- final head: the PR head
- result: the PR checks on that head

## Self-review

- exact head: the frozen head
- method/reviewer: task writer
- material findings: none open
- verdict: ready for review

## Independent review

- required: the control plane owns the review call.
- verdict: pending control plane.

## PR and closeout

- changed-file review: workflow, tools/repository, docs paths
- unresolved review threads: none
- protected auto-merge: control plane
- ownership release: on merge

## Context checkpoint

```yaml
last_progress: implemented, tests green locally
status: completed
branch: claude/ci-flow-c-game
pr: 2102
blocker: null
next_action: control plane freezes and sends the PR to review and merge
```
