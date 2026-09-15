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
updated_at: 2026-09-15T13:37:25Z
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
- `PROVEN`: the convergence changes materially revise three reusable prompt contracts, so `PROMPT_LIFECYCLE.json` advances `OTV2_WORK_DELIVERY_COORDINATOR` from `1.2` to `1.3`, `OTV2_WORK_DELIVERY_INDEPENDENT_AUDITOR` from `1.2` to `1.3`, and `OTV2_IMPL_DURABILITY` from `1.1` to `1.2` while preserving all IDs, owners, statuses, scopes and supersession relations. The auditor prompt's embedded `prompt_version` matches registry version `1.3`.

## High-risk authority/recovery qualification

```yaml
applicable: NOT_APPLICABLE
reason: Documentation/prompt governance only; no production mutation, authority-bearing session replacement, PREPARE/COMMIT, persisted recovery interpretation, runtime/database mutation, or protected integration is performed by this task.
```

## Acceptance criteria

- [x] Add one routed closure convergence protocol with explicit activation, one-shot sweep, finding classification, root-cause collapse, frozen inventory, coherent repair generation, final qualification/review and anti-drip novelty triggers.
- [x] Bind discovery sweeps to the exact frozen closure head and fail closed if the live target moved.
- [x] Bind discovery sweeps to the exact frozen protected-`main` generation: `base_main_sha` and auditor `audit_main_sha` must exact-match before the sweep; mixed-main evidence fails closed.
- [x] Keep `UNKNOWN`/`CONFLICT` material findings out of mutating repair generations until evidence is reconciled; only repair-eligible material blockers enter the batch.
- [x] Preserve every frozen root-cause inventory as immutable; evidence reconciliation creates one hashed successor inventory per coherent reconciliation generation, linked to its predecessor, rather than mutating the frozen digest in place.
- [x] Bind every active root-cause inventory generation to an RFC 8785 canonical JSON SHA-256 identity, including repository/Issue/task/PR/branch, protected-main/audit-main generation, Phase-1 closure head/tree, inventory generation and predecessor identity.
- [x] Require final-candidate review to resolve the live PR/branch and exact-match its current head to the qualified head; target movement fails closed rather than attaching review evidence to an unqualified generation.
- [x] Require final review to detect protected-`main` movement since the sweep; material current-gate movement invalidates the sweep base, while only proven immaterial movement may continue with exact evidence.
- [x] Add publication-safety fail-closed behavior for unavailable normal Git publication; no direct low-level Git-object fallback on canonical material branches.
- [x] Work coordinator routes convergence-mode workers and auditors through the protocol and treats a bounded task as a bounded coherent repair generation during closure.
- [x] Convergence audit dispatch fits the coordinator's existing minimal packet by using an `accepted_decisions` convergence descriptor rather than new top-level keys.
- [x] Durability implementer consumes frozen root-cause batches and does not stop after the first compatible blocker.
- [x] Independent auditor supports `DISCOVERY_SWEEP` and `FINAL_CANDIDATE_REVIEW` through nearest `docs/agents/AGENTS.md` + the routed protocol, without gaining implementation authority.
- [x] Auditor evidence `classification: PROVEN | DERIVED | UNKNOWN | CONFLICT` remains separate from convergence `gate_classification: MATERIAL_BLOCKER | EVIDENCE_GAP | HARDENING | OUT_OF_SCOPE`.
- [x] A late `FINAL_SWEEP_MISS` is an additional sweep disposition and never replaces/downgrades `gate_classification: MATERIAL_BLOCKER` for a real current-gate defect.
- [x] Lifecycle registry versions advance for the three reusable prompt contracts whose behavior changed, and each prompt's embedded version matches the registry.
- [x] No change to #356 material source, runtime/product behavior, workflows, rulesets, Merge Queue semantics, Platform/Atlas/META or production state.

## Excluded scope

No WP3 product repair, no #356 mutation, no Cargo/runtime source, no workflow/ruleset/protection change, no Merge Queue submission, no external repository write.

## Implementation / findings

- Added `docs/agents/CLOSURE_CONVERGENCE_PROTOCOL.md` as the single reusable closure procedure rather than duplicating the full flow in every prompt.
- Routed convergence activation/audit modes through `docs/agents/AGENTS.md` so the existing independent auditor consumes the modes through its mandatory nearest-instruction startup path.
- Added a compact convergence-mode delta to `OTV2_WORK_DELIVERY_COORDINATOR.md`.
- Added consolidated-repair and publication-safety deltas to `OTV2_IMPL_DURABILITY.md`.
- Advanced lifecycle metadata only for the three changed reusable prompt contracts and aligned the independent auditor's embedded version.

### Independent review generation 1

Codex review `5209285222` was bound to historical head `7abaa1fa2ad779e821435888549fcd61e686890c` and reported three P1 findings:

1. `4014990995` — task custody omitted `docs/agents/AGENTS.md`. `STALE_ON_CURRENT_LINEAGE`: already repaired before review publication by task-packet commit `b9ae8016583d4712cad5817dc8d7a274ed611d09`.
2. `4014991004` — convergence audit parameters conflicted with the coordinator's mandatory minimal context packet. `ACCEPTED_AND_REPAIRED`: convergence dispatch uses a structured descriptor inside existing `accepted_decisions`.
3. `4014991011` — convergence reused `classification` for gate disposition and conflicted with auditor evidence classification. `ACCEPTED_AND_REPAIRED`: evidence and gate classification are separate fields.

### Independent review generations 2–4

Subsequent exact-head reviews found and the same lineage repaired:

- `4015090632` — preserve `MATERIAL_BLOCKER` when marking `FINAL_SWEEP_MISS`;
- `4015090638` — advance lifecycle versions for materially revised reusable prompt contracts;
- `4015189472` — bind `DISCOVERY_SWEEP` to the exact frozen closure SHA;
- `4015189486` — prevent `UNKNOWN`/`CONFLICT` evidence from driving mutation;
- historical auditor lifecycle mismatch at head `bdd67024...` — auditor lifecycle advanced to `1.3`;
- generation-4 P1 on `bf0a44b...` — bind frozen inventory to immutable RFC 8785 + SHA-256 content identity.

All were accepted/repaired before the later exact-head qualification generations.

### Independent review generation 5

Codex review `5210336765` plus follow-up on exact head `a3de0bcd14b2adc02f7df9d614b2f6491084546e` reported two P1 findings and one related P2:

1. `4015903503` — final review could attach to a newer live head after qualification. `ACCEPTED_AND_REPAIRED`: final-review dispatch and reviewer resolve the live target and require exact equality with `qualified_head`; mismatch fails closed as stale qualified head.
2. `4015903521` — inventory digest did not bind repository/task/PR/closure generation. `ACCEPTED_AND_REPAIRED`: the RFC 8785 envelope binds repository, Issue, task, PR, branch, Phase-1 closure head and tree.
3. `4015996290` — lifecycle registry `1.3` did not match auditor embedded `prompt_version: "1.2"`. `ACCEPTED_AND_REPAIRED`: task custody was extended first, then the embedded version was aligned to `1.3` without authority/behavior change.

The resulting exact head `dae61ae0ff8b806dc66283c6de947aa76f670103` passed Agent Governance `34974838822`, Architecture Semantic Audit `34974435113`, and Merge Gate `34974838856` before the next final review.

### Independent review generation 6

Fresh Codex review `5210541937` on exact qualified head `dae61ae0ff8b806dc66283c6de947aa76f670103` reported two P1 findings:

1. `4016077633` — discovery sweep identity omitted the frozen protected-`main` generation, allowing the same PR head to be swept against a newer set of contracts. `ACCEPTED_AND_REPAIRED` in `928be692241791930b49711424950e8e9e83800f`: Phase 1 + discovery descriptor now bind `base_main_sha`; the auditor must independently resolve `audit_main_sha` and require equality before sweep; the immutable inventory sweep target carries both; final review detects later protected-main movement and fails closed on material drift.
2. `4016077641` — frozen inventory had no conforming transition after successful evidence reconciliation because changing `evidence_classification`/`repair_eligibility` would invalidate its digest. `ACCEPTED_AND_REPAIRED` in `928be692241791930b49711424950e8e9e83800f`: frozen inventories are never edited in place; one immutable RFC 8785/SHA-256 successor is created per coherent reconciliation generation, links its predecessor identity, preserves the exact sweep target, and becomes the exact active generation for repair/qualification/final review.

This task-record update is part of the same coherent review-repair generation and introduces no new protocol behavior beyond recording the accepted repairs.

## Validation

### Focused

- changed-file inventory vs `main`: PASS — only task-owned documentation/prompt/task/lifecycle paths.
- prompt semantic self-review: PASS after review repair — no authority expansion, no review weakening, no Merge Queue semantic change, no product/runtime mutation.
- META 3.1 prompting-standard reconciliation: PASS — one routed shared protocol with prompt-specific deltas; no full global procedure copied into every prompt.
- lifecycle integrity self-review: PASS — the three intentionally revised prompt versions and self-declared metadata are consistent; all other lifecycle identity/status/scope/supersession metadata is preserved.
- generation-6 repair scope: PASS — protocol semantic repair plus canonical task-record reconciliation only; #356 remains outside changed paths.

### Component/integration

- prior exact-head qualification on `dae61ae0ff8b806dc66283c6de947aa76f670103`: Agent Governance `34974838822` SUCCESS; Architecture Semantic Audit `34974435113` SUCCESS; Merge Gate `34974838856` SUCCESS. This evidence became historical after generation-6 repair.
- fresh exact-head repository checks: required on the final repair head.

### E2E

- scenario: NOT_APPLICABLE — prompt/governance documentation only
- result: NOT_APPLICABLE

### Exact-head CI

- final head: record from PR readback after this repair-generation task record commit
- trigger source: pull_request
- workflow/run/job: pending fresh exact-head generation
- runner assignment: pending
- classification: pending
- result: pending

## Self-review

- exact head: record from PR readback after this repair-generation task record commit
- method/reviewer: implementing agent whole-diff review against #624 + bound META 3.1 + all accepted review findings
- material findings: 0 open from the implementing-agent pass after generation-6 repair
- verdict: PASS_PENDING_FRESH_EXACT_HEAD_CI_AND_FINAL_INDEPENDENT_REVIEW

## Independent review

- required: YES — material control-plane/governance behavior under bound META AI review policy
- historical generation 1: `5209285222` / head `7abaa1fa...` / 3 P1, dispositioned above
- historical generation 5: `5210336765` / head `a3de0bcd...` / 2 P1 + follow-up P2, dispositioned above
- historical generation 6: `5210541937` / head `dae61ae0...` / 2 P1, dispositioned above
- exact final head: record after this repair-generation task record commit
- method/auditor: one fresh Codex deep review after deterministic validation
- material findings: pending
- verdict: pending

## PR and closeout

- PR: #625
- changed-file review: PASS pre-final-freeze
- unresolved review threads: generation-6 findings must be replied/resolved against exact repair evidence before final independent review
- related/superseded PRs: none
- protected integration: NOT_AUTHORIZED_BY_TASK
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: Repaired generation-6 protected-main binding and immutable successor-inventory findings; reconciled canonical task record.
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
repair_cycles_for_current_gate: 4
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: Resolve generation-6 review threads with exact commit evidence, freeze the resulting exact head, run fresh exact-head CI, then obtain one fresh independent deep review without moving code or metadata.
```
