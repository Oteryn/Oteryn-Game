# OTV2-20260928-owner-decision-batch-d118-d128

```yaml
task_id: OTV2-20260928-owner-decision-batch-d118-d128
title: "Owner decision batch D118-D128"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/gifted-rubin-a0axzx
issue: 162
pr: 1180
base_sha: 3dcf3c82abc5d424542388a9f65ca1507e8746a4
head_sha: 393ef8e698a8ea9f9254c4d9cad866ebdbabec24
final_head_sha: 393ef8e698a8ea9f9254c4d9cad866ebdbabec24
final_head_frozen_at: 2026-09-28T22:18Z
owner: claude-code-session-01XdHJyZNPJMcmMnmSDgwQvZ (Sol Supervising Architect)
created_at: 2026-09-28
updated_at: 2026-09-28
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_OWNER_DECISION_BATCH_D118_D128_2026-09-28.md
  - docs/agents/tasks/active/OTV2-20260928-owner-decision-batch-d118-d128.md
  - docs/agents/tasks/active/OTV2-20260928-a8-donor-item-identity-epoch-decision.md   # archive move after #1163
  - docs/agents/tasks/archive/OTV2-20260928-a8-donor-item-identity-epoch-decision.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

This task records owner decisions D118-D128 (#162 comment 5879395589), taken as one batch for the
open items of the 2026-09-27/28 decision records: creature-kill XP, respawn home town, low-level
death protection, loot rights, blessings through an NPC, character creation, tester Premium,
mounts, achievements, partial stack pickup and players per Channel. It records the Global facts
that make them implementable and routes each to its lane.

No schema, value, runtime or Platform change is made.

## Architecture and source of truth

- `PROVEN`: the tibia.com manual notes and snapshot (`docs/reference/tibia-manual/`,
  `imports/official/tibia-com/2026-09-28-160207Z`); the superseded or amended decision records
  named in §1.
- `UNKNOWN`: the Newhaven level-6 rule, Global partial stack pickup, the shared-experience
  activity window and remainder rule, respawn before a home city is chosen.

## High-risk authority/recovery qualification

Not applicable. Owner decisions and routing only.

## Acceptance criteria

- [x] The decision document is on an exact frozen head with passing validators.
- [x] Independent exact-head review.
- [x] Protected Merge Queue integration.

## Excluded scope

- Schemas, values, runtime code, Platform changes and content.

## Finding dispositions

Codex review of `caaf436`: two P1 and one P2, all ACCEPTED in repair generation 1 of 1 (#162
convergence rule 5869165340). No owner decision changed.

- 4127412556 (D120 vs D59): D59, the D58 formula from level 1, stays; D120 only confirms the D65
  item threshold (§1, §2.1, §3).
- 4127412566 (shared experience vs `COMBAT01-REWARD-PRINCIPALS` = 1): base XP and stamina ship
  with one principal; shared experience waits for an owner re-decision of the principal ceiling
  (§1, §3, §5).
- 4127412576 (Premium relocation timing): relocation happens on the next login, as the Premium
  activation decision specifies (§2.6).

Numbering correction after the owner's merge decision: the batch was posted as D98-D108, which
collide with D98-D101 (#162 5879348805); renumbered D118-D128 (D110-D117 are also taken), and the
owner's D109 (party principal ceiling, 5879706425) is recorded with the Global value 50.

## Validation

- `python3 tools/agents/validate_governance.py`: PASS on the authoring tree.
- `python3 tools/repository/validate_repository_policy.py`: PASS on the authoring tree.

## Terminal integration

- PR #1180 merged through the Merge Queue on 2026-09-29 as `53c0e004`.
- Review: one Codex review of `caaf436` (two P1, one P2), repaired in `e4cd78b`; renumbered to
  D118-D128 with D109 recorded in `393ef8e` (no logic change, no re-review under D101); the owner
  decided to merge.
- Protected-main readback: the decision document and this record are byte-identical to `393ef8e`;
  the A8 archive differs only by the #1179 edit merged into the branch.
- Archived under `OTV2-20260929-a10-d39-contract-amendments`.

## Context checkpoint

```yaml
last_progress: protected-integrated as 53c0e004; archived
status: completed
branch: claude/gifted-rubin-a0axzx
head_sha: 393ef8e698a8ea9f9254c4d9cad866ebdbabec24
pr: 1180
owner_action_required: null
blocker: null
next_action: null
```
