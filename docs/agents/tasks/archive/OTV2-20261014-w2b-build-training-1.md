# OTV2-20261014-w2b-build-training-1

```yaml
task_id: OTV2-20261014-w2b-build-training-1
title: "W2b: skill tries from hits banked with mana in the Character build training receipt (#38, #40, #41)"
mode: IMPLEMENT
status: completed
status_note: "#41 confirmed; #40 rules, banking and the owned-skill refresh; control plane 2026-10-14: attack.rs hook after LOOP-COMBAT-MODES-1, shielding hook deferred, Vocation::None split to LOOP-PROGRESS-1"
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/w2b-build-training
issue: 1622
pr: 2109
base_sha: 67cacb76
head_sha: "the PR head; the control plane records the frozen SHA"
created_at: 2026-10-14
updated_at: 2026-10-14
migration_lease: "0085 (assigned; not used: the 0030 training cause CHECK admits skill level/tries, proven by character_build_postgres_cases r4 training receipt advancing magic level and sword together)"
decisions:
  - docs/architecture/reviews/OTERYN_GAME_A13_CHARACTER_BUILD_STATE_DECISION_2026-09-29.md
  - docs/architecture/reviews/OTERYN_GAME_SKILLS0_WEAPON_SKILLS_DECISION_2026-09-30.md
owned_paths:
  - apps/game-server/src/spell/mana_training.rs
  - apps/game-server/src/spell/hit_training.rs
  - apps/game-server/src/spell/cast.rs   # control plane grant: owned_skills visibility only
  - apps/game-server/src/gameplay_transport/training_save.rs   # control plane grant: drain a due level after a replay
  - apps/game-server/src/durability/character_build.rs
  - apps/game-server/src/domain/progression.rs   # build/training parts only
  - apps/game-server/migrations/0085_*.sql
  - docs/agents/tasks/archive/OTV2-20261014-w2b-build-training-1.md
public_contracts: []
external_repositories: []
blocked: []
next_action: "control plane: review trigger on the frozen head"
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
  successor keeps its owned skill levels. Hits resolved while a checkpoint is retained for replay
  bank on that checkpoint and merge with its receipt; when they reach a level
  (or on a forced save, always), `save_spell_training` writes them before it reports `Saved`. Without owned training nothing trains: that is the production off switch, and the
  formula stays off until DEATH-ML-LOSS-1.
- **Live skill level.** `commit_training_checkpoint` refreshes the owned skill levels after the
  receipt, so a banked level is live only then (A13 §2). This is the control plane's `cast.rs`
  grant: a visibility change only.
- **Migration 0085: not needed.** The 0030 `training` cause CHECK already admits receipts that
  raise skill level and tries. The postgres case `character_build_postgres_cases` (r4) commits
  one training receipt that advances magic level and sword together.

## Follow-ups (control plane decisions, 2026-10-14)

- **Fist hook (`gameplay_transport/attack.rs`).** Lands after LOOP-COMBAT-MODES-1 merges. Each
  resolved swing calls `PlayerSpellState::train_skill(SkillFamily::Fist, now, |c|
  c.melee_hit(block))`. Zero damage counts as `HitBlock::Armor`. The swing also uses the owned
  fist level rather than `constants.starting_skill`.
- **Shielding hook (`creature_auto_attack.rs`).** Deferred. The block site has only the
  aggregated `facts.defense`, with no equipped-shield fact and no spell-state access. Once a real
  shield fact exists, a successful `try_block` calls `train_skill(Shielding, now, |c|
  c.block_hit(has_shield))`.
- **Club, sword, axe, distance and fishing.** API and tests only. They are wired when weapon
  attacks (`melee_hit`, `distance_shot`) and ITEM-USE-0 fishing (`FISHING_TRIES_PER_USE`) exist.
- **#38 vocation none.** Split to LOOP-PROGRESS-1. `Vocation` has no `None` variant, so a
  vocation-none Character gets `CastFactsLoad::NoVocation` and has no spell state. It can
  neither auto-attack nor train.

## Acceptance criteria

- [x] #41 confirmed on the base.
- [x] The hit rules match the reference evidence (unit tests).
- [x] Pending tries bank in a checkpoint; a reached level writes at once; a cast does not bank skills.
- [x] The owned skill levels refresh only after the checkpoint receipt.
- [x] 0085 is not needed (0030 proven by the postgres training case).
- [ ] Independent review, then owner acceptance (outside this worker).

## Validation

- `cargo +1.95.0 fmt --all --check`: pass
- `cargo +1.95.0 test --locked -p oteryn-game-server --lib -- mana_training hit_training training`: pass (17)
- `CARGO_BUILD_JOBS=2 cargo +1.95.0 clippy --locked -p oteryn-game-server --lib --tests -- -D warnings`: pass
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass

## Self-review

I read the whole diff against the SKILLS-0 decision, the A13 decision and the retained Canary
`Player::onAttackedCreatureBlockHit`, `Player::onBlockHit` and `Weapon*::getSkillType`.
Unverified: the runtime callers, which are outside the owned paths.
