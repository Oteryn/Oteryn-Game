# Native OAuth token and ticket v1

Byte-exact vectors for the two Platform calls the native Rust client makes
before it talks to the login Gateway: the OAuth code exchange (and refresh)
and native Game Login Ticket issuance. They describe what portal-rs does at
`main@f73244933228f399f1a5e9174ef064d7ffe70785`; a change to any file here is
a contract change.

| | `POST /oauth/token` | `POST /api/v1/game-auth/tickets` |
|---|---|---|
| Auth | none (public PKCE client, `application/x-www-form-urlencoded`) | `Authorization: Bearer <access token>`, scope `game:ticket` |
| Contract | `docs/contracts/OTCLIENT_GAME_AUTH_CONTRACT.md` *Step 5 - authorization code exchange* | `OTCLIENT_GAME_AUTH_CONTRACT.md` *Game Login Ticket issuance*; `OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md` §2 step 3, §4.1, §10 |
| Handler | `portal-rs/crates/gameauth/src/oauth/oauth_http.rs` (`token`), `crates/gameauth/src/oauth.rs` (`exchange`) | `portal-rs/crates/player-api/src/lib.rs` (`issue`), `crates/gameauth/src/tickets/mod.rs` |
| Mounted by | `crates/web/src/lib.rs` (`token_routes`, behind the OAuth rate-limit and audit layer) | `bin/oteryn-portal/src/game_login.rs` (`player_router`) |
| Flag | `PORTAL_OAUTH_CONFIG` (`enabled`, client allowlist; missing or malformed denies) | `PORTAL_GAME_TICKETS_ENABLED=true` and `OTERYN_ENV` `testing` or `preprod` |
| Byte test | `crates/gameauth/src/oauth/oauth_fixture_tests.rs` | `crates/player-api/src/client_fixtures_tests.rs` |

## Files

`manifest.json` lists every vector file with its byte length and SHA-256, the
synthetic values and one entry per case: method, path, request headers that
matter, request file, status, response headers that matter, response file
(`null` means an empty body) and the fields normalized before comparison.
Vector files are exact bytes with no trailing newline.

- `token-request-*.form` - token request bodies.
- `token-response.json` - the success body for both grants.
- `token-invalid-request.json`, `token-invalid-grant.json`,
  `token-unsupported-grant.json`, `token-invalid-client.json` - the RFC 6749
  error bodies.
- `ticket-request.json`, `ticket-request-unsupported.json` - ticket bodies.
- `ticket-response.json`, `ticket-unauthenticated.json`,
  `ticket-unavailable.json`, `ticket-invalid-request.json` - ticket responses.

## Cases

| case | status | body |
|---|---|---|
| token-code-success | 200 | `token-response.json` |
| token-refresh-success | 200 | `token-response.json` |
| token-code-replay | 400 | `token-invalid-grant.json` |
| token-unknown-field | 400 | `token-invalid-request.json` |
| token-unsupported-grant | 400 | `token-unsupported-grant.json` |
| token-unknown-client | 401 | `token-invalid-client.json` |
| ticket-success | 200 | `ticket-response.json` |
| ticket-missing-bearer, ticket-unknown-bearer, ticket-replay | 401 | `ticket-unauthenticated.json` |
| ticket-flag-off | 401 | `ticket-unavailable.json` |
| ticket-unsupported-version | 422 | `ticket-invalid-request.json` |
| ticket-rate-limited | 429 | empty, `Retry-After: 30` |
| ticket-database-unavailable | 503 | empty |

Token responses carry `Cache-Control: no-store` and `Pragma: no-cache`; ticket
responses carry `Cache-Control: no-store, private` and `Pragma: no-cache`.
The ticket handler answers no 403, 404 or 409. The token handler answers
no 403 or 409; its only 404 is the `invalid_request` body for OAuth disabled
at the handler, which the public router normally answers first (below).

## Synthetic values and normalization

- Client `019a5f4a-1000-7000-8000-000000000001`, redirect
  `http://127.0.0.1:41000/callback`, account
  `019a5f4a-2000-7000-8000-000000000003` (`fixture-player@example.invalid`).
- PKCE verifier and S256 challenge are the public RFC 7636 Appendix B vector.
- The authorization code, refresh token and bearer are fixed synthetic 32-byte
  base64url values. The test obtains real ones through the real code exchange
  and then pins their stored SHA-256 to these values, so request bytes are
  fixed.
- `access_token`, `refresh_token` and `ticket` in responses are fresh CSPRNG
  values. The test checks each is a canonical 32-byte unpadded base64url value
  and replaces it with the placeholder in `manifest.json` (`A` x 43, `_` x 42
  then `8`, `A` x 43) before comparing all bytes. Nothing else is normalized.

## Outside these vectors

- The public router answers `/oauth/token` before the handler when OAuth is
  disabled (empty 404) and when the source limit `identity-login-source` is
  spent (429 with `Retry-After` and the shared rate-limit JSON body); a 5xx
  whose audit write also fails becomes an empty 503. These come from
  `crates/web`, not from the handler, and are not byte-tested here.
- In the binary the ticket verifier is wrapped by the sanctions guard
  (`GuardedTickets`); the test uses the unwrapped `PlatformAuthority`, which
  gives the same bytes for an active account.
- Remembered-device ticket issuance
  (`/api/v1/game-auth/device-sessions/tickets`) is covered by
  `../remembered-device-v1/`.
