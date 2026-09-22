> Lifecycle closeout: **COMPLETED / ARCHIVED / OWNERSHIP RELEASED**. Delivery PR #736 exact candidate `bea9b1f130b4ab0cbf8e3a6d3d3b4401669881a2` passed real Merge Queue run `35656145580` with terminal `game-gate` SUCCESS and integrated as protected main `0bbaa898e5d5a33f844786054e8c7e82148b6180`. Source closure is `825/825`; native/executable Ability closure remains separate. Any nonterminal/checkpoint wording below is historical provenance only.

# OTV2-20260921-content-world-cw2-ability-family-source-catalogue-708

```yaml
task_id: OTV2-20260921-content-world-cw2-ability-family-source-catalogue-708
title: CW2 Ability whole-family source catalogue
mode: MIGRATE
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/content-world-cw2-ability-family-source-catalogue-708
issue: 708
pr: 736
pr: null
base_sha: fd9dcb55a0ea55d2d20488f80abccf8f6c8ec875
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: "Oteryn: content world import"
created_at: 2026-09-21T18:10:00Z
updated_at: 2026-09-21T20:03:00Z
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

Verified source closure:

`ABILITIES_TOTAL=825 / SOURCE_CATALOGUED=825 / SOURCE_REMAINING=0`.

This is source closure only. Native/executable Ability closure remains unresolved
and no legacy formula/effect value is promoted to Reference truth.

## Architecture and source of truth

- **PROVEN** — release: issue #708 comment 5765243741.
- **PROVEN** — scheduler preflight: issue #162 comment 5765163790.
- **PROVEN** — pinned source:
  `blakinio/Otheryn@e417c5e7c22986bf4acef0495eb47f7b72c97cce`.
- **PROVEN** — branch admitted on fresh path-disjoint protected
  `main@fd9dcb55a0ea55d2d20488f80abccf8f6c8ec875`.
- **PROVEN** — historical B4 evidence remains exact blob
  `56b8e4b1d143cc9a68aa691aa30a292d4befe2c3`; historical mapper remains exact
  blob `e6d98aadd352ad36b466970e1f7182e1bf93643b`.

## High-risk authority/recovery qualification

NOT_APPLICABLE — offline read-only migration/source evidence; no production,
authority-bearing, persisted-state or runtime mutation is performed.

## Acceptance criteria

- [x] Exactly 825 production source records are catalogued.
- [x] Partition arithmetic closes at 199 player / 36 rune / 590 monster.
- [x] Every source record retains exact repository/revision/path/blob/byte-size provenance.
- [x] Duplicate selected source paths fail closed in the mapper/self-test.
- [x] `gaz_functions.lua` and `data/scripts/spells/#example.lua` are exact exclusions.
- [x] Historical B4 overlay joins exactly two source records without duplicate counting.
- [x] Repeated generation and input-order permutation are enforced by the self-test.
- [x] Pinned revision/blob drift fails closed in the mapper/self-test.
- [x] No `oteryn:ability.*` identity, Reference formula/effect/parity promotion,
      runtime or shared-model write is introduced.
- [x] Existing historical two-record B4 evidence remains byte-identical.
- [ ] Exact-head hosted checks pass.

## Excluded scope

No changes under `apps/game-server/src/content/**`,
`apps/game-server/src/ability/**`, `apps/game-server/tests/**`,
`docs/contracts/**`, workflows, contracts, CW3/#722 paths, external repositories
or production. No new importer/framework/model.

## Implementation / findings

The existing B4 catalogue module now owns a whole-family source-catalogue mode.
It enumerates the exact seven player-spell roots, rune root and monster-spell root
at the pinned Otheryn revision, excludes the exact helper/example objects, binds
every source object by Git blob and byte size, and joins the protected B4 evidence
only to Ice Strike and Light Healing.

Independent live GitHub source-set readback verified:
- player spells: 199 = 69 attack + 28 healing + 39 support + 49 conjuring +
  5 party + 5 familiar + 4 house;
- runes: 36;
- monster root: 591 direct Lua files = 590 production + exactly one
  `gaz_functions.lua` helper;
- tracked product: missing source paths = 0, extra source paths = 0;
- overlay join = 2;
- silent drops = 0.

The protected historical B4 JSON is not regenerated or rewritten.

## Validation

### Focused

- local syntax proof for the exact published tool bytes:
  `python -m py_compile ability_catalog.py ability_catalog_self_test.py` — PASS;
- independent AST parse of both exact local files — PASS;
- local `git hash-object` matched live GitHub blobs exactly:
  - mapper: `e41e4d39a3a312a4086b1b3363afb2727b20eedf`;
  - self-test: `69faaec20691dd07e7766cfd8a294e7a0f59d168`;
- live GitHub source-set comparison against pinned Otheryn — PASS:
  `235/235` player+rune paths and `590/590` monster production paths,
  missing=[], extra=[];
- tracked evidence readback — PASS:
  `ABILITIES_TOTAL=825 / SOURCE_CATALOGUED=825 / SOURCE_REMAINING=0`,
  overlay join=2.

The complete source-backed Python self-test requires a local Git checkout of the
pinned external source. This execution surface cannot resolve github.com from its
isolated container, so that command is not claimed PASS here; hosted/external
validation remains pending rather than being fabricated.

### Component/integration

- B4 runtime adapter and tests are unchanged by changed-file readback.
- historical B4 evidence blob remains exact and mapper verifies its historical
  mapper fingerprint rather than rewriting it.

### E2E

- scenario: NOT_APPLICABLE — offline source evidence only.
- result: NOT_APPLICABLE.

### Exact-head CI

- final head: pending freeze after this task-record update.
- trigger source: PR exact head.
- workflow/run/job: pending.
- runner assignment: pending.
- classification: pending.
- result: pending.

## Self-review

- method/reviewer: implementing agent.
- material findings: none in four-path whole-diff/source-set review.
- verdict: candidate is source-complete; exact-head hosted validation pending.

## Independent review

- required: pending risk-policy/readback after exact-head checks.
- exact head: pending.
- method/auditor: pending or NOT_APPLICABLE.
- material findings: pending or NOT_APPLICABLE.
- verdict: pending or NOT_APPLICABLE.

## PR and closeout

- changed-file review: exactly four allocated paths.
- unresolved review threads: pending PR.
- related/superseded PRs: none.
- protected auto-merge: NOT_AUTHORIZED_BY_WORKER.
- merge commit/result: NOT_PERFORMED.
- ownership release: pending.

## Context checkpoint

```yaml
last_progress: full 825-record source closure and mapper/evidence readback complete
status: validating
branch: agent/content-world-cw2-ability-family-source-catalogue-708
head_sha: null
pr: null
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: PR
ci_check_generation: pending
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
blocker: source-backed self-test cannot execute on current isolated container; exact-head hosted checks pending
next_action: freeze live branch head, create one canonical PR, and reconcile exact-head checks
```
