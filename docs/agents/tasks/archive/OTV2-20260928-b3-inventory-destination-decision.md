# OTV2-20260928-b3-inventory-destination-decision

```yaml
task_id: OTV2-20260928-b3-inventory-destination-decision
title: "B3 inventory destination, capacity and stacks decision (D80-D83)"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/gifted-rubin-a0axzx
issue: 162
pr: 1137
base_sha: e6141b414e0cb20fa050ba451869a8ede52a5b53
head_sha: dea076fbee590789062a46b53e8d414dcf0b3da5
final_head_sha: dea076fbee590789062a46b53e8d414dcf0b3da5
final_head_frozen_at: 2026-09-28T16:57Z
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

- [x] The decision document is on an exact frozen head with passing validators.
- [x] Independent exact-head review (one Codex review, one repair generation; owner decision to merge after green CI).
- [x] Protected Merge Queue integration (`e5cbcfa4`).

## Excluded scope

- Registry rows, migrations, runtime code, weight capacity, nested bags, other equipment slots.

## Finding dispositions

Codex review of `ed4f32a`: five P1 and one P2, all ACCEPTED in repair generation 1 of 1 (#162
convergence rule 5869165340). No owner decision changed.

- 4124831687 (P1, §39.1 exclusions not superseded): decision §4.6 now lists the superseded
  statements; pointer notes in DUR-03 §39.1 and §39.3.
- 4124831695 (P1, quantity in stack compatibility): compatibility compares all non-quantity state.
- 4124831720 (P2, capacity off-by-one): `current_entry_count < definition_capacity` before insert.
- 4124831702 (P1, stack quantities as RL-03 value lines): RL-03 stays 0; quantities are item state.
- 4124831716 (P1, container slot without equip pattern): the definition must declare a
  `container`-slot equip pattern (GAME-ITEM-01 §6.2).
- 4124831709 (P1, RL-06 participants): participants 2 and work units 6 for the merge shapes,
  derived as participant plus effects.

## Validation

- `python3 tools/agents/validate_governance.py`: PASS on the authoring tree.
- `python3 tools/repository/validate_repository_policy.py`: PASS on the authoring tree.

## Terminal integration

- PR #1137 merged through the Merge Queue on 2026-09-28 as `e5cbcfa4`.
- Review: one Codex review of `ed4f32a` (five P1, one P2), repaired in the single repair
  generation `dea076f`; the owner decided to merge after green CI (#162 comment 5875083548).
- Protected-main readback: the decision, the DUR-03 pointers, this record and the archived #1132
  record on `e5cbcfa4` are byte-identical to the frozen head `dea076f`.
- The PR body predates the repair; the decision document §4 is authoritative.
- Next allocations: B3-1 (with a content definition proving a `container`-slot equip pattern).
- Archived under `OTV2-20260928-move-rl11-visibility-decision`.

## Context checkpoint

```yaml
last_progress: protected-integrated as e5cbcfa4; archived
status: completed
branch: claude/gifted-rubin-a0axzx
head_sha: dea076fbee590789062a46b53e8d414dcf0b3da5
pr: 1137
owner_action_required: null
blocker: null
next_action: null
```
