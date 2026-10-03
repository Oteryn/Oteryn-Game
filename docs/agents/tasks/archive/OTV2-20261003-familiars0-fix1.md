# OTV2-20261003-familiars0-fix1

```yaml
task_id: OTV2-20261003-familiars0-fix1
title: "FAMILIARS-0-FIX-1: session end fences return admission; one invariant per negative case"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: arch/familiars0-fix1-20261003
issue: 162
pr: 1674
head_sha: "exact frozen head in the control plane FREEZE_SHA entry"
owner: Sol Supervising Architect
created_at: 2026-10-03
updated_at: 2026-10-03
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_FAMILIARS0_FAMILIARS_DECISION_2026-10-03.md
  - docs/agents/tasks/archive/OTV2-20261003-familiars0-fix1.md
public_contracts: []
external_repositories: []
```

## Outcome

This task carries the two round-3 P1s that #1669 deferred (D317).

- **4174359097.** Logout and channel transfer now close return admission and drain an in-flight
  return before the clean-end write, and they hold the close until authority release (FAMILIARS-0
  §5). The new F-I11 states it, and a §5.2 negative case covers the cross-boundary race.
- **4174359104.** The compound §5.2 cases are split so each names one invariant:
  - reconciliation failure becomes three cases: F-I2, F-I3 and F-I10;
  - the delayed-return race becomes F-I4 (removal) and F-I7 (cast);
  - sibling creation becomes F-I6 and F-I7.

## High-risk authority/recovery qualification

```yaml
applicable: true
authority_invariants_added: [F-I11 clean-end write is the session's last row write until authority release]
one_invariant_per_negative_case: true   # FAMILIARS-0 §5.2 after the split
```

## Validation

- `python3 tools/agents/validate_governance.py`
- `python3 tools/repository/validate_repository_policy.py`
- `python -m unittest discover -s tools/agents/tests` (54 tests, OK)
- `git diff --cached --check`
