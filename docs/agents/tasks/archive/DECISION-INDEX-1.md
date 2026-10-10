# DECISION-INDEX-1

```yaml
task_id: DECISION-INDEX-1
title: "DECISION-INDEX-1: refresh docs/agents/DECISION_INDEX.md (D721 and missed main rows)"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/decision-index-1-20261008
pr: "opened for this branch"
base_sha: 9a1661a5
head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
final_head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
merge_commit: "squash merge of this PR"
owner: claude-code-session-01VvZNecK5U1yJKBKFwWx9FN
control_plane: claude-code-session-01CwP6d84eCPvpgoEuyci8Tx
created_at: 2026-10-08
updated_at: 2026-10-08
owned_paths:
  - docs/agents/DECISION_INDEX.md
  - docs/agents/tasks/archive/DECISION-INDEX-1.md
public_contracts: []
external_repositories: []
```

## Outcome

Ran `python tools/agents/build_decision_index.py --ref origin/main` on unshallowed history. It added 16 rows
from merged subjects and decision documents, including D721 (#1815, native login main track).
The index is derived only from merges on `main`, so the STATE decisions D951-D966 (issue comment, no merge
subject) are not tracked and were not added by hand.
