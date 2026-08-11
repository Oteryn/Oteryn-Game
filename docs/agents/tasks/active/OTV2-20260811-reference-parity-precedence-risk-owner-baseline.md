# OTV2-20260811-reference-parity-precedence-risk-owner-baseline

```yaml
task_id: OTV2-20260811-reference-parity-precedence-risk-owner-baseline
title: Persist GAME-VISION-01 Reference parity precedence and progression-risk owner baseline
mode: COORDINATE
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/OTV2-20260811-reference-parity-precedence-risk-owner-baseline
pr: 169
base_sha: 5b141e009f929226a59c2f13cd5075f20a2aeea6
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT architecture coordinator
created_at: 2026-08-11T13:02:00+02:00
updated_at: 2026-08-11T13:10:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/agents/tasks/active/OTV2-20260811-reference-parity-precedence-risk-owner-baseline.md
  - docs/architecture/GAME-VISION-01_REFERENCE_PARITY_PRECEDENCE_OWNER_BASELINE.md
public_contracts:
  - docs/architecture/GAME-VISION-01_REFERENCE_PARITY_PRECEDENCE_OWNER_BASELINE.md
depends_on:
  - GAME-VISION-01_REFERENCE_FIRST_OWNER_BASELINE.md
  - GAME-VISION-01_REFERENCE_HYBRID_TRACKING_OWNER_BASELINE.md
  - GAME-VISION-01_PLAYER_PROMISE_OWNER_BASELINE.md
  - GAME-VISION-01_PVP_SECONDARY_PILLAR_OWNER_BASELINE.md
  - GAME-VISION-01_SOLO_VIABLE_PARTY_REWARDED_OWNER_BASELINE.md
  - GAME-VISION-01_EVOLVED_RELIABILITY_UX_FIRST_OWNER_BASELINE.md
  - GAME-VISION-01_PREDECISION_ANALYSIS.md
  - ADR-0010-reference-and-evolved-world-product-profiles.md
blocks:
  - GAME-VISION-01 owner decision packet item 8 closure
  - correct interpretation of PvP and solo/party owner baselines in Reference worlds
  - GAME-CHAR-01 progression/death/risk product assumptions without accidental Reference divergence
cross_repository_coordination_id: OTV2-GLOBAL-ARCHITECTURE
external_repositories: []
```

## Outcome

Persist the owner's explicit acceptance that **Reference gameplay mechanics follow the selected named Global Tibia parity baseline**, including solo/party, PvP, progression, death and risk semantics, while `GAME-VISION-01` defines the long-term Oteryn/Evolved product direction and shared cross-profile quality bar rather than silently overriding Reference.

The owner also accepted the progression/death/risk direction: Reference keeps the selected baseline rules; the first Evolved package does not automatically redesign them; any Oteryn-specific progression/death/risk redesign requires a later explicit, versioned and measurable Evolved gate.

## Architecture and source of truth

- `USER_SOURCE`: on 2026-08-11 the owner explicitly accepted the recommended progression/death/risk direction and clarified that solo/party and PvP should be reproduced from Global Tibia in Reference, asking whether Vision describes how the target Oteryn game should eventually look.
- `PROVEN`: Reference-first and hybrid tracking already require named immutable Reference revisions and explicit promotion of upstream changes.
- `PROVEN`: player-promise and shared architecture decisions impose reliability, server-authority and explicit-difference requirements across profiles.
- `PROVEN`: the existing PvP-secondary and solo-viable/party-rewarded baselines already state that actual Reference mechanics remain governed by the named parity baseline; therefore no historical baseline rewrite is required.
- `DERIVED`: the new canonical precedence baseline centralizes the owner's clarification and prevents future-facing gameplay preferences from being misread as Reference overrides.

## Acceptance criteria

- [x] Record that Reference gameplay mechanics follow the selected named Global Tibia baseline unless an explicit documented Reference difference is separately accepted.
- [x] Explicitly include solo/party, PvP, progression, death and risk in that precedence rule.
- [x] Record that `PvP = secondary pillar` is product-direction guidance and does not authorize changing Reference PvP semantics or importance away from the selected baseline.
- [x] Record that `solo viable, party rewarded` is product-direction guidance and does not authorize changing Reference party/shared-XP/content semantics away from the selected baseline.
- [x] Record the accepted progression/death/risk direction: Reference is the launch oracle; first Evolved does not automatically redesign it; later redesign requires an isolated explicit Evolved gate.
- [x] Distinguish long-term Evolved gameplay direction from cross-profile quality/authority rules that apply to both Reference and Evolved.
- [x] Keep exact Reference patch/date/revision selection unresolved.
- [x] Keep exact death/progression formulas, blessings, loss rates and Evolved redesign unresolved.
- [x] Keep full `GAME-VISION-01` explicitly `NOT ACCEPTED` unless remaining required decisions are separately closed/deferred.
- [x] No runtime/client/server/content/production implementation is authorized.
- [ ] Exact-head self-review and repository-required CI pass before merge.

## Excluded scope

This task must not:

- implement or change gameplay/runtime behavior;
- select the exact Global Tibia patch/date/revision used by Reference;
- change Reference mechanics away from the selected baseline;
- define XP/skill loss, blessings, death penalties, PvP formulas, shared-XP formulas, party bonuses or content requirements;
- redefine channel/PvP switching policy;
- freeze a concrete Evolved progression/death redesign;
- rewrite historical owner baselines where their existing Reference boundary is already correct;
- change public branding/marketing claims;
- modify parallel draft PR #162 repository-engineering/governance paths;
- authorize production rollout.

## Implementation / findings

- Main at task start: `5b141e009f929226a59c2f13cd5075f20a2aeea6`.
- Open PR at task start: draft #162 (`ci: enforce aggregate pull request merge gate`), owning disjoint repository-engineering/governance paths.
- Active tasks at task start: foundation preimplementation contracts plus two disconnect-analysis tasks; no path overlap with this bounded GAME-VISION delivery.
- Added `docs/architecture/GAME-VISION-01_REFERENCE_PARITY_PRECEDENCE_OWNER_BASELINE.md` as the canonical precedence/interpretation record.
- Re-read the existing PvP and solo/party owner baselines. Both already preserve named Reference parity for actual mechanics, so this task intentionally avoids unnecessary edits to historical accepted baselines and instead centralizes the new owner clarification in the dedicated precedence document.
- Opened PR #169 with the required `## Summary`, `## Scope` and `## Validation` sections; declared changed-file scope remains exactly the task record plus the new owner baseline.

## Validation

### Focused

- command/run: compare current owner clarification against Reference-first/hybrid/player-promise/PvP/solo-party baselines, `GAME-VISION-01_PREDECISION_ANALYSIS.md` item 8 and ADR-0010; verify no parallel ownership collision; inspect PR #169 metadata and changed-file scope
- result: **PASS** for declared owner scope before final-head freeze; exact-head review required after this checkpoint

### Component/integration

- command/run: `NOT_APPLICABLE` — architecture/task documentation only; no executable behavior changes
- result: `NOT_APPLICABLE`

### E2E

- scenario: `NOT_APPLICABLE` — no runtime/client/server/content behavior changes
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: pending this PR checkpoint commit
- trigger source: `pull_request/synchronize`
- workflow/run/job: pending fresh generation for final candidate head
- runner assignment: pending
- classification: repository-required documentation checks
- result: pending

## Audit

- scope: owner-source fidelity, Reference-vs-Evolved authority, accidental parity override, accidental detailed gameplay freeze, cross-profile quality interpretation, task-template completeness and parallel ownership collision
- material findings: no material finding in focused pre-freeze review; exact-head audit pending
- unresolved findings: pending exact-head PR review-thread verification
- verdict: pending

## Self-review

- exact head: pending this checkpoint commit
- method/reviewer: implementing/coordinating agent full-diff product/architecture/governance review
- material findings: pending
- verdict: pending

## Independent review

- required: `NO` under current risk policy — bounded product/architecture documentation; no executable security/protocol/persistence/production/multichannel-authority semantics are changed
- exact head: `NOT_APPLICABLE`
- method/auditor: `NOT_APPLICABLE`
- material findings: `NOT_APPLICABLE`
- verdict: `NOT_APPLICABLE`

## PR and closeout

- changed-file review: pending exact final head
- unresolved review threads: pending
- related/superseded PRs: parallel disjoint draft PR #162; no supersession
- protected auto-merge: pending
- merge commit/result: pending
- ownership release: pending lifecycle archive after merge

## Context checkpoint

```yaml
last_progress: Persisted the dedicated owner baseline and opened PR #169; existing PvP/solo-party baselines were verified to already preserve Reference parity and therefore were not rewritten.
status: validating
branch: docs/OTV2-20260811-reference-parity-precedence-risk-owner-baseline
head_sha: null
pr: 169
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request/synchronize
ci_check_generation: final-head-pending
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: Freeze this checkpointed exact head, run mandatory full-diff self-review, verify review threads/current main/parallel #162 state and repository-required CI, then squash-merge only if clean.
```
