# OTV2-20260928-a8-donor-item-identity-epoch-decision

```yaml
task_id: OTV2-20260928-a8-donor-item-identity-epoch-decision
title: "A8 donor Item identity epoch decision (D96-D97)"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/gifted-rubin-a0axzx
issue: 162
pr: 1163
base_sha: e19b19d63d5ad45a5e97f298051fb46afcaf6236
head_sha: e4e447bf6c71b313e1e797aa75876ee7aff69bca
final_head_sha: e4e447bf6c71b313e1e797aa75876ee7aff69bca
final_head_frozen_at: 2026-09-28T20:14Z
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

- [x] The decision document is on an exact frozen head with passing validators.
- [x] Independent exact-head review.
- [x] Protected Merge Queue integration.

## Excluded scope

- Identity minting, bindings, Item records, allocator code and facts (B1b, B2, B3).

## Finding dispositions

Codex review of `c97bd25`: one P1, ACCEPTED in repair generation 1 of 1 (#162 convergence rule
5869165340). No owner decision changed.

- 4126589960 (aliases minted before resolution, against the G4 promotion discipline): B1b first
  resolves every donor id to a crosswalk state; only `NO_MATCH` ids are minted, aliases bind
  `ACCEPTED_ALIAS` to the existing key, and unresolved states mint nothing. An alias found after
  minting retires the duplicate key under the R7-P04 precedent (§2, §4.1, §4.2, §8).

## Validation

- `python3 tools/agents/validate_governance.py`: PASS on the authoring tree.
- `python3 tools/repository/validate_repository_policy.py`: PASS on the authoring tree.

## Terminal integration

- PR #1163 merged through the Merge Queue on 2026-09-28 as `9f98b067`.
- Review: one Codex review of `c97bd25` (one P1), repaired in the single repair generation
  `e4e447b`; the owner decided to merge after green CI.
- Protected-main readback: all four changed files on `9f98b067` are byte-identical to the frozen
  head `e4e447b`.
- The PR body predates the repair (the alias gate); the documents are authoritative.
- Archived under `OTV2-20260928-owner-decision-batch-d118-d128`.

## Context checkpoint

```yaml
last_progress: protected-integrated as 9f98b067; archived
status: completed
branch: claude/gifted-rubin-a0axzx
head_sha: e4e447bf6c71b313e1e797aa75876ee7aff69bca
pr: 1163
owner_action_required: null
blocker: null
next_action: null
```
