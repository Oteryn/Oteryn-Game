# PREPROD-ROUTE-PUBLISH-AUTH-1 Authority request for the preproduction route-publish operator path

- Decision: `ARCH-PREPROD-ROUTE-PUBLISH-AUTH-V1`
- Status: **CANDIDATE: AUTHORITY REQUEST**. Each §7 item takes effect independently, as soon as
  its own owner answer is recorded here; an item without a recorded answer grants nothing, and
  no item waits for the others. The document itself grants no authority beyond those recorded
  answers. Recorded and in effect: 1a (control plane D831, #162, given against the final §2
  owned-path list; it replaces D824 1a) and 2a (D824). Open, so granting nothing: items 3
  and 4.
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

Each finding carries an evidence class: PROVEN (read in code or configuration at the cited
revision), DERIVED (follows from PROVEN facts), UNKNOWN or CONFLICT. No finding is CONFLICT.

**F1. No Platform preproduction environment exists.** PROVEN for the repository: the deployment
files and workflows below at 3896bcd. UNKNOWN: whether a host outside the repository runs
Platform; none is declared.

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

**F2. The route-publish write exists only as a method.** PROVEN: code and tests at 3896bcd.

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

**F3. The connection guard admits only disposable stores.** PROVEN: `isolatedConnection()` at
3896bcd. The consequence for a persistent MariaDB is DERIVED.

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

**F4. The trust key has no operator path either.** PROVEN: `NativeSigningTrustRegistry` at
3896bcd.

The issuer public key must be in `native_game_signing_trust_profiles` (via
`NativeSigningTrustRegistry::publishTrustedKey`) before the Gateway accepts a grant. That method
has no command and no environment gate. It writes through `DB::transaction` on the default
connection, which is whatever database the process is configured with; it has no disposable-store
check like `isolatedConnection()`. It needs `GAME_AUTH_NATIVE_EVIDENCE_HIGH_WATER_DIRECTORY`.
The joint E2E needs it as much as the route, so this request covers both.

**F5. Issued identities are permanent.** PROVEN: the issuance code and migration at 3896bcd.

The WorldId and ChannelId from `game-auth:native-topology:issue` are immutable. The migration
refuses to roll back while issued records exist. On any persistent store, an issuance is a
permanent change to the Registry.

**F6. RUNBOOK-1 already assumes these entrypoints.** PROVEN for the plan (the Game decision
cited below). RUNBOOK-1 itself is not implemented, so its final shape is UNKNOWN.

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

**Where.** A developer machine, or a CI job on a GitHub-hosted runner. Never inside the Synology
staging stack or against its database. A per-run job on the self-hosted `oteryn-synology-game`
runner is possible if it starts its own disposable Platform process (UNKNOWN: that runner's PHP
toolchain; it has no Docker). The store guard below refuses any persistent database there too.

**By whom.** One of these:

- the RUNBOOK-1 operator, who is the person running `tools/qualification/login_local/run.sh`;
- the Game node-boot CI job that wraps it.

Each run has one writer. The commands run against that run's Platform only.

**Platform change.** One Platform PR, `PLATFORM-NATIVE-PREPROD-OPS-1`.

- Owned paths (the exact list the owner approved as answer 1a, D831; nothing outside it):
  - `app/Console/Commands/PublishNativeRoute.php`
  - `app/Console/Commands/PublishNativeTrustedKey.php`
  - `app/GameAuth/Worlds/DisposableNativeStore.php` (new: the shared guard below)
  - `app/GameAuth/Worlds/NativeTopologyRegistry.php` (only to call the shared guard from
    `isolatedConnection()`; no other change)
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
  - Before any write, including the high-water floor file, it applies the **same disposable-store
    guard as `isolatedConnection()`**, unchanged: `APP_ENV` is `testing` or `preproduction`; no
    outer transaction; a MySQL/MariaDB store only as the loopback `oteryn_concurrency` database in
    `testing`; a SQLite store only as `:memory:` in `testing` or the retained per-run file
    `<sys_get_temp_dir>/oteryn-native-topology-<hex>/oteryn-native-topology.sqlite` with no
    symlink. An environment check alone is not enough, because `publishTrustedKey` writes to the
    default connection, which in a deployed process is its persistent database.
  - The Platform PR moves that predicate out of `NativeTopologyRegistry` into one shared guard,
    `DisposableNativeStore`, that both the registry and this command call. It may not relax or
    fork it; the registry's behaviour stays identical, which its existing tests prove.
  - Before any write it also fences the high-water directory, because `publishTrustedKey` writes
    the floor file, the witness-store lock file and the provenance row through the separately
    configured `GAME_AUTH_NATIVE_EVIDENCE_HIGH_WATER_DIRECTORY`. The configured path must be a
    canonical directory (its `realpath` equals the configured value), neither it nor its parent
    a symlink, and directly beneath the per-run directory that holds the current database
    file. To bind that directory to the current run, the trust command accepts only the retained
    per-run SQLite file, in `testing` as in `preproduction`. It refuses `:memory:` and the
    loopback `oteryn_concurrency` database, which the shared guard otherwise allows, because
    neither names a per-run directory. Otherwise the command refuses with nothing written. This
    check lives in `DisposableNativeStore` and only the trust command calls it; the witness and
    the registry's guard are not changed.
  - It reads 32 raw bytes from a regular file that is not a symlink, and never takes the key from
    an argument.
  - It prints the key ID and the profile version.
- Not changed:
  - the guard predicate of `isolatedConnection()` (it moves, it does not change);
  - the trust registry rules (two fresh keys at most; no re-trust after revocation);
  - migrations, routes and the staging deployment.
- Tests:
  - each command refuses in `local`, `staging` and `production` with no row written;
  - each command refuses in `preproduction` when the default store is not disposable (a MariaDB
    connection as in the staging deployment, a SQLite file outside the per-run directory, a
    symlinked file) with no row written and no high-water floor file created;
  - the trust command refuses, with a disposable store, when the high-water directory is outside
    the per-run directory, in another run's directory, a symlink or under a symlinked parent,
    with no row, floor file or lock file written;
  - the trust command refuses in `testing` with a `:memory:` store or the loopback
    `oteryn_concurrency` database, even with a high-water directory under some other run's
    per-run directory, with no row, floor file or lock file written;
  - the registry's existing `isolatedConnection()` tests pass unchanged after the guard moves;
  - the happy path in `testing` writes the rows and prints the receipt (the trust command with a
    retained per-run SQLite file and a high-water directory beneath it);
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
- A wrong command in a deployed environment refuses before any write. Both commands check the
  environment and then the shared disposable-store guard, so neither can write to a staging,
  production or other persistent database, even when `APP_ENV` is set to `preproduction` by
  mistake. The trust command also refuses a high-water directory outside the run's own
  temporary directory, so no trust state is written to a shared or persistent path.
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

**The likely shape on Synology (not an owner decision).** The owner prefers Synology. The control
plane proposed a separate private preproduction stack there: its own database, no Cloudflare
route, and no guard relaxation. This is the probable form of Option B, but item 3 is open and
nothing here records it as decided. It has one tension: with the guard unchanged, a
`preproduction` Platform writes routes and trust keys only to the retained per-run SQLite file
under the system temporary directory. A persistent stack with its own MariaDB would be refused
(F3). So "no guard relaxation" means either a per-run stack on Synology (Option A on that host)
or a reviewed guard change (above). The owner's item 3 answer decides which.

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

## 6. Mandatory decision test

`docs/agents/ARCHITECTURE_DECISION_DISCIPLINE.md`:

1. **Must decide now?** YES for Option A. NO for Option B, which stays deferred (item 3).
2. **Blocked downstream work.** The joint native-login E2E (Platform contract §14 step 6, #1419
   item 5) and RUNBOOK-1 (`ARCH-LOGIN-FIRST-PACKETS-V1` §2.7). Neither can publish a route or a
   trust key from a shell today (F2, F4, F6).
3. **What becomes harder later.** Two operator commands become a Platform surface that runbooks
   and CI depend on. Changing their flags or receipts later means changing those callers. Moving
   the guard into a shared class makes it one place to keep correct.
4. **Evidence to supersede.** A persistent preproduction environment (Option B) that needs a
   different operator path; Platform operations tooling that replaces artisan commands; a security
   finding on either command or on the shared guard.
5. **Deliberately not decided.** Option B and its shape on Synology; any guard relaxation; the
   release and production operator path; production trust keys; Option C stays refused.

## 7. Owner approvals requested

Each item takes effect independently once its answer is recorded here. Recorded and in effect:
**1a** (control plane D831, #162, 2026-10-06) and **2a** (D824). Items 3 and 4 are open and grant
nothing. The D831 question listed the final §2 owned paths, including the new
`DisposableNativeStore.php` and the guard-call-only change to `NativeTopologyRegistry.php`.
Answer 1a rests on D831, which replaces the earlier D824 1a: that answer predates those two paths.

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

## 8. Non-authorization

Beyond the §7 answers recorded and in effect (1a and 2a, each independently), this document
authorizes no code, migration, deployment, secret, runner, Cloudflare, database or Platform
change. Items 3 and 4 grant nothing until answered. Each approved item needs its own #162 allocation. The Platform PR uses only the
write grant from item 1 (D831), and only within the §2 owned paths that D831 listed: the two
commands, the new `DisposableNativeStore.php`, the guard call in
`NativeTopologyRegistry::isolatedConnection()` and nothing else in that file, their tests under
`tests/Feature/GameAuth/`, one line in the native gateway login contract §14 or §17, and the
Platform task record. Any other path needs a new owner answer.
