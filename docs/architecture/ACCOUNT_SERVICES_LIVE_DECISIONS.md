# Oteryn account services: live prerequisites

ACCOUNT-SERVICES, 2026-11-08. These are Oteryn services. They must never contact
CipSoft. No real payment endpoints, provider keys or secrets belong in this repo.

The [accepted authority boundary](../contracts/CHARACTER_AUTHORITY_PLATFORM_BOUNDARY.md)
assigns account policy, commercial state, Bazaar listings/bids and coin wallets
to Platform. Game owns character/world custody and final fenced effects.
The existing [Bazaar contract](../contracts/OTERYN_GAME_BAZAAR_AUTHORITY_V1.md)
and Game lock/transfer/receipt implementations are the integration starting point.
Creating a Game wallet would make a second commercial authority.

## Platform continuation evidence

Read-only continuation check against `Oteryn/Oteryn-Platform` commit
`a2c93448a24fec7e15630ac1f6707c0d8a43e65f` confirms the following integration
state. Platform source is readable; it is not an allocated writable repository
for this Game task. These findings distinguish existing components from missing
client integration contracts.

- [Payment provider](https://github.com/Oteryn/Oteryn-Platform/blob/a2c93448a24fec7e15630ac1f6707c0d8a43e65f/portal-rs/crates/payments/src/provider.rs):
  `PaymentProvider` and HMAC-verifying `TestProvider` already exist. The test
  provider's `create_intent` returns `test:<intent_id>`, a provider reference,
  rather than a browser checkout URL. Extend that owner instead of adding a
  second provider/ledger authority in Game. Existing code is not evidence of
  executed Platform PostgreSQL qualification in this task.
- [Payment account routes](https://github.com/Oteryn/Oteryn-Platform/blob/a2c93448a24fec7e15630ac1f6707c0d8a43e65f/portal-rs/crates/payments/src/account.rs):
  the module exposes read-only premium, payment-history and receipt GET pages
  behind `PANEL_PREMIUM_PAYMENTS`. Those pages do not supply the native client's
  quoted order-create → checkout-URL → status/balance contract.
- [Character Bazaar routes](https://github.com/Oteryn/Oteryn-Platform/blob/a2c93448a24fec7e15630ac1f6707c0d8a43e65f/portal-rs/crates/bazaar/src/characters/http.rs):
  generic listing/bidding handlers exist, but `configured_router` mounts
  `unavailable_routes(true)`, returning HTTP 503 because custody/signing authority
  is not activated. Opening those refusal pages would not complete the feature.
- [Character services proposal](https://github.com/Oteryn/Oteryn-Platform/blob/a2c93448a24fec7e15630ac1f6707c0d8a43e65f/docs/contracts/OTERYN_V2_CHARACTER_SERVICES_COMMAND_CONTRACT_PROPOSAL.md):
  world transfer remains explicitly outside the slice; reserved command shapes
  do not authorize issuance or activation. Its coin reservation, eligibility
  and receipt integration needs a separate owning contract.

The payment provider implementation remains Platform-owned. The owner's revised
scope explicitly ships native client contracts, types, UI and test-only mocks
before Platform issuance. These integration gaps are deployment prerequisites,
not client blockers. The [Character Bazaar/world-transfer v1 contract](../contracts/OTERYN_CHARACTER_BAZAAR_V1.md)
and [coin purchase v1 contract](../contracts/OTERYN_COIN_PURCHASE_V1.md) define
the shipped native flows and their test-only mock authorities.
Missing live keys never block sandbox development. The task already authorizes
Oteryn implementation without another approval request.

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
must qualify the ledger/webhook PostgreSQL cases before live activation.
The native coin-purchase contract defines these endpoints for the client and
its test-only mock provider. Platform must implement issuance before deployment;
this does not prevent shipping and testing the native client flow now.

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
by this task. No invented default may debit users. The shipped client dialogs bind their
confirmation to server quotes, eligibility explanations and committed receipts;
test-only mock authority exercises the same typed HTTP contract.

The owner must also resolve the Bazaar contract's explicit activation questions:
snapshot/obligation policy, escrow account capacity, key custody and finite proof
retention. These blockers concern Oteryn's Platform and policies, not CipSoft.

For the independently implementable Game login challenge client and the exact
remaining Platform work, see [the challenge contract](../contracts/OTERYN_NATIVE_LOGIN_CHALLENGE_V1.md).

## Native Premium overview (T165)

The [Premium overview v1](../contracts/OTERYN_PREMIUM_PANEL_V1.md) uses the existing
Game character-information command (113, capability97), operation5, to observe
the current session's account through its existing Premium fence/coordinator.
The native in-game panel requires no additional Platform customer endpoint.
Actual subscription end remains separate from the shorter authority lease;
current indicators fail closed and all gameplay decisions remain unchanged.
The optional pre-login Platform projection remains unqualified and has no default
native entry; it is not required by the supported in-game flow. No payment or
Premium-grant control is added.
