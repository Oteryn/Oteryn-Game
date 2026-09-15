# OTV2-20260915-closure-convergence-protocol-624

```yaml
task_id: OTV2-20260915-closure-convergence-protocol-624
title: Add reusable closure convergence protocol
mode: GOVERNANCE
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/closure-convergence-protocol
pr: 625
base_sha: ebf581da4c824ca7ad68faf4b52cd3cb237a3ab1
head_sha: pending_final_commit
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT
created_at: 2026-09-15T10:49:00Z
updated_at: 2026-09-15T11:24:00Z
execution_policy: continuous_progress
owned_paths:
  - docs/agents/AGENTS.md
  - docs/agents/CLOSURE_CONVERGENCE_PROTOCOL.md
  - docs/agents/prompts/OTV2_WORK_DELIVERY_COORDINATOR.md
  - docs/agents/prompts/OTV2_IMPL_DURABILITY.md
  - docs/agents/tasks/active/OTV2-20260915-closure-convergence-protocol-624.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Introduce a reusable late-stage convergence protocol so delivery can perform one full read-only defect sweep, freeze root causes, repair compatible material blockers as one coherent generation, qualify once, and complete one final whole-diff review without weakening current authority or protected integration rules.

## Architecture and source of truth

- `PROVEN`: `OTERYN_ORGANIZATION_AGENT_POLICY@3.1.0` is bound by `docs/agents/META_AGENT_POLICY_BINDING.json` at META commit `23b21e9b1b2d4b6c3a5cac3d4c7a18747804c090`.
- `PROVEN`: the prompting standard requires task-specific deltas rather than duplicating the full agent operating system.
- `PROVEN`: current Work coordinator already owns evidence caching and anti-loop retry control, but its ordinary `one bounded task per worker` model had no explicit late-stage coherent-repair-generation override.
- `PROVEN`: current Durability prompt required one next handoff action but had no convergence-mode batch semantics or explicit low-level Git-object publication fallback prohibition.
- `PROVEN`: the independent auditor already must read the nearest `docs/agents/AGENTS.md`; convergence-specific audit semantics can therefore be routed through that canonical instruction surface plus `CLOSURE_CONVERGENCE_PROTOCOL.md` without widening auditor authority.
- `PROVEN`: `PROMPT_LIFECYCLE.json` needs no mutation because this change creates no new prompt ID and changes no prompt owner, reusable/retired status, supersession relation or registered lifecycle scope identity.

## High-risk authority/recovery qualification

```yaml
applicable: NOT_APPLICABLE
reason: Documentation/prompt governance only; no production mutation, authority-bearing session replacement, PREPARE/COMMIT, persisted recovery interpretation, runtime/database mutation, or protected integration is performed by this task.
```

## Acceptance criteria

- [x] Add one routed closure convergence protocol with explicit activation, one-shot sweep, finding classification, root-cause collapse, frozen inventory, coherent repair generation, final qualification/review and anti-drip novelty triggers.
- [x] Add publication-safety fail-closed behavior for unavailable normal Git publication; no direct low-level Git-object fallback on canonical material branches.
- [x] Work coordinator routes convergence-mode workers and auditors through the protocol and treats a bounded task as a bounded coherent repair generation during closure.
- [x] Convergence audit dispatch fits the coordinator's existing minimal packet by using an `accepted_decisions` convergence descriptor rather than new top-level keys.
- [x] Durability implementer consumes frozen root-cause batches and does not stop after the first compatible blocker.
- [x] Independent auditor supports `DISCOVERY_SWEEP` and `FINAL_CANDIDATE_REVIEW` through nearest `docs/agents/AGENTS.md` + the routed protocol, without gaining implementation authority.
- [x] Auditor evidence `classification: PROVEN | DERIVED | UNKNOWN | CONFLICT` remains separate from convergence `gate_classification: MATERIAL_BLOCKER | EVIDENCE_GAP | HARDENING | OUT_OF_SCOPE`.
- [x] No prompt lifecycle identity/status/supersession field changes; no lifecycle registry mutation required.
- [x] No change to #356 material source, runtime/product behavior, workflows, rulesets, Merge Queue semantics, Platform/Atlas/META or production state.

## Excluded scope

No WP3 product repair, no #356 mutation, no Cargo/runtime source, no workflow/ruleset/protection change, no Merge Queue submission, no external repository write.

## Implementation / findings

- Added `docs/agents/CLOSURE_CONVERGENCE_PROTOCOL.md` as the single reusable closure procedure rather than duplicating the full flow in every prompt.
- Routed convergence activation/audit modes through `docs/agents/AGENTS.md` so the existing independent auditor consumes the modes through its mandatory nearest-instruction startup path.
- Added a compact convergence-mode delta to `OTV2_WORK_DELIVERY_COORDINATOR.md`.
- Added consolidated-repair and publication-safety deltas to `OTV2_IMPL_DURABILITY.md`.
- Kept `PROMPT_LIFECYCLE.json` unchanged because lifecycle metadata is unchanged.

### Independent review generation 1

Codex review `5209285222` was bound to historical head `7abaa1fa2ad779e821435888549fcd61e686890c` and reported three P1 findings:

1. `4014990995` — task custody omitted `docs/agents/AGENTS.md`. `STALE_ON_CURRENT_LINEAGE`: already repaired before review publication by task-packet commit `b9ae8016583d4712cad5817dc8d7a274ed611d09`, which added `docs/agents/AGENTS.md` and removed the unmodified auditor prompt from `owned_paths`.
2. `4014991004` — convergence audit parameters conflicted with the coordinator's mandatory minimal context packet. `ACCEPTED_AND_REPAIRED`: convergence dispatch now uses one structured descriptor inside the existing `accepted_decisions` field; no new top-level packet keys.
3. `4014991011` — convergence reused `classification` for gate disposition and conflicted with auditor evidence classification. `ACCEPTED_AND_REPAIRED`: evidence classification remains `PROVEN | DERIVED | UNKNOWN | CONFLICT`; convergence now uses separate `gate_classification`.

The material repair invalidates generation-1 review as final qualification. Fresh exact-head deterministic validation and one fresh independent deep review are required.

## Validation

### Focused

- changed-file inventory vs `main`: PASS — only the five owned documentation/prompt/task paths above.
- prompt semantic self-review: PASS after review repair — no authority expansion, no review weakening, no Merge Queue semantic change, no product/runtime mutation.
- META 3.1 prompting-standard reconciliation: PASS — one routed shared protocol with prompt-specific deltas; no full global procedure copied into every prompt.

### Component/integration

- prior exact-head Agent Governance generation on `b9ae801...`: SUCCESS after PR metadata repair.
- prior Architecture Semantic Audit generation on `b9ae801...`: SUCCESS.
- prior Merge Gate generation on `b9ae801...`: SUCCESS.
- fresh exact-head repository checks: required after this material review repair.

### E2E

- scenario: NOT_APPLICABLE — prompt/governance documentation only
- result: NOT_APPLICABLE

### Exact-head CI

- final head: pending after this task-record commit
- trigger source: pull_request
- workflow/run/job: pending
- runner assignment: pending
- classification: pending
- result: pending

## Self-review

- exact head: pending after this task-record commit
- method/reviewer: implementing agent whole-diff review against #624 + bound META 3.1 + accepted review findings
- material findings: 0 open in producer self-review
- verdict: PASS_PENDING_EXACT_HEAD_CI_AND_FRESH_INDEPENDENT_REVIEW

## Independent review

- required: YES — material high-risk/control-plane governance behavior under bound META AI review policy
- historical review: `5209285222` / head `7abaa1fa...` / 3 P1, dispositioned above
- exact head: pending final head
- method/auditor: one fresh Codex deep review after deterministic validation
- material findings: pending
- verdict: pending

## PR and closeout

- PR: #625
- changed-file review: PASS pre-final-freeze
- unresolved review threads: 3 historical threads pending reply/resolution after repaired exact head is published
- related/superseded PRs: none
- protected integration: NOT_AUTHORIZED_BY_TASK
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: Reconciled first independent review as one coherent repair batch; separated evidence/gate classification and fit convergence dispatch into the existing minimal packet contract.
status: validating
branch: agent/closure-convergence-protocol
head_sha: pending_final_commit
pr: 625
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
owner_action_required: null
blocker: null
next_action: Freeze the new exact head, run fresh exact-head CI, reply/resolve historical review threads, then obtain one fresh independent deep review.
```
