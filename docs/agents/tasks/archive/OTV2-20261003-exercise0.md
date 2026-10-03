# OTV2-20261003-exercise0

```yaml
task_id: OTV2-20261003-exercise0
title: "EXERCISE-0: exercise weapons and exercise dummies"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: arch/exercise0-decision-20261003
pr: null
base_sha: 4728c4679df6b5dcfa98895342234391d4acd873
head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
final_head_frozen_at: null
owner: claude-code-session-016c5MQoe5CoMk9fxmuuMcFJ (Sol Supervising Architect)
created_at: 2026-10-03
updated_at: 2026-10-03
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_EXERCISE0_EXERCISE_WEAPONS_AND_DUMMIES_DECISION_2026-10-03.md
  - docs/agents/tasks/archive/OTV2-20261003-exercise0.md
public_contracts: []
depends_on:
  - "TIMED-ITEM-0 acceptance (timed-row table, §4)"
  - "TIMED-ITEM-0B (checkpoint shape, expiry sink; gates EXERCISE-1)"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Allocation D291 (#1622, control plane) answers TIMED-ITEM-0 R4, OFFLINE-0 R3 and SKILLS-0 §3.5 with
EXERCISE-0, as part of owner answer 2a.

- **Content.** It covers 24 exercise weapon definitions (eight kinds, three tiers) with charges in
  TIMED-ITEM-0's timed-row table, NPC BUY at TibiaWiki prices, public Training School dummies and a
  `PARITY_PENDING` per-charge value table.
- **Training.** Training starts with `USE-WITH` on a dummy, in a PZ and within reach, with a 30 s
  cooldown. It then ticks one charge every 2 s and stops on any act. The idle kick pauses while
  training.
- **Durability.** One atomic training checkpoint commits the spent charges and the paid tries or
  mana together, on TIMED-ITEM-0B's checkpoint shape. The last charge retires the weapon through
  TIMED-ITEM-0B's expiry sink.
- **Deferred.** House and expert dummies, training weapons (DAILY-REWARD-0), the Store and the
  client display are deferred.

## Architecture and source of truth

- `PROVEN`: TIMED-ITEM-0 §4-§5 and R4; SKILLS-0 §3.4-§3.5; A13 §4.5; WORLD-INTERACTION-0 §3;
  OFFLINE-0 R3.
- `TIBIAWIKI_STRUCTURED`: Exercise Weapons rev 1126820, Exercise Dummy rev 1011205, Training rev
  1126700, Training School rev 1101626.
- `OTS_HYPOTHESIS_ONLY`: Canary exercise rate and reach.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: docs only. EXERCISE-1 implements and tests the checkpoint with persistence review.

## Acceptance criteria

- [ ] Decision on an exact frozen head with passing validators.
- [ ] Independent exact-head review (persistence, determinism).
