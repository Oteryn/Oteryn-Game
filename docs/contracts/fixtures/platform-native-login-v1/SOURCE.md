SOURCE: Oteryn/Oteryn-Game@857f15f376907c592bb28d854fccff18a15cdf87
Paths: crates/platform-client/src/native_login/tests.rs and crates/platform-client/src/native_login.rs.
Vectors relayed verbatim by the programme coordinator; no Game repository access.
JSON body strings preserve literal bytes. Sorted request and rejected mutations are derived from those bytes.

Constants: NATIVE_LOGIN_SCOPE="game:ticket"; GATEWAY_PROTOCOL_VERSION=2;
GAME_ALPN="oteryn-game/1"; GRANT_PROFILE="oteryn-pre-admission-v1";
MAX_LOGIN_REQUEST_BYTES=2048; MAX_LOGIN_RESPONSE_BYTES=6144;
MAX_LOGIN_REQUESTS=6 per (attempt_ref,ticket), including reconciliation re-login;
MAX_TICKET_LIFETIME=60s; client honours Retry-After up to 30s.
Public classes: RETRY_LOGIN, AUTHENTICATION_REQUIRED, TEMPORARILY_UNAVAILABLE,
SESSION_UNAVAILABLE, CLIENT_UPDATE_REQUIRED, CHARACTER_ALREADY_ACTIVE.

Request: POST /v1/login; retries send identical bytes. channel_id is null or UUIDv7;
client_build is visible ASCII, at most 64 bytes. Responses deny unknown fields and
require every field, including nullable attempt_ref on errors. Security terminal
codes are public only as NATIVE_LOGIN_AUTHENTICATION_REQUIRED.

Sequences A-C describe client behaviour, not gateway retry orchestration. Tests
send A's three requests, B's first six of eight responses, and C's single request.
They do not implement or test the client's stopping logic.
