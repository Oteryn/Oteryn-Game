# Oteryn Game — Residence Free-Access Owner Decision Checkpoint

- Date: 2026-09-08
- Issue: #220
- Status: `OWNER_SELECTED_DIRECTION / PENDING_CANONICAL_AMENDMENT`
- Mode: architecture analysis/documentation only
- Runtime implementation authority: `NONE`
- Client implementation authority: `NONE`
- DDL/migration authority: `NONE`
- Production authority: `NONE`

## Purpose

Persist the owner-selected first-generation split between scarce physical houses and non-scarce residences/apartments.

The product goal is to let a non-Premium player have a real personal housing space without turning infrastructure scale into additional copies of scarce world addresses and without removing the prestige/economy role of physical houses.

## Existing housing direction preserved

This checkpoint preserves the already selected housing model:

- finite physical `HouseId` addresses are World-scoped and shared across Channels;
- physical-house scarcity does not scale with Channel count;
- physical houses use the public World auction lifecycle;
- acquiring an ordinary physical house requires active Premium plus `PhysicalHouseEligibility` under the separate eligibility decision;
- expiration of Premium after legitimate acquisition does not itself evict an existing physical-house owner;
- ordinary physical-house ownership remains subject to in-game rent/grace/eviction rules;
- physical-house ACL is managed through a GUI/panel rather than Tibia-style text commands;
- guildhouse semantics remain deferred to the future guild-system architecture.

## Owner-selected residence direction

`OWNER_SELECTED`.

Oteryn should provide a separate non-scarce residence/apartment housing class that can be available to players without Premium.

Conceptually:

```text
non-Premium / Free player
        |
        v
reasonable gameplay eligibility
        |
        v
non-scarce Residence
        |
        +-- personal housing space
        +-- decoration/personalization
        +-- GUI-managed access
        +-- baseline housing utility
```

This residence class is not a physical `HouseId` copied into multiple Channels. It is a separate housing product/identity whose exact final name and schema remain deferred.

## Scarcity boundary

A residence/apartment is intentionally non-scarce.

Creating another residence for an eligible player must not:

- create another copy of an existing physical world address;
- change the supply or auction price of physical houses;
- consume the ordinary physical-house ownership allowance;
- depend on Channel count;
- create a private market in scarce addresses.

The scarce/prestige layer remains the finite physical-house system.

## Free-access principle

Active Premium is not required merely to obtain or retain the baseline non-scarce residence class.

The exact Free-player eligibility threshold is deliberately deferred, but it may require reasonable evidence of real gameplay progression/participation so that trivial throwaway accounts do not receive unlimited durable storage or operational surface for free.

The eligibility rule must be designed for normal players first and must not turn the residence into a disguised Premium requirement.

## Relationship to Premium

Premium remains meaningful because it gates acquisition of scarce physical houses under the separate physical-house eligibility decision.

The intended product split is:

```text
Free / non-Premium
  -> accessible non-scarce personal residence

Premium + PhysicalHouseEligibility
  -> may additionally compete for scarce physical world houses
```

Premium may later add residence convenience, cosmetic catalog, layout/preset or presentation features, but the baseline residence itself remains a genuine usable housing space rather than a crippled advertisement for Premium.

No Premium residence upgrade is allowed by this checkpoint to create combat power or duplicate scarce physical `HouseId` supply.

## Residence exchange model

The first-generation residence is not a speculative property asset.

It does not require:

- a public scarcity auction;
- player-to-player resale;
- private house nomination/transfer;
- a market price derived from limited address supply.

A later owner decision may introduce additional residence lifecycle/economy mechanics if evidence justifies them. They are not inferred here.

## Basic utility direction

The residence should support the recognizable baseline idea of having a home:

- a persistent personal interior;
- decoration/personalization;
- a graphical housing/access panel consistent with the selected housing ACL UX;
- baseline social/access capability;
- compatibility with the later accepted Rested/bed design.

Exact feature limits remain deferred.

## Storage safety and anti-alt boundary

This checkpoint does not authorize residences to become effectively unlimited free storage accounts.

Any residence-specific storage, item-placement capacity or durable object budget must be bounded by the later item/storage/economy contract and preserve one authoritative semantic location for every durable item.

Exact capacity numbers and whether residence storage is distinct from normal account/depot storage remain deferred.

## Rested / bed boundary

A non-Premium residence may participate in the baseline Rested/housing recovery model, but this checkpoint does not freeze:

- the number of beds;
- the Rested recovery rate;
- whether a bed is required for all residence Rested recovery;
- Premium modifiers;
- offline-training semantics;
- bed permissions beyond the already selected GUI/capability direction.

Those remain a later owner decision tied to the Rested system.

## Runtime/topology boundary

This decision selects the product class, not the final hosting implementation.

A residence may later use an instance-style runtime primitive, but:

- it must have one authoritative runtime owner while active;
- durable item/state mutation remains in the World durable authority;
- origin Channel/routing semantics must remain explicit and fenced;
- Channel scaling must not duplicate durable residence contents;
- exact `ResidenceId`, runtime identifier, hosting placement and persistence schema remain deferred.

## What is deliberately deferred

This checkpoint does not freeze:

- final product name (`Residence`, `Apartment`, or another term);
- `ResidenceId` schema;
- exact acquisition quest/level/account-age/playtime threshold;
- whether the entitlement is per Character, per Account, or another bounded scope;
- exact number/size/layout of residence templates;
- rent or maintenance fee, if any;
- exact storage/item-placement budgets;
- exact ACL capability list beyond the selected housing GUI direction;
- bed/Rested values;
- Premium cosmetic/convenience upgrades;
- monetization prices;
- DDL/migrations;
- runtime/client/server implementation;
- production rollout.

## Decision timing

### Must decide now?

`YES` for the product/scarcity split.

Without this decision, later housing work could accidentally make scarce physical houses the only route to basic housing utility or could manufacture additional physical houses as population/Channel capacity grows.

### Downstream work unblocked

Later `EXP-HOUSES-01` composition may now assume:

1. physical houses are scarce/prestige World assets;
2. acquiring a physical house uses the Premium + eligibility rule;
3. non-Premium players may have a separate non-scarce residence class;
4. residences do not consume or multiply physical-house supply;
5. the first-generation residence does not require an auction/resale market;
6. exact residence balance, identity and runtime/schema details remain open.

## Decision

`NON-PREMIUM BASELINE HOUSING: NON-SCARCE RESIDENCE/APARTMENT CLASS`

`PHYSICAL HOUSE: SEPARATE SCARCE/PRESTIGE WORLD ASSET`

`RESIDENCE PUBLIC AUCTION / SPECULATIVE RESALE: NOT REQUIRED`

`RESIDENCE BASELINE: GENUINELY USABLE WITHOUT PREMIUM`

`EXACT RESIDENCE IDENTITY / LIMITS / RESTED / STORAGE / MONETIZATION: DEFERRED`

No runtime/client/server/protocol/DDL/migration/deployment/production implementation is authorized by this checkpoint.
