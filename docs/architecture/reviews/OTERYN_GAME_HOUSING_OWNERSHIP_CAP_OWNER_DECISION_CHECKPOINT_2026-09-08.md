# Oteryn Game — Physical-House Ownership Cap Owner Decision Checkpoint

- Date: 2026-09-08
- Issue: #220
- Source owner decision comment: Issue #220 comment `5585714024`
- Status: `OWNER_SELECTED_DIRECTION / PENDING_CANONICAL_AMENDMENT`
- Mode: architecture analysis/documentation only
- Runtime implementation authority: `NONE`
- Client implementation authority: `NONE`
- DDL/migration authority: `NONE`
- Production authority: `NONE`

## Purpose

Persist the owner-selected initial anti-concentration cap for scarce physical houses while preserving the already selected semantic owner identities and the hybrid housing model.

This checkpoint deliberately freezes only the initial ownership-cap direction. It does not freeze eligibility thresholds, cooldown durations, direct-transfer rules, auction/rent numbers, residence/apartment limits, database representation or production implementation.

## Existing decisions preserved

This decision consumes and does not replace the housing direction already selected in the ongoing architecture continuation:

1. physical houses are finite World-scoped properties and are not duplicated per Channel;
2. ordinary physical-house canonical owner identity is one `CharacterId` in the owning World;
3. guildhouse canonical owner identity is one `GuildId`;
4. `AccountId` is not the public/canonical property owner identity, but may be used privately for eligibility, caps and anti-speculation controls;
5. vacant physical houses use the selected World-scoped proxy-auction direction;
6. physical houses use recurring rent, grace and value-safe eviction semantics;
7. scalable residences/apartments remain a separate non-scarce housing class.

## Owner-selected cap

`OWNER_SELECTED`.

For the initial Oteryn physical-house economy:

```text
per AccountId + WorldId:
    <= 1 ordinary physical house

per GuildId + WorldId:
    <= 1 guildhouse
```

A guildhouse does not consume the personal physical-house allowance of the account controlling or leading the guild.

A different `WorldId` has an independent personal-house allowance because physical-house scarcity, ownership, rent and economy are World-scoped.

## Public owner identity versus aggregate cap identity

The cap does not change canonical property ownership identity.

For an ordinary house:

```text
HouseId -> CharacterId
```

The owning Character remains the public/gameplay-facing property owner identity.

The owning Character's current authoritative `AccountId` relation is used privately to enforce the aggregate one-house-per-account-per-World rule.

Therefore:

```text
public/canonical property owner = CharacterId
anti-concentration aggregate key = AccountId + WorldId
```

These concepts MUST NOT be collapsed into one field merely for implementation convenience.

## Alternate Characters

Alternate Characters belonging to one Account MUST NOT permit the account to acquire multiple ordinary physical houses in the same World.

Before an ordinary physical-house acquisition becomes final, the authoritative house/economy boundary must revalidate that the Account currently owning the proposed `CharacterId` does not already consume its ordinary-house allowance in that World.

A stale account/character projection cannot prove eligibility.

Character ownership transfer, Bazaar transfer or other account rebinding must not create a state where one Account silently owns multiple physical houses. The already selected explicit housing-disposition boundary therefore remains mandatory before such lifecycle operations become final where the cap would be violated.

## Guildhouse separation

Guildhouses use a separate scarcity/ownership lane:

```text
GuildId + WorldId -> <= 1 guildhouse
```

This means a player may simultaneously:

- own one ordinary physical house through one of their Characters in a World; and
- participate in or administer a Guild that owns one guildhouse in that World,

subject to later guild authority/administration rules.

The personal account cap and guildhouse cap MUST NOT be combined into one quota.

## Why one personal physical house initially

`DERIVED` from the selected hybrid housing direction.

Physical houses are intentionally scarce prestige/social/economy objects. Baseline housing utility is intended to remain available through scalable residences/apartments and other non-scarce systems.

A stricter initial physical-house cap therefore protects:

- address scarcity;
- broad player access to prestigious physical real estate;
- auction competitiveness;
- resistance to alt-account/account-character concentration;
- future balance flexibility while actual World population and physical-house supply are still unproven.

The cap is a product/economy control, not a technical requirement of `HouseId` or Channel topology.

## Reversibility

`OWNER_SELECTED`.

The one-house cap is intentionally easy to relax later.

Future evidence may justify changing:

```text
1 -> 2
1 -> 3
or another bounded value
```

without changing:

- `HouseId` identity;
- public `CharacterId` ownership semantics;
- `GuildId` guildhouse ownership semantics;
- World scope;
- item custody;
- house runtime ownership;
- auction settlement architecture.

Increasing the cap later is therefore expected to be materially cheaper than beginning with a high-concentration model and later forcing existing owners to divest.

## Cap enforcement timing

The later `EXP-HOUSES-01` contract must define authoritative eligibility checks at all value-bearing ownership transitions, including at least:

- proxy-auction bid acceptance where the bid can become winning;
- auction close/settlement;
- any later accepted direct property transfer mechanism;
- Character ownership/account transfer;
- World transfer;
- guildhouse acquisition and guild lifecycle changes where relevant.

A temporary eligibility state at bid time MUST NOT override authoritative eligibility at final settlement.

## Failure and race safety

Two simultaneous acquisitions by different Characters of the same Account must not both commit and exceed the cap.

The later implementation must use an authoritative transactional/revision-fenced boundary so concurrent auction settlements cannot create:

```text
Account A, World W
  owns House H1
  owns House H2
```

when the active cap is one.

Timeout is not proof of success or failure. Retry/reconciliation must reread authoritative ownership/cap state.

## Relationship to residences/apartments

This cap applies to scarce ordinary physical houses only.

It MUST NOT silently impose the same limit on the separate scalable residence/apartment product class.

Residence identity, count, acquisition and monetisation/entitlement rules remain separately deferred.

## Anti-speculation consequences

The selected cap materially reduces one form of concentration but does not by itself solve all speculation or collusion.

Still deferred:

- cooldown after voluntary sale;
- cooldown after eviction for non-payment;
- ability to transfer a physical house directly to another player versus mandatory return to public allocation;
- eligibility age/level/account-history requirements;
- sanctions for bid manipulation/collusion;
- related-account/household policy;
- minimum holding periods;
- rent or transaction taxes designed as economy sinks.

These must be decided independently rather than inferred from the numerical cap.

## Rejected direction

### Multiple ordinary houses through alternate Characters of one Account

`REJECTED` for the initial model.

Public ownership may remain Character-scoped, but allowing each alternate Character to consume an independent physical-house allowance would make the account-level anti-concentration control ineffective.

### Guildhouse consumes personal allowance

`REJECTED`.

A guildhouse is a Guild-owned social asset and has a distinct `GuildId` ownership lane. Counting it against the leader's personal house would create unstable behavior when guild leadership or administration changes.

## Decision timing

### Must decide now?

`YES` for the initial aggregate cap boundary.

Auction, ownership and Character-account transfer semantics need to know whether eligibility is Character-local or Account-aggregated before durable implementation is designed.

### What downstream work is unblocked?

The later house contract can now define:

1. authoritative acquisition eligibility;
2. concurrent settlement/cap enforcement;
3. Character/account transfer disposition;
4. guildhouse acquisition independently from personal housing;
5. anti-speculation rules without reopening the basic quota identity.

### What remains deliberately unresolved?

- direct player-to-player house transfer versus public re-auction;
- sale/eviction reacquisition cooldowns;
- exact auction/rent values and timing;
- account age/Character progression eligibility;
- guild administration rules;
- residence/apartment limits;
- schema/DDL/migration;
- production rollout.

## Next bounded owner decision

The next material decision should determine whether an owner may directly transfer/sell a scarce physical house to a chosen player, or whether every voluntary release/change of owner must return the property to a public World-scoped auction/allocation path.

No runtime/client/server/protocol/DDL/migration/deployment/production implementation is authorized by this checkpoint.
