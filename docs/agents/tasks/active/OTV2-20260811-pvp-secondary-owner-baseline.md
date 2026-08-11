# OTV2-20260811-pvp-secondary-owner-baseline

```yaml
task_id: OTV2-20260811-pvp-secondary-owner-baseline
title: Persist GAME-VISION-01 PvP secondary-pillar owner baseline
mode: COORDINATE
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/OTV2-20260811-pvp-secondary-owner-baseline
pr: 165
base_sha: 78c08e658cb4acb2f6e7298841b223d2ebf3cf5d
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT architecture coordinator
created_at: 2026-08-11T11:46:00+02:00
updated_at: 2026-08-11T11:54:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/agents/tasks/active/OTV2-20260811-pvp-secondary-owner-baseline.md
  - docs/architecture/GAME-VISION-01_PVP_SECONDARY_PILLAR_OWNER_BASELINE.md
public_contracts:
  - docs/architecture/GAME-VISION-01_PVP_SECONDARY_PILLAR_OWNER_BASELINE.md
depends_on:
  - GAME-VISION-01_PLAYER_PROMISE_OWNER_BASELINE.md
  - GAME-VISION-01_REFERENCE_FIRST_OWNER_BASELINE.md
  - GAME-VISION-01_REFERENCE_HYBRID_TRACKING_OWNER_BASELINE.md
  - GAME-VISION-01_EVOLVED_RELIABILITY_UX_FIRST_OWNER_BASELINE.md
  - GAME-VISION-01_PREDECISION_ANALYSIS.md
  - ADR-0010-reference-and-evolved-world-product-profiles.md
blocks:
  - GAME-VISION-01 owner decision packet item 6 closure
  - first Reference/Evolved product-proof and later alpha PvP breadth classification
  - PvP-specific backlog priority relative to PvE/progression/economy/client/content work
cross_repository_coordination_id: OTV2-GLOBAL-ARCHITECTURE
external_repositories: []
```

## Outcome

Persist the owner's explicit acceptance that **PvP is a secondary pillar of Oteryn**: a serious, supported part of the product where enabled, but not the dominant organizing principle for the entire game.

## Architecture and source of truth

- `USER_SOURCE`: on 2026-08-11 the owner accepted the explicit recommendation `PvP = secondary pillar` after the three product-level options were described.
- `PROVEN`: `GAME-VISION-01_PREDECISION_ANALYSIS.md` lists launch-level PvP importance as owner-decision packet item 6.
- `PROVEN`: the accepted player promise preserves recognizable Tibia depth/risk/world identity while keeping exact PvP rules unresolved.
- `PROVEN`: Reference-first and hybrid Reference tracking require any Reference PvP parity claim to remain tied to a named immutable target revision.
- `PROVEN`: Evolved reliability/UX-first means the first Evolved package is not a broad PvP/gameplay redesign.
- `DERIVED`: secondary pillar means PvP is important and production-quality where enabled, but product scope may remain bounded and Oteryn must remain coherent for PvE/progression/economy/social players without making PvP the universal center of the design.

## Acceptance criteria

- [x] Record PvP as the owner-accepted `secondary pillar` product priority.
- [x] State that secondary does not mean low correctness/safety/fairness priority.
- [x] State that PvP is not the sole organizing principle for progression, economy, content or social play.
- [x] Preserve Reference parity authority and explicit/versioned/measurable Evolved differences.
- [x] Keep exact PvP world types, skull/frag/war/death/damage/reward formulas unresolved.
- [x] Keep disconnect timers, channel-switch eligibility, anti-escape semantics, cooldowns and anti-hopping thresholds unresolved.
- [x] Preserve server authority, durable-value integrity, anti-duplication, anti-cheat and recovery safety without selecting `GAME-CHANNEL-01` PvP switching policy.
- [x] Keep first product-proof PvP breadth bounded and explicitly milestone-scoped rather than implicitly exhaustive.
- [x] Include mandatory decision-timing, concrete blocked work, cost-of-delay and supersession-evidence records.
- [x] Keep full `GAME-VISION-01` explicitly `NOT ACCEPTED`.
- [x] No runtime/client/server/content/production implementation is authorized.
- [ ] Exact-head self-review and repository-required CI pass before merge.

## Excluded scope

This task must not:

- implement PvP, combat, death, reconnect, channel, anti-cheat, persistence or content behavior;
- select launch world types or PvP taxonomy;
- define skulls, frags, wars, unjustified kills, PvP rewards or rankings;
- define damage scaling, vocation balance or combat formulas;
- define death penalties, blessings or protection-loss mechanics;
- define safe-zone/protection-zone/PvP-area policy;
- define disconnect grace/post-grace behavior or logout-abuse enforcement;
- define whether or under what conditions PvP/combat state blocks channel changes;
- define anti-escape semantics, channel-switch cooldowns, combat-lock durations or anti-hopping thresholds;
- define transfer rules between future PvP/non-PvP world types;
- alter accepted Reference-first, hybrid tracking, player-promise or Evolved reliability/UX-first decisions;
- modify parallel PR #162 repository-engineering/governance owned paths;
- authorize production rollout.

## Implementation / findings

- Current `main` at task start: `78c08e658cb4acb2f6e7298841b223d2ebf3cf5d`.
- Open PR at task start: #162 (`ci: enforce aggregate pull request merge gate`), currently a draft and owning repository-engineering/governance paths. No overlap exists with this task's two documentation paths.
- Repository search found no existing PvP secondary-pillar owner baseline.
- Added `docs/architecture/GAME-VISION-01_PVP_SECONDARY_PILLAR_OWNER_BASELINE.md` as a separate owner-accepted partial baseline.
- Opened PR #165 with the required `## Summary`, `## Scope` and `## Validation` metadata headings.
- The owner acceptance resolves only `GAME-VISION-01` owner-decision packet item 6; detailed PvP policy remains intentionally open.
- The baseline explicitly distinguishes PvP feature breadth from PvP-sensitive safety/integrity invariants: breadth may be bounded by milestone, but correctness and exploit resistance cannot be weakened merely because PvP is secondary.

### Review repair cycle 1 — keep channel/PvP policy in `GAME-CHANNEL-01`

Automatic review on older head `77c19a5a0845131f6ce2ea87a6a1fbb0541f8860` raised valid P1 `Keep the channel-escape rule in its owning gate`.

The initial architecture draft included a binding guardrail stating that channel switching must not provide an immediate combat/PvP escape path. Although related multichannel architecture already contains safety constraints, the final owner product policy for PvP/channel switching remains an unresolved `GAME-CHANNEL-01` decision and was not accepted by the owner in this turn.

Repair applied:

- removed the binding no-channel-escape requirement from this `GAME-VISION-01` owner baseline;
- explicitly stated that switching eligibility, combat/PvP interaction, anti-escape semantics and cooldowns remain owned by `GAME-CHANNEL-01` and related PvP/recovery gates;
- expanded the unresolved-decision list accordingly;
- retained only product-level PvP importance and generic already-owned safety/integrity expectations.

The owner-accepted meaning `PvP = secondary pillar` is unchanged.

## Validation

### Focused

- command/run: compare owner acceptance with `GAME-VISION-01_PREDECISION_ANALYSIS.md`, accepted Reference/player-promise/Evolved baselines and ADR-0010; verify no existing owner PvP baseline; inspect parallel PR #162 for path collision; inspect PR #165 metadata and authority boundaries after P1 repair
- result: **PASS** for declared product scope after repair; fresh exact-head self-review required after this repair commit

### Component/integration

- command/run: `NOT_APPLICABLE` — architecture/task documentation only; no executable component/integration behavior changes
- result: `NOT_APPLICABLE`

### E2E

- scenario: `NOT_APPLICABLE` — no runtime/client/server/content behavior changes
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: pending review-repair checkpoint commit
- trigger source: `pull_request/synchronize`
- workflow/run/job: pending fresh generation for repaired final candidate head
- runner assignment: pending
- classification: repository-required documentation checks applicable to exact PR head
- result: pending fresh generation

## Audit

- scope: owner-authority fidelity, PvP-priority overreach, accidental detailed PvP-rule freeze, `GAME-CHANNEL-01` authority leakage, Reference/Evolved conflict, safety/integrity weakening, milestone-scope ambiguity, task-template completeness, parallel ownership collision
- material findings: one valid P1 on unaccepted channel/PvP switching semantics; repaired in cycle 1; no owner-semantic conflict found
- unresolved findings: P1 thread pending repaired-head evidence reply/resolve; fresh exact-head CI pending
- verdict: pending final exact-head verification

## Self-review

- exact head: pending review-repair checkpoint commit
- method/reviewer: implementing/coordinating agent full-diff product/architecture/governance review
- material findings: prior PASS review `4904935503` invalidated by head movement; fresh self-review pending
- verdict: pending

## Independent review

- required: `NO` under current risk policy — bounded product-priority documentation only; this task does not change authentication/session, protocol/wire, persistence/economy conservation, production configuration or executable multichannel authority semantics
- exact head: `NOT_APPLICABLE`
- method/auditor: `NOT_APPLICABLE`
- material findings: `NOT_APPLICABLE`
- verdict: `NOT_APPLICABLE`

## PR and closeout

- changed-file review: pending repaired exact final head
- unresolved review threads: one valid P1 from older head pending repaired-head evidence reply/resolve
- related/superseded PRs: parallel disjoint draft PR #162; no supersession
- protected auto-merge: pending
- merge commit/result: pending
- ownership release: pending lifecycle archive after merge

## Context checkpoint

```yaml
last_progress: Repaired the valid authority-leakage P1 by removing unaccepted channel/PvP escape semantics from the PvP product-priority baseline; `PvP = secondary pillar` is unchanged.
status: validating
branch: docs/OTV2-20260811-pvp-secondary-owner-baseline
head_sha: null
pr: 165
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request/synchronize
ci_check_generation: repair-1-fresh-head-pending
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 1
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: Freeze the repaired exact head, perform fresh mandatory full-diff self-review, reply/resolve the P1 with exact-head evidence, verify fresh CI/current main/parallel #162 state, then squash-merge only if clean.
```
