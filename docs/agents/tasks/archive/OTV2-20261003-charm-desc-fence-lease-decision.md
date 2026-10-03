# OTV2-20261003-charm-desc-fence-lease-decision

```yaml
task_id: OTV2-20261003-charm-desc-fence-lease-decision
title: "CHARM-DESC-FENCE-LEASE (D324): lease lifecycle mechanics of the attacker write fence"
mode: CONTRACT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: arch/p2-bundle-d309-20261003
pr: "1651"
base_sha: ac6fdca88963d1b525843482e9913c7caa8661af
head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
final_head_frozen_at: null
owner: claude-code-session-016c5MQoe5CoMk9fxmuuMcFJ (Sol Supervising Architect)
created_at: 2026-10-03
updated_at: 2026-10-03
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_CHARM_DESC_FENCE_LEASE_DECISION_2026-10-03.md
  - docs/agents/tasks/archive/OTV2-20261003-charm-desc-fence-lease-decision.md
public_contracts: []
depends_on:
  - "CHARM-DESC-FENCE-V1 (D295, #1638)"
blocks:
  - "A2 live attacker fence (follow-up)"
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Way (2) of D295: fence first, then commit. Every transition that ends a GameSession's hold on the
Character lease does three things in order:
- it fences the slot's writes under the runtime lock, with no I/O;
- it commits the durable transition outside the lock;
- it settles the slot (retire or rebind, lift the fence by its exact token, or keep the fence and
  retry).

The runtime lock is never held across a durable transaction.

## Architecture and source of truth

- `PROVEN`: `durability/fresh_admission.rs` (`release_expired_loss`, `release_abandoned_session`),
  `gameplay_transport/mod.rs` (grace-expiry callers) and `foundation/runtime_actor_carrier.rs`
  (slot, control-loss mark, damage admission) at `ac6fdca8`.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: docs only. A2 carries the authority review and the tests of the decision's
checklist item 3.

## Acceptance criteria

- [x] Decision on an exact frozen head with passing validators.
- [ ] Independent exact-head review (authority, fencing).
- [ ] Protected Merge Queue integration.

## Excluded scope

- Code (A2). Uncontrolled-actor combat behaviour. Second-session semantics.

## Validation

- Run with the D309 bundle: `validate_governance.py`, `git diff --cached --check`, and the agent
  test suite on the frozen head.

## Self-review

- Method: whole-diff reread against D295 item 4, FND-04B §6 and the cited code.
- Verdict: no open finding at freeze.

## Independent review

- required: YES. The control plane triggers it on the frozen bundle head.

## PR and closeout

- Shipped in PR #1651 (D309 P2 bundle). Record archived in the final authoring commit; it reaches
  `main` only if the PR merges.
