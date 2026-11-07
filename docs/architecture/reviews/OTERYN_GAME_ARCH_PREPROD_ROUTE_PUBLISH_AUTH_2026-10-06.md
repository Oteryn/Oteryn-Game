# PREPROD-ROUTE-PUBLISH-AUTH-1 Authority request for the preproduction route-publish operator path

- Decision: `ARCH-PREPROD-ROUTE-PUBLISH-AUTH-V1`
- Status: **CANDIDATE: AUTHORITY REQUEST**. Each §7 item takes effect independently, as soon as
  its own owner answer is recorded here; an item without a recorded answer grants nothing, and
  no item waits for the others. The document itself grants no authority beyond those recorded
  answers. Recorded: 1a (control plane D831, #162, given against the §2 owned-path list; it
  replaces D824 1a) and 2a (D824). **Option A is DECIDED: it proceeds now under answer 1a**
  (control plane D842, 2026-10-06, the ruling on deferral versus proceeding, which the owner
  confirmed directly to this architect session; it supersedes this document's earlier deferral
  recommendation). §2 freezes its design and the Platform write that 1a grants. Open: items 3
  and 4, and no answer to either grants authority (§7). **Option B is a deferred, undecided
  backlog entry (§3).**
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: the control plane, D821 item 2 (#162, 2026-10-06; owner answer **2a**). The request
  covers the route-publish operator path that the joint native-login E2E needs. That E2E is
  Platform native gateway login contract §14 step 6 (#1419 item 5).
- Evidence read: Oteryn/Oteryn-Platform `origin/main` 3896bcd, read only (all Platform citations
  below are at that revision; "login contract" is its
  `docs/contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md`); Oteryn/Oteryn-Game `main` 6560803cf for RUNBOOK-1 (F6, F7; its
  files are unchanged since 36c586516).
- Runtime, migration, deployment, production, protected-World and Platform write authority: NONE
  in this PR.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 0. Brief

The joint E2E needs four things in Platform:

1. a Registry WorldId and ChannelId;
2. a published route for that Channel;
3. `native_login_enabled=true` for that Channel;
4. the admission issuer public key in the native signing trust registry.

Item 1 has an operator command. Items 2–4 exist only as PHP methods. Platform's tests call them,
and the merged RUNBOOK-1 calls them through `php -r` inside its throwaway Platform container (F6).

The owner said "preproduction". Platform has **no preproduction deployment**. Its only deployed
environment is the public Synology staging stack, and native login code refuses to run there by
design.

So the request is a choice:

- **Option A (decided, control plane D842).** One small Platform PR,
  `PLATFORM-NATIVE-PREPROD-OPS-1`, adds two artisan commands, gated to testing and preproduction,
  for the route and the trust key. Both are fenced to the disposable per-run store (§2). The
  merged RUNBOOK-1 reaches the methods through `php -r` without them (F6); the commands give that
  path stable, refusal-tested entry points, which the runbook can adopt after a Platform pin
  bump. Its first run still needs a store fix on the Game side either way (F7).
- **Option B (deferred, undecided).** A persistent private preproduction environment. It is a
  backlog entry (§3) with no chosen design, reopened when step 7 needs an environment that
  outlives one run.
- **Option C (rejected).** Enable native login on public staging.

## 1. Findings

Each finding carries an evidence class: PROVEN (read in code or configuration at the cited
revision), DERIVED (follows from PROVEN facts), UNKNOWN or CONFLICT. No finding is CONFLICT.

**F1. No Platform preproduction environment exists.** PROVEN for the repository: the deployment
files and workflows below. UNKNOWN: whether a host outside the repository runs Platform; none is
declared.

- `deploy/synology/` is the only deployment; `deploy/ci/` holds only two CI Dockerfiles. No file
  outside the documentation sets `APP_ENV=preproduction`.
- `deploy/synology/.env.example:48` sets `APP_ENV=staging`.
- It is public through a Cloudflare Tunnel (`deploy/synology/PUBLIC_ENDPOINTS.md:5-14`):
  `https://oteryn.molehill.cloud` maps to Platform on 127.0.0.1:8000, and
  `https://gateway.molehill.cloud` maps to the Gateway on 8080.
- It shares one MariaDB with Canary: one `mariadb` service (`deploy/synology/compose.yml:4-10`)
  is both `DB_HOST` (`:56`) and `CANARY_DB_HOST` (`:61`).
- The `Deploy Synology Staging` workflow is manual (`workflow_dispatch`, `:4`) and main-only
  (`:90`), and runs in the `synology-staging` environment on the `platform-runners` group,
  label `oteryn-platform` (`.github/workflows/deploy-synology-staging.yml:91-94`).
- In `APP_ENV=staging`, three things refuse by design:
  - the native issuer;
  - mode 33a (`config/game-auth.php:59`: "Refused (NATIVE_LOGIN_UNAVAILABLE) in any other
    environment");
  - every route-record method, through `NativeTopologyRegistry::isolatedConnection()`
    (`NativeTopologyRegistry.php:152-154`).

**F2. The route-publish write exists only as a method.** PROVEN.

`App\GameAuth\Worlds\NativeTopologyRegistry::publishRouteForPreproduction(worldRowId, channelKey,
host, port, tlsServerName, loginEnabled)` (`NativeTopologyRegistry.php:80`) does the write. It
calls `isolatedConnection()` (`:89`) and then, in one transaction (`:91`):

- locks `game_worlds` (by `id`) FOR UPDATE (`:92`);
- locks `game_channels` (by `game_world_id`, `channel_key`) FOR UPDATE (`:93-94`);
- requires an already issued `world_id` and `channel_id` (`:95-96`);
- updates `native_route_host`, `native_route_port`, `native_route_tls_server_name`,
  `native_route_version`, `native_route_revision`, `native_login_enabled` and `updated_at`
  (`:105-112`).

An endpoint change advances the version (`:101-103`), and a new `route_revision` invalidates every
outstanding grant for the old one (login contract line 259). After the commit it logs "Disposable
native route record published." with `world_id`, `channel_id`, `route_revision` and
`native_login_enabled` (`:118-123`).

There is no artisan command, route or job for it. In Platform, only the tests call it (no caller
under `app/`, `routes/` or `config/`); Game RUNBOOK-1 calls it through `php -r` (F6). The N4P-3
task record lists "an operator path for `publishRouteForPreproduction` outside the isolated
connection" as out of scope, under separate authority
(`docs/agents/tasks/archive/OTV2-20260930-n4p3-native-admission-issuer.md:21`).

**F3. The connection guard admits only disposable stores.** PROVEN: `isolatedConnection()`
(`NativeTopologyRegistry.php:150-194`). The consequence for a persistent MariaDB is DERIVED.

`isolatedConnection()` uses the application's default connection (`:156`) and refuses in these
cases:

- the environment is not `testing` or `preproduction` (`:152`);
- an outer transaction is open (`:157`);
- the store is MySQL/MariaDB, unless all of these hold: database `oteryn_concurrency`, a loopback
  host, no unix socket, and `APP_ENV=testing` (`:162-168`);
- the store uses any driver other than MySQL/MariaDB or SQLite (`:172-173`);
- the store is SQLite, unless it is `:memory:` in testing (`:175`), or a regular retained file at
  `<sys_get_temp_dir>/oteryn-native-topology-<hex>/oteryn-native-topology.sqlite` with no
  symlink in the file or in its directory (`:181-189`).

A preproduction Platform can therefore publish routes **only** when its whole default database is
that retained temporary SQLite file. A persistent MariaDB is refused. RUNBOOK-1 needs that profile;
the merged runbook does not use it yet (F7).

The SQLite clause also checks the per-run directory. It takes `realpath(dirname($database))`
(`:182`) and requires that:

- it equals the configured `dirname($database)` (`:186`);
- its parent is `realpath(sys_get_temp_dir())` (`:184`).

A per-run directory that is a symlink resolves to its target, so the two paths differ and the
guard refuses, even when the file inside is regular. The configured path must therefore be
canonical, built from `realpath(sys_get_temp_dir())`.
`NativeTopologyRegistryTest.php:269,287-297` proves that issuance and readback refuse in
`preproduction` through a symlinked per-run directory. Every caller of the guard, and any future
operator path that calls these methods, inherits this **run-directory check**.

**F4. The trust key has no operator path either.** PROVEN: `NativeSigningTrustRegistry`.

The issuer public key must be in `native_game_signing_trust_profiles` (via
`NativeSigningTrustRegistry::publishTrustedKey`, `NativeSigningTrustRegistry.php:19`) before the
Gateway accepts a grant. That method has no command and no environment gate. It enters the
high-water witness for its namespace (`:31`), which creates a lock file in
`GAME_AUTH_NATIVE_EVIDENCE_HIGH_WATER_DIRECTORY` (`NativeEvidenceHighWaterWitness.php:43-46`;
`config/game-auth.php:37`), and only then opens
`DB::transaction` (`:38`) on the default connection, which is whatever database the process is
configured with. It has no disposable-store check like `isolatedConnection()`. The joint E2E needs
it as much as the route, so this request covers both.

**F5. Issued identities are permanent.** PROVEN: the issuance code and migration.

`issueForPreproduction` writes a `world_id` only where none is set
(`NativeTopologyRegistry.php:27-34`) and re-issuing returns the same pair
(`NativeTopologyRegistryTest.php:288-291`). The migration refuses to roll back while issued
records exist (`database/migrations/2026_09_26_150000_add_native_world_topology.php:26-32`). On
any persistent store, an issuance is a permanent change to the Registry.

**F6. RUNBOOK-1 reaches these methods without new commands.** PROVEN for the code: Game `main`
6560803cf (RUNBOOK-1 merged in 3d297c9db, #1872). UNKNOWN: whether the runbook has run anywhere.
Its README says only that the documented run was "not executed in this environment"
(`tools/qualification/login_local/README.md:78-80`), and Game `main` holds no `LOGIN_LOCAL_RESULT`
evidence; RUNBOOK-1-FU (D834) is to record it
(`docs/agents/tasks/archive/OTV2-20261006-runbook-1-login-local.md:37`).

- The Platform container runs with `APP_ENV=preproduction`
  (`tools/qualification/login_local/compose.override.yml:7`).
- `tools/qualification/login_local/run.sh:163` defines `php_exec`: `compose exec ... platform
  php -r`, which boots the Laravel kernel and evaluates a PHP snippet. node_boot uses the same
  pattern (`tools/qualification/node_boot/run.sh:168-169`, including `publishTrustedKey`).
- `run.sh:166` calls `game-auth:world:ensure` and then `issueForPreproduction`; `run.sh:170`
  calls `publishRouteForPreproduction(..., true)`; `run.sh:182` calls `publishTrustedKey` for
  the fresh issuer and profile.
- The README lists these as "Platform main, no Platform change" and says no step is pending
  `PLATFORM-NATIVE-PREPROD-OPS-1` (`README.md:68-73`).
- The commands that Option A adds do not exist at Platform 3896bcd: `app/Console/Commands/`
  has `IssueNativeTopology.php` and `EnsureGameWorld.php`, but no route or trust-key command, and
  `routes/console.php` defines none.

So RUNBOOK-1 does not need the Option A commands. It runs the methods in a throwaway container.
DERIVED: no protected preproduction environment exists (F1), so there is no place yet where this
ad-hoc `php -r` path would be inadmissible.

**F7. The merged runbook's topology steps hit the store guard.** PROVEN for the configuration;
the failure is DERIVED (no recorded run, F6).

- The Platform default store is MariaDB: `DB_CONNECTION: mysql`, `DB_HOST: db`,
  `DB_DATABASE: oteryn_s3a` (`tools/qualification/wp5_s3a/compose.yml:37-40`). None of the
  overlays that `run.sh:84-87` adds (`run.sh:33-35`) sets a database variable.
- With `APP_ENV=preproduction`, `isolatedConnection()` refuses any MySQL/MariaDB store
  (`NativeTopologyRegistry.php:162-168`: only `oteryn_concurrency` on loopback in `testing`).
- So `issueForPreproduction` at `run.sh:166` is refused. `game-auth:world:ensure` runs first in
  the same snippet and does not call the guard, so its `game_worlds` row is already in the
  throwaway MariaDB. Under `set -e` (`run.sh:19`) the run then ends with `result=FAIL`
  (`run.sh:82`) before the route and trust steps.

The Option A commands do not fix this: issuance and route publication call
`isolatedConnection()` themselves (`NativeTopologyRegistry.php:16,89`), whoever calls them. The fix belongs to
RUNBOOK-1's own paths and is for RUNBOOK-1-FU (D834). One example, not a choice: running Platform
with the retained per-run SQLite file as its whole default database (F3), so that the world row,
issuance and route share one store. UNKNOWN: whether the rest of the Platform stack runs on that
SQLite profile.

## 2. Option A (decided): the joint E2E on a disposable stack, with two new commands

**Decided.** Control plane D842 (2026-10-06), which the owner confirmed directly to this
architect session, settles the ruling on Option A: it proceeds now under answer 1a (D831). It is
no longer deferred. This section is the frozen specification for `PLATFORM-NATIVE-PREPROD-OPS-1`,
reassessed against Platform `origin/main` 3896bcd, which is unchanged since review round 7. The
Platform write comes only from answer 1a and covers only the owned paths below. The control plane
allocates the PR on #162 after this document merges.


**What is mutated.** All mutations happen in the per-run SQLite file
`<tmp>/oteryn-native-topology-<hex>/oteryn-native-topology.sqlite` of one disposable Platform
process with `APP_ENV=preproduction`, in CI as on a developer machine. Option A never runs in
`testing`. The changed rows are:

- one `game_worlds` row from `game-auth:world:ensure` (login disabled);
- its `world_id` and one `game_channels` row from `game-auth:native-topology:issue`;
- the route columns and `native_login_enabled` from the new route command;
- one `native_game_signing_trust_profiles` row from the new trust-key command, plus the high-water
  floor file in the per-run `GAME_AUTH_NATIVE_EVIDENCE_HIGH_WATER_DIRECTORY`.

Nothing outside the run's temporary directories changes. No staging, production or shared
database is touched. In `preproduction` the unchanged shared guard admits only the retained
per-run SQLite file: it refuses every MySQL/MariaDB store, the loopback `oteryn_concurrency`
test database included, and `:memory:` (F3). That fences issuance to the file, as well as the
two new commands.

**Where.** A developer machine, or a CI job on a GitHub-hosted runner. Never inside the Synology
staging stack or against its database. A per-run job on the self-hosted `oteryn-synology-game`
runner is possible if it starts its own disposable Platform process (UNKNOWN: that runner's PHP
toolchain; it has no Docker). The store guard below refuses any persistent database there too.

**By whom.** One of these:

- the RUNBOOK-1 operator, who is the person running `tools/qualification/login_local/run.sh`;
- the Game node-boot CI job that wraps it.

Each run has one writer. The commands run against that run's Platform only.

**Platform change.** One Platform PR, `PLATFORM-NATIVE-PREPROD-OPS-1`.

- Owned paths (the exact list answered as 1a, control plane D831; nothing outside it):
  - `app/Console/Commands/PublishNativeRoute.php`
  - `app/Console/Commands/PublishNativeTrustedKey.php`
  - `app/GameAuth/Worlds/DisposableNativeStore.php` (new: the shared guard below)
  - `app/GameAuth/Worlds/NativeTopologyRegistry.php` (only to call the shared guard from
    `isolatedConnection()`; no other change)
  - their tests under `tests/Feature/GameAuth/`
  - one line in the Platform native gateway login contract §14 or §17
  - the Platform task record
- `php artisan game-auth:native-topology:issue` (existing, unchanged) runs first, because the
  route needs issued IDs. Its first write is permanent (F5), so it is fenced the same way.
  - The command calls `issueForPreproduction` (`IssueNativeTopology.php:29`). That method calls
    `isolatedConnection()` (`NativeTopologyRegistry.php:16`) before its transaction (`:18`).
  - The guard applies the environment fence (`:152`) and then the run-directory check
    (`:181-189`). On any failure the command refuses with nothing written.
  - After the move, `isolatedConnection()` calls the shared guard, so issuance and the route
    command share one fence.
  - In `testing` that guard also admits `:memory:` and the loopback `oteryn_concurrency` test
    database, which Platform's own tests use. Option A therefore runs issuance only with
    `APP_ENV=preproduction` (above). `IssueNativeTopology.php` and the predicate stay unchanged
    and outside the D831 list. Issuance run in `testing` by mistake writes only to those test
    stores, as it can at 3896bcd today, and both new commands then refuse, so no route or trust
    state follows.
- `php artisan game-auth:native-route:publish --world-row-id= --channel-key= --host= --port=
  --tls-server-name= --login-enabled=<true|false>`
  - It calls `publishRouteForPreproduction` unchanged.
  - Before any write it refuses unless `APP_ENV` is `testing` or `preproduction`.
  - Before any write it also passes the shared disposable-store guard, because
    `publishRouteForPreproduction` calls `isolatedConnection()` before its transaction
    (`NativeTopologyRegistry.php:89-91`). That guard includes the **run-directory check** (F3).
    When the default store is a SQLite file:
    - the configured per-run directory `<sys_get_temp_dir>/oteryn-native-topology-<hex>` equals
      its own `realpath`, so neither it nor any parent is a symlink;
    - that `realpath` sits directly beneath `realpath(sys_get_temp_dir())`;
    - the database file is `oteryn-native-topology.sqlite`, regular, not a symlink, and equal to
      its own `realpath`.

    Otherwise the command refuses with nothing written. The clause moves with the predicate into
    `DisposableNativeStore` unchanged, so every caller of the shared guard applies it: issuance,
    readback, this command and the trust command. On a host whose system temporary path is a
    symlink, the run passes the canonical path. The run's single writer owns the directory, so
    it does not change between the check and the write.
  - Before any write it also requires the **retained per-run SQLite file**, in `testing` as in
    `preproduction`. It refuses `:memory:` and the loopback `oteryn_concurrency` database, which
    the shared guard admits in `testing`, with nothing written. The trust command applies the
    same retained-file check (below).
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
    symlink, including the run-directory check (route command above). An environment check
    alone is not enough, because `publishTrustedKey` writes to the default connection, which in
    a deployed process is its persistent database.
  - The Platform PR moves that predicate out of `NativeTopologyRegistry` into one shared guard,
    `DisposableNativeStore`, that both the registry and this command call. It may not relax or
    fork it; the registry's behaviour stays identical, which its existing tests prove.
  - Before any write it also fences the high-water directory, because `publishTrustedKey` writes
    the floor file, the witness-store lock file and the provenance row through the separately
    configured `GAME_AUTH_NATIVE_EVIDENCE_HIGH_WATER_DIRECTORY`. The configured path must be a
    canonical directory (its `realpath` equals the configured value), neither it nor its parent
    a symlink, and directly beneath the per-run directory that holds the current database
    file. To bind that directory to the current run, the trust command applies the retained-file
    check (route command above): it refuses `:memory:` and the loopback `oteryn_concurrency`
    database, which the shared guard otherwise allows, because neither names a per-run
    directory. Otherwise the command refuses with nothing written. Both checks live in
    `DisposableNativeStore`. Both new commands call the retained-file check; only the trust
    command calls the high-water check. The witness and the registry's guard are not changed.
  - It reads 32 raw bytes from a regular file that is not a symlink, and never takes the key from
    an argument.
  - It prints the key ID and the profile version.
- Not changed:
  - the guard predicate of `isolatedConnection()`, its run-directory check included (it moves,
    it does not change);
  - `game-auth:native-topology:issue` and `issueForPreproduction`;
  - the trust registry rules (two fresh keys at most; no re-trust after revocation);
  - migrations, routes and the staging deployment.
- Tests:
  - each command refuses in `local`, `staging` and `production` with no row written;
  - each command refuses in `preproduction` when the default store is not disposable (a MariaDB
    connection as in the staging deployment, a SQLite file outside the per-run directory, a
    symlinked file) with no row written and no high-water floor file created;
  - the issue command and each new command refuse in `testing` and in `preproduction` when the
    per-run directory is a symlink to a persistent directory and the database file inside it is
    regular and not a symlink, with no row, floor file or lock file written in the target; for
    issuance, no `world_id` and no `game_channels` row;
  - the issue command refuses in `local`, `staging` and `production` and with a non-disposable
    `preproduction` store, including `:memory:` and the loopback `oteryn_concurrency` database,
    with no `world_id` and no `game_channels` row written (the issue command tests exercise the
    shared guard through its issuance caller);
  - the trust command refuses, with a disposable store, when the high-water directory is outside
    the per-run directory, in another run's directory, a symlink or under a symlinked parent,
    with no row, floor file or lock file written;
  - each new command refuses in `testing` with a `:memory:` store or the loopback
    `oteryn_concurrency` database, with no route column, row, floor file or lock file written;
    the trust command does so even with a high-water directory under some other run's per-run
    directory;
  - the registry's existing `isolatedConnection()` tests pass unchanged after the guard moves,
    including
    `test_controlled_regular_file_reconnect_retains_issuer_readback_and_symlink_profile_is_refused`;
  - the happy path in `preproduction`, with a retained per-run SQLite file (and, for the trust
    command, a high-water directory beneath it), issues, writes the rows and prints the receipts;
  - an endpoint change advances `route_version`;
  - `--login-enabled=false` keeps the endpoint and clears login;
  - a malformed selector, port or key file is refused before any write.
- Validation: `composer format:check`, `composer analyse` and `composer test`. Add the concurrency
  workflow only if a test is added there.
- Review: one independent review. No production trust change, because both commands refuse
  outside testing and preproduction.

**Crash and retry.** Each run has one writer, so a retry never races another writer of the same
run. Recovery never repairs a run in place:

- Issuance and the route command each commit in one transaction
  (`NativeTopologyRegistry.php:18,91`). A failure leaves the previous state, and the run either
  retries the same command or is discarded.
- The trust command enters the high-water witness, which creates its lock file before the
  database transaction (F4). A failure between the two leaves state only in the run's own
  directories. The run is then discarded, by deleting its temporary directories, and a new run
  starts with a new key; it is not retried in place.

**Relation to RUNBOOK-1.** The commands require the retained per-run SQLite file, so they do not
fix F7: the runbook's store fix stays RUNBOOK-1-FU work. Switching the runbook from its `php -r`
snippets to the commands needs a Platform pin bump and Game path changes. That is a separate Game
task with its own #162 allocation, and this decision does not grant it.

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
- A wrong command in a deployed environment refuses before any write. The issue command and
  both new commands pass the environment fence and then the shared disposable-store guard, whose
  run-directory check refuses a symlinked per-run directory. None can therefore write to a
  staging, production or shared database, even when `APP_ENV` is set to
  `preproduction` by mistake or the per-run directory is a symlink to a persistent one. The
  trust command also refuses a high-water directory outside the run's own temporary directory,
  so no trust state is written to a shared or persistent path.
- Option A runs only in `preproduction`, where the guard admits only the retained per-run file,
  and both new commands require that file in `testing` too. The existing issue command in
  `testing` still admits `:memory:` and the loopback `oteryn_concurrency` test database (F3).
  That is unchanged Platform behaviour, which Option A does not use; no route or trust state
  can follow it.
- Public staging, production, Canary and the Game repositories are not affected.


## 3. Option B (deferred, undecided): backlog entry

**Subject.** A persistent private preproduction environment for Platform native login, one that
outlives a single run.

**Status.** Registered for later, not designed. No technology, host, store, network, guard change,
workflow, operator or credential set is chosen here, and no answer to §7 item 3 authorizes any of
it. The owner prefers Synology and the control plane has proposed a separate private stack there
with no guard relaxation; both are recorded as history only, not as a design or a decision.

**Safety constraints** (facts from §1 that any later design must address):

- Issued WorldId and ChannelId are permanent on a persistent store; rollback cannot remove them
  (F5).
- In `preproduction` the unchanged `isolatedConnection()` admits only the retained per-run SQLite
  file and refuses any MySQL/MariaDB store (F3). A persistent store therefore needs either a
  reviewed change to a guard that the issuer review relied on, or a store that guard admits.
- `publishTrustedKey` writes to the default connection and the high-water directory with no store
  or environment guard (F4).
- Public staging shares one MariaDB with Canary and is publicly reachable (F1); Option C stays
  refused (§4).

**Reopening trigger.** Rollout step 7 ("enablement for internal builds in
`testing`/`preproduction` only", login contract line 425) is scheduled and needs an environment
that outlives one run. Option B is then designed in an amendment to this decision, or in a
successor decision, and its authority is asked for afresh.

## 4. Option C (rejected): native login on public staging

This would need `APP_ENV=staging` to be accepted by the issuer, mode 33a and `isolatedConnection()`
on a publicly reachable stack that shares MariaDB with Canary (F1). Issued identities would become
permanent in a shared database (F5). It contradicts the contract (§14 step 7 limits enablement to
testing and preproduction; production needs U8; login contract line 425). Not offered.

## 5. Sequencing

1. RUNBOOK-1 (merged) reaches the Platform methods through `php -r` (F6).
2. RUNBOOK-1-FU fixes the store profile in RUNBOOK-1's own paths (F7) and records the first run.
   It does not wait for item 3.
3. `PLATFORM-NATIVE-PREPROD-OPS-1` (Option A, control plane D842): the control plane allocates it
   on #162 after this document merges. It writes only the §2 owned paths.
4. After it merges, a separate Game task may bump the runbook's Platform pin and call the two
   commands instead of the `php -r` snippets. That task needs its own allocation.
5. The joint E2E (§14 step 6) runs on that stack. It uses mode 33a until PLATFORM-LCFA-1 and
   GAME-LCFA-ENABLE-1 land, and the projection feed afterwards (`ARCH-LCFA-PROJECTION-CONTRACT-V1`
   §4).
6. Option B stays a backlog entry until its §3 trigger; its design and authority are then asked
   for afresh.

## 6. Mandatory decision test

`docs/agents/ARCHITECTURE_DECISION_DISCIPLINE.md`:

1. **Must decide now?** YES for Option A, by ruling: control plane D842, confirmed by the owner,
   decided that it proceeds now under 1a. This document had recommended deferral, because nothing
   is blocked on the commands (item 2). The ruling settles that question, so §2 is frozen. NO for
   Option B: nothing needs a persistent environment before step 7, so it is a backlog entry (§3)
   with no chosen technology or topology.
2. **Blocked downstream work.** Nothing is strictly blocked on the commands. RUNBOOK-1 reaches
   every method it needs through `php -r` in its throwaway container (F6). Its first run is blocked
   by the store guard (F7), which the commands do not change; that fix is RUNBOOK-1-FU work in Game
   paths. What the commands add is a refusal-tested operator path for the unguarded trust write
   (F4) in place of an ad-hoc snippet, and stable flags and receipts that a CI gate can call
   across Platform revisions.
3. **What becomes harder later.** The two commands become a Platform surface that runbooks and
   CI depend on, so changing their flags or receipts later means changing those callers. The
   shared guard becomes the one place to keep correct, and a change to it reaches every caller at
   once. The runbook pins Platform 3896bcd (`run.sh:23`) and calls the methods by name, so a
   Platform change to those methods breaks it at the next pin bump.
4. **Evidence to supersede Option A.** Any one of:
   - a persistent preproduction environment (Option B, §3) that needs a different operator path;
   - Platform operations tooling that replaces artisan commands;
   - a security finding on either command or on the shared guard;
   - a Platform change that removes or renames the methods the commands call.

   Superseding means amending this decision with the evidence. Any path outside the §2 owned
   paths needs a new owner answer.

   **Evidence that reopens Option B:** the §3 trigger (step 7 scheduled and needing an
   environment that outlives one run).
5. **Deliberately not decided.** Option B, its host, store, network, workflow, operator and
   credentials; any guard relaxation; the RUNBOOK-1 store fix (RUNBOOK-1-FU); the runbook's
   switch from `php -r` to the commands; the release and production operator path; production
   trust keys. Option C stays refused.

## 7. Owner approvals requested

Each item takes effect independently once its answer is recorded here. Recorded and in effect:
**1a** (control plane D831, #162, 2026-10-06, given against the §2 owned-path list) and **2a**
(D824). The ruling that Option A proceeds now under 1a is control plane D842 (2026-10-06),
confirmed by the owner directly to this architect session. Items 3 and 4 are open, and no answer
to either grants authority: item 3 only confirms that Option B stays a backlog entry, and item 4
only confirms a refusal. An answer recorded to an earlier form of item 3, including one that read
"authorize now", is history only and grants nothing; Option B's design and authority are
reassessed at its §3 trigger. Answer 1a rests on D831, which replaces the earlier D824 1a: that
answer predated `DisposableNativeStore.php` and the guard call in `NativeTopologyRegistry.php`.

Answer as, for example, `1a 2a 3a 4a`.

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
   - a) confirm that it stays an undecided backlog entry until its §3 trigger (recommended).
4. **Public staging (Option C).**
   - a) confirm that it stays refused (recommended).

## 8. Non-authorization

Beyond the §7 answers recorded and in effect (1a and 2a, each independently) and the control
plane D842 ruling that Option A proceeds under 1a, this document authorizes no code, migration,
deployment, secret, runner, Cloudflare, database or Platform change. Items 3 and 4 grant nothing.
Each approved item needs its own #162 allocation. The Platform PR uses only the write grant from
item 1 (D831), and only within the §2 owned paths that D831 listed: the two commands, the new
`DisposableNativeStore.php`, the guard call in `NativeTopologyRegistry::isolatedConnection()` and
nothing else in that file, their tests under `tests/Feature/GameAuth/`, one line in the native
gateway login contract §14 or §17, and the Platform task record. Any other path needs a new owner
answer. No Game path change is granted here, neither the runbook's store fix nor its switch to
the commands. Option B is a backlog entry (§3), and §3 freezes no design.
