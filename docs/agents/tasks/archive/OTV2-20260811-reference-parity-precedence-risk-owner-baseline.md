# OTV2-20260811-reference-parity-precedence-risk-owner-baseline — archived

```yaml
task_id: OTV2-20260811-reference-parity-precedence-risk-owner-baseline
title: Persist GAME-VISION-01 Reference parity precedence and progression-risk owner baseline
mode: COORDINATE
status: completed
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/OTV2-20260811-reference-parity-precedence-risk-owner-baseline
pr: 169
base_sha: 5b141e009f929226a59c2f13cd5075f20a2aeea6
head_sha: c01f7478b70f47182c040efa8afb2dc7c54061f5
final_head_sha: c01f7478b70f47182c040efa8afb2dc7c54061f5
final_head_frozen_at: 2026-08-11T13:11:00+02:00
owner: ChatGPT architecture coordinator
created_at: 2026-08-11T13:02:00+02:00
updated_at: 2026-08-11T13:20:00+02:00
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
delivery_pr: 169
delivery_merge_sha: bf19f907003406bcab35c8dd5ba7f2fb611aa323
delivery_repair_cycles: 0
delivery_ci_recovery_actions: 0
lifecycle_closeout_pr: 170
implementation_status: NOT_APPLICABLE
```

## Outcome

Persisted the owner's explicit acceptance that **Reference gameplay mechanics follow the selected named Global Tibia parity baseline**, including solo/party, PvP, progression, death and risk semantics, while `GAME-VISION-01` defines long-term Oteryn/Evolved product direction and a shared cross-profile quality/authority bar rather than silently overriding Reference.

The owner also accepted the progression/death/risk direction: Reference keeps selected baseline rules; the first Evolved package does not automatically redesign them; any Oteryn-specific progression/death/risk redesign requires a later explicit, versioned and measurable Evolved gate.

The owner-accepted architecture delivery is complete. This archive preserves the full task record plus terminal delivery evidence. Lifecycle PR #170 is only the archival/ownership-release mechanism; it does not change the product decision and no further action belongs to this completed task.

## Architecture and source of truth

- `USER_SOURCE`: on 2026-08-11 the owner explicitly accepted the recommended progression/death/risk direction and clarified that solo/party and PvP should be reproduced from Global Tibia in Reference, asking whether Vision describes how the target Oteryn game should eventually look.
- `PROVEN`: Reference-first and hybrid tracking already require named immutable Reference revisions and explicit promotion of upstream changes.
- `PROVEN`: player-promise and shared architecture decisions impose reliability, server-authority and explicit-difference requirements across profiles.
- `PROVEN`: existing PvP-secondary and solo-viable/party-rewarded baselines already state that actual Reference mechanics remain governed by the named parity baseline; no historical baseline rewrite was needed.
- `PROVEN`: PR #169 final head `c01f7478b70f47182c040efa8afb2dc7c54061f5` passed exact-head self-review and all repository-required CI with zero review threads, then squash-merged as `bf19f907003406bcab35c8dd5ba7f2fb611aa323`.
- `DERIVED`: the dedicated precedence baseline centralizes the owner clarification and prevents future-facing gameplay preferences from being misread as Reference overrides.

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
- [x] Exact-head self-review and repository-required CI passed before merge.

## Excluded scope

This task did not:

- implement or change gameplay/runtime behavior;
- select the exact Global Tibia patch/date/revision used by Reference;
- change Reference mechanics away from the selected baseline;
- define XP/skill loss, blessings, death penalties, PvP formulas, shared-XP formulas, party bonuses or content requirements;
- redefine channel/PvP switching policy;
- freeze a concrete Evolved progression/death redesign;
- rewrite historical owner baselines where their existing Reference boundary was already correct;
- change public branding/marketing claims;
- modify parallel draft PR #162 repository-engineering/governance paths;
- authorize production rollout.

## Implementation / findings

- Main at task start was `5b141e009f929226a59c2f13cd5075f20a2aeea6`.
- Parallel PR #162 (`ci: enforce aggregate pull request merge gate`) remained a draft owning disjoint repository-engineering/governance paths; this task never modified those paths.
- Active tasks at task start were the foundation preimplementation contracts plus two disconnect-analysis tasks; no path overlap existed.
- Added `docs/architecture/GAME-VISION-01_REFERENCE_PARITY_PRECEDENCE_OWNER_BASELINE.md` as the canonical precedence/interpretation record.
- Re-read existing PvP and solo/party owner baselines; both already preserve named Reference parity for actual mechanics, so the task intentionally avoided unnecessary historical rewrites.

## Validation

### Focused

- command/run: compare owner clarification against Reference-first/hybrid/player-promise/PvP/solo-party baselines, `GAME-VISION-01_PREDECISION_ANALYSIS.md` item 8 and ADR-0010; verify no parallel ownership collision; inspect PR #169 metadata and changed-file scope
- result: **PASS** on final delivery head `c01f7478b70f47182c040efa8afb2dc7c54061f5`

### Component/integration

- command/run: `NOT_APPLICABLE` — architecture/task documentation only; no executable behavior changed
- result: `NOT_APPLICABLE`

### E2E

- scenario: `NOT_APPLICABLE` — no runtime/client/server/content behavior changed
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: `c01f7478b70f47182c040efa8afb2dc7c54061f5`
- trigger source: `pull_request/synchronize`
- workflow/run/job:
  - Agent Governance run `31485477875`: **success**
  - Dependency Review run `31485477883`: **success**
  - CodeQL run `31485477846`: **success**
- runner assignment: completed
- classification: repository-required documentation checks
- result: **PASS**

## Audit

- scope: owner-source fidelity, Reference-vs-Evolved authority, accidental parity override, accidental detailed gameplay freeze, cross-profile quality interpretation, task-template completeness and parallel ownership collision
- material findings: zero material findings on final delivery head
- unresolved findings: **0**; PR #169 had no review threads at delivery merge
- verdict: **PASS** on final delivery head

## Self-review

- exact head: `c01f7478b70f47182c040efa8afb2dc7c54061f5`
- method/reviewer: implementing/coordinating agent full-diff product/architecture/governance review
- review: `4905586233`
- material findings: `0`
- verdict: **PASS**

## Independent review

- required: `NO` under current risk policy — bounded product/architecture documentation; no executable security/protocol/persistence/production/multichannel-authority semantics changed
- exact head: `NOT_APPLICABLE`
- method/auditor: `NOT_APPLICABLE`
- material findings: `NOT_APPLICABLE`
- verdict: `NOT_APPLICABLE`

## PR and closeout

- changed-file review: **PASS** — delivery PR #169 changed only the task record and `GAME-VISION-01_REFERENCE_PARITY_PRECEDENCE_OWNER_BASELINE.md`
- unresolved review threads: **0** at delivery merge
- related/superseded PRs: parallel disjoint draft PR #162; no supersession
- protected auto-merge: `NOT_USED`; owner-authorized squash merge executed only after exact-head gates passed
- merge commit/result: PR #169 squash-merged as `bf19f907003406bcab35c8dd5ba7f2fb611aa323`
- ownership release: lifecycle PR #170 is the terminal archival/ownership-release mechanism; no remaining action belongs to this completed task

## Context checkpoint

```yaml
last_progress: Owner-accepted Reference parity-precedence and progression/risk direction delivered by PR #169 with all delivery gates PASS; lifecycle PR #170 is the terminal archival/ownership-release mechanism for this completed task.
status: completed
branch: docs/OTV2-20260811-reference-parity-precedence-risk-owner-baseline
head_sha: c01f7478b70f47182c040efa8afb2dc7c54061f5
pr: 169
final_head_sha: c01f7478b70f47182c040efa8afb2dc7c54061f5
final_head_frozen_at: 2026-08-11T13:11:00+02:00
ci_trigger_source: pull_request/synchronize
ci_check_generation: terminal-delivery
ci_checks_for_current_head: 3
ci_run_ids:
  - 31485477875
  - 31485477883
  - 31485477846
ci_job_ids: []
runner_assignment_state: completed
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 3
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: NONE
```
