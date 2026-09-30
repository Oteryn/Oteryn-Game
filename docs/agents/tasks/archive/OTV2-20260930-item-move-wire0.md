# OTV2-20260930-item-move-wire0

```yaml
task_id: OTV2-20260930-item-move-wire0
title: "ITEM-MOVE-WIRE-0 item views and item move"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/arch-item-move-wire0
pr: "exact PR in the #162 FREEZE_SHA entry"
base_sha: be8d1d79
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_frozen_at: null
owner: claude-code-session-01KbqAgmFfAYDSKHKkFmWKWW (Sol Supervising Architect)
created_at: 2026-09-30
updated_at: 2026-09-30
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_ITEM_MOVE_WIRE0_ITEM_VIEW_AND_MOVE_DECISION_2026-09-30.md
  - docs/architecture/ADR-0021-world-map-runtime-loading.md
  - docs/contracts/OTERYN_GAME_LIST_CHARACTERS_FOR_ACCOUNT_PROJECTION_V1.md
  - docs/agents/tasks/archive/OTV2-20260930-item-move-wire0.md
public_contracts:
  - docs/architecture/ADR-0021-world-map-runtime-loading.md
  - docs/contracts/OTERYN_GAME_LIST_CHARACTERS_FOR_ACCOUNT_PROJECTION_V1.md
depends_on: []
blocks: []
cross_repository_coordination_id: "Oteryn/Oteryn-Platform#1419 (the Platform consumer implements the LCFA 409 clarification)"
external_repositories: []
```

## Outcome

ITEM-MOVE-WIRE-0 gives the player a view of the own main backpack and of an opened corpse, and a
command to loot a corpse entry into the main backpack (owner D212: "1a 2 a").

- **Wire.** Capability 4 `ITEM_VIEW_MOVE_V1`; state domains 9 `CHARACTER_INVENTORY` and 11
  `OPEN_CONTAINER` (domain 10 released); command type 9 `ITEM_MOVE_INTENT`; a USE item target
  (field 2) to open a corpse; one handle field on D85 item entries. Handles are monotonic per
  GameSession.
- **Value.** The existing `0014` corpse-entry TRANSFER; no new DUR-03 shape.
- **Amends.** USE-WIRE-V1 (field 2), D85 (one handle field), and ADR-0021 §4.5 and §4.6 (teleport
  and palette-key rulings on #162 5910173902). ITEM-VIEW-1 updates the proto comment and adds a
  back-pointer to the MOVE-RL-11 decision.

- **Also carried (architect rulings batch).** The LCFA projection 409 clarification (#162
  5910360309): content means the character set only.

No code, migration or content change is made.

## Architecture and source of truth

- `PROVEN`: USE-WIRE-V1; D84-D87; B3 D80-D83; DUR-03 §39.3; D133/D134; `0011`, `0014`,
  `item_transfer.rs`; FND-02 §13.3 and §15; ADR-0021 §4.4 and §4.5.
- `DERIVED`: the Tibia manual (controls); OTClient `eb262531` (`OTS_HYPOTHESIS_ONLY`).

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: docs only. ITEM-MOVE-1 needs persistence review; ITEM-VIEW-1 protocol review.

## Acceptance criteria

- [ ] Decision on an exact frozen head with passing validators.
- [x] Owner answers Q1a and Q2a (D212); architect adjustments recorded in §9.1.
- [ ] Independent exact-head review (protocol and persistence).
- [ ] Protected Merge Queue integration.

## Excluded scope

- Code, migrations and content; ground and map-item pick-up, drop, reorder, equip, split, nested
  bags, depot, player trade, the client drag-and-drop UI.

## Validation

- `python3 tools/agents/validate_governance.py`: PASS on the final authoring tree.
- `python3 tools/repository/validate_repository_policy.py`: PASS on the final authoring tree.
- `git diff --check`: clean.

## Closeout

- PR: the one named in the #162 FREEZE_SHA entry. Merge commit/result: its squash merge.
- Self-review (`oteryn-hard-worker`, read-only) of the draft: 4 material findings, 1 evidence gap,
  3 hardening, all fixed (ground items stay in domain 1 per D85; handles and revisions monotonic
  per GameSession; USE-WIRE-V1 and D85 amendments declared; slice cut to corpse looting; result
  codes mapped to every writer refusal).
- Second pass: 3 high and 5 lower findings, all fixed (handle as an added field, not D85's
  identity; USE field 2 as a generic item target; handle table bound and resume continuity;
  adjustments recorded as architect corrections; container closing; ADR-0021 amendment wording;
  branch sequencing; VIS-2 and combat loot MINT dependencies).
- Control-plane rule (#162 5912405163): the ADR-0021 and LCFA edits read "pending on acceptance
  of ITEM-MOVE-WIRE-0".
- Independent review of `1afa4946` (5911190413): FIX, 2 medium and 4 low findings, all answered in
  one push: replay by CommandRef before handle resolution with the intent bound to the
  ItemInstanceId and the remaining writer refusals mapped; the handle table is reissued on every
  reconnect snapshot; the domain 10 release noted as narrower than D212; the D133-window
  disclosure declared; handle fields only for capability 4; the ADR-0021 and LCFA edits named in the
  PR review scope, with the Platform coordination recorded.
- This record was archived in the PR's final authoring commit (`docs/agents/tasks/archive/README.md`).

```yaml
last_progress: final authoring commit; archived before freeze
status: completed
branch: claude/arch-item-move-wire0
owner_action_required: null
blocker: null
next_action: null
```
