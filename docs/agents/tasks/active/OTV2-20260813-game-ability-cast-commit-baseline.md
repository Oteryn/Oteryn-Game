# OTV2-20260813-game-ability-cast-commit-baseline

```yaml
task_id: OTV2-20260813-game-ability-cast-commit-baseline
title: Record GAME-ABILITY-01 cast/channel/interruption and commit-point baseline
mode: CONTRACT
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/game-ability-cast-commit-baseline
pr: 231
base_sha: b4ea6d26ee8e783c4ce26a03655904e5f7786bb6
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT architecture coordinator
created_at: 2026-08-13T18:22:00+02:00
updated_at: 2026-08-13T18:29:00+02:00
execution_budget_minutes: 60
owned_paths:
  - docs/agents/tasks/active/OTV2-20260813-game-ability-cast-commit-baseline.md
  - docs/architecture/GAME-ABILITY-01_CAST_CHANNEL_COMMIT_OWNER_BASELINE.md
public_contracts:
  - docs/architecture/GAME-ABILITY-01_CAST_CHANNEL_COMMIT_OWNER_BASELINE.md
depends_on:
  - docs/architecture/GAME-ABILITY-01_TYPED_EFFECT_PIPELINE_OWNER_BASELINE.md
  - docs/architecture/GAME-ABILITY-01_TARGETING_AND_LEGALITY_BOUNDARY_OWNER_BASELINE.md
  - docs/architecture/FND-03_RUNTIME_EXECUTION_CONTRACT.md
  - docs/architecture/SIM-DETERMINISM-01_AUTHORITATIVE_SIMULATION_CONTRACT.md
  - docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md
  - docs/architecture/ANL-01_GAME_EVENT_AND_AUDIT_FOUNDATION_CONTRACT.md
blocks:
  - safe continuation of GAME-ABILITY-01 cooldown/charge, condition lifecycle and effect-composition decisions
external_repositories: []
```

## Outcome

Record the owner-accepted third bounded `GAME-ABILITY-01` subdecision: explicit authoritative ability lifecycle, explicit logical primary commit point, and versioned anchor policies for costs/cooldowns/charges. Overall `GAME-ABILITY-01` remains open / `REQUIRED_FOR_ALPHA`.

## Source of truth

- **PROVEN:** prior GAME-ABILITY baselines establish one authoritative typed-effect pipeline and deterministic target/legality boundary.
- **PROVEN:** FND-03/SIM require deterministic authoritative ordering; client or thread timing cannot decide interrupt-versus-commit races.
- **PROVEN:** DUR-03 conservation/idempotency prevents hidden rollback from bypassing item/value ownership.
- **PROVEN:** owner accepted this lifecycle/commit model on 2026-08-13.
- **UNKNOWN:** exact cast/GCD/cooldown/cost values, Reference timing rules, target revalidation timing and crash/logout/reconnect behavior remain later decisions.

## Acceptance criteria

- [x] Add the canonical lifecycle/commit owner baseline.
- [x] Define optional casting/channel phases and explicit terminal states without freezing scheduler implementation.
- [x] Define primary `COMMIT` as the existing typed-effect authoritative commit boundary, not a second engine or physical DB/CPU transaction.
- [x] Require costs/cooldowns/charges to use explicit versioned anchor policies; no single global anchor is frozen.
- [x] Allow only explicit resource reservation; reservation protects only that resource and never guarantees whole-ability legality.
- [x] Require post-commit reversal to be a new explicit compensation, never history erasure.
- [x] Model channel ticks/pulses as bounded deterministic authoritative occurrences, not an unbounded script loop.
- [x] Resolve interrupt/commit races by authoritative ordering with replay evidence.
- [x] Preserve targeting, ownership, conservation and audit/event boundaries.
- [x] Keep exact values/formulas, protocol/client UX, DDL/runtime/production implementation out of scope.
- [ ] Complete final-head full-diff self-review and exact-head documentation/governance CI before merge.

## Excluded scope

No Rust gameplay runtime, scheduler, protocol schema, database migration, numeric cast/GCD/cooldown/cost/refund rules, Reference spell timing claims, client animation/UI, Platform write, production behavior or external-repository mutation.

## Findings

PR #231 contains exactly this task record and `docs/architecture/GAME-ABILITY-01_CAST_CHANNEL_COMMIT_OWNER_BASELINE.md`.

Pre-final self-review repaired two semantic ambiguities: primary commit identity and over-broad reservation wording.

The first exact-head Agent Governance generation (`31720735496`) failed before checkout only because the shortened PR body lacked required `## Summary`, `## Scope`, and `## Validation` headings. PR metadata was repaired before this new head; no architecture semantics changed.

Open PRs #162 and #191 are non-overlapping and remain untouched. #162 has separate exhausted repair authority.

## Validation

Focused full diff: semantic findings repaired; final-head inspection pending.

Component/integration/E2E: `NOT_APPLICABLE` — architecture-only documentation.

Exact-head CI: fresh generation pending after PR metadata repair.

Self-review: final-head pending.

Independent review: `NO` unless final-head review discovers material uncertainty.

## Context checkpoint

```yaml
status: validating
branch: docs/game-ability-cast-commit-baseline
pr: 231
owner_action_required: false
blocker: null
next_action: Inspect the new exact full diff and require a fresh complete exact-head CI generation.
```
