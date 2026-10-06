# OTV2-20261006-ci-win-longpaths-2

```yaml
task_id: OTV2-20261006-ci-win-longpaths-2
title: "CI-WIN-LONGPATHS-2 core.longpaths in the Windows gate jobs with gate pin rotation"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
issue: 1622
lane_id: ci
base_branch: main
branch: claude/ci-win-longpaths-2-20261006
base_sha: e953f1ef
owner: claude-code-session-013HLWAoXJKbdvP1te3rPfVs
control_plane: claude-code-session-013KJX6mv8LQveCKKXYgAX94
created_at: 2026-10-06
updated_at: 2026-10-06
authority: "owner approval of pin rotation + core.longpaths in the gate (D811, D813), confirmed by the owner in the writer session"
head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
owned_paths:
  - .github/workflows/merge-gate.yml
  - .github/workflows/merge-group-gate.yml
  - tools/repository/validate_repository_policy_core.py
  - tools/repository/validate_pr_gate_pg_sim.py
  - tools/repository/test_validate_merge_group_pg_sim.py
  - docs/agents/tasks/archive/OTV2-20261006-ci-win-longpaths-2.md
```

## Scope

Optional hardening after CI-WIN-LONGPATHS-1 (#1854). The `rust_windows` job in `merge-gate.yml` and
`merge-group-gate.yml` runs `git config --system core.longpaths true` before checkout. Nothing else in
those jobs changes. The fail-closed path-length check from #1854 stays.

Pins rotated for exactly these edits:

- `EXPECTED_MERGE_GROUP_GATE_BLOB` (core validator) and `APPROVED` (merge-group test):
  `e25c0743…` → `ed36f5b9dfc3c2126e7bbef450efeffec7336157`
- `EXPECTED_EVIDENCE_JOB_SHA256["rust_windows"]` (`validate_pr_gate_pg_sim.py`):
  `b1480385…` → `f23adb6918f87366b6b73d76408ae9800b893e609ed0a99ea4abc11a887fb0e0`

The protected-base audit pins in `merge-authority-audit.yml` (merge-gate and merge-group-gate blobs) are
rotated in a separate preapproval PR that merges first (the #1814/#1695 precedent).

## Validation

- `python tools/repository/validate_repository_policy.py`: pass
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass (54 tests)
- `tools/repository/test_*.py`: pass, except `test_validate_merge_group_pg_sim.py` and
  `test_validate_pr_gate_pg_sim.py`, which need `pwsh` (absent locally; they fail identically on base)
- `git diff --check`: pass
