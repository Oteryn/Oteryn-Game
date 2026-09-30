# OTV2-20260930-char-position0

```yaml
task_id: OTV2-20260930-char-position0
title: "CHAR-POSITION-0 durable logout position"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/arch-char-position-0
pr: "exact PR in the #162 FREEZE_SHA entry"
base_sha: 09ccf36b
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_frozen_at: null
owner: claude-code-session-01KbqAgmFfAYDSKHKkFmWKWW (Sol Supervising Architect)
created_at: 2026-09-30
updated_at: 2026-09-30
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_CHAR_POSITION0_LOGOUT_POSITION_DECISION_2026-09-30.md
  - docs/architecture/reviews/OTERYN_GAME_CHARACTER_REVISION_ITEM_TRANSACTION_COMPOSITION_DECISION_2026-09-27.md
  - docs/architecture/MULTICHANNEL_SYSTEM_SCOPE_MATRIX.md
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - docs/agents/tasks/archive/OTV2-20260930-char-position0.md
public_contracts:
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

CHAR-POSITION-0 makes a character log in where it logged out (architect programme plan, #162
5910870596, M2).

- **Storage.** `game_character_last_positions`, one mutable row per Character, a runtime-state
  projection outside the revision chain (composition §3 amendment, scope matrix row).
- **Writes.** Session-fenced upserts ordered by a write sequence: the final one inside terminal
  release, and every 5 minutes (`CHARPOS0-RL-01`) while the position changed. None in instances or
  while a pending respawn or arrival exists.
- **Admission.** Only for a new runtime actor: pending respawn, pending arrival, valid last
  position, then home temple or `entry_start`; Premium relocation; house tiles invalid; NPC-0
  fallback for blocked tiles.

No code, migration or content change is made.

## Architecture and source of truth

- `PROVEN`: `fresh_admission.rs` terminal release; `runtime_actor_carrier.rs` `entry_start`;
  `gameplay_transport` first entry and resume; DEATH-0 and `0016`; the composition decision;
  PREMIUM-ACTIVATION-V1 §4.5; ADR-0001 §7 and §12; ADR-0021.
- `DERIVED`: the Tibia manual (`combat.md` §5.3.12.c) and Tibia login behaviour.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: docs only. CHAR-POSITION-1 needs persistence review.

## Acceptance criteria

- [ ] Decision on an exact frozen head with passing validators.
- [ ] Independent exact-head review (persistence).
- [ ] Protected Merge Queue integration.

## Excluded scope

- Code and migrations; home towns; house entry; channel transfer; PvP logout blocks.

## Validation

- `python3 tools/agents/validate_governance.py`: PASS on the final authoring tree.
- `python3 tools/repository/validate_repository_policy.py`: PASS on the final authoring tree.
- `git diff --check`: clean.

## Closeout

- PR: the one named in the #162 FREEZE_SHA entry. Merge commit/result: its squash merge.
- Self-review (`oteryn-hard-worker`, read-only) of the draft: 3 material findings, 2 evidence
  gaps, 5 hardening, all fixed (composition and scope matrix amendments; final write inside
  terminal release with a write sequence; Premium relocation; `entry_start` fallback and
  HOME-TOWN; read only for a new actor, fail closed on both obligations; placeability always
  checked; house tiles and instances; storage keys, grants and typed encoding; resource registry
  row; logout-block owner in ATTACK-0).
- Control-plane rule (#162 5912405163): the composition, scope matrix and registry edits read
  "pending on acceptance of CHAR-POSITION-0".
- This record was archived in the PR's final authoring commit (`docs/agents/tasks/archive/README.md`).

```yaml
last_progress: final authoring commit; archived before freeze
status: completed
branch: claude/arch-char-position-0
owner_action_required: null
blocker: null
next_action: null
```
