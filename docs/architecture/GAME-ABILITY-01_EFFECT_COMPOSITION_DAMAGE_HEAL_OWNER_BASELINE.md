# GAME-ABILITY-01 — Effect Composition and Damage/Heal Owner Baseline

- Status: **OWNER-ACCEPTED PARTIAL BASELINE / GAME-ABILITY-01 REMAINS OPEN**
- Date: 2026-08-13
- Gate: `GAME-ABILITY-01`
- Runtime/client/protocol/DDL/Platform/production authority: **NONE**

## Decision

Damage and healing use one typed, staged, deterministic composition framework, but remain distinct semantics. Calculation is read-only and yields bounded typed proposals; state changes only through the existing Effect Plan / `PRIMARY COMMIT`.

```text
Damage/Heal Intent
-> Base
-> Source contributions
-> Ruleset/context contributions
-> Target contributions
-> magnitude-side mitigation/resistance/absorb
-> explicit versioned RNG decision points
-> bounded normalized result
-> Effect Plan
-> validation
-> PRIMARY COMMIT
-> bounded reactive descendants [optional]
```

The diagram is not one universal formula order. A versioned formula/ruleset descriptor chooses applicable stages, behavior-affecting order, RNG anchors, arithmetic, rounding and clamps.

## Required boundaries

- Damage is not negative healing and healing is not negative damage.
- Content/Wasm may supply bounded typed inputs/contributions but cannot directly set authoritative final results or mutate state during calculation.
- Contributions are typed stage-local inputs/transforms, not an unordered generic modifier bag. Their operation vocabulary, bounds and order are versioned.
- Ordering cannot depend on registration/hash/pointer/thread/plugin-load/unspecified-DB order.
- Numeric representation is deferred; SIM-DETERMINISM formula/profile rules govern authoritative arithmetic, rounding, overflow, clamps and invalid values.
- Crit/block/dodge/proc/variance use SIM-owned stable occurrence/purpose identity, purpose isolation and retry-stable evidence. Exact Reference roll order/chances remain deferred.
- Effect/damage taxonomy is typed/versioned; the exhaustive list is deferred.
- State-consuming shields/absorbs express correlated consequences as typed plan/transition entries; they cannot hide resource mutation inside a calculator.
- Any item/currency/durable-value consequence stays under `GAME-ITEM` / `DUR-03`; this baseline defines no new durable-value ownership, conservation, reservation or recovery semantics.
- The accepted Target/Legality boundary stays authoritative. This baseline does not move target admissibility, PvP/PZ or legality-owned immunity into magnitude calculation. Magnitude mitigation/resistance/absorb occurs after admission unless a later explicit compatibility decision says otherwise.

## Multi-hit, multi-target and reactions

Multi-hit/multi-target behavior must declare a bounded versioned mode such as stabilized-snapshot or ordered sequential sub-occurrences. Worker/loop scheduling cannot decide whether prior results affect later ones. Per-hit/per-target order, RNG identity, caps and partial-failure behavior are deterministic when behavior-affecting.

Reflect, thorns, lifesteal and on-hit/on-damage/on-heal procs cannot recurse through hidden mutation inside the calculator. They create typed reactive descendant occurrences with stable parent lineage and re-enter applicable targeting/legality/effect-plan/commit machinery. Reactions require deterministic order, depth/cardinality/work budgets and cycle/re-entry policy. Exact Reference precedence remains deferred.

## Snapshot, revision and failure

A calculation uses an explicit authoritative occurrence/snapshot context and cannot opportunistically re-read changing state due to worker timing. Intentional observation of prior committed state uses explicit sequential occurrences/sub-occurrences.

Behavior-affecting formula/ruleset/SIM/content/script revisions remain bound to the logical occurrence. Retry cannot silently reroll or reinterpret under newer revisions.

Invalid numeric state, missing revision, invalid RNG evidence, illegal component or reaction-budget exhaustion fails deterministically before unauthorized partial mutation. No fallback to client results, alternate formulas, direct script mutation or nondeterministic arithmetic. Committed parent results are not erased by later reactive failure.

## Observability

ANL-01 remains read-only. Evidence may correlate source/target, reaction lineage, formula/ruleset/SIM revisions, effect family/type, final magnitude, relevant random decisions and commit/reaction result. Deep stage traces may be test/debug/parity evidence rather than unbounded production telemetry. Security-sensitive RNG seeds/roots are not exposed.

## Required analysis

**Problem:** support Reference/Evolved combat, items, conditions, AI and future content without per-ability formula order, RNG or mutation semantics drifting apart.

**Constraints:** one Effect Plan/commit engine; separate target/legality; explicit lifecycle/condition transitions; SIM deterministic numeric/RNG semantics; bounded proposal-only DUR-04 scripting; Reference unknowns fail closed; no runtime authority here.

**Option A — typed staged composition — SELECTED.** Strong replay/parity, static validation, Studio/analytics and consistent AI/player mechanics; costs more semantic vocabulary/tooling and risks primitive proliferation.

**Option B — per-ability authoritative callbacks — REJECTED.** Locally fast but duplicates ordering/RNG/mutation semantics and creates a second mutation engine.

**Option C — generic modifier/event bus — REJECTED.** Extensible but invites registration-order coupling, recursive proc storms and opaque results.

**Option D — closed data-only DSL — REJECTED.** Analyzable but exceptional mechanics force DSL growth into a hidden programming language; bounded Wasm already supplies the controlled escape hatch.

**Risks/mitigations:** stage explosion -> bounded Wasm; Reference-order mismatch -> per-formula versioned order; rounding drift -> SIM descriptors/fixtures; reaction cycles -> budgets/lineage; multi-target races -> explicit snapshot/sequential modes; hidden shield mutation -> typed companion consequences; overhead -> semantics-preserving precompilation/fusion.

**Player impact:** consistent/fair outcomes, bounded proc chains, better support evidence. **Producer impact:** reusable balancing surfaces and safer ruleset evolution at the cost of disciplined typed authoring. **Operational impact:** deterministic lineage and no process-global modifier/RNG authority improve postmortem/abuse analysis.

## Decision timing

**Must decide now: YES.** Reference formula fixtures, broad GAME-ABILITY combat work, AI combat integration, item/condition modifier composition and Studio authoring need one magnitude-resolution boundary.

**Downstream blocked:** Reference formula catalogue/fixtures, broad combat implementation acceptance, `GAME-AI-01` combat-result reasoning and typed item/condition combat modifiers.

**Harder later:** custom callbacks, implicit ordering, hidden RNG and recursive reactions become expensive to normalize after content exists.

**Supersession evidence:** representative mechanics not safely expressible; determinism failures; measured performance after semantic-preserving optimization; Studio production evidence; security/replay evidence favoring another boundary.

**Deliberately not decided:** exact formulas/values; exhaustive taxonomy; physical formula DSL/serializer; exact Reference stage/RNG order; armor/resistance/absorb/crit/block/dodge/proc/lifesteal rules; multi-hit snapshot policy; reactive precedence; numeric representation; protocol/client UI; persistence/runtime layout.

Unresolved Reference-sensitive behavior remains fail-closed.

## Current status

```text
GAME-ABILITY-01 -> REQUIRED_FOR_ALPHA / OPEN
accepted -> typed staged damage/heal composition
accepted -> explicit SIM-owned RNG decisions
accepted -> no mutation during magnitude calculation
accepted -> bounded reactive descendants
next -> exhaustive typed effect families + Reference combat/formula catalogue boundaries
```
