# OTV2-20260928-b3-inventory-destination-decision

```yaml
task_id: OTV2-20260928-b3-inventory-destination-decision
title: "B3 inventory destination, capacity and stacks decision (D80-D83)"
mode: CONTRACT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/gifted-rubin-a0axzx
issue: 162
pr: 1137
base_sha: e6141b414e0cb20fa050ba451869a8ede52a5b53
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude-code-session-01XdHJyZNPJMcmMnmSDgwQvZ (Sol Supervising Architect)
created_at: 2026-09-28
updated_at: 2026-09-28
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_B3_INVENTORY_DESTINATION_CAPACITY_AND_STACKS_DECISION_2026-09-28.md
  - docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md   # §39.3 amendment pointer only
  - docs/agents/tasks/active/OTV2-20260928-b3-inventory-destination-decision.md
  - docs/agents/tasks/active/OTV2-20260928-reference-first-player-death-decision.md   # archive move after #1132
  - docs/agents/tasks/archive/OTV2-20260928-reference-first-player-death-decision.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

This task records the durable text for the #513 gate B3: owner decisions D80-D83 (#162 comment
5874405992). It defines the first TRANSFER destinations (the container slot and direct entries of
the equipped main backpack, with placement ordinals shown newest first), entry-count capacity with
weight deferred, the stack ceiling of 100, the merge and top-up shapes, and the resource rows the
B3-1 allocation registers. It adds an amendment pointer to DUR-03 §39.3.

No registry, runtime or migration change.

## Architecture and source of truth

- `PROVEN`:
  - DUR-03 §5.2, §10, §11.5, §13, §39; GAME-ITEM-01 §4.1, §6, §7;
  - the DUR-03 resource maxima decision (D50/D51) and `RESOURCE_LIMITS_REGISTRY.json`;
  - the tibia.com manual notes (#1126) and snapshot `2026-09-28-160207Z` (#1129);
  - the backpack definition capacity (20).
- `CONFLICT`: item weights and base capacity (deferred to B3-3).
- `UNKNOWN`: Global partial pickup with a full backpack.

## High-risk authority/recovery qualification

Not applicable. The decision selects placement rules and bounds inside existing DUR-03 families and
identity rules (§13); B3-1 carries its own qualification when allocated. Current-authority fences,
idempotency and evidence obligations of DUR-03 §39 are unchanged.

## Acceptance criteria

- [ ] The decision document is on an exact frozen head with passing validators.
- [ ] Independent exact-head review.
- [ ] Protected Merge Queue integration.

## Excluded scope

- Registry rows, migrations, runtime code, weight capacity, nested bags, other equipment slots.

## Validation

- `python3 tools/agents/validate_governance.py`: PASS on the authoring tree.
- `python3 tools/repository/validate_repository_policy.py`: PASS on the authoring tree.

## Context checkpoint

```yaml
last_progress: authored; PR #1137 open
status: validating
branch: claude/gifted-rubin-a0axzx
pr: 1137
owner_action_required: null
blocker: null
next_action: exact-head review and Merge Queue integration of #1137
```
