# OTV2-20260813-game-ability-effect-families-reference-catalogue-baseline

```yaml
task_id: OTV2-20260813-game-ability-effect-families-reference-catalogue-baseline
title: Record GAME-ABILITY-01 effect-family and Reference mechanic catalogue boundary
mode: CONTRACT
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/game-ability-effect-families-reference-catalogue-baseline
pr: 245
base_sha: 463ee694edb5edc156e801e456a9c6298482b485
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT architecture coordinator
created_at: 2026-08-13T20:28:00+02:00
updated_at: 2026-08-13T20:31:00+02:00
execution_budget_minutes: 60
owned_paths:
  - docs/agents/tasks/active/OTV2-20260813-game-ability-effect-families-reference-catalogue-baseline.md
  - docs/architecture/GAME-ABILITY-01_EFFECT_FAMILIES_REFERENCE_CATALOGUE_OWNER_BASELINE.md
public_contracts:
  - docs/architecture/GAME-ABILITY-01_EFFECT_FAMILIES_REFERENCE_CATALOGUE_OWNER_BASELINE.md
depends_on:
  - docs/architecture/GAME-ABILITY-01_TYPED_EFFECT_PIPELINE_OWNER_BASELINE.md
  - docs/architecture/GAME-ABILITY-01_TARGETING_AND_LEGALITY_BOUNDARY_OWNER_BASELINE.md
  - docs/architecture/GAME-ABILITY-01_CAST_CHANNEL_COMMIT_OWNER_BASELINE.md
  - docs/architecture/GAME-ABILITY-01_COOLDOWN_CHARGE_CONDITION_OWNER_BASELINE.md
  - docs/architecture/GAME-ABILITY-01_EFFECT_COMPOSITION_DAMAGE_HEAL_OWNER_BASELINE.md
  - docs/architecture/GAME-ITEM-01_ITEM_MODEL_AND_EQUIPMENT_CONTRACT.md
  - docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md
  - docs/architecture/DUR-04_CONTENT_WORLD_AND_SCRIPTING_CONTRACT.md
  - docs/architecture/SIM-DETERMINISM-01_AUTHORITATIVE_SIMULATION_CONTRACT.md
  - docs/architecture/ANL-01_GAME_EVENT_AND_AUDIT_FOUNDATION_CONTRACT.md
  - docs/architecture/REFERENCE_EVIDENCE_PARITY_MANIFEST_CONTRACT.md
blocks:
  - safe construction of the Reference ability/combat mechanic catalogue and later representative parity fixtures
external_repositories: []
```

## Outcome

Record the owner-accepted sixth bounded `GAME-ABILITY-01` subdecision: a small stable typed effect-family vocabulary is distinct from concrete Reference mechanics; cross-domain effects remain subordinate to their owning domains; and the Reference Mechanic Catalogue is an evidence-linked declarative index/composition layer rather than runtime code, protocol vocabulary or a second mutation engine.

Overall `GAME-ABILITY-01` remains open / `REQUIRED_FOR_ALPHA`.

## Acceptance criteria

- [x] Add one canonical owner baseline separating effect families, domain-owned transitions and concrete Reference mechanic catalogue entries.
- [x] Freeze the separation/selection criteria without freezing an exhaustive effect enum, Rust type graph or physical serializer.
- [x] Keep item/currency/loot/durable-value effects subordinate to `GAME-ITEM` / `DUR-03`.
- [x] Keep movement/spatial, entity lifecycle, AI/threat and world/tile interactions under their respective domain owners; ability may propose/initiate but not take mutation ownership.
- [x] Prevent generic state-patch/event-bus or per-mechanic hardcoded effect families from becoming alternate mutation engines.
- [x] Require new core effect families to satisfy explicit reuse/invariant/boundedness/ownership/testability criteria; prefer composition or bounded Wasm for exceptional mechanics.
- [x] Define the Reference Mechanic Catalogue as stable mechanic identity + evidence references + behavior-affecting revision bindings + parity/unknown state, not executable truth.
- [x] Make evidence-manifest `UNKNOWN`/`CONFLICT`/empty cases fail closed; catalogue presence never implies Reference parity.
- [x] Record realistic options, trade-offs, risks, player/producer/operations impact and the mandatory architecture decision test.
- [x] Keep exact formulas, exact catalogue population, exact effect taxonomy, AI/world/item domain internals, runtime, protocol, DDL, Platform and production out of scope.
- [ ] Complete exact-head full-diff self-review, review classification, zero unresolved threads and aggregate `Merge gate / validate` before merge.

## Findings

PR #245 contains exactly this task record and `docs/architecture/GAME-ABILITY-01_EFFECT_FAMILIES_REFERENCE_CATALOGUE_OWNER_BASELINE.md`.

The owner baseline explicitly treats effect-family vocabulary as semantic routing/validation, not domain ownership. Cross-domain atomicity is not invented: mechanics needing stronger guarantees remain fail-closed until participating owner contracts define them. Reference Catalogue presence cannot upgrade the existing Reference evidence/parity manifest axes.

PR #191 remains a non-overlapping stale-base GAME-CHAR provenance correction classified `REBASE/FIX`; it is not merged, closed or absorbed.

## Validation

Focused full diff against all five accepted GAME-ABILITY partial baselines, `GAME-ITEM`/DUR-03, DUR-04, SIM, ANL-01, the Reference evidence/parity manifest and architecture decision discipline: pending exact-head self-review.

Component/integration/runtime E2E: `NOT_APPLICABLE` — architecture-only documentation.

Exact-head aggregate CI: pending.

Independent review classification: `NOT_REQUIRED` only if final self-review confirms no durable/protocol/security/recovery/multichannel/governance ownership change and no unresolved material uncertainty. Owner-funded Codex/OpenAI use is not authorized.

## Context checkpoint

```yaml
status: validating
branch: docs/game-ability-effect-families-reference-catalogue-baseline
pr: 245
owner_action_required: false
blocker: null
next_action: Inspect the exact full diff, repair material boundary findings, freeze the final head and require aggregate exact-head validation.
```
