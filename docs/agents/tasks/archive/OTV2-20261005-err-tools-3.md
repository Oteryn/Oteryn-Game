# OTV2-20261005-err-tools-3

```yaml
task_id: OTV2-20261005-err-tools-3
title: "ERR-TOOLS-3: coded validator output"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
issue: 1622
base_branch: main
branch: claude/err-tools-3-20261005
pr: 1855
control_plane: claude-code-session-013KJX6mv8LQveCKKXYgAX94
created_at: 2026-10-05
updated_at: 2026-10-06
packet: "docs/architecture/reviews/OTERYN_GAME_ARCH_ERROR_CODES_2026-10-05.md §2.4, scope §1.7"
owned_paths:
  - tools/agents/validate_governance.py
  - tools/repository/validate_repository_policy.py
  - tools/repository/test_validate_coded_output.py
  - docs/contracts/OTERYN_GAME_ERROR_CODE_REGISTRY.json
  - docs/agents/tasks/archive/OTV2-20261005-err-tools-3.md
depends_on:
  - "ERR-REGISTRY-0 merged (#1841)"
public_contracts: []
external_repositories: []
```

## Outcome

`validate_governance.py` (E8001 `GOVERNANCE_CHECK_FAILED`) and `validate_repository_policy.py`
(E8002 `REPOSITORY_POLICY_CHECK_FAILED`) print each failure as `E8xxx NAME: message`, and under
`GITHUB_ACTIONS=true` also an `::error title=…::` annotation with `%`, CR and LF escaped. Pass/fail
logic and exit codes are unchanged. Two 8xxx registry rows added. The pinned
`validate_repository_policy_core.py` and `.github/workflows` are untouched; the core's own failure
lines stay uncoded. Annotations carry no `file=`/`line=` because the validators' errors have no
single source location.

## Validation

- `python tools/agents/validate_governance.py`: pass.
- `python tools/repository/validate_repository_policy.py`: pass.
- `python -m unittest discover -s tools/agents/tests`: pass (54 tests).
- `python tools/repository/test_validate_coded_output.py`: pass (2 tests, codes match registry).
- `python tools/errors/tests/test_registry.py`: 14 of 15 pass; `test_every_protocol_code_resolves_with_derived_progression` (20 != 37) fails identically on unmodified `main` in this checkout.
- `tools/repository/test_validate_merge_group_pg_sim.py` and `test_validate_pr_gate_pg_sim.py`: fail identically on unmodified `main` in this checkout; unrelated files.
- `git diff --check`: pass.

## Review

Control plane decides review on the frozen head.
