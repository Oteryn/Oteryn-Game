# Oteryn Game — Physical House Eligibility Owner Decision Checkpoint

- Date: 2026-09-08
- Issue: #220
- Status: `OWNER_SELECTED_DIRECTION / PENDING_CANONICAL_AMENDMENT`
- Mode: architecture analysis/documentation only
- Runtime implementation authority: `NONE`
- Client implementation authority: `NONE`
- DDL/migration authority: `NONE`
- Production authority: `NONE`

## Purpose

Persist the owner-selected acquisition eligibility direction for scarce physical houses while separating the right to **acquire** a physical house from the right to **retain** a legitimately acquired house.

The objective is to make physical-house acquisition costly enough in real account commitment to resist disposable-account concentration, while keeping Premium a meaningful server-funding mechanism without turning expiry into confiscation of an already acquired property right.

## Owner-selected acquisition rule

`OWNER_SELECTED`.

To acquire an ordinary physical house, the destination account/Character must satisfy both:

```text
active Premium
        AND
PhysicalHouseEligibility
```

This applies to all first-generation acquisition routes that can cause a destination Account to newly retain a physical `HouseId`, including:

- winning the World-scoped public house auction;
- Character Bazaar settlement using `INCLUDE_HOUSE_WITH_CHARACTER` when the buyer elects to retain the incoming house.

Premium alone is not sufficient. Gold alone is not sufficient.

## PhysicalHouseEligibility direction

`PhysicalHouseEligibility` represents proven real participation rather than a purchasable second house slot.

The later owning contract must derive eligibility from authoritative account/Character/World evidence sufficient to make disposable house-only accounts materially harder to create at scale.

The selected architecture requires eligibility to include real World participation/progression evidence in addition to active Premium at acquisition time.

Exact thresholds are deliberately deferred. Candidate evidence families may include:

- account age or trusted tenure;
- Character progression on the relevant World;
- meaningful server-authoritative activity/play history;
- account conduct/security standing;
- completion of a later housing-license or progression milestone.

No single candidate threshold in this document is final.

## Premium is acquisition eligibility, not retention rent

`OWNER_SELECTED`.

Premium expiry by itself MUST NOT:

- evict an existing physical-house owner;
- transfer `HouseId` ownership;
- destroy or reclaim house items;
- invalidate ACL state;
- create an immediate forced auction.

The existing recurring in-game rent/grace/eviction policy remains the property-retention/economy mechanism.

Therefore:

```text
acquire physical house
  => active Premium + PhysicalHouseEligibility required

Premium later expires
  => existing house remains owned
  => ordinary in-game rent obligations continue
```

A future explicit product decision may supersede this split, but Premium expiry is not currently a confiscation trigger.

## Anti-concentration relationship

This decision complements, rather than replaces, the accepted initial cap:

- one ordinary physical house per `AccountId` per `WorldId`;
- one guildhouse per `GuildId` per `WorldId` as a separate allowance.

The account cap stops alt Characters on one Account from accumulating houses. Eligibility raises the cost of producing many separate disposable Accounts solely to occupy scarce addresses.

The design does not claim multi-account abuse becomes impossible. Later anti-abuse telemetry and enforcement may detect coordinated disposable-account patterns, but shared IP/device identity alone must not be treated as automatic proof because legitimate households may share infrastructure.

## Character Bazaar interaction

When a Character+house Bazaar purchase would make the destination Account retain the incoming house, acquisition eligibility is revalidated at authoritative settlement time.

If the buyer already owns an ordinary physical house on the same World, the accepted Bazaar checkpoint requires an explicit `KEEP_EXISTING_HOUSE` or `KEEP_INCOMING_HOUSE` choice.

- `KEEP_EXISTING_HOUSE`: no new incoming-house retention occurs; the incoming house is relinquished safely.
- `KEEP_INCOMING_HOUSE`: the incoming house is an acquisition/retention event for the buyer and the destination must satisfy active Premium + `PhysicalHouseEligibility`; the previous same-World house is safely relinquished within the settlement.

The settlement never bypasses the one-house-per-Account-per-World cap.

## Guildhouses

Guildhouse acquisition eligibility remains deferred to the future guild-system architecture because canonical guildhouse owner identity is `GuildId` and Oteryn does not yet have the owning guild governance/lifecycle contract.

This checkpoint does not infer that ordinary-account Premium/eligibility rules map directly onto guildhouses.

## Non-scarce residences/apartments

`DEFERRED_PRODUCT_ANALYSIS`.

The owner explicitly wants non-scarce residences/apartments investigated as a possible housing path for players without Premium.

This checkpoint does not require Premium for that future residence class and does not overload physical-house eligibility onto it. The residence/apartment acquisition, rent, storage, Rested and decorating model will be analyzed separately after physical-house access semantics are sufficiently closed.

## Deliberately unresolved

- exact account-age threshold;
- exact Character level/progression threshold;
- exact activity/play-history threshold;
- whether a housing-license quest/milestone exists;
- conduct/security threshold details;
- how eligibility is displayed in UI;
- temporary suspension versus permanent eligibility revocation for abuse;
- guildhouse eligibility;
- non-scarce residence/apartment eligibility;
- schema/DDL;
- runtime implementation;
- production rollout.

## Decision

`PHYSICAL HOUSE ACQUISITION: ACTIVE PREMIUM AND PHYSICAL_HOUSE_ELIGIBILITY REQUIRED`

`PREMIUM ALONE: INSUFFICIENT`

`GOLD ALONE: INSUFFICIENT`

`PREMIUM EXPIRY: DOES NOT EVICT EXISTING HOUSE OWNER`

`REAL WORLD PARTICIPATION/PROGRESSION: REQUIRED IN ELIGIBILITY, EXACT THRESHOLDS DEFERRED`

`NON-SCARCE RESIDENCE/APARTMENT FOR NON-PREMIUM PLAYERS: EXPLICIT ANALYSIS CANDIDATE, NOT DECIDED`

No runtime/client/server/protocol/DDL/migration/deployment/production implementation is authorized by this checkpoint.
