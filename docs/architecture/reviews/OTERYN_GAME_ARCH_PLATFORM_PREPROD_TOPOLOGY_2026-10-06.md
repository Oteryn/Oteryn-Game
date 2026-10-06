# ARCH-PLATFORM-PREPROD-TOPOLOGY-0 Persistent Platform preproduction topology for the Synology Game node

- Decision: `ARCH-PLATFORM-PREPROD-TOPOLOGY-V1`
- Status: **PROPOSED** 2026-10-06. It needs owner acceptance through the control plane. It takes
  effect when the PR that carries it merges. Three rulings are already settled:
  - Registry issuance of the WorldId and ChannelId (§1) is owner-confirmed (D855).
  - The new GitHub environment `platform-preproduction`, with `synology-staging` not reused
    (§1), is owner-confirmed (D856).
  - The dedicated MariaDB store (§2) was decided by the control plane (D607).
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
   publish receipt, the Game-chosen readiness tokens, the four character interpretation tokens,
   the assignment epoch and the certificate subjects. Both runbooks read the manifest, and the Game deploy refuses a node configuration that
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
| Platform environment | owner-confirmed (D856): new private stack `oteryn-preprod` on the Synology host, GitHub environment `platform-preproduction` (main-only, owner as required reviewer), the same `platform-runners`/`oteryn-platform` runner and the same image build as staging; `APP_ENV=preproduction` |
| Not reused | owner-confirmed (D856): `synology-staging` (`APP_ENV=staging`, public via Cloudflare Tunnel, MariaDB shared with Canary: F1, Option C) |
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
| Environment protection | `platform-preproduction` is provisioned with the owner as required reviewer, a `main`-only branch policy and no administrator bypass before any dispatch, and every run checks all three first and fails closed |
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
    client certificate, passes requests to Platform and the gateway, and returns `404` for
    `/internal/` so that internal routes stay behind `internal-mtls`.
  - `platform-web` (nginx) listens on `8080` on the compose network only, with no published
    port, for Gateway's service-token call (below).
  - There is no `canary` service, no Cloudflare Tunnel and no public endpoint.
- **Environment provisioning, before any dispatch.** Naming an environment in workflow YAML
  creates it without protection, so the owner first creates `platform-preproduction` in the
  Platform repository settings or through the REST API (`PUT /repos/{owner}/{repo}/environments/
  platform-preproduction`) with:
  - the owner as the only required reviewer;
  - a custom deployment branch policy that admits only `main`;
  - administrators not allowed to bypass.

  The owner then reads the environment back (`GET` on the same path) and checks all three:
  the reviewer rule, the branch policy, and `can_admins_bypass: false`.
- **Deploy workflow.** Add `.github/workflows/deploy-synology-preprod.yml`. It is
  `workflow_dispatch` only and refuses any branch other than `main`.
  - Its first job, `protection-check`, runs on a GitHub-hosted runner with no environment and
    `permissions: actions: read`. It reads the `platform-preproduction` environment through the
    REST API and fails, with nothing deployed, unless all three hold:
    - the `required_reviewers` rule names exactly one reviewer, the owner. A rule that also
      names any other user or team fails, because one approval from any listed reviewer is
      enough;
    - the deployment branch policy admits only `main`;
    - `can_admins_bypass` is present and `false`.

    A missing environment or a missing `can_admins_bypass` field fails the same way.
  - The deploy job `needs: protection-check` and only then names the environment. A run that
    fails the check therefore never creates the environment and never reaches the NAS. Every
    deploy that passes is approved by the owner at run time.
  - The deploy job sets
  `COMPOSE_PROJECT_NAME=oteryn-preprod`. The secrets in the environment are only the
  deploy-runner values the staging workflow already has, under new names (`OTERYN_PREPROD_*`).
  TLS material is not a GitHub secret: it is mounted read-only from the NAS path
  `/volume1/oteryn/platform-preprod/secrets/`.
- **mTLS terminator (`internal-mtls`).** The staging `internal.conf` neither verifies client
  certificates nor restricts the TLS version. The preprod terminator copies the server block of
  `tools/qualification/login_local/nginx.conf` (port `8443` there):
  - `ssl_protocols TLSv1.3`, `ssl_verify_client on`, `ssl_verify_depth 2`,
    `ssl_session_tickets off`, `ssl_client_certificate <preprod CA>`, and server name
    `platform-internal.preprod.oteryn.internal`;
  - exactly seven `POST` routes, and `404` for everything else. All seven are
    `/internal/v1/game-auth/` followed by:
    - `native-evidence`;
    - `character-bootstrap-intents/read`;
    - `native-runtime-status`;
    - `native-scope-assignments`;
    - `native-scope-revocations` (the assignment reporter's revocation report, over the same
      ownership-authority identity, `apps/game-server/src/native_admission_source/scope_assignment.rs`);
    - `native-account-characters`;
    - `native-account-characters/watermark`.

    The last two are the LCFA feed routes (LCFA contract §3,
    `apps/game-server/src/native_admission_source/descriptor.rs`). Platform answers them only
    once `PLATFORM-LCFA-1` is deployed, and admits only the projection certificate (§5);
  - each route is passed to the Platform PHP-FPM upstream over FastCGI with
    `fastcgi_param HTTPS on`, `SSL_CLIENT_VERIFY`, `SSL_PROTOCOL` and `SSL_CLIENT_S_DN` taken from
    the TLS session, as `GuardNativeRuntimeStatusPeer` requires. The six request-header forms
    (`HTTP_SSL_*`, `HTTP_X_SSL_*`) are set empty, so a client cannot inject a subject. An HTTP
    `proxy_pass` would turn the subject into a request header, so it is not used for these routes.
- **Gateway upstream (`platform-web`).** Gateway calls Platform's native admission route with its
  service token, not with mTLS. A third nginx server block listens on `8080` on the compose
  network only, with no published port. It copies the `login_local` port-`8080` block: it passes
  `/` and exactly `/internal/v1/game-auth/native-admissions` to Platform, and returns `404` for
  every other `/internal/` path. Gateway's `OTERYN_PLATFORM_BASE_URL` points at it.
- **Mounts.** All host paths are under `/volume1/oteryn/platform-preprod/`:

  | Host path | Container path | Mode | Content |
  |---|---|---|---|
  | `db/` | MariaDB data directory | volume | the dedicated store (D607) |
  | `state/` | Platform `/var/lib/oteryn-preprod` (the state root) | read-write, persistent | `witness/` (high-water directory, owned by the PHP-FPM user, chowned at container start as `wp5_s3a` does), the Laravel session files |
  | `secrets/admission/` | Platform `/run/oteryn-admission` | read-only | `signing.seed` (base64url Ed25519 seed, `0600`, owned by the PHP-FPM user uid) |
  | `secrets/tls/` | `internal-mtls`, `edge-https` | read-only | server certificates and keys, the public CA certificate |
  | `.env` | compose environment file | `0600`, root | the variables below, including the Gateway service token |

  No path is a symlink, and the state root is never a temporary directory or tmpfs.
- **Environment variables** (in the NAS `.env`, never in a repository). The Platform values below
  are the `login_local` set (`wp5_s3a` + `wp5_s3b` + `node_boot` + `login_local` overlays), with
  the preprod names, plus the persistent store.

  | Platform variable | Value |
  |---|---|
  | `APP_ENV` / `APP_DEBUG` / `APP_URL` | `preproduction` / `false` / `https://platform.preprod.oteryn.internal` |
  | `APP_KEY`, `DB_*` | generated on the NAS / the dedicated MariaDB service and database `oteryn_preprod` |
  | `SESSION_DRIVER` | a persistent driver (`file` under the state root, as `login_local`, or the staging driver), never `array` |
  | `GAME_AUTH_NATIVE_PERSISTENT_PREPROD_STORE` / `_DATABASE` / `_STORE_ID` / `_STATE_ROOT` | `true` / `oteryn_preprod` / provisioned UUIDv7 / `/var/lib/oteryn-preprod` |
  | `GAME_AUTH_NATIVE_EVIDENCE_ACTIVATED` / `_SOURCE_AUTHORITY` | `true` / `platform` (equal to `node.toml [platform].source_authority`) |
  | `GAME_AUTH_NATIVE_EVIDENCE_MTLS_CLIENT_IDENTITY` | `CN=oteryn-preprod-node-1-native-evidence` |
  | `GAME_AUTH_NATIVE_EVIDENCE_HIGH_WATER_DIRECTORY` | `/var/lib/oteryn-preprod/witness` (inside the state root, §2.3) |
  | `GAME_AUTH_NATIVE_EVIDENCE_FRESH_ACCOUNT_PURPOSE` / `_FRESH_ACCOUNT_SCOPE` / `_FRESH_KEY_PURPOSE` | `platform_security` / `fresh_admission` / `fresh_admission` |
  | `GAME_AUTH_NATIVE_EVIDENCE_CLOCK_UNCERTAINTY_SECONDS` / `_REQUESTS_PER_MINUTE` | `0` (Platform and node share the NAS clock) / `600` |
  | `GAME_AUTH_CHARACTER_BOOTSTRAP_INTENT_MTLS_CLIENT_IDENTITY` / `_TTL_SECONDS` | `CN=oteryn-preprod-node-1-native-evidence` / `300` |
  | `GAME_AUTH_NATIVE_ADMISSION_ENABLED` / `_SIGNING_KEY_FILE` / `_SIGNING_KEY_ID` | `true` / `/run/oteryn-admission/signing.seed` / `preprod-admission-key-1` |
  | `GAME_AUTH_GATEWAY_SERVICE_TOKEN_SHA256` | lowercase hex SHA-256 of the Gateway service token |
  | `GAME_AUTH_NATIVE_ADMISSION_UNVERIFIED_CHARACTER_OWNERSHIP` / `_WORLD_ID` | `true` / `<WorldId>` until the LCFA feed is on here (§7), then `false` |
  | `GAME_AUTH_NATIVE_RUNTIME_STATUS_ENABLED` / `_IDENTITIES` | `true` / `{"CN=oteryn-preprod-node-1-runtime-status":["<WorldId>/<ChannelId>"]}` |
  | `GAME_AUTH_NATIVE_SCOPE_ASSIGNMENT_ENABLED` / `_IDENTITIES` | `true` / `{"CN=oteryn-preprod-game-ops":["<WorldId>/<ChannelId>"]}` |

  | Gateway variable | Value |
  |---|---|
  | `OTERYN_PLATFORM_BASE_URL` | `http://platform-web:8080` (compose network only) |
  | `OTERYN_PLATFORM_SERVICE_TOKEN` | the Gateway service token |
  | `GATEWAY_LISTEN_ADDR` / `GATEWAY_NATIVE_LOGIN_ENABLED` | `:8080` / `true` |

  The character bootstrap identity is the native evidence identity on purpose. The Game node
  sends its one `[platform]` certificate for both the evidence route and the bootstrap-intent read
  (`apps/game-server/src/node/serve.rs`, OPS-NODE-BOOT-01 D1, as `node_boot` and `login_local`
  configure). The evidence, runtime-status and scope-assignment identities stay distinct, each
  with its own key.
- **Secret provisioning, on the NAS only.**
  - Gateway service token: `openssl rand -hex 32`. It is written once into the `.env` as
    `OTERYN_PLATFORM_SERVICE_TOKEN`, and only its SHA-256
    (`printf '%s' "$token" | openssl dgst -sha256 -r | cut -d ' ' -f 1`) goes into
    `GAME_AUTH_GATEWAY_SERVICE_TOKEN_SHA256`. Rotation replaces both and recreates `platform`
    and `gateway` together.
  - Admission signing key: `openssl genpkey -algorithm ed25519`. The 32-byte seed is written
    base64url without padding to `secrets/admission/signing.seed`, as `login_local` does. The
    raw 32-byte public key is extracted from the PEM into a temporary regular file the
    `platform` container can read
    (`openssl pkey -in <pem> -pubout -outform DER | tail -c 32 > <tmp>`, the last 32 bytes of the
    Ed25519 SubjectPublicKeyInfo). `game-auth:native-trust:publish-key --public-key-file=<tmp>`
    publishes it under key id `preprod-admission-key-1` and key purpose `fresh_admission` (§7
    step 5). Only after that publish succeeds are the PEM and the temporary file deleted; the
    seed file stays.
  - `APP_KEY`, the MariaDB passwords and the store id are generated on the NAS.
  - None of these is a GitHub secret, and none is printed into a workflow log.

### 3.1 Parity with `login_local`

Each `login_local` setting is carried as above, except for these, which are replaced on purpose:

| `login_local` | Preprod | Reason |
|---|---|---|
| retained SQLite topology fixture, plus a mirror into MariaDB | the persistent guard on the default connection (§2) | removes the split store |
| admission seed copied into a tmpfs per run | persistent read-only secrets file | the key must survive restarts |
| `LD_PRELOAD` / `WP5_FSYNC_FAULT` | not set | qualification fault injection only |
| per-run CAs and a self-signed gameplay certificate | one preprod CA (§5) | persistent trust |
| loopback ports | NAS LAN address only | testers on the LAN |
| test account inserted into `identities` | tester accounts created through Platform account registration (§8) | no direct table writes in a persistent stack |
| `game-auth:world:ensure --host 127.0.0.1` | `--host <NAS LAN address> --port <node gameplay port>` | LAN route |

## 4. Endpoints and TLS names

| Name (TLS identity) | Served by | Used by |
|---|---|---|
| `platform-internal.preprod.oteryn.internal` | `internal-mtls`, LAN `:8543`, client certificate required | Game node `[platform]` and `[platform.runtime_status]`; Game ops `report.toml` |
| `platform.preprod.oteryn.internal` | `edge-https`, LAN `:443`, no client certificate | internal-build client `OTERYN_PLATFORM_URL` (directory, OAuth token, ticket) and the system browser for the OAuth authorization page |
| `gateway.preprod.oteryn.internal` | `edge-https`, LAN `:443`, no client certificate | internal-build client `OTERYN_GATEWAY_URL` |
| `platform-web:8080` | `platform-web`, compose network only, never published | Gateway `OTERYN_PLATFORM_BASE_URL`, service token (§3) |
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
   - `OTERYN_CHARACTER_ID=<CharacterId>`: one per tester account. It is the UUID that the Game
     Character Authority assigns when it bootstraps the character from a Platform intent (§8,
     Game operator steps 5..6), read from `game_character_roots` for that bootstrap. It is not
     invented on the client.
   - `OTERYN_DEV_ROOT=<path to the public CA file>`
   - `OTERYN_OAUTH_CLIENT_ID` is the client id printed by `game-auth:oauth-client:ensure`, run
     on the preprod stack, as `login_local` does.

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
| `CN=oteryn-preprod-character-projection` | LCFA feed (U-LC6) | Game `<BASE>/node/secrets/account-characters.*`, issued before §7 step 8; a root copy of the public `account-characters.crt` in `<ROOT_BASE>/ops` is listed in `report.toml` `other_producer_certificate_files` |

The node evidence certificate also authenticates the character bootstrap intent read, so there
is no separate bootstrap certificate (§3). The runtime-status certificate's subject is the
`NODE_IDENTITY` in `scope.env` and the `[node_certificate_files]` key in `report.toml`; root
copies of `runtime-status.crt` and of the public CA as `platform-roots.pem` live in
`<ROOT_BASE>/ops`, as the Game README requires. The authority key never shares a public key with
a node certificate. `oteryn-game-ops` enforces that against the files in `report.toml`
`other_producer_certificate_files`, so every node producer certificate has a root-owned public
copy there, the LCFA projection certificate included (§7 step 8).

The admission signing seed is generated in the Platform secrets directory (§3). Its public half
is registered only in the Platform trust registry with `game-auth:native-trust:publish-key`
(§2.3). It is not handed to the Game node, which never verifies admission signatures itself. Rotation reissues from the same CA and edits the identity variables; it never
changes a WorldId or a ChannelId.

## 6. Revision tokens and the assignment epoch

- `route_revision` and `route_version` come from the JSON receipt of
  `game-auth:native-route:publish`. The operator copies them into the manifest, and from there into
  `node.toml [readiness].route_revision`. Republishing an unchanged descriptor keeps the revision.
  Any descriptor change advances the version and requires a Game redeploy with the new value.
- The remaining tokens are Game-chosen and bind to the deployed build, so a build change visibly
  moves them:
  - `ruleset_revision` and `content_revision`: `game.<12-hex Game commit>`.
  - The character interpretation needs all four tokens. The manifest pins them in
    `[interpretation]`:
    - `profile_revision = "profile.1"`;
    - `ruleset_revision` as above;
    - `content_revision` as above;
    - `starter_template_revision = "starter.1"`.

    `ops character interpretation --profile --ruleset --content --starter` takes exactly these
    four. Every Platform `game-auth:character-bootstrap-intent:issue` passes the same four as
    `--profile-revision`, `--ruleset-revision`, `--content-revision` and
    `--starter-template-revision`. Profile and starter advance by hand when their inputs change.
  - `map_revision`: `map.<first 16 hex of the map file SHA-256>`.
  - `runtime_observation_revision`, `world_policy_revision` and `offer_revision`: `obs.1`, `wp.1`
    and `offer.1`. These advance by hand when their inputs change.
- `assignment_epoch`: `1` for the first node. `node.toml [platform.runtime_status]` and
  `report.toml` must both equal the manifest value. Platform takes the highest value it has seen,
  so the epoch never moves down. The assignment command depends on the scope's state:
  - **No assignment yet** (first start only): `ops assignment assign`, README step 10.
  - **Assigned** (every later case, epoch raises included): `assign` is refused with
    `PredecessorMismatch`, so it is never used again. To raise the epoch, edit the manifest, then
    `node.toml` and the root copy of `report.toml`, then run the owner-approved Game deploy. The
    deploy restarts the node and runs `assignment replace` through the wrapper, which reports the
    new epoch to Platform. No manual assignment step follows.
  - **Unknown outcome:** the deploy reconciles it (`assignment reconcile`, as the README describes)
    before it touches the service.
  - **Committed, but the Platform report failed:** the root owner runs `ops assignment report
    --report-config ROOT_BASE/ops/report.toml --node-config BASE/node/node.toml
    --node-identity <NODE_IDENTITY> --world <WorldId> --channel <ChannelId>`. This re-reports the
    current assignment without writing a new one.

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
   3. `game-auth:world:ensure --id 1 --slug <slug> --name <name> --region <region>
      --host <NAS LAN address> --port <node gameplay port>`, which provisions the local World
      row;
   4. `game-auth:oauth-client:ensure` for the internal-build client.
5. **Owner run-time approval, permanent:** `native-topology:issue --world-row-id=<row>
   --channel-key=ch1`. Then:
   1. pin the WorldId and the ChannelId in the manifest;
   2. run `native-route:publish` with login disabled, and pin `route_version` and
      `route_revision`;
   3. run `native-trust:publish-key --public-key-file=<tmp>` for `preprod-admission-key-1`,
      purpose `fresh_admission`, with the raw public key file from §3, then delete the PEM and
      the temporary file;
   4. set the identity variables (§3), then redeploy Platform with owner approval.
6. A manifest PR in this repository fills in the pinned values. The Game operator then runs the
   one-time first-start sequence by hand (Game README "First start sequence", steps 1..10). It
   ends with `ops assignment assign`, after which the node reaches `ready`. Only after that does an
   owner-approved Game deploy to `preproduction` run, and every later deploy is the automatic
   workflow path.
7. `native-route:publish` runs again with login enabled for the scope. Then the joint internal-build
   login E2E runs with mode 33a on (D171).
8. `PLATFORM-LCFA-1` and `GAME-LCFA-ENABLE-1` (#1898) land in either order (LCFA contract §4).
   `GAME-LCFA-ENABLE-1` owns the code and the `[platform.account_characters]` template section.
   This document owns only the operator steps and their order. Once both are deployed here with
   owner approval, the operator runs these steps in order, and mode 33a stays on until the last
   one passes:
   1. On Platform, set the projection identity variable that `PLATFORM-LCFA-1` defines to
      `CN=oteryn-preprod-character-projection`, then redeploy with owner approval.
   2. Render `[platform.account_characters]` in `node.toml` with:
      - the projection certificate and key (§5);
      - the Character Authority source authority;
      - the epoch-fence file path, `<BASE>/node/lcfa/epoch.fence`.

      The fence file lives outside the Character store and outside every Game backup. In the
      same step, install a public copy of `account-characters.crt` at
      `<ROOT_BASE>/ops/account-characters.crt` (root, `0644`; never the key) and add it to
      `report.toml` `other_producer_certificate_files`, so every later `oteryn-game-ops`
      assignment report, replace or revoke checks the ownership-authority key against the
      projection key.
   3. Create the fence file once with value `0`, in the format `GAME-LCFA-ENABLE-1` defines. It
      shares the lifetime of the Platform read model: delete it only when Platform `state/` and
      the database volume are deleted, and never when the Game store is restored (LCFA contract
      §5 epoch fence).
   4. On Platform, configure the complete `native_account_characters` section (LCFA contract §4,
      Platform packet item 6) and redeploy with owner approval:
      - `identities` set to the projection certificate subject from step 1;
      - `source_authority` set to the same Character Authority value rendered in step 2;
      - `liveness_seconds` 30, `clock_uncertainty_seconds` 1, and `requests_per_minute`;
      - `enabled` set to `true` last. `APP_ENV=preproduction` is one of the two environments
        where the switch is honoured.

      With the switch on and no feed yet, issuance fails closed on the stale feed, because the
      feed takes precedence over mode 33a. Do this step and steps 5 and 6 in one maintenance
      window, with no tester logins expected until the step 6 gate passes.
   5. Start the publisher, then run the initial resync,
      `game_character_account_projection_resync(false)`.
   6. Gate on freshness:
      - the latest watermark is at the highest epoch, and Platform does not report it stale
        under S = 30 s (LCFA contract §5.1, U-LC5);
      - the account Character list read returns each tester's Character.

      The precondition for turning mode 33a off is a feed that is fresh and non-empty. Anything
      else blocks this step, and mode 33a stays on.
   7. Turn mode 33a off, set
      `GAME_AUTH_NATIVE_ADMISSION_UNVERIFIED_CHARACTER_OWNERSHIP=false`, and redeploy Platform
      with owner approval. Then rerun the joint E2E with the full §5.4 check.

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
3. Create `/volume1/oteryn/platform-preprod/{db,secrets/admission,secrets/tls,state/witness}`
   with the §3 owners and modes. Generate the admission seed and the Gateway service token as §3
   says, then write the `.env` with every §3 Platform and Gateway variable. Keep the
   runtime-status and assignment switches off until step 6.
4. Dispatch `deploy-synology-preprod.yml`. Its `protection-check` job must pass.
5. Run the provision, issue, route-publish and trust-publish steps of §7 steps 4..5, and save each
   JSON receipt to the manifest PR.
6. Turn the identities on and redeploy.
7. Run three checks:
   - `internal-mtls` rejects a client without a certificate, and rejects TLS 1.2;
   - `edge-https` returns `404` for `/internal/`;
   - a provisioned tester machine (§4.1) opens `https://platform.preprod.oteryn.internal/`
     without a certificate warning.
8. Create each tester account through Platform account registration on that page, never by a
   direct table write.

**Game operator:**

1. Install the Game-side certificates (§5) under `<BASE>/node/secrets` and `<ROOT_BASE>/ops`.
2. Render `node.toml`, `report.toml` and `ops.toml` from the templates (README steps 1..6), using
   the manifest values. The endpoint is the socket address `<NAS LAN address>:8543`, with no
   scheme, in both `node.toml` `[platform]` and `report.toml`, as the existing templates and
   `login_local` write it. The peer name is `platform-internal.preprod.oteryn.internal`.
3. Run the existing one-time "First start sequence" of `deploy/synology-game/README.md`
   (steps 1..10) by hand, before any workflow dispatch, since the workflow cannot perform the
   first start. The manifest `[interpretation]` table supplies all four step 6
   `ops character interpretation` tokens (`--profile`, `--ruleset`, `--content`, `--starter`). Step 10 `ops assignment assign --world <WorldId> --channel <ChannelId>` uses
   the manifest epoch through `report.toml`. Recover an expired first start as that README
   describes.
4. Confirm that the node reports `ready`.
5. For each tester account, generate one fresh UUIDv7 `OPERATION_ID` and record it with the
   account. On the Platform stack, run the full `login_local` invocation:

   ```sh
   php artisan game-auth:character-bootstrap-intent:issue \
     --identity-id=<the tester account's Platform identity id> \
     --operation-id="$OPERATION_ID" \
     --target-world-id=<WorldId> \
     --requested-name=<the tester's character name> \
     --profile-revision=<[interpretation] profile> \
     --ruleset-revision=<[interpretation] ruleset> \
     --content-revision=<[interpretation] content> \
     --starter-template-revision=<[interpretation] starter>
   ```

   Then, on the Game node, run
   `ops character bootstrap --socket BASE/run/control.sock --operation-id "$OPERATION_ID"` with
   the same value. The identity id is read from the Platform `identities` row of the registered
   account (§8 Platform operator step 8); the operator never writes that row.
6. Record the resulting `character_id` from `game_character_roots` as that tester's
   `OTERYN_CHARACTER_ID` (§4.1).
7. Only then dispatch `synology-game-deploy.yml` with owner approval, for this deploy and every
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
     `protection-check` fails closed on five cases:
     - a missing environment;
     - a missing owner reviewer rule;
     - a reviewer rule that names any user or team besides the owner;
     - a branch policy that admits anything other than `main`;
     - `can_admins_bypass` that is missing or `true`.
  10. `edge-https` is TLS 1.3, serves only the two client-facing names and refuses `/internal/`;
      the internal-build client logs in through it in the joint E2E (§7 step 7) with an account
      created through Platform registration.
  11. The `.env` template sets every §3 variable, including
      `GAME_AUTH_NATIVE_EVIDENCE_HIGH_WATER_DIRECTORY` inside the state root on the persistent
      `state/` mount, and no `LD_PRELOAD`. The bootstrap-intent identity equals the native
      evidence identity.
  12. Gateway has `GATEWAY_NATIVE_LOGIN_ENABLED=true` and `OTERYN_PLATFORM_SERVICE_TOKEN`;
      Platform holds only `GAME_AUTH_GATEWAY_SERVICE_TOKEN_SHA256`. `platform-web` has no
      published port and exposes only `native-admissions` under `/internal/`.
  13. `internal-mtls` passes `SSL_CLIENT_VERIFY`, `SSL_PROTOCOL` and `SSL_CLIENT_S_DN` as FastCGI
      parameters, clears the `HTTP_SSL_*` and `HTTP_X_SSL_*` forms, and has session tickets
      off. It allows `POST` on exactly seven internal routes: `native-evidence`,
      `character-bootstrap-intents/read`, `native-runtime-status`, `native-scope-assignments`,
      `native-scope-revocations`, `native-account-characters` and
      `native-account-characters/watermark`.
- **Validation:**
  - Platform CI;
  - unit and feature tests for each refusal;
  - a persistent-mode feature test that issues and publishes a topology, then admits a grant
    through `RegistryNativeAdmissionScopeResolver` on the same connection;
  - `docker compose config` on the new compose file;
  - an `nginx -t` check of the three nginx server configs;
  - a test of the `protection-check` evaluation against fixture API responses (protected,
    missing environment, no reviewer, owner plus another reviewer or team, any-branch policy, admin bypass `true`, admin bypass
    field missing).
- **Review:** independent review, because this touches persistence, identity issuance and trust.

## 10. GAME-PREPROD-TOPOLOGY-1 packet (this repository)

- **Authority:** one ordinary Game PR from a #1622/#162 allocation. It performs no deploy.
- **Owned paths:**
  - `deploy/synology-game/preprod-topology.toml` (new). It is non-secret and holds the IDs, the
    route descriptor, `route_version`/`route_revision`, the readiness tokens, the four
    `[interpretation]` tokens, both source authorities, the epoch and the certificate subjects,
    with placeholders until §7 step 6. The two source authorities stay distinct, because
    `NodeConfig::parse` rejects equal values (`apps/game-server/src/node/config.rs`):
    - `[readiness].source_authority` is `oteryn:runtime:synology-preprod` (§1);
    - `[platform].source_authority` is `platform`, equal to the Platform
      `GAME_AUTH_NATIVE_EVIDENCE_SOURCE_AUTHORITY`.
  - `deploy/synology-game/README.md`: the Platform section, names and runbook of §8. Its NAS
    values table takes `<WORLD_ID>`/`<CHANNEL_ID>` from the Registry-pinned manifest (D855), not
    from an owner-chosen UUIDv7, and takes `NODE_IDENTITY`, the platform endpoint and peer name,
    the readiness tokens and the epoch from the manifest too.
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
