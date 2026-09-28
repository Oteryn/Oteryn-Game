# OTV2-20260928-spell-d7-cw1-chest-amendments

```yaml
task_id: OTV2-20260928-spell-d7-cw1-chest-amendments
title: "Record SPELL-D7 (D89), CW1 D90/D91 and reward chest D92 amendments"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/gifted-rubin-a0axzx
issue: 162
pr: 1151
base_sha: 75e502a8e90020afacbf37a8791e5eec54ea1b41
head_sha: faffe7d4bd44f1847cb67c560b0cced44499852a
final_head_sha: faffe7d4bd44f1847cb67c560b0cced44499852a
final_head_frozen_at: 2026-09-28T19:04Z
owner: claude-code-session-01XdHJyZNPJMcmMnmSDgwQvZ (Sol Supervising Architect)
created_at: 2026-09-28
updated_at: 2026-09-28
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/OTERYN_PLAYER_SPELL_CAST_WIRE_AND_VITALS_CONTRACT_CANDIDATE_V1.md
  - docs/architecture/OTERYN_INTERACTION_RELOCATION_AND_WORLD_OBJECT_OWNERS_PROPOSAL_V1.md
  - docs/architecture/OTERYN_REWARD_CHEST_PLAYABLE_SLICE_DECISIONS_V1.md
  - docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md   # §39.1/§39.3 amendment pointers
  - docs/architecture/reviews/OTERYN_GAME_CHARACTER_REVISION_ITEM_TRANSACTION_COMPOSITION_DECISION_2026-09-27.md   # §3.1 amendment
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

- [x] The amendments are on an exact frozen head with passing validators.
- [x] Independent exact-head review (one Codex review, one repair generation; owner decision to merge after green CI).
- [x] Protected Merge Queue integration (`9961e4d4`).

## Excluded scope

- Proto files, registries, migrations, runtime code.

## Finding dispositions

Codex review of `40cef29`: five P1, all ACCEPTED in repair generation 1 of 1 (#162 convergence rule
5869165340). No owner decision changed.

- 4125982620 ("at target" for position spells): `ATTACK_TARGET` resolves the attack target's
  position and applies the `POSITION` checks (spell contract §8.1).
- 4125982659 (Ground-only MINT shape): DUR-03 §39.3 supersedes the creature-death source and
  Ground-custody clauses for the bounded `CHEST-1` shape and defines its audit evidence.
- 4125982633 (re-arm after a post-revert variant): lowering synthesizes a C→B forward edge (§9 D90).
- 4125982641 (event ownership not enforceable): a typed binding origin `PLAYER_USE` / `EVENT`
  excluded from USE selection and session `apply` (§9 D91).
- 4125982667 (composition scoped to `CharacterInventory`): composition decision §3.1 extends rule
  1 to the container slot and direct backpack entries with the same fences.

## Validation

- `python3 tools/agents/validate_governance.py`: PASS on the authoring tree.
- `python3 tools/repository/validate_repository_policy.py`: PASS on the authoring tree.

## Terminal integration

- PR #1151 merged through the Merge Queue on 2026-09-28 as `9961e4d4`.
- Review: one Codex review of `40cef29` (five P1s), repaired in the single repair generation
  `faffe7d`; the owner decided to merge after green CI.
- Protected-main readback: all seven changed files on `9961e4d4` are byte-identical to the frozen
  head `faffe7d`.
- The PR body predates the repair (composition §3.1, typed transition origin, C→B edge, the
  CHEST-1 supersession); the documents are authoritative.
- Archived under `OTV2-20260928-wo0-world-object-terrain-format-decision`.

## Context checkpoint

```yaml
last_progress: protected-integrated as 9961e4d4; archived
status: completed
branch: claude/gifted-rubin-a0axzx
head_sha: faffe7d4bd44f1847cb67c560b0cced44499852a
pr: 1151
owner_action_required: null
blocker: null
next_action: null
```
