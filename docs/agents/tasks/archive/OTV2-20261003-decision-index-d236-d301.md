# OTV2-20261003-decision-index-d236-d301

```yaml
task_id: OTV2-20261003-decision-index-d236-d301
title: "Decision index refresh D236-D301 and allocation-line source"
mode: TOOLING
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/decision-index-d236-d301
pr: "1646"
base_sha: bec95a92
head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
final_head_frozen_at: null
owner: claude-code-session_016c5MQoe5CoMk9fxmuuMcFJ (Sol Supervising Architect)
created_at: 2026-10-03
updated_at: 2026-10-03
execution_policy: continuous_progress
owned_paths:
  - tools/agents/build_decision_index.py
  - tools/agents/tests/test_governance_lifecycle_decision_index.py
  - docs/agents/DECISION_INDEX.md
  - docs/agents/tasks/archive/OTV2-20261003-decision-index-d236-d301.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Scope

Control-plane allocation D288 (#1622), with its answer D302 = a): the builder keeps the merge-subject
mapping and adds a second source, the allocation line in the header of a decision document the merge
adds (`- Allocation: D286 ...`, `- control-plane allocation D300 (#1622)` or
`- Answers: control-plane allocation D292 (#1622)`). Two different
owner-sequence numbers for one document or one merge fail the build. No hand-written rows.
`--gaps` lists added decision documents with no number from either source.

## State

#1637-#1645 merged. The index was regenerated on `main` after #1638 (107 rows, D291-D300 added)
and the head frozen; the FREEZE_SHA is on #1622. The `--gaps` list (114 documents, mostly before
D236 or owner batches) stays a report, not rows.

## Validation

- `python3 tools/agents/build_decision_index.py --gaps`
- `python -m unittest discover -s tools/agents/tests -p "*governance_lifecycle*.py" -v`
- `python3 tools/agents/validate_governance.py`
- `git diff --check`
