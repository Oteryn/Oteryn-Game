# OTV2-20260928-reference-first-player-death-decision

```yaml
task_id: OTV2-20260928-reference-first-player-death-decision
title: "Reference first player death decision (D58-D68)"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/gifted-rubin-a0axzx
issue: 162
pr: 1132
base_sha: 0a3d795992a68f57c595baf3f5ced87d1224a60c
head_sha: 1765cd32a37d069922e63f2d307dce7ee5178bd3
final_head_sha: 1765cd32a37d069922e63f2d307dce7ee5178bd3
final_head_frozen_at: 2026-09-28T16:21Z
owner: claude-code-session-01XdHJyZNPJMcmMnmSDgwQvZ (Sol Supervising Architect)
created_at: 2026-09-28
updated_at: 2026-09-28
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_REFERENCE_FIRST_PLAYER_DEATH_DECISION_2026-09-28.md
  - docs/architecture/OTERYN_REFERENCE_DEATH_XP_SPAN_OWNER_BASELINE_2026-09-09.md   # amendment pointer only
  - docs/agents/tasks/active/OTV2-20260928-reference-first-player-death-decision.md
  - docs/agents/tasks/active/OTV2-20260928-vsl-combat-resource-rows-decision.md   # archive move after #1123
  - docs/agents/tasks/archive/OTV2-20260928-vsl-combat-resource-rows-decision.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

This task records the durable text for the Reference first player death decision: owner decisions
D58-D60 and D62-D68 (#162 comments 5871032954 and 5871151324), updated for Premium D76 (#1118). It
defines the trigger, the death occurrence, one Character death transaction (XP loss, blessings,
Amulet of Loss, lost-item set, respawn position), per-item DUR-03 drops, respawn and the delivery
children DEATH-1 to DEATH-4. It amends difference 1 of the 2026-09-09 death XP baseline; difference
2 stays.

No runtime, migration, protocol or registry change.

## Architecture and source of truth

- `PROVEN`:
  - `OTERYN_REFERENCE_DEATH_XP_SPAN_OWNER_BASELINE_2026-09-09.md`;
  - `domain/progression.rs` `ApplyDeathExperienceLoss`;
  - the Premium activation decision (#1118) and the GAME-AI first creature slice (#1110, D54);
  - the Global sources in #162 5871032954 and the tibia.com snapshot (#1125).
- `UNKNOWN`: the Newhaven low-level exemption at the target date; temple positions (content).

## High-risk authority/recovery qualification

Not applicable. This decision selects gameplay rules and routes the durable parts to existing
Character (P03 session-generation fence) and DUR-03 TRANSFER paths; each DEATH child carries its
own qualification when allocated. The death occurrence is the idempotency key, and the restart rule
never duplicates or doubly loses an item.

## Acceptance criteria

- [x] The decision document is on an exact frozen head with passing validators.
- [x] Independent exact-head review (one Codex review, one repair generation; owner decision to merge after green CI).
- [x] Protected Merge Queue integration (`e6141b41`).

## Excluded scope

- Runtime code, the calculator change, blessing prices, PvP death, Death Redemption, charms.

## Finding dispositions

Codex review of `846788d`, three P1 findings, all ACCEPTED in repair generation 1 of 1 (#162
convergence rule 5869165340). No owner decision changed.

- 4124521185 (restart left selected items in the inventory): the committed lost-item set is the
  durable outcome; a death item workflow resumes every outstanding (occurrence, item) cause after
  a restart, and the character is not respawned until all item operations commit (decision §4.4,
  §4.5, §6).
- 4124521176 (the P03 store cannot commit the death transaction): the `0009` guard admits only XP
  increases with an XP-award receipt; DEATH-0 (death receipt and migration decision) is added as a
  prerequisite of DEATH-1 (decision §4.3, §5).
- 4124521197 (Amulet of Loss consumed outside DUR-03): the amulet is consumed by a DUR-03 destroy
  with a typed sink cause, composed after the Character outcome like the drops (decision §4.3,
  §4.4, §6).

## Validation

- `python3 tools/agents/validate_governance.py`: PASS on the authoring tree.
- `python3 tools/repository/validate_repository_policy.py`: PASS on the authoring tree.

## Terminal integration

- PR #1132 merged through the Merge Queue on 2026-09-28 as `e6141b41`.
- Review: one Codex review of `846788d` (three P1s), repaired in the single repair generation
  `1765cd3`; the owner decided to merge after green CI (#162 comment 5874517804).
- Protected-main readback: the decision, the baseline pointer, this record and the archived #1123
  record on `e6141b41` are byte-identical to the frozen head `1765cd3`.
- The PR body still describes the pre-repair restart rule; decision §4.4 is authoritative.
- Follow-up: the fresh empty bag after a lost backpack (tibia.com manual §characters 5.1.11) is
  carried to DEATH-3 through the B3 decision.
- Next allocations: DEATH-0; DEATH-1 after DEATH-0 and progression readiness.
- Archived under `OTV2-20260928-b3-inventory-destination-decision`.

## Context checkpoint

```yaml
last_progress: protected-integrated as e6141b41; archived
status: completed
branch: claude/gifted-rubin-a0axzx
head_sha: 1765cd32a37d069922e63f2d307dce7ee5178bd3
pr: 1132
owner_action_required: null
blocker: null
next_action: null
```
