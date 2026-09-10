# OTV2-20260910-reference-investigation-prompts-486

```yaml
task_id: OTV2-20260910-reference-investigation-prompts-486
title: Register reusable Reference investigation prompt and operator runbook
mode: GOVERNANCE
status: ready
repository: Oteryn/Oteryn-Game
issue: 486
base_branch: main
branch: docs/reference-investigator-486
pr: 527
base_sha: 43ff3341e079f2883b78d01db5cee649290d90be
head_sha: null
final_head_sha: null
final_head_frozen_at: 2026-09-10T09:43:00+02:00
owner: OTV2_WORK_DELIVERY_COORDINATOR
created_at: 2026-09-10T08:51:00+02:00
updated_at: 2026-09-10T09:43:00+02:00
execution_policy: continuous_progress
owned_paths:
  - docs/agents/prompts/OTV2_REFERENCE_INVESTIGATOR.md
  - docs/agents/PROMPT_LIFECYCLE.json
  - docs/agents/prompts/README.md
  - docs/agents/programs/OTERYN_REFERENCE_INVESTIGATION_SOURCE_REGISTRY_20260910.md
  - docs/agents/programs/OTERYN_REFERENCE_INVESTIGATION_OPERATOR_RUNBOOK_20260910.md
  - docs/agents/tasks/active/OTV2-20260910-reference-investigation-prompts-486.md
public_contracts: []
depends_on:
  - Oteryn/Oteryn-Game#162
  - Oteryn/Oteryn-Game#486
  - Oteryn/Oteryn-Game#483
blocks: []
cross_repository_coordination_id: null
external_repositories:
  - public CipSoft/Tibia sources (READ_ONLY)
  - tibiawiki.com.br (READ_ONLY_STRUCTURED_REFERENCE_DATA)
  - other public Tibia encyclopedias/databases (READ_ONLY_STRUCTURED_REFERENCE_DATA)
  - historical/Canary/Crystal/OTS repositories (READ_ONLY_HYPOTHESIS)
```

## Outcome

Register one reusable read-only Reference investigator prompt with eight short lane aliases, a shared source/evidence registry and an owner-facing launch/effort runbook so programme #486 can research Reference data in parallel without duplicating long prompts or creating a second control plane.

## Architecture and source of truth

- **PROVEN:** protected `main@43ff3341e079f2883b78d01db5cee649290d90be` is the admission base for this docs-only delivery.
- **PROVEN:** #162 / `OTV2_WORK_DELIVERY_COORDINATOR` remains the current active control plane; the new investigator prompt is read-only and cannot allocate/integrate/mutate gameplay work.
- **PROVEN:** `docs/agents/PROMPTING_STANDARD.md` requires every reusable prompt to be registered in `PROMPT_LIFECYCLE.json`; alias invocation grants no write authority.
- **PROVEN:** programme #486 already decomposes Reference-first work across R0-R10 and requires official/primary evidence before OTS hypotheses.
- **DERIVED:** Tibia Wiki (`tibiawiki.com.br`) is treated operationally as a first-class high-volume structured data source for bulk content extraction; exact evidence strength remains field-specific and below stronger conflicting official/controlled evidence.
- **DERIVED:** one parameterized prompt plus lane aliases creates less prompt-governance duplication than seven or eight separate prompt files while preserving lane-specific contracts.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: this delivery changes only reusable read-only prompt/orchestration documentation. It authorizes no runtime mutation, persistence/value operation, PREPARE/COMMIT, authority-bearing session/fence, production action or recovery interpretation.

## Acceptance criteria

- [x] One reusable prompt `OTV2_REFERENCE_INVESTIGATOR` exists with aliases `Oteryn: ref world|combat|char|npc|move|durability|evidence|qa`.
- [x] Alias is explicitly read-only and cannot become a second #162 control plane or implementation writer.
- [x] Source registry separates Oteryn project truth from external Reference evidence strength.
- [x] CipSoft official remains primary external evidence for exact rules/chronology/conflicts.
- [x] Tibia Wiki is first-class `STRUCTURED_REFERENCE_DATA` for bulk content extraction and field-level cross-check without automatic `PROVEN` promotion.
- [x] Canary/Crystal/legacy OTS remain `OTS_HYPOTHESIS_ONLY` regardless of cross-OTS consensus.
- [x] Target cut remains the accepted post-2026-07-28 Global boundary; post-target data requires continuity analysis.
- [x] Operator runbook gives exact recommended aliases, wave ordering and effort levels.
- [x] Existing #511/#525, #507, WP3/#356, WP4/#335 and other implementation lineages are not mutated or replaced.
- [x] `PROMPT_LIFECYCLE.json` registers the new prompt exactly once; Agent Governance generation on the pre-freeze content passed after the PR metadata heading repair.
- [x] Prompt README exposes the new reusable alias family.
- [ ] Final exact-head repository Agent Governance / Architecture Semantic Audit / Merge Gate are green; record externally on the PR after this freeze commit.
- [x] Whole-diff self-review found no open material P0/P1/P2 finding.

## Excluded scope

No runtime/client/server/Content/Ability/Combat/AI/Character/Item/NPC/Movement/Durability implementation, resource-registry change, Cargo/workflow/ruleset/protection change, external-repository write, proprietary data import, production/live-data access or Oteryn Evolved activation.

No existing reusable implementation/lane-lead prompt is superseded. The investigator prompt is an additive read-only research accelerator for #486.

## Implementation / findings

Owner direction selected a practical evidence model:

```text
PROJECT TRUTH
  GitHub LIVE -> protected accepted Oteryn evidence/contracts

REFERENCE EVIDENCE
  CipSoft official
  + controlled Global observation
  + Tibia Wiki as first-class structured bulk data
  + other structured Tibia data for cross-check
  + reputable community corroboration
  + historical Oteryn migration evidence
  + Canary/Crystal/OTS as OTS_HYPOTHESIS_ONLY
```

Structured/wiki data is decomposed to atomic fields. Wiki/OTS consensus may efficiently create explicit `DERIVED` candidates when target continuity is reasoned and no stronger conflict exists; it cannot silently produce `PROVEN`. Subtle runtime mechanics use the slower official/controlled-observation path.

### Whole-diff self-review

Reviewed the complete intended six-path docs/governance delta against the admission base. Falsified for: creating a second coordinator; implicit tracked-file or external-repository write authority; implementation-worker replacement; OTS-to-Reference promotion; wiki-only automatic `PROVEN`; post-target current-Global target drift; paid-review authorization; runtime/production authority; Evolved leakage; and alias ambiguity.

Result:

```text
P0=0
P1=0
P2=0
VERDICT=PASS
```

### Independent review routing

Bound META `docs/governance/AI_REVIEW_POLICY.md` routes default/low-risk documentation to no external AI review and reserves mandatory deep review for material high-risk/control-plane changes that can alter autonomous write/integration authority. This additive prompt explicitly grants no such authority and does not change workflows, permissions, protection, rulesets, required checks, Merge Queue or production authority.

```text
INDEPENDENT_EXTERNAL_AI_REVIEW=NO
REASON=LOW_RISK_READ_ONLY_PROMPT_AND_DOCUMENTATION; DETERMINISTIC_GOVERNANCE_AND_SELF_REVIEW_APPLY
```

No owner-funded/quota AI is consumed for this candidate.

## Validation

### Focused

- command/run: repository Agent Governance validates prompt lifecycle/discovery/policy on PR exact head
- result: pre-freeze content generation PASS; final frozen-head generation pending externally

### Component/integration

- command/run: Architecture Semantic Audit + repository Merge Gate selected by changed paths
- result: final frozen-head generation pending externally

### E2E

- scenario: `NOT_APPLICABLE` — documentation/read-only research prompt only
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: intentionally recorded in immutable PR/check evidence after this commit exists; a commit cannot contain its own SHA
- trigger source: pull_request
- workflow/run/job: pending final frozen-head generation
- runner assignment: repository-hosted/selected by workflows
- classification: docs/governance
- result: pending external exact-head evidence

## Self-review

- exact head: final SHA to be bound externally after this freeze commit exists
- method/reviewer: coordinating author, complete changed-file/whole-diff challenge
- material findings: P0=0 / P1=0 / P2=0
- verdict: PASS

## Independent review

- required: NO — bound META risk policy; read-only docs/prompt with zero autonomous write/integration authority
- exact head: NOT_APPLICABLE
- method/auditor: NOT_APPLICABLE
- material findings: NOT_APPLICABLE
- verdict: NOT_APPLICABLE

## PR and closeout

- changed-file review: PASS — exactly six allocated docs/governance paths
- unresolved review threads: verify externally before integration
- related/superseded PRs: none for this prompt family at preflight
- protected Merge Queue: required if/when exact candidate integration is authorized
- merge commit/result: pending
- ownership release: after protected integration/readback

## Context checkpoint

```yaml
last_progress: final intended six-path prompt/source-registry/operator-runbook/lifecycle/README/task candidate frozen; remaining qualification is external exact-head CI/MQ/readback
status: ready
branch: docs/reference-investigator-486
head_sha: external exact-head PR evidence after freeze commit
pr: 527
final_head_sha: external exact-head PR evidence after freeze commit
final_head_frozen_at: 2026-09-10T09:43:00+02:00
ci_trigger_source: pull_request
ci_check_generation: final frozen-head generation pending
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: repository_selected
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: require final frozen-head governance/semantic/merge-gate success, then protected Merge Queue and protected-main readback
```
