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
base_sha: external_pr_evidence
head_sha: external_pr_evidence
final_head_sha: external_pr_evidence
final_head_frozen_at: external_pr_evidence
owner: oteryn-governance-controller
created_at: 2026-08-28T11:12:01Z
updated_at: 2026-08-30T00:40:00+02:00
execution_budget_minutes: 120
large_budget_reason: full reusable-prompt governance sweep plus deterministic exact-head qualification
owned_paths:
  - .github/workflows/agent-governance.yml
  - AGENTS.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/agents/PROMPTING_STANDARD.md
  - docs/agents/PROMPT_EVAL_STANDARD.md
  - docs/agents/prompts/**
  - docs/agents/tasks/active/OTV2-20260828-remote-desktop-per-action-gate.md
  - docs/superpowers/plans/2026-08-28-game-remote-desktop-per-action-adoption.md
  - tools/agents/test_validate_remote_desktop_prompt_routing.py
  - tools/agents/test_validate_remote_desktop_prompt_routing_codex_regressions.py
  - tools/agents/test_validate_remote_desktop_prompt_routing_codex_round12.py
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

Any later branch-only repair helper is also temporary evidence only. In particular, `.github/workflows/rdc-final-p2-green-retry.yml` was still present in the PR changed-file set at the 2026-08-28 continuation handoff and MUST be deleted before a final candidate is frozen.

## Validation

RED: exact head `7fc92624838718594283761632496ab2afc4e3b4`, Agent governance run `33166551928`, job `98833233706`: existing governance PASS; focused Remote Desktop routing step FAIL as intended against unaligned provider state.

Sweep: GitHub-hosted run `33168046139`, job `98838083652`: exactly 43 lifecycle-derived reusable prompts updated; existing governance PASS; focused routing validator PASS; bounded changed-path set PASS; `git diff --check` PASS.

Focused final proof: `python tools/agents/test_validate_remote_desktop_prompt_routing.py`, `python tools/agents/validate_remote_desktop_prompt_routing.py` and existing `python tools/agents/validate_governance.py` in `Agent governance / validate` on immutable PR exact-head evidence. Focused regressions include hyphenated `Remote-Desktop` authority, visible text after multiline HTML-comment closure, Markdown soft-line-break `ping` capability-discovery wording, inline-comment/fence handling, rendered Markdown entities/emphasis/links, preservation of literal `Remote_Desktop_Commander.*` identifiers, generic direct-connector authorization/exemption wording, and zero-width obfuscation inside connector/tool identifiers.

Runtime/component/E2E: `NOT_APPLICABLE` — governance/prompt-only change.

A commit cannot contain its own SHA. Final exact head, review/check evidence and merge evidence therefore remain in immutable GitHub PR/check/review state instead of causing a self-referential follow-up commit.

## Current continuation checkpoint

This checkpoint is intentionally SHA-neutral: exact head/base/check/review coordinates are immutable GitHub evidence and MUST be refreshed from live GitHub immediately before any mutation or merge decision. A commit cannot truthfully embed its own final SHA.

Current durable repository state represented by this task packet:

- the canonical META provider binding remains `Oteryn/Oteryn@e002fc7532188e73a0f495da3e20710541ed50e0`;
- all **44** lifecycle-reusable Game prompts and the four canonical governance surfaces are covered by the retained provider validator;
- `tools/agents/test_validate_remote_desktop_prompt_routing.py` plus the durable `tools/agents/test_validate_remote_desktop_prompt_routing_codex_regressions.py` retain tests-first coverage for every repaired independent-review bypass class;
- no branch-only repair workflow, patch script or qualification marker is part of the intended final changed-file set;
- Remote Desktop/Desktop Commander was not used for this task and remains default-deny/exception-only;
- the task remains `validating` until one exact final head passes all four required Game workflows, receives one fresh independent clean Codex review, is guarded-squash-merged, and protected `main` readback succeeds.

### Required terminal continuation order

1. Refresh protected `main`, Issue #237, PR #239, exact PR head, changed filenames, unresolved review threads and exact-head workflow evidence from live GitHub.
2. Confirm the durable changed-file set contains no temporary helper/workflow/marker and the PR is `behind_by=0` against protected `main`; if main advanced, reconcile only by a normal merge-up, never reset/rebase/force.
3. Require `Agent governance`, `Architecture semantic audit`, `Merge authority audit` and the full `Merge gate` to PASS on the same exact final SHA. Agent governance must prove the complete base and durable Remote Desktop regression suites plus 44 reusable prompts + 4 canonical surfaces and repository policy.
4. Request exactly one fresh independent Codex review for that frozen exact SHA. Any material P0/P1/P2 reopens strict tests-first RED→GREEN repair and invalidates prior qualification.
5. With zero unresolved blocking threads, refresh head/main once more and squash-merge PR #239 using `expected_head_sha` equal to the reviewed frozen SHA; do not bypass protections.
6. Read back protected `main`, verify the returned squash SHA, canonical META binding, validator, representative reusable prompts and retained Agent-governance binding; then close Issue #237 as completed if its acceptance criteria are satisfied.

## Autonomous continuation prompt

> Continue autonomously and bring `Oteryn/Oteryn-Game#239` to a safe squash merge. GitHub live state is the sole source of truth. Do not use Remote Desktop/Desktop Commander. Preserve strict tests-first repair for any new P0/P1/P2. Never rebase/force. Freeze one exact final head, require all four Game workflows plus one fresh independent Codex review on that same SHA, then guarded squash merge with `expected_head_sha`, protected-main readback, and Issue #237 closeout. Never treat this checkpoint's prose as a substitute for live GitHub evidence.

## Context checkpoint

```yaml
last_progress: canonical provider binding, 44-prompt sweep, durable validator hardening and accumulated Codex regression coverage are implemented; live exact-head qualification remains authoritative
status: validating
branch: governance/remote-desktop-per-action-gate-237
head_sha: external_pr_evidence
pr: 239
final_head_sha: external_pr_evidence
final_head_frozen_at: external_pr_evidence
ci_trigger_source: pull_request
ci_checks_for_current_head: external_pr_evidence
ci_run_ids: external_pr_evidence
ci_job_ids: external_pr_evidence
runner_assignment_state: github_hosted
remote_desktop_used_for_this_task: false
owner_action_required: null
blocker: refresh live GitHub; require one exact final SHA with four required workflow PASS results, zero unresolved blocking threads, and one fresh independent clean Codex review
next_action: refresh live PR/main and durable changed-file scope; qualify one immutable exact head; request one fresh Codex review; if clean, guarded squash merge with expected-head guard; protected-main readback; close Issue #237
```
