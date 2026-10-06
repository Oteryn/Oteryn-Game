# OTV2-20261005-err-registry-0

```yaml
task_id: OTV2-20261005-err-registry-0
title: "ERR-REGISTRY-0: Game error code registry, validator and explain tool"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
issue: 1622
base_branch: main
branch: claude/err-registry-0-20261005
pr: 1841
base_sha: origin/main at branch creation
head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
final_head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
owner: claude-code-session-014fnNUJUAqPisoEVNP7vPGF
control_plane: claude-code-session-013KJX6mv8LQveCKKXYgAX94
created_at: 2026-10-05
updated_at: 2026-10-05
packet: "docs/architecture/reviews/OTERYN_GAME_ARCH_ERROR_CODES_2026-10-05.md §2.1 with §1.1-§1.3 and §1.8"
leases: none
owned_paths:
  - docs/contracts/OTERYN_GAME_ERROR_CODE_REGISTRY.json
  - tools/errors/**
  - tools/repository/validate_repository_policy.py
  - docs/agents/tasks/archive/OTV2-20261005-err-registry-0.md
depends_on:
  - "ARCH-ERROR-CODES-0 merged (#1840)"
public_contracts: []
external_repositories: []
```

## Outcome

- `OTERYN_GAME_ERROR_CODE_REGISTRY.json`: `blocks` (§1.2) and 35 seed codes: `BootError` 2001-2013, SQLSTATEs `OTN01`-`OTN03`, `OTI01`-`OTI05`, `OTC01` as 3001-3009, `NotDelivered` 5001-5007, ops failures 6001-6006 (exit statuses 2-7 unchanged).
- `tools/errors/registry.py`: schema, block, uniqueness (both registries), category, disposition-table checks, and append-only plus no-semantic-edit comparison against `origin/main` for both registries. Wired into `validate_repository_policy.py`.
- `tools/errors/explain.py`: lookup by `E3004`, `3004` or name, and `--scan FILE`; protocol entries without `progression` show the progression `derived` from the §1.3 table.
- Tests in `tools/errors/tests/test_registry.py`.

## Validation

- `python3 -m unittest discover -s tools/errors/tests`: pass
- `python3 tools/repository/validate_repository_policy.py`: pass
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass

## Closeout

Review state and merge are tracked on PR #1841 by the control plane.
