# OTV2-20260828-remote-desktop-per-action-gate

```yaml
task_id: OTV2-20260828-remote-desktop-per-action-gate
title: Adopt canonical META Remote Desktop per-action gate
mode: GOVERNANCE
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: governance/remote-desktop-per-action-gate-237
issue: 237
pr: 239
base_sha: 7c2da078596a7d2e27c3066ff74ac69b8b7f9af6
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: oteryn-governance-controller
created_at: 2026-08-28T11:12:01Z
updated_at: 2026-08-28T11:23:00Z
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

- [ ] Root/Game GitHub-only instructions bind to META `e002fc7532188e73a0f495da3e20710541ed50e0` and require positive exact per-action authorization before every direct `Remote_Desktop_Commander.*` call.
- [ ] All reusable prompt bodies contain the canonical Remote Desktop execution-routing section; no direct connector call is ordinary capability discovery.
- [ ] Deterministic governance validation discovers reusable prompts from `PROMPT_LIFECYCLE.json` and fails closed when the section/markers are missing or contradictory.
- [ ] Remote Desktop remains unavailable as routine repository-test, Git-inspection or CI/log-polling fallback; DENY does not become a generic blocker.
- [ ] Exact-head Game governance/merge checks and required independent review pass before squash merge.

## Excluded scope

No Game runtime, Cargo/workspace, protocol/schema/resource registry, deployment, production/protected environment, secrets, runner-host configuration, external-repository write or live Remote Desktop invocation. No claim of connector/router physical enforcement.

The temporary `.github/workflows/rdc-prompt-sweep-once.yml` is authorized only as a branch-scoped GitHub-hosted migration helper to append the already-approved identical section to the lifecycle-derived reusable prompt set. It must fail closed on an unexpected prompt count/path/state, may commit only `docs/agents/prompts/*.md`, and must be deleted before the final candidate. Final proof comes from the retained `agent-governance.yml` on the exact removal head.

## Validation

Focused: `python tools/agents/validate_remote_desktop_prompt_routing.py` plus existing `python tools/agents/validate_governance.py` in `Agent governance / validate`.

Runtime/component/E2E: `NOT_APPLICABLE` — governance/prompt-only change.

Final exact head, review fingerprint, check/review evidence and merge evidence remain in immutable GitHub PR/check/review state rather than self-referential tracked metadata.

## Context checkpoint

```yaml
last_progress: RED proven; canonical Game instruction surfaces aligned; bounded branch-only prompt sweep helper authorized
status: implementing
branch: governance/remote-desktop-per-action-gate-237
head_sha: null
pr: 239
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request
ci_check_generation: null
ci_checks_for_current_head: 0
ci_run_ids: [33166551928]
ci_job_ids: [98833233706]
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
next_action: run bounded lifecycle-derived reusable-prompt sweep and remove the temporary helper
```
