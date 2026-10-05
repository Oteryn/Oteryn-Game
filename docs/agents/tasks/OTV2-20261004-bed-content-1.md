# OTV2-20261004-bed-content-1

```yaml
task_id: OTV2-20261004-bed-content-1
title: BED-CONTENT-1 bed facts, validator and exception list
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/bed-content-1-20261005
pr: null
base_sha: null
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: BED-CONTENT-1 content-lane worker
created_at: 2026-10-05T00:00:00Z
updated_at: 2026-10-05T00:00:00Z
execution_policy: continuous_progress
owned_paths:
  - tools/content-schema/item-authoring/lower_bed_packet.py
  - tools/content-schema/item-authoring/test_lower_bed_packet.py
  - tools/content-schema/house-authoring/validate_beds.py
  - tools/content-schema/house-authoring/test_validate_beds.py
  - tools/content-schema/house-authoring/bed-exception-houses.json
  - docs/agents/evidence/OTV2-20261005-bed-facts-v1.json
  - docs/agents/tasks/OTV2-20261004-bed-content-1.md
public_contracts: []
depends_on: [ITEM-SEM-BED-1]
blocks: [BED-1]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

BED-0 §3 content half: bed facts lowered from the pinned Canary `items.xml`, a House bed validator, and the 84-house exception list. No runtime reads the facts before BED-1.

## Architecture and source of truth

- BED-0 §3 (`docs/architecture/reviews/OTERYN_GAME_BED0_HOUSE_BEDS_DECISION_2026-10-01.md`), packet §2.2. DERIVED.
- Canary `items.xml` pin D384 is `OTS_HYPOTHESIS_ONLY`. Its transform links are mutual, so free/occupied types are not derivable from it. UNKNOWN until an architect rule or the placed map.
- CP D784 split the `bed` semantics group off as ITEM-SEM-BED-1 (architect). This task does not touch Rust or the item schema and does not freeze until it merges.

## Acceptance criteria

- [x] `lower_bed_packet.py` plus tests; `--check` reproduces the packet (377 rows, 39 holds).
- [x] `validate_beds.py` plus fixture-house tests; the exception list equals the 84 ids of `samples/otbm-tile-check.json`.
- [ ] After ITEM-SEM-BED-1: `bed` facts in the bed definitions; freeze once.

## Excluded scope

Rust, `item.schema.json`, `content/world/**`, runtime reads, BED-1.

## Implementation / findings

Packet rows carry part, partner direction, partner key and raw transform targets; unresolved facts are `null` and listed under `holds`.

## Validation

### Focused

- command/run: `python test_lower_bed_packet.py`, `python lower_bed_packet.py --check`, `python test_validate_beds.py`
- result: pass

### Component/integration

- command/run: NOT_APPLICABLE until the definitions carry `bed`
- result: pending

### E2E

- scenario: NOT_APPLICABLE, no runtime reader before BED-1
- result: pending

### Exact-head CI

- final head: pending
- trigger source: pending
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
- protected auto-merge: not enabled
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: validator, exception list and bed packet tool authored
status: implementing
branch: claude/bed-content-1-20261005
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
blocker: ITEM-SEM-BED-1 not merged
next_action: wait for ITEM-SEM-BED-1, then lower bed facts into definitions and freeze
```
