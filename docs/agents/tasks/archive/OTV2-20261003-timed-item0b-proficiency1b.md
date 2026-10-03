# OTV2-20261003-timed-item0b-proficiency1b

```yaml
task_id: OTV2-20261003-timed-item0b-proficiency1b
title: "TIMED-ITEM-0B and PROFICIENCY-1B decisions (D351)"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/timed-item0b-proficiency1b-decisions
pr: 1662
base_sha: d30e271b
head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
final_head_frozen_at: null
owner: claude-code-session-01QFRdKvFbNsNiCzMyR75FrC (Sol Supervising Architect, second lane)
created_at: 2026-10-03
updated_at: 2026-10-03
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_TIMED_ITEM0B_RUNTIME_CHARGES_AND_DURATION_DECISION_2026-10-03.md
  - docs/architecture/reviews/OTERYN_GAME_PROFICIENCY1B_PERK_MODIFICATION_VALUE_SHAPES_DECISION_2026-10-03.md
  - docs/agents/tasks/archive/OTV2-20261003-timed-item0b-proficiency1b.md
public_contracts: []
depends_on:
  - "control-plane leases D353: migrations 0054 (TIMED-RT-1) and 0055 (PROF-SHAPE-1); capability 11 (TIMED_ITEMS_V1); command types 15-20 under capability 2"
  - "owner answer D360 (carried torches: c, Ground-deadline model; a lit item goes out in any container)"
blocks:
  - TIMED-RT-1, TIMED-FX-1, TIMED-WIRE-1, TIMED-PARITY-1, TIMED-HOUSE-1, TIMED-REPAIR-1, EXERCISE-1
  - PROF-SHAPE-CONTENT-1, PROF-SHAPE-1, PROF-SHAPE-WIRE-1
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Allocation: D351 (#1622, owner 1b), an exception to D252. Two decisions in one PR, docs only.

- **TIMED-ITEM-0B** answers TIMED-ITEM-0 §5's eight entry conditions: a per-item write lane
  serializes checkpoint and expiry (the D285 race); write records keyed by (item, expected
  revision); checkpoints at the A13 cadence, logout, handoff and death; stop before leaving a slot;
  one-item expiry shapes; equip and use forms with ceilings and `SWAP_TIMED_BOTH` as `BLOCKED`;
  the EQUIP-0 active rule, `ITEM_REGENERATION` and the item mana shield; `TIMED_ITEMS_V1`;
  torches per owner answer D360 (live in a slot, a durable database-time deadline on Ground and
  house tiles, put out in any container, so no container-tree shape); the EXERCISE-0 composed
  checkpoint and expiry. The first freeze (4e442b72) assumed equipped-only torches; D360 superseded
  it and the head was re-frozen.
- **PROFICIENCY-1B** answers PROFICIENCY-1 §6's fourteen entry conditions: the shaping content
  family with per-cell evidence classes; the modification table, lines and terminal records; the
  six operations with typed results; draws under `proficiency_shaping`; composed dust and orb
  burns; the retention lock with content activation; integrity extensions; `MODIFIED_LEVEL`; six
  commands under capability 2 with RL-01 raised to 65,536 B. Every operation answers
  `NOT_ADMITTED` until its cells are evidenced.

Contract amendments are listed in each header and written by the named child in its own docs
commit, so this PR touches only its three owned paths.

## Architecture and source of truth

- `PROVEN`: TIMED-ITEM-0 §3-§5 (merged #1471); its round 1-12 draft at `d708a63c`; EQUIP-0;
  ITEM-USE-0; EXERCISE-0; DUR-03; PROFICIENCY-0, PROFICIENCY-1, PROF-WIRE-0; IMBUE-FORGE-0 §9-§10;
  migration 0032; the Tibia manual `combat.md`.
- `UNKNOWN`: TibiaWiki was unreachable (HTTP 402) from this container; per-cell values stay
  parity gates.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: docs only. The children implement and test.

## Acceptance criteria

- [x] Leases filled from the control plane (D353).
- [ ] Decisions on an exact frozen head with passing validators.
- [ ] Independent exact-head review (persistence, economy, determinism, protocol).

## Validation

- `git diff --check`
- `python tools/agents/validate_governance.py`
- `python -m unittest discover -s tools/agents/tests`
