# Game coin market and Platform handoff v1 (T125)

Owner decision B, 2026-11-09. Platform wire authority is the accepted
`OTERYN_V2_COIN_TRANSFER_AND_MARKET_CONTRACT.md`, copied as
[PLATFORM_COIN_WIRE_V1.md](PLATFORM_COIN_WIRE_V1.md). Fixture provenance is in
`fixtures/platform-coin-wire-v1/PROVENANCE.md`. This wire supersedes the candidate
MARKET-COINS-0 §5 HOLD/CLAIM/ABORT proposal; those calls are not sent.

Game owns one virtual coin book per World, its gold bank/escrow, matching,
expiry, character-name resolution and durable retry intents. Platform owns every
coin, its transferable split, reservations, immutable receipts and coin history.
Channel is only session authority, never the coin or gold book key. Coin balances
are display snapshots, never Game balances, items or Inbox entries.

Rules: `docs/reference/tibia-manual/store.md` §6.1–6.2 (account-wide coins,
free and Premium participants, transferable-only trades, expandable history),
`docs/reference/tibia-manual/controls_trading.md` §4.3.3 and MARKET-0 §3–§6
(bank-only fee, per-World gold, 30-day expiry, 100 open offers, maker price/time,
no self-account trades, no junior bank users). Canary `04b83b5`
`src/game/game.cpp:10739–10748` pins a 2% fee clamped to 20..1,000,000 gold.
Coin offers allow 1..64,000 coins, positive per-coin gold prices and total at most
999,999,999,999 gold; market amounts are not restricted to the gift step of 25.
The fee is never refunded after an offer opens. No Premium restriction applies.

SELL opens after Platform reserves transferable coins. BUY holds gold locally
and makes no Platform offer call. Taking BUY first creates a transient SELL
reservation, then settles it to the buyer. Taking SELL holds the buyer's gold
before settlement. Crossing offers match by price then creation time, at the
older maker's price. Price improvement returns unused buy escrow. Cancellation
and expiry release only unused reserves or buy gold, using fresh immutable keys.

There is no distributed transaction. Every Platform request is bound to a durable
Game intent before network I/O. Retries keep exactly its method, route, canonical
body and UUIDv7 key. A timeout, unknown operation or refusal never proves an older
request cannot still commit. Unresolved settlement gold remains held; the worker
retries and reconciles on startup and while serving. Only a receipt bound to the
exact request permits final gold settlement. A full bank defers gold completion;
it never discards or reissues coins. Operations and receipts remain inspectable.

Every fresh user intent re-proves recovery, node, World/Channel scope, session
generation and character lease and resolves current account ownership. Background
steps act only on the immutable, previously authorized offer/intent, under fresh
recovery/node/scope proof. Gold steps append conservation events and bank entries
in the same transaction; bank delta + escrow delta + fee burn is zero. Existing
bank proof branches are retained. Published coin receipts are immutable.

## Wire

MARKET_V1 capability 63, command 6126. Closed protobuf request fields:
1 action; 2 amount; 3 per-coin gold price; 4 offer/operation-to-inspect UUIDv7;
5 immutable confirmation UUIDv7; 6 history UUID cursor or bounded decimal offer-page cursor.
Offers use 100-row pages, at most 700 visible per side; My Offers shows up to 700
rows and 100 recent operations so unresolved confirmations remain recoverable. Actions 1 book, 2 own offers,
3 sell, 4 buy, 5 take, 6 cancel, 7 Platform history, 8 operation detail, 9 balance.
Mutations require a confirmation UUID; account/world/channel IDs are never inputs.
Reply is one bounded UTF-8 JSON view in protobuf field 1, at most 65,536 bytes.
Server account IDs and credentials are excluded from the client projection.

The node reads optional `OTERYN_COIN_SOURCE_CONFIG`, a bounded JSON file with
`source_authority`, `endpoint` (IP:port), `peer_name`, `trust_roots_der` (paths),
`client_certificate_der`, `client_key_der`, optional `market_enabled` (false by default).
Owner decision B 2026-11-09, reference silent: this node admission switch prevents
fresh local gold holds and fees when the coin market has not been enabled by operators.
Disabling it retains reconciliation of existing intents. This is a separate mTLS identity in
Platform's exact `game-coins` group, not the Store/evidence identity. No file means
unavailable. Platform mutation flags default OFF; no payment provider is enabled.
