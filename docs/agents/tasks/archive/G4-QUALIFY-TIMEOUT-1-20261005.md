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

## Added scope: spawn donor

`qualify` also fails in "Validate the spawn family" (runs 37370258298 and, on #1843,
37362304607; evidence in #1622 comment 6001186523): the workflow fetched
`opentibiabr/canary@47dfd51f.../otservbr-monster.xml`, but `convert_spawns.py` pins
`zimbadev/crystalserver@00ce02a57ca5a12e48f32a3476e37471167e4c3f`
`data-global/world/world-monster.xml` (sha256 `a3188bc1...`). The step now fetches the
pinned CrystalServer file into `world-monster.xml` and its comment is corrected.
`SOURCE` and the hash check in `convert_spawns.py` are unchanged.

## Validation

- Downloaded the CrystalServer file: sha256 `a3188bc1275fbf5bac1ff5c06cc26b1d2999e51c088a7ffa464c40a1aff81570` matches; `python3 tools/world-bundle-compiler/convert_spawns.py --xml <file> --check`: pass.
- `python tools/agents/validate_governance.py`: pass.
- `python -m unittest discover -s tools/agents/tests`: pass.
- `python tools/repository/validate_repository_policy.py`: pass (23 files, 62 workflows).
- `python tools/repository/test_<name>.py` for 8 of the 10 `tools/repository/test_*.py` files: pass.
- `test_validate_merge_group_pg_sim.py` and `test_validate_pr_gate_pg_sim.py`: not run locally; both require `pwsh`, which this environment lacks (they fail with `pwsh is required for WP1 native failure-propagation qualification`). Left to CI.
- `git diff --check`: pass.
