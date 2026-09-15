# GAME-ABILITY-01 — Cast, Channel and Commit Owner Baseline

- Status: **OWNER-ACCEPTED PARTIAL BASELINE / GAME-ABILITY-01 REMAINS OPEN**
- Date: 2026-08-13
- Gate: `GAME-ABILITY-01`
- Scope: ability temporal lifecycle, interruption and resource/cooldown commitment semantics
- Runtime/client/protocol/DDL/Platform/production authority: **NONE**

## Decision

An authoritative ability occurrence has an explicit semantic lifecycle and an explicit logical primary commit boundary.

```text
Invocation
-> Prepare / Admission
-> Casting [optional]
-> target / legality / Effect Plan work required for this occurrence
-> PRIMARY COMMIT
-> apply / publish committed resolution
-> Channel / Repeated Occurrences [optional]
-> Completed
```

The diagram is semantic, not a required Rust enum, scheduler or thread layout. A policy may perform bounded target/legality checks at more than one explicit lifecycle anchor, but the final target/effect consequences committed for an occurrence are validated before that occurrence's primary commit.

`PRIMARY COMMIT` is the same authoritative effect-commit concept already accepted by the typed-effect pipeline. It is **not a second mutation engine**. The post-commit `apply / publish committed resolution` step means materializing and reporting the already validated committed plan; it does not permit new hidden target selection, formula interpretation or arbitrary world re-query after commit.

`PRIMARY COMMIT` is a logical simulation/domain boundary; this baseline does **not** claim that it is one database transaction, one thread operation or one machine instruction.

## Prepare and pre-commit validation

Prepare/Admission may normalize the invocation, check capabilities/prerequisites, perform target resolution/legality work required at that stage and establish explicitly modelled reservations when a later policy needs them.

A pre-check is not automatically a permanent guarantee. Long-running casts may require authoritative revalidation at a later explicit anchor. Exact target snapshot/re-resolution and prerequisite revalidation timing remain versioned Reference/ruleset policy and are not frozen here.

Client prediction, cast bars and cancellation requests remain intent/presentation only. The client does not decide that a cast committed, completed or was successfully interrupted.

## Explicit commitment anchors

The primary effect commit is not necessarily the only commit anchor in an ability lifecycle. Every behavior-affecting cost, cooldown, charge or similar consumptive side effect uses its own explicit versioned anchor policy.

Possible semantic anchors include invocation/admission, cast start, `PRIMARY COMMIT`, completion or an explicit channel occurrence. This list is illustrative, not an exhaustive physical representation.

There is no implicit global rule that all mana, consumables, cooldowns or charges commit at the same phase, and an ancillary side effect may commit earlier or later than the primary effect plan when the accepted Reference/ruleset policy requires that behavior.

Once any anchor commits its named side effect, that side effect is authoritative history even if the overall ability later fails, is interrupted or is cancelled. A Reference mechanic may therefore reproduce observed timing without forking the protocol or creating a second execution engine, while Evolved rulesets may select different versioned policies where explicitly allowed.

Anchor selection is part of behavior-affecting semantic revision/provenance and must be available to deterministic replay/evidence.

## Reservation is explicit, not hidden rollback

A future rule may reserve a resource before `PRIMARY COMMIT` when necessary to prevent races or reserve availability of that specific resource for a later anchor. Such a reservation does **not** guarantee that the ability will remain legal: targets, world state, source state and other prerequisites may still invalidate the occurrence under later policy.

Reservation must be an explicit bounded state with a named owner and release/consume policy; it is not speculative mutation hidden behind an eventual refund.

If an occurrence ends before the relevant commit anchor, only explicitly modelled reservations may be released according to policy. The engine must not silently undo arbitrary authoritative mutations to simulate a pre-commit cancellation.

After a side effect has committed, a later reversal/refund is a new explicit compensating domain action/effect with its own authority, ordering, conservation and audit evidence. It is not deletion or rewriting of committed history.

Item, currency or durable-value reservations/consumption/compensation remain subordinate to `GAME-ITEM`/`DUR-03` ownership, fencing, idempotency and conservation rules.

## Interruption and cancellation

Interruption and cancellation are authoritative state transitions evaluated by server policy against ordered simulation state.

When an interrupt, movement, damage event, death, explicit cancellation or other relevant cause competes with a commit anchor, the result is determined by the authoritative event/order contract from FND-03/SIM. Network arrival races, thread scheduling, unsynchronised wall-clock observation or client presentation timing may not decide the outcome.

Cancellation/interruption after the primary effect commit may prevent later **uncommitted** channel occurrences or other future work when policy permits, but it cannot erase already committed primary or ancillary consequences.

The exact causes that can interrupt a particular ability, interruption precedence, failure messages, penalties and Reference behavior remain later versioned policy.

## Channeling and repeated effects

Channeling is not an unbounded script loop with direct mutation authority.

Each authoritative channel pulse/tick/repeated application is an explicit bounded deterministic occurrence or sub-occurrence under the same GAME-ABILITY pipeline. As applicable it performs its policy-selected target resolution/revalidation, legality, cost/charge anchor, typed Effect Plan validation and primary commit steps.

A channel definition must eventually have explicit bounds and termination rules. Exact tick cadence, maximum count/duration, whether targets are sticky or re-resolved, per-tick versus upfront costs and interruption behavior remain later decisions.

Wasm/WIT extensions remain proposal-only and may not create private timers, hidden commit points, unbounded repeated mutation loops or bypass the authoritative lifecycle.

## Authoritative future state and replay

A cast/channel state that can affect future authoritative outcomes is authoritative simulation state for ordering and replay purposes. Deterministic evidence must be able to explain at least the relevant semantic revision, lifecycle phase/anchor, normalized inputs, ordered interrupt/commit decisions, resource/cooldown/charge commitments and produced effect/result events.

This baseline does not decide whether an in-progress cast/channel survives logout, reconnect, process crash, channel recovery or migration. If a future contract allows continuation across such a boundary, it must preserve the exact behavior-affecting revisions and enough authoritative state to continue without silent reinterpretation.

## Relationship to targeting and effect planning

The previously accepted targeting boundary remains authoritative. This lifecycle does not let casting or channel logic secretly choose a different target set inside effect application.

Target resolution/legality may occur at explicit lifecycle anchors according to later policy, but any re-resolution must use the accepted bounded deterministic resolver. For each occurrence, Effect Plan generation/validation consumes only authoritative validated targets before the plan's primary commit.

## Failure semantics

Structured failure before a named commitment anchor does not imply a refund because no hidden consumption is assumed. It may release an explicit reservation if one exists.

Failure after one or more committed consequences does not retroactively erase those consequences. Any compensation must be explicit. This property is required for deterministic replay, anti-duplication, auditability and recovery correctness.

## Decision timing

**Must decide now: YES.** Cooldown/charge architecture, condition lifecycle, channeled mechanics, AI ability execution, Reference parity fixtures and Studio authoring all need one common model of time, interruption and commitment.

Without this boundary, individual spells could invent incompatible refund, rollback and channel-loop behavior that would be expensive to reconcile later.

## Deliberately not decided

Not decided here: global cooldown existence/model; exact cast/channel times; exact mana/stamina/item/currency costs; cooldown/charge durations; refund percentages; interruption causes/precedence; target snapshot/revalidation timing; channel cadence/count; death/logout/reconnect/crash continuation; client cast-bar UX; scheduler/tick implementation; physical persistence schema; exact Reference formulas/catalogue; protocol layout; physical content authoring format.

Unresolved Reference-sensitive behavior remains fail-closed.

## Supersession

Reopen only with representative-mechanic, Reference-parity, concurrency, recovery, performance or authoring evidence showing this model cannot express required behavior safely. Any supersession must preserve or explicitly replace authoritative ordering, deterministic replay, explicit commitment/reservation semantics, conservation and no-hidden-rollback guarantees.

## Current status

```text
GAME-ABILITY-01 overall
-> REQUIRED_FOR_ALPHA / OPEN

accepted partial baselines
-> data-first typed Effect Plan + bounded Wasm proposals
-> deterministic Target Resolver + separate Legality
-> explicit ability lifecycle + logical primary commit
-> versioned cost/cooldown/charge anchors
-> explicit reservation / explicit compensation
-> bounded deterministic channel occurrences

next paper-only decision
-> cooldown/charge scopes and condition lifecycle semantics
```
