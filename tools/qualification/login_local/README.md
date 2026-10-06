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
```

## Run

```bash
# Platform main carrying U1 (>= 71bbe6c), exactly the pinned SHA in run.sh:
git clone https://github.com/Oteryn/Oteryn-Platform _platform && git -C _platform checkout 3896bcdf75a511f1e386ac645303eaf8f234ffcf
bash tools/qualification/login_local/run.sh
```

Needs Docker, `openssl`, `sudo`, Rust 1.94.0 and a display for the OAuth browser step. Ports:
`LOGIN_LOCAL_PLATFORM_MTLS_PORT` (18563), `_PLATFORM_HTTP_PORT` (18564), `_GATEWAY_PORT` (18565),
`_GAME_PORT` (17281), `_PG_PORT` (15533). `LOGIN_LOCAL_RUN_CLIENT=0` stops at READY; `LOGIN_LOCAL_HOLD=1`
keeps the services up for a Windows client; `LOGIN_LOCAL_KEEP=1` skips teardown (the work directory
then holds per-run secrets).

## What it does

1. Blocks (`BLOCKED reason=...`) without Docker, the pinned Platform checkout, or PostgreSQL 17.6.
2. Generates per-run PKI: four distinct mTLS identities with distinct keys (native evidence/Character
   intent, node-host runtime status `CN=oteryn-game-node-runtime-status`, ownership authority
   `CN=oteryn-game-ops`), the gameplay dev root, and the Ed25519 admission issuer key.
3. Starts Platform with `APP_ENV=preproduction`, mode 33a
   (`GAME_AUTH_NATIVE_ADMISSION_UNVERIFIED_CHARACTER_OWNERSHIP=true`, world id = the node's world) and
   the issuer key file (0600, `www-data`); publishes its public key with `publishTrustedKey`.
4. Issues the topology (`issueForPreproduction`) and reads back the uuid7 WorldId/ChannelId, publishes the
   route (`publishRouteForPreproduction`, `native_login_enabled=true`, `tls_server_name=localhost`) and
   writes its `rt.<version>.<digest>` into the node's `route_revision`.
5. Runs the Go gateway with `GATEWAY_NATIVE_LOGIN_ENABLED=true` and the hashed/plain service token pair.
6. Runs one node with `[platform.runtime_status]` and `assignment_epoch = 1` (declared, never raised by ops),
   then `ops assignment assign --node-config --report-config`, which must print `report=ReportScopeAssignmentV1`.
7. Creates the test account, ensures the OAuth client, bootstraps one Character from a real Platform intent.
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

`game-auth:world:ensure`, `NativeTopologyRegistry::issueForPreproduction` / `publishRouteForPreproduction`,
`NativeSigningTrustRegistry::publishTrustedKey` (via `php -r` inside the throwaway container, as node_boot does),
`game-auth:oauth-client:ensure`, `game-auth:character-bootstrap-intent:issue`. No step is pending
PLATFORM-NATIVE-PREPROD-OPS-1. The account is inserted into `identities` by the container's `php -r` (local-only route; no
operator command creates accounts).

## Validation record

`bash -n` and `shellcheck -x` pass on `run.sh`. The documented local run was **not executed in this environment**: the Docker
daemon is unreachable. The first operator/CI run is the first executed evidence; the OAuth client-id parse, the
`character_id` read-back from `game_character_roots` and the browser sign-in step are the parts to confirm there.
