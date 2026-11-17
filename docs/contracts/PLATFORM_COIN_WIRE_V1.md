# Coin transfer and market wire v1

Status: **ACCEPTED** — owner decision 2026-11-09 (#3058 CLIENT-GAPS, option B).
Platform Wallet owns coins. Game consumes this API and owns gold, trade matching,
character resolution, offer expiry scheduling and the durable retry queue. No
payment provider or real payments are enabled by this contract.

## Transport and configuration

TLS 1.3, HTTP/1.1, verified client certificate, exact identity allowlist group
`game-coins`. `PORTAL_GAME_COINS_IDENTITIES` is a JSON object keyed by exact
certificate SAN, with an empty array per identity, for example
`{"coins.test.invalid":[]}`. This follows the existing identity-map object format;
coin routes have no world/channel scopes.
`PORTAL_COIN_TRANSFERS_ENABLED` and `PORTAL_COIN_MARKET_ENABLED` default false.
Only literal `true` enables a flag. Transfers require the first flag; market
mutations require the second. Reads require either. An off flag returns 503
`{"result":"unavailable"}` before body or authentication processing. A missing,
wrong or unverified peer returns 401 with an empty body. Identities must not overlap
another route group. Public headers never supply verification.

All responses use application/json and Cache-Control: no-store except the empty
401. Every request body is bounded to 1024 bytes, JSON depth 4 (root consumes one
level; a container at level 4 is refused). POST requires application/json with no
parameters or compression. GET has no body; the guard authenticates an internally
supplied empty object after enforcing that rule. History response is at most 65536
bytes. Other responses are at most 2048 bytes.

Canonical encoding follows Store wire: compact UTF-8, exact member order below,
no whitespace, escapes, BOM, duplicate or unknown members, floats or exponents.
No Unicode normalization or key sorting. Digest = lowercase SHA-256 of the exact
canonical request bytes, bound to method and route. GET does not claim a key.
Changing any mutation member under a used operation key is a conflict. Identifiers
are lowercase canonical UUID text; operation and reservation IDs are UUIDv7;
account IDs are non-nil UUIDs. Amounts, gold prices, balances and deltas are decimal
strings in signed bigint range, without plus signs or leading zeroes. Amount and
gold_price must be positive. Timestamps are RFC3339 UTC.

## Routes and ordered members

All paths start `/internal/v1/coins/`. Objects are closed. Fields listed together
are in wire order; no member is optional unless explicitly nullable.

| Method/path | Request members |
| --- | --- |
| POST transfers | operation_id, sender_account_id, recipient_account_id, amount |
| POST market/offers | operation_id, seller_account_id, amount, gold_price |
| POST market/settlements | operation_id, reservation_id, buyer_account_id, amount |
| POST market/cancellations | operation_id, seller_account_id, reservation_id |
| GET operations/{operation_id} | empty body |
| GET accounts/{account_id}/balance | empty body |
| GET accounts/{account_id}/history?cursor=&limit= | empty body |

The offer operation ID is the offer/reservation ID. Only SELL offers call offers;
BUY offers reserve gold in Game and never call this route. gold_price is a positive
per-coin price in Game gold; it is bound into offer idempotency, never debited here.
Recipient resolution accepts account ID only. Existing character projections do
not provide a globally authoritative name-to-account money authorization port;
Game resolves names before calling transfers.

A successful mutation or operation lookup returns `{result,receipt}`. result is
`ok` or `replay`; operation lookup returns `ok` for the stored receipt. Receipt:
`{operation_id,kind,account_id,counterparty_account_id,amount,transferable_after,
non_transferable_after,reserved_after,occurred_at}`. Counterparty is UUID or null;
kind is transfer, reserve, settlement or release. The receipt describes the sender
or seller immediately after that operation, never the current balance on replay.
A missing operation returns unknown_account (no separate not-found result).

Balance returns `{result,account_id,transferable,non_transferable,reserved}`.
History returns `{result,entries,next_cursor}`; entry members are
`{entry_id,operation_id,kind,coin_class,counterparty_account_id,available_delta,
reserved_delta,occurred_at}`. coin_class is always `transferable`, including for
legacy non-transferable ledger rows.
Counterparty is the other account, including on a recipient entry. Cursor is the
last entry UUID, exclusive, ordered descending. next_cursor is null on an empty
page, otherwise the last entry ID (a following empty page ends the scan). Default
limit 25, permitted range 1–100. Empty cursor means newest; duplicate or unknown
query members, invalid cursor and invalid limit return invalid.

## Results and atomicity

| Result | HTTP | Meaning |
| --- | --- | --- |
| ok | 200 | committed mutation or successful read |
| replay | 200 | original committed receipt; no repeated effect |
| insufficient_transferable | 409 | spendable transferable balance or reservation remainder too small/closed |
| account_inactive | 409 | either account is not Active |
| transfer_blocked | 409 | either account explicitly blocked |
| unknown_account | 422 | account or operation absent |
| idempotency_conflict | 409 | key belongs to another complete payload or operation kind |
| invalid | 422 | invalid body, identifiers, amount, cursor, reservation ownership, self-transfer or overflow |
| unavailable | 503 | off, misconfigured, ambiguous authorization, storage or transport failure |

Errors contain only `{result}`. Transport/body/canonical failures are invalid
(422), including size violations; unauthorized remains empty 401. No refusal
commits a claim or ledger entries. Unknown authorization fails closed as unavailable.

Platform has one coin class: every available coin is transferable. Top-ups,
reward/promotion/staff grants, gifts and settlements credit that one balance;
Store spending, transfers and SELL reserves consume it. `transferable` and
`transferable_after` carry the whole available balance; `non_transferable` and
`non_transferable_after` are always `"0"`. Reserved coins remain unavailable.
Wire names and shapes are unchanged; no field or enum value is removed and no
`coin_kind` field is added. Game still accepts legacy non-zero non-transferable
fields and presents their saturating sum with the transferable amount as one balance.
Both transfer parties and market settlement parties must be Active and transfer Allowed.

This one-class rule follows Platform contract commit `bcaa1605`, section "One coin
class", and implementation `a058456e`.

Settlement uses the trade UUIDv7 as operation_id and atomically reduces the seller's
remaining reservation and credits the buyer by the same amount in one transaction.
Partial fills are allowed up to the remaining amount; exhaustion closes the reserve.
Cancel and expiry use cancellations to release only the remaining reserve. Each
trade and cancellation uses a fresh key. Full/partial settlement and cancellation
serialize on the same seller wallet and reservation. Balances never go negative
or overflow bigint. Entries and operation receipts are immutable.

Keys have global wallet scope and do not expire. Successful exact replay returns
the original receipt even after later fills or cancellation; changed payloads
conflict. Authorization still applies on retries. Refusals are retryable and are
not stored as successful decisions. A timeout means neither success nor failure:
Game GETs operations/{operation_id} or repeats the identical request and key.
An absent operation permits retry, never proves that an in-flight request failed.
Game retains unresolved intents durably and reconciles before releasing its gold.
Platform does not claim atomicity across the Game gold database.

## Fixtures

`fixtures/coin-wire-v1/manifest.json` lists every route, valid request and response,
all result codes, canonical byte counts and SHA-256 vectors, plus invalid inputs.
GET request fixtures use `{}` as a guard adapter artifact; their HTTP bodies are empty.
