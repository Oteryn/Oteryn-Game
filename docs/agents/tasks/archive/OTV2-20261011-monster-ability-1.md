# OTV2-20261011-monster-ability-1

```yaml
task_id: OTV2-20261011-monster-ability-1
title: MONSTER-ABILITY-1 monster attack runtime-source coverage gate
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/monster-ability-runtime-20261011
pr: 1982
base_sha: null
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: MONSTER-ABILITY-1 worker (CP session_01CwP6d84eCPvpgoEuyci8Tx, #1622)
created_at: 2026-10-11T12:00:00Z
updated_at: 2026-10-11T14:00:00Z
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/monster_combat_lane.rs
  - docs/agents/tasks/archive/OTV2-20261011-monster-ability-1.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

A test-only gate pins which authored monster attacks have a runtime source constructor in the monster combat lane. No runtime behavior changes.

## Architecture and source of truth

- PROVEN: the lane already executes monster abilities through `MeleeSource::from_native` and `SpellSource::from_native` plus special owners (composed, chain, encounter-seven, summon, heals); the CP decision for this task is option (c) of #1622, a data-coverage gate first.
- PROVEN (PC run, WSL, before the final gate edit): histogram `melee:Ok(())` 1689, `spell:Ok(())` 4198, `spell:Err(UnsupportedShape)` 116.
- UNKNOWN: how many of the 116 gaps are owned by a non-source owner (composed, chain, seven, summon), since those take a `ProjectV2Draft`.

## High-risk authority/recovery qualification

NOT_APPLICABLE: test-only change; no production mutation, fence, session, authority or recovery path is touched.

## Acceptance criteria

- [x] The gate test `authored_monster_attacks_have_a_runtime_source_or_are_in_the_pinned_gap_set` asserts the exact histogram above.
- [x] The 116 gaps are printed (`--nocapture`, `GAPS` block) and recorded here as a follow-up, not hidden.

## Excluded scope

No runtime, protocol, content or CI changes. The gate covers only the Melee and Spell source constructors.

## Implementation / findings

- The test loads `content/{creatures/definitions,behaviors,abilities/{definitions,effects,formulas}}`, builds per-creature closures and runs the two constructors for every behavior attack.
- Follow-up (open): list the 116 `spell:Err(UnsupportedShape)` attacks from the `GAPS` output, classify them by ability and owner, and implement the smallest unhandled kind in a follow-up PR; extend the gate to the draft-based owners.

## Validation

### Focused

- `cargo test -p oteryn-game-server --lib authored_monster_attacks`: pass on the exploration head on the owner's PC; final-head run by the CP before FREEZE
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass (60 tests)
- result: pass

### Component/integration

- NOT_APPLICABLE: test-only change.

### E2E

- NOT_APPLICABLE: test-only change.

### Exact-head CI

- final head: recorded in the PR and checks, not in this file
- result: pending

## Self-review

- exact head: PR #1982 authoring head
- method/reviewer: implementing agent
- material findings: none; the gate does not claim owner coverage beyond the two source constructors
- verdict: ready for CP freeze

## Independent review

- required: NO (test-only, no production path)
- exact head: NOT_APPLICABLE
- method/auditor: NOT_APPLICABLE
- material findings: NOT_APPLICABLE
- verdict: NOT_APPLICABLE

## PR and closeout

- changed-file review: monster_combat_lane.rs test module and this record only
- unresolved review threads: none
- related/superseded PRs: none
- protected auto-merge: not enabled by the worker
- merge commit/result: pending
- ownership release: on merge

## Context checkpoint

```yaml
last_progress: gate test and record authored
status: completed
branch: claude/monster-ability-runtime-20261011
head_sha: null
pr: 1982
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
next_action: CP freezes after the PC gate run
```
