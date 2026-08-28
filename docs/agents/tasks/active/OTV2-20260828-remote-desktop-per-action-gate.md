# OTV2-20260828-remote-desktop-per-action-gate

```yaml
task_id: OTV2-20260828-remote-desktop-per-action-gate
title: Adopt canonical META Remote Desktop per-action gate
mode: GOVERNANCE
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: governance/remote-desktop-per-action-gate-237
issue: 237
pr: 239
base_sha: 7c2da078596a7d2e27c3066ff74ac69b8b7f9af6
head_sha: external_pr_evidence
final_head_sha: external_pr_evidence
final_head_frozen_at: external_pr_evidence
owner: oteryn-governance-controller
created_at: 2026-08-28T11:12:01Z
updated_at: 2026-08-28T11:46:00Z
execution_budget_minutes: 120
large_budget_reason: full reusable-prompt governance sweep plus deterministic exact-head qualification
owned_paths:
  - .github/workflows/agent-governance.yml
  - .github/workflows/rdc-prompt-sweep-once.yml
  - AGENTS.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/agents/PROMPTING_STANDARD.md
  - docs/agents/PROMPT_EVAL_STANDARD.md
  - docs/agents/prompts/**
  - docs/agents/tasks/active/OTV2-20260828-remote-desktop-per-action-gate.md
  - docs/superpowers/plans/2026-08-28-game-remote-desktop-per-action-adoption.md
  - tools/agents/validate_remote_desktop_prompt_routing.py
public_contracts:
  - Oteryn/Oteryn@e002fc7532188e73a0f495da3e20710541ed50e0:ecosystem/agent-execution-routing-policy.json
depends_on:
  - Oteryn/Oteryn PR #93 merged as e002fc7532188e73a0f495da3e20710541ed50e0
blocks: []
cross_repository_coordination_id: Oteryn/Oteryn#85
external_repositories:
  - Oteryn/Oteryn
```

## Outcome

Game adopts the exact merged META per-action Remote Desktop gate by reference. Every reusable prompt remains self-contained about the direct-call boundary, and a focused provider validator wired into the existing `Agent governance / validate` job prevents prompt regressions.

## Acceptance criteria

- [x] Root/Game GitHub-only instructions bind to META `e002fc7532188e73a0f495da3e20710541ed50e0` and require positive exact per-action authorization before every direct `Remote_Desktop_Commander.*` call.
- [x] All reusable prompt bodies contain the canonical Remote Desktop execution-routing section; no direct connector call is ordinary capability discovery.
- [x] Deterministic governance validation discovers reusable prompts from `PROMPT_LIFECYCLE.json` and fails closed when the section/markers are missing or contradictory.
- [x] Remote Desktop remains unavailable as routine repository-test, Git-inspection or CI/log-polling fallback; DENY does not become a generic blocker.
- [ ] Exact-head Game governance/merge checks and required independent review pass before squash merge.

## Excluded scope

No Game runtime, Cargo/workspace, protocol/schema/resource registry, deployment, production/protected environment, secrets, runner-host configuration, external-repository write or live Remote Desktop invocation. No claim of connector/router physical enforcement.

The temporary `.github/workflows/rdc-prompt-sweep-once.yml` was authorized only as a branch-scoped GitHub-hosted migration helper to append the already-approved identical section to the lifecycle-derived reusable prompt set. It failed closed on prompt count/path/state, committed only `docs/agents/prompts/*.md`, validated exactly 43 reusable prompts and was deleted before the final candidate. Final proof comes from the retained `agent-governance.yml` on the exact final head.

## Validation

RED: exact head `7fc92624838718594283761632496ab2afc4e3b4`, Agent governance run `33166551928`, job `98833233706`: existing governance PASS; focused Remote Desktop routing step FAIL as intended against unaligned provider state.

Sweep: GitHub-hosted run `33168046139`, job `98838083652`: exactly 43 lifecycle-derived reusable prompts updated; existing governance PASS; focused routing validator PASS; bounded changed-path set PASS; `git diff --check` PASS.

Focused final proof: `python tools/agents/validate_remote_desktop_prompt_routing.py` plus existing `python tools/agents/validate_governance.py` in `Agent governance / validate` on immutable PR exact-head evidence.

Runtime/component/E2E: `NOT_APPLICABLE` — governance/prompt-only change.

A commit cannot contain its own SHA. Final exact head, review/check evidence and merge evidence therefore remain in immutable GitHub PR/check/review state instead of causing a self-referential follow-up commit.

## Context checkpoint

```yaml
last_progress: RED proven, 43-prompt bounded sweep completed, one-shot helper removed, canonical provider surfaces aligned and whole changed-file scope reviewed
status: validating
branch: governance/remote-desktop-per-action-gate-237
head_sha: external_pr_evidence
pr: 239
final_head_sha: external_pr_evidence
final_head_frozen_at: external_pr_evidence
ci_trigger_source: pull_request
ci_check_generation: final_candidate_pending
ci_checks_for_current_head: external_pr_evidence
ci_run_ids: external_pr_evidence
ci_job_ids: external_pr_evidence
runner_assignment_state: github_hosted
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: qualify the immutable final PR head through exact-head Game gates and fresh independent Codex review, then squash merge and read back protected main
```
