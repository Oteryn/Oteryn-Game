# Oteryn Game — Physical-House Proxy Auction Owner Decision Checkpoint

- Date: 2026-09-08
- Issue: #220
- Source owner decision comment: Issue #220 comment `5585369075`
- Status: `OWNER_SELECTED_DIRECTION / PENDING_CANONICAL_AMENDMENT`
- Mode: architecture analysis/documentation only
- Runtime implementation authority: `NONE`
- Client implementation authority: `NONE`
- DDL/migration authority: `NONE`
- Production authority: `NONE`

## Purpose

Persist the owner-selected allocation mechanism for scarce physical houses without freezing balance values, storage layout, transport, UI details or production implementation.

This checkpoint is intentionally narrow. It selects the semantic auction direction only.

## Context preserved

Current accepted/pending housing analysis establishes that a physical `HouseId` is a World-scoped scarce property and must not be duplicated merely because a World has multiple Channels.

The ongoing owner decision sequence also selected a semantic ownership direction in which ordinary physical houses are associated with a Character identity and guildhouses with a Guild identity, while account identity may be used privately for later eligibility/cap/anti-speculation policy. That ownership checkpoint remains independently qualified through repository controls and is not rewritten here.

This document therefore defines how an eligible physical house is allocated when the product uses an auction. It does not itself define every eligibility rule or final ownership schema.

## Owner-selected auction model

`OWNER_SELECTED`.

Scarce physical houses use a **World-global proxy auction** direction.

For one auctioned `HouseId`, each bidder may submit a private maximum willingness-to-pay value.

Conceptually:

```text
Bidder A private max: 1,000,000
Bidder B private max:   850,000
Bidder C private max:   700,000

System does not charge A 1,000,000 merely because A entered that maximum.
System advances A only as competition requires.
Final price is the minimum valid price needed to beat the next-best competing maximum under the later-defined increment rule.
```

The private maximum is a ceiling for automated bidding authority, not the automatic purchase price.

## Required semantic properties

### 1. Competition-driven advancement

The auction service advances an eligible bidder only when another valid competing bid requires it.

It MUST NOT manufacture price increases merely to move the current leader toward their private maximum.

### 2. Private maximum

A bidder's maximum bid is private auction authority state.

The product may expose the current effective price and bounded status information, but the exact competing maximum MUST NOT be exposed as public authoritative state merely because the system stores it.

Exact UX and disclosure policy remain deferred.

### 3. Winner price

The winner pays the minimum valid amount required to defeat the next-best valid competing maximum according to the later-defined increment/tie rule.

The submitted maximum MUST NOT automatically become the settlement amount.

### 4. Escrow / reservation

A bid capable of becoming the winning bid must have enforceable settlement capacity.

The architecture requires a reservation/escrow-style boundary so a bidder cannot win a house using value that has already been spent elsewhere.

The exact implementation may be a hold, reservation, escrow balance or another accepted economy primitive. This checkpoint does not select the schema or storage representation.

Reservation/release/settlement operations must be idempotent and auditable.

### 5. Anti-sniping

The auction must prevent an unanswerable last-moment bid from being the dominant strategy.

The selected direction is an **extension-based anti-sniping rule**: a valid competitive bid near the current deadline extends the auction sufficiently to allow another participant a reasonable response opportunity.

Exact trigger window, extension duration, maximum extension behavior and UX remain balance/product decisions.

### 6. World scope

There is one auction lifecycle for one physical `HouseId` within its World.

Adding/removing/relocating Channels MUST NOT:

- create another auction for the same physical property;
- duplicate bids;
- reset the auction deadline;
- duplicate settlement;
- change house supply.

### 7. Settlement safety

Auction close and ownership settlement must be explicit, retry-safe and revision/fencing aware.

Timeout or retry MUST NOT create:

- two winners;
- two owners;
- double debit;
- duplicated refunds;
- a house assigned without a proven winning settlement;
- a stale auction worker overwriting a newer committed result.

Exact transaction boundaries remain with the later housing/economy/durability contract.

## Tie handling

A deterministic tie rule is required before implementation, but its exact product rule is deliberately not selected here.

Examples that may later be evaluated include earliest valid maximum at the same amount or another deterministic, abuse-resistant ordering.

Random or process-local ordering is not acceptable as authoritative settlement behavior.

## Bid replacement and cancellation

The semantic API must eventually define whether and when a bidder may raise, lower or cancel a maximum bid.

This checkpoint does not freeze those user-facing rules.

Any supported change must preserve reservation consistency and must not allow value to be reused while still backing a potentially winning bid.

## Relationship to house ownership

Auction winning and durable house ownership are distinct states.

Conceptually:

```text
auction winner determined
  -> settlement/reservation validated
  -> owner eligibility revalidated
  -> authoritative HouseId ownership mutation
  -> durable receipt/result
  -> losing reservations released/reconciled
```

The exact command names, operation IDs and database representation remain deferred.

A stale auction result cannot override a newer house ownership revision.

## Relationship to character/account/guild lifecycle

Winning eligibility must be revalidated at settlement.

A bidder becoming ineligible because of a Character lifecycle, World membership, Guild lifecycle or later account-level anti-abuse rule must not be silently converted into valid ownership merely because the bidder had previously been leading.

Exact fallback behavior — next bidder, failed auction, relist or another explicit state — is deliberately deferred.

## Anti-abuse direction

The proxy model should reduce incentives for last-second click automation, but it is not itself a complete anti-abuse policy.

Later policy may use private account-level information for:

- ownership caps;
- bid eligibility;
- correlated bidder/alt abuse detection;
- cooldowns;
- sanctions or fraud controls.

Those controls MUST NOT require exposing private `AccountId` relationships to ordinary players.

## Economy consequences

The auction may act as a World-economy value sink depending on later settlement/fee rules.

This checkpoint does not freeze:

- starting price;
- reserve price;
- bid increment;
- auction duration;
- anti-sniping trigger/extension duration;
- auction currency;
- listing/settlement fees;
- taxes;
- recurring rent;
- ownership caps;
- eligibility levels/status;
- Premium or entitlement requirements;
- bid cancellation penalties;
- winner default policy.

These are product/economy decisions unless a later architecture invariant requires otherwise.

## Rejected default directions

### Manual click-by-click last-second auction

`REJECTED_AS_DEFAULT`.

It disproportionately rewards constant online presence, low latency and automation around the deadline.

### Submitted maximum equals purchase price

`REJECTED`.

The maximum is automated bidding authority, not an instruction to charge the entire amount absent competition.

### Per-Channel auction copies

`REJECTED`.

They would make runtime capacity alter World housing scarcity and risk duplicate ownership/settlement.

### Unbacked winning bids

`REJECTED`.

A bid may not become final ownership without enforceable settlement capacity and retry-safe reconciliation.

## Decision timing

### Must decide now?

`YES` for the high-level allocation mechanism.

Without this decision, later economy/persistence/UX work could accidentally optimize for manual deadline clicking or select incompatible bid/escrow semantics.

### What remains deliberately reversible?

All numeric and presentation details remain changeable. The system may later replace proxy auctions with another explicit allocation mechanism through an owner-approved supersession, provided durable bid/reservation/ownership state is migrated safely.

## Deliberately unresolved next decisions

- recurring rent versus another ongoing ownership-retention model;
- grace period and eviction lifecycle;
- treatment of house items on eviction/transfer;
- ownership caps and anti-speculation;
- exact auction duration/increment/anti-sniping window;
- bid cancellation/lowering rules;
- tie rule;
- winner ineligibility/default fallback;
- guildhouse allocation differences;
- DDL/schema/migrations;
- production rollout.

## Next owner decision

Continue one material decision at a time. The next bounded question should decide the **ongoing retention/eviction model for scarce physical houses**: whether ownership requires recurring World-economy rent with a forgiving grace/recovery lifecycle, or another explicit retention mechanism.

No runtime/client/server/protocol/DDL/migration/deployment/production implementation is authorized by this checkpoint.
