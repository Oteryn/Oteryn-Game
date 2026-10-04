# OTV2-20261004-accept-itemuse-bank-0

```yaml
task_id: OTV2-20261004-accept-itemuse-bank-0
title: "ACCEPT-ITEMUSE-BANK-0: accept ITEM-USE-0 and BANK-0"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/accept-itemuse-bank-0-20261004
issue: 162
pr: 1750
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
owner: claude-code-session_01YL1cQaLL3BquJajKivZVhw (Sol Supervising Architect)
created_at: 2026-10-04
updated_at: 2026-10-04
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_ACCEPT_ITEMUSE_BANK0_ACCEPTANCE_DECISION_2026-10-04.md
  - docs/architecture/reviews/OTERYN_GAME_ITEM_USE0_USING_ITEMS_DECISION_2026-09-30.md
  - docs/architecture/reviews/OTERYN_GAME_BANK0_ACCOUNT_BANK_BALANCE_DECISION_2026-09-30.md
  - docs/architecture/reviews/OTERYN_GAME_ARCH_BATCH_ROOT_PACKETS_2026-10-04.md
  - docs/agents/tasks/archive/OTV2-20261004-accept-itemuse-bank-0.md
public_contracts: []
depends_on: []
blocks: [BANK-RET-0, BANK-1, ITEM-USE-WIRE-1, ITEM-USE-1]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

- Control plane D498 (10a): ITEM-USE-0 and BANK-0 move from CANDIDATE to ACCEPTED when this
  decision merges, with the self-pending amendments listed in its §1.
- Independent review by separate read-only agents. ITEM-USE-0: P1 capability number pinned to the
  leased 15; P2 RUNE-USE-0 self-reference fixed. BANK-0: no P1 (§2).
- #1750 P1 4177200352: ITEM-USE-0 §3 pins `use_with`'s `creature` to the existing `EntityRefV1`
  (16 identity bytes, nonzero generation, else `REJECTED`; stale or unseen is `NO_TARGET`). The
  ITEM-USE-WIRE-1 packet in ARCH-BATCH-ROOT-PACKETS-V1 §2.2 imports it and tests both constraints.
- No code, contract or wire change.

## Validation

- `python tools/agents/validate_governance.py`: pass
- `python tools/repository/validate_repository_policy.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass
