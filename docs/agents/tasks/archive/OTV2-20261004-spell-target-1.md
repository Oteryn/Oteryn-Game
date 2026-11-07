# OTV2-20261004-spell-target-1

```yaml
task_id: OTV2-20261004-spell-target-1
title: SPELL-TARGET-1 - single-target spells resolve the held attack target
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/spell-target-1-20261004
issue: 1622
pr: 1808
base_sha: 64c1bb3d
decision: docs/architecture/reviews/OTERYN_GAME_ARCH_BATCH_SPELL_ATTACK_PACKETS_2026-10-04.md §1.4, §2.2
owner: "SPELL-TARGET-1 worker (control plane session_013KJX6mv8LQveCKKXYgAX94)"
created_at: 2026-10-05
owned_paths:
  - apps/game-server/src/spell/cast.rs                         # AttackTarget arm comment only
  - apps/game-server/src/spell/cast_tests.rs
  - apps/game-server/src/gameplay_transport/ordinary_combat.rs # the AttackTarget arm
  - apps/game-server/src/gameplay_transport/native_combat_cast.rs  # LEASE EXTENSION (CP D708): the combat_state(...).target read in cast_native_combat_inner and the new `attack_target` argument to ordinary_combat::prepare only
  - apps/game-server/src/gameplay_transport/attack.rs          # LEASE EXTENSION (CP D713): the `sees` visibility change (pub(super)) and its tests only
  - apps/game-server/src/gameplay_transport/spell_book_sweep_tests.rs
  - docs/agents/tasks/archive/OTV2-20261004-spell-target-1.md
```

## Lease extension

The attack target lives in `ChannelOwner.attack`; only `cast_native_combat_inner` holds it.
CP approved adding `native_combat_cast.rs` (D708), limited to the read and the `prepare` argument.
CP approved adding `attack.rs` (D713), limited to exposing `sees` and its tests: the held target is
rechecked against the caster's reference visibility window.

## Behaviour

`SpellTarget::AttackTarget` resolves the held target: none -> `TargetRequired`; not a visible
creature (including outside the reference window), other floor, out of sight, out of range or
not attackable -> `TargetIllegal`. A single-target spell is centred on the target. A directional or
aimed spell (Fire Wave, Energy Beam) keeps the caster as geometry origin and takes only its facing
from the target. `prepare_named` passes no target.

## Validation

- `cargo fmt --all -- --check`: pass
- `cargo clippy -p oteryn-game-server --all-targets -- -D warnings`: pass
- `bash tools/qualification/spells/run.sh runtime`: pass (322 passed)
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass

Review: combat review (Codex) on the final frozen head, routed by the control plane.
