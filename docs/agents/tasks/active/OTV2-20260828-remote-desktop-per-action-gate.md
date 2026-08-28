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
updated_at: 2026-08-28T18:55:00+02:00
execution_budget_minutes: 120
large_budget_reason: full reusable-prompt governance sweep plus deterministic exact-head qualification
owned_paths:
  - .github/workflows/agent-governance.yml
  - .github/workflows/rdc-prompt-sweep-once.yml
  - .github/workflows/rdc-final-p2-green-retry.yml
  - AGENTS.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/agents/PROMPTING_STANDARD.md
  - docs/agents/PROMPT_EVAL_STANDARD.md
  - docs/agents/prompts/**
  - docs/agents/tasks/active/OTV2-20260828-remote-desktop-per-action-gate.md
  - docs/superpowers/plans/2026-08-28-game-remote-desktop-per-action-adoption.md
  - tools/agents/test_validate_remote_desktop_prompt_routing.py
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

## Continuation handoff — 2026-08-28

This section is a locator/checkpoint, not authority. The next agent MUST refresh all GitHub facts before mutation.

Observed immediately before this handoff commit:

```yaml
protected_main_sha: 0fa962b4e4f688331fea899ae496dbfdb914583d
pr: 239
pr_state: open
pr_mergeable: true
handoff_source_head_sha: 6962698fa328a5bf12c59709236cce61105948c7
handoff_source_changed_files: 53
handoff_source_agent_governance:
  run_id: 33182421714
  conclusion: failure
handoff_source_architecture_semantic_audit:
  run_id: 33182421698
  conclusion: success
handoff_source_merge_authority_audit:
  run_id: 33182421702
  conclusion: success
handoff_source_merge_gate:
  run_id: 33182421778
  conclusion: success
temporary_helper_still_in_diff: .github/workflows/rdc-final-p2-green-retry.yml
remote_desktop_used_for_this_task: false
```

The source head is intentionally RED for the focused governance path and is NOT merge-qualified. Three current unresolved independent-review P2 threads remain:

1. **Raw HTML block inertness** — `<pre>`, `<script>` or equivalent raw HTML blocks can contain the exact routing heading/section while remaining non-operative Markdown; heading discovery must fail closed rather than accept an inert section.
2. **Generic direct connector/tool authorization synonyms** — wording such as `Direct connector operations ...` or `Direct tool requests ...` can evade rules that only recognize `calls`/`invocations`; equivalent authorization/exemption nouns must be rejected outside the canonical routing authority.
3. **Default-ignorable Unicode obfuscation** — visually hidden characters beyond the earlier narrow zero-width set, including U+2063 INVISIBLE SEPARATOR, can split protected identifiers; normalization must remove the relevant default-ignorable characters without breaking canonical identifier matching.

Do not resolve those review threads until a new exact head contains regression tests and a passing fix. The source-head success of architecture/merge gates does not qualify any later head.

### Required continuation order

1. Refresh protected `main`, Issue #237, PR #239, exact PR head, changed-file set, unresolved review threads and exact-head workflow evidence from live GitHub.
2. Re-read root `AGENTS.md`, this task packet, the implementation plan and canonical META policy pinned above. Do not use Remote Desktop/Desktop Commander; no host-only exception exists for this repository-governance work.
3. Preserve TDD. Add/confirm focused RED regressions for all three unresolved P2 classes on both reusable prompts and applicable canonical surfaces before changing validator behavior.
4. Implement the smallest fail-closed parser/normalization changes in `tools/agents/validate_remote_desktop_prompt_routing.py`. Do not weaken existing regressions or broaden Remote Desktop authorization.
5. Prove GREEN with the focused suite, provider routing validator, existing governance validator and repository policy checks through the normal exact-head `Agent governance` workflow.
6. Delete `.github/workflows/rdc-final-p2-green-retry.yml` and any other branch-only repair helper before final freeze. Final changed-file scope must contain only durable product/governance artifacts.
7. If protected `main` advanced, classify the upstream delta. Preserve work; use a normal non-force merge-up when reconciliation is safe. Never reset/rebase/force-push merely because `main` moved.
8. Freeze one exact candidate head. Require fresh exact-head `Agent governance`, `Architecture semantic audit`, `Merge authority audit` and full `Merge gate` success for that same SHA.
9. Request exactly one fresh independent Codex review for the frozen exact head under `docs/agents/CODEX_REVIEW_POLICY.json`. Any material P0/P1/P2 reopens TDD and invalidates prior qualification. Require zero unresolved blocking review threads.
10. Re-read PR head and protected `main` immediately before integration. Squash-merge PR #239 with `expected_head_sha` equal to the frozen reviewed head; do not bypass protections.
11. Read back protected `main`, confirm the squash merge SHA is canonical, verify the Remote Desktop routing binding/validator and a representative reusable prompt on merged `main`, and report exact immutable evidence.

## Autonomous continuation prompt

Use the following prompt verbatim or as the authoritative minimum for the successor agent:

> Continue autonomously and bring `Oteryn/Oteryn-Game#239` to a safe squash merge. GitHub live state is the sole source of truth: do not trust SHAs, cached worktrees, old handoffs or prior review summaries without refreshing them. Start by resolving protected `main`, Issue #237, PR #239, the exact PR head, changed filenames, unresolved review threads, task `docs/agents/tasks/active/OTV2-20260828-remote-desktop-per-action-gate.md`, implementation plan `docs/superpowers/plans/2026-08-28-game-remote-desktop-per-action-adoption.md`, root `AGENTS.md`, and the pinned META routing policy. Do not use Remote Desktop/Desktop Commander; this task has no valid host-only exception and GitHub/GitHub Actions are sufficient.
>
> The last handoff locator observed PR head `6962698fa328a5bf12c59709236cce61105948c7` against `main@0fa962b4e4f688331fea899ae496dbfdb914583d`, but both are locators only. At that source head, Architecture semantic audit run `33182421698`, Merge authority audit `33182421702` and Merge gate `33182421778` were success, while Agent governance run `33182421714` was RED. Three unresolved Codex P2 threads remained: raw `<pre>/<script>` HTML blocks can hide an inert canonical routing section; generic `direct connector/tool operations/requests/...` authorization synonyms can evade the outside-authority detector; and default-ignorable Unicode such as U+2063 can split protected identifiers. Also, temporary `.github/workflows/rdc-final-p2-green-retry.yml` still appeared in the PR diff and must not survive the final candidate.
>
> Use strict TDD. First add or confirm focused failing regressions for all three P2 classes on reusable prompts and applicable canonical surfaces. Then implement only the minimal fail-closed parser/normalization changes in `tools/agents/validate_remote_desktop_prompt_routing.py`; never weaken existing tests or broaden Remote Desktop authority. Prove GREEN through the focused suite, provider validator, existing governance and repository-policy validation. Remove all temporary repair workflows before final freeze.
>
> If `main` advanced, classify the delta and merge-up normally if safe; do not restart, reset, rebase or force-push just because upstream moved. After every material head change, discard superseded CI/review evidence. Freeze one exact final head, require fresh exact-head Agent governance, Architecture semantic audit, Merge authority audit and full Merge gate PASS on that exact SHA, then request one fresh independent Codex review under the repository policy. If Codex finds any material P0/P1/P2, repair with another RED→GREEN cycle and repeat exact-head qualification. Do not merge with unresolved blocking threads.
>
> When the exact final head is clean and fully qualified, refresh PR head and protected `main` once more and squash-merge PR #239 using an expected-head guard. Do not bypass protections. Then read back protected `main`, verify the merge SHA and the canonical Remote Desktop provider binding/validator plus a representative reusable prompt, and return the final exact evidence. Do not claim completion before the protected-main readback succeeds.

## Context checkpoint

```yaml
last_progress: provider binding and 43-prompt sweep are complete; multiple parser bypasses were repaired TDD; three fresh independent-review P2 parser/normalization bypasses remain RED at handoff; temporary GREEN retry helper still needs cleanup
status: validating
branch: governance/remote-desktop-per-action-gate-237
head_sha: external_pr_evidence
pr: 239
final_head_sha: external_pr_evidence
final_head_frozen_at: external_pr_evidence
handoff_source_head_sha: 6962698fa328a5bf12c59709236cce61105948c7
handoff_source_main_sha: 0fa962b4e4f688331fea899ae496dbfdb914583d
ci_trigger_source: pull_request
ci_check_generation: handoff_source_red
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
blocker: three unresolved Codex P2 parser/normalization bypasses plus temporary rdc-final-p2-green-retry workflow still present in PR diff
next_action: refresh live GitHub state; TDD-fix the three unresolved P2s; delete temporary helper; qualify one immutable exact head through all Game gates and fresh independent Codex review; squash merge with expected-head guard; protected-main readback
```
