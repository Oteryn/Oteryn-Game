# OTV2-20260928-vsl-combat-resource-rows-decision

```yaml
task_id: OTV2-20260928-vsl-combat-resource-rows-decision
title: "VSL-COMBAT-01 §19 Combat resource rows (D77-D79)"
mode: CONTRACT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/gifted-rubin-a0axzx
issue: 162
pr: 1123
base_sha: 943e17b07ba42c95a91f6a431990905846a69968
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude-code-session-01XdHJyZNPJMcmMnmSDgwQvZ (Sol Supervising Architect)
created_at: 2026-09-28
updated_at: 2026-09-28
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_VSL_COMBAT_RESOURCE_ROWS_DECISION_2026-09-28.md
  - docs/agents/tasks/active/OTV2-20260928-vsl-combat-resource-rows-decision.md
  - docs/agents/tasks/active/OTV2-20260928-premium-activation-decision.md   # archive move after #1118
  - docs/agents/tasks/archive/OTV2-20260928-premium-activation-decision.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

This task records the durable text for the VSL-COMBAT-01 §19 Combat resource-row packet (owner
decisions D77-D79, #162 comment 5873104041). It answers the Combat D readiness routing item 1
(#162 comment 5873043638):

- the ceilings for §19 items 3-12;
- the D1 owner split exception;
- the flagged `MOVE-RL-11` dependency.

No registry, runtime or migration change: the Combat D allocation registers the rows.

## Architecture and source of truth

- `PROVEN`:
  - VSL-COMBAT-01 §19, §24.1, §24.3;
  - the registered `ABILITY01-*`, `INTERACTION01-*`, `DUR03-*` and `MOVE-RL-11` rows;
  - D52 (#1079); D57 (#1110).
- `DERIVED`: the loot plan bytes (16 × 1,792 B + 2,048 B = 30,720 B, decision §4.1.1).
- `UNKNOWN`: boss loot sizes, corpse decay times.

## High-risk authority/recovery qualification

Not applicable. This decision selects numeric bounds only. D1 (D79) reuses the death identity and
MINT fences qualified in #1079 and #1112, whose negative cases still bind the implementation.
Boundary tests bind the Combat D registration.

## Acceptance criteria

- [ ] The decision document is on an exact frozen head with passing validators.
- [ ] Independent exact-head review.
- [ ] Protected Merge Queue integration.

## Excluded scope

- Registry rows, runtime code, loot tables, XP values, `MOVE-RL-11`.

## Finding dispositions

- Codex P1 (PR #1123 review comment 4124139006, `COMBAT01-LOOT-PLAN-BYTES` undersized): ACCEPTED.
  The row was 12,288 B from a 704 B entry, below one maximum-width `TypedDefinitionRef` (1,152 B)
  and with the purpose key sized as technical text instead of a 512 B content key. It is now
  30,720 B, derived from the `item_mint.rs` encoding in decision §4.1.1. The owner choices D77-D79
  are unchanged; only the derived byte value changed. Repair generation 1 of 1.

## Validation

- `python3 tools/agents/validate_governance.py`: PASS on the authoring tree.
- `python3 tools/repository/validate_repository_policy.py`: PASS on the authoring tree.

## Context checkpoint

```yaml
last_progress: authored; PR #1123 open
status: validating
branch: claude/gifted-rubin-a0axzx
pr: 1123
owner_action_required: null
blocker: null
next_action: exact-head review and Merge Queue integration of #1123
```
