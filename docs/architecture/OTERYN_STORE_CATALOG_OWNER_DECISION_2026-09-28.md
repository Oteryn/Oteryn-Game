# Store Catalog Owner Decision — 2026-09-28

- Status: Owner-accepted decision record
- Date: 2026-09-28
- Decision owner: Oteryn project owner
- Issue: #162, Jira `KAN-16`
- Applies to: ownership of the in-game Tibia Store catalog and purchase delivery
- Evidence: `tools/content-schema/store-authoring/` (schema, converter, census over
  pinned Canary `47dfd51f` and Crystal `ff7ede5` GameStore catalog sources)
- Does not authorize: runtime, protocol, payment, entitlement or production
  implementation; does not resolve refunds, fraud or entitlement lifecycle

## 1. Catalog and delivery are Game content/runtime

The Store **catalog** — categories, offers, prices, product references — and
**delivery** of a purchased offer to the character (the store inbox) are Game content
and Game runtime. They are authored and served the same way as any other Game content
family (`tools/content-schema/store-authoring/`, mirroring the spell/item packages), and
their runtime behavior belongs to `protocol-oteryn` and the world model, not to Platform.

## 2. The coin balance and purchase ledger stay with Platform

The player's Tibia Coin balance, the payment flow, and the purchase ledger (what was
bought, when, for how much) remain Platform's, under Platform's existing web
identity/commercial ownership (ADR-0012) and `ARCHITECTURE_ANALYSIS_GAP_REGISTER.md`
§32 (`PROD-ENTITLEMENTS-01`). Game reads a coin balance and requests a debit through
whatever integration contract Platform exposes; Game never becomes the ledger of record
for real-money-backed currency.

## 3. Scope of this decision

This **partially** resolves §32: only catalog authorship and delivery ownership. Still
open there, unchanged by this decision:

- entitlement identity, scope, expiry and revocation;
- idempotent purchase delivery across a Game/Platform boundary failure;
- refunds, chargebacks and fraud/audit/support correction.

No monetization choice, coin-to-money exchange rate, or specific Platform integration
contract is decided here.

## 4. Item `storevalue` stays an external-domain field

An Item's `storevalue` (its price in Tibia Coins, when sold directly from the Item's own
in-world context rather than through a catalog offer) remains an `EXTERNAL_DOMAIN` field
on the Item authoring schema, per the existing item-authoring field disposition
convention. Price for a Store *offer* lives on the offer (`price_coins`), never on the
Item; the two are separate numbers that may legitimately disagree.

## Follow-ups

1. Define the Game/Platform integration contract for coin balance reads and purchase
   debits (a `docs/contracts/` deliverable), before any Store offer can actually be
   purchased end to end.
2. Resolve entitlement lifecycle (refunds, fraud, revocation) as a separate, explicitly
   scoped decision against the remaining open items in gap register §32.
