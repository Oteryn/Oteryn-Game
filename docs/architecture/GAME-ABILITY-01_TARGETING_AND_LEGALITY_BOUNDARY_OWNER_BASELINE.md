# GAME-ABILITY-01 — Targeting and Legality Boundary Owner Baseline

- Status: **OWNER-ACCEPTED PARTIAL BASELINE / GAME-ABILITY-01 REMAINS OPEN**
- Date: 2026-08-13
- Gate: `GAME-ABILITY-01`
- Runtime/client/protocol/DDL/Platform/production authority: **NONE**

## Decision

Target discovery is a separate deterministic authoritative stage:

```text
Ability Invocation
-> Target Intent
-> bounded typed Target Query
-> authoritative Target Resolver
-> Resolved Target Set
-> Legality Evaluation
-> Validated Target Set / structured failure
-> typed Effect Plan
```

Ability content describes target policy; it does not directly enumerate authoritative world objects or provide the final target set. The server resolver interprets intent against authoritative state and stabilizes targets before effect planning.

## One boundary for every origin

Player/client intent, creature AI, NPC logic and explicitly permitted system mechanics all use the same Target Resolver. Origin changes capability/context, not the targeting engine. Client-provided IDs, positions or directions are requests only, never authoritative target results.

## Target Intent, Query and Set

Target Intent expresses what an invocation attempts to target. The exhaustive vocabulary is not frozen.

A versioned Ability Definition may describe bounded target policy such as allowed target kinds, cardinality, geometry/shape intent, relationship constraints and ordering/selection policy. Target Query is a typed bounded semantic request, not an unrestricted general-purpose world-query language.

The Resolved Target Set must have bounded cardinality, deterministic membership and explicit deterministic ordering whenever order matters. Tie-breaking may not depend on memory address, hash/container iteration, thread completion or unspecified database order.

Target resolution respects existing FND-03 Channel/Instance ownership. Cross-boundary target search is not implicitly authorized.

## Legality boundary

Discovery and legality are separate. After resolution, Legality Evaluation determines whether the invocation and relevant targets may participate under the active ruleset/profile and current state.

Later legality decisions include, where applicable, range, line of sight, floor relationship, PvP/world profile, protection-zone policy, relationship, immunities and source prerequisites. This baseline freezes the separation/order, **not** exact algorithms, numeric values, precedence or player-facing errors.

Effect Plan generation consumes only targets admitted by this boundary.

## No hidden retargeting

Ability definitions, effect primitives and Wasm/WIT extensions may not silently re-scan authoritative world state after the target set is stabilized.

Chain, jump, bounce, nearest-N and other dynamic retargeting are supported only as explicit bounded Target Resolution Steps using the same resolver. Later contracts must define snapshot timing, maximum depth/cardinality, deterministic ordering/ties and termination/failure behavior.

## Wasm/WIT

DUR-04 extensions remain capability-bounded and proposal-only. They cannot acquire unrestricted world iteration, bypass resolver/legality checks, directly construct authoritative target sets or hide retargeting inside effect application. Custom targeting must pass through a typed bounded resolver surface.

## Determinism, limits and evidence

Target resolution obeys SIM-DETERMINISM. The same authoritative state, revisions, normalized intent and ordered inputs must yield the same normalized target membership/order on supported targets.

Implementation acceptance requires explicit resource ceilings for candidate enumeration, result count, geometry complexity and dynamic retarget depth as applicable. Replay/evidence must distinguish divergence across intent normalization, query construction, resolution, legality and effect planning. Analytics remains read-only.

## Decision timing

**Must decide now: YES.** Timing, interruption, costs, cooldowns, conditions, AI abilities, combat parity and Studio authoring need one stable target-authority boundary first.

## Deliberately not decided

Not decided here: exhaustive intent/query grammar; geometry catalogue; range metric/values; LoS/visibility algorithm; z-level rules; PvP/PZ/friendly-fire/immunity rules; legality/error precedence; partial-target semantics; Reference tie priorities; dynamic retarget snapshot timing; spatial index; protocol/client UX; physical authoring format; combat formulas or Reference ability catalogue.

Unresolved Reference-sensitive behavior remains fail-closed.

## Supersession

Reopen only with representative-mechanic, performance, replay, security or authoring evidence showing this boundary cannot satisfy required behavior safely. Supersession must preserve or explicitly replace server authority, deterministic ordering, bounded resource use, multichannel ownership safety and no hidden effect-stage re-query.

## Current status

```text
GAME-ABILITY-01 overall
-> REQUIRED_FOR_ALPHA / OPEN

accepted
-> data-first typed effect pipeline
-> Target Intent -> Target Query -> Target Resolver
-> deterministic Resolved Target Set
-> separate Legality Evaluation
-> Effect Plan from validated targets only
-> explicit bounded re-resolution for dynamic targeting

next paper-only decision
-> cast/channel/interruption timing and cost-commit semantics
```
