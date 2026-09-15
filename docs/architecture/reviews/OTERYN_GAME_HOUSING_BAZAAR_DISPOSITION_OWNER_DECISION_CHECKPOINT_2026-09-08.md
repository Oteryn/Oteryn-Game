# Oteryn Game — Housing Bazaar Disposition Owner Decision Checkpoint

- Date: 2026-09-08
- Issue: #220
- Status: `OWNER_SELECTED_DIRECTION / PENDING_CANONICAL_AMENDMENT`
- Mode: architecture analysis/documentation only
- Runtime implementation authority: `NONE`
- Client implementation authority: `NONE`
- DDL/migration authority: `NONE`
- Production authority: `NONE`

## Purpose

Persist the owner-selected first-generation rule that scarce physical houses are acquired through the public World house-auction lifecycle and are not exposed as a general direct player-to-player property market, while explicitly defining what happens when the owning Character is listed on Character Bazaar.

The intent is to keep the initial system simple, preserve the value of physical addresses, avoid creating a dedicated house-flipping market before evidence justifies one, and still allow a Character seller to choose whether the owned house is relinquished or travels with the Character through an explicit Bazaar sale.

This checkpoint refines the earlier physical-house ownership checkpoint. It does not silently rewrite it.

## Existing decisions preserved

The following previously selected directions remain unchanged:

- ordinary physical house owner identity is `CharacterId`;
- guildhouse owner identity is `GuildId`;
- ordinary physical-house scarcity is World-scoped and independent of Channel count;
- one ordinary physical house per `AccountId` per `WorldId` is the initial aggregate cap;
- one guildhouse per `GuildId` per `WorldId` is a separate allowance;
- physical houses are not duplicated by Channel;
- ordinary vacant houses use the World-scoped public proxy-auction lifecycle;
- recurring rent, grace and value-safe eviction remain the selected maintenance direction;
- Premium/subscription expiry alone does not evict an existing owner;
- house ownership must never move silently as a side effect of a Character lifecycle operation;
- World transfer and deletion/finalization require explicit housing disposition.

## First-generation exchange model

`OWNER_SELECTED`.

For first-generation Oteryn, an ordinary physical `HouseId` is not a freely transferable standalone player-to-player investment asset.

The initial acquisition path is:

```text
vacant physical HouseId
        |
        v
World-scoped public proxy auction
        |
        v
eligible winning CharacterId
        |
        v
active ordinary-house ownership
```

The product does not initially provide a general command equivalent to:

```text
SellHouseDirectlyTo(CharacterId buyer, price)
```

or a free-form private property marketplace.

A future owner decision may add direct house transfer/sale if player demand and economy evidence justify it. That future capability must be explicit and must not be inferred from this checkpoint.

## Character Bazaar is the explicit exception

`OWNER_SELECTED`.

When a Character that currently owns an ordinary physical house is prepared for Character Bazaar, the seller must explicitly choose one housing disposition before the listing becomes eligible for settlement.

Conceptually:

```text
BazaarHousingDisposition =
    RELINQUISH_HOUSE
  | INCLUDE_HOUSE_WITH_CHARACTER
```

This is an explicit product choice, not an implicit side effect of account ownership transfer.

## Option A — RELINQUISH_HOUSE

The seller chooses not to include the physical house with the Character Bazaar listing.

Before the Bazaar sale may commit:

1. current authoritative Character/House ownership is revalidated;
2. the house enters an explicit disposition/settlement state;
3. outgoing durable items/value that are not explicitly part of a later approved transfer bundle are moved into safe authoritative reclaim/depot/custody;
4. stale house/runtime/Channel writers are fenced;
5. the `HouseId` is released from the Character;
6. the house becomes vacant;
7. the house returns to the normal World-scoped public auction/allocation lifecycle;
8. only after the required Character-side lifecycle gates are clear may the Character Bazaar ownership transfer settle.

Failure or timeout must not produce both a sold Character and an ambiguously owned house.

## Option B — INCLUDE_HOUSE_WITH_CHARACTER

The seller chooses to include the ordinary physical house with the Character Bazaar listing.

The semantic rule is:

```text
CharacterId C owns HouseId H
AccountId A owns CharacterId C

Character Bazaar sells CharacterId C to AccountId B
with explicit INCLUDE_HOUSE_WITH_CHARACTER

=> CharacterId C remains the owner identity of HouseId H
=> AccountId ownership of CharacterId C changes A -> B
=> HouseId H remains attached to CharacterId C
```

The house therefore travels with the Character because the canonical public house owner remains the same `CharacterId`.

This is not a standalone sale of `HouseId H` from seller to buyer. The commercial object exposed to the user is the Bazaar Character listing with an explicitly included house.

## Buyer eligibility and cap enforcement

`INCLUDE_HOUSE_WITH_CHARACTER` must not bypass housing scarcity controls.

Before authoritative Bazaar settlement commits, the buyer-side account must satisfy the then-current physical-house acquisition/ownership eligibility rules for the relevant World.

At minimum the later implementation must revalidate:

- destination `AccountId` is allowed to own the sold Character;
- the relevant `PhysicalHouseEligibility` or superseding housing-eligibility policy is satisfied;
- `HouseId` is still owned by the listed `CharacterId` at the expected revision;
- no rent/eviction/transfer state conflicts with settlement;
- no World transfer or other incompatible Character lifecycle is active;
- all operation/fencing identities still match the current authoritative state.

If the destination `AccountId` already consumes its ordinary-house allowance on the same `WorldId`, the buyer must explicitly choose exactly one post-settlement house disposition before the Character+house purchase can commit:

```text
BuyerHouseConflictDisposition =
    KEEP_EXISTING_HOUSE
  | KEEP_INCOMING_HOUSE
```

### KEEP_EXISTING_HOUSE

The buyer keeps the ordinary physical house already owned by another Character on the destination Account for that World. The incoming Character is purchased without retaining its included physical house. Before Bazaar settlement commits, the incoming `HouseId` must be safely relinquished, its non-transferred durable contents moved to authoritative reclaim/depot/custody, and the house returned to the public World auction lifecycle.

### KEEP_INCOMING_HOUSE

The buyer keeps the house attached to the incoming Character. Before Bazaar settlement commits, the destination Account's currently held ordinary physical house on that World must be safely relinquished, its non-transferred durable contents moved to authoritative reclaim/depot/custody, and that released `HouseId` returned to the public World auction lifecycle.

The Character Bazaar settlement must be idempotent/reconcilable across Character ownership change and the chosen house relinquishment. It must not expose a committed authoritative state in which the destination Account owns more ordinary physical houses than its accepted per-World cap.

A house on another `WorldId` does not conflict with this rule because the initial ordinary-house cap is per Account per World.

## House-content disposition when the house is included

`OWNER_SELECTED`.

The seller decides whether an `INCLUDE_HOUSE_WITH_CHARACTER` Bazaar listing transfers only the property right or also deliberately transfers house contents/furnishings.

Conceptually:

```text
BazaarHouseContentsDisposition =
    HOUSE_ONLY
  | INCLUDE_FURNISHINGS
```

### HOUSE_ONLY

This is the safe default. The Character and physical `HouseId` may be sold together, but ordinary movable durable items/value belonging to the seller are not transferred merely because they were left inside the house. They are moved to authoritative reclaim/depot/custody before final settlement.

### INCLUDE_FURNISHINGS

The seller may explicitly choose to include furnishings/house contents in the Character+house Bazaar offer. This is never inferred from physical presence alone.

Before activation, the later owning contract must define an authoritative, reviewable transfer manifest or equivalent bounded representation so that both sides know exactly which durable items/value are part of the sale. The listed transfer set must be locked/revisioned for settlement, conserve item identity/location under `DUR-03`, and fail closed on stale or ambiguous state.

The implementation may initially choose the smallest safe product surface, for example an all-or-nothing explicit furnished-house bundle, and later add item-level selection if product evidence justifies the complexity. This checkpoint does not require a specific UI or manifest schema.

The invariant is:

**seller intent is explicit; forgotten items never transfer accidentally.**

## Interaction with Character Bazaar authority

This checkpoint preserves the accepted split in `CHARACTER_AUTHORITY_PLATFORM_BOUNDARY.md`:

- Platform may own Bazaar listing, bidding, wallet/commission and commercial workflow state;
- Game Character Authority owns current Character ownership validation and the authoritative `AccountId` rebinding;
- Game housing authority owns authoritative `HouseId` ownership/disposition state;
- no distributed ACID assumption is introduced;
- timeout is not success or failure proof;
- settlement/retry must use authoritative operation state and idempotent reconciliation.

A Bazaar listing may advertise that a house and, when explicitly selected, furnishings are included, but the listing/read model is never authoritative proof that the transfer conditions still hold at settlement time.

## Why this does not contradict the earlier ownership checkpoint

PR #436 selected that house ownership does not **silently** follow Character Bazaar/account ownership transfer and that Character lifecycle operations require an explicit housing disposition/settlement boundary.

This checkpoint supplies that missing explicit disposition for Character Bazaar.

The refined rule is:

```text
silent follow-through                    = FORBIDDEN
explicit RELINQUISH_HOUSE                = ALLOWED
explicit INCLUDE_HOUSE_WITH_CHARACTER    = ALLOWED
```

Therefore this is a refinement of #436, not a reversal of `CharacterId` house ownership.

## Guildhouses

Guildhouses do not follow this Character Bazaar rule because their canonical owner is `GuildId`, not `CharacterId`.

Selling a guild leader Character does not transfer the guildhouse merely because that Character changes Account ownership.

Guild leadership succession, guildhouse administration and guild lifecycle remain deferred until the Oteryn guild-system architecture exists. Housing architecture must integrate with that future authority rather than invent guild ownership semantics in advance.

## World transfer

A physical house is a World address and does not move across Worlds with a Character.

Therefore `INCLUDE_HOUSE_WITH_CHARACTER` is a Character Bazaar disposition only; it does not authorize world-transfer relocation of a `HouseId`.

A Character World transfer must resolve/relinquish any incompatible physical-house ownership under the later accepted World-transfer housing policy before the transfer can commit.

## Character deletion/finalization

Terminal Character deletion/finalization cannot leave a durable physical house with a nonexistent owner identity.

Deletion/finalization therefore still requires explicit house settlement/relinquishment before terminal lifecycle completion.

Exact grace/recovery behavior remains deferred to the owning Character/housing lifecycle contract.

## Anti-speculation consequence

The first-generation model intentionally avoids a general direct house-sale market.

A player cannot simply acquire a physical house and invoke a dedicated private resale operation at an arbitrary price.

The remaining explicit value-transfer route is selling the entire owning Character through Character Bazaar with the house included. That route is materially different because:

- the canonical Character itself changes Account ownership;
- Bazaar eligibility/fees/commercial workflow apply;
- buyer housing eligibility and account cap must still pass;
- the seller gives up the owning Character rather than only flipping the `HouseId`;
- later anti-abuse/eligibility policy may impose additional protections without adding a direct property market.

This does not claim that speculation becomes impossible. It deliberately keeps the initial product surface smaller and leaves further anti-concentration controls to evidence-driven housing eligibility/economy policy.

## Deliberately deferred

This checkpoint does not freeze:

- direct standalone house sales/transfers;
- house-transfer nomination;
- minimum house holding period;
- Bazaar cooldown after selling a Character with a house;
- exact `PhysicalHouseEligibility` criteria;
- Character+house Bazaar fees or surcharges;
- exact furnished-house manifest schema/UI or item-selection granularity;
- exact item reclaim representation;
- exact notification UX;
- guildhouse leadership/succession/administration policy;
- schema/DDL;
- runtime implementation;
- production rollout.

## Decision timing

### Must decide now?

`YES` for the first-generation exchange surface, Character Bazaar disposition, cap-conflict behavior and seller-controlled house-content disposition.

Without these decisions, later Bazaar and housing contracts could accidentally create an unintended private property market, an unconditional rule that blocks Character sale whenever a house exists, an account-cap violation, or accidental transfer/loss of house contents.

### What downstream work is unblocked?

Later housing/Character Bazaar analysis can now assume:

1. vacant physical houses enter through public auction;
2. no generic direct house-sale feature is required for first generation;
3. a Character seller explicitly chooses relinquish-house or include-house;
4. buyer housing eligibility/cap is revalidated at settlement;
5. same-World cap conflict requires an explicit keep-existing/keep-incoming choice;
6. seller explicitly chooses house-only or furnished-house disposition;
7. guildhouses do not follow the Character sale;
8. World transfer still cannot move the physical address.

### Evidence that may justify supersession

- sustained player demand for direct house transfers;
- evidence that Character Bazaar is an inadequate or excessively costly route for legitimate housing transfers;
- house-market liquidity problems;
- evidence of Bazaar-based house speculation or concentration;
- economy telemetry showing a dedicated private property market would be healthier;
- user demand for more granular furnished-house item selection;
- a later explicit owner product decision.

## Decision

`FIRST-GENERATION PHYSICAL-HOUSE ACQUISITION: PUBLIC WORLD AUCTION`

`GENERAL DIRECT HOUSE SALE: NOT ENABLED`

`CHARACTER BAZAAR SELLER DISPOSITION: EXPLICIT RELINQUISH OR INCLUDE HOUSE`

`INCLUDE-HOUSE SEMANTICS: HOUSE REMAINS WITH THE SAME CHARACTERID THROUGH ACCOUNT OWNERSHIP TRANSFER`

`BUYER SAME-WORLD CAP CONFLICT: EXPLICIT KEEP-EXISTING OR KEEP-INCOMING`

`HOUSE CONTENTS ON BAZAAR: SELLER EXPLICITLY CHOOSES HOUSE-ONLY OR FURNISHED-HOUSE`

`FORGOTTEN ITEMS: NEVER TRANSFER ACCIDENTALLY`

`GUILDHOUSE BAZAAR SEMANTICS: DEFERRED TO FUTURE GUILD SYSTEM`

No runtime/client/server/protocol/DDL/migration/deployment/production implementation is authorized by this checkpoint.
