# OTV2-20261003-charm-desc-fence-decision

```yaml
task_id: OTV2-20261003-charm-desc-fence-decision
title: "CHARM-DESC-FENCE: live attacker authority for Ability damage commits (D295)"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: arch/charm-desc-fence-decision-20261003
pr: 1638
base_sha: 3d8c14d0f6326e278cf09e97acd2453b6af31926
head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
final_head_frozen_at: null
owner: claude-code-session-016c5MQoe5CoMk9fxmuuMcFJ (Sol Supervising Architect)
created_at: 2026-10-03
updated_at: 2026-10-03
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_CHARM_DESC_FENCE_DECISION_2026-10-03.md
  - docs/agents/tasks/archive/OTV2-20261003-charm-desc-fence-decision.md
public_contracts: []
depends_on: []
blocks:
  - "CHARM-DESC-FENCE-1 structural gate test (follow-up)"
  - "A2 live attacker fence (follow-up)"
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

D295 is ruled C plus (a):
- the live attacker fence is deferred to composition;
- the `CharacterLease` seam stays;
- a binding gate allows no production caller of the three Ability damage bridges until the A2
  follow-up (a character-bound single live slot, rebind only after a terminal session or an
  accepted takeover flow, and every damage write fenced by the current owning lease authority) is merged.

This also corrects the packet: `remove_terminal_session` has production callers on the
grace-expiry path.

## Architecture and source of truth

- `PROVEN`: `ability/commit.rs`, `ability/mod.rs`, `foundation/runtime_actor_carrier.rs` and
  `gameplay_transport/mod.rs` at `3f7f1700`.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: docs only. The A2 task carries the authority review.

## Acceptance criteria

- [x] Decision on an exact frozen head with passing validators.
- [ ] Independent exact-head review (authority, fencing).
- [ ] Protected Merge Queue integration.

## Excluded scope

- Code. The structural gate check is CHARM-DESC-FENCE-1's (decision item 3); the fence is A2's.
  #1625 merged before this decision, without the check.

## Validation

- `python3 tools/agents/validate_governance.py`: "Validated 22 required policy documents and 9
  project lanes." `git diff --cached --check`: clean. Both run on the frozen head before the push.
- Agent test suite (`python -m unittest discover -s tools/agents/tests`): passed locally and in CI
  (`Agent governance / validate` success) on the final frozen head
  `1b374a94aefb4ced1da5a7131fd65fc26eef3c54`. Added by the D309 P2 bundle (review finding
  4173346829).

## Self-review

- Method: whole-diff reread against FND-04 and FND-04B, `ability/commit.rs`, `ability/mod.rs`,
  `foundation/runtime_actor_carrier.rs` and `gameplay_transport/mod.rs`.
- Verdict: no open finding at freeze.

## Independent review

- required: YES (authority and fencing decision). The control plane triggers it on the frozen head.
- Round on 7a26bec3/8bf2ce08: in-place rebind for A2 (fixed in 7bcf8e48).
- Round on 7bcf8e48: P1 4173284936 (the gate trigger is a non-test production reference, checked
  structurally) and P2 4173284940 (this record's sections); both fixed in 4d191ecd.
- Control-plane amendment on 4d191ecd: #1625 had merged; the check moves to the follow-up
  CHARM-DESC-FENCE-1 and the #1625 HOLD lines are removed.
- Record moved to its own task id and path (main already held the #1625 record's path).
- Round on 87396ffb: P1 4173424040 (the lease-generation check is mandatory at the write) and
  P1 4173424046 (no preemption of a healthy controller; rebind only after a terminal session or an
  accepted takeover flow). Fixed in 36d6483d.
- Round on 36d6483d: P1 4173440012 (a stored generation is a snapshot). D282 narrowing: A2 requires
  every damage write to be fenced by the current owning lease authority; stored equality is never
  authority; the lease lifecycle mechanics move to the follow-up decision CHARM-DESC-FENCE-LEASE.

## PR and closeout

- PR #1638. Record archived in the final authoring commit; it reaches `main` only if the PR merges.
