# RUNBOOK-1: local native login (Platform -> Gateway -> game node)

One local run of the whole native login path (CP D825, packet §2.7 of
`docs/architecture/reviews/OTERYN_GAME_ARCH_LOGIN_FIRST_PACKETS_2026-10-05.md`). It extends the
`tools/qualification/node_boot` topology and is run by the RUNBOOK-1 operator (owner decision D824 2a).
It is not yet invoked by any CI workflow; wiring it into the node-boot job is a follow-up outside this
directory (gate workflows). Secrets are generated per run, kept in a 0700 work
directory and never committed.

```text
OTERYN_PLATFORM_URL ─ OAuth code+PKCE (game:ticket) ─> Platform (APP_ENV=preproduction)
client ─ POST /v1/login (protocol_version 2) ─> Go gateway ─ service token ─> Platform admission issuer
client ─ oteryn-session-tcp, TLS 1.3 against the dev root ─> game node ─ Session::admit ─> join snapshot ─> step
game node ─ runtime status (own mTLS identity) ─> Platform      ops assignment assign ─ ReportScopeAssignmentV1 ─> Platform
game node ─ Character projection (own mTLS identity) ─> Platform (ownership read model; mode 33a off)
```

## Run

```bash
# Platform main carrying U1 (>= 71bbe6c) and #1472, exactly the pinned SHA in run.sh:
git clone https://github.com/Oteryn/Oteryn-Platform _platform && git -C _platform checkout b18d32d30c4c4e496330077d12b37db3f0f29011
bash tools/qualification/login_local/run.sh
```

Needs Docker, `openssl`, `sudo`, Rust 1.94.0 and a display for the OAuth browser step. Ports:
`LOGIN_LOCAL_PLATFORM_MTLS_PORT` (18563), `_PLATFORM_HTTP_PORT` (18564), `_GATEWAY_PORT` (18565),
`_GAME_PORT` (17281), `_PG_PORT` (15533). `LOGIN_LOCAL_RUN_CLIENT=0` stops at READY; `LOGIN_LOCAL_HOLD=1`
keeps the services up for a Windows client; `LOGIN_LOCAL_KEEP=1` skips teardown (the work directory
then holds per-run secrets).

**Hosts.** By default every port is bound to loopback, so the full client walk needs the Windows client on the
same host as the services (on a Windows PC run the script under WSL2 with Docker). To serve from another machine
on the same LAN, such as a Synology NAS, set `LOGIN_LOCAL_HOST` (next section).

## NAS server + PC client

`LOGIN_LOCAL_HOST=<private LAN IPv4>` (default `127.0.0.1`, which keeps the loopback behaviour unchanged). Only
10/8, 172.16/12 and 192.168/16 are accepted; `0.0.0.0`, public and malformed values end `BLOCKED`.

The client (`crates/platform-client/src/lib.rs`, `PlatformClientConfig::new`) accepts plain `http` only for
`localhost`, `127.0.0.1` and `[::1]`; any other host must be `https`. So in LAN mode nginx serves the Platform and the
gateway over TLS on the LAN IP with a per-run LAN test CA (leaf SAN `IP:<host>`), and `client.env` carries `https://`
URLs. The OAuth redirect needs nothing extra: the client's loopback redirect listener and the browser both run on the
PC, and the browser opens `OTERYN_PLATFORM_URL`. The service-token admission route stays unreachable from the LAN.

On the NAS (Docker, repo and the pinned `_platform` checkout as above):

```bash
LOGIN_LOCAL_HOST=<NAS LAN IP> LOGIN_LOCAL_HOLD=1 LOGIN_LOCAL_RUN_CLIENT=0 bash tools/qualification/login_local/run.sh
```

What changes with a LAN host: the Platform HTTP port (18564) and gateway port (18565) are published by nginx over
TLS on the LAN IP; the game node listens on `<LAN IP>:17281` (default `_GAME_PORT`); the gameplay certificate SAN is
`DNS:localhost,IP:<host>`; the route and `game-auth:world:ensure` use the LAN IP; the route `tls_server_name` stays
`localhost`. PostgreSQL (15533), the Platform mTLS port (18563), the game node's database and control connections, and
the report endpoint stay on `127.0.0.1` (they are node-local).

Then copy to the PC (all three are written next to `client.env`, normally the work directory, kept because of
`LOGIN_LOCAL_HOLD=1`):

1. `client.env` (contains the per-run test password in a comment).
2. `login-local-lan-ca.crt`: import into the Windows **Trusted Root Certification Authorities** store (it signs only
   this run's Platform/gateway leaf and lives one day). The client verifies through the OS trust store.
3. `login-local-gameplay.crt`: set `OTERYN_DEV_ROOT` in the PC's environment to its Windows path (in LAN mode `client.env` holds
   the NAS path, which does not exist on the PC, and no Windows path is needed on the NAS).

Load `client.env` into the PC shell and run the client there; after the server step result is visible write text
containing the run id to the attestation file on the NAS (see Result).

**Firewall.** Open only to the LAN (the NAS firewall source `192.168.x.0/24` or your subnet) the TCP ports 18564
(Platform), 18565 (gateway) and 17281 (game node). Do not open or forward PostgreSQL 15533 or 18563. **Do not forward
any of these ports on the router**: the test account password is in `client.env` and the run is a disposable
qualification topology, not a hardened service. Stop the run (Ctrl-C) when done.

**Unverified here.** No Docker daemon with the pinned Platform was available for a LAN run: the Platform's use of the
request host for its own URLs behind the TLS listener, and the Platform's acceptance of the route host, are
confirmed only by the first real run.

## NAS + DSM reverse proxy

Variant of the section above when the NAS already has a TLS reverse proxy (Synology DSM): the Platform and the gateway
are reached through DSM and only the game node is reached directly on the LAN. Set both
`LOGIN_LOCAL_PUBLIC_PLATFORM_URL` and `LOGIN_LOCAL_PUBLIC_GATEWAY_URL` (each `https://host[:port]`, nothing else) and
`LOGIN_LOCAL_HOST` (required). Anything else ends `BLOCKED` (`public_url_pair_required`, `public_url_not_https`,
`host_required_for_public_urls`). No LAN listener and no LAN CA are created; Platform HTTP and the gateway stay published
on `127.0.0.1:18564` / `127.0.0.1:18565`, and the run does not probe the public URLs, so DSM can be configured after the
run starts.

DSM, Control Panel -> Login Portal -> Advanced -> Reverse Proxy, two rows (source HTTPS, destination HTTP, `127.0.0.1`):

| Source | Destination |
| --- | --- |
| `https://synology:18574` | `http://127.0.0.1:18564` (Platform) |
| `https://synology:18575` | `http://127.0.0.1:18565` (gateway) |

DSM terminates TLS with its own certificate. The PC must trust it: if it is self-signed, export it from DSM (Control
Panel -> Security -> Certificate) and import it into the Windows Trusted Root Certification Authorities store; the
PC must also resolve the proxy host name (`synology`) to the NAS.

On the NAS:

```bash
LOGIN_LOCAL_HOST=192.168.1.2 LOGIN_LOCAL_PUBLIC_PLATFORM_URL=https://synology:18574 LOGIN_LOCAL_PUBLIC_GATEWAY_URL=https://synology:18575 LOGIN_LOCAL_HOLD=1 LOGIN_LOCAL_RUN_CLIENT=0 LOGIN_LOCAL_DB_START_PERIOD=600s bash tools/qualification/login_local/run.sh
```

What changes in proxy mode: `client.env` `OTERYN_PLATFORM_URL` / `OTERYN_GATEWAY_URL` are the public URLs; the Platform's
`APP_URL` is the public Platform URL, and the loopback Platform listener (nginx `8447`) presents that authority over
https to PHP (`HTTPS on`, `HTTP_HOST`, `SERVER_PORT`; `X-Forwarded-*` headers are blanked) so Laravel generates the public
https issuer, authorize and redirect URLs without trusted-proxy configuration. The gateway's call to the Platform
(`https://nginx:8444`) is unchanged. The game node still listens on `<LOGIN_LOCAL_HOST>:17281` with `IP:<host>` in the
gameplay SAN and the LAN IP as route host (`tls_server_name` stays `localhost`).

Copy only `client.env` and `login-local-gameplay.crt` to the PC and point `OTERYN_DEV_ROOT` at the certificate there.
Firewall: allow TCP 17281 from the LAN only; DSM's own 18574 / 18575 follow your DSM firewall policy. Do not forward
any of these ports on the router, and do not publish PostgreSQL (15533) or 18563.

**Slow hosts.** The MariaDB service gets a `start_period` of `300s` from `compose.override.yml` (the
`wp5_s3a` healthcheck alone allows about 2 minutes of first initialisation, which a NAS can exceed and
ends `dependency failed to start: ... db-1 is unhealthy`). Override it with
`LOGIN_LOCAL_DB_START_PERIOD=600s bash tools/qualification/login_local/run.sh`.

## What it does

1. Blocks (`BLOCKED reason=...`) without Docker, the pinned Platform checkout, or PostgreSQL 17.6.
2. Generates per-run PKI: four distinct mTLS identities with distinct keys (native evidence/Character
   intent, node-host runtime status `CN=oteryn-game-node-runtime-status`, ownership authority
   `CN=oteryn-game-ops`, Character projection `CN=oteryn-game-character-projection`), the gameplay dev
   root, the Ed25519 admission issuer key, and a test CA plus an nginx server certificate (SAN `nginx`) for
   the gateway's Platform upstream. Nothing is committed; all of it lives in the work directory and is
   removed on exit.
3. Starts Platform with `APP_ENV=preproduction`, mode 33a off
   (`GAME_AUTH_NATIVE_ADMISSION_UNVERIFIED_CHARACTER_OWNERSHIP=false`), the Character projection feed on
   (`GAME_AUTH_NATIVE_ACCOUNT_CHARACTERS_ENABLED=true`, the projection identity list and
   `source_authority`), and the issuer key file (0600, `www-data`); publishes its public key with
   `publishTrustedKey`. Ownership is checked from the projection (PLATFORM-LCFA-1, contract §5.4).
4. Issues the topology (`issueForPreproduction`) on a retained per-run SQLite fixture, because Platform's
   `isolatedConnection()` refuses MySQL outside `APP_ENV=testing` (the fixture lives in the 0700 work directory,
   is bind-mounted at `/tmp/oteryn-native-topology-<hex>/` and its world/channel rows are mirrored into the
   Platform database), and reads back the uuid7 WorldId/ChannelId, publishes the
   route (`publishRouteForPreproduction`, `native_login_enabled=true`, `tls_server_name=localhost`) and
   writes its `rt.<version>.<digest>` into the node's `route_revision`.
5. Runs the Go gateway with `GATEWAY_NATIVE_LOGIN_ENABLED=true` and the hashed/plain service token pair.
   The gateway accepts `http` only for loopback hosts, so it calls Platform at `https://nginx:8444`
   (in-network TLS listener, not published; only the native-admissions route) and trusts only the per-run
   test CA, mounted read-only as `SSL_CERT_FILE`, with `SSL_CERT_DIR` set to an empty read-only directory so Go
   loads no system roots (the run checks both before starting the gateway).
6. Creates the projection epoch fence `/srv/oteryn-login-local/projection/epoch-fence` with value 0 (owned by
   the node service user, outside the Character fence directory), runs
   `ops projection resync --raise-epoch true` under the ops `[projection]` credential (the local PostgreSQL
   admin login: migration 0024 grants the resync function to no runtime or control role), and checks the
   printed epoch was persisted as F. It then runs one node with `[platform.runtime_status]`,
   `assignment_epoch = 1` (declared, never raised by ops) and `[platform.account_characters]`, then
   `ops assignment assign --node-config --report-config`, which must print `report=ReportScopeAssignmentV1`.
7. Creates the test account, ensures the OAuth client, bootstraps one Character from a real Platform intent,
   and waits until the node logs an accepted `PublishAccountCharactersV1` and `PublishProjectionWatermarkV1`.
8. Writes `client.env` (0600; includes the per-run test password in a comment) and runs the client with
   `OTERYN_PLATFORM_URL`, `OTERYN_GATEWAY_URL`, `OTERYN_OAUTH_CLIENT_ID`, `OTERYN_WORLD`,
   `OTERYN_CHARACTER_ID` and `OTERYN_DEV_ROOT`.

## Result

`LOGIN_LOCAL_RESULT=` is `BLOCKED`, `FAIL`, `READY` (services up, environment written), `ADMITTED`
(the client printed `Oteryn: admitted to World ...`) or `WALKED`.

`WALKED` is only emitted from a post-READY operator attestation tied to the current run. **Gap:** the walk happens in
the Windows shell, which has no machine-readable step output, and the node logs no per-step event, so `WALKED`
cannot be derived automatically. The non-Windows client binary stops at admission (`ADMITTED`). For the walk, run
with `LOGIN_LOCAL_HOLD=1 LOGIN_LOCAL_RUN_CLIENT=0` and use `client.env` on Windows (the dev root and ports are
loopback, so run the node and client on the same host). `client.env` carries a Windows-readable
`OTERYN_DEV_ROOT` (`wslpath -w` under WSL; otherwise set `LOGIN_LOCAL_DEV_ROOT_WINDOWS` to a writable path the
certificate is copied to, plus `LOGIN_LOCAL_DEV_ROOT_WINDOWS_AS_SEEN` if Windows sees it under another name; without
either the run is `BLOCKED`). After seeing the server step result, write text containing the run id printed in the
`holding services` line to `LOGIN_LOCAL_WALKED_ATTEST_FILE` (default `<work dir>/walked.attest`); the file is cleared
before READY, and only text containing this run's id ends the run `WALKED`.
With `LOGIN_LOCAL_RUN_CLIENT=1` a client that does not print the admission line ends `FAIL` (exit 1).

## Platform commands used (Platform main, no Platform change)

The projection feed needs a Platform checkout carrying PLATFORM-LCFA-1 (Platform#1465); until the pin above
moves to such a commit, the run fails at the projection wait or at ticket issuance.

`game-auth:world:ensure`, `NativeTopologyRegistry::issueForPreproduction` / `publishRouteForPreproduction`,
`NativeSigningTrustRegistry::publishTrustedKey` (via `php -r` inside the throwaway container, as node_boot does),
`game-auth:oauth-client:ensure`, `game-auth:character-bootstrap-intent:issue`. No step is pending
PLATFORM-NATIVE-PREPROD-OPS-1. The account is inserted into `identities` by the container's `php -r` (local-only route; no
operator command creates accounts).

## Validation record

`bash -n` passes on `run.sh`; `shellcheck` was not available when LOGIN-LOCAL-LAN-1 was written. LOGIN-LOCAL-LAN-1: host validation was exercised
for loopback, the three private ranges and rejected values; `docker compose config` with and without `compose.lan.yml` merges as intended
(default publishes unchanged); a leaf with `IP:` SAN verifies with `openssl verify -verify_ip`. `bash -n` and `shellcheck -x` passed on `run.sh` earlier. LOGIN-LOCAL-TLS-1 (#1914): the gateway upstream certificate chain
generated by `run.sh` verifies with `openssl verify -purpose sslserver -verify_hostname nginx`; the TLS run itself was not
executed here either (Docker daemon unreachable). The documented local run was **not executed in this environment**: the Docker
daemon is unreachable. The first operator/CI run is the first executed evidence; the OAuth client-id parse, the
`character_id` read-back from `game_character_roots` and the browser sign-in step are the parts to confirm there.
