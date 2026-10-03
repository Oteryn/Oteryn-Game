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
first (a dodge cancels damage, drain and attached conditions, and nothing else rolls); then
mitigation without commit; hook 4 runs only on a hit (health damage after mitigation > 0), Parry
before the commit; Void Inversion replaces the drain before the commit; the commit then applies
the mana shield (after the defensive charms, CONDITIONS-0 §3.3), health, the drain and attached
conditions; the minor applies after the commit. Parry reflects the unmitigated rolled damage,
reduced by the creature's armor, not its resistances. Mixed occurrences run both hooks on their
own components. Child CHARM-DEF-1.

## Architecture and source of truth

- PROVEN: `apps/game-server/src/combat/charm_effects.rs`; CHARM-0; ATTACK-0 §4; charm-authoring
  `INTEGRATION.md`.
- CIPSOFT_OFFICIAL: archive 4386 (Parry: monster armor applies, resistance ignored).
- TIBIAWIKI_STRUCTURED: Parry revision 1084043.
- OTS_HYPOTHESIS_ONLY: Canary and Crystal minor ordering, OTS Parry call sites (rejected).

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: docs only, no durable write; CHARM-DEF-1 carries the determinism review.

## Acceptance criteria

- [x] Decision on an exact frozen head with passing validators.
- [ ] Independent exact-head review (combat, determinism).
- [ ] Protected Merge Queue integration.

## Validation

- `python3 tools/agents/validate_governance.py`: "Validated 22 required policy documents and 9
  project lanes." `git diff --cached --check`: clean. Both run before each push.

## Self-review

- Method: whole-diff reread against `charm_effects.rs` (hooks, draws, the event-fact checks),
  CHARM-0, ATTACK-0 §4 and CONDITIONS-0 §3.2-§3.3.
- Verdict: no open finding at freeze.

## Independent review

- required: YES. The control plane triggers it on the frozen head.
- Round on 00ae677c: P1 4173326922 (condition ticks keep charm evaluation per CONDITIONS-0 §3.2),
  P2 4173326927 (the hit gate excludes a pure mana drain), P2 4173326933 (these sections); all
  fixed in one push.

## PR and closeout

- PR #1645. Record archived in the final authoring commit; it reaches `main` only if the PR merges.
