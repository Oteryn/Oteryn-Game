# G4-QUALIFY-TIMEOUT-1-20261005

```yaml
task_id: G4-QUALIFY-TIMEOUT-1-20261005
title: "G4-QUALIFY-TIMEOUT-1-20261005 raise qualify job timeout to 90 minutes"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
issue: 1622
base_branch: main
branch: claude/g4-qualify-timeout-1-20261005
pr: 1851
base_sha: fc3db9ab
head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
control_plane: session_013KJX6mv8LQveCKKXYgAX94
created_at: 2026-10-05
updated_at: 2026-10-05
owned_paths:
  - .github/workflows/g4-canonical-worldproject-package-seed.yml
  - docs/agents/tasks/archive/G4-QUALIFY-TIMEOUT-1-20261005.md
```

## Defect

The `qualify` job had `timeout-minutes: 45`, but its two materializations take
about 43 minutes on main-based PRs (#1845's run), so the job is cancelled at
45 minutes (#1807), a systemic failure.

## Fix

`timeout-minutes` of the `qualify` job raised from 45 to 90. No other line or
step changed; no step is weakened.

## Validation

- `python tools/agents/validate_governance.py`: pass.
- `python -m unittest discover -s tools/agents/tests`: pass.
- `git diff --check`: pass.
