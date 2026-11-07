# GAME-ABILITY-01 — Typed Effect Pipeline Owner Baseline

- Status: **OWNER-ACCEPTED PARTIAL BASELINE / GAME-ABILITY-01 REMAINS OPEN**
- Date: 2026-08-13
- Gate: `GAME-ABILITY-01`
- Scope: ability-definition and effect-resolution authority model only
- Runtime/client/protocol/DDL/Platform/production authority: **NONE**

## Decision

Oteryn abilities use a **data-first semantic definition + typed effect pipeline + bounded Wasm/WIT extension** model.

```text
Ability Definition
-> Ability Invocation
-> targeting / legality / cost checks
-> typed Effect Plan
-> authoritative validation
-> authoritative commit
-> typed Result / domain events
```

An Ability Definition is versioned content/ruleset data under DUR-04. It is not unrestricted executable code with direct write authority.

An Effect Plan is a bounded typed proposal of candidate consequences. Producing a plan does not mutate authoritative state. Only the authoritative simulation/domain owner may validate and commit accepted effects.

## Decision timing

**Must decide now: YES.** Targeting, legality, cast/interruption, cost, cooldown, condition and Reference-parity decisions need one common execution/authority model first.

This prevents later content work from creating two competing mutation engines: typed domain mechanics and script-driven direct state mutation.

It unblocks paper-only design of target resolution, legality, timing, costs, cooldowns/charges, conditions, effect-family composition, parity fixtures and Oteryn Studio authoring.

## Semantic boundaries

### Ability Definition

A semantic content/ruleset definition may later describe targeting, timing, costs, prerequisites, cooldowns/charges, effect composition, condition intent, presentation references and an optional bounded extension hook.

This baseline does **not** select a physical file format, serializer or Rust representation.

### Ability Invocation

All gameplay origins use the same authoritative ability pipeline. An invocation may begin from untrusted player/client intent or from an already-authoritative server-side source such as creature AI, NPC logic, encounter/world mechanics or another explicitly permitted domain action.

Origin does not create a second execution engine. Each invocation carries/derives the authority and capability context appropriate to its source, and the authoritative server still resolves current legality, targets, costs and results before commit. Client input therefore remains intent only, while AI/NPC/system origin remains subject to the same typed effects, determinism, revision and domain-invariant boundaries.

This document creates no competing foundation identity; invocation/CommandRef identity remains with existing FND/SIM contracts and later GAME-ABILITY work.

### Effect Plan

An Effect Plan contains typed candidate consequences. Damage, healing, resource changes, condition proposals, dispel, movement, summon/spawn and item-related proposals are illustrative examples only; the exhaustive effect vocabulary is not frozen here.

### Authoritative commit

Commit must preserve applicable accepted contracts, including FND-03 one-writer/order/generation authority, FND-04 fencing, GAME-ITEM/DUR-03 conservation and idempotency, SIM-DETERMINISM exact arithmetic/RNG/order/revision semantics, DUR-04 script determinism/resource limits and ANL-01 event/audit boundaries.

Ability content cannot bypass those contracts.

## Data-first, not data-only

Most abilities should use validated declarative composition over typed mechanic/effect primitives because this improves static validation, deterministic testing, parity fixtures, runtime efficiency, editor tooling, migration analysis and Game Intelligence observability.

Oteryn still keeps a bounded Wasm/WIT extension path for mechanics that would otherwise require excessive primitive proliferation.

## Wasm/WIT extension boundary

Ability extensions consume only capability-bounded, snapshot-bound inputs exposed by DUR-04 and return bounded typed proposals.

They may not directly mutate Character, Item, World, Channel or Instance state; bypass persistence, ownership/session fences, DUR-03 conservation or SIM determinism; or return untyped authoritative state patches.

Extension failure, resource exhaustion, invalid proposals or revision mismatch must fail under an explicit later policy and never fall back to direct mutation or a different ruleset interpretation.

## Determinism and revision binding

Authoritative ability resolution must remain reproducible under SIM-DETERMINISM. Later implementation binds every behavior-affecting revision required for replay, including as applicable exact ability/content revision, ruleset/profile revision, numeric/formula profile, script profile/artifact, RNG evidence and normalized authoritative inputs/order.

A delayed, retried or resumed logical occurrence may not silently switch to a newer incompatible ability definition or formula revision.

## Deliberately not decided

This partial baseline does not decide target grammar/LoS, legality error precedence, cast/channel/interruption, cost reservation/consumption timing, cooldown/charge semantics, condition stacking/refresh/dispel/ticks, exhaustive effect families, combat formulas, Reference ability catalogue/parity, physical authoring format, exact WIT/Wasmtime implementation, protocol layout, client UI or persistence layout.

## Supersession criteria

Reopen only with concrete evidence that representative mechanics cannot be expressed without unsafe complexity or excessive primitive proliferation, measured overhead prevents accepted performance goals, replay/migration/security evidence favors a safer boundary, or Oteryn Studio authoring evidence shows the model is impractical.

Any supersession must explicitly preserve or replace the authority, determinism, conservation and bounded-extension safety properties accepted here.

## Current status

```text
GAME-ABILITY-01 overall
-> REQUIRED_FOR_ALPHA / NOT YET ACCEPTED AS A WHOLE

accepted subdecision
-> data-first Ability Definition
-> one pipeline for player, AI, NPC and system origins
-> typed bounded Effect Plan
-> authoritative validation + commit
-> bounded DUR-04 Wasm/WIT proposal extension
-> no direct script/client mutation authority

next paper-only decision
-> targeting model and legality/effect-resolution boundary
```
