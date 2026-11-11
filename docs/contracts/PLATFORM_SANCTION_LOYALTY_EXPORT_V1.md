# Platform sanction and loyalty export V1

Status: Game-owned consumer contract, 2026-11-11 (owner decision 2026-11-11: Game pulls, Platform
never calls Game SQL). Programme items N009 (own sanction record, capability 497) and N014
(Loyalty highscores, capability 498).

## Authority

Platform owns identity, commercial state, sanction decisions and loyalty
(`CHARACTER_AUTHORITY_PLATFORM_BOUNDARY.md`, ADM-01 §6.7). Game pulls two normalized Platform
streams and keeps read-only projections for its own clients. Game never decides, enforces,
edits, expires or revokes a sanction, and never awards, derives or changes loyalty. A missing
projection row means "nothing pulled", never "no sanction" in Platform.

Platform has no route into Game SQL (`OTERYN_V2_GAME_ENFORCEMENT_COMMAND_CONTRACT`: no direct or
shared SQL). The projection functions run only as `oteryn_game_runtime`, called by Game's own
puller; `oteryn_game_control` has no `EXECUTE` on them (migrations 2802, 2803).

## Transport

Each request is one `POST` with `Content-Type: application/json` over private mutual TLS, built
by the node's shared Platform client builder (the one the Premium snapshot read uses: HTTPS only,
TLS 1.3, only the Platform CA trusted, no proxy, no redirects, 5-second timeout). No other
transport or dependency is used.

Each stream has its own dedicated client identity, distinct from the Premium, native-evidence
and bootstrap identities and from the other stream's; Platform answers a repeated identity with
`503`. Each identity's certificate carries a DNS SAN that Platform binds to that stream.

| Variable | Value |
|---|---|
| `OTERYN_PLATFORM_STREAMS_URL` | Platform's private origin, `https://host:port`, shared by both streams |
| `OTERYN_PLATFORM_STREAMS_PLATFORM_CA_PEM` | PEM file of the Platform CA bundle, shared by both streams |
| `OTERYN_PLATFORM_SANCTION_STREAM_CLIENT_IDENTITY_PEM` | PEM file: the sanction stream's client certificate chain and private key |
| `OTERYN_PLATFORM_LOYALTY_STREAM_CLIENT_IDENTITY_PEM` | PEM file: the loyalty stream's client certificate chain and private key |

Game refuses a configuration in which an enabled stream's leaf certificate equals the Premium
identity's (`OTERYN_PREMIUM_CLIENT_IDENTITY_PEM`) or the other enabled stream's; no puller
starts then.

- Only a `200` with `application/json` and a body of at most 262,144 bytes is a page. Any other
  status, a redirect, another content type, an oversized body, a timeout or a transport failure
  is a failed pull. A `429` or `503` `Retry-After` is honoured up to 60 seconds.
- A failed pull, a refused page or a store failure is retried with exponential backoff from
  1 second, capped at 60 seconds, with jitter.

Each stream is off unless its variable is exactly `true`: `OTERYN_PLATFORM_SANCTION_STREAM_ENABLED`
and `OTERYN_PLATFORM_LOYALTY_STREAM_ENABLED`. Unset or `false` is off. Any other value, or an
enabled stream without its own valid identity, origin or CA, starts no puller and logs
`platform_streams_disabled`. Gameplay and admission never wait on a pull.

## Cursor and paging

Platform's cursor is opaque to Game: 1..=256 printable ASCII bytes without spaces. Game stores
one cursor per stream in `game_platform_stream_cursors` (migration 2802; `NULL` before the first
page).

Request, both streams:

```json
{"schema": "<request schema>", "after": "<stored cursor>" | null, "limit": 200}
```

`limit` is 1..=500; Game asks for 200. Response:

```json
{"schema": "<page schema>", "items": [ ... ], "next_cursor": "<cursor>" | null}
```

- At most `limit` items, in Platform's stream order.
- `next_cursor` is the cursor to send next. It is required when `items` is non-empty. On an
  empty page it may be `null`, which keeps the stored cursor.
- A page of exactly `limit` items is followed at once; a shorter page means caught up, and the
  next pull follows 30 seconds later.
- Unknown fields, another schema, or any item outside its rules refuse the whole page: nothing
  of it is projected and the cursor stays.

Game projects a page in one transaction: it locks the stream's cursor row, requires it to still
equal the request's `after` (otherwise another puller advanced it and the page is dropped),
projects every item through its revisioned function, and stores `next_cursor`. A crash or a
refused page leaves the cursor at the last fully projected page. Because every projection is
idempotent by revision, Platform may redeliver items, deliver an older revision after a newer
one, or be replayed from `after: null`.

An item whose account has no Game Character, or whose Character is not a Character of that
account, is skipped and counted; it never blocks the stream. An account that gets its first Game
Character later shows Platform's values from the next revision Platform streams for it, or after
the stream is replayed from `null`.

## Sanction stream

`POST /internal/v1/support/sanctions/stream/read`, request schema
`oteryn.sanction_stream_request.v1`, page schema `oteryn.sanction_stream_page.v1`. One item is
one decision revision of one sanction stream. Platform sends only records the affected player
may see about their own account. Staff-only notations, staff notes, the deciding staff identity,
evidence, reports and other accounts are never exported.

| Field | JSON | Rule |
|---|---|---|
| `sanction_stream_id` | string | canonical lowercase UUIDv7, ADM-01 `sanction_stream_id`; one projection row per stream |
| `decision_revision` | integer | 1..=2^63-1, ADM-01 `decision_revision`; strictly increases per stream |
| `account_id` | string | canonical lowercase UUID of the Game account; unchanged per stream |
| `character_id` | string or null | a Character of that account; null for an account-wide record; unchanged per stream |
| `kind` | integer | 1 mute, 2 jail, 3 character_ban, 4 account_ban, 5 namelock, 6 notation; unchanged per stream |
| `reason_code` | integer | 1..=65535, Platform's player-facing reason category id |
| `reason_text` | string | 1..=120 UTF-8 bytes, no control characters; the player-facing reason text |
| `starts_at_ms` | integer | > 0, Unix milliseconds when the decision took effect |
| `ends_at_ms` | integer or null | > `starts_at_ms`; null means permanent (bans) or not applicable (namelock, notation) |
| `revoked` | boolean | true once Platform revoked or lifted the decision (ADM-01 unmute, release, revoke) |

Shape rules: kinds 1, 2, 3 and 5 name a Character; kind 4 names none; kind 6 may name one.
Kinds 1 and 2 need an end; kinds 5 and 6 have none; kinds 3 and 4 have an end or are permanent.
A Platform lift is a higher revision of the same stream with `revoked: true`, not a second
stream.

Projection (`game_account_sanction_project`, migration 2800): a new stream or a higher revision
applies; the same revision with identical content is a replay; a lower revision is stale and
keeps the stored row. The same revision with other content, or a revision that moves the stream
to another account, Character or kind, refuses the page.

## Loyalty stream

`POST /internal/v1/products-entitlements/loyalty-snapshots/stream`, request schema
`oteryn.loyalty_stream_request.v1`, page schema `oteryn.loyalty_stream_page.v1`. One item is one
account's loyalty snapshot revision.

| Field | JSON | Rule |
|---|---|---|
| `account_id` | string | canonical lowercase UUID of the Game account |
| `spent_premium_days` | integer | 0..=2147483647, the account-wide spent-Premium-day total |
| `loyalty_points` | integer | 0..=2147483647, Platform's loyalty points; Game ranks by these |
| `platform_revision` | integer | > 0, strictly increases per account |

Projection (`game_account_loyalty_project_points`, migration 2803): a new account or a higher
revision applies and may lower the points (refund, chargeback, correction); the same revision
with the same points is a replay; a lower revision is stale. The same revision with other points
refuses the page. Game stores the points in `game_account_loyalty_projections` and never derives
them from `spent_premium_days`.

## Game consumers

- Capability 497, command 6240 (`protocol-oteryn/v1/sanction-record-v1.md`): the admitted
  Character reads its own account-wide records and its own Character records, newest first,
  at most 32.
- Capability 498 (`protocol-oteryn/v1/highscores-loyalty-v1.md`): highscores category 13
  ranks the World's eligible Characters by their account's projected loyalty points.
  Category 13 is never republished through `OTERYN_GAME_PUBLIC_PROJECTIONS_V1`.
