# Oteryn native coin purchases — v1

Status: implemented Game-client contract and test-only sandbox provider. Platform
issuance, durable accounting and hosted checkout are prerequisites to deployment.
This is an Oteryn service; no CipSoft account or payment service is involved.
No real payment adapter, endpoint, key or secret ships in Game.

## Authentication and transport

Use the configured Oteryn Platform origin and a separate OAuth access token with
`game:account-services` scope (PKCE, loopback callback and login challenge as in
OTERYN_NATIVE_LOGIN_CHALLENGE_V1.md). Game ticket tokens remain separate. Every
endpoint below authenticates its payer from the token; clients never choose an
account ID. TLS is required outside the existing local development exception.
Reject redirects, unknown JSON fields, missing fields, unsupported versions,
noncanonical UUIDv7 IDs, oversized requests/replies and invalid state bindings.
The client limits request/reply bodies to 8192/65536 bytes and uses the existing
cancellation/deadline policy. Nullable fields listed below must be present.

In-game entry includes the admitted `character_id`. Platform must verify the
current OAuth subject owns it before issuing a catalogue or accepting an order.
The selector's account-services entry sends null. Orders permanently bind to the
payer account; a later character transfer cannot move coins or payment liability.
Status endpoints authorize against that payer, not current character ownership.

## HTTP interface

All paths start `/api/v1/native-account/coin-purchases`:

| Method/path | Input | Reply |
| --- | --- | --- |
| GET `/catalogue` | optional `character_id` query | CoinCatalogue |
| GET `/balance` | none | CoinBalance |
| POST `/orders` | CoinOrderRequest | CoinOrder |
| GET `/requests/{request_id}` | canonical UUIDv7 | CoinOrder |

Success is HTTP 200 with JSON. Authentication/authorization failures use 401/403;
unknown requests use 404; mismatched request replays, consumed/expired quotes or
changed account eligibility use 409; invalid data use 400; temporary failure uses
503. Non-200 is never evidence of payment or permission to replace a request ID.
A POST may have committed even when its reply is lost. Retry its **identical**
body or poll by request ID. Retain request-to-order bindings durably; identical
replays return the same order's current state, even after quote expiration.
A changed body with an existing ID must never create or alter an order.

### Types

All objects deny unknown fields. All IDs are canonical lowercase UUIDv7 strings.
`version` is integer 1. Coin values are integers in 0..999999999999; package/order
coin counts are positive. No floating point arithmetic is used for money.

- Price: `minor_units` (1..9999999999), `currency` (three uppercase ASCII letters),
  `exponent` (0..3). Display uses integer division and zero-padded fractional digits.
- CoinPackage: `id`, `quote_id`, `title` (1..64 characters), `coins`, `price`,
  `valid_for_seconds` (1..300). Quotes bind payer, optional character, package,
  exact coins, price, environment and expiration; each is consumed at most once.
  Server rechecks expiry and eligibility atomically at order creation. Client
  starts expiry before fetching the catalogue and confirms the exact displayed
  coins and price. Client POST contains neither a chosen price nor coin count.
- CoinCatalogue: `version`, `sandbox` (must be true), nullable `character_id`,
  `packages` (at most 20), `recent_orders` (at most 20). Package IDs and order
  request IDs are unique. History is payer-scoped; catalogue character context
  must equal the request. Retain complete history server-side beyond this bounded
  recent view and provide support/audit access with authorization.
- CoinBalance: `version`, `revision` (positive, monotonic per payer wallet),
  `total_coins`, `available_coins`, `transferable_coins`, `review_required`.
  Platform has one coin class: all grants and purchases credit the same balance,
  and all available coins are transferable. `available_coins` and
  `transferable_coins` describe the same available balance; neither exceeds total.
  Reservations remain unavailable. Legacy snapshots with a smaller
  `transferable_coins` remain accepted; the UI presents `available_coins`.
  The internal coin wire retains `transferable` / `transferable_after` for the
  whole available balance and always-zero `non_transferable` /
  `non_transferable_after`; history `coin_class` is always `transferable`.
  Client refuses revision regression or contradictory equal-revision snapshots.
- CoinOrderRequest: `version`, `request_id`, `package_id`, `quote_id`, nullable
  `character_id`. Generate request ID once after confirmation and retain on
  ambiguous failure. Pending requests prevent another create in that view.
- CoinOrder: `version`, `order_id`, `request_id`, `revision` (positive),
  `package_id`, `quote_id`, nullable `character_id`, `coins`, `price`,
  `sandbox` (true), `state`, nullable `checkout_url`, nullable `receipt_id`,
  `review_required`, `code` (1..96 characters, public support text/code).
  Immutable bindings never change across revisions. Client checks the create
  response against its request and confirmed price/coins. Status cannot regress
  revision, contradict equal revisions, or move lifecycle backwards.

States: `awaiting_payment` has checkout URL and no receipt; `processing` and
`failed` have neither; `paid`, `refunded`, `chargeback` have a receipt and no URL.
Awaiting may advance to any later state; processing cannot return to awaiting;
paid may become refunded or chargeback; refunded may become chargeback; failed
and chargeback are terminal. Polling may skip intermediate states. Browser open,
return, query string, local UI and checkout success page **never credit coins**.
Client reads authenticated order status and a separate fresh wallet snapshot.

Sandbox URL policy: exact configured Platform origin, exact path
`/native-account/coin-checkout/{order_id}`, no credentials, query or fragment.
Foreign provider hosts are refused, including on history/status replies. URL
Debug is redacted and it is not persisted in settings. A failed browser launch
can reopen the same order without another create. Closing the Game session
cancels its local shop; reopening can recover orders through payer history.

## Platform implementation prerequisites

Implement a `PaymentProvider` abstraction with checkout creation and raw signed
webhook/receipt verification. Ship a sandbox adapter only until owner approval.
Game's test-only adapter implements this trait, generates its HMAC-SHA256 key at
runtime, verifies the signature before decoding, and refuses malformed, unbound,
amount-mismatched or changed event-ID replays. It is not a production adapter.
The local mock webhook route exists only in tests and is never mounted by Game.

Platform must supply:

1. Durable orders, unique `(payer, request_id)` immutable request bindings,
   consumed quotes, provider order bindings, unique provider event IDs, an
   append-only wallet ledger and immutable receipts. Reserve/create the order
   atomically; do provider I/O outside database transactions with a durable
   outbox/idempotency key, then persist the checkout binding.
2. Verify raw-body signatures with the chosen provider's current/rotated keys,
   environment, audience, timestamp/replay policy and payload limits before any
   application. Match provider account/order, currency, exact amount and payer
   binding. Never trust a client-delivered receipt without provider verification.
3. In one transaction lock the order/wallet, record the immutable verified event,
   transition state, insert uniquely keyed ledger credit/reversal and receipt,
   update wallet revision and commit. Retries/concurrent deliveries/restarts must
   produce at most one credit and at most one full reversal. A changed event ID
   payload is rejected and retained for audit; late paid events cannot resurrect
   a refunded/charged-back order. Out-of-order reversal before credit is flagged
   for review and must fence later credit.
4. Full refunds/chargebacks reverse available credited coins once. Insufficient
   unreserved balance flags order/account review and freezes new purchasing;
   never underflow or silently mint/erase balances. Preserve liabilities, receipts
   and evidence for reconciliation. Partial refunds require a new contract.
5. Hosted sandbox checkout, authenticated history/status/balance, ownership
   checks and shared wallet accounting with Bazaar/transfer/Store. Before live,
   run real PostgreSQL rollback/concurrency/replay/restart/insufficient-balance
   cases with executed evidence. Game's HTTP mock tests are not that PG proof.

## Owner decisions before real money

Choose provider and merchant entity; supported countries, age/guardian rules,
prices/packages/currencies/tax treatment; transferable coin restrictions;
refund/chargeback/support policy; terms/privacy/consumer disclosures and consent;
fraud/rate limits, retention/deletion/reconciliation obligations and operations.
Obtain legal review and define settlement, key rotation, webhook signing and
incident procedures. Keep keys in deployment secret management. A real provider
requires a separately reviewed adapter and revised checkout-origin policy;
changing `sandbox` to false is deliberately rejected by this client contract.
