# Oteryn Character Bazaar and world transfer v1

Owner decision, 2026-11-08: ship the Game-client contract, UI and mock authority
now; Platform issuance is a deployment prerequisite. No CipSoft service or asset
is used. This contract extends the accepted
[authority boundary](CHARACTER_AUTHORITY_PLATFORM_BOUNDARY.md), not coin custody:
Platform owns auctions, bids, coin reservations and orchestration; Game owns
character/world mutation, current leases/session generations and final receipts.

## Authentication and transport

The character selector's **Bazaar / account services** button opens a separate
authorization-code + PKCE S256 flow with scope `game:account-services`. Normal
login still requests `game:ticket` only. Platform must explicitly grant and check
the service scope and current account policy on every request. No password,
account selector, bearer or signing material enters the UI model or saved settings.
On return, Game-only sign-in is refreshed before character selection/ticket issue.
The existing Oteryn login-challenge extension also applies to this OAuth flow.

HTTPS to the configured Oteryn Platform origin only (HTTP loopback in local
tests), no redirects or proxies. Responses are HTTP 200 JSON, `Cache-Control:
no-store`; refusals use non-200 status and never imply success. Requests are at
most 8192 bytes and responses at most 65536 bytes, within the existing 15-second
HTTP timeout and cancellation boundary. UTF-8 JSON uses exact fields, rejects
duplicates/unknown members, and has `version: 1`. IDs are lowercase UUIDv7.
Amounts/counters are JSON unsigned integers; coin amounts are 0–999999999999.
Text has no control characters and is bounded below. No Game socket opcode is
allocated: these operations precede admission and use the Platform HTTP contract.

## Endpoints and exact DTOs

Types live in `oteryn_platform_client::native_login::bazaar`, with the field
names below. Object member order is immaterial; duplicate members are rejected.
Every listed field is required; nullable fields use JSON null.
Row identities within each returned collection must be unique.

| Endpoint | Request | Response |
|---|---|---|
| GET `/api/v1/native-account/bazaar/auctions` | `view=all|own|history`, `limit=50`, optional URL-encoded `cursor` (1–256 bytes) | `BazaarPage` |
| POST `/api/v1/native-account/bazaar/commands` | `BazaarRequest` | `OperationReceipt` |
| GET `/api/v1/native-account/bazaar/commands/{request_id}` | scoped to current account | `OperationReceipt` |
| POST `/api/v1/native-account/bazaar/transfer-quotes` | `{version,character_id,destination_world_id}` | `TransferQuote` |

- `BazaarPage`: `{version,view,auctions,owned_characters,worlds,balance,next_cursor,recent_operations}`.
  At most 50 auctions and 50 current-account operations per page, 100 owned
  characters and 256 destination worlds. `next_cursor` is null at the end;
  the cursor advances the auction/operation view together. History includes
  terminal auctions and receipts; every view exposes current-account pending
  operations before terminal history for recovery. A reopened client polls these
  IDs and blocks new mutations until resolved; it never reconstructs a lost POST
  payload from a receipt or resubmits it with a new identity.
- `Auction`: `{id,character_id,character_name,world_id,world_name,level,revision,
  minimum_next_bid,highest_bid,ends_at,state}`. Names are 1–64 UTF-8 bytes;
  revision is positive; minimum next bid is positive; `ends_at` is positive Unix
  seconds; state is `open|settling|sold|expired|cancelled`.
- `OwnedCharacter`: `{id,name,world_id}`; `TransferWorld`: `{id,name}`. Names
  are 1–64 bytes. These are server-owned portfolios and eligible world choices.
- `BazaarRequest`: `{version,request_id,command}`. `command` is one of:
  `{"operation":"bid","auction_id":ID,"expected_revision":N,"coins":N}`;
  `{"operation":"sell","character_id":ID,"minimum_bid":N,"duration_hours":N}`;
  `{"operation":"transfer","quote_id":ID}`. Bid/minimum are positive;
  auction revision is positive; duration is 1–168 hours. Product-specific tighter
  duration rules are enforced by Platform; unsupported values are refused.
- `TransferQuote`: `{version,quote_id,character_id,destination_world_id,price_coins,
  cooldown_seconds,valid_for_seconds,blockers}`. Lifetime is 1–300 seconds;
  at most 16 blockers, each 1–160 bytes. Quote binds the account, character,
  source/destination worlds, current Game revisions/eligibility, exact price and
  cooldown. The client never supplies price on confirmation and never overrides
  a blocker. Client lifetime starts before the quote request; Platform also
  enforces expiry at commit.
- `OperationReceipt`: `{version,request_id,state,receipt_id,code,charged_coins,balance}`.
  State is `pending|committed|rejected`; code is 1–96 bytes. Committed requires a
  non-null receipt ID. Pending/rejected require null receipt ID and zero charged
  coins. Balance is the immutable post-operation available-coin snapshot, not an
  inferred local delta. The client refreshes the account page after a terminal
  receipt and displays that current read separately from receipt history.
  Request IDs must match the submitted/polled identity. A committed **bid**
  receipt proves the reservation only; auction `sold` requires settlement below.

## Required authority, atomicity and replay semantics

Platform authenticates seller/bidder from the bearer, checks current account
restrictions, rejects self-bids and closed/stale auctions, locks accounts in
deterministic order, and reserves funds before accepting a bid. Bid `coins` is
the absolute new maximum, not an additional debit. Outbid release, expiration,
fees, seller proceeds and commissions belong to the Platform ledger and retain
immutable receipts; no browser/UI result authorizes a credit.

Sell must acquire a Game-owned durable character lock before publishing a usable
auction. Game proves current ownership, actor absence, no live lease, current
session/recovery generation and all eligibility obligations under the existing
[Bazaar custody contract](OTERYN_GAME_BAZAAR_AUTHORITY_V1.md). Offline is a current
authority check, never a client assertion. Admission and all character writers
must respect the lock. Minimum bid and duration are bound to the listing.

Settlement must be a durable, restartable orchestrator. Game atomically transfers
the existing character's ownership with immutable receipt/outbox under current
portfolio, admission, lease/session and recovery fences. Items, equipment, bank
gold and character state remain with the same CharacterId; no character cloning.
Platform settles seller credit only after the exact committed Game transfer
receipt; ambiguous delivery cannot release funds or unlock the character.
Auction history preserves the listing, bid and settlement receipt relationships.

World transfer preserves AccountId and CharacterId and changes WorldId through
Game authority, never a ChannelId edit. Before quote and again under the final
mutation fence, require offline/no live lease, cooldown elapsed, no guild, house,
or active market offers, and safe disposition of all source/destination world
obligations. Reserve the quoted coins; debit/settle once only on the matching
Game committed transfer receipt. Reject stale/expired/foreign quotes. A concurrent
login, guild join, house acquisition, market offer or world change must serialize
against transfer, not pass between the check and mutation.

`(account_id,request_id)` is the durable operation identity. Platform stores the
full semantic request binding before effect; identical retries return the same
pending or immutable terminal receipt. A changed payload on the same key returns
409 and changes nothing. GET status and account history recover lost replies.
The native controller keeps the exact request ID/payload after transport failure
and prevents another mutation until it is resolved. Back/closing UI does not
cancel a durable command. Platform retains all pending operations and bounded
paginated receipts so a later session can recover them. Contradictory terminal
receipts are an error, never a second success.

## Client, tests, and remaining Platform issuance

`apps/client/src/features/local-account-services/` provides the browser, own
auctions/history, bid confirmation, character/minimum/duration sell form, world
selector, eligible quote confirmation and receipt/status/retry UI. It uses real
typed HTTP calls. `native_entry/authorize.rs` registers the production selector
entry and refreshes Game-only authorization afterwards. A responsive waiting
notice accompanies bounded HTTP operations on Windows.

The test-only HTTP authority verifies current mock eligibility and request
bindings; it exercises a committed-but-lost response, same-ID retry, changed-ID
regression, wrong payload replay, history recovery, all six transfer guards and
a lease acquired after quote. The production entry test traverses real OAuth
loopback callbacks, the service presenter/controller/HTTP path, then ticket issue.
Wire tests reject malformed, wrong-version, oversized and mismatched replies.
This proves the shipped client slice, not real financial or Game persistence.

Platform must implement these scoped endpoints and DTOs, OAuth scope issuance,
durable auction/coin/quote/receipt journals, restartable custody/settlement workers,
Game lock/ownership/world-transfer integration and current-policy enforcement.
Qualify the atomic fences, concurrent bids/settlement, changed replays, restores,
receipt publication, world obligations and ledger conservation with real Game and
Platform PostgreSQL cases before enabling issuance. No SQL or new Game persistence
is introduced by this client/contract slice, so there are no new Game PG cases.
