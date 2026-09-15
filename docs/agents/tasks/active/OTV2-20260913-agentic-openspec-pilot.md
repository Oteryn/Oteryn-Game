# OTV2-20260913-agentic-openspec-pilot

```yaml
task_id: OTV2-20260913-agentic-openspec-pilot
title: Pilot GitHub Agentic Workflows plus OpenSpec orchestration
mode: GOVERNANCE
status: blocked
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/otv2-agentic-openspec-pilot-01
issue: 591
pr: 592
base_sha: 253b5c0c464e9b73c8398bcf479bdcca9a1f0932
head_sha: external_pr_evidence
final_head_sha: external_pr_evidence
final_head_frozen_at: external_pr_evidence
owner: ChatGPT GPT-5.6 Sol
created_at: 2026-09-13
updated_at: 2026-09-14
execution_policy: continuous_progress
owned_paths:
  - .gitattributes
  - .github/workflows/agentic-pilot-qualification.yml
  - .github/workflows/oteryn-agentic-pilot-router.md
  - .github/workflows/oteryn-agentic-pilot-router.lock.yml
  - .github/workflows/oteryn-agentic-pilot-worker.md
  - .github/workflows/oteryn-agentic-pilot-worker.lock.yml
  - openspec/config.yaml
  - openspec/schemas/oteryn-agent-flow/**
  - openspec/changes/agentic-orchestration-pilot/**
  - docs/agents/tasks/active/OTV2-20260913-agentic-openspec-pilot.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Qualify a reversible canary proving that OpenSpec can carry Oteryn task contracts while GitHub Agentic Workflows routes a read-only worker through repository-native GitHub state without replacing existing Oteryn governance, CI, exact-head checks or Merge Queue authority. A runtime capability blocker is an accepted terminal pilot outcome when recorded exactly.

## Architecture and source of truth

- `PROVEN`: allocation baseline is protected `main@253b5c0c464e9b73c8398bcf479bdcca9a1f0932`.
- `PROVEN`: Issue #591 is the live pilot authority and PR #592 is the candidate.
- `PROVEN`: bound META policy is `OTERYN_ORGANIZATION_AGENT_POLICY@3.1.0` from immutable META commit `3b39e0be05aef008f1bd442821daefa898a201dd`.
- `PROVEN`: gh-aw is pinned to `v0.88.7` / `bde367913adeb3132f0a171594c88a17f4b7d08c` and OpenSpec to `1.13.0`.
- `PROVEN`: router and worker use explicit `copilot/gpt-5.6`; generated locks are compiler-owned.
- `PROVEN`: OpenSpec and agent output never grant merge, production, credential, protection or Merge Queue authority.
- `PROVEN`: live Router inference reaches the gh-aw Copilot provider but is rejected with HTTP 403 by centralized Copilot billing/entitlement despite `copilot-requests: write`.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: this pilot does not mutate production authority, recovery controllers, sessions or persisted production evidence. It changes only GitHub Actions/agent control-plane surfaces and remains bounded by existing repository governance.

## Acceptance criteria
- [x] Project-local `oteryn-agent-flow` OpenSpec schema validates with OpenSpec `1.13.0`.
- [x] Pilot OpenSpec change validates in strict non-interactive mode.
- [x] Router and worker gh-aw sources compile with pinned gh-aw `v0.88.7` on the exact candidate generation.
- [x] Router has read-only repository permissions apart from `copilot-requests: write`; its only intended live mutation is same-repository dispatch to the allowlisted pilot worker.
- [x] Worker has read-only repository permissions apart from inference and human-facing handoff is staged.
- [x] Representative behavior canary recorded the exact unavailable runtime capability: centralized Copilot provider/billing entitlement returns HTTP 403 before agent output or worker dispatch.
- [x] Existing `agent-governance`, `game-gate`, merge gates, Merge Queue and exact-head integration authority remain unchanged.
- [ ] Stable material successor receives one independent exact-head deep review after the accepted P1 repair; record the result in immutable PR evidence without moving the frozen head.

## Excluded scope

No WP3 runtime implementation, existing task migration, production mutation, secret creation, external-repository write, protection/ruleset change, required-check change, direct merge, Merge Queue bypass or autonomous integration authority.

## Implementation / findings

- Router dispatch remains limited to `oteryn-agentic-pilot-worker` on the pilot branch; the worker is read-only and staged-output only.
- The first runtime attempt exposed a gh-aw/Copilot model-resolution problem. Pinning `copilot/gpt-5.6` repaired that tooling contract and regenerated both locks with pinned gh-aw `v0.88.7`.
- The successor runtime reached the Copilot provider but received HTTP 403 twice. The gh-aw harness explicitly classified this as centralized Copilot billing/entitlement denial and stopped retries. No bypass token, alternate engine or authority weakening is permitted.
- Independent review on predecessor head `b92632f3fad2a2b72ad9acb660241a4a210f4614` found P1: implicit `missing-tool`, `missing-data`, `report-incomplete`, and agent-failure fallbacks could grant durable issue-writing authority outside staged handoff.
- P1 repair is material and accepted: both source workflows explicitly set `report-failure-as-issue: false` and `create-issue: false` for `missing-tool`, `missing-data`, and `report-incomplete`.
- Qualification now fails closed unless regenerated locks contain no `issues: write`, no fallback `...: "true"`, and the three explicit runtime fallback envs are `false`.
- Threat detection remains enabled; only its durable issue-writing authority is removed by the least-privilege conclusion permissions.

## Validation

### Focused
- command/run: pinned OpenSpec validation, pinned gh-aw compile/validate, source permission fences, generated-lock durable-write regression
- result: PASS on the repair generation before committed-lock synchronization; the only expected failure was stale committed locks

### Component/integration

- command/run: Router inference on the exact PR head with explicit `copilot/gpt-5.6`
- result: external capability unavailable — Copilot provider returned HTTP 403 from centralized billing/entitlement; no worker dispatch occurred

### E2E

- scenario: router -> allowlisted worker -> staged handoff
- result: stopped at the external inference boundary; static compilation is not claimed as behavior proof

### Exact-head CI
- final head: external PR/check evidence after generated-lock synchronization
- trigger source: pull_request
- workflow/run/job: exact successor generation required
- runner assignment: GitHub-hosted qualification plus normal repository merge-gate runners
- classification: control-plane pilot
- result: pending final successor generation at file freeze

## Self-review

- exact head: final successor generation via immutable PR evidence
- method/reviewer: implementing/coordinating agent
- material findings: reviewed fallback behavior; no integration authority added
- verdict: repair ready for final exact-head CI and re-review

## Independent review

- required: YES — material GitHub workflow change
- predecessor evidence: review of `b92632f3fad2a2b72ad9acb660241a4a210f4614` found one material fallback-write finding
- successor exact head: record externally after final generated-lock synchronization
- method/auditor: one independent deep review
- material findings: pending successor review at file freeze
- verdict: pending successor review at file freeze

## PR and closeout

- changed-file review: final exact-head review pending
- unresolved review threads: predecessor P1 is repaired in the successor generation; thread response pending final evidence
- protected auto-merge: NOT_AUTHORIZED
- merge commit/result: NOT_APPLICABLE without separate owner integration authorization
- ownership release: implementation frozen after final generated-lock synchronization; live runtime remains unavailable until centralized Copilot entitlement changes

## Context checkpoint

```yaml
last_progress: fallback-write finding repaired in source configuration and qualification fences
status: blocked
branch: agent/otv2-agentic-openspec-pilot-01
head_sha: external_pr_evidence
pr: 592
final_head_sha: external_pr_evidence
final_head_frozen_at: external_pr_evidence
ci_trigger_source: pull_request
ci_check_generation: final successor pending at file freeze
ci_checks_for_current_head: external_pr_evidence
ci_run_ids: external_pr_evidence
ci_job_ids: external_pr_evidence
runner_assignment_state: active
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 2
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: enable the organization Copilot runtime capability before a live behavior canary can complete
blocker: provider HTTP 403 at inference despite the declared workflow permission
next_action: synchronize generated locks, freeze the successor head, finish exact-head CI and independent review, then retain terminal blocked disposition without integration
```
