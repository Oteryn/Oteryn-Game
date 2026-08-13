# OTV2-20260813-game-ability-effect-composition-baseline

```yaml
task_id: OTV2-20260813-game-ability-effect-composition-baseline
title: Record GAME-ABILITY-01 effect composition and damage/heal calculation baseline
mode: CONTRACT
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/game-ability-effect-composition-baseline
pr: 235
base_sha: 2b813f713a70c2be91c4ef7b6f052836a4658d16
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT architecture coordinator
created_at: 2026-08-13T19:07:00+02:00
updated_at: 2026-08-13T19:16:00+02:00
execution_budget_minutes: 60
owned_paths:
  - docs/agents/tasks/active/OTV2-20260813-game-ability-effect-composition-baseline.md
  - docs/architecture/GAME-ABILITY-01_EFFECT_COMPOSITION_DAMAGE_HEAL_OWNER_BASELINE.md
public_contracts:
  - docs/architecture/GAME-ABILITY-01_EFFECT_COMPOSITION_DAMAGE_HEAL_OWNER_BASELINE.md
depends_on:
  - docs/architecture/GAME-ABILITY-01_TYPED_EFFECT_PIPELINE_OWNER_BASELINE.md
  - docs/architecture/GAME-ABILITY-01_TARGETING_AND_LEGALITY_BOUNDARY_OWNER_BASELINE.md
  - docs/architecture/GAME-ABILITY-01_CAST_CHANNEL_COMMIT_OWNER_BASELINE.md
  - docs/architecture/GAME-ABILITY-01_COOLDOWN_CHARGE_CONDITION_OWNER_BASELINE.md
  - docs/architecture/SIM-DETERMINISM-01_AUTHORITATIVE_SIMULATION_CONTRACT.md
  - docs/architecture/DUR-04_CONTENT_WORLD_AND_SCRIPTING_CONTRACT.md
  - docs/architecture/ANL-01_GAME_EVENT_AND_AUDIT_FOUNDATION_CONTRACT.md
blocks:
  - safe continuation of GAME-ABILITY-01 Reference combat/formula catalogue
external_repositories: []
```

## Outcome

Record the owner-accepted fifth bounded `GAME-ABILITY-01` subdecision: damage/heal share typed staged deterministic composition while remaining distinct semantics; RNG is explicit/replayable; calculation is non-mutating; reactions are bounded descendant occurrences.

Overall `GAME-ABILITY-01` remains open / `REQUIRED_FOR_ALPHA`.

## Acceptance criteria

- [x] Canonical typed effect-composition/damage-heal baseline added.
- [x] Damage/heal distinct semantics with shared deterministic infrastructure.
- [x] No arbitrary authoritative formula callback/direct mutation during calculation.
- [x] Typed stage-local contributions with deterministic order and SIM numeric/RNG semantics.
- [x] State-consuming absorbs emit typed companion consequences; durable value remains under GAME-ITEM/DUR-03.
- [x] Multi-hit/multi-target snapshot/sequential semantics explicit and bounded.
- [x] Reactive descendants use lineage/depth/cardinality/work budgets; no recursive hidden mutation.
- [x] Alternatives, trade-offs, risks, player/producer/operations impact and full mandatory decision test recorded.
- [x] Exact formulas/values/runtime/protocol/DDL/Platform/production remain deferred/out of scope.
- [ ] Complete final exact-head review classification, zero unresolved material threads and Agent Governance/Dependency Review/CodeQL before merge.

## Findings and repairs

Pre-final full-diff review found three material boundary ambiguities and repaired them before freeze:

1. magnitude-side mitigation wording could have been read as moving immunity/admissibility from the accepted Target/Legality boundary into the calculator; the final text preserves legality ownership and limits this stage to post-admission magnitude transforms;
2. typed contributions could have degraded into a generic unordered modifier bag; the final text requires stage-local typed operations, bounds and versioned composition order;
3. state-consuming shields could have implied new item/durable-value semantics in GAME-ABILITY; the final text keeps every item/currency/durable consequence subordinate to GAME-ITEM/DUR-03 and defines no new ownership/conservation/recovery rule.

Open PR #162 remains non-overlapping and its own body states its repair budget is exhausted. PR #191 remains non-overlapping stale-base GAME-CHAR provenance work. Neither is modified.

## Validation

Focused full diff against prior GAME-ABILITY boundaries, current SIM numeric/RNG semantics, DUR-04, ANL-01, GAME-ITEM/DUR-03 and architecture decision discipline: final-head review pending after this bookkeeping commit.

Component/integration/runtime E2E: `NOT_APPLICABLE` — architecture-only documentation.

Exact-head CI: pending.

Independent review classification: default `NOT_REQUIRED` only if final-head self-review shows no mandatory AGENTS.md trigger, material uncertainty or unusual unresolved complexity. Owner-funded Codex/OpenAI usage is not authorized.

## Context checkpoint

```yaml
status: validating
branch: docs/game-ability-effect-composition-baseline
pr: 235
owner_action_required: false
blocker: null
next_action: Inspect the new exact full diff; if clean, freeze head and require exact-head Agent Governance, Dependency Review and CodeQL.
```
