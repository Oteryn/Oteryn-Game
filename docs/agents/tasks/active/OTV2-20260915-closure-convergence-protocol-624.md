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
updated_at: 2026-09-15T12:15:11Z
execution_policy: continuous_progress
owned_paths:
  - docs/agents/AGENTS.md
  - docs/agents/CLOSURE_CONVERGENCE_PROTOCOL.md
  - docs/agents/PROMPT_LIFECYCLE.json
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
- `PROVEN`: the convergence changes materially revise three reusable prompt contracts, so `PROMPT_LIFECYCLE.json` advances `OTV2_WORK_DELIVERY_COORDINATOR` from `1.2` to `1.3`, `OTV2_WORK_DELIVERY_INDEPENDENT_AUDITOR` from `1.2` to `1.3`, and `OTV2_IMPL_DURABILITY` from `1.1` to `1.2` while preserving all IDs, owners, statuses, scopes and supersession relations.

## High-risk authority/recovery qualification

```yaml
applicable: NOT_APPLICABLE
reason: Documentation/prompt governance only; no production mutation, authority-bearing session replacement, PREPARE/COMMIT, persisted recovery interpretation, runtime/database mutation, or protected integration is performed by this task.
```

## Acceptance criteria

- [x] Add one routed closure convergence protocol with explicit activation, one-shot sweep, finding classification, root-cause collapse, frozen inventory, coherent repair generation, final qualification/review and anti-drip novelty triggers.
- [x] Bind discovery sweeps to the exact frozen closure head and fail closed if the live target moved.
- [x] Keep `UNKNOWN`/`CONFLICT` material findings out of mutating repair generations until evidence is reconciled; only repair-eligible material blockers enter the batch.
- [x] Bind every frozen root-cause inventory to an RFC 8785 canonical JSON SHA-256 identity, carry the locator and identity through the checkpoint/final-review descriptor, and fail closed on missing identity or content drift before repair dispatch or final review.
- [x] Add publication-safety fail-closed behavior for unavailable normal Git publication; no direct low-level Git-object fallback on canonical material branches.
- [x] Work coordinator routes convergence-mode workers and auditors through the protocol and treats a bounded task as a bounded coherent repair generation during closure.
- [x] Convergence audit dispatch fits the coordinator's existing minimal packet by using an `accepted_decisions` convergence descriptor rather than new top-level keys.
- [x] Durability implementer consumes frozen root-cause batches and does not stop after the first compatible blocker.
- [x] Independent auditor supports `DISCOVERY_SWEEP` and `FINAL_CANDIDATE_REVIEW` through nearest `docs/agents/AGENTS.md` + the routed protocol, without gaining implementation authority.
- [x] Auditor evidence `classification: PROVEN | DERIVED | UNKNOWN | CONFLICT` remains separate from convergence `gate_classification: MATERIAL_BLOCKER | EVIDENCE_GAP | HARDENING | OUT_OF_SCOPE`.
- [x] A late `FINAL_SWEEP_MISS` is an additional sweep disposition and never replaces/downgrades `gate_classification: MATERIAL_BLOCKER` for a real current-gate defect.
- [x] Lifecycle registry versions advance for the three reusable prompt contracts whose behavior changed.
- [x] No change to #356 material source, runtime/product behavior, workflows, rulesets, Merge Queue semantics, Platform/Atlas/META or production state.

## Excluded scope

No WP3 product repair, no #356 mutation, no Cargo/runtime source, no workflow/ruleset/protection change, no Merge Queue submission, no external repository write.

## Implementation / findings

- Added `docs/agents/CLOSURE_CONVERGENCE_PROTOCOL.md` as the single reusable closure procedure rather than duplicating the full flow in every prompt.
- Routed convergence activation/audit modes through `docs/agents/AGENTS.md` so the existing independent auditor consumes the modes through its mandatory nearest-instruction startup path.
- Added a compact convergence-mode delta to `OTV2_WORK_DELIVERY_COORDINATOR.md`.
- Added consolidated-repair and publication-safety deltas to `OTV2_IMPL_DURABILITY.md`.
- Advanced lifecycle metadata only for the three changed reusable prompt contracts.

### Independent review generation 1

Codex review `5209285222` was bound to historical head `7abaa1fa2ad779e821435888549fcd61e686890c` and reported three P1 findings:

1. `4014990995` — task custody omitted `docs/agents/AGENTS.md`. `STALE_ON_CURRENT_LINEAGE`: already repaired before review publication by task-packet commit `b9ae8016583d4712cad5817dc8d7a274ed611d09`, which added `docs/agents/AGENTS.md` and removed the unmodified auditor prompt from `owned_paths`.
2. `4014991004` — convergence audit parameters conflicted with the coordinator's mandatory minimal context packet. `ACCEPTED_AND_REPAIRED`: convergence dispatch now uses one structured descriptor inside the existing `accepted_decisions` field; no new top-level packet keys.
3. `4014991011` — convergence reused `classification` for gate disposition and conflicted with auditor evidence classification. `ACCEPTED_AND_REPAIRED`: evidence classification remains `PROVEN | DERIVED | UNKNOWN | CONFLICT`; convergence now uses separate `gate_classification`.

### Independent review generation 2

Fresh Codex review sequence on exact head `b8e54b146dda34459aa8725ab2d9553c5968f64a` reported four findings, all consumed into one coherent review-repair generation before final freeze:

1. `4015090632` — P1: Durability prompt presented `FINAL_SWEEP_MISS` as an alternative classification. `ACCEPTED_AND_REPAIRED`: a real current-gate defect remains `gate_classification: MATERIAL_BLOCKER`; `sweep_disposition: FINAL_SWEEP_MISS` is additional metadata only.
2. `4015090638` — P2: lifecycle versions did not advance for materially revised reusable prompt contracts. `ACCEPTED_AND_REPAIRED`: coordinator `1.2 -> 1.3`, durability `1.1 -> 1.2`, and lifecycle path added to task custody.
3. `4015189472` — P1: discovery sweep descriptor was not bound to the frozen closure SHA. `ACCEPTED_AND_REPAIRED`: `DISCOVERY_SWEEP` now carries exact `closure_head`, and the auditor must fail closed if the live target differs.
4. `4015189486` — P1: `UNKNOWN`/`CONFLICT` evidence could still drive mutation merely because gate classification was `MATERIAL_BLOCKER`. `ACCEPTED_AND_REPAIRED`: root causes now carry repair eligibility; unresolved evidence cannot enter a mutating repair generation until reconciled.

This complete material review-repair generation requires fresh exact-head deterministic validation and one final independent review on the resulting stable head.

### Independent review generation 3

Final Codex review on exact head `bdd67024fc3aaea880e66864fb8aa92547625e0d` reported one P2: the convergence-aware independent auditor contract remained lifecycle `1.2`. `ACCEPTED_AND_REPAIRED`: `OTV2_WORK_DELIVERY_INDEPENDENT_AUDITOR` advances `1.2 -> 1.3`; its prompt ID, owner, status, scope and supersession metadata remain unchanged. No prompt, runtime, workflow, ruleset or integration semantics change in this bounded repair.

### Independent review generation 4

Final Codex review on exact head `bf0a44bcef08900e7754683ba572c6057df06aa1` reported one P1: the frozen inventory's editable GitHub evidence-note locator did not prove immutable inventory content. `ACCEPTED_AND_REPAIRED`: each `FINAL_ROOT_CAUSE_INVENTORY` now carries an RFC 8785 canonical JSON SHA-256 content identity alongside the editable evidence locator; the checkpoint and final-review descriptor preserve both, and exact-match verification fails closed before repair dispatch and final review. The evidence note remains a locator/evidence surface only.

## Validation

### Focused

- changed-file inventory vs `main`: PASS — only the six owned documentation/prompt/task paths above.
- prompt semantic self-review: PASS after review repair — no authority expansion, no review weakening, no Merge Queue semantic change, no product/runtime mutation.
- META 3.1 prompting-standard reconciliation: PASS — one routed shared protocol with prompt-specific deltas; no full global procedure copied into every prompt.
- lifecycle integrity self-review: PASS — only the three intentionally revised prompt versions changed; all other lifecycle identity/status/scope/supersession metadata is preserved.

### Component/integration

- exact-head Agent Governance on `b8e54b...`: SUCCESS before generation-2 repair.
- exact-head Architecture Semantic Audit on `b8e54b...`: SUCCESS before generation-2 repair.
- exact-head Merge Gate on `b8e54b...`: SUCCESS before generation-2 repair.
- fresh exact-head repository checks: required after the complete generation-2 repair.

### E2E

- scenario: NOT_APPLICABLE — prompt/governance documentation only
- result: NOT_APPLICABLE

### Exact-head CI

- final head: record in immutable PR/check evidence after this commit
- trigger source: pull_request
- workflow/run/job: pending
- runner assignment: pending
- classification: pending
- result: pending

## Self-review

- exact head: record in immutable PR evidence after this commit
- method/reviewer: implementing agent whole-diff review against #624 + bound META 3.1 + all accepted review findings
- material findings: 0 open in producer self-review
- verdict: PASS_PENDING_EXACT_HEAD_CI_AND_FINAL_INDEPENDENT_REVIEW

## Independent review

- required: YES — material high-risk/control-plane governance behavior under bound META AI review policy
- historical generation 1: `5209285222` / head `7abaa1fa...` / 3 P1, dispositioned above
- historical generation 2: head `b8e54b...` / 3 P1 + 1 P2, dispositioned above
- exact final head: record in immutable PR evidence after this commit
- method/auditor: one fresh Codex deep review after deterministic validation
- material findings: pending
- verdict: pending

## PR and closeout

- PR: #625
- changed-file review: PASS pre-final-freeze
- unresolved review threads: final-review auditor lifecycle P2 to be replied/resolved against the published repair head
- related/superseded PRs: none
- protected integration: NOT_AUTHORIZED_BY_TASK
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: Repaired final-review P1 by binding frozen inventories to canonical immutable content identities and requiring fail-closed exact-match verification.
status: validating
branch: agent/closure-convergence-protocol
head_sha: record_after_commit
pr: 625
final_head_sha: record_in_pr_evidence
final_head_frozen_at: after_this_commit
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
repair_cycles_for_current_gate: 2
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: Freeze this exact head, resolve all historical review threads with exact evidence, run fresh exact-head CI, then obtain one final independent deep review without moving the head.
```
