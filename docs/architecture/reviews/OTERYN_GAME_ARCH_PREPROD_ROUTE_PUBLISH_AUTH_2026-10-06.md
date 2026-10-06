# PREPROD-ROUTE-PUBLISH-AUTH-1 Authority request for the preproduction route-publish operator path

- Decision: `ARCH-PREPROD-ROUTE-PUBLISH-AUTH-V1`
- Status: **CANDIDATE: AUTHORITY REQUEST**. Nothing here takes effect until the owner answers
  §6 item by item. This document grants no authority.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: the control plane, D821 item 2 (#162, 2026-10-06; owner answer **2a**). The request
  covers the route-publish operator path that the joint native-login E2E needs. That E2E is
  Platform native gateway login contract §14 step 6 (#1419 item 5).
- Evidence read: Oteryn/Oteryn-Platform `origin/main` 3896bcd, read only.
- Runtime, migration, deployment, production, protected-World and Platform write authority: NONE
  in this PR.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 0. Brief

The joint E2E needs four things in Platform:

1. a Registry WorldId and ChannelId;
2. a published route for that Channel;
3. `native_login_enabled=true` for that Channel;
4. the admission issuer public key in the native signing trust registry.

Item 1 has an operator command. Items 2–4 exist only as PHP methods that the tests call.

The owner said "preproduction". Platform has **no preproduction deployment**. Its only deployed
environment is the public Synology staging stack, and native login code refuses to run there by
design.

So the request is a choice:

- **Option A (recommended).** Run step 6 on a disposable stack (RUNBOOK-1). This needs one
  small Platform PR that adds two testing/preproduction-only artisan commands. Nothing persistent
  is mutated.
- **Option B.** Stand up a persistent private preproduction environment. This needs deployment,
  database, secret and Platform code authority. Ask for it only when step 7 needs a persistent
  environment.
- **Option C (rejected).** Enable native login on public staging.

## 1. Findings

**F1. No Platform preproduction environment exists.**

- `deploy/synology/` is the only deployment.
- `deploy/synology/.env.example` sets `APP_ENV=staging`.
- It is public through a Cloudflare Tunnel: `https://oteryn.molehill.cloud` maps to Platform on
  127.0.0.1:8000, and `https://gateway.molehill.cloud` maps to the Gateway on 8080.
- It shares one MariaDB with Canary.
- The `Deploy Synology Staging` workflow is manual and main-only, and runs on the `oteryn-staging`
  runner.
- In `APP_ENV=staging`, three things refuse by design:
  - the native issuer;
  - mode 33a (`config/game-auth.php`: "Refused (NATIVE_LOGIN_UNAVAILABLE) in any other
    environment");
  - every route-record method, through `NativeTopologyRegistry::isolatedConnection()`.

**F2. The route-publish write exists only as a method.**

`App\GameAuth\Worlds\NativeTopologyRegistry::publishRouteForPreproduction(worldRowId, channelKey,
host, port, tlsServerName, loginEnabled)` does the write. In one transaction it:

- locks `game_worlds` (by `id`) FOR UPDATE;
- locks `game_channels` (by `game_world_id`, `channel_key`) FOR UPDATE;
- requires an already issued `world_id` and `channel_id`;
- updates `native_route_host`, `native_route_port`, `native_route_tls_server_name`,
  `native_route_version`, `native_route_revision`, `native_login_enabled` and `updated_at`.

An endpoint change advances the version, which invalidates every outstanding grant for the old
revision. After the commit it logs "Disposable native route record published." with `world_id`,
`channel_id`, `route_revision` and `native_login_enabled`.

There is no artisan command, route or job for it. Only the tests call it. The N4P-3 task record
lists "an operator path for `publishRouteForPreproduction` outside the isolated connection" as
out of scope, under separate authority.

**F3. The connection guard admits only disposable stores.**

`isolatedConnection()` uses the application's default connection and refuses in these cases:

- the environment is not `testing` or `preproduction`;
- an outer transaction is open;
- the store is MySQL/MariaDB, unless all of these hold: database `oteryn_concurrency`, a loopback
  host, no unix socket, and `APP_ENV=testing`;
- the store is SQLite, unless it is `:memory:` in testing, or a regular retained file at
  `<sys_get_temp_dir>/oteryn-native-topology-<hex>/oteryn-native-topology.sqlite` with no
  symlink.

A preproduction Platform can therefore publish routes **only** when its whole default database is
that retained temporary SQLite file. That is the RUNBOOK-1 profile. A persistent MariaDB is
refused.

**F4. The trust key has no operator path either.**

The issuer public key must be in `native_game_signing_trust_profiles` (via
`NativeSigningTrustRegistry::publishTrustedKey`) before the Gateway accepts a grant. That method
has no command and no environment gate. It needs `GAME_AUTH_NATIVE_EVIDENCE_HIGH_WATER_DIRECTORY`.
The joint E2E needs it as much as the route, so this request covers both.

**F5. Issued identities are permanent.**

The WorldId and ChannelId from `game-auth:native-topology:issue` are immutable. The migration
refuses to roll back while issued records exist. On any persistent store, an issuance is a
permanent change to the Registry.

**F6. RUNBOOK-1 already assumes these entrypoints.**

RUNBOOK-1 (`ARCH-LOGIN-FIRST-PACKETS-V1` §2.7, `tools/qualification/login_local/`, not yet
implemented) plans:

- Platform with `APP_ENV=preproduction`;
- a route published with `publishRouteForPreproduction` and `native_login_enabled=true`;
- the issuer key published with `publishTrustedKey`.

It cannot do that from a shell without the commands from Option A.

## 2. Option A (recommended): the joint E2E on a disposable stack

**What is mutated.** All mutations happen in the per-run SQLite file
`<tmp>/oteryn-native-topology-<hex>/oteryn-native-topology.sqlite` of one disposable Platform
process (`APP_ENV=preproduction`, or `testing` in CI). The changed rows are:

- one `game_worlds` row from `game-auth:world:ensure` (login disabled);
- its `world_id` and one `game_channels` row from `game-auth:native-topology:issue`;
- the route columns and `native_login_enabled` from the new route command;
- one `native_game_signing_trust_profiles` row from the new trust-key command, plus the high-water
  floor file in the per-run `GAME_AUTH_NATIVE_EVIDENCE_HIGH_WATER_DIRECTORY`.

Nothing outside the run's temporary directories changes. No staging, production or shared
database is touched.

**Where.** A developer machine, or a CI job on a GitHub-hosted runner. Never on the Synology host.

**By whom.** One of these:

- the RUNBOOK-1 operator, who is the person running `tools/qualification/login_local/run.sh`;
- the Game node-boot CI job that wraps it.

Each run has one writer. The commands run against that run's Platform only.

**Platform change.** One Platform PR, `PLATFORM-NATIVE-PREPROD-OPS-1`.

- Owned paths:
  - `app/Console/Commands/PublishNativeRoute.php`
  - `app/Console/Commands/PublishNativeTrustedKey.php`
  - their tests under `tests/Feature/GameAuth/`
  - one line in the Platform native gateway login contract §14 or §17
  - the Platform task record
- `php artisan game-auth:native-route:publish --world-row-id= --channel-key= --host= --port=
  --tls-server-name= --login-enabled=<true|false>`
  - It calls `publishRouteForPreproduction` unchanged.
  - Before any write it refuses unless `APP_ENV` is `testing` or `preproduction`.
  - It prints a JSON readback receipt: `world_id`, `channel_id`, `route_version`,
    `route_revision`, `native_login_enabled`.
  - On failure it prints only a generic error, never the exception.
- `php artisan game-auth:native-trust:publish-key --key-id= --public-key-file=`
  - The issuer and profile are fixed to `NativeEvidenceContract::FRESH_ISSUER` and
    `FRESH_PROFILE`. The key purpose is `game-auth.native_evidence.fresh_key_purpose`, the value
    the issuer uses for its lookup. The command accepts no other scope.
  - Before any write it refuses unless `APP_ENV` is `testing` or `preproduction`.
  - It reads 32 raw bytes from a regular file that is not a symlink, and never takes the key from
    an argument.
  - It prints the key ID and the profile version.
- Not changed:
  - `isolatedConnection()` and its guards;
  - the trust registry rules (two fresh keys at most; no re-trust after revocation);
  - migrations, routes and the staging deployment.
- Tests:
  - each command refuses in `local`, `staging` and `production` with no row written;
  - the happy path in `testing` writes the rows and prints the receipt;
  - an endpoint change advances `route_version`;
  - `--login-enabled=false` keeps the endpoint and clears login;
  - a malformed selector, port or key file is refused before any write.
- Validation: `composer format:check`, `composer analyse` and `composer test`. Add the concurrency
  workflow only if a test is added there.
- Review: one independent review. No production trust change, because both commands refuse
  outside testing and preproduction.

**Credentials (names only).** All are generated per run and never committed or reused:

- `GAME_AUTH_NATIVE_ADMISSION_SIGNING_KEY_FILE` and `GAME_AUTH_NATIVE_ADMISSION_SIGNING_KEY_ID`
  (an Ed25519 seed file and its key ID; the public half goes to the trust command);
- `GAME_AUTH_GATEWAY_SERVICE_TOKEN_SHA256` and the Gateway's matching service token;
- `GAME_AUTH_NATIVE_EVIDENCE_HIGH_WATER_DIRECTORY` (a per-run directory) and
  `GAME_AUTH_NATIVE_EVIDENCE_FRESH_KEY_PURPOSE` (configuration, not a secret);
- `GAME_AUTH_NATIVE_RUNTIME_STATUS_IDENTITIES` and `GAME_AUTH_NATIVE_SCOPE_ASSIGNMENT_IDENTITIES`,
  bound to certificates from the per-run development CA;
- the per-run development root (`oteryn-dev-client`) and the server certificate for
  `--tls-server-name`;
- with LCFA, the projection identity from `ARCH-LCFA-PROJECTION-CONTRACT-V1` §2.

No GitHub, Synology, Cloudflare or database credential of any deployed environment is used.

**Rollback.**

- Set `--login-enabled=false` for the Channel. The route record stays, and outstanding grants
  expire within 30 s.
- Or stop the stack and delete its temporary directories. That removes every issued identity with
  it.
- Code rollback: revert the Platform PR. The commands are additive and nothing else calls them.

**Blast radius.**

- One disposable Platform process and its temporary files.
- A wrong command in a deployed environment refuses before any write: the environment gate, then
  `isolatedConnection()`.
- Public staging, production, Canary and the Game repositories are not affected.

## 3. Option B (deferred): a persistent private preproduction environment

Use this only when step 7 ("enablement for internal builds in `testing`/`preproduction`") needs
an environment that outlives one run.

**What is mutated.**

- A new Synology compose project, `oteryn-preprod`, with:
  - `APP_ENV=preproduction`;
  - its own MariaDB volume and database;
  - its own Redis;
  - no Cloudflare Tunnel route (reached only over a private network or VPN);
  - its own Gateway and game-node ports.
- In that database, the same rows as Option A, but **permanent**: the issued WorldId and ChannelId
  cannot be removed (F5).
- Platform code: `isolatedConnection()` must accept the configured preproduction MariaDB, in
  `preproduction` only. That changes a guard that the issuer review relied on, so it needs its
  own security review.
- The deployment workflow: a new manual, main-only `Deploy Synology Preproduction` job on the
  `oteryn-staging` runner, or on a new runner label.

**By whom.**

- A named operator identity with Synology shell or workflow-dispatch access runs the artisan
  commands inside the preproduction Platform container.
- The control plane records each run.

**Credentials (names only).**

- The Option A names, held as preproduction secrets on the Synology host, never in Git.
- `PREPROD_PLATFORM_DB_PASSWORD` and `PREPROD_MARIADB_ROOT_PASSWORD`.
- The runner's deploy access.
- No Cloudflare token, because no public route is created.

**Rollback.**

- `--login-enabled=false`; grants expire within 30 s.
- Stop the compose project.
- The issued identities remain (F5).
- Deleting the database volume removes them, but that is a separate destructive act that needs
  its own approval.

**Blast radius.**

- The shared Synology host: CPU, memory, disk and the runner queue it shares with staging.
- A misconfigured `.env` could point at the staging database. The relaxed guard must therefore
  check the configured preproduction database name and host, not only the environment.

**Approvals needed.**

- Deployment.
- A new database and volume.
- Preproduction secrets.
- The Platform guard change, with security review.
- The workflow change.
- The named operator.

## 4. Option C (rejected): native login on public staging

This would need `APP_ENV=staging` to be accepted by the issuer, mode 33a and `isolatedConnection()`
on a publicly reachable stack that shares MariaDB with Canary. Issued identities would become
permanent in a shared database. It contradicts the contract (§14 step 7 limits enablement to
testing and preproduction; production needs U8). Not offered.

## 5. Sequencing

1. `PLATFORM-NATIVE-PREPROD-OPS-1` (Option A).
2. Then RUNBOOK-1 uses the two commands in place of the PHP method calls in its §2.7 scope.
3. The joint E2E (§14 step 6) runs on that stack. It uses mode 33a until PLATFORM-LCFA-1 and
   GAME-LCFA-ENABLE-1 land, and the projection feed afterwards (`ARCH-LCFA-PROJECTION-CONTRACT-V1`
   §4).
4. Option B is asked for separately when step 7 is scheduled.

## 6. Owner approvals requested

Answer as, for example, `1a 2a 3b`.

1. **Platform write for one PR, `PLATFORM-NATIVE-PREPROD-OPS-1`** (the §2 owned paths, two
   commands gated to testing and preproduction, no guard change).
   - a) approve (recommended);
   - b) approve the route command only, and keep the trust key on test fixtures;
   - c) decline.
2. **Who may run the commands on the disposable stack.**
   - a) the RUNBOOK-1 operator and the Game node-boot CI job, with per-run secrets only
     (recommended);
   - b) the RUNBOOK-1 operator only, with no CI job.
3. **Persistent preproduction (Option B).**
   - a) authorize now: deployment, database, secrets, guard change and operator;
   - b) defer until step 7 is scheduled (recommended).
4. **Public staging (Option C).**
   - a) confirm that it stays refused (recommended).

## 7. Non-authorization

This document authorizes no code, migration, deployment, secret, runner, Cloudflare, database or
Platform change. Each approved item needs its own #162 allocation. The Platform PR uses only the
write grant from item 1, and only within the §2 owned paths.
