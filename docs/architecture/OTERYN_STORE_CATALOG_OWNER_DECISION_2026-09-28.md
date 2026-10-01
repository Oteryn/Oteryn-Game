# Store Catalog Owner Decision — 2026-09-28

- Status: Owner-accepted decision record
- Date: 2026-09-28
- Decision owner: Oteryn project owner
- Issue: #162, Jira `KAN-16`
- Applies to: authorship of the in-game Tibia Store catalog
- Evidence: `tools/content-schema/store-authoring/` (schema, converter, census over
  pinned Canary `47dfd51f` and Crystal `ff7ede5` GameStore catalog sources)
- Does not authorize: runtime, protocol, payment, entitlement or production
  implementation; does not resolve refunds, fraud or entitlement lifecycle

## 1. The catalog is Game content

The Store **catalog** — categories, offers, prices in Tibia Coins and product
references — is Game content: the server and client must know which products are
offered and for how much. It is authored like any other Game content family
(`tools/content-schema/store-authoring/`, mirroring the spell/item packages), following
the GameStore catalog model that both reference engines already share.

Delivery of a purchased offer to the character is in-game server behavior in both
reference engines, but its ownership across the Game/Platform boundary is **not**
decided here; it stays open under §32 together with entitlement lifecycle.

## 2. The coin balance and purchase ledger stay with Platform

The player's Tibia Coin balance, the payment flow, and the purchase ledger (what was
bought, when, for how much) remain Platform's, under Platform's existing web
identity/commercial ownership (`OTERYN_WORLD_PROJECT_SOURCE_PROFILE_V2_DECISION.md`) and `ARCHITECTURE_ANALYSIS_GAP_REGISTER.md`
§32 (`PROD-ENTITLEMENTS-01`). The Game/Platform integration for balance reads and debits
is not decided here.

## 3. Scope of this decision

This **partially** resolves §32: only catalog authorship. A later owner decision resolves
purchase scope (see below). Still open there:

- ownership of purchase delivery across the Game/Platform boundary;
- entitlement identity, expiry and revocation;
- idempotent purchase delivery across a Game/Platform boundary failure;
- refunds, chargebacks and fraud/audit/support correction.

No monetization choice, coin-to-money exchange rate, or specific Platform integration
contract is decided here.

Later owner decision (2026-09-28): the scope and portability of Store purchases are decided by D47
and D49 in `docs/architecture/reviews/OTERYN_GAME_ACCOUNT_PROGRESS_AND_QUEST_707_DISPOSITION_DECISION_2026-09-28.md`
§4.5 (account scope; cosmetic unlocks on every world while the entitlement is usable; items
claimable on compatible worlds of the bought-for profile family). Delivery ownership, entitlement
identity, expiry, revocation, refunds and fraud stay open as listed above.

## 4. Item `storevalue` is unchanged

This decision does not change the Item authoring disposition of `storevalue`
(`EXTERNAL_DOMAIN`). A Store offer's price is `price_coins` on the offer, never on the
Item. How `storevalue` relates to catalog offer prices is a follow-up.

## Follow-ups

1. Define the Game/Platform integration contract for coin balance reads and purchase
   debits (a `docs/contracts/` deliverable), before any Store offer can actually be
   purchased end to end.

   **Amendment (pending on acceptance of MARKET-COINS-0;
   `reviews/OTERYN_GAME_MARKET_COINS0_TIBIA_COINS_ON_THE_MARKET_DECISION_2026-09-30.md` §5).**
   The first slice of that contract is the Market coin custody contract (hold, settle, release,
   abort, status, balance read), written by MKTCOIN-CONTRACT-1 and accepted by Platform for
   its Wallet. Store purchase debits remain open here.
2. Resolve entitlement lifecycle (refunds, fraud, revocation) as a separate, explicitly
   scoped decision against the remaining open items in gap register §32.
3. Reconcile the Item `storevalue` disposition with catalog offer prices.
