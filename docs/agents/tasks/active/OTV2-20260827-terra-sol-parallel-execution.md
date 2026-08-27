# OTV2-20260827-terra-sol-parallel-execution

```yaml
task_id: OTV2-20260827-terra-sol-parallel-execution
title: Package Terra control plane and Sol parallel lead architecture
mode: GOVERNANCE
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: docs/terra-sol-parallel-agent-architecture-20260827
issue: 213
pr: 214
base_sha: 4c395ece416c3c56aed5607653a0730c52dcb3fd
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT GPT-5.6 Sol
created_at: 2026-08-27T09:27:00+02:00
updated_at: 2026-08-27T12:34:00+02:00
execution_budget_minutes: 120
large_budget_reason: authority-sensitive multi-agent execution architecture, prompt family and lifecycle registration
owned_paths:
  - docs/superpowers/specs/2026-08-27-oteryn-game-terra-sol-parallel-execution-design.md
  - docs/superpowers/plans/2026-08-27-oteryn-game-terra-sol-parallel-execution.md
  - docs/agents/prompts/OTV2_TERRA_GAME_CONTROL_PLANE.md
  - docs/agents/prompts/OTV2_WORK_DELIVERY_COORDINATOR.md
  - docs/agents/prompts/OTV2_SOL_SUPERVISING_ARCHITECT.md
  - docs/agents/prompts/OTV2_SOL_DURABILITY_LEAD.md
  - docs/agents/prompts/OTV2_SOL_SERVER_SEAM_LEAD.md
  - docs/agents/prompts/OTV2_SOL_CLIENT_QA_LEAD.md
  - docs/agents/prompts/OTV2_SOL_MOVEMENT_LEAD.md
  - docs/agents/prompts/OTV2_SOL_COMBAT_LEAD.md
  - docs/agents/prompts/OTV2_SOL_POST_VSL_EXPANSION.md
  - docs/agents/programs/OTERYN_V2_TERRA_SOL_EXECUTION_SCHEDULER.md
  - docs/agents/prompts/README.md
  - docs/agents/PROMPT_LIFECYCLE.json
  - docs/agents/tasks/active/OTV2-20260827-terra-sol-parallel-execution.md
public_contracts: []
depends_on:
  - owner direction in Issue #213
  - existing Sol-lead design and plan on protected main
  - terminal ownership release of Issue #179 through PR #184 / main:c082060c10a54758ebcc3655d0bffa83f4af51e3
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Publish a reusable execution-governance package in which ChatGPT Work on Terra High is a deterministic control plane with zero technical/architecture discretion, while exact Sol Extra High lane leads own implementation reasoning and a Sol Supervising Architect owns material cross-lane architecture decisions.

Both `Oteryn: work coordinator` and `Oteryn: terra game coordinator` remain reusable globally, but one programme lifecycle may have exactly one active mutating control-plane profile. Legacy #162 remains Work-controlled until a separate durable transition explicitly selects Terra.

## Architecture and source of truth

- `PROVEN`: protected admission main is `4c395ece416c3c56aed5607653a0730c52dcb3fd`.
- `PROVEN`: Issue #213 records owner scope for this execution architecture.
- `PROVEN`: the existing owner-approved Sol-lead execution design and adoption plan are already on protected main.
- `PROVEN`: current Durability work is live under Issue #167 and draft PR #212 at task admission; live GitHub may advance during this governance task.
- `PROVEN`: independent exact-head review of PR #214 head `5fa107837f0b8b3c31c493ec0511b00176a23f75` returned `CHANGES_REQUIRED` with P1 findings for stale predecessor ownership, competing reusable control planes and stale task metadata.
- `PROVEN`: predecessor design closeout PR #184 was merge-refreshed, passed exact-head governance checks and squash-merged as protected `main@c082060c10a54758ebcc3655d0bffa83f4af51e3`; Issue #183 is closed completed and the old task now has `owned_paths: []`.
- `PROVEN`: PR #214 branch was normally merge-updated with `main@c082060c10a54758ebcc3655d0bffa83f4af51e3` before review repairs; no force-push/reset/restart was used.
- `DERIVED`: the ownership collision from the first independent review is terminally removed.
- `DERIVED`: keeping both control-plane aliases reusable is safe only with a durable single-active-profile rule enforced by both coordinator prompts; this repair adds that fail-closed rule without silently taking over the active #162 lifecycle.

## Acceptance criteria

- [x] Terra coordinator prompt has explicit zero technical/architecture discretion.
- [x] Work/Terra control-plane coexistence is mutually exclusive for programme mutation and legacy #162 remains Work-controlled until durable transfer.
- [x] Sol Supervising Architect prompt routes material architecture decisions without granting implicit runtime writes.
- [x] Sol Durability, Server Seam, Client/QA, Movement and Combat lead prompts are present and bind mutation to exact live allocation.
- [x] Post-VSL expansion prompt can decompose later accepted work without speculative runtime authority.
- [x] Scheduler defines V0-V4 promotion, useful parallel preparation, writer limits and unique active control-plane resolution.
- [x] README and prompt lifecycle register every new alias without implicit supersession.
- [ ] Final exact-head prompt evaluation against `PROMPT_EVAL_STANDARD.md` is complete after repair freeze.
- [ ] Exact-head agent governance and applicable repository governance checks pass on the repaired head.
- [ ] Genuinely independent exact-head review passes on the repaired head before merge because execution/merge authority boundaries change.

## Excluded scope

No gameplay/runtime, Cargo/workspace, protocol/schema/registry, production/protected environment, secrets, live data, Platform/Atlas/META or external-repository mutation. Do not modify active Durability #167/#212 implementation history or current product worker branches.

## Implementation / findings

The initial package correctly constrained Terra technical discretion and Sol lane ownership, but genuinely independent review found three material governance defects on head `5fa107837f0b8b3c31c493ec0511b00176a23f75`:

1. predecessor Issue #179 still owned `README.md` and `PROMPT_LIFECYCLE.json` because its closeout #184 was not terminal;
2. Work and Terra profiles were both reusable without a fail-closed per-programme single-active-control-plane rule;
3. this active task packet still claimed `pr: null`, pending construction and a stale next action.

Repair sequence:

- terminally complete #184/#183 first, releasing predecessor ownership;
- merge-up the resulting protected main into #214 without history rewrite;
- add the single-active-control-plane rule to both Terra and Work profiles, scheduler, design and README while leaving aliases reusable/unsuperseded;
- refresh this packet with known PR/progress/ownership facts without attempting to self-record its own final SHA.

No runtime worker ownership is changed. In particular, draft Durability PR #212 remains on its existing branch/history and paths.

## Validation

### Focused

- command/run: independent review against original exact head `5fa107837f0b8b3c31c493ec0511b00176a23f75`
- result: `CHANGES_REQUIRED`; P1-01 predecessor ownership, P1-02 competing control planes, P1-03 stale active task metadata
- repair status: P1-01 terminally resolved by #184/#183; P1-02/P1-03 incorporated into this repair candidate

### Component/integration

- command/run: exact-head GitHub governance workflows after repaired-head freeze
- result: pending repaired head generation; prior head results are historical only and will not be reused

### E2E

- scenario: `NOT_APPLICABLE` — governance/prompt package only, no executable product behavior
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: intentionally not self-recorded in this commit; GitHub PR head is the immutable qualification key
- trigger source: pull_request #214
- workflow/run/job: pending repaired final generation
- runner assignment: pending
- classification: governance-only
- result: pending

## Self-review

- exact head: repaired PR head after this candidate commit
- method/reviewer: repair-authoring session; whole-scope review against Issue #213, first independent findings and `PROMPT_EVAL_STANDARD.md`
- material findings: P1-01 is externally closed by #184/#183; P1-02 and P1-03 are addressed in the candidate content; final exact-head self-review remains required after commit
- verdict: `PENDING_EXACT_HEAD_SELF_REVIEW`

## Independent review

- required: YES — execution/merge authority boundary change under `docs/agents/AGENTS.md`
- previous exact head: `5fa107837f0b8b3c31c493ec0511b00176a23f75`
- previous method/auditor: genuinely independent non-authoring reviewer session
- previous material findings: P1=3, P0=0, plus one non-blocking P2
- previous verdict: `CHANGES_REQUIRED`
- repaired exact head: pending final candidate commit
- repaired-head method/auditor: must be a genuinely independent non-authoring reviewer/session; this repair-authoring session does not count
- repaired-head verdict: pending

## PR and closeout

- PR: #214
- changed-file review: 15 intended governance/docs paths after adding the bounded Work-coordinator coexistence amendment
- unresolved review threads: zero at the reviewed pre-repair head; must be re-read on repaired exact head
- related/superseded PRs: #184 merged as `c082060c10a54758ebcc3655d0bffa83f4af51e3` and Issue #183 closed completed; no predecessor ownership remains
- protected auto-merge: not requested
- merge commit/result: pending repaired exact-head qualification
- ownership release: pending terminal delivery

## Context checkpoint

```yaml
last_progress: predecessor ownership #184/#183 is terminal; PR #214 is merge-updated with main c082060c10a54758ebcc3655d0bffa83f4af51e3 and the repair candidate adds fail-closed Work/Terra mutual exclusion plus current task metadata
status: validating
branch: docs/terra-sol-parallel-agent-architecture-20260827
head_sha: null
pr: 214
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request
ci_check_generation: repaired_candidate_pending
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 1
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: genuinely independent exact-head review remains mandatory after repaired head freezes
next_action: freeze the repaired PR #214 candidate, then require fresh exact-head governance CI and genuinely independent review on that unchanged head
```
