# OTV2-20260828-remote-desktop-per-action-gate

> Historical evidence only. No active allocation or dispatch authority.

```yaml
task_id: OTV2-20260828-remote-desktop-per-action-gate
title: Adopt canonical META Remote Desktop per-action gate
mode: GOVERNANCE
status: archived
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
updated_at: 2026-08-31T23:36:00+02:00
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

## Supersession

Issue #367 replaces the provider-copied policy and validators with the central META v3 binding and consumer. Current policy comes from that binding and the Game bootstrap. This archived record preserves #237/#239 evidence without an executable continuation prompt or next action.
