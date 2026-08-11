# OTV2-20260811-evolved-reliability-ux-owner-baseline

```yaml
task_id: OTV2-20260811-evolved-reliability-ux-owner-baseline
title: Persist GAME-VISION-01 Evolved reliability/UX-first owner baseline
mode: COORDINATE
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/OTV2-20260811-evolved-reliability-ux-owner-baseline
pr: 163
base_sha: f184930fac66fdf9ae0cc7f606d3502c17626a79
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT architecture coordinator
created_at: 2026-08-11T10:59:00+02:00
updated_at: 2026-08-11T11:12:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/agents/tasks/active/OTV2-20260811-evolved-reliability-ux-owner-baseline.md
  - docs/architecture/GAME-VISION-01_EVOLVED_RELIABILITY_UX_FIRST_OWNER_BASELINE.md
public_contracts:
  - docs/architecture/GAME-VISION-01_EVOLVED_RELIABILITY_UX_FIRST_OWNER_BASELINE.md
depends_on:
  - GAME-VISION-01_PLAYER_PROMISE_OWNER_BASELINE.md
  - GAME-VISION-01_REFERENCE_FIRST_OWNER_BASELINE.md
  - GAME-VISION-01_REFERENCE_HYBRID_TRACKING_OWNER_BASELINE.md
  - GAME-VISION-01_PREDECISION_ANALYSIS.md
  - ADR-0010-reference-and-evolved-world-product-profiles.md
blocks:
  - GAME-VISION-01 owner decision packet item 5 closure
  - first Evolved product-proof / later alpha milestone scope and acceptance plan
  - first Evolved candidate backlog prioritization between reliability/UX and systemic gameplay gates
cross_repository_coordination_id: OTV2-GLOBAL-ARCHITECTURE
external_repositories: []
```

## Outcome

Persist the owner's explicit acceptance that the **first Evolved differentiation strategy is reliability/UX-first**: Oteryn should first improve player-visible trust, usability, recovery and clarity while avoiding an initial broad redesign of combat power, progression, death, PvP, boss rewards or economy.

## Architecture and source of truth

- `USER_SOURCE`: on 2026-08-11, after the recommendation was stated as “first Evolved difference = reliability/UX-first; better client, reconnect, messages, interface, convenience and technical quality before balance/death/economy/core progression”, the owner answered `tak`.
- `PROVEN`: `GAME-VISION-01_PLAYER_PROMISE_OWNER_BASELINE.md` requires modern reliable native quality and explicit/versioned/measurable intentional Oteryn differences.
- `PROVEN`: `GAME-VISION-01_REFERENCE_FIRST_OWNER_BASELINE.md` makes the first external evaluation Reference-first rather than Evolved-first.
- `PROVEN`: `GAME-VISION-01_REFERENCE_HYBRID_TRACKING_OWNER_BASELINE.md` preserves immutable released Reference revisions and explicit promotion.
- `PROVEN`: `GAME-VISION-01_PREDECISION_ANALYSIS.md` defines reliability/UX-first as Strategy 1, contrasts it with immediate systemic gameplay redesign and a broad feature pack, and lists the first Evolved strategy as owner-decision packet item 5 required for the minimum product contract.
- `PROVEN`: ADR-0010 keeps Reference and Evolved on one canonical engine/client/`protocol-oteryn` and requires Evolved differences to be explicit and reviewable.
- `DERIVED`: this decision selects the ordering/character of the first Evolved package, not its exact feature list. Semantically neutral reliability/UX improvements that preserve Reference gameplay should not be artificially withheld from Reference merely to manufacture Evolved differentiation.

## Acceptance criteria

- [x] Record reliability/UX-first as the owner-accepted first Evolved strategy.
- [x] State that the first Evolved package should minimize gameplay-power/economy disturbance while improving trust, usability, recovery and clarity.
- [x] Preserve server authority, persistence/value integrity, Reference parity and one shared engine/client/protocol foundation.
- [x] Make clear that shared semantic-neutral reliability/UX improvements may benefit Reference too and need not be Evolved-exclusive.
- [x] Keep exact feature inventory, UI design, numeric KPIs and rollout mechanics unresolved.
- [x] Keep death/progression/PvP/economy/boss-loot/systemic redesign out of the first package unless separately owner-accepted.
- [x] Keep full `GAME-VISION-01` explicitly `NOT ACCEPTED`.
- [x] Include decision-timing, concrete blocked downstream work, cost-of-delay and supersession-evidence records.
- [x] No runtime/client/server/content/production implementation is authorized.
- [ ] Exact-head self-review and repository-required CI pass before merge.

## Excluded scope

This task must not:

- implement any client, server, protocol, persistence, content, deployment or production change;
- freeze an exact Evolved feature list or UI mockup;
- select exact reconnect timers, grace periods, channel-switch mechanics or recovery algorithms;
- redesign or accept death penalty, progression rates, PvP rules, vocation balance, boss rewards, spawn economics or economy formulas;
- claim that every reliability/UX improvement is Evolved-only;
- alter the Reference-first or hybrid Reference tracking baselines;
- finalize public branding, monetization, KPI targets or LiveOps cadence;
- modify the parallel PR #162 merge-gate/governance owned paths.

## Implementation / findings

- Current main at task start: `f184930fac66fdf9ae0cc7f606d3502c17626a79`.
- Open PR at task start: #162 (`ci: enforce aggregate pull request merge gate`). Its changed paths are repository engineering/governance files plus `docs/agents/tasks/active/OTV2-20260811-merge-gate-hardening.md`; no overlap exists with this task's two owned paths.
- This task therefore proceeds as a disjoint documentation change while preserving #162 ownership.
- Added `docs/architecture/GAME-VISION-01_EVOLVED_RELIABILITY_UX_FIRST_OWNER_BASELINE.md` as a separate owner-accepted partial baseline.
- The owner acceptance resolves only owner-decision packet item 5 from `GAME-VISION-01_PREDECISION_ANALYSIS.md`.
- The baseline distinguishes shared semantic-neutral reliability/UX quality from true Evolved-specific product/ruleset differences, preventing an artificial “worse Reference” interpretation.
- Full `GAME-VISION-01` remains `NOT ACCEPTED`; no executable implementation or production rollout is authorized.

### CI recovery cycle 1 — PR metadata event payload

Initial exact-head Agent Governance run `31476089174` failed before checkout because the original PR body used `## Parallel coordination` instead of the mandatory literal `## Scope` heading. The workflow log proved the exact cause: `PR body is missing ## Scope`.

Recovery actions:

- corrected PR #163 metadata only; the body now contains mandatory `## Summary`, `## Scope` and `## Validation` headings;
- attempted one job rerun without moving the head;
- the rerun failed identically because GitHub reruns reuse the original `pull_request` event payload, so the job continued to see the old body despite live PR metadata being corrected;
- a task checkpoint then generated a fresh `synchronize` event with corrected metadata; fresh Agent Governance passed on that generation.

No architecture/product semantics changed during this recovery.

### Review repair cycle 1 — concrete blocked work

Automatic review on an older head raised a valid P1: the decision-timing section answered `YES` but only described work the decision broadly constrained, while the task metadata still had `blocks: []`.

Repair applied:

- the architecture baseline now explicitly names work that cannot safely close while the strategy is ambiguous: `GAME-VISION-01` owner-decision packet item 5, the scope/acceptance plan for the first Evolved product proof or later alpha milestone, and prioritization of the first Evolved candidate backlog;
- the task `blocks` metadata mirrors those concrete dependencies;
- later `DUR-04`/profile planning is classified as constrained rather than falsely claimed as fully blocked.

This repair strengthens decision-timing evidence only; the owner-accepted reliability/UX-first semantics are unchanged.

## Validation

### Focused

- command/run: full diff and authority-boundary inspection against `GAME-VISION-01_PREDECISION_ANALYSIS.md`, `GAME-VISION-01_PLAYER_PROMISE_OWNER_BASELINE.md`, accepted Reference baselines, ADR-0010 and PR #162 changed-path inventory
- result: **PASS** for declared product scope after repair; fresh exact-head review required after repair checkpoint

### Component/integration

- command/run: `NOT_APPLICABLE` — architecture/task documentation only; no executable component/integration behavior changes
- result: `NOT_APPLICABLE`

### E2E

- scenario: `NOT_APPLICABLE` — no runtime/client/server/content behavior changes
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: pending review-repair checkpoint
- trigger source: `pull_request/synchronize`
- workflow/run/job: pending fresh generation for repaired head
- runner assignment: pending
- classification: repository-required documentation checks applicable to exact PR head
- prior metadata failure: Agent Governance run `31476089174`, initial job `93730023249` and rerun job `93730563639` — both failed on stale original event body missing `## Scope`; not a document/content failure
- result: pending fresh generation

## Audit

- scope: owner-authority fidelity, decision-timing justification, accidental gameplay-policy freeze, accidental Reference degradation, shared-stack/profile-boundary conflict, task-template completeness, parallel ownership collision, PR-governance metadata/recovery correctness
- material findings: one PR-metadata governance failure repaired through CI recovery; one valid review P1 on concrete blocked work repaired in cycle 1; no owner-semantics conflict found
- unresolved findings: review P1 threads pending reply/resolve on repaired exact head; fresh CI pending
- verdict: pending final exact-head verification

## Self-review

- exact head: pending review-repair checkpoint
- method/reviewer: implementing/coordinating agent full-diff product/architecture/governance review
- material findings: earlier reviews invalidated by head movement; fresh review pending
- verdict: pending

## Independent review

- required: `NO` under current risk policy — bounded product-definition documentation only; no authentication/session, protocol/wire, durable-data/economy conservation, security, production or multichannel-authority semantics are changed
- exact head: `NOT_APPLICABLE`
- method/auditor: `NOT_APPLICABLE`
- material findings: `NOT_APPLICABLE`
- verdict: `NOT_APPLICABLE`

## PR and closeout

- changed-file review: pending repaired exact final head
- unresolved review threads: two automatic P1 threads from older heads; one substantive decision-timing repair and one already-outdated checkpoint finding; both pending exact-head evidence reply/resolve
- related/superseded PRs: parallel disjoint PR #162; no supersession
- protected auto-merge: pending
- merge commit/result: pending
- ownership release: pending lifecycle archive after merge

## Context checkpoint

```yaml
last_progress: Repaired the valid decision-timing P1 by naming the exact product/gate work blocked until reliability/UX-first is recorded; owner semantics remain unchanged.
status: validating
branch: docs/OTV2-20260811-evolved-reliability-ux-owner-baseline
head_sha: null
pr: 163
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
identical_failure_retries: 1
repair_cycles_for_current_gate: 1
ci_recovery_actions_for_current_head: 1
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: Freeze the repaired exact head, perform fresh mandatory full-diff self-review, reply/resolve both older P1 threads with exact-head evidence, verify fresh CI and current main/#162 state, then squash-merge only if clean.
```
