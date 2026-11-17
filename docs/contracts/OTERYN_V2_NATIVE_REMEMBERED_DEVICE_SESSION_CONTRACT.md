# Native device session enrollment and rotation, version 1

Status: **ACCEPTED — owner decision 2026-11-09 (Oteryn-Game #3058, option B).**
Platform owns the missing contracts and Rust implementation. This accepts the candidate's
protocol 1 and DB-1 session classes. It grants no production rollout authority. The
candidate remains historical evidence; this contract governs enrollment and rotation.
The gateway login contract §19, including its recovery/device-binding release gates,
continues to govern recovery. U23 remains open: a persistent family cannot recover an
existing actor after restart without a live in-memory recovery handle. No new callback
purpose is introduced.

## Compatibility and activation

Protocol 1 retains every existing member, code, digest, alphabet and size limit. Existing
PHP clients can send the original two-member remembered enrollment request unchanged.
Device-key enrollment is an additive four-member request variant, also protocol 1.
Old clients need not implement it to use remembered rotation; an unbound family cannot
create a device-bound recovery handle. No fallback from required binding is allowed.

Rust routes activate only with `OTERYN_NATIVE_REMEMBERED_DEVICES_ENABLED=true`,
`APP_ENV=testing|preproduction`, valid enabled `PORTAL_OAUTH_CONFIG`, and a matching
explicitly enabled `oauth_native_clients` row. The native row must assert all of
`first_party`, `public_client`, and `native_ticket`. Missing, duplicate or invalid OAuth
configuration refuses. No client is automatically opted in by migration. The older
`GAME_AUTH_DEVICE_SESSIONS_ENABLED` key does not activate the Rust authority. Production
refuses even with the switch on. Disabling the switch returns the original 503 refusal.

First sign-in remains external-browser Authorization Code + PKCE S256 with `game:ticket`.
Passwords, MFA and account recovery never move into a native credential form. Native
refresh tokens must not be persisted or used. Enrollment precedes normal ticket issuance;
it does not consume or replace that normal OAuth bearer. A fresh browser sign-in is the
fallback after an uncertain enrollment or rotation response.

## Classes, proof and one-family enrollment (DB-1)

One access token enrolls at most one family across **both** classes. A unique
`enrollment_token_id` and account/client/token locks enforce this, including concurrent
requests. A repeat cannot recover plaintext or upgrade its class. Remembered replaces
session on that login; there is no two-family flow.

Every protocol 3 login enrolls session unless remembered is chosen on that login.

- `session`: no consent required, process memory only for bearer and key. Absolute and
  idle deadlines both equal enrollment time + H, with H = 43200 seconds (12 hours).
  Rotation never extends either deadline. Session enrollment requires a device key.
- `remembered`: explicit opt-in (`remember_device:true`); at most 30 days absolute and
  7 days idle, configurable only downward through the service settings. Defaults are
  those maxima. Each successful rotation sets idle to min(now + idle TTL, absolute).
  The bearer lives in the OS credential vault, keyed by trusted origin and client ID.
  Under owner decision Q2 (2026-10-16), the bound remembered key requires an OS vault or
  another non-exportable store; without one only the in-memory session class is available.
  The server cannot attest local vault behavior; the client must enforce this choice.

A fresh Ed25519 key belongs to exactly one bound family. Store its 32-byte public key
and unique thumbprint = base64url-no-pad(SHA-256(public key)), 43 ASCII characters.
Enrollment verifies strict Ed25519 possession inside the transaction. The signature
covers the UTF-8 JCS object from gateway contract §19.15.2, in this lexical member order:

```json
{"access_token_sha256":"<lowercase hex SHA-256 of the exact bearer>","client_id":"<canonical client UUID>","persistence":"session|remembered","public_key":"<base64url public key>","purpose":"oteryn-native-device-enroll-v1"}
```

The public key and signature use canonical unpadded base64url (32 and 64 decoded bytes).
Changing any bound member invalidates the proof. Key reuse across families refuses.
Original protocol-1 remembered requests remain unbound; they carry no invented proof
and cannot satisfy recovery's device-binding requirement.

Under the same connection and transaction, lock/revalidate the authoritative account,
first-party public native client policy, OAuth family/token, and original game/native
generations. Enrollment uses the normal OAuth opaque-token grammar, credential stamp,
allowlist, expiry and revocation checks; the current OAuth implementation grants only
`game:ticket`. Reject unusable/unverified accounts and unavailable security facts. Store
immutable original generations. OAuth families also snapshot these generations at exchange;
grants predating that snapshot cannot enroll. Never adopt a newer generation to keep a family alive.

## HTTP protocol 1

All routes are POST under `/api/v1/game-auth/device-sessions`, require verified TLS
transport evidence, and use `Content-Type: application/json`. Forwarding headers do
not establish TLS or source identity. There is no Rust plain-HTTP loopback exception.
Authorization uses exactly one header, the exact scheme and one ASCII space.

| Suffix | Authorization | Exact body |
|---|---|---|
| empty (original remembered) | `Bearer <OAuth access token>` | `{"protocol_version":1,"remember_device":true}` |
| empty (bound extension) | `Bearer <OAuth access token>` | `{"protocol_version":1,"remember_device":false,"device_public_key":"<43 chars>","enrollment_proof":"<86 chars>"}` |
| `/native-characters` | `OterynDevice <credential>` | `{"protocol_version":1,"client_id":"<canonical client UUID>"}` |
| `/tickets` | `OterynDevice <credential>` | same as characters |
| `/revoke` | `OterynDevice <credential>` | same as characters |

For bound remembered enrollment the four-member body uses `remember_device:true`.
No `persistence`, owner, account, generation, scope or purpose request member is accepted.
False without a proof is invalid. Bodies are at most 1024 bytes, flat objects: reject
unknown/duplicate members (including escaped names), wrong types, query strings,
compressed/transfer-encoded bodies and inconsistent lengths. Preserve the candidate's
canonical lowercase client UUID rules and all existing protocol-1 limits.

Enrollment succeeds with HTTP 200 and these existing members, with no new response fields:

```json
{"protocol_version":1,"device_credential":"otd1.<43 base64url chars>","family_id":"<UUID>","absolute_expires_at":1794132000,"idle_expires_at":1792144800}
```

Rotation returns those members with the successor, same family and original absolute
deadline. Characters adds `characters` (at most 64) with `character_id`, `world_id`,
`name` (1..96 UTF-8 bytes) and `availability` (`AVAILABLE|UNAVAILABLE`). Rust reads the
existing owner-scoped projection under its shared feed lock, requires a live highest
epoch, matching source authority and valid account snapshot, and refuses stale/missing
state. `OTERYN_NATIVE_DEVICE_PROJECTION_AUTHORITY` selects the trusted source; missing
configuration refuses reads. The default projection bounds are 30 seconds freshness,
1 second clock uncertainty. The projection is evidence, never admission authority.

Tickets adds `ticket`, integer `expires_in` (1..60), integer UTC `expires_at` (no more
than now+60). Rotation uses the existing single-use Game Login Ticket issuer on the
same connection, not another token format or issuer. Tickets record nullable
`device_family_id`; the normal OAuth issue path also links the family enrolled by that
same access token. Ticket audience and redemption semantics remain unchanged.
No OAuth token is minted by a rotation. Responses are at most 16384 bytes.

Revoke returns exactly `{"protocol_version":1,"revoked":true}` (200). All expiry times
are integer UTC Unix seconds; equality is expired. Responses and failures carry
`Cache-Control: no-store, private` and `Pragma: no-cache`.

| HTTP | Exact error value | Meaning |
|---|---|---|
| 400 | `invalid_request` | malformed shape, type, proof encoding or query |
| 413 | `invalid_request` | declared/actual body exceeds bounds |
| 401 | `device_authorization_unavailable` | credential, proof, account, client, replay, expiry or generation refusal |
| 429 | `too_many_requests` | source throttle; `Retry-After: 60` |
| 503 | `device_authorization_unavailable` | disabled, production, transport, configuration or dependency unavailable |

Errors contain only `{"error":"<value>"}`. No input or exception is echoed. A durable
source-IP-hash throttle covers all four operations independently of credential rotation:
default 30 requests/minute, settings bounded to 1..60. Socket source is required; caller
forwarding headers are ignored. Native client policy is checked again at each use.

## Rotation, revocation and retained state

Credential = `otd1.` + canonical unpadded base64url of 32 CSPRNG bytes, 48 ASCII bytes.
Only SHA-256 digests persist, including every consumed predecessor. Compare secret-derived
digests in constant time in application verification. Never log tokens, hashes, keys,
signatures, request Authorization, or successful credential/ticket response bodies.
Debug output is redacted. Do not put secrets in URLs, cookies, diagnostics or settings.

Lock order is account, native client, family, credential; enrollment additionally locks
the OAuth family/token. Locator reads confer no authority. All checks and native effects
are on the same connection. Rotation consumes exactly one predecessor, inserts exactly
one random successor and runs only `NativeCharactersRead` or `NativeTicketIssue` before
commit. Failure rolls back both rotation and native effects. Only confirmed deadlock or
serialization aborts may retry internally (at most three); ambiguous commits never retry.

A known consumed predecessor revokes the **whole family**, including the newest successor.
Commit terminal denial before returning 401. Concurrent use permits at most one success;
the second use revokes the successor. Unknown/malformed or wrong-client credentials cannot
revoke someone else's family. The client serializes operations across processes, stores
the successor before using its result, and clears the credential on a failed/uncertain
rotation; no grace window, automatic predecessor retry or plaintext recovery exists.

Password change, verified email/security change, MFA confirmation/replacement/removal,
account suspension and explicit owner revocation invalidate old families. PostgreSQL
account/MFA hooks advance generations and durably revoke families in the same transaction.
A transaction stamp preserves an advance already made by an existing Identity writer,
so its subsequent password/MFA cleanup does not add another automatic native fence.
Routine TOTP step changes do not fence a family. The existing native-evidence retained
witness continues to require explicit acknowledgement by its owning security writer:
a generation advanced in PostgreSQL but not yet acknowledged by that witness makes
evidence unavailable. These routes never auto-advance a retained witness or perform
filesystem effects inside a device transaction. Writer/witness operational qualification
remains an enablement gate. Native sign-out/cancellation revokes the
family idempotently, including with its used predecessor; clear the local vault even if
server revocation is unreachable. A normal browser logout is not silently redefined as
native sign-out. Owner revocation is scoped to an authenticated active owner; a future
browser adapter must enforce CSRF and recent authentication.

Absolute/idle expiry and changed generations refuse every next use. Revoked families
never revive on account reactivation. Recovery must read the current family state on each
use, so expired/revoked families cannot recover even if a cleanup write has not occurred.
DB-5/DB-6 recovery handle issuance/linkage and DB-9 review remain separate recovery release
gates: this task adds no recovery callback or restart recovery authority. Existing admitted
game sessions and already-issued tickets retain their own security-generation checks.
No destructive rollback, history purge or production-data reset is provided.

## Fixtures and qualification

`fixtures/remembered-device-v1/manifest.json` pins every included vector's exact byte count
and lowercase SHA-256. JSON files have no trailing newline. Vectors are synthetic public
test data, never live credentials. Tests verify those bytes and exercise original and
bound request parsing, enrollment, native ticket persistence, replay, security revocation,
expiry, transactional rollback, throttling and disabled refusal on PostgreSQL 18.6.

Source tests do not qualify OS-vault behavior, TLS deployment/redaction infrastructure,
client restart/cross-process storage, Game-world E2E or independent security review.
Those remain enablement evidence, especially gateway contract §19.15 DB-9. Acceptance of
this protocol does not assert production readiness or resolve U23.
