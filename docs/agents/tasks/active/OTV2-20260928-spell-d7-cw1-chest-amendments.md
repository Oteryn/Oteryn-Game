# OTV2-20260928-spell-d7-cw1-chest-amendments

```yaml
task_id: OTV2-20260928-spell-d7-cw1-chest-amendments
title: "Record SPELL-D7 (D89), CW1 D90/D91 and reward chest D92 amendments"
mode: CONTRACT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/gifted-rubin-a0axzx
issue: 162
pr: 1151
base_sha: 75e502a8e90020afacbf37a8791e5eec54ea1b41
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude-code-session-01XdHJyZNPJMcmMnmSDgwQvZ (Sol Supervising Architect)
created_at: 2026-09-28
updated_at: 2026-09-28
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/OTERYN_PLAYER_SPELL_CAST_WIRE_AND_VITALS_CONTRACT_CANDIDATE_V1.md
  - docs/architecture/OTERYN_INTERACTION_RELOCATION_AND_WORLD_OBJECT_OWNERS_PROPOSAL_V1.md
  - docs/architecture/OTERYN_REWARD_CHEST_PLAYABLE_SLICE_DECISIONS_V1.md
  - docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md   # §39.3 pointer only
  - docs/agents/tasks/active/OTV2-20260928-spell-d7-cw1-chest-amendments.md
  - docs/agents/tasks/active/OTV2-20260928-death0-character-death-receipt-decision.md   # archive move after #1148
  - docs/agents/tasks/archive/OTV2-20260928-death0-character-death-receipt-decision.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

This task records the durable text of owner decisions taken on #162 on 2026-09-28:

- D89 SPELL-D7 (comment 5875958040): the position target intent and the stateless per-cast
  `aim_at_target` flag, as §8.1 and wire changes in the P3b spell cast contract;
- D90 and D91 (same comment): the timed teleporter re-arms on each owning event, and USE cannot
  reach event-owned transitions, in the owners proposal §9;
- D92 and the architect answers on the reward chest (comment 5876398790): slots-only first slice,
  `CHEST-1` MINT into a backpack entry, no mint into a stack, non-container rewards first, the
  `RewardClaim` schema, once-only first and the cooldown cycle ordinal, as §5.1 of the reward chest
  decisions, with a DUR-03 §39.3 pointer.

No registry, proto, migration or runtime change.

## Architecture and source of truth

- `PROVEN`: the owner answers on #162; the P3b contract §3 and §8; the owners proposal §7 and §9;
  the reward chest decisions §4-§7; DUR-03 §39; the B3 decision.

## High-risk authority/recovery qualification

Not applicable. The amendments record accepted decisions in the owning documents; each delivery
child (P3b step 1, #1144, `CHEST-1`) carries its own qualification.

## Acceptance criteria

- [ ] The amendments are on an exact frozen head with passing validators.
- [ ] Independent exact-head review.
- [ ] Protected Merge Queue integration.

## Excluded scope

- Proto files, registries, migrations, runtime code.

## Validation

- `python3 tools/agents/validate_governance.py`: PASS on the authoring tree.
- `python3 tools/repository/validate_repository_policy.py`: PASS on the authoring tree.

## Context checkpoint

```yaml
last_progress: authored; PR #1151 open
status: validating
branch: claude/gifted-rubin-a0axzx
pr: 1151
owner_action_required: null
blocker: null
next_action: exact-head review and Merge Queue integration of #1151
```
