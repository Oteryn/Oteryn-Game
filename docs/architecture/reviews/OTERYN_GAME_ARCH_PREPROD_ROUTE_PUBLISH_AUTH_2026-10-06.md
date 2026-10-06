# PREPROD-ROUTE-PUBLISH-AUTH-1 Authority request for the preproduction route-publish operator path

- Decision: `ARCH-PREPROD-ROUTE-PUBLISH-AUTH-V1`
- Status: **CANDIDATE: AUTHORITY REQUEST**. Each §7 item takes effect independently, as soon as
  its own owner answer is recorded here; an item without a recorded answer grants nothing, and
  no item waits for the others. The document itself grants no authority beyond those recorded
  answers. Recorded: 1a (control plane D831, #162, given against the owned-path list of its
  question, §7; it replaces D824 1a) and 2a (D824). Open: items 3 and 4, and no answer to
  either grants authority (§7). **Option B is a deferred, undecided backlog entry (§3).**
  **Option A is deferred, pending the owner's ruling (§6).** This document recommends deferral,
  because the merged RUNBOOK-1 reaches the Platform methods without new commands (F6). The owner
  has been asked, through the control plane, whether Option A stays deferred or proceeds
  under answer 1a; that is not decided. Until the ruling is recorded here, no Platform PR is
  allocated under 1a, and Option A has no frozen design (§2).
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

- **Run step 6 on a disposable stack (RUNBOOK-1), with no new Platform command (recommended
  route).** The merged runbook already does this through `php -r` (F6). Its first run still
  needs a store fix on the Game side (F7).
- **Option A (deferred, pending the owner's ruling).** The same stack, plus one small Platform
  PR that adds two testing/preproduction-only artisan commands. It is a backlog entry (§2), not a
  frozen design; §6 lists what would reopen it.
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
- The commands that Option A would add do not exist at Platform 3896bcd: `app/Console/Commands/`
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

New commands would not fix this: issuance and route publication call `isolatedConnection()`
themselves (`NativeTopologyRegistry.php:16,89`), whoever calls them. The fix belongs to
RUNBOOK-1's own paths and is for RUNBOOK-1-FU (D834). One example, not a choice: running Platform
with the retained per-run SQLite file as its whole default database (F3), so that the world row,
issuance and route share one store. UNKNOWN: whether the rest of the Platform stack runs on that
SQLite profile.

## 2. Option A (deferred, pending the owner's ruling): backlog entry

**Subject.** An operator command surface on a disposable stack for `publishRouteForPreproduction`
and `publishTrustedKey`: one Platform PR, `PLATFORM-NATIVE-PREPROD-OPS-1`, adding two
testing/preproduction-only artisan commands (§7 item 1 as asked).

**Status.** Registered for later, not designed. The design discussed in PR #1871 review rounds 1–7
is history, not a specification: no class, command name, flag, test list or path list is frozen
here. If the owner's ruling proceeds under 1a, or a §6 reopen trigger occurs, the design and its
authority are reassessed against Platform `main` at that time and recorded in an amendment to
this decision before the control plane allocates any Platform PR.

**Known constraints for that reassessment** (from F2–F5; these are safety facts, not a design):

- `publishTrustedKey` has no store or environment guard and touches the high-water directory
  before its database transaction (F4). A check of `APP_ENV` alone is therefore not enough, and
  any guard has to run before the witness is entered.
- Issuance is permanent on any persistent store (F5), and it runs before the route can be
  published.
- In `testing`, `isolatedConnection()` also admits `:memory:` and the loopback `oteryn_concurrency`
  database (F3).

**Authority.** This document grants no Platform write. Answer 1a stays recorded as given (§7);
whether it covers a reassessed design is for the owner's ruling, not for this document.

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

1. RUNBOOK-1 (merged) reaches the Platform methods through `php -r` (F6). No Platform PR is
   needed for that.
2. RUNBOOK-1-FU fixes the store profile in RUNBOOK-1's own paths (F7) and records the first run.
3. The joint E2E (§14 step 6) runs on that stack. It uses mode 33a until PLATFORM-LCFA-1 and
   GAME-LCFA-ENABLE-1 land, and the projection feed afterwards (`ARCH-LCFA-PROJECTION-CONTRACT-V1`
   §4).
4. `PLATFORM-NATIVE-PREPROD-OPS-1` (Option A) stays deferred until the owner rules. It starts only
   after an amendment with a reassessed design is recorded here (§2) and the control plane
   allocates it on #162.
5. Option B stays a backlog entry until its §3 trigger; its design and authority are then asked
   for afresh.

## 6. Mandatory decision test

`docs/agents/ARCHITECTURE_DECISION_DISCIPLINE.md`:

1. **Must decide now?** NO for Option A: nothing is blocked on it (item 2), so this document
   recommends deferral and registers it as a backlog entry (§2) without a frozen design. The
   owner's ruling on deferral versus proceeding under 1a is pending. NO for Option B: nothing
   needs a persistent environment before step 7, so it is a backlog entry (§3) with no chosen
   technology or topology.
2. **Blocked downstream work.** None on the Option A commands. RUNBOOK-1 reaches every method it
   needs through `php -r` in its throwaway container (F6), and its README says no step is pending
   `PLATFORM-NATIVE-PREPROD-OPS-1`. Its first run is blocked by the store guard (F7), which new
   commands would not change. That fix is RUNBOOK-1-FU work in Game paths, not a Platform grant.
3. **What becomes harder later.** Little. The runbook pins Platform 3896bcd (`run.sh:23`) and
   calls the methods by name, so a Platform change to those methods breaks it at the next pin
   bump. Adding commands later is additive.
4. **Evidence that reopens Option A.** Any one of:
   - a protected or persistent preproduction environment (Option B, §3), or a CI or operator
     context where evaluating ad-hoc PHP inside the Platform container is not admissible;
   - a CI gate job that needs stable flags and receipts across Platform revisions instead of a
     pinned `php -r` snippet;
   - a security finding on running the unguarded `publishTrustedKey` (F4) through `php -r`
     against a store that is not disposable;
   - a Platform change that removes or renames the methods the runbook calls.

   Reopening, like a ruling to proceed under 1a, means amending this decision with the evidence
   and a reassessed design and authority (§2). The control plane then allocates the Platform PR on
   #162. Whether answer 1a covers it is the owner's call; any path the D831 question did not list
   needs a new owner answer.

   **Evidence that reopens Option B:** the §3 trigger (step 7 scheduled and needing an
   environment that outlives one run).
5. **Deliberately not decided.** Option A's design and the owner's ruling on it; Option B, its
   host, store, network, workflow, operator and credentials; any guard relaxation; the RUNBOOK-1
   store fix (RUNBOOK-1-FU); the release and production operator path; production trust keys.
   Option C stays refused.

## 7. Owner approvals requested

Each item takes effect independently once its answer is recorded here. Recorded: **1a**
(control plane D831, #162, 2026-10-06) and **2a** (D824). Their use waits for the owner's ruling
on Option A (§6): no Platform PR is allocated under 1a now. Items 3 and 4 are open, and no
answer to either grants authority: item 3 only confirms that Option B stays a backlog entry, and
item 4 only confirms a refusal. An answer recorded to an earlier form of item 3, including one
that read "authorize now", is history only and grants nothing; Option B's design and authority
are reassessed at its §3 trigger. Answer 1a rests on D831, which replaces the earlier D824 1a:
that answer predated two of the paths below.

Paths listed in the D831 question, kept as the record of what 1a answered (not a design and not
an allocation): `app/Console/Commands/PublishNativeRoute.php`,
`app/Console/Commands/PublishNativeTrustedKey.php`, `app/GameAuth/Worlds/DisposableNativeStore.php`
(new), `app/GameAuth/Worlds/NativeTopologyRegistry.php` (guard call in `isolatedConnection()`
only), their tests under `tests/Feature/GameAuth/`, one line in the Platform native gateway login
contract §14 or §17, and the Platform task record.

Answer as, for example, `1a 2a 3a 4a`.

1. **Platform write for one PR, `PLATFORM-NATIVE-PREPROD-OPS-1`** (the paths above, two
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

This document authorizes no code, migration, deployment, secret, runner, Cloudflare, database or
Platform change now. Answers 1a and 2a stay recorded, but Option A is deferred pending the
owner's ruling (§6), so no Platform PR is allocated under them, and §2 freezes no design. Option
B is a backlog entry (§3); no answer to item 3 or 4 grants anything, and §3 freezes no design.
Each approved item needs its own #162 allocation. A Platform PR for Option A starts only after
the §6 amendment, and any path outside the D831 list needs a new owner answer.
