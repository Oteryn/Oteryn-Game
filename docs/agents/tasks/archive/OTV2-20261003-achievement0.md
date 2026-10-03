# OTV2-20261003-achievement0

```yaml
task_id: OTV2-20261003-achievement0
title: "ACHIEVEMENT-0: achievement grant runtime"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: arch/achievement0-decision-20261003
pr: 1637
base_sha: 4728c4679df6b5dcfa98895342234391d4acd873
head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
final_head_frozen_at: null
owner: claude-code-session-016c5MQoe5CoMk9fxmuuMcFJ (Sol Supervising Architect)
created_at: 2026-10-03
updated_at: 2026-10-03
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_ACHIEVEMENT0_ACHIEVEMENT_GRANT_RUNTIME_DECISION_2026-10-03.md
  - docs/agents/tasks/archive/OTV2-20261003-achievement0.md
public_contracts: []
depends_on:
  - "OTERYN_ACHIEVEMENT_OWNER_CONTRACT_V1 acceptance (grant path §3)"
  - "QUEST-STATE-1, ENC-OUTCOME-1 (gate ACH-QUEST-1, ACH-ENC-1)"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Allocation D292 (#1622). This is the close-out ruling's ACHIEVEMENT-0, with the Character
progression lane as the runtime owner.

- **Granters.** Every remaining granter rides the fenced commit that earns it, through the existing
  `record_achievement_grant`:
  - quest transitions gain an optional `achievement` field, which also covers map and dialogue
    interactions;
  - encounter outcomes bind an `achievement` consumer, whose grant rides a durable per-character
    outcome receipt in the same transaction;
  - counter thresholds grant at the counter owner's durable commit, first for level and skills.
- **Coverage.** A coverage report lists unearnable keys.
- **Notification.** A post-commit delta of state domain 13 (capability 8) announces only a
  `Granted` fact; the domain revision is cumulative per GameSession and never reset at a snapshot.
- **Unchanged.** The reward-claim grant, the chest `USE` path and both contracts.
- **Review round (Codex on c2f2547d).** P1 4173296253 (durable earning commit for encounter
  grants) and P1 4173296257 (cumulative notice revision); both fixed in one push.

## Architecture and source of truth

- `PROVEN`: owner and display contracts; `account_achievement.rs`; QUEST-STATE-0 §4;
  ENCOUNTER-RT-0 §6.4; QUEST-GATE-0 §5.6.
- `CIPSOFT_OFFICIAL`: tibia.com manual §achievements (capture 2026-09-28).
- `OTS_HYPOTHESIS_ONLY`: Canary `addAchievement` and its message text.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: docs only. The children carry persistence and protocol review.

## Acceptance criteria

- [ ] Decision on an exact frozen head with passing validators.
- [ ] Independent exact-head review (persistence, protocol).
