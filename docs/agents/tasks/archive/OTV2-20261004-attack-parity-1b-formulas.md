# OTV2-20261004-attack-parity-1b-formulas

```yaml
task_id: OTV2-20261004-attack-parity-1b-formulas
title: "ATTACK-PARITY-1b: player auto-attack bounds follow the pinned TibiaTools expression"
mode: IMPLEMENTATION
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/loop-combat-modes-1-parity-1b-20261013
pr: 2091
base_sha: main
head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
final_head_frozen_at: null
owner: oteryn-impl-worker (CP #1622, LOOP-COMBAT-MODES-1 Wave 1)
created_at: 2026-10-13
updated_at: 2026-10-13
execution_policy: continuous_progress
migration_lease: none
owned_paths:
  - content/combat/attack_constants_v1.json
  - apps/game-server/src/combat/attack/constants.rs
  - apps/game-server/src/combat/attack/formulas.rs
  - apps/game-server/src/combat/attack/mod.rs
  - apps/game-server/src/combat/attack/tests.rs
  - apps/game-server/src/combat/attack/parity_tests.rs
  - docs/agents/tasks/archive/OTV2-20261004-attack-parity-1b-formulas.md
public_contracts: []
depends_on: [OTV2-20261004-attack-parity-1a-fixtures]
blocks: [ATTACK-DIST-0]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Packet: `docs/architecture/reviews/OTERYN_GAME_ATTACK_PARITY1_AUTO_ATTACK_PARITY_SCOPE_PACKET_2026-10-04.md` 1.3, 1.4, 2.2.
The player melee and fist bounds now follow the pinned TibiaTools expression (commit a1d368906c, `damage.ts:13-25`,
`character-state.ts:73-75`): `v = floor(6*attack/5) * (skill+4)/28`, `min = floor(flat + v/2)`, `max = floor(flat + 2v)`,
`flat` being the official level curve, fists attack 7. The constants table holds those integers instead of
`damage_coefficient`; no migration, no protocol or registry change.

- Min and max equal all 1536 fixture rows exactly; the engine equals the transcribed expression over the offline range.
- Mean residual: the uniform-draw mean differs from `raw.avg` (`flat + v`) on all 1536 rows, so every attack value keeps
  `PARITY_PENDING` (per-value labels added) and ATTACK-DIST-0 must decide the distribution. Report on #162.
- Attack-mismatch list is empty: all 15 selected weapons agree with the catalogue attack.
- Assumption: the fight-mode attack factor multiplies `v` (Offensive = 1.0 is the fixture binding, so parity holds; Balanced
  and Defensive stay pending).
- Changed expected values in `combat/attack/tests.rs`: fists (8,10) Off 3..9, Bal 2..7, Def 2..5; fists (100,50) Off 27..50;
  melee (100,100,50,Off) 131..465; (300,120,45,Bal) 149..418; level term (500,10) 102..108, (600,10) 118..124.

## Validation

- `cargo test --locked -p oteryn-game-server --lib -- combat_attack`: pass
- `cargo fmt --all --check`: pass
- `cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings`: pass
- `git diff --check`: pass
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
