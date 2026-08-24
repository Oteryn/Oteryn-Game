# OTV2-20260824-close-next-wave-blockers

```yaml
task_id: OTV2-20260824-close-next-wave-blockers
title: Add autonomous next-wave blocker closer prompt
mode: GOVERNANCE
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: docs/otv2-close-next-wave-blockers-128
issue: 128
pr: null
base_sha: 5834e1dc44a4963ba1645d26e9f5599f5eda7604
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT prompt-packaging coordinator
created_at: 2026-08-24T23:48:55+02:00
updated_at: 2026-08-25T00:00:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/agents/prompts/OTV2_CLOSE_NEXT_WAVE_BLOCKERS.md
  - docs/agents/prompts/README.md
  - docs/agents/PROMPT_LIFECYCLE.json
  - docs/agents/tasks/active/OTV2-20260824-close-next-wave-blockers.md
public_contracts: []
depends_on:
  - Issue #128 owner authorization
  - Issues #93, #115, #116, #123 live blocker state
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Publish one reusable prompt and alias `Oteryn: close next-wave blockers` that can autonomously close the current next-wave blocker set under the exact owner-decision envelope in Issue #128, without granting unrelated runtime or production authority.

## Architecture and source of truth

- `PROVEN`: GitHub `main@5834e1dc44a4963ba1645d26e9f5599f5eda7604` is the exact task base.
- `PROVEN`: #93, #115, #116 and #123 are the current blocker issues named by the owner-approved scope.
- `PROVEN`: Issue #128 explicitly authorizes evidence-backed first-slice numeric acceptance and the bounded #115 implementation exception.
- `PROVEN`: retained prompts require lifecycle registration in `docs/agents/PROMPT_LIFECYCLE.json`.
- `PROVEN`: this prompt-package authority expansion requires genuinely independent exact-head review before merge.

## Acceptance criteria

- [ ] New prompt satisfies all required sections in `PROMPTING_STANDARD.md`.
- [ ] Prompt evaluation is `PASS` against every `PROMPT_EVAL_STANDARD.md` gate.
- [ ] README exposes the exact alias.
- [ ] Lifecycle registry contains exactly one reusable entry for the new prompt.
- [ ] Governance validation, placeholder scan and diff check pass.
- [ ] Whole-diff self-review finds no material P0/P1/P2 issue.
- [ ] Genuinely independent exact-head review passes.
- [ ] Exact-head repository gates including `game-gate` pass with zero unresolved threads.
- [ ] PR squash-merges, Issue #128 closes, task archives and ownership releases.
## Excluded scope

No gameplay/runtime implementation except the future #115 blocker-specific verifier/consumer exception governed by the reusable prompt. No Server Seam, Durability, Ability, Interaction, AI, Client, Movement or Combat implementation. No production deployment, live keys/secrets, Platform or external-repository writes.

## Implementation / findings

Issue #128 and branch `docs/otv2-close-next-wave-blockers-128` were created from exact `main@5834e1dc44a4963ba1645d26e9f5599f5eda7604`. No overlapping open PR for #115/#116/#123 or this prompt was found at preflight.

The prompt records the owner authorization as durable GitHub authority rather than relying on chat and preserves serialized registry mutation plus fail-closed downstream allocation gates.

Pre-freeze prompt evaluation is `PASS` against Authority, Resolution, Ownership, Architecture, Completeness, Evidence, Validation, Autonomy, Handover and Safety. Local governance validation also passes (25 required policy documents / 9 lanes).

## Validation

### Focused

- command/run: `python tools/agents/validate_governance.py`, prompt lifecycle/alias/placeholder checks
- result: PASS â€” governance, lifecycle uniqueness, required-section, placeholder and diff checks passed locally.

### Component/integration

- command/run: `python tools/agents/validate_governance.py`; standalone `validate_repository_policy.py` is not present on this tree; exact-head CI remains authoritative for repository policy.
- result: PASS for available local governance validator; repository policy pending exact-head CI.

### E2E

- scenario: `NOT_APPLICABLE` — prompt/governance packaging only
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: pending
- trigger source: pull_request
- workflow/run/job: pending
- runner assignment: pending
- classification: required exact-head governance/semantic/merge-authority/merge-gate
- result: pending

## Self-review

- exact head: pre-freeze staged tree
- method/reviewer: implementing/coordinating agent; full four-file diff and prompt-eval checklist
- material findings: P0=0/P1=0; repaired task YAML issue binding and prompt markdown/newline clarity before freeze
- verdict: PASS pre-freeze; exact-head self-review will be repeated after commit.

## Independent review

- required: YES — bounded owner-decision/coordination authority expansion and #115 trust-boundary implementation exception
- exact head: pending
- method/auditor: non-authoring qualified reviewer; no owner-funded AI without separate exact authorization
- material findings: pending
- verdict: pending
## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: none at preflight
- protected auto-merge: not enabled
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: Final staged four-file tree passes governance, prompt-eval, lifecycle uniqueness, placeholder and diff validation.
status: validating
branch: docs/otv2-close-next-wave-blockers-128
head_sha: null
pr: null
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: null
ci_check_generation: null
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
next_action: Commit and push the validated four-file prompt package, then open the PR.
```
