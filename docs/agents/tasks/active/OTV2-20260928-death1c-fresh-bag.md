# OTV2-20260928-death1c-fresh-bag

```yaml
task_id: OTV2-20260928-death1c-fresh-bag
title: DEATH-1c - empty bag after a lost container
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
issue: 162
pr: null
allocation_comment: "#162 5879250283 row 2 (PROVEN), owner answer 2 and standing mode 12A in 5879169470"
base_branch: main
branch: claude/death1c-fresh-bag
base_sha: df2dd464ceb97287fea0bdcc717911c4c8ef182c
head_sha: null
owner: "Oteryn: impl domains" (Claude Code)
created_at: 2026-09-28T22:20:00Z
updated_at: 2026-09-28T22:20:00Z
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/domain/death.rs
  - docs/agents/tasks/active/OTV2-20260928-death1c-fresh-bag.md
public_contracts: []
depends_on: []
blocks: [DEATH-3]
cross_repository_coordination_id: null
external_repositories: []
```

`execution_policy: continuous_progress` means productive authorized work has no wall-clock stop window.

## Outcome

`PveDeathOutcome::grants_empty_bag` is true exactly when the item in the container slot is in the
lost-item set: "In case a character has lost its bag or backpack after death, it will receive an
empty bag in the container slot" (tibia.com manual §5.1.11, PROVEN in #162 5879250283; owner
answer 2 "as Global"). The bag itself is minted by a DUR-03 operation in DEATH-3.

## Excluded scope

The bag item definition, its mint and placement (DEATH-3, DUR-03), the lost-item moves.

## Validation

- `cargo fmt --all --check`
- `cargo clippy -p oteryn-game-server --all-targets -- -D warnings`
- `cargo test -p oteryn-game-server --lib domain::death`
- `python tools/agents/validate_governance.py`, `python tools/repository/validate_repository_policy.py`,
  `git diff --check`
