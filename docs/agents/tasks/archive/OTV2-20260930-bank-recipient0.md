# OTV2-20260930-bank-recipient0

```yaml
task_id: OTV2-20260930-bank-recipient0
title: "BANK-0 amendment: typed transfer results and UNKNOWN_RECIPIENT"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/arch-bank-recipient-0
pr: 1423
base_sha: 17a917ed
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_frozen_at: null
owner: claude-code-session-018aTt5eRKVUGPJJYwwoMcqw (worker for the Sol Supervising Architect)
created_at: 2026-09-30
updated_at: 2026-09-30
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_BANK0_ACCOUNT_BANK_BALANCE_DECISION_2026-09-30.md
  - docs/agents/tasks/archive/OTV2-20260930-bank-recipient0.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Applies the owner's recipient-feedback decision (2026-09-30, #162) to BANK-0 bank transfers, as
MAIL-0 (#1404) applies it to mail. An in-place amendment of BANK-0 §4.3 (with pointers in the
status line and §4.1):

- **Typed results** the banker tells the sender: `OK`, `UNKNOWN_RECIPIENT`,
  `RECIPIENT_CANNOT_RECEIVE_TRANSFERS`, `SAME_ACCOUNT`, `INSUFFICIENT_BALANCE`, `JUNIOR_ACCOUNT`;
  the recipient-side `BANK0-RL-01` limit is reported as `RECIPIENT_CANNOT_RECEIVE_TRANSFERS`.
- **Anti-enumeration.** A name that never existed, a deleted character and one of another World
  all give `UNKNOWN_RECIPIENT` and "This player does not exist."
- **No value moves on a refusal.** All checks run before the first ledger or balance write; the
  recipient root is locked `FOR SHARE` and rechecked at commit (live, same World, same
  `name_key`), as MAIL-0 §6 does; the refused result is recorded for replay.
- **Reference.** Canary and Crystal Server `bank_system.lua` and `bank.cpp` (both `main`, read
  2026-09-30), whose replies the results reuse.

No code, migration or content change is made.

## Architecture and source of truth

- `PROVEN`: BANK-0; MAIL-0 (candidate, #1404) §5, §6 and §7; `0022`.
- `DERIVED`: Canary and Crystal Server `main` (`OTS_HYPOTHESIS_ONLY`).

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: docs only. BANK-1 and BANK-NPC-1 implement the results.

## Acceptance criteria

- [ ] Amendment on an exact frozen head with passing validators.
- [ ] Independent exact-head review, as the #162 control plane routes it.
- [ ] Protected Merge Queue integration.

## Excluded scope

- Code, migrations and content; deposit and withdraw replies; MAIL-0 and other decisions.

## Validation

- `python3 tools/agents/validate_governance.py`: PASS on the final authoring tree.
- `python3 tools/repository/validate_repository_policy.py`: PASS on the final authoring tree.
- `git diff --cached --check`: clean.

## Closeout

- PR: #1423. Merge commit/result: its squash merge.
- This record was archived in the PR's final authoring commit (`docs/agents/tasks/archive/README.md`).

```yaml
last_progress: final authoring commit; archived before freeze
status: completed
branch: claude/arch-bank-recipient-0
owner_action_required: null
blocker: null
next_action: "#162 validates this exact head, routes the required independent review and integrates it."
```
