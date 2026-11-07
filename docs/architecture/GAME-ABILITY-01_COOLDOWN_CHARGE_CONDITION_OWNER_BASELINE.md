# GAME-ABILITY-01 — Cooldown, Charge and Condition Lifecycle Owner Baseline

- Status: **OWNER-ACCEPTED PARTIAL BASELINE / GAME-ABILITY-01 REMAINS OPEN**
- Date: 2026-08-13
- Gate: `GAME-ABILITY-01`
- Scope: cooldown/charge runtime semantics and condition lifecycle boundaries
- Runtime/client/protocol/DDL/Platform/production authority: **NONE**

## Decision

Cooldowns, rechargeable ability charges and gameplay conditions are authoritative typed state governed by the same deterministic GAME-ABILITY execution model. They are not ad-hoc timers, script-owned mutable state or process-global gameplay registries.

The accepted semantic separation is:

```text
Ability / mechanic occurrence
-> typed cooldown / charge / condition transition proposal
-> applicable Target / Legality / condition admission-conflict evaluation
-> typed Effect Plan validation
-> PRIMARY COMMIT
-> authoritative cooldown / charge / ConditionInstance transition
-> typed modifiers and/or bounded future occurrences
-> future mutating occurrences re-enter the same Effect Plan / commit pipeline
```

`ConditionDefinition` remains immutable/versioned content semantics; `ConditionInstance` is authoritative runtime state created or changed only through the authoritative lifecycle described below.

## Cooldown state and scope

A cooldown is keyed typed gameplay state, not a free-form timestamp attached to an ability implementation.

A versioned policy identifies the semantic cooldown key/domain and the authoritative subject whose availability is restricted. Supported policies may express ability-specific, explicitly shared cooldown-group, and broader action/global-style restrictions when required by verified Reference behavior or an accepted ruleset. This baseline does **not** require such broader scopes to exist. Here `global-style` means broad within an explicitly named gameplay subject/domain (for example that actor's action set); it does not authorize process-global, node-global, world-global or cross-Channel mutable state.

Cooldown state must have a named authoritative owner consistent with FND-03. Process-global mutable cooldown maps are forbidden. Cross-Channel or cross-Instance cooldown effects are not implicitly authorized by sharing a textual key.

The previously accepted commit-anchor model applies: an ability/mechanic-driven cooldown start, extension or consumption is a named committed side effect at an explicit versioned anchor; it cannot be written directly by content or script code. Exact durations, grouping and anchor choices remain Reference/ruleset policy.

A cooldown becoming ready is a deterministic owner-local lifecycle consequence of authoritative simulation/time semantics. Wall-clock observation, client timers or thread wake-up order may not decide readiness.

## Charge pools

Rechargeable ability charges are a separate semantic concept from cooldowns and from item-stack/item-charge state.

A `ChargePool`-like semantic policy may define bounded capacity, current authoritative count, consumption anchor and deterministic recharge policy. The exact physical type/name is not frozen.

Ability/mechanic-driven charge consumption is a named committed side effect at an explicit anchor. Recharge is an authoritative owner-local lifecycle transition that may be represented as bounded scheduled occurrences or an equivalent deterministic derivation. Both must preserve replayable ordering and exact behavior-affecting revision/provenance.

Sharing a charge pool across abilities must be explicit through a typed shared semantic key/policy. Accidental sharing through string equality or implementation-global state is forbidden.

Item charges, consumable stacks and durable-value ownership remain governed by `GAME-ITEM` / `DUR-03`; an ability charge pool cannot silently alias or mutate those values.

## Condition Definition versus Condition Instance

`ConditionDefinition` is the immutable/versioned semantic definition of a condition family or concrete content mechanic. It may describe typed behavior such as duration policy, conflict/stack policy, typed modifier contributions, bounded scheduled occurrences, tags/categories and dispel/immunity/resistance/suppression participation.

`ConditionInstance` is an authoritative runtime occurrence bound to the exact behavior-affecting definition revision. It carries only runtime state needed to continue deterministic behavior, such as the affected subject, provenance/source, start/expiry state, stack or potency state where applicable, and bounded tick/occurrence schedule state.

An ability, item, creature mechanic, script proposal or other external gameplay occurrence may not create, refresh, stack, replace, transform, suppress, unsuppress or remove a `ConditionInstance` directly. It produces a typed condition transition effect/action that is admitted, conflict-resolved, included in the validated Effect Plan where applicable and changed only at the authoritative commit boundary.

The exact Rust type graph, ID widths and serializer are not frozen. A runtime condition occurrence must nevertheless be distinguishable enough for deterministic ordering, audit evidence and explicit removal/transition decisions; container position or pointer identity cannot be semantic identity.

## Condition provenance

A runtime condition must preserve sufficient provenance to explain how it was admitted and how future occurrences should behave. Where relevant this includes source entity/context, source ability/mechanic revision, ruleset/profile revision and condition definition revision.

A later source disappearing, dying, logging out or migrating does not silently reinterpret already admitted condition semantics. Exact source-liveness dependencies remain explicit versioned policy.

## Conflict, stacking and refresh

Condition collision behavior must be explicit and deterministic. There is no implicit last-write-wins, hash-order-wins or newest-packet-wins rule.

A versioned condition policy may choose bounded semantics equivalent to such families as:

- reject the new occurrence;
- refresh or extend an existing occurrence;
- replace an existing occurrence;
- merge/update bounded stack or potency state;
- allow multiple bounded instances.

These names are semantic examples, not a frozen enum. Every accepted policy must define the matching/conflict domain, source partitioning when relevant, deterministic tie-breaking, maximum cardinality/stack bounds and which revision controls the resulting future behavior.

Admission/conflict evaluation produces a deterministic typed transition decision before authoritative mutation. Refresh/replace/merge becomes authoritative only at the relevant commit/lifecycle boundary and must emit explicit evidence. It must not silently erase already committed effects produced by the previous state.

## Condition effects and ticks

Conditions do not receive a second mutation engine.

A condition may contribute typed, bounded modifiers to authoritative rule evaluation and/or schedule future typed occurrences. Any condition tick, DoT, HoT, pulse, proc or repeated application that mutates gameplay state must pass through the same applicable target/legality, typed Effect Plan validation and authoritative `PRIMARY COMMIT` boundaries already accepted for abilities.

Condition code or Wasm/WIT may not directly mutate Character, Item, World, cooldown, charge or condition-instance state; create private unbounded timers; select hidden targets during effect application; or bypass commit anchors.

Scheduled condition occurrences must be bounded, deterministically ordered and independent of unsynchronised wall-clock/thread scheduling. Exact cadence and simulation scheduler implementation remain later decisions.

## Expiry and removal

Time/policy-driven expiry is an explicit deterministic owner-local lifecycle transition, not a client timer callback or arbitrary script mutation. It is ordered with other authoritative state transitions and emits typed evidence.

Ability-/mechanic-driven removal, dispel, cleanse, replacement, transformation, suppression or unsuppression must arrive as a typed authoritative effect/action and cannot mutate the condition collection out of band. Death/world-transition policy or other internal lifecycle causes must likewise use the same named owner and deterministic ordering boundary.

Removing, expiring, replacing or dispelling a condition is forward-only. It can prevent future uncommitted contributions/occurrences but does not retroactively erase already committed ticks, effects or audit history.

If a mechanic requires a compensating restoration after committed consequences, that restoration is a new explicit typed effect/action under the previously accepted compensation rules.

## Immunity, resistance, suppression and dispel

These concepts remain distinct typed policy layers rather than spell-specific arbitrary branches:

- **Immunity** determines whether a relevant condition/effect family is admissible at the applicable legality/admission boundary.
- **Resistance** deterministically modifies an otherwise admissible mechanic according to explicit policy, for example magnitude, duration, chance or another typed parameter; it is not implicit immunity.
- **Suppression** keeps an authoritative condition instance or typed contribution present while policy temporarily prevents selected effects/contributions from applying. Whether duration/tick clocks continue, pause or transform while suppressed must be explicit policy. Ability-/mechanic-driven suppression changes use the same typed transition/commit boundary rather than direct collection mutation.
- **Dispel/Cleanse** is a typed authoritative action/effect that deterministically selects eligible existing condition instances/contributions and removes or transforms them only through the authoritative condition transition boundary.

This baseline freezes the separation, not exact evaluation precedence, categories, formulas or Reference behavior. Those remain evidence-driven later decisions and fail closed where Reference parity is unresolved.

## Modifier safety

A condition modifier is a typed contribution to an authoritative calculation/rule surface, not an arbitrary callback with unrestricted world access.

Examples may eventually include movement speed, damage/healing modifiers, resource regeneration, capability gates, resistances or other bounded mechanic-specific inputs. The exhaustive modifier catalogue and combination/precedence algebra are deliberately deferred to effect-composition/combat architecture.

Modifier combination must eventually define deterministic ordering and bounded composition; no dependence on registration order, memory address or unordered container iteration is allowed.

## Determinism and ownership

Cooldown, charge and condition transitions obey FND-03 and SIM-DETERMINISM:

- one authoritative mutation owner for the relevant runtime state;
- exact behavior-affecting semantic revisions are preserved;
- ties and simultaneous transitions use explicit authoritative ordering;
- result cardinality, stack count, pending recharge/tick occurrences and dynamic work are bounded;
- client clocks, network arrival races and thread scheduling do not become gameplay authority.

Moving gameplay ownership across Channel/Instance/Node boundaries does not implicitly make cooldown/condition state process-global. Any transfer/recovery contract must preserve fencing and exact semantic state.

## Recovery and persistence boundary

This baseline does not decide which cooldowns, charge pools or conditions survive logout, reconnect, channel recovery, process crash, migration or server restart.

If a future policy requires survival across a boundary, the durable/recovery contract must preserve enough authoritative state and exact revisions to resume without extending, shortening, duplicating, dropping or reinterpreting commitments/ticks/recharges.

If a policy declares a state session-local and non-surviving, that behavior must likewise be explicit and Reference-compatible where applicable.

## Audit and Game Intelligence

ANL-01 remains observational/read-only. Typed evidence should be able to distinguish, as relevant:

- cooldown started/extended/became ready;
- charge consumed/recharged/capped;
- condition transition proposed/admitted/rejected;
- condition refreshed/replaced/stacked/merged;
- suppressed/unsuppressed;
- dispelled/cleansed/expired/removed;
- scheduled tick/occurrence attempted, committed or failed.

Evidence must preserve enough semantic revision/provenance to explain parity, balance and anti-abuse behavior without giving analytics mutation authority.

## Wasm/WIT

DUR-04 capability boundaries remain unchanged. Wasm/content may propose typed condition/cooldown/charge behavior only through approved bounded surfaces. It may not own authoritative timers, mutate runtime condition collections directly, forge committed cooldown/charge transitions or create an alternative stacking/dispel engine.

## Decision timing

**Must decide now: YES.** Damage/healing composition, buff/debuff semantics, AI ability usage, combat Reference fixtures, item/spell interactions and Studio authoring need one stable distinction between definitions, runtime occurrences, availability state and future scheduled effects.

Without this boundary, each ability could invent incompatible cooldown groups, condition timers, stacking and dispel behavior, creating replay, anti-duplication, parity and migration risk.

## Deliberately not decided

Not decided here: exact cooldown/global-cooldown groups or durations; exact charge capacities/recharge cadence; physical cooldown/condition IDs or serializer; exact condition conflict key rules; exact poison/fire/energy/haste/paralyze/buff/debuff values; tick cadence; immunity/resistance/suppression/dispel precedence; chance formulas; typed modifier catalogue and algebra; logout/reconnect/restart persistence; scheduler/spatial/runtime implementation; client status UI; protocol layout; database schema; Reference spell/condition catalogue.

Unresolved Reference-sensitive behavior remains fail-closed.

## Supersession

Reopen only with representative-mechanic, Reference-parity, replay, anti-duplication, concurrency, recovery, performance or authoring evidence showing this boundary cannot safely express required behavior. Supersession must preserve or explicitly replace typed state ownership, bounded deterministic transitions, exact revision/provenance, no hidden rollback and no second mutation engine.

## Current status

```text
GAME-ABILITY-01 overall
-> REQUIRED_FOR_ALPHA / OPEN

accepted partial baselines
-> data-first typed Effect Plan + bounded Wasm proposals
-> deterministic Target Resolver + separate Legality
-> explicit cast/channel lifecycle + logical PRIMARY COMMIT
-> versioned cost/cooldown/charge anchors
-> typed cooldown and distinct ChargePool state
-> ConditionDefinition != ConditionInstance
-> ability-driven condition transitions commit as typed effects/actions
-> explicit bounded conflict/stack/refresh/replace policies
-> condition ticks use the same Effect Plan / commit pipeline
-> distinct immunity / resistance / suppression / dispel layers

next paper-only decision
-> effect composition and damage/heal calculation pipeline
```
