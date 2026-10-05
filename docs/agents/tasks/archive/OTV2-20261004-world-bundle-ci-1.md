# OTV2-20261004-world-bundle-ci-1

```yaml
task_id: OTV2-20261004-world-bundle-ci-1
title: "WORLD-BUNDLE-CI-1: world_bundle job in game-gate, World pin and derived identity"
mode: IMPLEMENT
status: done
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/world-bundle-ci-1-20261005
issue: 1622
pr: 1805
owner: claude-code-session_016fm93wMk1YFrXyG9HP9Fzj
created_at: 2026-10-05
updated_at: 2026-10-05
execution_policy: continuous_progress
owned_paths:
  - .github/workflows/merge-gate.yml
  - .github/workflows/merge-group-gate.yml
  - tools/repository/classify_pr_test_lanes.py
  - tools/repository/validate_repository_policy_core.py
  - tools/repository/test_classify_pr_test_lanes.py
  - tools/repository/test_main_job_applicability.py
  - tools/repository/test_validate_pr_gate_pg_sim.py
  - tools/repository/test_validate_merge_group_pg_sim.py
  - content/world/pins/**
  - tools/world-bundle-compiler/src/main.rs
  - tools/world-bundle-compiler/tests/**
  - crates/world-bundle/src/bundle.rs
  - docs/agents/tasks/active/OTV2-20261004-world-bundle-ci-1.md
  - docs/agents/tasks/archive/OTV2-20261004-world-bundle-ci-1.md
public_contracts: []
depends_on: [ARCH-WORLD-CONTENT-SERVE-1]
blocks: [WORLD-CONTENT-SERVE-1]
cross_repository_coordination_id: null
external_repositories: []
```

## Scope

Packet §2.2 of `docs/architecture/reviews/OTERYN_GAME_ARCH_WORLD_CONTENT_SERVE_PACKETS_2026-10-04.md`.
A `world_bundle` job inside `game-gate` (PR and merge group) builds the World bundle twice,
checks it against the reviewed pin `content/world/pins/<world slug>.json` and the derived
identity, and uploads it as a digest-named artifact. `game-gate` stays the only required status.
Out of scope: a production pin, artifact storage or node fetching.

## Validation

- `cargo fmt --all -- --check`: pass
- `cargo clippy --locked -p oteryn-world-bundle-compiler --all-targets -- -D warnings`: pass
- `cargo test --locked -p oteryn-world-bundle-compiler`: pass (the full `pin-check` test runs with `--release`)
- `cargo clippy --locked -p oteryn-world-bundle --all-targets -- -D warnings`: pass
- `cargo test --locked -p oteryn-world-bundle`: pass
- `python3 tools/repository/validate_repository_policy.py`: pass
- `python3 -m unittest discover -s tools/repository -p 'test_*.py'`: pass
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass
