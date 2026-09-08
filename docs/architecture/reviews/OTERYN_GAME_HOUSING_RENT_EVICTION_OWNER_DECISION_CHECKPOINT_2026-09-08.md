# Oteryn Game — Physical-House Rent and Eviction Owner Decision Checkpoint

- Date: 2026-09-08
- Issue: #220
- Reference-comparison evidence: Issue #220 comment `5585489933`
- Owner decision: accepted in the active `Oteryn: architektura` continuation
- Status: `OWNER_SELECTED_DIRECTION / PENDING_CANONICAL_AMENDMENT`
- Mode: architecture analysis/documentation only
- Runtime implementation authority: `NONE`
- Client implementation authority: `NONE`
- DDL/migration authority: `NONE`
- Production authority: `NONE`

## Purpose

Persist the owner-selected rent/eviction lifecycle for scarce physical houses after comparing the current Global Tibia reference model with Oteryn's already accepted World-scoped housing, item-conservation and lifecycle constraints.

This checkpoint adopts the useful structural idea of recurring rent with a protected delinquency period, while explicitly improving value safety and avoiding a Premium-subscription ownership dependency.

It does not freeze numeric rent, cadence, grace duration, notification timing, currency, fees, schema, storage quotas or production rollout.

## Existing binding context preserved

This decision consumes and does not replace the accepted Oteryn housing invariants:

1. one physical `HouseId` belongs to one World and is independent of `ChannelId`;
2. ownership, rent and access are one World-scoped durable state across all Channels;
3. house items have one authoritative state and are never duplicated per Channel;
4. value-bearing mutations remain ordered, revisioned, idempotent/retry-safe and subject to item-conservation authority;
5. stale Channel/runtime state cannot overwrite newer committed house or Character state;
6. risky mutation fails closed when current authority cannot be proven;
7. physical-house ownership does not silently follow Character sale, account transfer, world transfer or deletion/finalization.

The previously selected proxy-auction direction remains separate and compatible: eviction returns the physical property to the same World-scoped allocation/auction domain rather than creating Channel-local supply.

## Owner-selected rent lifecycle

`OWNER_SELECTED`.

A scarce physical house is not a one-time perpetual purchase without maintenance. Continued ownership requires recurring World-economy rent.

The semantic lifecycle is:

```text
active ownership
    |
    v
scheduled rent due
    |
    +-- payment succeeds --------------------> active ownership
    |
    v
payment unavailable / insufficient funds
    |
    v
delinquent + grace period
    |
    +-- payment succeeds during grace -------> active ownership
    |
    v
grace expires unpaid
    |
    v
fenced eviction settlement
    |
    +-- safely evacuate owner-controlled value
    +-- terminate/revoke ownership and ACL authority
    +-- finalize property as vacant
    |
    v
return physical HouseId to World allocation / auction lifecycle
```

The rent obligation exists once per physical `HouseId` in the World. Adding, removing or relocating Channels does not duplicate rent, reset due dates or create another lifecycle.

## Automatic collection direction

`OWNER_SELECTED_DIRECTION`.

Rent should be collected automatically from an explicitly authorized authoritative player-economy funding source rather than requiring the player to be online at the due instant or manually visit an NPC solely to keep the house.

The later economy contract must define the exact funding source, reservation semantics and failure result. This checkpoint intentionally does not freeze whether the source is represented as bank balance, account wallet, Character funds, a dedicated property wallet or another accepted World-economy balance.

Collection must be idempotent: retrying the same rent cycle cannot charge twice.

A timeout or dependency failure is not proof of successful payment and is not, by itself, sufficient reason to evict. The owning service must reconcile authoritative payment/rent-cycle state before progressing to an irreversible transition.

## Grace period

`OWNER_SELECTED`.

Insufficient available funds do not cause immediate eviction.

The house enters an explicit delinquent/grace state that gives the owner a meaningful opportunity to restore payment capacity.

During grace:

- ownership remains identifiable and auditable;
- notifications may be presented through later approved game/portal channels;
- the player does not need to be logged in at a specific server-save instant;
- the same unpaid cycle cannot produce duplicate charges or duplicate eviction jobs;
- later policy may restrict selected value-bearing house mutations if evidence shows that unrestricted use during delinquency enables abuse, but no such restriction is frozen here.

The exact grace duration and reminder schedule remain balance/product decisions.

## Eviction trigger

`OWNER_SELECTED`.

Eviction may begin only after the owning house/rent authority proves that:

1. the correct rent cycle is due;
2. payment has not committed;
3. the grace period for that exact cycle has expired;
4. no later successful reconciliation supersedes the delinquent state;
5. the current ownership revision and `HouseId` still match the eviction operation.

Eviction is therefore a fenced state transition, not a blind timer callback.

## Value-safe eviction

`OWNER_SELECTED`.

Oteryn deliberately improves on reference behavior by requiring lossless authoritative custody of the outgoing owner's durable value before ownership is released.

Before a physical house can become vacant or be assigned to a new owner, the eviction settlement must account for every owner-controlled durable item/location covered by the later housing/item contracts.

The previous owner's items MUST NOT:

- be destroyed merely because rent expired;
- remain silently available for the next owner to claim;
- be copied into both the old-owner reclaim state and the house;
- be reconstructed from client state;
- be left under a stale house-runtime writer after ownership revocation.

Instead, eviction must move eligible durable value through an authoritative typed reclaim/depot/custody transition governed by `GAME-ITEM-01` / `DUR-03` conservation semantics.

The exact reclaim container, capacity policy, UI, access duration and optional fees remain deferred. Capacity limits must not turn a successful eviction into silent item deletion; later design must provide a safe overflow/fail-closed path.

## Ownership release ordering

The safe semantic order is:

```text
freeze/fence outgoing property mutation authority
        -> determine authoritative outgoing item/value set
        -> commit item custody/reclaim transition
        -> revoke outgoing ownership/ACL authority
        -> mark HouseId vacant
        -> publish eligibility for the next auction/allocation lifecycle
```

The implementation may use a saga or transactional composition appropriate to the owning durability boundaries, but it must never expose a completed new-owner assignment while old-owner item custody remains ambiguous.

If settlement is ambiguous, the property remains unavailable for new ownership until reconciliation proves a single authoritative result.

## Relationship to direct house transfer

The same value-safety principle applies to voluntary sale/transfer of a physical house.

Oteryn MUST NOT define ordinary forgotten/remaining house items as automatically transferred to the buyer merely because property ownership changes.

Any explicitly tradable fixture/furnishing category, if Oteryn later wants one, must be a separately typed and intentionally transferable property asset. It cannot arise accidentally from items left on the floor or in a container.

This distinction keeps property transfer semantics separate from generic item ownership/location semantics.

## No Premium ownership dependency

`OWNER_SELECTED`.

Physical-house ownership is not conditioned on maintaining a paid Premium subscription merely to prevent eviction.

A later monetization decision may grant convenience, cosmetics or other explicitly accepted entitlements, but expiration of a commercial subscription MUST NOT by itself perform physical-house eviction or transfer ownership.

This preserves housing as a World/economy/social system rather than making durable property custody depend on an external subscription timer.

## World and Channel behavior

Rent, delinquency, eviction and vacancy are World-scoped.

They MUST NOT be repeated per Channel and MUST NOT vary merely because the owning Character is currently routed through a different same-World Channel or geographic region.

House-runtime failure, Channel drain or region relocation cannot reset rent state or manufacture a new grace period.

The authoritative durable World domain remains the source of truth for ownership/rent state.

## Notifications are advisory, authority is durable state

The product should make delinquency understandable to the player, but notification delivery cannot be the authority for eviction.

A missing email, portal notification, client toast or in-game message does not mutate the durable rent state. Conversely, stale notification content does not prove that the house is still delinquent after payment has committed.

Later UX design should provide clear due/grace/eviction information from an authorized read projection of current durable state.

## Reference-baseline disposition

The Global Tibia comparison performed before this owner decision is retained as evidence in Issue #220 comment `5585489933`.

The Oteryn disposition is:

### Preserve structurally

- recurring rent for scarce physical property;
- automatic collection from an authoritative balance;
- a meaningful non-immediate grace period;
- eviction and reallocation after unresolved delinquency;
- property lifecycle independent of whether the owner is online at the due moment.

### Improve for Oteryn

- durable items are always moved to safe authoritative reclaim/custody before ownership release;
- direct house transfer cannot accidentally transfer ordinary forgotten items;
- timeout/retry is reconciled idempotently rather than treated as payment/eviction proof;
- ownership does not depend on Premium subscription status;
- World-scoped semantics remain independent of Channel count and placement.

### Do not freeze from the reference

- exact monthly/weekly cadence;
- exact grace duration;
- exact monetary values;
- reference-specific account penalties;
- reference-specific Premium eligibility;
- exact depot/inbox representation.

## Deliberately deferred

This checkpoint intentionally does not decide:

- exact rent amount or formula;
- rent cadence;
- payment currency/source representation;
- grace duration;
- number/timing/channel of reminders;
- delinquent-state access/mutation restrictions;
- late fees or penalties;
- re-acquisition cooldowns;
- reclaim storage representation/capacity/fees;
- ownership-count caps;
- account-level anti-speculation rules;
- guildhouse-specific rent modifiers;
- tax/indexation rules;
- DDL/schema/migration;
- production implementation or rollout.

## Decision timing

### Must decide now?

`YES` for the lifecycle shape.

The project needs to know whether physical-house ownership is perpetual or maintained by recurring rent, and whether eviction is destructive or value-safe, before later auction, ownership-cap, item-custody and economy contracts can compose coherently.

### What concrete downstream work is unblocked?

The later `EXP-HOUSES-01` contract can now design:

1. rent accounting and cycle identity;
2. grace-state transitions;
3. value-safe eviction custody;
4. re-auction vacancy handoff;
5. notification/read-model behavior;
6. anti-speculation/ownership-cap policy;

without reopening the fundamental rent-versus-perpetual-property question.

### What remains easy to change later?

All balance and product numbers remain replaceable: rent amount, cadence, grace duration, fees, notifications and reclaim UX.

The harder invariant intentionally frozen now is that ownership release is value-safe and idempotent, because implementing destructive or ambiguous eviction first would create later migration and trust costs.

## Next owner decision

Continue one material owner decision at a time.

The next bounded decision should concern **physical-house scarcity controls / ownership caps**: how many scarce physical houses one underlying player/account may control in one World, including whether guildhouse ownership is counted separately, while keeping the public property owner identity as the already selected `CharacterId` / `GuildId`.

No runtime/client/server/DDL/migration/deployment/production implementation is authorized by this checkpoint.
