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
