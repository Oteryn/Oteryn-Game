# OTV2-20260826-sol-lead-selective-codex-design

```yaml
task_id: OTV2-20260826-sol-lead-selective-codex-design
title: Record Sol lead + selective Codex execution architecture
mode: GOVERNANCE
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: docs/sol-lead-selective-codex-179
issue: 179
pr: 180
base_sha: cb9c5f4f53dd880c9d338dafd21b6184a4419993
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT supervising architecture session
created_at: 2026-08-26T07:20:08Z
updated_at: 2026-08-26T09:04:00Z
execution_budget_minutes: 90
large_budget_reason: execution-model architecture, reusable prompt packaging and governance validation
owned_paths:
  - docs/superpowers/specs/2026-08-26-oteryn-game-sol-lead-selective-codex-execution-design.md
  - docs/superpowers/plans/2026-08-26-oteryn-game-sol-lead-selective-codex-execution.md
  - docs/agents/prompts/OTV2_SOL_EXECUTION_ARCHITECTURE_CONTINUATION.md
  - docs/agents/prompts/README.md
  - docs/agents/PROMPT_LIFECYCLE.json
  - docs/agents/tasks/active/OTV2-20260826-sol-lead-selective-codex-design.md
public_contracts: []
depends_on:
  - owner-approved execution direction in Issue #179
  - owner written-spec approval on 2026-08-26
  - OTV2_WORK_DELIVERY_COORDINATOR
  - OTV2_WORK_DELIVERY_INDEPENDENT_AUDITOR
blocks:
  - Packages A-D from docs/superpowers/plans/2026-08-26-oteryn-game-sol-lead-selective-codex-execution.md until this design/plan package is canonical
cross_repository_coordination_id: OTV2-SOL-LEAD-SELECTIVE-CODEX
external_repositories: []
```

## Outcome

Persist the owner-approved execution-model design, continuation prompt and adoption plan so a fresh Sol Extra High agent can continue from repository truth without relying on this chat transcript.

## Architecture and source of truth

- `PROVEN`: admission protected `main` was `cb9c5f4f53dd880c9d338dafd21b6184a4419993`.
- `PROVEN`: Issue #179 records the owner-approved direction: Work control plane + Sol lane leads + selective Codex assistance.
- `PROVEN`: owner reviewed and approved the written spec on 2026-08-26.
- `PROVEN`: fresh planning main after approval advanced to `2e8c1605535167e91fe0c7775e520f599e3d89ea`, merging the serialized Durability shared Cargo/CI lease allocation.
- `PROVEN`: Ability #171, Interaction #172 and AI #178 were merged before this execution-model design; Durability #167 remains on the critical path and now has a coordinator-owned shared Cargo/CI lease lifecycle.
- `PROVEN`: the current Work auditor prompt is read-only and remains an independent control.
- `DERIVED`: this design/plan package does not widen runtime/production/cross-repository authority because it records execution governance only; actual Sol worker writes remain future exact-allocation-gated.

## Acceptance criteria

- [x] Record the Sol-lead/selective-Codex execution design under `docs/superpowers/specs/`.
- [x] Define Work as control plane, Sol leads as bounded reasoning/implementation owners, and Codex as selective implementation assistance.
- [x] Define five-chat useful concurrency, narrower mutating concurrency and serialized shared surfaces.
- [x] Preserve Durability -> Server Seam -> Client/QA -> Movement -> Combat dependency order.
- [x] Preserve audit/reconciliation findings rather than rewriting history.
- [x] Add a continuation prompt that requires live GitHub, written-spec owner review, writing-plans, prompt evaluation and fail-closed authority.
- [x] Register the continuation prompt in README/lifecycle metadata.
- [x] Self-review the written spec for placeholders, contradictions, scope and unsupported authority.
- [x] Owner reviews and approves the written spec before implementation-plan authoring.
- [x] Create the adoption implementation/governance plan and split it into independently reviewable Packages A-D.
- [ ] Refresh the PR branch against current protected main before final integration qualification.
- [ ] Exact-head governance/repository-policy/semantic/merge-authority/game-gate qualification is complete before merge.

## Excluded scope

No gameplay/runtime, Cargo/workspace, `Cargo.lock`, composition, resource registry, stable IDs, protocol/schema, workflow, production, secrets, Platform/Atlas/META or external-repository mutation. No Sol lane implementation begins from this task.

## Design self-review

- placeholder scan: `PASS` — no `TBD`, `TODO` or `fill in` markers found in the written spec;
- internal consistency: `PASS` — Work remains control plane, Sol writes stay allocation-gated, shared surfaces remain serialized and Codex is optional/bounded;
- scope: `PASS` — the document changes execution architecture only and explicitly excludes runtime/product architecture;
- ambiguity: `PASS` — direct Codex use is conditional on a real supported mechanism; otherwise `CODEX_HANDOFF_REQUIRED` or safe continuation without Codex is mandatory;
- historical truth: `PASS` — current Ability review/lifecycle findings are preserved as transition findings rather than rewritten.

## Plan self-review

- spec coverage: `PASS` — transition reconciliation, Work specialization, Sol prompt family, selective Codex contract, scheduler, launch sheet, escalation and terminal adoption are mapped to Packages A-D plus the first execution-wave gate;
- dependency consistency: `PASS` — reconciliation precedes control-plane specialization; Work specialization precedes Sol prompt integration; prompt family precedes scheduler publication;
- drift handling: `PASS` — the plan records `main@2e8c1605535167e91fe0c7775e520f599e3d89ea` as provenance only and requires every package to resolve newer live truth;
- runtime boundary: `PASS` — no product/runtime implementation is authorized by this package.

## Prompt evaluation

`OTV2_SOL_EXECUTION_ARCHITECTURE_CONTINUATION` against `docs/agents/PROMPT_EVAL_STANDARD.md`:

- Authority: `PASS` — exact docs/governance continuation authority and runtime/production/cross-repo exclusions are explicit.
- Resolution: `PASS` — fresh protected main, live coordinator, audit findings, task/PR state and canonical spec are mandatory startup inputs.
- Ownership: `PASS` — future writes require a fresh exact task/branch/PR lifecycle; Sol implementation remains future allocation-gated.
- Architecture: `PASS` — accepted runtime architecture and DAG remain authoritative; execution-model conflicts escalate rather than being invented through.
- Completeness: `PASS` — spec review, writing-plan transition, prompt family, selective Codex, concurrency, transition truth and owner launch sheet are covered.
- Evidence: `PASS` — `PROVEN / DERIVED / UNKNOWN / CONFLICT`, exact SHAs and no-fake-Codex evidence are explicit.
- Validation: `PASS` — prompt evaluation plus repository exact-head governance/CI/review obligations are explicit.
- Autonomy: `PASS` — the agent can continue architecture packaging after owner confirmation but has concrete fail-closed stop conditions.
- Handover: `PASS` — final owner launch sheet and exact transition truth are required.
- Safety: `PASS` — no runtime, production, secrets, external-repo or unapproved owner-funded AI authority.

## Validation

### Focused

- command/run: prompt self-evaluation against `docs/agents/PROMPT_EVAL_STANDARD.md`
- result: `PASS` on all 10 gates as recorded above

### Component/integration

- initial PR #180 agent-governance run `32942962226`: `FAIL` only because PR body lacked mandatory `## Validation`; repository validation steps did not execute.
- corrective action: PR body now includes `## Validation`; next exact-head generation must prove the corrected metadata and package.

### E2E

- scenario: `NOT_APPLICABLE` — execution architecture/prompt/plan packaging only
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: pending final candidate freeze
- trigger source: pull_request #180
- workflow/run/job: pending new generation after plan/task changes
- runner assignment: pending
- classification: docs/governance only
- result: pending

## Self-review

- exact head: pending external exact-head evidence after final candidate freeze
- method/reviewer: implementing architecture session, whole-scope review against Issue #179, approved spec and governing prompt standard
- material findings: no authored-scope material finding; initial CI metadata defect was PR-body-only and has been corrected
- verdict: `PASS_PENDING_MAIN_REFRESH_AND_EXACT_HEAD_CI`

## Independent review

- required: pending final governance risk classification; the package is intended to specialize roles without widening runtime or merge authority
- exact head: pending or `NOT_APPLICABLE`
- method/auditor: pending or `NOT_APPLICABLE`
- material findings: pending or `NOT_APPLICABLE`
- verdict: pending or `NOT_APPLICABLE`

## PR and closeout

- changed-file review: six intended documentation/governance paths including the approved adoption plan
- unresolved review threads: pending
- related/superseded PRs: open PR #150 is unrelated root-governance work and must not be modified by this task
- protected auto-merge: not requested
- merge commit/result: pending current-main refresh and exact-head qualification
- ownership release: pending terminal delivery

## Context checkpoint

```yaml
last_progress: owner approved written spec; adoption plan created; PR metadata validation defect corrected; fresh main includes Durability shared Cargo/CI lease allocation
status: validating
branch: docs/sol-lead-selective-codex-179
head_sha: null
pr: 180
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request
ci_check_generation: pending_new_head
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 1
ci_recovery_actions_for_current_head: 1
stall_warnings: 0
owner_action_required: null
blocker: current PR branch must be reconciled with protected main before final integration qualification
next_action: refresh PR #180 branch against current protected main, then freeze exact head and require a fresh exact-head CI/review generation
```
