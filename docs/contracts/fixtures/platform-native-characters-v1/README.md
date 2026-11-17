# Native characters v1

Byte-exact vectors for the native client's character list read. They describe
what portal-rs does at `main@f73244933228f399f1a5e9174ef064d7ffe70785`; a
change to any file here is a contract change.

- Route: `GET /api/v1/game-auth/native-characters`, no body, no query.
- Auth: `Authorization: Bearer <access token>` of the native OAuth client,
  scope `game:ticket`, same client and generation checks as ticket issuance;
  the read never consumes the token.
- Contract: `docs/contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md` §5.3
  (read), §5.2 (feed liveness), §10 (10 reads per minute per bearer, response
  at most 16384 bytes).
- Handler: `portal-rs/crates/player-api/src/lib.rs` (`characters`), read model
  `crates/characters/src/lcfa/store.rs` (`list_characters`); mounted by
  `bin/oteryn-portal/src/game_login.rs` (`player_router`).
- Flags: `PORTAL_GAME_TICKETS_ENABLED=true` with `OTERYN_ENV` `testing` or
  `preprod`; the projection keeps its own `OTERYN_LCFA_ENABLED`.
- Byte test: `portal-rs/crates/player-api/src/client_fixtures_tests.rs`;
  manifest digests in `crates/gameauth/src/oauth/oauth_fixture_tests.rs`.

## Cases

`manifest.json` lists the files with byte length and SHA-256 and one entry
per case (method, path, request headers, status, response headers, response
file; `null` is an empty body). Files have no trailing newline.

| case | status | body |
|---|---|---|
| characters-success | 200 | `characters-response.json` |
| characters-no-snapshot | 200 | `characters-empty.json` |
| characters-missing-bearer, characters-unknown-bearer | 401 | `characters-unauthenticated.json` |
| characters-rate-limited | 429 | empty, `Retry-After: 30` |
| characters-feed-stale | 503 | empty |
| characters-flag-off | 503 | empty |

Every response carries `Cache-Control: no-store, private` and
`Pragma: no-cache`; a 200 is `Content-Type: application/json`. Characters are
ordered by `character_id`. The handler answers no 403, 404 or 409. An invalid
account read model, a snapshot in an old epoch and a list over 64 entries are
also an empty 503.

## Synthetic values

Client `019a5f4a-1000-7000-8000-000000000001`, account
`019a5f4a-2000-7000-8000-000000000003` (`fixture-player@example.invalid`),
world `019a5f4a-3000-7000-8000-000000000004`, characters
`019a5f4a-4000-7000-8000-000000000005` (Aldric, AVAILABLE) and
`...0006` (Brenna, UNAVAILABLE), bearer
`QUJDREVGR0hJSktMTU5PUFFSU1RVVldYWVpbXF1eX2A` (a fixed synthetic value the
test pins to a real exchanged token). No response field is normalized.

The test seeds the LCFA feed and snapshot rows directly; ingestion has its own
tests in `crates/characters` and `bin/oteryn-portal/src/game_login_tests.rs`.
