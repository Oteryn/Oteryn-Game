# Oteryn Game — Hybrid Housing Owner Decision Checkpoint

- Date: 2026-09-08
- Issue: #220
- Source owner decision comment: Issue #220 comment `5584904453`
- Status: `OWNER_SELECTED_DIRECTION / PENDING_CANONICAL_AMENDMENT`
- Mode: architecture analysis/documentation only
- Runtime implementation authority: `NONE`
- Client implementation authority: `NONE`
- DDL/migration authority: `NONE`
- Production authority: `NONE`

## Purpose

Persist the owner-selected Oteryn housing topology/scarcity direction reached after the dedicated housing/auction/scarcity/multichannel deep-dive, while keeping the design deliberately reversible and without silently rewriting accepted World/Channel, item, durability, instance or economy contracts.

This checkpoint selects the product/architecture direction only. It does not freeze final database schema, rent/auction numbers, residence identity, hosting technology, physical format, migration mechanics or production rollout.

## Existing binding invariants preserved

The decision consumes and does not replace the current accepted housing safety invariants in `MULTICHANNEL_SYSTEM_SCOPE_MATRIX.md`:

1. `HouseId` belongs to a World and does not contain `ChannelId`.
2. Ownership, rent and access lists exist once per World.
3. House items have one authoritative state, not one copy per Channel.
4. House item mutations remain ordered, revisioned and retry-safe.
5. Entering or leaving a house cannot silently change the Character's gameplay Channel.
6. A stale Channel cannot overwrite newer house or Character state.
7. Risky house mutations fail closed when the authoritative house service/runtime is unavailable.

The decision also preserves the accepted first-generation durable topology: one World `DurableHomeRegion` / one authoritative PostgreSQL write domain for World-shared durable mutations, while ordinary Channel simulation remains region-local and does not add synchronous WAN work to the authoritative gameplay tick.

## Owner-selected hybrid model

`OWNER_SELECTED`.

Oteryn will pursue a hybrid housing model with two distinct product classes.

### A. Physical world houses

Physical houses are finite, visible, prestigious World landmarks/addresses.

They preserve one canonical `HouseId` and one authoritative World-scoped state across every Channel of the same `WorldId`.

Adding, removing, draining or relocating a Channel MUST NOT:

- create another copy of the physical house;
- create another auction or rent lifecycle for the same house;
- duplicate house items;
- create another ACL/ownership record;
- change physical-house scarcity or supply.

Physical-house scarcity is therefore a deliberate product/economy property, not a side effect of runtime Channel capacity.

### B. Non-scarce residences / apartments

Oteryn may provide a separate scalable residence/apartment class for baseline housing utility that should not depend on owning one of the finite physical addresses.

The intended product purpose is broad access to capabilities such as, where later contracts permit them:

- personalisation/decorating;
- private or permissioned space;
- storage/convenience;
- Rested/bed utility;
- social invitation/access.

This checkpoint does not select the final canonical identifier, schema, rent/payment model, map representation, UX or persistence representation for residences. The later `EXP-HOUSES-01` contract must name these explicitly instead of overloading `HouseId` or assuming the physical-house lifecycle automatically applies.

## Physical-house interior runtime direction

`OWNER_SELECTED_DIRECTION`.

A physical house must not be independently simulated once per Channel.

The preferred semantic model is:

```text
World-visible physical address / HouseId
             |
             v
explicit house-entry ownership transition
             |
             v
one authoritative World-scoped house interior runtime
             |
             v
explicit exit / recovery transition
             |
             v
validated preserved origin Channel
```

Characters entering the same physical `HouseId` from different Channels converge on the same authoritative house interior/presence and observe the same authoritative item state.

House entry does not itself change the Character's World or silently perform an ordinary Channel switch. Normal exit returns through validated preserved origin-Channel routing unless a later accepted recovery policy explicitly selects another same-World safe destination.

## Relationship to accepted instance architecture

The house interior should reuse the already accepted **instance-style ownership-transfer/fencing mechanics** where useful:

- explicit prepared/committed ownership transition;
- one logical authoritative runtime writer;
- unique transfer identity;
- generation fencing;
- idempotent retry/recovery;
- retained origin routing metadata;
- stale-owner rejection;
- full authoritative resynchronisation where required.

This reuse MUST NOT redefine canonical property identity.

`HouseId` remains the semantic property/ownership identity. If an implementation internally uses `InstanceRuntime` infrastructure or an `InstanceId` for one concrete runtime lifecycle, that runtime identity is subordinate and MUST NOT replace `HouseId` in ownership, rent, auction, ACL, item custody or durable property history.

## Presence and Channel semantics

For a physical house:

- house interior presence is shared across Channels of one World because there is one authoritative interior runtime;
- two Characters entering the same physical house from different source Channels may meet inside that authoritative house context;
- origin `ChannelId` remains routing/recovery metadata rather than house identity;
- entry/exit cannot be used as a hidden Channel-hop mechanism;
- a Character remains subject to one-authoritative-simulation-owner fencing through entry, interior presence and exit.

Exact visibility, invitation, occupancy and concurrent-player limits remain deferred.

## Ownership, ACL and guild semantics

Physical-house ownership, rent, transfer and access policy remain World-global.

The later `EXP-HOUSES-01` contract must define at least:

- allowed owner identity types (for example Character, Account, Guild, or a stricter subset);
- co-ownership if any;
- guest/access-list and role semantics;
- guildhouse ownership/administration;
- permission revisioning;
- permission revalidation on entry and value-bearing mutation;
- revocation behavior for Characters currently inside;
- bed occupancy/access semantics;
- offline-owner lifecycle.

Client possession of stale ACL state never grants mutation authority.

## Item and value safety

Physical-house item state remains subject to `GAME-ITEM-01` and `DUR-03`.

Every live durable item has one authoritative immediate location. House placement, pickup, container movement, transfer, eviction and recovery must preserve conservation and idempotency.

The system MUST NOT resolve transfer/eviction/failure by:

- producing Channel-local copies;
- trusting client inventory or house state;
- replaying a mutation without a stable idempotency/transaction identity;
- allowing a stale house runtime or source Channel to overwrite a newer committed state.

The exact reclaim/depot/escrow mechanism for items during ownership transfer or eviction is deliberately deferred to the owning house/economy/durability decision.

## Auction, rent and scarcity consequences

Physical-house auction/ownership/rent lifecycle is one World-scoped lifecycle per `HouseId`.

Adding/removing Channels MUST NOT duplicate auctions, reset rent, create extra supply or alter ownership.

A later contract should use an idempotent, revision-fenced settlement boundary for ownership transfer and must explicitly specify the item-custody consequence before ownership changes become final.

The following remain `BALANCE / ECONOMY DEFERRED`:

- auction mechanism;
- bid currency and reservation semantics;
- minimum/maximum bids;
- anti-sniping behavior;
- rent amount and cadence;
- taxes/fees;
- grace and eviction periods;
- anti-speculation rules;
- owner eligibility;
- house-count limits per Character/account/guild;
- reclaim/storage fees.

## Rested and fairness consequence

This hybrid model intentionally prevents scarce physical real estate from becoming the only route to core Rested/recovery utility.

The current Rested direction already permits baseline offline recovery and public-inn recovery independently of physical-house ownership. Physical house/guildhall beds may remain a stronger convenience class under later balance evidence, while non-scarce residences may provide baseline housing/rest functionality without creating a permanently weaker class of non-house owners.

This does not freeze exact recovery multipliers or residence entitlements.

## Cross-region and failure model

Do not independently mirror one mutable physical-house interior into each geographic Channel region.

The architecture requires one authoritative runtime owner for one active physical-house interior context. Physical placement may change later, but relocation/failover must be fenced so a stale runtime cannot resume authoritative mutation.

World-shared durable ownership/item truth remains in the accepted durable authority. A Character using a house from a remote Channel region may therefore experience explicit WAN latency at durable house/item boundaries in the first-generation topology, but this must not move WAN round trips into ordinary regional Channel combat/movement simulation.

Risky mutations fail closed when current house authority cannot be proven. Exact read-only degraded presentation policy may be decided later.

## Reversibility contract

`OWNER_SELECTED`.

The hybrid model is intentionally adopted as a reversible architecture direction rather than an irreversible final housing implementation.

To preserve reversibility, future implementation MUST keep these boundaries separate:

```text
physical HouseId / physical-house product semantics
!= residence/apartment identity and product semantics
!= runtime-lifecycle identity
!= ChannelId
!= durable item identity/location
```

This checkpoint deliberately does not freeze:

- a shared database table/schema for both housing classes;
- residence identity representation;
- a permanent interior hosting primitive;
- an auction/rent formula;
- permanent physical-house supply counts;
- permanent residence availability rules;
- final entry/loading UX;
- production deployment topology.

A future owner decision may therefore:

- keep the hybrid model;
- remove or narrow the scalable residence class;
- expand the residence class into the primary player-housing system;
- reduce physical houses to prestige/social-only functionality;
- supersede the interior runtime implementation while preserving world-global property semantics.

Such a future decision still requires explicit migration/compatibility treatment for any durable ownership/items already created under the then-active model.

## Rejected directions

### Per-Channel copies of one physical house

`REJECTED`.

This conflicts with existing World-scoped `HouseId`, one authoritative item state and one ownership/ACL/rent lifecycle, and creates multi-writer/duplication/cross-region consistency hazards.

### Channel-local physical HouseId / ownership

`REJECTED`.

This contradicts current accepted World-scoped house semantics and would make operational Channel scaling silently alter housing supply/economy.

### Mandatory scarce physical housing for baseline progression utility

`REJECTED_DIRECTION`.

Finite physical houses may remain scarce prestige/social/economy objects, but scarce ownership must not be the sole route to core baseline recovery/housing utility for the wider player population.

### Fully non-scarce instanced housing as the only housing product

`NOT_SELECTED`.

Technically valid and scalable, but not selected as the sole model because it would discard the physical-world prestige/address/scarcity value the owner wants to preserve. It remains a future supersession option if product evidence proves the physical-house layer is not valuable enough.

## Decision timing

### Must decide now?

`YES` for the topology/scarcity split and the prohibition on per-Channel physical-house duplication.

This decision is needed before later housing, world, item, economy and Rested work accidentally couples durable housing identity or supply to Channel topology.

### What concrete downstream work is unblocked?

The later `EXP-HOUSES-01` analysis can now separately design:

1. physical-house ownership/auction/rent lifecycle;
2. physical-house ACL/guild/bed semantics;
3. physical-house item custody/eviction/recovery;
4. scalable residence/apartment product semantics;
5. runtime placement/capacity/failure policy;
6. client UX and content representation;

without reopening whether physical houses are duplicated by Channel.

### What becomes harder later?

The largest migration cost would appear only after persistent house/residence ownership or item custody is implemented. Therefore schema/identity/lifecycle details stay deferred now and future supersession must provide explicit durable migration when those states exist.

### Evidence that may justify supersession

- player testing showing physical-house scarcity creates more frustration than prestige/social value;
- economy telemetry showing auctions/rent create harmful concentration/speculation;
- operational evidence showing one authoritative physical-house runtime has disproportionate latency/cost;
- player demand demonstrating scalable residences make physical houses redundant;
- abuse/duplication/security findings;
- a later explicit Oteryn product-direction decision.

## Deliberately unresolved next decisions

- physical-house owner identity class;
- auction versus other allocation mechanism;
- bidding/settlement lifecycle;
- rent/fees/taxes and grace/eviction;
- house transfer item settlement/reclaim;
- ownership caps and anti-speculation policy;
- guildhouse semantics;
- ACL/co-owner/guest revision model;
- bed occupancy and Rested modifiers;
- scalable residence identity and acquisition model;
- scalable residence storage/decorating limits;
- interior occupancy/capacity;
- exact runtime placement/failover strategy;
- DDL/schema/migration;
- production rollout.

## Next owner decision

Continue one material decision at a time. The next bounded question should concern **physical-house ownership and allocation lifecycle** — specifically who/what may own a physical `HouseId` and whether allocation begins from a Tibia-like auction model or another explicit World-economy mechanism.

No runtime/client/server/DDL/migration/deployment/production implementation is authorized by this checkpoint.
