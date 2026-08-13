# OTV2-20260813-game-ability-cooldown-condition-baseline

```yaml
task_id: OTV2-20260813-game-ability-cooldown-condition-baseline
title: Record GAME-ABILITY-01 cooldown, charge and condition lifecycle baseline
mode: CONTRACT
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/game-ability-cooldown-condition-baseline
pr: 233
base_sha: 2632594b0617dd922ad32c4260fa09383c15ba45
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT architecture coordinator
created_at: 2026-08-13T18:49:00+02:00
updated_at: 2026-08-13T18:55:00+02:00
execution_budget_minutes: 60
owned_paths:
  - docs/agents/tasks/active/OTV2-20260813-game-ability-cooldown-condition-baseline.md
  - docs/architecture/GAME-ABILITY-01_COOLDOWN_CHARGE_CONDITION_OWNER_BASELINE.md
public_contracts:
  - docs/architecture/GAME-ABILITY-01_COOLDOWN_CHARGE_CONDITION_OWNER_BASELINE.md
depends_on:
  - docs/architecture/GAME-ABILITY-01_TYPED_EFFECT_PIPELINE_OWNER_BASELINE.md
  - docs/architecture/GAME-ABILITY-01_TARGETING_AND_LEGALITY_BOUNDARY_OWNER_BASELINE.md
  - docs/architecture/GAME-ABILITY-01_CAST_CHANNEL_COMMIT_OWNER_BASELINE.md
  - docs/architecture/FND-03_RUNTIME_EXECUTION_CONTRACT.md
  - docs/architecture/SIM-DETERMINISM-01_AUTHORITATIVE_SIMULATION_CONTRACT.md
  - docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md
  - docs/architecture/ANL-01_GAME_EVENT_AND_AUDIT_FOUNDATION_CONTRACT.md
blocks:
  - safe continuation of GAME-ABILITY-01 effect composition, damage/heal pipeline and Reference mechanic catalogue decisions
external_repositories: []
```

## Outcome

Record the owner-accepted fourth bounded `GAME-ABILITY-01` subdecision: typed cooldown/charge runtime state, immutable/versioned condition definitions, authoritative condition instances, explicit stacking/refresh/replacement policies, and typed dispel/immunity/resistance/suppression boundaries. Overall `GAME-ABILITY-01` remains open / `REQUIRED_FOR_ALPHA`.

## Source of truth

- **PROVEN:** prior GAME-ABILITY baselines establish one typed authoritative effect pipeline, deterministic targeting/legality and explicit commit anchors.
- **PROVEN:** FND-03/SIM require owner-local deterministic state/order and prohibit process-global gameplay mutation state.
- **PROVEN:** DUR-03 forbids value duplication through hidden rollback or competing ownership.
- **PROVEN:** owner accepted this cooldown/charge/condition direction on 2026-08-13.
- **UNKNOWN:** exact Reference cooldown groups, durations, charge counts/recharge, condition stacking identity, immunity/resistance precedence, tick cadence and logout/restart persistence remain evidence-driven later policy.

## Acceptance criteria

- [x] Add one canonical owner baseline for cooldown/charge scopes and condition lifecycle.
- [x] Keep cooldown and charge-pool state distinct from each other and from item charges.
- [x] Require typed/versioned cooldown keys/scopes without process-global gameplay cooldown state.
- [x] Separate immutable/versioned `ConditionDefinition` semantics from authoritative runtime `ConditionInstance` state.
- [x] Require explicit deterministic condition conflict/stack policies; no implicit last-write-wins behavior.
- [x] Make condition ticks/repeated effects use the same typed Effect Plan and authoritative commit pipeline.
- [x] Model dispel, immunity, resistance and suppression as distinct typed policy layers.
- [x] Make removal/expiry/dispels forward-only; past committed ticks/effects are never silently rolled back.
- [x] Preserve proposal-only Wasm, deterministic ordering/resource bounds and typed audit evidence.
- [x] Keep exact Reference values/precedence, physical persistence, protocol/client UX and runtime implementation out of scope.
- [ ] Complete final-head full-diff self-review and exact-head Agent Governance, Dependency Review and CodeQL before merge.

## Excluded scope

No Rust gameplay implementation, scheduler/timer implementation, protocol schema, database migration, physical serializer, exact cooldown/GCD/charge values, exact condition duration/tick/stacking rules, Reference spell catalogue, client status UI, Platform write, production behavior or external-repository mutation.

## Findings

PR #233 contains exactly this task record and `docs/architecture/GAME-ABILITY-01_COOLDOWN_CHARGE_CONDITION_OWNER_BASELINE.md`.

Pre-final self-review found and repaired two material boundary ambiguities:

1. initial condition admission/refresh/stack/replace/dispel wording could have allowed an out-of-band condition-collection mutation path; the repaired baseline requires ability-/mechanic-driven condition transitions to arrive as typed effects/actions and become authoritative only through the existing validation/commit boundary, while owner-local lifecycle expiry remains deterministic and auditable;
2. `global-style` cooldown scope and suppression wording could be misread as process/world-global mutable state or direct suppress/unsuppress mutation; the repaired baseline limits broad cooldowns to an explicitly named gameplay subject/domain and routes ability-/mechanic-driven suppression transitions through the same typed commit boundary.

Final semantic review after those repairs found no new material issue.

Open PRs #162 and #191 are non-overlapping and remain untouched. #162 has exhausted separate repair authority and is not authorized for repair by this task.

## Validation

### Focused

- command/run: inspect complete PR #233 diff against prior GAME-ABILITY, FND-03, SIM, DUR-03 and ANL-01 boundaries
- result: pre-final findings repaired; final-head inspection pending after this bookkeeping commit

### Component/integration

- command/run: `NOT_APPLICABLE` — architecture-only documentation
- result: `NOT_APPLICABLE`

### E2E

- scenario: `NOT_APPLICABLE` — no executable/player-visible behavior
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: pending
- trigger source: pull request
- workflow/run/job: pending
- classification: documentation/governance
- result: pending

## Self-review

- exact head: pending
- method/reviewer: implementing/coordinating agent
- material findings: two pre-final boundary ambiguities repaired; final-head findings pending
- verdict: pending

## Independent review

- required: `NO` unless final-head review discovers material uncertainty; bounded paper-only partial baseline with no runtime/protocol/durable-schema/security/production authority change
- exact head: `NOT_APPLICABLE`
- method/auditor: `NOT_APPLICABLE`
- material findings: `NOT_APPLICABLE`
- verdict: `NOT_APPLICABLE`

## Context checkpoint

```yaml
last_progress: Two pre-final architecture ambiguities repaired and recorded; semantic scope remains the owner-accepted cooldown/charge/condition baseline.
status: validating
branch: docs/game-ability-cooldown-condition-baseline
pr: 233
owner_action_required: false
blocker: null
next_action: Inspect this new exact head, freeze it if clean and require exact-head Agent Governance, Dependency Review and CodeQL.
```
