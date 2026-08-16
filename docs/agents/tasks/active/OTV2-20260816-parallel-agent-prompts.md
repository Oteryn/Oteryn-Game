# OTV2-20260816-parallel-agent-prompts

```yaml
task_id: OTV2-20260816-parallel-agent-prompts
title: Prepare bounded parallel-agent prompts for current Oteryn-v2 work
mode: GOVERNANCE
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/parallel-agent-prompts-20260816
pr: 305
base_sha: d2af53855046df25b4e52edbd5ec14e0513a63ec
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: current coordinating agent
created_at: 2026-08-16T19:09:00+02:00
updated_at: 2026-08-16T19:14:37+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/agents/tasks/active/OTV2-20260816-parallel-agent-prompts.md
  - docs/agents/prompts/OTV2_GAME_ABILITY_OWNER_DECISION_AGENT.md
  - docs/agents/prompts/OTV2_DEPENDABOT_TOKIO_239_AGENT.md
  - docs/agents/prompts/OTV2_DEPENDABOT_SERDE_JSON_240_AGENT.md
  - docs/agents/prompts/OTV2_PROD_ENTITLEMENTS_115_AGENT.md
  - docs/agents/prompts/OTV2_GOVERNANCE_CHECKPOINT_CLEANUP_AUDITOR.md
public_contracts: []
depends_on:
  - live main d2af53855046df25b4e52edbd5ec14e0513a63ec
  - docs/agents/PROMPTING_STANDARD.md
  - docs/agents/PROMPTING_HANDOVER.md
  - docs/agents/PROMPT_EVAL_STANDARD.md
  - docs/agents/MULTI_AGENT_ARCHITECTURE_ORCHESTRATION.md
  - docs/agents/ARCHITECTURE_DECISION_DISCIPLINE.md
  - docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md
blocks: []
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories:
  - blakinio/Oteryn-Platform
```

## Outcome

Create exactly five reusable, self-contained prompts for the currently identified parallel lanes while preserving serial canonicalization and existing repository authority boundaries.

## Architecture and source of truth

- `PROVEN`: live base at task start was `main@d2af53855046df25b4e52edbd5ec14e0513a63ec`.
- `PROVEN`: canonical programme state selects a paper-only `GAME-ABILITY-01` owner-decision package as the next architecture action.
- `PROVEN`: PR #239 and #240 are separate dependency-maintenance lanes and must be revalidated on any moved exact head.
- `PROVEN`: issue #115 remains open for `PROD-ENTITLEMENTS-01` consumer/enforcement architecture; Platform is read-only in this task.
- `PROVEN`: prompt creation grants no runtime, DDL, Platform, production or owner-funded-AI authority.

## Acceptance criteria

- [x] Five prompt files exist under `docs/agents/prompts/`.
- [x] Each prompt has an explicit alias, role/mode, authority boundary, trusted source order, target outcome, validation, stop conditions and handover/completion rule.
- [x] No prompt grants standing Codex/OpenAI/paid-review authority.
- [x] No prompt grants production, protected-environment, runtime implementation, DDL or cross-repository write authority beyond its explicit lane.
- [x] GAME-ABILITY remains the canonical paper-only decision lane and cannot self-accept without owner disposition.
- [x] Dependency PR lanes are non-overlapping and do not grant merge/close authority without exact owner authorization.
- [x] Entitlement work is security-sensitive, Oteryn-v2 consumer/enforcement architecture only, with independent-review gating before canonical acceptance.
- [x] Governance cleanup lane is read-only and cannot perform repair.
- [ ] Exact-head governance/merge-gate validation passes before merge.

## Excluded scope

- runtime/client/server/protocol/content implementation;
- PostgreSQL DDL/migrations;
- Platform writes;
- production/live changes;
- modifying PR #239 or #240 as part of this prompt-package task;
- making GAME-ABILITY or PROD-ENTITLEMENTS semantics accepted merely by creating prompts;
- invoking Codex/OpenAI/paid review.

## Implementation / findings

Five prompt files were created on the dedicated branch and draft PR #305 was opened early.

Full-diff self-review found one P2 bookkeeping defect in the initial task record: `mode: GOVERNANCE_DOCUMENTATION` was not a canonical `TASK_TEMPLATE.md` mode. This update repairs it to `mode: GOVERNANCE` and expands the task record to the current template shape before final-head freeze.

Prompt evaluation against `PROMPT_EVAL_STANDARD.md`:

- authority: PASS;
- resolution: PASS;
- ownership/non-overlap: PASS;
- architecture boundaries: PASS;
- completeness: PASS;
- evidence/truth labels: PASS;
- validation: PASS;
- autonomy/stop conditions: PASS;
- handover: PASS;
- safety: PASS.

No material prompt-content finding remains open.

## Validation

### Focused

- command/run: full PR diff inspection plus manual evaluation against `docs/agents/PROMPTING_STANDARD.md`, `PROMPTING_HANDOVER.md` and `PROMPT_EVAL_STANDARD.md`
- result: PASS after repairing the task-mode P2

### Component/integration

- command/run: repository governance validation via PR CI
- result: pending final-head CI

### E2E

- scenario: `NOT_APPLICABLE` — prompt/governance documentation only; no executable runtime behavior changes
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: pending after this final content repair
- trigger source: pull_request
- workflow/run/job: pending
- runner assignment: pending
- classification: pending
- result: pending

## Self-review

- exact head: pending after this repair
- method/reviewer: current coordinating agent, deliberate complete PR diff review
- material findings: initial noncanonical task mode P2, repaired before freeze; no open prompt-content finding
- verdict: pending exact-head confirmation

## Independent review

- required: NO — this package adds bounded prompts while preserving or narrowing authority; it does not reduce a safety gate or expand repository/merge/production/cross-repository authority
- exact head: `NOT_APPLICABLE`
- method/auditor: `NOT_APPLICABLE`
- material findings: `NOT_APPLICABLE`
- verdict: `NOT_APPLICABLE`

## PR and closeout

- changed-file review: five prompt files plus this one task record; no runtime/architecture contract path
- unresolved review threads: pending exact-head check
- related/superseded PRs: #239 and #240 are future prompt targets only and are not modified by PR #305
- protected auto-merge: not configured
- merge commit/result: pending
- ownership release: pending post-merge lifecycle closeout

## Context checkpoint

```yaml
last_progress: full-diff prompt evaluation completed; task-mode P2 repaired before final-head freeze
status: validating
branch: docs/parallel-agent-prompts-20260816
head_sha: null
pr: 305
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request
ci_check_generation: pending
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
owner_action_required: false
blocker: null
next_action: freeze the new exact PR #305 head, perform exact-head self-review confirmation, and verify required repository CI without further content changes
```
