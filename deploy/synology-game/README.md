# Synology Game preproduction deploy

`preproduction` is the only environment. `.github/workflows/synology-game-deploy.yml` deploys the native Game server to the internal Synology host. It runs only by `workflow_dispatch` from `main`, and the owner's approval of the GitHub environment `preproduction` is the human gate.

The workflow builds the Linux x86_64 binaries (`oteryn-game-server`, `oteryn-game-ops`, `oteryn-game-migrate`; the workflow installs only server and migrate) on a GitHub-hosted runner and passes them as an artifact; nothing is compiled on the NAS. On the `game-runners`/`oteryn-game` runner (`oteryn-synology-game`, runner user, no root, no sudo, no Docker) it installs them under `BASE`, runs `oteryn-game-migrate`, issues a launch authorization (`--supersedes <previous node id>`) before touching the service, restarts the single service, assigns the scope with `assignment replace` and waits for ready, both as root through the `oteryn-game-deploy-ops` wrapper and one sudoers rule.

`BASE=/volume1/oteryn/game-preprod` (root-owned; only its runtime subdirectories belong to the runner) and `ROOT_BASE=/volume1/oteryn/game-preprod-root` (root-owned, runner-unwritable).

`oteryn-game-ops` runs only as root (euid 0) and the runner is uid 1001, so the workflow never installs it. The owner installs and upgrades it, and its config, out of band under `ROOT_BASE`.

## One-time operator setup

Done once by the owner. The workflow never creates or changes it, and fails if it is missing. Commands marked **root** run as root on the NAS; **1001** run as the runner user.

1. **PostgreSQL 17 with TLS.** In Container Manager run a PostgreSQL 17 container with `ssl=on` and a server certificate whose name matches `tls_server_name`; its data volume is `BASE/db`. Create the database; the migration role is the schema owner, plus a control role in `oteryn_game_control` and a runtime role in `oteryn_game_runtime` (the role statements are in `tools/qualification/node_boot/run.sh`).
2. **Directory layout.**

   | Path | Owner | Mode | Content |
   | --- | --- | --- | --- |
   | `BASE` | root | 0755 | must stay root-owned: ops runs as root and rejects a non-root ancestor of `BASE/state` |
   | `BASE/bin`, `BASE/gameplay` | 1001 | 0755 | installed server and migrate binaries, `supervisor.sh`, staged content inputs |
   | `BASE/state` | root | 0755 (ancestors root too) | launch and S2 authorization files (ops hands each to uid 1001) |
   | `BASE/fence-parent` | root | 0755 | parent of the fence directory |
   | `BASE/fence-parent/fence` | 1001 | 0700 | Character fence directory |
   | `BASE/ops` | 1001 | 0700 | `migration-url` (one line, TLS-verified PostgreSQL URL), mode 0600 |
   | `BASE/node`, `BASE/node/secrets` | 1001 | 0700 | `node.toml` (0600), runtime `pg-password`, `db-ca.pem`, Platform and gameplay TLS files (each 0600) |
   | `BASE/run`, `BASE/log` | 1001 | 0700 | pid file, control socket, `current-node-id`, service log |
   | `BASE/db` | n/a | n/a | PostgreSQL volume |
   | `ROOT_BASE`, `ROOT_BASE/bin`, `ROOT_BASE/ops` | root | 0755 / 0755 / 0700 | `bin/oteryn-game-ops`, `ops/ops.toml` (0600), control `pg-password`, `db-ca.pem` |

3. **Config.** Copy `ops.toml.template` to `ROOT_BASE/ops/ops.toml` (root, 0600), `report.toml.template` to `ROOT_BASE/ops/report.toml` (root, 0600) and `node.toml.template` to `BASE/node/node.toml` (1001, 0600) and fill the placeholders (see "NAS values"). No secret is in the repository or the workflow.
4. **Ops binary (root).** Install `oteryn-game-ops` from a release bundle: `install -o root -g root -m 0755 oteryn-game-ops ROOT_BASE/bin/`. Upgrade it the same way. Each deploy prints `OPS_SHA256_INSTALLED` and `OPS_SHA256_BUNDLE` in its summary and **fails before any install or migration** if the installed ops binary or wrapper differs from the bundle, so upgrade both as root before deploying a new bundle; compare with `sha256sum ROOT_BASE/bin/oteryn-game-ops` against the `bin/oteryn-game-ops` of the bundle you trust (the artifact `game-preprod-bundle`, whose `SHA256SUMS` lists it).
5. **Wrapper and sudoers (root).** Install the wrapper from the repository (it is also in the bundle as `deploy-ops.sh`, and a mismatch fails the deploy): `install -o root -g root -m 0755 deploy/synology-game/deploy-ops.sh ROOT_BASE/bin/oteryn-game-deploy-ops`, and create `ROOT_BASE/ops/scope.env` (root, 0600) with the three lines `WORLD_ID=<uuid>`, `CHANNEL_ID=<uuid>` (the scope in `node.toml`) and `NODE_IDENTITY=<subject of the runtime-status certificate, RFC 4514, e.g. CN=oteryn-preprod-node>` (the value in `report.toml`). The wrapper takes only validated positional values (run id, attempt, node id, revision), fixes the binary, config, report config, node config, scope and node identity, and rejects extra arguments, so the runner cannot choose ops arguments. One file `/etc/sudoers.d/oteryn-game-deploy`, mode 0440, validated with `visudo -cf`:

   ```
   # runner uid 1001 may run the deploy wrapper's two subcommands (`#1001` is the uid, not a comment)
   #1001 ALL=(root) NOPASSWD: /volume1/oteryn/game-preprod-root/bin/oteryn-game-deploy-ops issue *, /volume1/oteryn/game-preprod-root/bin/oteryn-game-deploy-ops assign replace *
   ```

   `*` in sudoers matches spaces, so sudoers alone does not bound the arguments; the wrapper's strict regexes and argument counts do. DSM updates can reset `/etc/sudoers.d`; re-check with `sudo -n -l` as the runner user after each DSM update. The deploy fails early if `sudo -n -l` does not work. The wrapper does not set `OTERYN_NATIVE_GAMEPLAY_MANIFEST`: only `content activate` reads it, and the deploy never runs that.
6. **Supervision without root.** The workflow installs `supervisor.sh` to `BASE/bin/supervisor.sh`. It starts `oteryn-game-server serve --config BASE/node/node.toml` detached, records `BASE/run/node.pid`, logs to `BASE/log/node.log` and checks the pid's command line before signalling. After a NAS reboot start it from a DSM Task Scheduler boot-up task as the runner user, with `OTERYN_NATIVE_GAMEPLAY_MANIFEST=BASE/gameplay/content/spells.manifest.json` set, or by hand.

## NAS values

Known: DB `transport_ip` 172.30.187.2, `port` 5432, `tls_server_name` "oteryn-game-postgres", BASE `/volume1/oteryn/game-preprod`, runner uid 1001.

| Placeholder | Where the value comes from |
| --- | --- |
| `<BASE>`, `<ROOT_BASE>`, `<RUNNER_UID>`, `<DB_IP>`, `<DB_PORT>`, `<DB_TLS_SERVER_NAME>` | the known values above |
| `<DB_NAME>` | the database the owner created in step 1 |
| `<CONTROL_ROLE>`, `<RUNTIME_ROLE>` | the login roles created in step 1 (members of `oteryn_game_control` and `oteryn_game_runtime`); passwords go only into the `pg-password` files |
| `<WORLD_ID>`, `<CHANNEL_ID>` | owner-chosen distinct UUIDv7 values for the one preproduction World and Channel, e.g. `python3 -c 'import uuid; print(uuid.uuid7())'` (Python 3.14+) or `uuidgen -7` (util-linux 2.41+), run twice (`WorldId` and `ChannelId` stay distinct); the ops control role needs scope grants for exactly that pair (`game_control_scope_grants`, operations 1 to 4, as `run.sh` inserts) |
| `<LISTEN_ADDRESS>`, `<GAME_PORT>` | the NAS LAN address and the port the owner exposes for the Game client |
| gameplay `certificate_chain_file`, `private_key_file` | a TLS end-entity (not CA) certificate with `serverAuth` for the name clients use, placed by the owner as `BASE/node/secrets/gameplay.crt` and `gameplay.key` (0600) |
| `<READINESS_SOURCE_AUTHORITY>`, `<ROUTE_REVISION>`, `<RUNTIME_OBSERVATION_REVISION>`, `<RULESET_REVISION>`, `<CONTENT_REVISION>`, `<MAP_REVISION>`, `<WORLD_POLICY_REVISION>`, `<OFFER_REVISION>` | free-form revision tokens the node publishes in its readiness report; they must equal the values Platform pins for this scope. The agreed values come from the Platform preproduction topology (S3-B route and runtime-status contract); none is derivable from this repository. `run.sh` uses `nb-` style tokens only for its disposable run. UNKNOWN until Platform preprod exists |
| `<ASSIGNMENT_EPOCH>`, `<NODE_IDENTITY>` | the Platform-agreed ownership-authority epoch (non-zero) and the node host's runtime-status certificate subject. UNKNOWN until Platform preprod exists |
| `<PLATFORM_ENDPOINT>`, `<PLATFORM_PEER_NAME>` | Platform preproduction address (IP:port) and the TLS name on its certificate |
| `<UNIX_SECONDS_AT_SETUP>` | `date +%s` at setup; do not change it later without raising `descriptor_revision` (changed facts under an unchanged revision are refused at boot) |

### Platform

The Platform client certificate and key (`platform-client.crt`, `platform-client.key`), the node host's own runtime-status certificate and key (`runtime-status.crt`, `runtime-status.key`, a different identity from the evidence one; its subject is the `NODE_IDENTITY`) and the trust roots (`platform-roots.pem`) are files the owner places in `BASE/node/secrets`, mode 0600, owned by 1001. `oteryn-game-ops` loads the files named in `report.toml` as trusted input, which requires root ownership under a root-owned directory, so the owner also places root copies of `platform-roots.pem` and of the public `runtime-status.crt` in `ROOT_BASE/ops` (root, 0644; never the runtime-status key), and `report.toml` points at those copies. The ownership-authority certificate and key used by `ops` (`authority.crt`, `authority.key`) are files the owner places in `ROOT_BASE/ops`, root, 0600; the authority key must not share a public key with any node certificate. `node.toml` `[platform.runtime_status]` and `report.toml` carry file paths and the declared `assignment_epoch` (non-zero, the same in both), never key material. Nothing is fetched at deploy time. The node parses these files and the gameplay TLS pair at boot, so valid PEM files are required even before Platform exists. DERIVED from the code, not tested here: the node can start and register without a reachable Platform and wait in `awaiting_assignment`, but the scope assignment is reported to Platform, and character bootstrap and gameplay evidence need Platform, so reaching `ready` and real play are blocked until Platform preproduction exists and pins matching revisions.

## First start sequence

Run once, after steps 1 to 6. `ops` below is `sudo env OTERYN_NATIVE_GAMEPLAY_MANIFEST=BASE/gameplay/content/spells.manifest.json ROOT_BASE/bin/oteryn-game-ops --config ROOT_BASE/ops/ops.toml`; the owner (root) runs everything marked root directly; the deploy sudo rule covers only the wrapper's `issue` and `assign replace`.

1. (1001) Download the `game-preprod-bundle` artifact of a build on `main`, verify it with `sha256sum --check SHA256SUMS`, and copy `bin/oteryn-game-server`, `bin/oteryn-game-migrate` and `supervisor.sh` into `BASE/bin` and `gameplay/` into `BASE/gameplay`. (The deploy workflow cannot be the first start: the node needs the S2 file and the first authorization below before it can boot.)
2. (1001) `OTERYN_GAME_MIGRATION_DATABASE_URL="$(cat BASE/ops/migration-url)" BASE/bin/oteryn-game-migrate`
3. (root) `ops authorization issue --file launch-initial.json --binding preprod-initial` (no `--supersedes`; `node.toml` already names `BASE/state/launch-initial.json`)
4. (root) `ops s2 issue --node-config BASE/node/node.toml --file s2-fresh-store.json --namespace preproduction --authorization <owner-authorization-text>`
5. (root) `ops character fresh-store --request fresh-store.json` (run twice; the second run is idempotent)
6. (root) `ops character interpretation --profile <p> --ruleset <r> --content <c> --starter <s>` after the fresh store; the four tokens are the Platform-agreed interpretation revisions
7. (1001) `OTERYN_NATIVE_GAMEPLAY_MANIFEST=BASE/gameplay/content/spells.manifest.json BASE/bin/supervisor.sh start`, then `BASE/bin/supervisor.sh health` (it records `BASE/run/current-node-id`)
8. (root) grant the control role its scope rows (`game_control_scope_grants`), then `ops content activate --request content-1.json --world <WORLD_ID> --channel <CHANNEL_ID> --sequence 1 --previous empty`
9. (root, first start only) `ops assignment assign --report-config ROOT_BASE/ops/report.toml --node-config BASE/node/node.toml --node-identity <NODE_IDENTITY> --request assign-1.json --world <WORLD_ID> --channel <CHANNEL_ID> --node-id <node id from current-node-id> --revision <registration_revision from the awaiting_assignment log line>`
10. (1001) Record the activated gameplay inputs: `(cd BASE && find gameplay -type f -print0 | sort -z | xargs -0 sha256sum | sha256sum | cut -d' ' -f1 > run/activated-gameplay.sha256)`.

Steps 1 to 10 happen once by hand. Whenever the manifest or any pinned gameplay input changes, the owner first stages the new bundle's `gameplay/` outside `BASE/gameplay`, for example in `ROOT_BASE/staging/gameplay` (root-owned, from the verified `game-preprod-bundle` of the build), runs `ops content activate` against exactly those files (`OTERYN_NATIVE_GAMEPLAY_MANIFEST=ROOT_BASE/staging/gameplay/content/spells.manifest.json`, the next `--sequence`, the current one as `--previous`), and records the digest of the same staged files: `(cd ROOT_BASE/staging && find gameplay -type f -print0 | sort -z | xargs -0 sha256sum | sha256sum | cut -d' ' -f1) > BASE/run/activated-gameplay.sha256`. Then the deploy of that same bundle installs them. The deploy fails closed, before replacing any gameplay input, when the bundle's gameplay digest differs from that file.
 Every later deploy is fully automatic, with no manual root step: it migrates, issues the superseding authorization, restarts, waits for `awaiting_assignment`, runs `assignment replace` through the wrapper with the node id from `BASE/run/current-node-id` and the revision from the `awaiting_assignment` log line, then waits up to 120 s for `readiness ready=true` and fails if it does not come. Until Platform preproduction exists the final wait can fail (see Platform).

## Health check

`supervisor.sh health` waits up to 120 s for the process to stay alive and the log to show `awaiting_assignment` or `readiness ready=true`. `awaiting_assignment` is a reported state, not a failure.
