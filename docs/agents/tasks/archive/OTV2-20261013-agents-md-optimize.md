# OTV2-20261013-agents-md-optimize

```yaml
task_id: OTV2-20261013-agents-md-optimize
title: "Trim root AGENTS.md and route operating rules"
mode: GOVERNANCE
status: completed
repository: Oteryn/Oteryn-Game
base_branch: claude/agents-batch-guideline-20261013
branch: claude/agents-md-optimize-20261013
pr: 2038
head_sha: "exact frozen head in the report to the control plane"
final_head_sha: "exact frozen head in the report to the control plane"
owner: claude-code worker for control plane session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-13
updated_at: 2026-10-13
execution_policy: continuous_progress
owned_paths:
  - AGENTS.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/tasks/archive/OTV2-20261013-agents-md-optimize.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

- Root `AGENTS.md` shrinks (9918 to about 7.8 KB): archive rule matches the archive README; one split test for batches; lifecycle paragraph is a pointer to `docs/agents/AGENTS.md` (frozen-head rule kept); Silent operation and Owner questions moved to `CONTEXT_ROUTING.md`.
- `CONTEXT_ROUTING.md` gains *Push and CI economy* (coherent pushes, one exact-head validation, path-based CI).
- Stacked on PR 2037 (unmerged at authoring time).

## Architecture and source of truth

- PROVEN: owner approval in chat ("tak"), relayed by the control plane.
- PROVEN: nested `AGENTS.md` exist, so the "nearest `AGENTS.md`" sentence is left unchanged.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: documentation only.

## Acceptance criteria

- [x] Root AGENTS.md smaller; mandatory phrases and pinned test phrases intact.
- [x] Moved text exists once, in `CONTEXT_ROUTING.md`, with root pointers.

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

- changed-file review: AGENTS.md, CONTEXT_ROUTING.md and this record only
- unresolved review threads: none
- protected auto-merge: SQUASH enabled
- ownership release: on merge

## Context checkpoint

```yaml
last_progress: PR opened, record archived
status: completed
branch: claude/agents-md-optimize-20261013
pr: 2038
blocker: null
next_action: control plane review
```
