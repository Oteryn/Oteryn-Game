# Game coin gifts v1 (T132)

Owner decision B, 2026-11-09. Platform alone owns coins, coin classes,
transfer policy, idempotency and receipts under [PLATFORM_COIN_WIRE_V1.md](PLATFORM_COIN_WIRE_V1.md).
The copied coin-wire-v1 fixtures and their provenance also govern transfers.
Game sends exactly POST `/internal/v1/coins/transfers`; no Game wallet exists.

Reference: `docs/reference/tibia-manual/store.md:15–19` and `store.md:38–43`: use the recipient's
full character name, in any World, never the sender's own account. Gifts use
positive multiples of 25 transferable coins, complete immediately at Platform,
and cannot be recalled. Non-transferable coins cannot fund gifts.

Game resolves the exact normalized full name against current live character
roots, without World restriction or prefix matching. Owner decision B 2026-11-09,
reference silent: ambiguous names across Worlds are rejected; the UI must not
silently choose an account. The wire's signed-bigint limit bounds gift amounts.

Review creates a 60-second server quote binding the resolved character, account,
name, amount, sender character, World, session generation and lease generation.
Confirmation re-proves current sender authority and recipient identity/ownership.
Owner decision B 2026-11-09, reference silent: expired or changed quotes require a
new review. A quote authorizes one immutable operation UUIDv7. An already prepared
operation retries its original exact Platform body/key even after quote expiry,
rename, disconnect or restart; recipient ownership is fixed at preparation.
A timeout/refusal never proves an older request cannot still commit. Pending
operations remain inspectable, retryable and reconciled by the node outbox.
No coin debit or optimistic wallet change occurs in Game.

## Wire

STORE_V1 capability 154, command 6127. Closed protobuf fields: 1 action
(1 review, 2 confirm, 3 operation status), 2 recipient UTF-8 name (64 bytes),
3 amount, 4 quote UUIDv7, 5 operation UUIDv7. Confirmation includes the exact
reviewed name and amount. Replies use the existing bounded coin JSON envelope.
Only authorized parties can inspect receipts; sender balances are hidden from
counterparties. Session initialization clears local gift drafts and pending UI.

The purpose-separated `OTERYN_COIN_SOURCE_CONFIG` adds `transfers_enabled`,
default false. This denies fresh gift confirmations while retaining reconciliation
of previously authorized intents. Platform flags also default OFF; no provider
keys or real payment integration are introduced.
