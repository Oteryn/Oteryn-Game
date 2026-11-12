# Game ↔ portal-rs qualification

Run from the Game repository root with its normal Rust toolchain, Docker,
Compose, OpenSSL, Python 3, Node 22+, PostgreSQL client, GNU timeout/setsid and
sudo (including permission to create node_boot's disposable service user):

```sh
PLATFORM_SOURCE=/absolute/path/to/Oteryn-Platform bash tools/qualification/game_platform/run.sh
```

The checkout must be clean and exactly
`f73244933228f399f1a5e9174ef064d7ffe70785`. The readonly pin is independent of
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

At this Platform pin, channel and route seed explicitly return pending
(`portal-rs/bin/oteryn-portal/src/seed.rs:148`). After identity qualification the
lane therefore exits nonzero with:

```text
GAME_PLATFORM_EVIDENCE blocker=portal-rs/bin/oteryn-portal/src/seed.rs:148 pending=channel,route
GAME_PLATFORM_RESULT=BLOCKED owner=Platform what=T19c
```

No assignment, Character bootstrap, Game admission or downstream seam success
is claimed while T19c is absent. The original seam functions stay reusable in
node_boot; enabling downstream stages requires a separately reviewed Platform
pin with genuine route/channel provisioning. There are no manually seeded
routes, grants, sessions or projections. A missing login/OAuth handler produces
an exact Platform source blocker. Other assertions produce FAIL and nonzero
exit. PASS is reserved for a complete run with no pending Platform steps.

All generated inputs, credentials, the JSON report and raw diagnostics remain
in mode-0700 scratch directories and are removed on exit. Only evidence and a
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
