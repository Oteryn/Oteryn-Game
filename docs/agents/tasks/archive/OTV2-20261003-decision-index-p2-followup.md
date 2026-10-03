# OTV2-20261003-decision-index-p2-followup

```yaml
task_id: OTV2-20261003-decision-index-p2-followup
title: "Decision index: deferred #1646 P2s (shallow boundary, lane-local conflict, archived status)"
mode: TOOLING
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/decision-index-p2-followup
pr: "1654"
base_sha: 10373b64
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
  - docs/agents/tasks/archive/OTV2-20261003-decision-index-p2-followup.md
public_contracts: []
depends_on:
  - "#1646"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

This PR fixes the three P2s that #1646 deferred:
- **4173590743:** shallow-boundary commits are skipped. A depth-1 clone no longer reads the whole
  tree as added documents.
- **4173590749:** two different header allocations now fail even when the subject carries only a
  lane-local id.
- **4173590750:** the #1646 archived record is set to `completed`.

The index is regenerated on `10373b64` (109 rows; it adds #1646 D288 and #1650 D298).

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: agent tooling and docs only.

## Validation

- `python -m unittest discover -s tools/agents/tests` passes (52 tests), including two new tests:
  the lane-local conflict and a real depth-1 clone boundary.
- `python3 tools/agents/validate_governance.py` passes, and `git diff --cached --check` is clean.
- The build ran on this shallow checkout and only added rows.

## Independent review

- required: per the bound review policy for agent tooling; the control plane decides.

## PR and closeout

- PR #1654. This record is archived in its final authoring commit.
