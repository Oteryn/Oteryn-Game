# OTERYN Three.js 2D — connected-client implementation slice (#3099 / PR #3100)

**PARTIAL: local development adapter, not a production or zero-install browser client.**
The Rust example contains real TLS/admission calls, but successful Rust compilation,
Game-server admission and WebGL rendering have not yet been demonstrated. Passing
presentation tests must not be reported as a successful connection to the game.

## Audit baseline

Inspected `Oteryn/Oteryn-Game` at
`c255c6b88dff9467deae5dfc7cd0ff1a0c8fb75f` (2026-11-10).
Continuation fresh-read found the same `main` and PR #3100 at
`7a7d8e64e3849dedb132a14a5b8417b59f75b007`; work continues on the same branch.

| Area | Verified source / conclusion |
| --- | --- |
| Network | `tools/dev-client/src/lib.rs`, `crates/session`, `crates/session-tcp`: native TLS 1.3, ALPN `oteryn-game/1`, grants, snapshots and ordered commands already exist. |
| Browser architecture | `docs/architecture/ADR-0018-browser-client-web-ready-boundaries.md` and `WEB_CLIENT_IMPLEMENTATION_PLAN.md` are proposed/planning documents, not browser transport registration. Their historical no-networking description is not current runtime evidence. |
| Live map | `crates/session/src/map_transport.rs` validates domain 17. `apps/client/src/features/local-world-view/protocol.rs` handles deltas and complete replacement, discarding superseded tiles. |
| Map rendering | `apps/client/src/features/local-world-view/world.rs`: the server owns placements and visibility. Local assets supply pixels, not substitute offline placements. |
| Offline world | `crates/client-assets/src/world.rs`: pinned B3/OTRB regions, 256-tile regions and 32-tile zstd sectors, with bounded decoding. This adapter does not load offline map placements. |
| Assets | `crates/client-assets`: hash-pinned 15.30 appearances/catalog/sprite cells, rooted at `OTERYN_ASSET_DIR`. Missing appearances use native diagnostic glyphs. No original game assets are bundled here. |
| Shared client | `apps/client/src/play/{session_adapter,session_loop,tick}.rs`, `scene.rs`, `crates/renderer/src/batch.rs`: reuse `Session`, `PlayView`, map/entity/outfit projection and GPU-free quad/atlas data. |

## Implemented boundary

```text
local browser: Three.js orthographic sprite renderer + cardinal input
    <-> loopback presentation HTTP / bounded native-process pipes
    <-> apps/client/examples/threejs_host: PlayView + run_session
    <-> Session + session-tcp + protocol-oteryn
    <-> local Rust Game server: TLS 1.3, genuine admission required
```

The browser receives RGBA pixels, rectangles and UV coordinates, not a duplicate
world simulation. `N/E/S/W` input enters the native paced input queue. Rust keeps
application framing, command IDs, revisions, reconciliation and map replacement.
There is no new GameNode listener, transport-profile registration, gameplay JSON
API, Platform mutation, production deployment or change to the native client.
The opt-in Rust executable is a Cargo example using existing dependencies.

**Startup is fail-closed.** The bridge verifies local static files and the exact
installed Three.js version before it can start the native host. It then waits
for an empty same-origin `/connect` signal from the page, sent only after Three.js
loads and WebGL2 initializes. A reload attaches to the same active process; a
failed or disconnected process never automatically restarts or retries a grant.
A renderer-ready signal is not Game admission: input stays disabled until the
native host supplies a complete validated presentation frame/atlas pair.

## Build and run

Use the workspace's pinned Rust toolchain, Node 22 or newer, and an authorized
local qualification Game server. Native Game address and browser listener are
loopback-only. A DER trust root and matching TLS name are mandatory; certificate
and ALPN checks are not disabled. Supply a fresh development grant for the chosen
character, schema revision `1`, `oteryn_client::CLIENT_BUILD`, and TCP/TLS profile.
Do not invent/reuse a grant or point this tool at production.

```sh
# From repository root; these Rust commands still require execution evidence:
cargo build --locked -p oteryn-client --example threejs_host
cargo test --locked -p oteryn-client --example threejs_host
cargo clippy --locked -p oteryn-client --example threejs_host -- -D warnings

cd tools/dev-client/web
npm ci --ignore-scripts
npm test

# Operator-supplied paths/settings, not credentials in a URL:
# HOST_BINARY: absolute target/debug/examples/threejs_host[.exe] path
# GAME_ADDR: actual configured loopback Game listener
# TLS_NAME / ROOT_DER / GRANT_FILE: authorized local qualification material
# CHARACTER_HEX: selected UUIDv7 as 32 hex digits, without hyphens
# OTERYN_ASSET_DIR: existing authorized asset root, inherited by the host
node bridge.mjs "$HOST_BINARY" "$GAME_ADDR" "$TLS_NAME" "$ROOT_DER" "$GRANT_FILE" "$CHARACTER_HEX"
```

Open the exact `http://127.0.0.1:<ephemeral-port>/` printed by the bridge. Do not
replace the host with `localhost` or forward the port: exact Host/Origin checks
are intentional. Arrow keys/WASD and buttons submit movement. `Rozłącz` closes
the transport; it does **not** claim a committed gameplay logout. After a terminal
session, restart explicitly with valid qualification material.

Three.js is pinned to `0.187.0` and served locally, not from a runtime CDN.
`package-lock.json` was generated by npm from the version, tarball and SHA-512
integrity returned by the official npm registry on 2026-11-10:
`https://registry.npmjs.org/three/latest` (response version was exactly `0.187.0`).
Because the authoring container cannot reach the registry, those observed metadata
fields were supplied to a temporary loopback metadata server for
`npm install --package-lock-only --ignore-scripts --no-audit --no-fund`.
A subsequent offline lock-only npm run succeeded. **No tarball was downloaded or
installed by this procedure**; it is not installation or supply-chain audit proof.

## Presentation IPC and bounds

Each pipe record has a 4-byte unsigned little-endian body length. All integers
and float32 fields are little-endian; stdout is reserved for records.

* `OTAT | atlas_id:u32 | width:u32 | height:u32 | RGBA8`: nonzero increasing ID,
  at most 4096 by 4096, exact body size.
* `OTFR | frame_id:u32 | atlas_id:u32 | width:u32 | height:u32 | count:u32 |
  [x,y,width,height,u0,v0,u1,v1]:f32*8`: at most 81,920 quads, finite bounded
  coordinates/UVs, exact size, increasing frame ID, matching atlas. Painter order
  is retained. Magic bytes are exact; high-bit-corrupted aliases are rejected.

There is one pending native frame/input and one committed browser scene plus at
most one incoming atlas. Blocked stdout does not block the native session future.
Presentation frames may be dropped, not native session events. Atlas copies may
reach 64 MiB each; this is not a bandwidth-optimized production transport.

HTTP binds only `127.0.0.1`, checks Host/Origin/view headers, serves an exact static
allowlist, limits connections/body size/input rate, and exposes no arbitrary
endpoint, executable, path or credential API. Input is rechecked after body reads
so a concurrent disconnect cannot dispatch a stale request. Native exit, invalid
IPC and explicit disconnect clear retained scenes. Blur/visibility loss stops held
keys. Inactive views, stalled startup and stale pipe output close the native host.

## Verification actually executed

In an isolated Node `v22.16.0` workspace, with the inspected file bytes checked
against GitHub blob hashes:

- Existing baseline: `npm test` — 10 passed, 0 failed.
- Added lifecycle suite: `npm test` — **16 passed, 0 failed** in the repaired tree.
- Focused baseline regression replay: corrupt magic and late input after disconnect
  both fail against the original bridge (exit 1), then pass with the repair.
- `node --check` for bridge, browser modules and tests; `git diff --check`.
- npm-generated lockfile, then a successful offline lock-only npm validation.

The new tests exercise actual loopback HTTP and a spawned child process: delayed
activation, input-to-pipe-to-frame delivery, idempotent activation, terminal close,
missing dependencies/wrong installed version, missing executable and malformed IPC.
**The child and static files are explicitly synthetic fixtures, not Rust Game or
Three.js. These are process/HTTP integration tests, not live-game or GPU tests.**

GitHub run `42895563921`, job `130108489322`, on the original head failed
`cargo fmt` in `presentation.rs`. This batch applies the formatter's exact diff.
That source change alone is not a passing formatting/build claim; fresh CI remains
required. No CI workflow, protection, test assertion or protocol rule was weakened.

## Remaining gates

1. Successful exact-head Rust format/build/clippy/example-test evidence. The local
   authoring environment has no Rust toolchain; GitHub is the validation route.
2. `npm ci` with the locked package and actual browser/WebGL2 visual checks,
   including UV orientation, transparency and reference screenshots.
3. Genuine local Game admission, server map/movement/collision/floor/entity updates
   and disconnection using authorized fixtures. No live endpoint/grant was used
   during this authoring session.
4. Full client UI: inventory, chat, combat, NPC/quest controls and reconnect/login.
   This slice exposes movement only. Native lighting/per-sprite opacity/effects/
   audio parity are not implemented in the Three.js renderer.
5. Zero-install distribution needs reviewed browser transport/admission/Platform
   integration, asset-distribution authority, integrity/cache and performance tests.
6. Independent Codex review and required repository checks before merging. Keep
   the PR draft until the build, visual and real-server evidence exists.
