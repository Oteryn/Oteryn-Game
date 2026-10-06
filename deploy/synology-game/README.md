# Synology Game preproduction deploy

`preproduction` is the only environment. `.github/workflows/synology-game-deploy.yml` deploys the native Game server to the internal Synology host. It runs only by `workflow_dispatch` from `main`, and the owner's approval of the GitHub environment `preproduction` is the human gate.

The workflow builds the Linux x86_64 binaries (`oteryn-game-server`, `oteryn-game-ops`, `oteryn-game-migrate`) on a GitHub-hosted runner and passes them as an artifact; nothing is compiled on the NAS. On the `game-runners`/`oteryn-game` runner (`oteryn-synology-game`, runner user, no root, no sudo, no Docker) it installs them under `BASE`, runs `oteryn-game-migrate`, issues a launch authorization with `--supersedes <previous node id>`, restarts the single service and health-checks it.

`BASE=/volume1/oteryn/game-preprod`

## One-time operator setup

Everything here is done once by the owner. The workflow never creates or changes it, and it fails if it is missing.

1. **PostgreSQL 17 with TLS.** In Container Manager run a PostgreSQL 17 container with `ssl=on` and a server certificate whose name matches `tls_server_name`. Its data volume is `BASE/db`. Create the database, the migration role (owner of the schema), a control role in `oteryn_game_control` and a runtime role in `oteryn_game_runtime` (see `tools/qualification/node_boot/run.sh` for the role statements).
2. **Directory layout**, all owned by the runner user:

   | Path | Mode | Content |
   | --- | --- | --- |
   | `BASE/bin` | 0755 | installed binaries |
   | `BASE/gameplay` | 0755 | staged hash-bound content inputs |
   | `BASE/state` | 0700 | launch authorization files, `current-node-id`, S2 file |
   | `BASE/fence-parent/fence` | 0700 | Character fence directory |
   | `BASE/ops` | 0700 | `ops.toml`, `migration-url`, control `pg-password`, `db-ca.pem` |
   | `BASE/node`, `BASE/node/secrets` | 0700 | `node.toml`, runtime `pg-password`, `db-ca.pem`, Platform and gameplay TLS files |
   | `BASE/run`, `BASE/log` | 0700 | pid file, control socket, service log |
   | `BASE/db` | n/a | PostgreSQL volume |

3. **Config files.** Copy `ops.toml.template` to `BASE/ops/ops.toml` and `node.toml.template` to `BASE/node/node.toml`, replace every `<PLACEHOLDER>`, and set mode 0600 owned by the runner user. Put the migration database URL (a single line, `postgresql://...` with TLS verification) in `BASE/ops/migration-url`, mode 0600. Secrets stay on the NAS in these files; none is in the repository or the workflow.
4. **First launch.** Issue the first authorization and S2 file with `oteryn-game-ops`, run the fresh-store and interpretation steps and assign the scope as `run.sh` stages `operator_setup` and `node_assigned_ready` do. The first deploy has no `current-node-id`, so it issues without `--supersedes`; later deploys supersede the node recorded there.
5. **Supervision without root.** The workflow installs `supervisor.sh` to `BASE/supervisor.sh`. It starts `oteryn-game-server serve --config BASE/node/node.toml` detached, records `BASE/run/node.pid`, logs to `BASE/log/node.log`, and verifies the pid's command line before signalling it. The workflow stops and restarts it on each deploy. After a NAS reboot the owner runs `BASE/supervisor.sh start` (with `OTERYN_NATIVE_GAMEPLAY_MANIFEST=BASE/gameplay/content/spells.manifest.json` set) from a DSM Task Scheduler "boot-up" task running as the runner user, or by hand.

## Health check

`supervisor.sh health` waits up to 120 s for the process to stay alive and the log to show `awaiting_assignment` or `readiness ready=true`. After a superseding restart the node waits for the operator's scope assignment (`assignment replace`); that is a reported `awaiting_assignment` state, not a failure.
