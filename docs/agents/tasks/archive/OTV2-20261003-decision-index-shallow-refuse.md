# OTV2-20261003-decision-index-shallow-refuse

```yaml
task_id: OTV2-20261003-decision-index-shallow-refuse
title: "Decision index refuses unreadable shallow boundaries; D340 amendment; D331 draft triage"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/decision-index-shallow-refuse
pr: null
head_sha: "exact frozen head in the FREEZE_SHA entry"
final_head_sha: "exact frozen head in the FREEZE_SHA entry"
owner: claude-code-session_016c5MQoe5CoMk9fxmuuMcFJ (Sol Supervising Architect)
created_at: 2026-10-03
updated_at: 2026-10-03
execution_policy: continuous_progress
owned_paths:
  - tools/agents/build_decision_index.py
  - tools/agents/tests/test_governance_lifecycle_decision_index.py
  - docs/agents/DECISION_INDEX.md
  - docs/architecture/reviews/OTERYN_GAME_ARCH_BATCH_D327_AMENDMENT_D340_2026-10-03.md
  - docs/agents/tasks/archive/OTV2-20261003-decision-index-shallow-refuse.md
public_contracts: []
depends_on: [#1654]
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

- **#1654 P2 4173832438.** A merge at the shallow boundary is skipped only when the index already has a row for its PR. In every other case the build fails and asks for deeper history. A depth-1 run therefore no longer reports success while missing rows.
  - A test covers both cases on a real depth-1 clone.
  - The index was regenerated on full history. It adds the 6 rows that earlier shallow runs had missed, including #1651 (D309 and D324).
- **D340.** The amendment to the D327 batch moves CHAR-REV-SEQ-1 ahead of A2 and grants two test-support paths.
- **D331 (2a).** Drafts #228, #295, #571 and #574 were triaged. All four are recommended for closing, and nothing was carried over. The reasons were sent to the control plane, so this record does not repeat them.

## Validation

- `python3 -m unittest discover -s tools/agents/tests` (53 tests)
- `python3 tools/agents/validate_governance.py`
- `git diff --cached --check`
