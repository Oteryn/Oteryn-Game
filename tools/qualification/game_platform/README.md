# Game ↔ portal-rs qualification

Run from the Game repository root:

```sh
PLATFORM_SOURCE=/absolute/path/to/Oteryn-Platform bash tools/qualification/game_platform/run.sh
```

The clean Platform checkout must be exactly
`a85b5da4de23f3e8633cfb25a2db1ad7f6133acd`. No main merge, PHP fallback,
mock handler or tool installation is performed. Required tools remain Docker,
Compose, OpenSSL, Python 3, Node 22+, psql, Cargo, timeout, setsid and sudo.
The upstream harness requires its prepared Node pg dependency.

The fixed node_boot lifecycle hook reuses real Game binaries, migrations,
operator initialization, TLS PostgreSQL 18.6 and the real World bundle. Platform
uses a second disposable PostgreSQL 18.6 container and its unchanged
portal-rs/e2e/start.sh lifecycle (migrate/serve/worker/gateway), real account,
OAuth client, route/channel and trust seeds. Native TLS defaults to port 18100
(GAME_PLATFORM_NATIVE_PORT), using the harness CA/server certificate.

The private exec launcher supplies the explicit remembered-device flag,
APP_ENV=testing, enabled OAuth allowlist and projection authority. Before serve,
after real migrations, it invokes the real `seed signing-custody` CLI. Platform
seed.rs:629-687 provisions an encrypted Active signing key/trust publication and
a per-run 32-byte owner-only wrapping key. Files are in WORK/portal/secrets/admission-custody;
the launcher verifies wrapping-key size 32 and mode 0600, exports
PORTAL_NATIVE_ADMISSION_KEY_CUSTODY_FILE and
PORTAL_NATIVE_ADMISSION_KEY_WRAPPING_KEY_FILE, and enables admission grants.
Only serve loads the issuer (main.rs:109); worker/gateway do not read custody.
Private seed output stays in WORK/custody-result.json and is never printed or
copied into diagnostics. Existing WORK cleanup removes both files and output.

Locator decision A: both GAME_AUTH_NATIVE_RECOVERY_LOCATOR_IDENTITIES and
PORTAL_NATIVE_RECOVERY_LOCATOR_IDENTITIES remain unset. At this pin,
gameauth/src/runtime.rs:725-748 enables the locator only with protocol-3 AND
recovery flags, and reads each mapping only when present. Neither flag is enabled
by this lane or upstream test.env. Missing mappings do not refuse boot;
locator.rs:337-341 mounts a disabled protected route. None of browser identity,
OAuth ticket/Gateway refusal or device enrollment/rotation/revoke calls the
recovery-locator route. No locator identity, certificate or node mapping is
invented. The optional formats remain {"SAN":["world/channel"]} and
{"SAN":"node_identity"}, defined in gameauth/src/locator.rs:29-33 and documented
in portal-rs/README.md:32-33.

The lane runs, in order, web_identity_pkce_isolation (60s),
native_ticket_and_gateway_fail_closed (60s), native_device_rotation_replay (120s).
At this pin the two-member remembered enrollment and proofless rotation must
succeed: every refusal is FAIL, never an old Platform BLOCKED classification.
The test-only TLS relay proves the actual method/path/content-type/encoding,
authorization and byte-exact fixture body, then forwards to real verified TLS
Platform. Every response is from Platform. Rotation authorization is compared
with the real issued credential. Independent psql observers require zero Game
admission receipts and zero successful Platform native_admissions rows.

Custody does not manufacture a Game Character projection or control grant. Before
the device phase publishes an empty LCFA snapshot, Gateway therefore still must
return the exact 503 NATIVE_LOGIN_ROUTE_UNAVAILABLE and roll back ticket use.
The device phase asserts four rotations, account/ticket ownership, exact 401
replay/revoke/generation refusals, disabled OAuth and offline local sign-out via
shipping DeviceLogin/DeviceAttempt. U23 restart recovery remains excluded.

Accept JSON/English remains explicit. Native routes are still mounted inside
web/src/lib.rs:495-500 security_headers; surface.rs:244-268 renders 400 envelopes
and ratelimit.rs:356-389 renders API 503 envelopes. Thus DeviceRefusal retains the
exact site-envelope parser (400 Invalid request; 503 Temporarily unavailable).
These envelopes never grant device BLOCKED. PHP P already returns JsonResponse.

Expected evidence:

```text
NODE_BOOT_EVIDENCE flow=web_identity result=pass account_isolation=1
NODE_BOOT_EVIDENCE flow=native_ticket_gateway result=fail_closed game_admissions=0
NODE_BOOT_EVIDENCE flow=native_device_rotation result=pass rotations=4 replay_refused=1 signout_offline=1
GAME_PLATFORM_EVIDENCE blocker=tools/qualification/node_boot/run.sh:999 pending=control_scope_grant_issuer
GAME_PLATFORM_RESULT=BLOCKED owner=Game what=tools/qualification/node_boot/run.sh:999
```

FAIL overrides BLOCKED; one terminal result is printed. No old custody/device
blocker is emitted. Failed journey logs are retained after substring redaction,
including panic/assertion/compiler output; explicit evidence digests are exempt,
never arbitrary log values. Diagnostics are private; generated credentials,
custody and databases are removed on exit. Local full-lane preflight cannot run
without sudo; focused tests emit no live success evidence without topology.
