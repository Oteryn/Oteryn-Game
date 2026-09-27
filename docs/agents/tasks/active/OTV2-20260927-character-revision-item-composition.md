# OTV2-20260927-character-revision-item-composition

```yaml
task_id: OTV2-20260927-character-revision-item-composition
title: Character revision and item transaction composition decision
mode: CONTRACT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/gifted-rubin-a0axzx
issue: 162
pr: null
base_sha: bab42d5c9900b05d9a7b4ff941df1fb60d2ea760
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude-code-session-01XdHJyZNPJMcmMnmSDgwQvZ (Sol Supervising Architect)
created_at: 2026-09-27
updated_at: 2026-09-27
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/OTV2-20260927-character-revision-item-composition.md
  - docs/architecture/reviews/OTERYN_GAME_CHARACTER_REVISION_ITEM_TRANSACTION_COMPOSITION_DECISION_2026-09-27.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

This task resolves the DUR-03 §39.3 `CONFLICT` and the reward-chest decisions §5: how a non-XP
item transaction concerning a Character (pickup TRANSFER, reward MINT plus `RewardClaim`) relates
to the global `CharacterRevision` chain in migration `0009`. The decision is
`CHARACTER-REVISION-ITEM-TRANSACTION-COMPOSITION-V1`:

- such a transaction is a DUR-03 value transaction;
- it does not advance `CharacterRevision`;
- it is fenced like the XP writer;
- it serializes on the `character_root` row lock.

`0009` is unchanged.

## Architecture and source of truth

- `PROVEN`: DUR-02 owner baseline rule 2 and schema packet §4.1; DUR-03 §§5.1, 7.2, 39.3;
  GAME-ITEM-01 location ownership; `0009_character_progression.sql`;
  `durability/character_progression.rs` fence and lock order; D40-D42.
- `DERIVED`: item locations and `RewardClaim` are DUR-03 state, not Character root semantic state.
- `UNKNOWN`: inventory position, capacity and weight policy; `RewardClaim` schema; cooldown
  identity.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`. This task is a CANDIDATE architecture document only. It performs no mutation,
PREPARE/COMMIT or recovery interpretation. The implementing allocation must complete this section.

## Acceptance criteria

- [ ] The decision document is on an exact frozen head with passing governance and repository
  policy.
- [ ] Independent exact-head review, routed by #162.
- [ ] Protected Merge Queue integration by the Work coordinator. This role has no merge authority.

## Excluded scope

- Runtime code, migrations, the resource registry, protocol, client and production.
- Editing the DUR-03, DUR-02, GAME-ITEM-01 or reward-chest texts. The D40-D42 contract text is
  step 2 of the reward-chest order of work, under its own allocation.

## Validation

- `python3 tools/agents/validate_governance.py`: PASS on the authoring tree.
- `python3 tools/repository/validate_repository_policy.py`: PASS on the authoring tree.

## Independent review

- required: YES. The decision is about persistence/value and session-fence semantics.

## Context checkpoint

```yaml
last_progress: decision drafted
status: implementing
branch: claude/gifted-rubin-a0axzx
head_sha: null
pr: null
owner_action_required: null
blocker: null
next_action: "Freeze the exact head and hand it to #162 for review and integration."
```
