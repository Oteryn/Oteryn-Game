# OTV2-20260908-atlas-farm-intelligence-75

```yaml
task_id: OTV2-20260908-atlas-farm-intelligence-75
title: Game-owned Atlas farm-intelligence v1 export
mode: IMPLEMENT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/atlas-farm-intelligence-75
issue: 75
pr: null
base_sha: 78397d42d082da8abdc47f378e16b05949ec66c1
head_sha: null
final_head_sha: null
owner: Codex sole material writer
created_at: 2026-09-08T00:00:00Z
updated_at: 2026-09-08T00:00:00Z
execution_policy: continuous_progress
owned_paths:
  - docs/contracts/OTERYN_GAME_ATLAS_FARM_INTELLIGENCE_V1.md
  - tools/game-atlas-farm-intelligence/**
  - docs/agents/tasks/active/OTV2-20260908-atlas-farm-intelligence-75.md
  - docs/superpowers/plans/2026-09-08-atlas-farm-intelligence-75.md
public_contracts:
  - oteryn-game-atlas-farm-intelligence-v1
depends_on:
  - creature-gameplay-profiles-v1
blocks:
  - Oteryn-Atlas Item & Spawn Farm Explorer probability features
cross_repository_coordination_id: ATLAS-ITEM-SPAWN-FARM-EXPLORER
external_repositories:
  - Oteryn/Oteryn-Atlas (read-only consumer evidence)
```

## Outcome

A deterministic fail-closed producer/validator refuses caller-authored farm
facts and emits only capability blockers until an authenticated admitted Game
publication exists. Atlas cannot mistake synthetic fixtures, count bounds, or
unsupported task/weekly/respawn/supply families for authority.

## Architecture and source of truth

- **PROVEN:** protected admission and activated allocation are
  `main@78397d42d082da8abdc47f378e16b05949ec66c1` and allocation PR #431.
- **PROVEN:** `OTERYN_GAME_ATLAS_CREATURE_GAMEPLAY_PROFILES_V1` supplies stable
  creature identity and integer static chance/count bounds.
- **UNKNOWN:** exact per-kill quantity process, live modifier context, complete
  task/grouped-credit/weekly catalogue, complete placement supply, live respawn.
- **UNKNOWN:** public producer bounds because the admitted corpus is unavailable
  to census. Synthetic validation limits are explicitly test-only.

High-risk authority/recovery qualification: **NOT_APPLICABLE**. This is a static
public read model with no production mutation, controller, persistence, session,
or authority-bearing recovery operation.

## Acceptance criteria

- [x] Per-family source qualification precedes supported capability claims.
- [x] Missing proof is partial/unsupported/unknown, never guessed or empty success.
- [x] Canonical deterministic producer has no input surface; validator accepts
      only the exact blocked product, without invented public bounds.
- [x] TDD covers identity, rational probability, quantity models, provenance,
      relation integrity, capability states, malformed/corrupt/oversized input.
- [x] No dynamic parsing/scraping and no Atlas farm calculations.
- [x] Focused/component/governance checks and whole-diff review pass.
- [ ] Exactly one draft PR is opened for Issue #75.

## Excluded scope

All paths outside the activated lease; runtime/content and existing producer
changes; workflows/Cargo/migrations; external writes; production/live data;
Reference parity; guessed PMF/task/weekly/respawn; Atlas computations.

## Implementation / findings

The accepted gameplay contract is useful but insufficient for an exact farm
model, and its product/corpus is not present here. A syntactically valid SHA,
digest, generation, repository name, or caller capability flag cannot establish
authority. Production consequently accepts no caller input and emits only an
exact blocked product. Rich relation cases use a distinct non-publishable test
contract and fixed synthetic marker; their limits and capability exercises are
test-only.

Remediation RED evidence: coordinator review demonstrated that arbitrary
syntactically valid provenance plus a caller `COMPLETE` flag could promote
synthetic `FIXED`/`EXACT_PMF` facts, and that numeric production limits lacked
the required admitted-corpus census. GREEN removes the production input surface,
accepts only exact blocked bytes, and moves rich semantics and their numeric
limits behind a distinct test-only contract and non-authority marker.

## Validation

### Focused

- command/run: `python3 -m unittest discover -s tools/game-atlas-farm-intelligence -p 'test_*.py'`
- result: PASS, 8 tests include caller-authority rejection,
  exact blocked bytes, test-only isolation, and negative/adversarial cases

### Component/integration

- command/run: deterministic double export plus exact validator of the blocked product
- result: PASS, byte-identical outputs; SHA-256
  `b37bcf5f259d1c9a5c1a4887b0e367ea4dd5f847b5fd77e3b54890c813784fb8`

### E2E

- scenario: `NOT_APPLICABLE` — static export/read-model only; no runtime gameplay behavior changes.
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: pending (recorded in PR/check evidence; commits cannot self-reference)
- trigger source/workflow/result: pending repository CI

## Self-review

- exact head: final commit will be reported in PR evidence
- method/reviewer: implementing agent, adversarial whole-diff inspection
- material findings/verdict: P0=0, P1=0, P2=0; PASS. Confirmed exact lease,
  no production caller-fact input, no public numeric ceiling without census,
  exact blocked-product verification, conspicuous non-publishable synthetic
  fixtures, unsupported source families, and no dynamic source access.

## Independent review

- required: YES — new public consumer contract; Work coordinates exact-head review
- exact head/method/findings/verdict: pending external lifecycle

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: no farm-intelligence implementation PR at admission
- protected auto-merge/merge/ownership release: forbidden to this worker; pending Work

## Context checkpoint

```yaml
last_progress: focused/component/governance validation and whole-diff review passed
status: validating
branch: agent/atlas-farm-intelligence-75
head_sha: null
pr: null
final_head_sha: null
ci_trigger_source: null
ci_check_generation: null
ci_checks_for_current_head: 0
ci_run_ids: []
runner_assignment_state: unknown
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: commit and open exactly one draft PR for Issue 75
```
