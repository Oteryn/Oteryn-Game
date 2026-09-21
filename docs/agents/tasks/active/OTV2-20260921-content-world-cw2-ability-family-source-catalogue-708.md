# OTV2-20260921-content-world-cw2-ability-family-source-catalogue-708

```yaml
task_id: OTV2-20260921-content-world-cw2-ability-family-source-catalogue-708
title: CW2 Ability whole-family source catalogue
mode: MIGRATE
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/content-world-cw2-ability-family-source-catalogue-708
pr: null
base_sha: fd9dcb55a0ea55d2d20488f80abccf8f6c8ec875
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: "Oteryn: content world import"
created_at: 2026-09-21T18:10:00Z
updated_at: 2026-09-21T18:10:00Z
execution_policy: continuous_progress
owned_paths:
  - tools/reference-world-corridor-census/ability_effect_formula_evidence_catalog.py
  - tools/reference-world-corridor-census/ability_effect_formula_evidence_catalog_self_test.py
  - docs/agents/evidence/OTV2-20260921-content-world-cw2-ability-family-source-catalogue.json
  - docs/agents/tasks/active/OTV2-20260921-content-world-cw2-ability-family-source-catalogue-708.md
public_contracts: []
depends_on:
  - issue: 708
  - issue: 162
blocks: []
cross_repository_coordination_id: null
external_repositories:
  - blakinio/Otheryn@e417c5e7c22986bf4acef0495eb47f7b72c97cce
```

## Outcome

Catalogue the complete finite Ability source family in one partition: 199 player
spell modules, 36 rune modules and 590 registered monster-spell modules.
Target closure is `ABILITIES_TOTAL=825 / SOURCE_CATALOGUED=825 /
SOURCE_REMAINING=0`.

This is source closure only. Native/executable Ability closure remains unresolved
and no legacy formula/effect value is promoted to Reference truth.

## Architecture and source of truth

- **PROVEN** — release: issue #708 comment 5765243741.
- **PROVEN** — scheduler preflight: issue #162 comment 5765163790.
- **PROVEN** — pinned source:
  `blakinio/Otheryn@e417c5e7c22986bf4acef0495eb47f7b72c97cce`.
- **PROVEN** — the release base `22e3ba83...` advanced path-disjointly to
  protected `main@fd9dcb55a0ea55d2d20488f80abccf8f6c8ec875` before authoring;
  none of the four owned paths changed.
- **PROVEN** — existing B4 evidence remains historical overlay input only and
  must remain byte-identical.

## High-risk authority/recovery qualification

NOT_APPLICABLE — offline read-only migration/source evidence; no production,
authority-bearing, persisted-state or runtime mutation is performed.

## Acceptance criteria

- [ ] Exactly 825 production source records are catalogued.
- [ ] Partition arithmetic closes at 199 player / 36 rune / 590 monster.
- [ ] Every source record retains exact repository/revision/path/blob/byte provenance.
- [ ] Duplicate selected source paths fail closed.
- [ ] `gaz_functions.lua` and `data/scripts/spells/#example.lua` are exact exclusions.
- [ ] Historical B4 overlay joins exactly two source records without duplicate counting.
- [ ] Repeated assembly and input-order permutation are deterministic.
- [ ] Pinned revision/blob drift fails closed.
- [ ] No `oteryn:ability.*` identity, Reference formula/effect/parity promotion,
      runtime or shared-model write is introduced.
- [ ] Existing historical two-record B4 evidence remains byte-identical.
- [ ] Exact-head hosted checks pass.

## Excluded scope

No changes under `apps/game-server/src/content/**`,
`apps/game-server/src/ability/**`, `apps/game-server/tests/**`,
`docs/contracts/**`, workflows, contracts, CW3/#722 paths, external repositories
or production. No new importer/framework/model.

## Implementation / findings

Whole-family source catalogue is being added by extending the existing B4
catalogue module. Intermediate API-authoring commits are WIP and carry no
candidate-specific qualification evidence.

## Validation

### Focused

- command/run: pending
- result: pending

### Component/integration

- command/run: existing historical B4 regression + governance
- result: pending

### E2E

- scenario: NOT_APPLICABLE — offline source evidence only
- result: NOT_APPLICABLE

### Exact-head CI

- final head: pending
- trigger source: PR exact head
- workflow/run/job: pending
- runner assignment: pending
- classification: pending
- result: pending

## Self-review

- exact head: pending
- method/reviewer: implementing agent
- material findings: pending
- verdict: pending

## Independent review

- required: pending
- exact head: pending
- method/auditor: pending
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: none
- protected auto-merge: NOT_AUTHORIZED_BY_WORKER
- merge commit/result: NOT_PERFORMED
- ownership release: pending

## Context checkpoint

```yaml
last_progress: authoring branch admitted on fresh path-disjoint protected main
status: implementing
branch: agent/content-world-cw2-ability-family-source-catalogue-708
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
next_action: enumerate and bind all 825 pinned source objects
```
