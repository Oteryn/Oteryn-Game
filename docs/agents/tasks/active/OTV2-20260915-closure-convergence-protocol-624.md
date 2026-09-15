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
head_sha: pending_final_qualification
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT
created_at: 2026-09-15T10:49:00Z
updated_at: 2026-09-15T13:45:00Z
execution_policy: continuous_progress
owned_paths:
  - docs/agents/AGENTS.md
  - docs/agents/CLOSURE_CONVERGENCE_PROTOCOL.md
  - docs/agents/PROMPT_LIFECYCLE.json
  - docs/agents/prompts/OTV2_WORK_DELIVERY_COORDINATOR.md
  - docs/agents/prompts/OTV2_WORK_DELIVERY_INDEPENDENT_AUDITOR.md
  - docs/agents/prompts/OTV2_IMPL_DURABILITY.md
  - docs/agents/tasks/active/OTV2-20260915-closure-convergence-protocol-624.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Introduce a reusable late-stage convergence protocol so delivery can perform one full read-only defect sweep, freeze a generation-bound root-cause inventory, reconcile uncertain evidence without mutating frozen evidence, repair compatible material blockers as one coherent generation, qualify one exact candidate, and complete one final whole-diff review without weakening current authority or protected integration rules.

## Architecture and source of truth

- `PROVEN`: `OTERYN_ORGANIZATION_AGENT_POLICY@3.1.0` is bound by `docs/agents/META_AGENT_POLICY_BINDING.json` at META commit `23b21e9b1b2d4b6c3a5cac3d4c7a18747804c090`.
- `PROVEN`: the prompting standard requires task-specific deltas rather than duplicating the full agent operating system.
- `PROVEN`: current Work coordinator already owns evidence caching and anti-loop retry control, but its ordinary `one bounded task per worker` model had no explicit late-stage coherent-repair-generation override.
- `PROVEN`: current Durability prompt required one next handoff action but had no convergence-mode batch semantics or explicit low-level Git-object publication fallback prohibition.
- `PROVEN`: the independent auditor already must read the nearest `docs/agents/AGENTS.md`; convergence-specific audit semantics can therefore be routed through that canonical instruction surface plus `CLOSURE_CONVERGENCE_PROTOCOL.md` without widening auditor authority.
- `PROVEN`: lifecycle versions are coordinator `1.3`, independent auditor `1.3`, durability `1.2`; the auditor prompt's embedded `prompt_version` also equals `1.3`.

## High-risk authority/recovery qualification

```yaml
applicable: NOT_APPLICABLE
reason: Documentation/prompt governance only; no production mutation, authority-bearing session replacement, PREPARE/COMMIT, persisted recovery interpretation, runtime/database mutation, or protected integration is performed by this task.
```

## Acceptance criteria

- [x] Add one routed closure convergence protocol with explicit activation, one-shot sweep, finding classification, root-cause collapse, frozen inventory, coherent repair generation, final qualification/review and anti-drip novelty triggers.
- [x] Bind discovery sweeps to the exact frozen closure head and exact protected-`main` generation; mixed-generation sweep evidence fails closed.
- [x] Re-resolve protected `main` before contract-sensitive evidence reconciliation and before mutating repair dispatch; material movement invalidates the sweep base before mutation.
- [x] Keep `UNKNOWN`/`CONFLICT` material findings out of mutating repair generations until evidence is reconciled.
- [x] Preserve every frozen root-cause inventory as immutable; evidence reconciliation creates one hashed successor inventory per coherent reconciliation generation.
- [x] Give every inventory generation its own evidence locator; never overwrite or repurpose the predecessor locator for successor content.
- [x] Represent the hashed `inventory` envelope member as a structured JSON value rather than a string placeholder, matching the RFC 8785 canonicalization contract.
- [x] Bind each active inventory generation to RFC 8785 + SHA-256 identity including repository/Issue/task/PR/branch, protected-main/audit-main generation, Phase-1 closure head/tree, inventory generation and predecessor identity.
- [x] Bind the active-inventory pointer as `(inventory_generation, evidence_locator, content_identity)` and fail closed on ambiguous/partial transition.
- [x] Require final-candidate review to exact-match the live target to `qualified_head` and independently reconcile protected-`main` movement since the sweep.
- [x] Preserve `MATERIAL_BLOCKER` when recording `FINAL_SWEEP_MISS`.
- [x] Add publication-safety fail-closed behavior for unavailable normal Git publication; no direct low-level Git-object fallback on canonical material branches.
- [x] Keep coordinator packet compatibility by routing convergence data inside existing `accepted_decisions` rather than new top-level keys.
- [x] Preserve auditor evidence `classification: PROVEN | DERIVED | UNKNOWN | CONFLICT` separately from convergence gate classification.
- [x] Lifecycle versions advance for the three materially revised reusable prompt contracts and embedded prompt metadata matches registry state.
- [x] No change to #356 material source, runtime/product behavior, workflows, rulesets, Merge Queue semantics, Platform/Atlas/META or production state.

## Excluded scope

No WP3 product repair, no #356 mutation, no Cargo/runtime source, no workflow/ruleset/protection change, no Merge Queue submission, no external repository write.

## Implementation / findings

- Added `docs/agents/CLOSURE_CONVERGENCE_PROTOCOL.md` as the single reusable closure procedure rather than duplicating the full flow in every prompt.
- Routed convergence activation/audit modes through `docs/agents/AGENTS.md`.
- Added a compact convergence-mode delta to `OTV2_WORK_DELIVERY_COORDINATOR.md`.
- Added consolidated-repair and publication-safety deltas to `OTV2_IMPL_DURABILITY.md`.
- Advanced lifecycle metadata only for the three changed reusable prompt contracts and aligned the independent auditor's embedded version.

### Independent review generations 1–4

Historical exact-head reviews found and the same lineage repaired:

- task custody omitted `docs/agents/AGENTS.md`;
- convergence audit parameters conflicted with the coordinator minimal packet;
- convergence/evidence classifications collided;
- `FINAL_SWEEP_MISS` could replace blocker classification;
- lifecycle versions did not advance for revised prompt contracts;
- discovery sweep was not bound to exact frozen closure head;
- `UNKNOWN`/`CONFLICT` evidence could drive mutation;
- auditor lifecycle version required advancement;
- editable evidence locator did not prove immutable inventory content.

All were dispositioned and resolved before later qualification generations.

### Independent review generation 5

Codex review `5210336765` plus follow-up on exact head `a3de0bcd14b2adc02f7df9d614b2f6491084546e` reported:

1. `4015903503` P1 — final review could attach to a newer live head after qualification. Repaired by exact live-target equality with `qualified_head`.
2. `4015903521` P1 — inventory digest did not bind repository/task/PR/closure generation. Repaired by immutable sweep-target identity inside the canonical envelope.
3. `4015996290` P2 — lifecycle registry `1.3` did not match auditor embedded `prompt_version: "1.2"`. Repaired after custody extension by aligning the embedded version to `1.3`.

The resulting exact head `dae61ae0ff8b806dc66283c6de947aa76f670103` passed Agent Governance `34974838822`, Architecture Semantic Audit `34974435113`, and Merge Gate `34974838856` before the next review.

### Independent review generation 6

Fresh Codex review `5210541937` on exact qualified head `dae61ae0ff8b806dc66283c6de947aa76f670103` reported:

1. `4016077633` P1 — discovery sweep identity omitted the frozen protected-`main` generation. `ACCEPTED_AND_REPAIRED` in `928be692241791930b49711424950e8e9e83800f`: Phase 1/sweep now bind `base_main_sha`, independently resolved `audit_main_sha`, and the inventory sweep target.
2. `4016077641` P1 — frozen inventory had no conforming transition after successful evidence reconciliation. `ACCEPTED_AND_REPAIRED` in `928be692241791930b49711424950e8e9e83800f`: frozen inventories are never edited in place; coherent evidence reconciliation creates a linked immutable successor generation.

Both review threads were answered with exact repair evidence and resolved.

### Implementing-agent bounded self-review after generation 6

Before treating the generation-6 repair as final, a bounded review of only the protected-main binding and successor-inventory state machine found three handoff gaps and repaired them together in `4db3390ac65bf3902fb59e2716716c3cd4f125a7`:

1. the JSON envelope example rendered the structured `inventory` member as a quoted string despite prose requiring a structured value;
2. protected `main` was rechecked at sweep/final-review boundaries but not immediately before contract-sensitive reconciliation or a mutating repair dispatch;
3. successor inventory semantics required immutable predecessors but did not explicitly require a new generation-specific evidence locator, allowing accidental predecessor-note reuse/overwrite.

The repair makes the envelope example structurally consistent, adds fail-closed live-main reconciliation before mutation, requires a new locator per successor generation, and binds the active inventory as the exact generation/locator/digest tuple. No authority was added and no product/runtime surface changed.

## Validation

### Focused

- changed-file inventory vs `main`: PASS — only task-owned documentation/prompt/task/lifecycle paths.
- prompt semantic self-review: PASS after bounded handoff repair — no authority expansion, no review weakening, no Merge Queue semantic change, no product/runtime mutation.
- META 3.1 prompting-standard reconciliation: PASS — one routed shared protocol with prompt-specific deltas; no full global procedure copied into every prompt.
- lifecycle integrity self-review: PASS — intentionally revised prompt versions and embedded metadata are consistent.
- #356 isolation: PASS — no #356 material path is in this task's changed-file set.

### Component/integration

- prior exact-head qualification on `dae61ae0ff8b806dc66283c6de947aa76f670103`: Agent Governance `34974838822` SUCCESS; Architecture Semantic Audit `34974435113` SUCCESS; Merge Gate `34974838856` SUCCESS. Historical after later repairs.
- later exact-head qualification evidence is historical whenever the candidate moves.
- fresh exact-head repository checks: required on the final candidate produced by this task-record commit.

### E2E

- scenario: NOT_APPLICABLE — prompt/governance documentation only
- result: NOT_APPLICABLE

### Exact-head CI

- final head: record from PR readback after this task-record commit
- trigger source: pull_request
- workflow/run/job: pending fresh exact-head generation
- runner assignment: pending
- classification: pending
- result: pending

## Self-review

- exact head: record from PR readback after this task-record commit
- method/reviewer: implementing agent bounded review against #624 + generation-6 findings + bound META 3.1
- material findings: 0 open in the bounded pass after `4db3390ac65bf3902fb59e2716716c3cd4f125a7`
- verdict: PASS_PENDING_FRESH_EXACT_HEAD_CI_AND_FINAL_INDEPENDENT_REVIEW

## Independent review

- required: YES — material control-plane/governance behavior under bound META AI review policy
- historical generation 1: `5209285222` / head `7abaa1fa...` / 3 P1, dispositioned
- historical generation 5: `5210336765` / head `a3de0bcd...` / 2 P1 + follow-up P2, dispositioned
- historical generation 6: `5210541937` / head `dae61ae0...` / 2 P1, dispositioned
- exact final head: record after this task-record commit
- method/auditor: one fresh Codex deep review after deterministic validation
- material findings: pending
- verdict: pending

## PR and closeout

- PR: #625
- changed-file review: PASS pre-final-freeze
- unresolved review threads: 0 before this self-review repair; must remain 0 before final integration readiness
- related/superseded PRs: none
- protected integration: NOT_AUTHORIZED_BY_TASK
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: Closed generation-6 handoff gaps for structured envelope identity, live-main mutation preflight and generation-specific successor locators; reconciled canonical task record.
status: validating
branch: agent/closure-convergence-protocol
head_sha: record_after_this_commit
pr: 625
final_head_sha: null
final_head_frozen_at: after_current_repair_generation
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
repair_cycles_for_current_gate: 5
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: Freeze exact PR head, update PR body once with required metadata headings, run fresh exact-head CI, then obtain one fresh independent deep review without moving code or metadata.
```
