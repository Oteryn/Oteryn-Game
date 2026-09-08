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

The one-ordinary-house-per-Account-per-World cap is a post-settlement invariant. A Bazaar listing or bid may exist while the buyer already owns another ordinary physical house on that same World, but authoritative settlement MUST NOT commit until the buyer has explicitly selected which ordinary house remains after the transaction.

## Buyer cap-conflict resolution

`OWNER_SELECTED`.

If the buyer does not already consume an ordinary-house allowance on the incoming Character's `WorldId`, the included house may remain with the bought Character subject to all other eligibility checks.

If the buyer already owns an ordinary physical house on the same World, the settlement surface must require exactly one explicit choice:

```text
BazaarBuyerHouseResolution =
    KEEP_EXISTING_HOUSE
  | KEEP_INCOMING_HOUSE
```

### KEEP_EXISTING_HOUSE

The buyer keeps the ordinary house already owned by another Character on the destination Account.

The incoming Character may still be bought, but its included physical house is relinquished as part of the same authoritative Bazaar settlement. Before ownership release:

- the incoming `HouseId` is revalidated at the expected revision;
- seller/incoming-house durable items that are not part of a separately accepted transfer bundle are moved to safe authoritative reclaim/depot/custody;
- stale house/runtime writers are fenced;
- the incoming physical house becomes vacant and returns to the public World auction lifecycle.

The Character purchase must not commit while leaving the Account above the ordinary-house cap.

### KEEP_INCOMING_HOUSE

The buyer chooses to keep the house attached to the incoming Character.

The buyer's existing ordinary physical house on that same World must therefore be relinquished as part of the same settlement. Before that existing house is released:

- current buyer-side house ownership is revalidated;
- buyer-owned durable items/value are moved to safe authoritative reclaim/depot/custody;
- stale house/runtime writers are fenced;
- the existing physical house becomes vacant and returns to the public World auction lifecycle.

Only after that disposition is prepared safely may the Bazaar ownership transfer commit with the incoming Character retaining its `HouseId`.

### No same-World conflict

If the buyer owns an ordinary physical house only on a different `WorldId`, there is no cap conflict because the initial allowance is per Account per World.

Guildhouse custody is separate because guildhouses consume the `GuildId` allowance, not the Account's ordinary-house allowance.

### No implicit choice

If a same-World cap conflict exists and the buyer has not explicitly selected a resolution, settlement must remain non-committed. The system must not silently pick one house, silently discard one house, or temporarily create a committed two-house Account state.

## Atomic Bazaar / housing settlement requirement

`OWNER_SELECTED`.

A Character+house purchase with a cap conflict is one semantic settlement even if Platform and Game use separate durable systems.

The implementation contract must provide a prepared/idempotent/reconcilable saga or equivalent authority boundary such that the externally visible committed result is either:

```text
SUCCESS:
- Character ownership changed to the buyer Account;
- exactly one allowed ordinary house remains on that World;
- the selected relinquished house is safely released;
- all required durable item custody is committed;
- stale writers are fenced;
```

or:

```text
NO COMMIT / RECOVERY:
- no ambiguous double ownership;
- no silently lost house;
- no silently lost or duplicated durable items;
- no Account left committed above the house cap;
- authoritative operation state remains reconcilable after timeout/retry.
```

A failed Character Bazaar settlement must not permanently consume the buyer's existing house merely because a relinquishment was prepared. A failed relinquishment must not produce a successful Character+house sale that violates the cap.

Exact command names, transaction IDs, transport, orchestration state and DDL remain deferred to the later owning contracts.

## Interaction with Character Bazaar authority

This checkpoint preserves the accepted split in `CHARACTER_AUTHORITY_PLATFORM_BOUNDARY.md`:

- Platform may own Bazaar listing, bidding, wallet/commission and commercial workflow state;
- Game Character Authority owns current Character ownership validation and the authoritative `AccountId` rebinding;
- Game housing authority owns authoritative `HouseId` ownership/disposition state;
- no distributed ACID assumption is introduced;
- timeout is not success or failure proof;
- settlement/retry must use authoritative operation state and idempotent reconciliation.

A Bazaar listing may advertise that a house is included, but the listing/read model is never authoritative proof that the house transfer condition still holds at settlement time.

## Why this does not contradict the earlier ownership checkpoint

PR #436 selected that house ownership does not **silently** follow Character Bazaar/account ownership transfer and that Character lifecycle operations require an explicit housing disposition/settlement boundary.

This checkpoint supplies that missing explicit disposition for Character Bazaar.

The refined rule is:

```text
silent follow-through                    = FORBIDDEN
explicit RELINQUISH_HOUSE                = ALLOWED
explicit INCLUDE_HOUSE_WITH_CHARACTER    = ALLOWED
explicit buyer cap-conflict resolution   = REQUIRED WHEN APPLICABLE
```

Therefore this is a refinement of #436, not a reversal of `CharacterId` house ownership.

## Guildhouses

Guildhouses do not follow this Character Bazaar rule because their canonical owner is `GuildId`, not `CharacterId`.

Selling a guild leader Character does not transfer the guildhouse merely because that Character changes Account ownership.

Guild leadership succession, guildhouse administration and guild lifecycle remain separate later decisions.

## World transfer

A physical house is a World address and does not move across Worlds with a Character.

Therefore `INCLUDE_HOUSE_WITH_CHARACTER` is a Character Bazaar disposition only; it does not authorize world-transfer relocation of a `HouseId`.

A Character World transfer must resolve/relinquish any incompatible physical-house ownership under the later accepted World-transfer housing policy before the transfer can commit.

## Character deletion/finalization

Terminal Character deletion/finalization cannot leave a durable physical house with a nonexistent owner identity.

Deletion/finalization therefore still requires explicit house settlement/relinquishment before terminal lifecycle completion.

Exact grace/recovery behavior remains deferred to the owning Character/housing lifecycle contract.

## Item-content policy remains intentionally narrow

The owner selected that a Character may be sold with its house. This checkpoint deliberately does not infer that every movable item currently present inside the house is automatically sold to the Character buyer.

The safe first-generation architectural default remains:

- forgotten or ordinary seller-owned durable value must not transfer accidentally;
- transfer/relinquishment must preserve item conservation and one authoritative semantic location;
- any future capability to deliberately include specified house contents/decorations in a Character+house Bazaar bundle requires an explicit product rule and transactional representation.

This keeps the Character+house ownership decision independent from a later optional furnished-house marketplace.

## Anti-speculation consequence

The first-generation model intentionally avoids a general direct house-sale market.

A player cannot simply acquire a physical house and invoke a dedicated private resale operation at an arbitrary price.

The remaining explicit value-transfer route is selling the entire owning Character through Character Bazaar with the house included. That route is materially different because:

- the canonical Character itself changes Account ownership;
- Bazaar eligibility/fees/commercial workflow apply;
- buyer housing eligibility and account cap must still pass;
- any same-World cap conflict requires one explicit house relinquishment;
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
- whether active Premium is one acquisition-eligibility route;
- Character+house Bazaar fees or surcharges;
- exact Bazaar UI timing for buyer house-resolution selection;
- furnished-house / selected-item transfer bundles;
- exact item reclaim representation;
- exact notification UX;
- schema/DDL;
- runtime implementation;
- production rollout.

## Decision timing

### Must decide now?

`YES` for the first-generation exchange surface, Character Bazaar disposition and same-World ownership-cap conflict semantics.

Without this decision, later Bazaar and housing contracts could accidentally create either an unintended private property market, an unconditional rule that blocks Character sale whenever a house exists, or a committed Account state that violates the one-house-per-World cap.

### What downstream work is unblocked?

Later housing/Character Bazaar analysis can now assume:

1. vacant physical houses enter through public auction;
2. no generic direct house-sale feature is required for first generation;
3. a Character seller explicitly chooses relinquish-house or include-house;
4. buyer housing eligibility/cap is revalidated at settlement;
5. same-World buyer cap conflicts are resolved by explicitly keeping either the existing or incoming house;
6. the other house is safely relinquished to the public auction lifecycle in the same semantic settlement;
7. guildhouses do not follow the Character sale;
8. World transfer still cannot move the physical address.

### Evidence that may justify supersession

- sustained player demand for direct house transfers;
- evidence that Character Bazaar is an inadequate or excessively costly route for legitimate housing transfers;
- house-market liquidity problems;
- evidence of Bazaar-based house speculation or concentration;
- economy telemetry showing a dedicated private property market would be healthier;
- a later explicit owner product decision.

## Decision

`FIRST-GENERATION PHYSICAL-HOUSE ACQUISITION: PUBLIC WORLD AUCTION`

`GENERAL DIRECT HOUSE SALE: NOT ENABLED`

`CHARACTER BAZAAR SELLER DISPOSITION: EXPLICIT RELINQUISH OR INCLUDE HOUSE`

`INCLUDE-HOUSE SEMANTICS: HOUSE REMAINS WITH THE SAME CHARACTERID THROUGH ACCOUNT OWNERSHIP TRANSFER`

`BUYER SAME-WORLD CAP CONFLICT: EXPLICIT KEEP_EXISTING_HOUSE OR KEEP_INCOMING_HOUSE`

`POST-SETTLEMENT ORDINARY-HOUSE CAP: MUST HOLD`

`CONFLICTING HOUSE RELINQUISHMENT + CHARACTER SALE: ONE IDEMPOTENT RECONCILABLE SEMANTIC SETTLEMENT`

No runtime/client/server/protocol/DDL/migration/deployment/production implementation is authorized by this checkpoint.
