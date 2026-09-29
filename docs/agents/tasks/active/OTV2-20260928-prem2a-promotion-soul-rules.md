# OTV2-20260928-prem2a-promotion-soul-rules

```yaml
task_id: OTV2-20260928-prem2a-promotion-soul-rules
title: PREM-2a - pure promotion-benefit and soul-maximum rules
mode: IMPLEMENT
status: ready
repository: Oteryn/Oteryn-Game
issue: 162
pr: 1171
allocation_comment: "#162 5878791533 (request), owner-authorized in this window (2026-09-28)"
base_branch: main
branch: claude/prem2a-promotion-soul
base_sha: df2dd464ceb97287fea0bdcc717911c4c8ef182c  # main after PR 1154 (DEATH-1a) integrated
head_sha: null
owner: "Oteryn: impl domains" (Claude Code)
created_at: 2026-09-28T21:00:00Z
updated_at: 2026-09-28T21:00:00Z
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/domain/premium.rs
  - apps/game-server/src/domain/mod.rs  # one line: pub mod premium;
  - docs/agents/tasks/active/OTV2-20260928-prem2a-promotion-soul-rules.md
  - docs/agents/tasks/{active -> archive}/OTV2-20260928-death1a-outcome-calculator.md  # closeout of PR 1154
public_contracts: []
depends_on: []
blocks: [PREM-2]
cross_repository_coordination_id: null
external_repositories: []
```

`execution_policy: continuous_progress` means productive authorized work has no wall-clock stop window.

## Outcome

`apps/game-server/src/domain/premium.rs` holds the pure rules of the Premium activation decision
§4.2-§4.3. It has no caller yet.

- `promotion_benefit_current(promoted, premium_current)`: both must hold (D76). It feeds the
  DEATH-1a promotion input and the soul rules.
- `soul_maximum`: 200 with a current promotion benefit, 100 otherwise (D72, D76).
- `apply_soul_gain`: a gain or regeneration step raises soul to at most the current maximum;
  soul above the maximum after a demotion is kept until spent and never clamped (D74).

## Excluded scope

PREM-1 (the entitlement consumer fence that supplies `premium_current`), durable promotion state,
regeneration timing, soul gain on kill, the promotion NPC, the displayed title, spell and Channel
wiring.

## Validation

- `cargo fmt --all --check`
- `cargo clippy -p oteryn-game-server --all-targets -- -D warnings`
- `cargo test -p oteryn-game-server --lib domain::premium`
- `python tools/agents/validate_governance.py`, `python tools/repository/validate_repository_policy.py`,
  `git diff --check`
