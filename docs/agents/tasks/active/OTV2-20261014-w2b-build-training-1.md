# OTV2-20261014-w2b-build-training-1

```yaml
task_id: OTV2-20261014-w2b-build-training-1
title: "W2b: skill tries from hits banked with mana in the Character build training receipt (#38, #40, #41)"
mode: IMPLEMENT
status: implementing
status_note: "#41 magic level training on cast confirmed working; #40 hit-training rules and live/checkpoint banking in owned paths; runtime hooks and #38 Vocation::None await the control plane's scope answer"
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/w2b-build-training
issue: 1622
pr: null
base_sha: 67cacb76
head_sha: "the PR head; the control plane records the frozen SHA"
created_at: 2026-10-14
updated_at: 2026-10-14
migration_lease: "0085 (assigned; not used: 0030 already admits training receipts that raise skill level and tries)"
decisions:
  - docs/architecture/reviews/OTERYN_GAME_A13_CHARACTER_BUILD_STATE_DECISION_2026-09-29.md
  - docs/architecture/reviews/OTERYN_GAME_SKILLS0_WEAPON_SKILLS_DECISION_2026-09-30.md
owned_paths:
  - apps/game-server/src/spell/mana_training.rs
  - apps/game-server/src/spell/hit_training.rs
  - apps/game-server/src/durability/character_build.rs
  - apps/game-server/src/domain/progression.rs   # build/training parts only
  - apps/game-server/migrations/0085_*.sql
  - docs/agents/tasks/active/OTV2-20261014-w2b-build-training-1.md
public_contracts: []
external_repositories: []
blocked: []
next_action: "control plane: scope answer for the runtime hooks (cast.rs owned_skills, attack.rs, creature_auto_attack.rs) and #38 Vocation::None"
```

## Scope

- **#41 magic level training.** Training on paid casts already works: the D151 path stages a
  `training` receipt on every level advance and at the 60 s checkpoint. The
  `mana_training` tests pass on the base.
- **#40 skill tries from hits.** `spell/hit_training.rs` holds the Canary rules from the retained
  reference evidence:
  - Blood and shield streaks of 30.
  - Melee and fist hits give 1 try; distance gives 2 unblocked and 1 blocked.
  - Shielding gives 1 try per block with a shield, while the streak lasts.
  - Fishing gives 1 try per use.

  `LiveManaTraining` banks these tries live. A checkpoint commits them with the mana spent in
  one `training` receipt; a checkpoint runs when a level is reached, at the 60 s window, and at
  logout or shutdown (A13 §2). A cast banks mana only and leaves the tries pending, so a paid
  successor keeps its owned skill levels. Tries are dropped while a checkpoint is retained for
  replay. Without owned training nothing trains: that is the production off switch, and the
  formula stays off until DEATH-ML-LOSS-1.
- **#38 vocations.** Waiting on the control plane's answer: `Vocation` has no `None` variant, so
  a vocation-none Character has no spell state and cannot train.

## Acceptance criteria

- [x] #41 confirmed on the base.
- [x] The hit rules match the reference evidence (unit tests).
- [x] Pending tries bank in a checkpoint; a reached level writes at once; a cast does not bank skills.
- [ ] Runtime hooks (fist swing, shielding block) and the owned-skill refresh after a skill advance.
- [ ] #38 Vocation::None.
- [ ] Independent review, then owner acceptance (outside this worker).

## Validation

- `cargo +1.95.0 fmt --all --check`: pass
- `cargo +1.95.0 test --locked -p oteryn-game-server --lib -- mana_training hit_training`: pass (11)
- `CARGO_BUILD_JOBS=2 cargo +1.95.0 clippy --locked -p oteryn-game-server --lib --tests -- -D warnings`: pass
- `python tools/agents/validate_governance.py`: see the candidate
- `python -m unittest discover -s tools/agents/tests`: see the candidate
- `git diff --check`: pass

## Self-review

I read the whole diff against the SKILLS-0 decision, the A13 decision and the retained Canary
`Player::onAttackedCreatureBlockHit`, `Player::onBlockHit` and `Weapon*::getSkillType`.
Unverified: the runtime callers, which are outside the owned paths.
