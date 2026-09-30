# OTV2-20260930-attack0-auto-attack

```yaml
task_id: OTV2-20260930-attack0-auto-attack
title: "ATTACK-0 attack target and auto-attack"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/arch-attack-0
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
  - docs/architecture/reviews/OTERYN_GAME_ATTACK0_ATTACK_TARGET_AND_AUTO_ATTACK_DECISION_2026-09-30.md
  - docs/agents/tasks/archive/OTV2-20260930-attack0-auto-attack.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

ATTACK-0 lets a player pick a creature target and fight it, and lets creatures hit back
(architect programme plan, #162 5910870596, M1).

- **Wire.** Capability `ATTACK_V1` (number at allocation; 5 went to `DEPOT_V1`), command types 10 `ATTACK_TARGET_INTENT` and 11
  `FIGHT_MODES_INTENT`, state domain 10 `ACTOR_COMBAT_STATE` (#162 reservations, re-checked at
  allocation).
- **Runtime.** One target per actor; one swing occurrence per 2,000 ms deadline
  (`DEADLINE_STATE`), with a stable identity, RNG purposes and charm hooks, through
  GAME-ABILITY-01. Creature melee on the same rules. A 60 s in-fight deadline blocks logout.
- **Slice.** Creatures only, fists and melee, STAND only; distance, wands, chase and PvP later.
- **Formulas.** Canary formulas on the existing spell formula engine, `PARITY_PENDING` against
  TibiaPal.

No code, migration or content change is made.

## Architecture and source of truth

- `PROVEN`: GAME-ABILITY-01 and the whole gate; VSL-COMBAT-01; DUR-03 §15; GA-XD-02;
  `spell/cast.rs`, `spell/formula.rs`, `charm_effects.rs`; MOVE-RL-11 (D85); the Tibia manual
  (`combat.md`).
- `DERIVED`: Canary `04b83b51` (`OTS_HYPOTHESIS_ONLY`); TibiaPal (owner-trusted).

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: docs only. ATTACK-WIRE-1 needs protocol review; ATTACK-1 combat review.

## Acceptance criteria

- [ ] Decision on an exact frozen head with passing validators.
- [ ] Independent exact-head review (protocol and combat).
- [ ] Protected Merge Queue integration.

## Excluded scope

- Code, migrations and content; PvP; chase; distance, ammunition, wands and rods; combat effects
  view; creature spells.

## Validation

- `python3 tools/agents/validate_governance.py`: PASS on the final authoring tree.
- `python3 tools/repository/validate_repository_policy.py`: PASS on the final authoring tree.
- `git diff --check`: clean.

## Closeout

- PR: the one named in the #162 FREEZE_SHA entry. Merge commit/result: its squash merge.
- Self-review (`oteryn-hard-worker`, read-only) of the draft: 4 material findings, 2 evidence
  gaps, 5 hardening, all fixed (swing occurrence identity and RNG purposes; `DEADLINE_STATE`
  catch-up; distance cut from the slice for lack of a DUR-03 ammunition cause; creature melee
  added; formula citations completed and moved onto the spell formula engine; time-based blocks;
  attacker protection zone; in-fight deadline owner; all charm hooks and independent cooldown;
  SPELL-TARGET-1 split out; dependencies and protocol reservations stated).
- Protocol ledger update (#162 5912405163): capability 5 is `DEPOT_V1`; this decision's capability
  number is reserved at allocation, and command type 11 is requested.
- This record was archived in the PR's final authoring commit (`docs/agents/tasks/archive/README.md`).

```yaml
last_progress: final authoring commit; archived before freeze
status: completed
branch: claude/arch-attack-0
owner_action_required: null
blocker: null
next_action: null
```
