# OTV2-20261014-ranged-0-amend

```yaml
task_id: OTV2-20261014-ranged-0-amend
title: "RANGED-0: amend the decision after the four-lane review"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/ranged-0-amend-20261014
pr: 2111
head_sha: "exact frozen head in the report to the control plane"
final_head_sha: "exact frozen head in the report to the control plane"
owner: claude-code worker for control plane session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-14
updated_at: 2026-10-14
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_RANGED0_DISTANCE_WEAPONS_AMMUNITION_WANDS_AND_CHASE_DECISION_2026-10-01.md
  - docs/agents/tasks/archive/OTV2-20261014-ranged-0-amend.md
public_contracts:
  - "Decision §6.4 WeaponUseAggregateV1 (DUR-03 §15, §39.1, §39.3 amendment text, active on acceptance only): persistence review"
  - "Decision §6.2 allocated UUIDv7 TransactionId, server-originated fence, composition rule 1 scope: persistence review"
  - "Decision §6.1.2 Ground order is the existing stack_ordinal (no migration): persistence review"
  - "Decision §5 and §4 hit_chance semantics, cap rows, AutoAttack routing, SwingRngPurpose: combat review"
  - "Decision §8 chase stays client-driven, CHASE-1 gated: movement review"
  - "Decision §7 missile stream waits for SPELL-PRESENT-0 registration: protocol review"
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

- The RANGED-0 decision is amended to close every blocker of the 2026-10-14 four-lane review (table in decision §14). Status remains CANDIDATE; the four lanes re-run on the amended head. Text only: no code, contract file, migration or protocol change.

## Architecture and source of truth

- PROVEN: blockers cited from code (`target.rs`, migration 0034, `spell_item_transaction.rs`, DUR-03 §20) were checked against the repository before the text changed.

## High-risk authority/recovery qualification

`APPLICABLE` to the contract effects (persistence, combat, movement, protocol), not to this diff. Acceptance still needs all four independent lanes.

## Acceptance criteria

- [x] Each review blocker maps to a section (decision §14).

## Excluded scope

- No code, content, migration or protocol change; no acceptance.

## Validation

### Focused

- `python3 tools/agents/validate_governance.py`: see PR checks
- `python3 -m unittest discover -s tools/agents/tests`: see PR checks
- `git diff --check`: pass

### Component/integration

- `NOT_APPLICABLE`: documentation only.

### E2E

- `NOT_APPLICABLE`: documentation only.

### Exact-head CI

- final head: the PR head
- result: the PR checks on that head

## Self-review

- exact head: the frozen head
- method/reviewer: task writer.
- material findings: none open
- verdict: ready for the four-lane re-review

## Independent review

- required: the control plane re-runs combat, persistence, movement and protocol lanes.
- verdict: pending.

## PR and closeout

- changed-file review: documentation paths only
- unresolved review threads: none
- protected auto-merge: control plane
- ownership release: on merge

## Context checkpoint

```yaml
last_progress: amendment pushed
status: completed
branch: claude/ranged-0-amend-20261014
pr: 2111
blocker: null
next_action: control plane re-runs the four lanes
```
