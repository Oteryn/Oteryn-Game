# OTV2-20261003-charm-desc-fence1

```yaml
task_id: OTV2-20261003-charm-desc-fence1
title: "CHARM-DESC-FENCE-1: D295 hard wiring gate test"
mode: WORK
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/charm-desc-fence1-20261003
pr: null
base_sha: ac6fdca8
head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
final_head_frozen_at: null
owner: implementation worker under control plane session_013KJX6mv8LQveCKKXYgAX94 (#1622)
created_at: 2026-10-03
updated_at: 2026-10-03
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/ability/charm_desc_fence_gate_tests.rs
  - apps/game-server/src/ability/mod.rs
  - docs/agents/tasks/archive/OTV2-20261003-charm-desc-fence1.md
public_contracts: []
depends_on:
  - "PR #1638 (D295), merged"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Implements D295 §3 item 3 (`docs/architecture/reviews/OTERYN_GAME_CHARM_DESC_FENCE_DECISION_2026-10-03.md`):
a `game-server` test scans `src/` and fails on any non-test production reference to
`commit_exact_owner_damage`, `commit_exact_owner_primary_damage` or
`commit_exact_owner_charm_damage`. Exempt: the three bridges' own definitions and bodies, comments,
string and char literals, `#[cfg(test)]` items, files with an inner `#![cfg(test)]`, and
`*_tests.rs`/`tests.rs` modules. `#[cfg(not(test))]` and `#[allow(dead_code)]` are not exemptions.
The failure message names D295 and A2. Only the A2 PR, with the fence, may relax it.

No production change: one new test module and its `#[cfg(test)]` mod line.

## Acceptance criteria

- [x] Gate test passes on `main` (no production reference exists).
- [x] Negative self-checks on synthetic sources: production references flagged; comment, string,
  test, and bridge-body references ignored.
- [x] Manual mutation: a probe reference appended to `src/world_runtime.rs` fails the gate with the
  D295/A2 message; reverted.
- [ ] Independent exact-head review (combat).
- [ ] Protected Merge Queue integration.

## Validation

- `cargo test -p oteryn-game-server`: all suites pass, 0 failures.
- `cargo fmt --all --check`: clean.
- `cargo clippy -p oteryn-game-server --all-targets -- -D warnings`: clean.

## Self-review

- Whole-diff reread. The scanner is lexical, not a parser: it fails closed (a reference it cannot
  classify is reported), and the exemptions are exactly D295's list.

## PR and closeout

- Record archived in the final authoring commit; it reaches `main` only if the PR merges.
