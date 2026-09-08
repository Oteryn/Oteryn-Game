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

Persist the owner-selected first-generation split between scarce physical houses and non-scarce residences/apartments, including the selected ownership scope and mutual-exclusivity rule.

The product goal is to let a non-Premium player have a real personal housing space without turning infrastructure scale into additional copies of scarce world addresses, without removing the prestige/economy role of physical houses, and without allowing one account to stack both housing classes on the same World.

## Existing housing direction preserved

This checkpoint preserves the already selected housing model:

- finite physical `HouseId` addresses are World-scoped and shared across Channels;
- physical-house scarcity does not scale with Channel count;
- physical houses use the public World auction lifecycle;
- acquiring an ordinary physical house requires active Premium plus `PhysicalHouseEligibility` under the separate eligibility decision;
- expiration of Premium after legitimate acquisition does not itself evict an existing physical-house owner;
- ordinary physical-house ownership remains subject to in-game rent/grace/eviction rules;
- physical-house ACL is managed through a GUI/panel rather than Tibia-style text commands;
- ordinary physical-house public owner identity remains `CharacterId`, while the aggregate ordinary-house cap is enforced at `AccountId + WorldId` scope;
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

This residence class is not a physical `HouseId` copied into multiple Channels. It is a separate housing product/identity whose exact final name and persistence representation remain deferred.

## Residence ownership scope

`OWNER_SELECTED`.

The first-generation residence entitlement is scoped to the account on a World, not to every Character independently.

Semantic rule:

```text
maximum residences(AccountId A, WorldId W) = 1
```

Therefore multiple Characters owned by the same Account on the same World do not each create an additional residence entitlement.

The intended model is conceptually:

```text
AccountId A / WorldId W
  +-- Character C1
  +-- Character C2
  +-- Character C3
  +-- at most one Residence R
```

This deliberately reduces free-alt amplification, durable-object multiplication and the risk that residences become a cheap way to manufacture many storage surfaces.

The exact `ResidenceId` schema remains deferred. Selecting `AccountId + WorldId` as the entitlement/ownership scope does not require a final database shape now.

## One housing slot per Account and World

`OWNER_SELECTED`.

An Account on a World may hold either:

- one non-scarce Residence; or
- one ordinary physical house through one of its Characters;

but never both at the same time.

Conceptually:

```text
HousingSlot(AccountId A, WorldId W) =
    NONE
  | RESIDENCE
  | PHYSICAL_HOUSE
```

The following state is forbidden:

```text
AccountId A / WorldId W
  +-- Residence R
  +-- Physical House H
```

This is a semantic exclusivity invariant, not a commitment to a table or enum named `HousingSlot`.

Guildhouses remain outside this ordinary personal-housing slot because their owner identity is `GuildId`; their relationship to future guild/account rules remains deferred to guild-system architecture.

## Switching from Residence to physical house

A player who currently has a Residence may later become eligible for and acquire a scarce physical house.

The exact auction UI may be decided later, but authoritative settlement must never commit both housing classes.

Before physical-house acquisition becomes authoritative:

1. current account/World housing state is revalidated;
2. physical-house Premium + `PhysicalHouseEligibility` requirements are revalidated;
3. Residence durable contents/value are safely settled under later accepted item/storage rules;
4. stale Residence runtime writers are fenced;
5. the Residence entitlement is relinquished or otherwise made non-active;
6. only then may the physical-house acquisition commit;
7. failure/retry/reconciliation must not leave both housing classes active or lose authoritative durable value.

A later UX may allow a player to pre-authorize this switch while bidding, but the exact bidding interaction is deliberately deferred.

## Switching from physical house to Residence

A physical-house owner who wants a Residence instead must first resolve the physical house through the accepted physical-house lifecycle.

The transition must preserve the existing rules:

- outgoing durable house items/value are safely settled before ownership release;
- the physical `HouseId` becomes vacant and returns to the public World allocation/auction lifecycle;
- stale house runtime writers are fenced;
- only after the ordinary physical-house ownership slot is clear may the Residence become active.

The player may not keep the physical house as an investment while simultaneously activating a Residence.

## Character Bazaar interaction

This checkpoint further constrains the accepted Character Bazaar housing disposition rules.

If a buyer already has a Residence on the same World and buys a Character whose listing includes a physical house, settlement must resolve the housing-slot conflict explicitly before commit.

Conceptually:

```text
buyer currently has Residence R
incoming Character C includes Physical House H

buyer chooses before commit:
  KEEP_RESIDENCE
    -> incoming H is safely relinquished under the Bazaar/house rules
    -> Residence R remains active

  KEEP_INCOMING_HOUSE
    -> Residence R is safely relinquished
    -> Character C retains House H
```

The settlement must be idempotent/reconcilable and must never commit a state where the destination Account simultaneously has an active Residence and an ordinary physical house on the same World.

If the buyer already has an ordinary physical house, the separate accepted Bazaar rule for choosing which physical house is kept still applies.

A Character Bazaar transaction that contains no physical house does not by itself transfer or remove the buyer Account's Residence because the Residence entitlement is account/World-scoped rather than Character-owned.

## Scarcity boundary

A residence/apartment is intentionally non-scarce.

Creating a Residence for an eligible Account must not:

- create another copy of an existing physical world address;
- change the supply or auction price of physical houses;
- consume a physical `HouseId`;
- depend on Channel count;
- create a private market in scarce addresses;
- permit the Account to retain a physical house simultaneously.

The scarce/prestige layer remains the finite physical-house system.

## Free-access principle

Active Premium is not required merely to obtain or retain the baseline non-scarce Residence class.

The exact Free-player eligibility threshold is deliberately deferred, but it may require reasonable evidence of real gameplay progression/participation so that trivial throwaway accounts do not receive unlimited durable storage or operational surface for free.

The eligibility rule must be designed for normal players first and must not turn the Residence into a disguised Premium requirement.

## Relationship to Premium

Premium remains meaningful because it gates acquisition of scarce physical houses under the separate physical-house eligibility decision.

The intended product split is:

```text
Free / non-Premium
  -> may use the personal housing slot for a non-scarce Residence

Premium + PhysicalHouseEligibility
  -> may instead use the personal housing slot for a scarce physical house
```

A player does not stack both benefits on the same Account/World.

Premium may later add residence convenience, cosmetic catalog, layout/preset or presentation features, but the baseline Residence itself remains a genuine usable housing space rather than a crippled advertisement for Premium.

No Premium Residence upgrade is allowed by this checkpoint to create combat power or duplicate scarce physical `HouseId` supply.

## Residence exchange model

The first-generation Residence is not a speculative property asset.

It does not require:

- a public scarcity auction;
- player-to-player resale;
- private house nomination/transfer;
- a market price derived from limited address supply.

A later owner decision may introduce additional residence lifecycle/economy mechanics if evidence justifies them. They are not inferred here.

## Basic utility direction

The Residence should support the recognizable baseline idea of having a home:

- a persistent personal interior;
- decoration/personalization;
- a graphical housing/access panel consistent with the selected housing ACL UX;
- baseline social/access capability;
- compatibility with the later accepted Rested/bed design.

Exact feature limits remain deferred.

## Storage safety and anti-alt boundary

This checkpoint does not authorize Residences to become effectively unlimited free storage accounts.

Any Residence-specific storage, item-placement capacity or durable object budget must be bounded by the later item/storage/economy contract and preserve one authoritative semantic location for every durable item.

Account-scoped Residence entitlement does not itself make all Character inventories, depots or other Character-owned value account-shared.

Exact capacity numbers and whether Residence storage is distinct from normal account/depot storage remain deferred.

## ACL and account Characters

The Residence ownership/entitlement is account/World-scoped, but player-facing access remains governed through the selected graphical housing ACL model.

The later implementation may provide convenient default access for Characters owned by the Residence Account, but this checkpoint does not convert Residence access into an unrestricted cross-Character item-sharing authority.

Guest/Manager permissions, functional capabilities and stale-access revocation remain governed by the selected ACL principles.

## Rested / bed boundary

A non-Premium Residence may participate in the baseline Rested/housing recovery model, but this checkpoint does not freeze:

- the number of beds;
- the Rested recovery rate;
- whether a bed is required for all Residence Rested recovery;
- Premium modifiers;
- offline-training semantics;
- bed permissions beyond the already selected GUI/capability direction.

Those remain a later owner decision tied to the Rested system.

## Runtime/topology boundary

This decision selects the product class and ownership scope, not the final hosting implementation.

A Residence may later use an instance-style runtime primitive, but:

- it must have one authoritative runtime owner while active;
- durable item/state mutation remains in the World durable authority;
- origin Channel/routing semantics must remain explicit and fenced;
- Channel scaling must not duplicate durable Residence contents;
- exact `ResidenceId`, runtime identifier, hosting placement and persistence schema remain deferred.

## What is deliberately deferred

This checkpoint does not freeze:

- final product name (`Residence`, `Apartment`, or another term);
- `ResidenceId` schema;
- exact acquisition quest/level/account-age/playtime threshold;
- exact number/size/layout of Residence templates;
- exact auction/bidding UX for pre-authorizing Residence -> physical-house switching;
- rent or maintenance fee for Residence, if any;
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

`YES` for the product/scarcity split, Residence ownership scope and personal-housing exclusivity.

Without this decision, later housing work could accidentally make scarce physical houses the only route to basic housing utility, manufacture additional physical houses as population/Channel capacity grows, create one Residence per alt, or allow the same Account to stack both the free and scarce personal-housing layers.

### Downstream work unblocked

Later `EXP-HOUSES-01` composition may now assume:

1. physical houses are scarce/prestige World assets;
2. acquiring a physical house uses the Premium + eligibility rule;
3. non-Premium players may have a separate non-scarce Residence class;
4. the Residence entitlement is at most one per `AccountId + WorldId`;
5. the same `AccountId + WorldId` may hold a Residence or an ordinary physical house, never both;
6. transitions between those classes must safely settle durable value and cannot transiently commit double ownership;
7. the first-generation Residence does not require an auction/resale market;
8. exact Residence balance, identity representation and runtime/schema details remain open.

## Decision

`NON-PREMIUM BASELINE HOUSING: NON-SCARCE RESIDENCE/APARTMENT CLASS`

`RESIDENCE ENTITLEMENT: MAXIMUM ONE PER ACCOUNTID PER WORLDID`

`PERSONAL HOUSING SLOT: RESIDENCE XOR ORDINARY PHYSICAL HOUSE`

`PHYSICAL HOUSE: SEPARATE SCARCE/PRESTIGE WORLD ASSET`

`RESIDENCE PUBLIC AUCTION / SPECULATIVE RESALE: NOT REQUIRED`

`RESIDENCE BASELINE: GENUINELY USABLE WITHOUT PREMIUM`

`SWITCHING HOUSING CLASS: AUTHORITATIVE VALUE-SAFE SETTLEMENT REQUIRED`

`EXACT RESIDENCE IDENTITY / LIMITS / RESTED / STORAGE / MONETIZATION: DEFERRED`

No runtime/client/server/protocol/DDL/migration/deployment/production implementation is authorized by this checkpoint.
