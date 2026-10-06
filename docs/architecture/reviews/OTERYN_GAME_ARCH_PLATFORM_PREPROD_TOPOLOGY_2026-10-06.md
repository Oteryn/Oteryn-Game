# ARCH-PLATFORM-PREPROD-TOPOLOGY-0 Persistent Platform preproduction topology for the Synology Game node

- Decision: `ARCH-PLATFORM-PREPROD-TOPOLOGY-V1`
- Status: **PROPOSED** 2026-10-06. It needs owner acceptance through the control plane. It takes
  effect when the PR that carries it merges. Two rulings are already settled:
  - Registry issuance of the WorldId and ChannelId (§1) is owner-confirmed (D855).
  - The dedicated MariaDB store (§2) was decided by the control plane (D607).

  The `platform-preproduction` environment ruling is still open with the owner.
- Role: Supervising Architect (architect worker for the control plane, #1622 D852)
- Answers: the control plane, D852 (#1622, owner answer **1a**, 2026-10-06). The question is the
  smallest path to a persistent Platform preproduction topology in which the Synology Game node
  (`deploy/synology-game/`, SYNOLOGY-GAME-DEPLOY-1 #1874) can reach `ready`. This reopens
  ARCH-PREPROD-ROUTE-PUBLISH-AUTH-V1 Option B under its trigger, Platform login contract §14
  step 7 (internal builds in `testing`/`preproduction` only).
- Builds on: `OTERYN_GAME_ARCH_PREPROD_ROUTE_PUBLISH_AUTH_2026-10-06.md` (findings F1..F5, Option C
  rejected) and `OTERYN_GAME_ARCH_LCFA_PROJECTION_CONTRACT_2026-10-06.md` §4.
- Evidence: Oteryn/Oteryn-Platform `main` at `81898fc1` (#1469). Paths below are Platform paths
  unless they are marked Game.
- Runtime, migration, deployment, production and protected-environment authority: NONE in this PR.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 0. Implementation brief

1. **Host.** A separate private Platform stack runs on the same Synology host. It has the compose
   project `oteryn-preprod`, `APP_ENV=preproduction`, its own MariaDB, no Cloudflare Tunnel and LAN
   bindings only. It is deployed through a new GitHub environment, `platform-preproduction`.
   Public `synology-staging` is not reused (F1, Option C rejected).
2. **Store.** `DisposableNativeStore` stays as it is. A new, separate guard,
   `PersistentPreprodNativeStore`, admits only the stack's dedicated database. That database
   carries a provisioned store identity row. The topology, route and trust commands run against
   whichever of the two guards admits the store. Persistent mode uses the default connection, so
   the topology writes and the admission resolver read the same store. This removes the split
   store that `login_local` works around by mirroring.
3. **Identity.** The Platform Registry issues the WorldId and the ChannelId once, as UUIDv7, with
   `game-auth:native-topology:issue`. The owner approves that one run and the values are pinned
   from its receipt. They are distinct values and they are permanent (F5).
4. **Agreement.** A non-secret Game manifest, `deploy/synology-game/preprod-topology.toml`,
   records the pinned values. It holds the IDs, the route descriptor, the `route_revision` from the
   publish receipt, the Game-chosen readiness tokens, the assignment epoch and the certificate
   subjects. Both runbooks read the manifest, and the Game deploy refuses a node configuration that
   disagrees with it.
5. **Trust.** One preproduction development CA is held offline on the NAS. It issues per-purpose
   certificates with distinct subjects (Platform U15). A TLS 1.3-only nginx mTLS terminator in
   front of the Platform internal routes supplies `SSL_CLIENT_*`. No key or certificate enters any
   repository or GitHub secret store, apart from the deploy credentials that already exist.
6. **Packets.** `PLATFORM-PREPROD-TOPOLOGY-1` (Platform, one PR, needs a Platform write grant) and
   `GAME-PREPROD-TOPOLOGY-1` (Game: manifest, README runbook, deploy check).
7. **Deploys.** Every deploy to a protected environment needs a separate owner approval at run
   time. That covers `platform-preproduction`, the Game environment `preproduction` and the
   one-time topology issuance. Before the first dispatch, the `platform-preproduction`
   environment must be provisioned with its protection rule. The workflow reads that rule back
   and refuses to run if the rule is missing (§3). This decision performs no deploy.
8. **Client access.** The internal-build client reaches Platform over a client-facing HTTPS
   endpoint without mTLS, and reaches the Gateway the same way. The tester machine trusts the
   preproduction CA and resolves the private names (§4.1).

## 1. Rulings

| Item | Ruling |
|---|---|
| Platform environment | new private stack `oteryn-preprod` on the Synology host, GitHub environment `platform-preproduction` (main-only, owner as required reviewer), the same `platform-runners`/`oteryn-platform` runner and the same image build as staging; `APP_ENV=preproduction` |
| Not reused | `synology-staging` (`APP_ENV=staging`, public via Cloudflare Tunnel, MariaDB shared with Canary: F1, Option C) |
| Store | decided by the control plane (D607): dedicated MariaDB service in `oteryn-preprod` with its own volume, reachable only on the compose network; never the staging/Canary database |
| Guard | new `PersistentPreprodNativeStore`; `DisposableNativeStore` unchanged ("may not be relaxed or forked"); a `TMPDIR` or retained-file trick to make the disposable guard admit a persistent file is a configuration bypass and is refused |
| Trust-key write (F4) | `game-auth:native-trust:publish-key` runs only behind one of the two guards; in persistent mode its high-water directory must lie inside a configured persistent root |
| WorldId / ChannelId | owner-confirmed (D855): issued once by the Registry (UUIDv7, Platform owns identity), distinct; the owner approves the issuance run; values are pinned from the receipt and never reissued |
| Multichannel | one World, one Channel (`channel_key` `ch1`) for the first node; more Channels are later issuances under the same World |
| `route_revision` | Platform-computed (`rt.<version>.<digest>`) and read only from the `native-route:publish` receipt; the node reports it verbatim (`hash_equals`) |
| Other readiness tokens | Game-chosen, grammar `^[A-Za-z0-9._:-]{1,64}$`, recorded in the manifest; Platform does not pin them and copies them into grants |
| `source_authority` (readiness) | `oteryn:runtime:synology-preprod` |
| Assignment epoch | declared by Game ops, starts at `1`, equal in `node.toml` and `report.toml`; Platform keeps the highest epoch seen; a raise is a Game ops act recorded in the manifest |
| Endpoint names | private names under `preprod.oteryn.internal` (§4); the Game node connects by LAN address and verifies the name over TLS; the client resolves the names through the LAN DNS (§4.1) |
| Client access | `https://platform.preprod.oteryn.internal` (no mTLS, `/internal/*` refused) and `https://gateway.preprod.oteryn.internal`; the tester OS trust store holds the preproduction CA |
| Environment protection | `platform-preproduction` is provisioned with the owner as required reviewer and a `main`-only branch policy before any dispatch, and every run checks that rule first and fails closed |
| PKI | one offline preproduction development CA on the NAS; per-purpose subjects (§5); production PKI stays open (U15, U3) |
| Fencing | unchanged; admission and Character writes stay session-generation fenced on the Game side; this topology adds no write path |
| Mode 33a | allowed in this stack only until both LCFA packets are deployed here, then switched off (LCFA contract §4) |
| Production | out of scope; U8 and U7 stay open |

## 2. Store and guard

### 2.1 Why a new guard

The disposable guard admits only `:memory:` or a retained temporary SQLite file, and MySQL only
on loopback in `testing` (F3). A persistent preproduction store therefore needs either a reviewed
change to that guard or a second guard. The docblock forbids relaxing or forking the disposable
guard, and its behaviour is what keeps `testing` runs disposable. So this decision adds a second
guard with its own, narrower admission rule and leaves the first one untouched.

### 2.2 `PersistentPreprodNativeStore::connection()`

It admits the store only when all of the following hold, and otherwise refuses with nothing
written:

1. `app()->environment('preproduction')`. `testing` is not admitted, because tests use the
   disposable guard.
2. `GAME_AUTH_NATIVE_PERSISTENT_PREPROD_STORE=true`. The default is `false`.
3. The connection is the default connection, its driver is `mysql` or `mariadb`, and its database
   name equals `GAME_AUTH_NATIVE_PERSISTENT_PREPROD_DATABASE`. That name may not be empty, and no
   Canary database is configured in the process (`CANARY_DB_DATABASE` and the other Canary
   database keys that staging fills from `CANARY_DB_NAME` are empty), so a Canary-sharing stack
   such as staging is refused.
4. The table `native_preprod_store_identity` holds exactly one row, and that row's `store_id`
   equals `GAME_AUTH_NATIVE_PERSISTENT_PREPROD_STORE_ID`, a UUIDv7 generated at provisioning.
   This stops a misconfigured stack from writing permanent topology into a database that was never
   provisioned for it.
5. There is no outer transaction, which is the same rule as the disposable guard.

The identity row is written only by a new operator command,
`game-auth:native-preprod-store:provision --store-id=<uuid7>`. It runs behind conditions 1..3 and
5, and it refuses when a row already exists.

### 2.3 Command integration

`IssueNativeTopology`, `PublishNativeRoute`, `PublishNativeTrustedKey` and the
`NativeTopologyRegistry` methods resolve the store through one selector:

- If the persistent switch is on, the persistent guard is used and the retained run is not.
- Otherwise the disposable guard is used, exactly as today, including `retainedRun()` and
  `assertHighWaterDirectoryWithin`.
- There is never a fallback from one guard to the other.

In persistent mode, `PublishNativeTrustedKey` asserts that the configured
`GAME_AUTH_NATIVE_EVIDENCE_HIGH_WATER_DIRECTORY` lies inside
`GAME_AUTH_NATIVE_PERSISTENT_PREPROD_STATE_ROOT`. That root is a mounted persistent volume, and
symlinks are refused. Route-record reads keep their own `testing`/`preproduction` refusal (mode
34a). They read the default connection, which in persistent mode is the topology store, so no
mirror step exists.

## 3. Platform stack

- **Compose.** Add `deploy/synology-preprod/compose.yml`, built from the staging services with
  these differences:
  - `mariadb` gets its own volume and no port mapping.
  - `redis` is unchanged.
  - `platform` has `APP_ENV=preproduction` and binds to the NAS LAN address only.
  - `gateway` binds to the NAS LAN address only.
  - `internal-mtls` (nginx) listens on the NAS LAN address, port `8543`.
  - `edge-https` (nginx) listens on the NAS LAN address, port `443`. It serves
    `platform.preprod.oteryn.internal` and `gateway.preprod.oteryn.internal` over TLS 1.3 with no
    client certificate, proxies to `platform:8000` and the gateway, and returns `404` for
    `/internal/` so that internal routes stay behind `internal-mtls`.
  - There is no `canary` service, no Cloudflare Tunnel and no public endpoint.
- **Environment provisioning, before any dispatch.** Naming an environment in workflow YAML
  creates it without protection, so the owner first creates `platform-preproduction` in the
  Platform repository settings or through the REST API (`PUT /repos/{owner}/{repo}/environments/
  platform-preproduction`) with:
  - the owner as the only required reviewer;
  - a custom deployment branch policy that admits only `main`;
  - administrators not allowed to bypass.

  The owner then reads the environment back (`GET` on the same path) and checks that both rules
  are present.
- **Deploy workflow.** Add `.github/workflows/deploy-synology-preprod.yml`. It is
  `workflow_dispatch` only and refuses any branch other than `main`.
  - Its first job, `protection-check`, runs on a GitHub-hosted runner with no environment and
    `permissions: actions: read`. It reads the `platform-preproduction` environment through the
    REST API and fails, with nothing deployed, unless both rules are present: a
    `required_reviewers` rule naming the owner, and a deployment branch policy that admits only
    `main`. A missing environment fails the same way.
  - The deploy job `needs: protection-check` and only then names the environment. A run that
    fails the check therefore never creates the environment and never reaches the NAS. Every
    deploy that passes is approved by the owner at run time.
  - The deploy job sets
  `COMPOSE_PROJECT_NAME=oteryn-preprod`. The secrets in the environment are only the
  deploy-runner values the staging workflow already has, under new names (`OTERYN_PREPROD_*`).
  TLS material is not a GitHub secret: it is mounted read-only from the NAS path
  `/volume1/oteryn/platform-preprod/secrets/`.
- **mTLS terminator.** The staging `internal.conf` neither verifies client certificates nor
  restricts the TLS version. The preprod terminator does three things:
  - it sets `ssl_protocols TLSv1.3`, `ssl_verify_client on` and
    `ssl_client_certificate <preprod CA>`;
  - it forwards `SSL_CLIENT_VERIFY`, `SSL_PROTOCOL` and `SSL_CLIENT_S_DN` to Platform, as
    `GuardNativeRuntimeStatusPeer` requires;
  - it serves `platform-internal.preprod.oteryn.internal`.

  The `tools/qualification/login_local/nginx.conf` shape is the reference.
- **Platform environment variables** (on the NAS, not in the repository):

| Variable | Value |
|---|---|
| `GAME_AUTH_NATIVE_PERSISTENT_PREPROD_STORE` / `_DATABASE` / `_STORE_ID` / `_STATE_ROOT` | `true` / `oteryn_preprod` / provisioned UUIDv7 / `/var/lib/oteryn-preprod` |
| `GAME_AUTH_NATIVE_ADMISSION_ENABLED` | `true`; signing key file under the secrets mount |
| `GAME_AUTH_NATIVE_RUNTIME_STATUS_ENABLED` / `_IDENTITIES` | `true` / `{"CN=oteryn-preprod-node-1-runtime-status":["<WorldId>/<ChannelId>"]}` |
| `GAME_AUTH_NATIVE_SCOPE_ASSIGNMENT_ENABLED` / `_IDENTITIES` | `true` / the subject `CN=oteryn-preprod-game-ops` |
| `GAME_AUTH_NATIVE_EVIDENCE_MTLS_CLIENT_IDENTITY` | `CN=oteryn-preprod-node-1-native-evidence` |
| `GAME_AUTH_CHARACTER_BOOTSTRAP_INTENT_MTLS_CLIENT_IDENTITY` | `CN=oteryn-preprod-character-bootstrap` |
| `GAME_AUTH_NATIVE_ADMISSION_UNVERIFIED_CHARACTER_OWNERSHIP` / `_WORLD_ID` | `true` / `<WorldId>` until the LCFA feed is on here (§7), then `false` |

Platform already requires the identities to be distinct per purpose, and this table satisfies that.

## 4. Endpoints and TLS names

| Name (TLS identity) | Served by | Used by |
|---|---|---|
| `platform-internal.preprod.oteryn.internal` | `internal-mtls`, LAN `:8543`, client certificate required | Game node `[platform]` and `[platform.runtime_status]`; Game ops `report.toml` |
| `platform.preprod.oteryn.internal` | `edge-https`, LAN `:443`, no client certificate | internal-build client `OTERYN_PLATFORM_URL` (directory, OAuth token, ticket) and the system browser for the OAuth authorization page |
| `gateway.preprod.oteryn.internal` | `edge-https`, LAN `:443`, no client certificate | internal-build client `OTERYN_GATEWAY_URL` |
| `node-1.preprod.oteryn.internal` | Game node gameplay listener | route record `tls_server_name`; client verifies it against the configured preprod root (§17 mode 34a) |

`.internal` is reserved for private use, so these names never resolve publicly. Endpoints carry
the NAS LAN address and port. `peer_name` and `tls_server_name` carry the name above, which is
verified against the preproduction CA. The route record is `host=<NAS LAN address>`,
`port=<node gameplay port>` and `tls_server_name=node-1.preprod.oteryn.internal`.

### 4.1 Client trust and name resolution

`PlatformClientConfig` accepts `https` and loopback `http` only, so the client cannot reach
Platform through a plain LAN binding. Its HTTPS client verifies servers against the operating
system trust store, and the OAuth authorization page opens in the system browser. Each tester
machine is provisioned once:

1. **Trust.** Install the public preproduction CA certificate, never its key, into the operating
   system trust store. On Windows that is the Local Machine "Trusted Root Certification
   Authorities" store; on Linux it is `/usr/local/share/ca-certificates/` followed by
   `update-ca-certificates`. Browsers that use the system store then trust the OAuth page too.
   For gameplay TLS, `OTERYN_DEV_ROOT` points at the same public CA file (§17 mode 34a).
   Removing the CA from the store reverses this.
2. **Names.** Add the three client-facing names (`platform`, `gateway`, `node-1` under
   `preprod.oteryn.internal`) as A records for the NAS LAN address in the LAN DNS, for example the
   Synology DNS Server. Where there is no LAN DNS, add the same lines to the tester's hosts file.
   The names never appear in public DNS.
3. **Client settings.**
   - `OTERYN_PLATFORM_URL=https://platform.preprod.oteryn.internal/`
   - `OTERYN_GATEWAY_URL=https://gateway.preprod.oteryn.internal/`
   - `OTERYN_WORLD=<WorldId>`
   - `OTERYN_DEV_ROOT=<path to the public CA file>`
   - `OTERYN_OAUTH_CLIENT_ID` is the value of `game-auth:native-oauth-client:ensure`, or of the
     existing `EnsureNativeOAuthClient` command, run on the preprod stack.

## 5. Certificates and placement

The CA is generated once by the owner or the operator on the NAS. Its key lives in
`/volume1/oteryn/preprod-ca/` (root-owned, `0700`) and is never copied off the NAS, into a
repository or into GitHub. The CA issues:

| Subject / SAN | Purpose | Placement |
|---|---|---|
| SAN `platform-internal.preprod.oteryn.internal` | internal mTLS server | `/volume1/oteryn/platform-preprod/secrets/` |
| SANs `platform.preprod.oteryn.internal`, `gateway.preprod.oteryn.internal` | client-facing HTTPS server (`edge-https`) | same |
| SAN `node-1.preprod.oteryn.internal` | Game gameplay server | Game `<BASE>/node/secrets/` |
| `CN=oteryn-preprod-node-1-native-evidence` | node `[platform]` client | Game `<BASE>/node/secrets/platform-client.*` |
| `CN=oteryn-preprod-node-1-runtime-status` | node runtime status | Game `<BASE>/node/secrets/runtime-status.*` |
| `CN=oteryn-preprod-game-ops` | ownership authority (assignment) | Game `<ROOT_BASE>/ops/authority.*` |
| `CN=oteryn-preprod-character-bootstrap` | character bootstrap intent | Game Character Authority secrets |
| `CN=oteryn-preprod-character-projection` | LCFA feed, later (U-LC6) | issued when GAME-LCFA-ENABLE-1 is deployed here |

The public CA certificate is copied to both sides as the trust root, for example to
`platform-roots.pem` and the root copies in `<ROOT_BASE>/ops`. The admission signing key pair is
generated in the Platform secrets directory. Its public half is registered with
`game-auth:native-trust:publish-key` (§2.3) and handed to the Game node the same way as in
`login_local`. Rotation reissues from the same CA and edits the identity variables; it never
changes a WorldId or a ChannelId.

## 6. Revision tokens and the assignment epoch

- `route_revision` and `route_version` come from the JSON receipt of
  `game-auth:native-route:publish`. The operator copies them into the manifest, and from there into
  `node.toml [readiness].route_revision`. Republishing an unchanged descriptor keeps the revision.
  Any descriptor change advances the version and requires a Game redeploy with the new value.
- The remaining tokens are Game-chosen and bind to the deployed build, so a build change visibly
  moves them:
  - `ruleset_revision` and `content_revision`: `game.<12-hex Game commit>`. These are the same
    values that `ops character interpretation --ruleset/--content` binds into bootstrap intents.
  - `map_revision`: `map.<first 16 hex of the map file SHA-256>`.
  - `runtime_observation_revision`, `world_policy_revision` and `offer_revision`: `obs.1`, `wp.1`
    and `offer.1`. These advance by hand when their inputs change.
- `assignment_epoch`: `1` for the first node. `node.toml [platform.runtime_status]` and
  `report.toml` must both equal the manifest value. A raise means three steps: edit the manifest,
  redeploy, and `ops assignment assign`. Platform takes the highest value it has seen, so the epoch
  never moves down.

## 7. Rollout, including the LCFA packets

1. The owner accepts this decision, and its PR merges.
2. `PLATFORM-PREPROD-TOPOLOGY-1` merges in Platform. It is inert, because every switch defaults
   off and there is no deploy.
3. `GAME-PREPROD-TOPOLOGY-1` merges here, with the manifest values as placeholders. It is inert
   too. Steps 2 and 3 may run in either order.
4. The owner provisions the `platform-preproduction` environment and reads it back (§3). Then,
   with **owner run-time approval**, the operator deploys `platform-preproduction`; the run's
   `protection-check` must pass first. Then, on the NAS:
   1. migrations;
   2. `native-preprod-store:provision`;
   3. provisioning the local World row.
5. **Owner run-time approval, permanent:** `native-topology:issue --world-row-id=<row>
   --channel-key=ch1`. Then:
   1. pin the WorldId and the ChannelId in the manifest;
   2. run `native-route:publish` with login disabled, and pin `route_version` and
      `route_revision`;
   3. run `native-trust:publish-key`;
   4. set the identity variables (§3), then redeploy Platform with owner approval.
6. A manifest PR in this repository fills in the pinned values. The Game operator then runs the
   one-time first-start sequence by hand (Game README "First start sequence", steps 1..10). It
   ends with `ops assignment assign`, after which the node reaches `ready`. Only after that does an
   owner-approved Game deploy to `preproduction` run, and every later deploy is the automatic
   workflow path.
7. `native-route:publish` runs again with login enabled for the scope. Then the joint internal-build
   login E2E runs with mode 33a on (D171).
8. `PLATFORM-LCFA-1` and `GAME-LCFA-ENABLE-1` land in either order (LCFA contract §4). Once both
   are deployed here with owner approval, the feed switch goes on and mode 33a goes off, and the
   joint E2E reruns with the full §5.4 check.

**Rollback.**

- Disable native login on the route by republishing.
- Turn the runtime-status, assignment or feed switches off.
- Stop the `oteryn-preprod` stack.

Issued IDs are never deleted, and the migration refuses rollback while they exist (F5). A
discarded stack keeps its database volume until the owner decides to delete it.

## 8. Runbooks

**Platform operator**, all on the NAS and each deploy with owner approval:

1. Create the CA, then issue the server and client certificates in §5.
2. Have the owner provision `platform-preproduction` with its protection rule and read it back
   (§3). Do not dispatch until that is done.
3. Create `/volume1/oteryn/platform-preprod/{secrets,state}` and write the `.env` with the §3
   variables. Keep the runtime-status and assignment identities off until step 6.
4. Dispatch `deploy-synology-preprod.yml`. Its `protection-check` job must pass.
5. Run the provision, issue, route-publish and trust-publish steps of §7 steps 4..5, and save each
   JSON receipt to the manifest PR.
6. Turn the identities on and redeploy.
7. Run three checks:
   - `internal-mtls` rejects a client without a certificate, and rejects TLS 1.2;
   - `edge-https` returns `404` for `/internal/`;
   - a provisioned tester machine (§4.1) opens `https://platform.preprod.oteryn.internal/`
     without a certificate warning.

**Game operator:**

1. Install the Game-side certificates (§5) under `<BASE>/node/secrets` and `<ROOT_BASE>/ops`.
2. Render `node.toml`, `report.toml` and `ops.toml` from the templates (README steps 1..6), using
   the manifest values and endpoint `https://<NAS LAN address>:8543` with peer name
   `platform-internal.preprod.oteryn.internal`.
3. Run the existing one-time "First start sequence" of `deploy/synology-game/README.md`
   (steps 1..10) by hand, before any workflow dispatch, since the workflow cannot perform the
   first start. The manifest supplies the step 6 `ops character interpretation` ruleset and
   content tokens. Step 10 `ops assignment assign --world <WorldId> --channel <ChannelId>` uses
   the manifest epoch through `report.toml`. Recover an expired first start as that README
   describes.
4. Confirm that the node reports `ready`.
5. Only then dispatch `synology-game-deploy.yml` with owner approval, for this deploy and every
   later one. The deploy refuses if the rendered configuration disagrees with the manifest.

## 9. PLATFORM-PREPROD-TOPOLOGY-1 packet (Oteryn/Oteryn-Platform)

- **Authority:** Oteryn/Oteryn-Platform, one PR. It needs an owner grant of Platform write for
  this PR once the decision is accepted, the same pattern as D821. Platform CI, review and merge
  rules apply. It covers `preproduction` only, with every switch defaulting off.
- **Owned paths:**
  - `app/GameAuth/Worlds/PersistentPreprodNativeStore.php` (new);
  - `app/GameAuth/Worlds/NativeTopologyRegistry.php` and
    `app/GameAuth/Worlds/DisposableNativeStore.php`, for the shared selector call site only, with
    the admission logic of the disposable guard unchanged;
  - `app/Console/Commands/{IssueNativeTopology,PublishNativeRoute,PublishNativeTrustedKey}.php`;
  - `app/Console/Commands/ProvisionNativePreprodStore.php` (new);
  - one migration for `native_preprod_store_identity`;
  - `config/game-auth.php`, for the four persistent-store keys only;
  - `deploy/synology-preprod/` (new: compose, nginx `internal-mtls` and `edge-https` configs,
    README runbook including environment provisioning and tester trust/DNS, §4.1);
  - `.github/workflows/deploy-synology-preprod.yml` (new);
  - the matching tests under `tests/`;
  - the Platform contract §17, for one paragraph naming the persistent mode.
- **Acceptance:**
  1. Every condition in §2.2 refuses on its own, with nothing written.
  2. The persistent guard refuses in `testing`, `staging` and `production`, and refuses when a Canary
     database is configured.
  3. Existing disposable-guard tests pass unchanged.
  4. `issue`, `publish` and `readback` in persistent mode use the default connection, so the
     resolver sees the route without a mirror.
  5. The trust publish refuses a high-water directory outside the state root.
  6. Provisioning refuses a second identity row.
  7. The compose file publishes nothing on `0.0.0.0` and has no tunnel or Canary service.
  8. The nginx config is TLS 1.3 only, with `ssl_verify_client on`.
  9. The workflow is main-only and `workflow_dispatch` only, and uses the environment
     `platform-preproduction` only in a job that `needs` a `protection-check` job.
     `protection-check` fails closed on three cases: a missing environment, a missing owner
     reviewer rule, and a branch policy that admits anything other than `main`.
  10. `edge-https` is TLS 1.3, serves only the two client-facing names and refuses `/internal/`;
      the internal-build client logs in through it in the joint E2E (§7 step 7).
- **Validation:**
  - Platform CI;
  - unit and feature tests for each refusal;
  - a persistent-mode feature test that issues and publishes a topology, then admits a grant
    through `RegistryNativeAdmissionScopeResolver` on the same connection;
  - `docker compose config` on the new compose file;
  - an `nginx -t` check of both nginx configs;
  - a test of the `protection-check` evaluation against fixture API responses (protected,
    missing environment, no reviewer, any-branch policy).
- **Review:** independent review, because this touches persistence, identity issuance and trust.

## 10. GAME-PREPROD-TOPOLOGY-1 packet (this repository)

- **Authority:** one ordinary Game PR from a #1622/#162 allocation. It performs no deploy.
- **Owned paths:**
  - `deploy/synology-game/preprod-topology.toml` (new). It is non-secret and holds the IDs, the
    route descriptor, `route_version`/`route_revision`, the readiness tokens, `source_authority`,
    the epoch and the certificate subjects, with placeholders until §7 step 6.
  - `deploy/synology-game/README.md`: the Platform section, names and runbook of §8.
  - `deploy/synology-game/deploy-ops.sh`: a fail-closed comparison of the rendered `[readiness]`
    tokens and `assignment_epoch` (in `node.toml` and `report.toml`) against the manifest.
  - the task record.
- **Acceptance:**
  1. A deploy refuses on any mismatch, on a placeholder value, on an epoch of `0`, and on a
     WorldId equal to the ChannelId.
  2. The manifest contains no key, no certificate and no address outside the private range.
- **Validation:**
  - `git diff --check`;
  - `shellcheck deploy/synology-game/*.sh`;
  - the existing deploy-script tests, if any, extended with the mismatch cases;
  - `python tools/agents/validate_governance.py` for the task record.

## 11. Mandatory decision test

`docs/agents/ARCHITECTURE_DECISION_DISCIPLINE.md`:

1. **Must decide now?** YES, for `preproduction` only. The Synology Game node is deployed but
   cannot reach `ready` without a Platform that is persistent, private and mTLS-fronted.
2. **Blocked downstream work.**
   - the first `ready` node on Synology;
   - the joint internal-build login E2E on persistent topology;
   - the deployment of `PLATFORM-LCFA-1` and `GAME-LCFA-ENABLE-1` into a real stack.
3. **What becomes harder later.** Issued IDs are permanent, so this World and this Channel become
   the long-lived preproduction identity. A second guard is one more refusal surface to keep
   correct.
4. **Evidence to supersede.** Any of the following would supersede this decision:
   - Platform gains a production topology path (U8) that can serve preproduction too;
   - the store identity row proves insufficient to separate stores;
   - a security finding on the shared host;
   - the owner moves preproduction off the Synology host.
5. **Deliberately not decided.** This decision leaves open:
   - production topology and login (U8, U7);
   - production PKI and the gameplay CA (U15, U3);
   - multi-node and multi-Channel scaling;
   - automated certificate rotation;
   - removal of mode 33a code.

## 12. Non-authorization

This decision authorizes no code, migration, configuration, deployment or Platform change until
the owner accepts it. After acceptance it authorizes only the two packets above, each through its
own allocation, and the Platform packet additionally needs the owner's Platform write grant.

No deploy to a protected environment is authorized here. That includes `platform-preproduction`,
the Game `preproduction` environment and any topology issuance on the NAS. Each one needs a
separate owner approval at run time.

Production enablement, release-entry rulings, production PKI, credentials and any change to
`synology-staging` need separate authority. No secret, key or certificate is placed in any
repository.
