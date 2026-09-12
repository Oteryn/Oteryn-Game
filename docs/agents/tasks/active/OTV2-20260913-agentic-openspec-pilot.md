# OTV2-20260913-agentic-openspec-pilot

```yaml
task_id: OTV2-20260913-agentic-openspec-pilot
title: Pilot GitHub Agentic Workflows plus OpenSpec orchestration
mode: GOVERNANCE
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/otv2-agentic-openspec-pilot-01
issue: 591
pr: 592
base_sha: 253b5c0c464e9b73c8398bcf479bdcca9a1f0932
head_sha: external_pr_evidence
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT GPT-5.6 Sol
created_at: 2026-09-13
updated_at: 2026-09-13
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

Qualify a reversible canary proving that OpenSpec can carry Oteryn task contracts while GitHub Agentic Workflows routes a read-only worker through repository-native GitHub state without replacing existing Oteryn governance, CI, exact-head checks or Merge Queue authority.

## Architecture and source of truth

- `PROVEN`: allocation baseline is protected `main@253b5c0c464e9b73c8398bcf479bdcca9a1f0932`.
- `PROVEN`: Issue #591 is the live pilot work authority and PR #592 is the pilot candidate.
- `PROVEN`: root `AGENTS.md` makes live GitHub Issue, PR and check state lifecycle authority and prefers repository-native GitHub/CI execution.
- `PROVEN`: `.github/workflows/agent-governance.yml` already verifies a live PR head SHA and checks out that exact target before governance validation.
- `PROVEN`: bound META policy is `OTERYN_ORGANIZATION_AGENT_POLICY@3.1.0` from immutable META commit `3b39e0be05aef008f1bd442821daefa898a201dd`.
- `PROVEN`: gh-aw is pinned for the pilot to release `v0.88.7` / commit `bde367913adeb3132f0a171594c88a17f4b7d08c`.
- `PROVEN`: OpenSpec is pinned for the pilot to `@fission-ai/openspec@1.13.0`.
- `DERIVED`: OpenSpec is contract/planning state only; it cannot grant repository, merge, production or cross-repository authority.
- `UNKNOWN`: behavior-level benefit versus the current manual agent flow until the staged canary actually runs.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: this pilot does not mutate production authority, PREPARE/COMMIT state, recovery controllers, sessions or persisted production evidence. It does change a GitHub Actions/agent control-plane surface, so control-plane review rules still apply separately.

## Acceptance criteria

- [ ] Project-local `oteryn-agent-flow` OpenSpec schema validates with OpenSpec `1.13.0`.
- [ ] Pilot OpenSpec change validates in strict non-interactive mode.
- [ ] Router and worker gh-aw sources compile with pinned gh-aw `v0.88.7` on the exact PR head.
- [ ] Router has read-only repository permissions apart from the special `copilot-requests: write` inference permission; its only live safe-output mutation is a same-repository dispatch to the allowlisted pilot worker.
- [ ] Worker has read-only repository permissions apart from inference and all human-facing write output remains `staged: true`.
- [ ] One representative behavior canary records actual router/worker delivery or records the exact unavailable capability; static compilation alone is not called behavior proof.
- [ ] Existing `agent-governance`, `game-gate`, merge gates, Merge Queue and exact-head integration authority are unchanged.
- [ ] Stable material candidate receives one independent deep review before any integration decision.

## Excluded scope

No WP3 runtime implementation, existing task migration, production mutation, secret creation, external-repository write, protection/ruleset change, required-check change, direct merge, Merge Queue bypass or autonomous integration authority.

## Implementation / findings

- Pilot architecture uses a router plus one read-only worker instead of embedding Oteryn lifecycle authority in gh-aw.
- Router dispatch is intentionally limited to `oteryn-agentic-pilot-worker` on the pilot branch. The worker can inspect live evidence and preview a structured handoff, but cannot modify tracked files or GitHub lifecycle state.
- OpenSpec custom schema captures `authority -> evidence -> work-package -> review -> acceptance`; those artifacts remain planning evidence, never live lifecycle authority.
- Candidate-controlled workflow output is advisory. Deterministic GitHub checks and the bound Oteryn policy remain authoritative.

## Validation

### Focused

- command/run: pinned OpenSpec schema/change validation plus pinned gh-aw compilation in `agentic-pilot-qualification`
- result: bootstrap generation proved both validators and both gh-aw sources can pass; final exact-head result remains pending after generated locks are committed

### Component/integration

- command/run: router -> real allowlisted worker dispatch -> staged worker handoff preview
- result: pending behavior canary

### E2E

- scenario: exact-head pilot PR canary with no durable worker write
- result: pending

### Exact-head CI

- final head: external PR evidence after material freeze
- trigger source: pull_request
- workflow/run/job: pending final generation
- runner assignment: GitHub-hosted qualification; repository aggregate runners resolved by normal control plane
- classification: control-plane pilot
- result: pending final generation

## Self-review

- exact head: pending stable material candidate
- method/reviewer: implementing/coordinating agent
- material findings: pending
- verdict: pending

## Independent review

- required: YES — material GitHub Actions / agent control-plane change under bound AI review policy
- exact head: pending stable material candidate
- method/auditor: one independent Codex deep review on stable material candidate
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: in progress on PR #592
- unresolved review threads: pending
- related/superseded PRs: none
- protected auto-merge: NOT_AUTHORIZED
- merge commit/result: NOT_APPLICABLE until owner authorization + required qualification
- ownership release: pending

## Context checkpoint

```yaml
last_progress: OpenSpec validation and gh-aw source compilation passed on bootstrap generation; existing governance exposed and required explicit issue/pr locator
status: implementing
branch: agent/otv2-agentic-openspec-pilot-01
head_sha: external_pr_evidence
pr: 592
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request
ci_check_generation: bootstrap
ci_checks_for_current_head: pending
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: active
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 1
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: commit pinned compiler-generated gh-aw locks, then requalify the exact successor head and run the router-to-worker canary
```
