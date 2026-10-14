# MAGIC-CAST-1-BUNDLE

```yaml
task_id: MAGIC-CAST-1-BUNDLE
title: "MAGIC-CAST-1: bundle World casts (locate gate, heal/support evidence, #26 confirmation)"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
issue: 2048
base_branch: main
branch: claude/magic-cast-1-cast-20261014
pr: 2112
base_sha: 67cacb76
owner: claude-code-session-01PuXSgNvMavAtgh4pb4U6TA
control_plane: session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-14
updated_at: 2026-10-14
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/spell/cast_tests.rs
  - apps/game-server/src/gameplay_transport/parameter_cast.rs
  - docs/agents/tasks/archive/MAGIC-CAST-1-BUNDLE.md
public_contracts: []
depends_on: []
```

## Outcome

- **#49 / #53, parameter cast bundle gate.** On a bundle World `qualified_room` is None, so
  `cast_parameters` refused every parameter spell. A locate message (Find Person, Find Fiend) reads
  its room only for the caster's protection-zone tile. `parameter_room` now serves it from the combat
  room, which on a bundle World is the entry room's bundle view (BUNDLE-COMBAT-ENABLE-1). House access
  and vertical moves read fixture house and floor facts, so they stay closed on a bundle World.
- **#50, healing and support on a bundle World: evidence.** Self heals and support Effects spells go
  through `cast_native_combat`, which is gated only on `combat_room` and is already open on a bundle
  World. A new test casts Light Healing, Intense Healing, Light and Cure Poison from the real native
  book on the bundle protection-zone tile. On the same tile an aggressive Terra Wave is refused, and
  off the tile it casts. The DB-free book sweep (SPELL-BOOK-ACTIVATE-1) already covers every self
  heal and support Effects spell.
- **#52, support spells.** Haste-family spells (`companion_haste`) are native-combat keys on the same
  open `combat_room` gate. Nothing in the owned paths is missing for them.
- **#26, mana shield for other vocations: confirmed, no PR.** The absorb in
  `spell/actor_conditions.rs` applies `mana_shield_at(now)` with no vocation check. Magic Shield
  `utamo vita` is limited to druid and sorcerer lines at level 14 by content
  (`player-spells.json`), matching the source.

## Follow-ups

- **PARAM-WIRE (child; the control plane hands it to the architect).** `cast_parameters` has no
  production caller. Three things are missing:
  - registry ids for `ACTOR_SPELL_PARAMETERS_V2` / `WORLD_ACTOR_SPELL_CAST_PARAMETERS_V2`;
  - `connection.rs` dispatch;
  - utevo res (`acquire_summon`).

  Until then #53 (exiva, utevo res) is not reachable by a client.
- **SPELL-RT-PREM-1.** The premium gate (§4.6) is the last commit, after PREMIUM-1 merges.

## Validation

- `cargo +1.95.0 fmt --all --check`: pass
- `git diff --check`: pass
- `cargo +1.95.0 clippy --locked -p oteryn-game-server --all-targets -- -D warnings`: pass
- `cargo +1.95.0 test --locked -p oteryn-game-server --lib -- spell:: gameplay_transport::parameter_cast gameplay_transport::actor_spell`: 451 passed, 0 failed
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: 60 tests, OK (code head 436788b1; this commit changes only this record)
- New tests:
  - `bundle_world_opens_locate_message_only`
  - `a_bundle_world_protection_zone_casts_heal_and_support_and_refuses_aggression`
