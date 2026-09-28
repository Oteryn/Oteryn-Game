# OTV2-20260928-death0-character-death-receipt-decision

```yaml
task_id: OTV2-20260928-death0-character-death-receipt-decision
title: "DEATH-0 Character death receipt decision"
mode: CONTRACT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/gifted-rubin-a0axzx
issue: 162
pr: null
base_sha: 7d1134f090ac249f964fede017efabba91e22b90
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude-code-session-01XdHJyZNPJMcmMnmSDgwQvZ (Sol Supervising Architect)
created_at: 2026-09-28
updated_at: 2026-09-28
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_DEATH0_CHARACTER_DEATH_RECEIPT_DECISION_2026-09-28.md
  - docs/agents/tasks/active/OTV2-20260928-death0-character-death-receipt-decision.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

This task records the architecture decision for DEATH-0, the storage prerequisite of the Reference
first player death decision (#1132, §4.3). It adds a second receipt kind (death) to the
`0009` Character revision chain, defines the guard changes, a blessing state table, the respawn
position record and the `commit_character_death` writer contract for DEATH-1.

No migration, runtime or registry change; the migration belongs to the DEATH-0 allocation.

## Architecture and source of truth

- `PROVEN`: migration `0009_character_progression.sql`; `durability/character_progression.rs`;
  the Character/item composition decision §3.6; the first player death decision §4.3-§4.6.
- `UNKNOWN`: Character position persistence; blessing purchase receipts (DEATH-4).

## High-risk authority/recovery qualification

The decision changes the Character revision chain's admissible successors, a persistence
invariant. It keeps every existing guard property for XP receipts, adds a death kind with its own
monotonic constraint (experience never increases), requires exactly one receipt of either kind per
revision and continuity across kinds, and reuses the XP writer's fences and occurrence
idempotency. DEATH-0 and DEATH-1 carry the negative cases in the handback when allocated.

## Acceptance criteria

- [ ] The decision document is on an exact frozen head with passing validators.
- [ ] Independent exact-head review.
- [ ] Protected Merge Queue integration.

## Excluded scope

- The migration, the writer code, Character position persistence, blessing purchases, item effects.

## Validation

- `python3 tools/agents/validate_governance.py`: PASS on the authoring tree.
- `python3 tools/repository/validate_repository_policy.py`: PASS on the authoring tree.

## Context checkpoint

```yaml
last_progress: authored
status: implementing
branch: claude/gifted-rubin-a0axzx
pr: null
owner_action_required: null
blocker: null
next_action: open the PR, bind this record to it, freeze and route one external review
```
