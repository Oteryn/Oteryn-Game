# OTV2-20261003-charm4-effect-order: CHARM-4 incoming charm effect order decision

```yaml
task_id: OTV2-20261003-charm4-effect-order
title: CHARM-4 incoming charm effect order decision
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: arch/charm4-effect-order-decision-20261003
pr: 1645
base_sha: 3d8c14d0f6326e278cf09e97acd2453b6af31926
head_sha: exact frozen head in the #1622 FREEZE_SHA entry
final_head_sha: exact frozen head in the #1622 FREEZE_SHA entry
final_head_frozen_at: null
owner: claude-code-session-016c5MQoe5CoMk9fxmuuMcFJ (Sol Supervising Architect)
created_at: 2026-10-03
updated_at: 2026-10-03
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_CHARM4_INCOMING_EFFECT_ORDER_DECISION_2026-10-03.md
  - docs/agents/tasks/archive/OTV2-20261003-charm4-effect-order.md
public_contracts: []
depends_on: [D296, D300]
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Candidate decision `CHARM4-INCOMING-EFFECT-ORDER-V1`: one creature attack is one occurrence; Dodge
first (a dodge cancels damage, drain and attached conditions, and nothing else rolls); then the
owner commits; hook 4 runs only on a hit taken, Parry then the minor; Parry reflects the
unmitigated rolled damage, reduced by the creature's armor, not its resistances; Void Inversion
last. Child CHARM-DEF-1.

## Architecture and source of truth

- PROVEN: `apps/game-server/src/combat/charm_effects.rs`; CHARM-0; ATTACK-0 §4; charm-authoring
  `INTEGRATION.md`.
- CIPSOFT_OFFICIAL: archive 4386 (Parry: monster armor applies, resistance ignored).
- TIBIAWIKI_STRUCTURED: Parry revision 1084043.
- OTS_HYPOTHESIS_ONLY: Canary and Crystal minor ordering, OTS Parry call sites (rejected).
