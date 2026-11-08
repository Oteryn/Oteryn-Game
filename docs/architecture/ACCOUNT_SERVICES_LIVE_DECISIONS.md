# Oteryn account services: live prerequisites

ACCOUNT-SERVICES, 2026-11-08. These are Oteryn services. They must never contact
CipSoft. No real payment endpoints, provider keys or secrets belong in this repo.

The [accepted authority boundary](../contracts/CHARACTER_AUTHORITY_PLATFORM_BOUNDARY.md)
assigns account policy, commercial state, Bazaar listings/bids and coin wallets
to Platform. Game owns character/world custody and final fenced effects.
The existing [Bazaar contract](../contracts/OTERYN_GAME_BAZAAR_AUTHORITY_V1.md)
and Game lock/transfer/receipt implementations are the integration starting point.
Creating a Game wallet would make a second commercial authority.

## Payments before going live

The owner must decide:

- Provider and operating entity, supported countries/currencies and sandbox/live
  account separation; signing algorithm, key custody/rotation and webhook replay
  window. Do not configure a live provider until commercial/legal approval.
- Coin package sizes, fiat prices, tax/VAT treatment, rounding, promotions and
  whether acquired coins are transferable. Prices must be immutable server quotes;
  the client must never supply the amount to credit.
- Terms, privacy notice, consumer cancellation/refund rights, age restrictions,
  fraud/chargeback handling, dispute operations, proof retention and support.
- Negative balance/debt policy when refunded coins have already been spent or
  reserved. Refunds must never silently create spendable value or release a
  reservation whose delivery might still commit.

Platform's implementation must own `PaymentProvider`: create order → checkout URL
→ signed receipt/webhook verification. Ship only a sandbox/mock implementation
until the decisions above are approved. Persist immutable order/package/fiat/coin
bindings, unique provider event IDs, ledger and receipts. Credit exactly once in
the transaction that records the verified event. Reject changed replay payloads;
out-of-order events and concurrent retries must not double-credit. Refunds and
chargebacks append compensating entries or a durable debt/review flag; never
erase an original receipt. No provider network call belongs inside a DB transaction.

The client needs authenticated Platform APIs for package quotes, idempotent order
creation, order status, coin balance and receipts. It opens only validated HTTPS
sandbox checkout URLs in the system browser; browser return is not payment proof.
Refresh status/balance from Platform after return or explicit refresh. Platform
must qualify the ledger/webhook PostgreSQL cases before this can be called DONE.
No such public order API is defined by the current Game contract, so this Game
lane cannot supply a working checkout or payment provider without Platform work.

## Bazaar and world transfer

Platform must supply authenticated listing, own-auctions, bid, bounded history,
reservation and receipt APIs and their durable auction worker. The worker must
settle only after Game's committed fenced ownership-transfer receipt; replay and
concurrent bids/expiry/settlement need real PostgreSQL qualification. Items and
gold remain with the character. Neither a UI success nor a wallet reservation
is evidence of a completed ownership transfer.

World transfer needs an accepted coin quote/reservation contract and cooldown
policy as well as Game eligibility (offline and lease/session fenced, no guild,
house or active market obligations). Price and cooldown values were not supplied
by this task. No invented default may debit users. Client dialogs require these
real services, quotes, eligibility explanations and committed receipts.

The owner must also resolve the Bazaar contract's explicit activation questions:
snapshot/obligation policy, escrow account capacity, key custody and finite proof
retention. These blockers concern Oteryn's Platform and policies, not CipSoft.

For the independently implementable Game login challenge client and the exact
remaining Platform work, see [the challenge contract](../contracts/OTERYN_NATIVE_LOGIN_CHALLENGE_V1.md).
