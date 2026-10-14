# OTV2-20261013-agents-batch-guideline

```yaml
task_id: OTV2-20261013-agents-batch-guideline
title: "Batch guideline: coherent reviewable unit, size is a signal only"
mode: GOVERNANCE
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/agents-batch-guideline-20261013
pr: 2037
head_sha: "exact frozen head in the report to the control plane"
final_head_sha: "exact frozen head in the report to the control plane"
owner: claude-code worker for control plane session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-13
updated_at: 2026-10-13
execution_policy: continuous_progress
owned_paths:
  - AGENTS.md
  - docs/agents/tasks/active/OTV2-20261013-agents-batch-guideline.md
  - docs/agents/tasks/archive/OTV2-20261013-agents-batch-guideline.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

- Root `AGENTS.md`, Work in batches, Fixes: a batch is one coherent reviewable unit; the
  500-line / five-finding figure is a warning signal only. The P0 rule is kept.
- Task records: archived in the final authoring commit of the completing PR; no archive-only PR.

## Architecture and source of truth

- PROVEN: owner approval in chat ("zgoda na stale"), relayed by the control plane.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: documentation only.

## Acceptance criteria

- [x] Fixes bullet no longer caps a batch at five findings or 500 lines.
- [x] Task records bullet forbids archive-only PRs.

## Excluded scope

- No workflow, ruleset or audit blob pin change.

## Validation

### Focused

- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: OK
- `git diff --check`: pass

### Component/integration

- `NOT_APPLICABLE`: documentation only.

### E2E

- `NOT_APPLICABLE`: documentation only.

## Self-review

- exact head: the frozen head
- method/reviewer: task writer
- material findings: none
- verdict: ready for review

## PR and closeout

- changed-file review: AGENTS.md and this record only
- unresolved review threads: none
- protected auto-merge: SQUASH enabled
- ownership release: on merge

## Context checkpoint

```yaml
last_progress: edit written
status: completed
branch: claude/agents-batch-guideline-20261013
pr: 2037
blocker: null
next_action: control plane review
```
