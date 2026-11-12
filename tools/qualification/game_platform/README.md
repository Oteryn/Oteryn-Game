# Game ↔ portal-rs qualification

Run from the Game repository root with its normal Rust toolchain, Docker,
Compose, OpenSSL, Python 3, Node 22+, PostgreSQL client, GNU timeout/setsid and
sudo (including permission to create node_boot's disposable service user):

```sh
PLATFORM_SOURCE=/absolute/path/to/Oteryn-Platform bash tools/qualification/game_platform/run.sh
```

The checkout must be clean and exactly
`ed9b33729865a77051566182a17aaf8577abf347`. The readonly pin is independent of
node_boot's unchanged `3bc2d0e9` pin. There is no PHP or mock-server fallback.
No tool is installed by this lane. Platform's unchanged `e2e/start.sh` owns
preparation of its locked Node `pg` library when absent; provision that library
in advance if dependency installation is unavailable.

The fixed `NODE_BOOT_LIFECYCLE=game_platform` hook reuses node_boot's Game TLS
PostgreSQL 18.6, shipping binaries, migrations, operator initialization and real
World bundle. It replaces only Platform lifecycle/configuration and selects the
identity phase before assignment. The default lane retains its original stages,
pin, recursive basic pass and `NODE_BOOT_RESULT=NODE_BOOT_PASS` printer.

Platform runs in a **second PostgreSQL container**, using the same pinned 18.6
image with its own random administrator credential and role. Its original
`portal-rs/e2e/start.sh --seed-plan ...` creates/migrates the database, generates
the CA and sixteen single-SAN client identities, and runs serve/worker/gateway.
`report.py` consumes the stdout JSON object and validates listener origins,
certificate paths, roles' log artifacts, identities, seed outputs and pending
steps. Logs are never used for service discovery.

The upstream harness forwards only its `test.env` whitelist. `prepare.py`
generates a private **exec-only** launcher to add `OTERYN_ACCOUNT_LOGIN`,
`OTERYN_ACCOUNT_MFA` and `PORTAL_OAUTH_CONFIG`, then exec the real freshly built
`oteryn-portal` for every role/seed. It does not implement a handler or boot a
substitute server. Accounts, the World, OAuth client and signing trust use the
real testing-only `oteryn-portal seed` paths. Passwords and signing keys are
random per run. Fresh account evidence uses `platform_security` /
`fresh_admission` from the pinned harness settings.

`web_identity_pkce_isolation` is a real `oteryn-platform-client` integration
journey, running on a 64 MiB thread with a sixty-second deadline (also bounded
by GNU timeout after compilation). Its origin-scoped cookie jars drive real
form/CSRF, wrong-password refusal, missing/bad CSRF, login rotation, consent,
loopback state refusal, PKCE refusal, NativeLoginClient exchange, code replay,
account isolation and logout-cookie replay. The loopback listener is the actual
OAuth callback receiver, not a Platform substitute. Independent read-only psql
connections inspect persisted sessions/token ownership and Game admission
receipts. An authenticated bearer cookie remains its original account: posting
B's credentials cannot relabel A's session; logout destroys A's cookie while
B's independently authenticated jar remains B.

Only this script enables the external-topology test. Ordinary crate tests do
not emit identity evidence when `GAME_PLATFORM_INPUT` is absent. The required
success line is emitted only after every live assertion:

```text
NODE_BOOT_EVIDENCE flow=web_identity result=pass account_isolation=1
```

At this pin (#1833), the real harness seeds World `Entry`, the node_boot World/Channel
pair and a localhost route matching the Game TLS SAN and port. Channel accepts only
`worldId`/`channelId`; route uses `tlsServerName`, `version: 1`, and defaults
`nativeLoginEnabled` to true (`seed.rs:397`, `:406`, `:436`). Same-version changes
conflict (exit 2); identical input returns the same deterministic revision (`:454`).
The harness seeds the route twice; report validation requires equal records and
`rt.1.<32 lowercase hex>` revisions. `route_check.py` invokes the real seed binary
with the harness database environment and asserts changed-port conflict exit 2.
Only then is this line emitted:

```text
NODE_BOOT_EVIDENCE flow=route_seed result=pass revision_format=1 idempotent=1 conflict_exit=2
```

Identity runs with the real unassigned node. The next boundary is Game's missing
control-scope grant issuer: the default node_boot assignment stage inserts grants
with SQL (`tools/qualification/node_boot/run.sh:999`), which this lane does not copy.
It ends BLOCKED owner=Game with that source coordinate, rather than claiming assignment
or admission. Platform admission issuance now exists, but enabling its flag alone is
insufficient: `portal-rs/crates/gameauth/src/admission/grant.rs:24` requires encrypted
key custody and wrapping-key files, beyond the public `signing-trust` seed. This lane
does not manufacture that custody snapshot or claim a grant/admission PASS.
No route/channel pending mapping remains at this pin; any unexpected pending seed
fails validation. PASS remains reserved for a run with no pending work.

Generated inputs, credentials and the JSON report are removed on exit. On failure,
node-boot.log, node.log and portal-harness.log (when created) are retained in a
user-owned mode-0700 directory printed as `GAME_PLATFORM_DIAGNOSTICS=<dir>`.
The node state uses a root-owned disposable `/srv/oteryn-game-platform-<pid>`
base, created and removed by node_boot through sudo. Only evidence and a
closed-vocabulary terminal result are printed. The harness process group and
both disposable database containers are torn down, including on failure.
Ports can be changed with `GAME_PLATFORM_{GAME_PG,PLATFORM_PG,GAME,HTTP,INTERNAL,ADMISSION,GATEWAY,CALLBACK}_PORT`.

GPE2E-2 validation in the supplied workspace stopped at:

```text
GAME_PLATFORM_RESULT=FAIL flow=preflight reason=missing_binary_sudo
```

Consequently neither identity success nor the live T19c report has been
observed here. The default node_boot regression was attempted with its exact
P checkout; it failed when Docker Compose tried to create
`/home/agent/.docker/buildx` on a read-only filesystem, and its cleanup also
reported `sudo: command not found`. These are environment prerequisites, not
E2E passes. Tools were not reinstalled.

GPE2E-2b routes both callback states through the shipping `oteryn_identity::IdentityFlow`
and asserts the exact `StateMismatch` refusal. Operator setup runs through node_boot's
`stage` wrapper; a failed stage stops identity before node startup or success evidence.
The lifecycle sweep also removed the conditional call context from `await_log`.

Diagnostic audit: shell tracing stays disabled; the journey maps transport/database
errors to fixed reasons and never prints responses, cookies, codes or token values.
The pinned portal harness logs lifecycle progress and seed error JSON; seed database
errors are mapped to `unavailable/database`, and account errors contain fixed reasons,
not password values. Its report and generated secret inputs are not copied to diagnostics.
The local preflight diagnostic scan found no password/token/cookie/code/URL output;
no live node or portal logs were available in this workspace for a runtime audit.

GPE2E-3 adds two integration targets immediately after `web_identity`:

- `native_ticket_and_gateway_fail_closed` (60 s) exchanges real browser OAuth,
  persists a real native ticket, and calls the real Gateway with a deliberately
  absent Character (verified independently). The current topology has no LCFA
  owner snapshot: the shipping protocol parser must accept exactly HTTP 503 /
  `NATIVE_LOGIN_ROUTE_UNAVAILABLE`. Its identical-request retries must leave the
  ticket unused and `native_admissions` and Game admission receipts empty.
- `native_device_rotation_replay` (120 s) uses the direct TLS device listener,
  the existing harness CA/server identity, and Game's `NativeLoginClient`.
  The shipping transport preserves each actual request's exact HTTP status and
  closed-set refusal value. With explicit `Accept: application/json` and
  `Accept-Language: en`, only exact HTTP 400 /
  `{"status":400,"error":"Invalid request"}`, with unchanged family/credential state, identifies the
  pinned handler's rejection of the contract's original two-member enrollment.
  Other failures are assertions, not protocol blockers. A later successful
  enrollment proceeds into character/ticket rotation, real LCFA mTLS publication
  of an empty owner snapshot, predecessor replay, cross-account/client checks,
  revoke, a generation change, disabled-OAuth real-process refusal, and offline
  sign-out through shipping `DeviceLogin`/`DeviceAttempt`. No successful
  credential response is injected. The security mutation changes only the
  current native generation; no expiry rows are edited. On Linux the real OS
  vault reports unavailable; offline sign-out uses the shipping session class
  and process-memory secret lifecycle rather than substituting a vault.

The native TLS port defaults to 18100 (`GAME_PLATFORM_NATIVE_PORT`). The private
launcher supplies `APP_ENV=testing`, the explicit remembered-device flag and
projection authority; the existing real seed supplies the native OAuth client
allowlist. The disabled-OAuth and offline cases launch additional instances of
that same real binary with isolated public, native TLS and admission listeners,
require HTTP 200 readiness from all three, and stop/reap them on every exit.
OAuth exchanges reuse authenticated browser sessions (one password login per
account in the device journey); an unused disabled-OAuth token is reused for
session enrollment. No limiter window is slept out or limit increased.

At ed9b3372, Platform requires four-member enrollment (`devices_http.rs:224`) and
rotation proof (`:199`), contrary to the ACCEPTED Game contract lines 15–16,
65–66 and its HTTP request table. An observed enrollment gap emits both pending
coordinates; it does not claim to have reached rotation. If enrollment becomes
available while proofless rotation remains refused, only the rotation coordinate
is recorded. Both blocked paths compare all persisted family/credential fields
before/after the shipping request and run the same zero-admission observers.
Neither blocked path prints the device success evidence.

Result precedence is FAIL > BLOCKED > PASS: a later assertion, timeout or node
exit overwrites a recorded blocker with one FAIL line. The final Game assignment
blocker does not overwrite an earlier Platform blocker. Both existing downstream
blocker lines are still emitted, custody before assignment. All successful live
assertions are required before these success lines can appear:

```text
NODE_BOOT_EVIDENCE flow=native_ticket_gateway result=fail_closed game_admissions=0
NODE_BOOT_EVIDENCE flow=native_device_rotation result=pass rotations=4 replay_refused=1 signout_offline=1
```

Focused observer/protocol tests (no external topology, no E2E success lines):

```sh
cargo test -p oteryn-platform-client --test native_ticket_and_gateway_fail_closed
cargo test -p oteryn-platform-client --test native_device_rotation_replay
```


The PC run on a172ee373 lost its assertion reason because cleanup retained no
journey logs. The lane now redacts and retains `identity-build.log`, `identity.log`,
`native_ticket_and_gateway_fail_closed.log` and `native_device_rotation_replay.log`
before an explicit failure and again during cleanup (including unexpected exit
and timeout). Retained files are mode 0600 in the private diagnostics directory.
Redaction retains only structured Rust test summaries and source-defined fixed
failure reasons; every other line becomes `[REDACTED]`. This suppresses arbitrary
opaque secrets as well as bearer/device headers, credentials, OAuth codes/tokens,
cookies, passwords and database URLs, including multiline values. Test drivers
still must never print secrets in their original private logs.

Root-cause trace at unchanged Platform ed9b3372 (M paths below):

| Hypothesis | Source evidence and outcome |
|---|---|
| Disabled/misconfigured device route | `devices_http.rs:69` reads exact `true` and `:70` reads APP_ENV; Game's launcher supplies both (`prepare.py`). `devices.rs:128` accepts `testing`. Gate configuration is correct. |
| Wrong listener or missing TLS | `web/src/lib.rs:495` mounts devices; `main.rs:441`–`:442` passes that public router to native TLS. `devices_tls.rs:20`–`:22` read the same bind/certificate/key names set by the launcher, and `:86`–`:87` insert verified HTTPS/socket evidence. The configured trusted certificate contains the loopback IP. |
| OAuth/token or native-client refusal before body validation | `devices_http.rs:169`–`:172` decodes enrollment before calling `enroll_class`; `devices.rs:252`–`:257` then checks OAuth deployment and token authority. The two-member body cannot reach those checks. The real native-client seed and OAuth allowlist remain configured. |
| Four-member body rejection | `devices_http.rs:220` first deserializes mandatory key/proof members (`devices_wire.rs:6`–`:11`), so it returns Invalid before the explicit length check at `:224`. `devices_http.rs:23` maps that to HTTP 400 / `invalid_request`. |
| Outer response rendering — most likely PC failure | `web/src/lib.rs:65`–`:67` wraps devices in `surface::finish` and API error policy. `surface.rs:206`–`:221` requires an explicit JSON Accept header; `:244`–`:268` replaces 400 bodies, even ones already JSON. The previous shipping request had no explicit Accept, so the final response was HTML and could not satisfy Game's JSON refusal decoder; the journey maps transport/payload failure to `enrollment_transport_or_payload_failed`. The missing PC log prevents proving which assertion actually fired. |

The repair negotiates JSON/English in the real transport and accepts the exact
rendered 400 envelope above (`surface.rs:251`, `ui/i18n/en.ftl:9`). It does not
accept another enrollment status. The same renderer changes the malformed
account-override response; disabled OAuth's exact 503 envelope is
`{"status":503,"error":"Temporarily unavailable"}` (`ratelimit.rs:356`–`:389`,
`:284`, `ui/i18n/en.ftl:142`). HTTP 401 is not in the rendering list, so replay,
revocation, wrong-client and generation expectations stay unchanged.

The pinned-envelope decoder regression failed before repair with
`Error: InvalidPayload`; diagnostics retention also failed before repair for
all three journey log names. Both regressions pass after repair. This is focused
red/green evidence, not a substitute for rerunning the PC topology.
