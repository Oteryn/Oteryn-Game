# OTV2-20260928-a8-donor-item-identity-epoch-decision

```yaml
task_id: OTV2-20260928-a8-donor-item-identity-epoch-decision
title: "A8 donor Item identity epoch decision (D96-D97)"
mode: CONTRACT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/gifted-rubin-a0axzx
issue: 162
pr: 1163
base_sha: e19b19d63d5ad45a5e97f298051fb46afcaf6236
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude-code-session-01XdHJyZNPJMcmMnmSDgwQvZ (Sol Supervising Architect)
created_at: 2026-09-28
updated_at: 2026-09-28
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_A8_DONOR_ITEM_IDENTITY_EPOCH_DECISION_2026-09-28.md
  - docs/architecture/OTERYN_G4_MULTI_SOURCE_IDENTITY_BINDING_DECISION.md   # pointer only
  - docs/agents/tasks/active/OTV2-20260928-a8-donor-item-identity-epoch-decision.md
  - docs/agents/tasks/active/OTV2-20260928-wo0-world-object-terrain-format-decision.md   # archive move after #1157
  - docs/agents/tasks/archive/OTV2-20260928-wo0-world-object-terrain-format-decision.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

This task records the durable text for architecture package item A8 (#162 comment 5877360232)
with owner decisions D96-D97 (#162 comment 5877519880). The 412 donor-only ids get an additive
epoch-2 opaque Item sequence after the highest allocated number (38,094 to 38,505), and the frozen
CW2-B1 import stays byte-identical. All 412 get an Item key, including the 138 routed to
WorldObject or Terrain, which follow the WO-0 D93 rule. A pointer is added to the G4 multi-source
identity decision.

No identity is minted and no code, binding or registry change is made; B1b follows.

## Architecture and source of truth

- `PROVEN`: `cw2_b1_import.rs` (positional closed-corpus allocator, 38,093 opaque keys,
  `R7_P04_GOLD_COIN_OLD_KEY`); `imports/crystalserver/bindings/items.json`; the donor census
  sample; the G4 identity decision; WO-0 D93.
- `UNKNOWN`: which donor ids later prove to be aliases of existing items.

## High-risk authority/recovery qualification

Not applicable. The decision fixes an allocation rule but mints nothing; B1b, which mints
identity, requires independent identity review when allocated.

## Acceptance criteria

- [ ] The decision document is on an exact frozen head with passing validators.
- [ ] Independent exact-head review.
- [ ] Protected Merge Queue integration.

## Excluded scope

- Identity minting, bindings, Item records, allocator code and facts (B1b, B2, B3).

## Validation

- `python3 tools/agents/validate_governance.py`: PASS on the authoring tree.
- `python3 tools/repository/validate_repository_policy.py`: PASS on the authoring tree.

## Context checkpoint

```yaml
last_progress: authored; PR #1163 open
status: validating
branch: claude/gifted-rubin-a0axzx
pr: 1163
owner_action_required: null
blocker: null
next_action: exact-head review and Merge Queue integration of #1163
```
