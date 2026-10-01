# OTV2-20261001-mail-parcel-tree0

```yaml
task_id: OTV2-20261001-mail-parcel-tree0
title: "MAIL-0 amendment: MAIL0-RL-03 parcel trees after BAGS-0"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/arch-mail-parcel-tree-0
pr: "1427"
base_sha: "origin/main at branch creation (after #1404)"
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_frozen_at: null
owner: claude-code-session_018aTt5eRKVUGPJJYwwoMcqw (Sol Supervising Architect)
created_at: 2026-10-01
updated_at: 2026-10-01
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_MAIL0_PARCELS_AND_LETTERS_DECISION_2026-09-30.md
  - docs/architecture/reviews/OTERYN_GAME_BAGS0_CONTAINERS_WITH_CONTENTS_DECISION_2026-09-30.md
  - docs/agents/tasks/archive/OTV2-20261001-mail-parcel-tree0.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Handoff item 3 (#162 comment 5919202418): once BAGS-0 (PR #1416) is in, amend `MAIL0-RL-03`.
MAIL-0 (PR #1404) and BAGS-0 have both merged.

- MAIL-0 §7: a parcel carries its whole tree, as in Tibia. It has at most 10 direct children (its
  capacity), and any child may have contents, within BAGS-0 §3's bounds. Posted from depth 1 of the
  main backpack, so the height is at most 7 and the parcel holds at most 499 items.
- The address is still read only from direct-child labels, matching Canary
  `Mailbox::getReceiver`, which iterates only the parcel's own item list.
- Posting is BAGS-0 §4.3's tree move plus the stamp transform. Taking a child out is the one-item
  TRANSFER, or the tree move for a child with contents. A parcel goes into a depot box only under
  BAGS-DEPOT-1.
- MAIL-PARCEL-1 depends on BAGS-1 and BAGS-DEPOT-1 (Inbox trees). The MAIL-0 implementation-brief
  paragraph on BAGS-0 now gives its current state (candidate merged in #1416, awaiting acceptance).
- MAIL-0 §11 rows: `MAIL0-RL-03`, parcel posting (at most 499 items and 502 work units, under
  `DUR03-RL-05-TREE` and `BAGS0-RL-06`), parcel child out.
- MAIL-0 §13 and §15: nested bags moved from declared differences to parity kept.
- BAGS-0 §9 and the follow-up list now point to this amendment.

No code, migration or content change is made.

## Architecture and source of truth

- `PROVEN`: BAGS-0 §3, §4, §9 (PR #1416); MAIL-0 §7 (PR #1404); Canary
  `src/items/containers/mailbox/mailbox.cpp` `getReceiver` (direct item list only).

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: docs only. The amendment changes a persistence bound, so it gets one independent
review on the frozen head.

## Acceptance criteria

- [ ] Amendment on an exact frozen head with passing validators and one Codex review.
- [ ] Protected Merge Queue integration.

## Excluded scope

- Code, migrations, content; Store-bound items; house mailboxes; posting from the ground.

## Validation

- `python3 tools/agents/validate_governance.py`: PASS.
- `python3 tools/repository/validate_repository_policy.py`: PASS.
- `git diff --cached --check`: clean.

## Closeout

- PR: #1427. Merge commit/result: its squash merge.
- This record was archived in the PR's final authoring commit (`docs/agents/tasks/archive/README.md`).

```yaml
last_progress: final authoring commit; archived before freeze
status: completed
branch: claude/arch-mail-parcel-tree-0
owner_action_required: null
blocker: null
next_action: null
```
