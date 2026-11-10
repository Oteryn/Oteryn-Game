# OTERYN Three.js 2D — first local connected-client implementation (#3099)

**Status: development implementation, not a production browser release.**
The native host has real connection/admission code, not a mocked game or an
HTTP gameplay server. Its Rust compilation and live-server operation have not
yet been verified. Do not describe the implementation as a successful live join.

## Audit baseline

Inspected `Oteryn/Oteryn-Game` at
`c255c6b88dff9467deae5dfc7cd0ff1a0c8fb75f` (2026-11-10).

| Area | Verified source / conclusion |
| --- | --- |
| Network | `tools/dev-client/src/lib.rs`, `crates/session`, `crates/session-tcp`: real TLS 1.3, ALPN `oteryn-game/1`, admission grants, snapshots and ordered native commands already exist. |
| Browser architecture | `docs/architecture/ADR-0018-browser-client-web-ready-boundaries.md` and `WEB_CLIENT_IMPLEMENTATION_PLAN.md` are proposed/planning documents. Their old no-networking description is not present runtime evidence. They do not register a browser transport. |
| Live map | `crates/session/src/map_transport.rs`: validated domain-17 snapshots/deltas. `apps/client/src/features/local-world-view/protocol.rs` handles delta coalescing and replacement. A full replacement must discard old tiles. |
| Map rendering | `apps/client/src/features/local-world-view/world.rs`: the server owns placements/visibility. Local assets supply pixels. Do not replace live tiles with an offline B3 map. |
| Offline world | `crates/client-assets/src/world.rs`: pinned B3/OTRB regions, 256-tile regions, 32-tile zstd sectors and bounded decoding. This adapter does not read offline placements. |
| Assets | `crates/client-assets`: hash-pinned 15.30 appearances/catalog/sprite cells, rooted at `OTERYN_ASSET_DIR`. The existing loader verifies the manifest. Missing appearances use native diagnostic glyphs, not invented textures. |
| Reuse | `apps/client/src/play/{session_adapter,session_loop,tick}.rs`, `scene.rs`, `crates/renderer/src/batch.rs`: `Session` implements `Stepper`; `PlayView` already supplies the map/entity/outfit scene and GPU-free quad/atlas data. |

## Implemented boundary

```text
local browser (Three.js orthographic sprite rendering + input)
    <-> bounded loopback presentation HTTP / native-process pipes
    <-> apps/client/examples/threejs_host (PlayView + run_session)
    <-> Session + session-tcp + protocol-oteryn
    <-> local Rust Game server (TLS 1.3, genuine admission required)
```

This is deliberately a **local adapter first**, not zero-install web gameplay.
The browser receives pixels, rectangles and UV coordinates, not an alternative
world model. `N/E/S/W` inputs enter the native chronological/paced input queue.
The same native code retains application framing, command IDs, revisions,
reconciliation, map replacement, visible entities and session failure handling.
No application-protocol JSON implementation, new transport profile, listener on
GameNode, Platform change, production binary, deployment or asset redistribution
is included. No changes to existing Cargo manifests, registries or gameplay code
are required: the Rust entry is a Cargo example using existing dependencies.

## Build / run (not verified in the authoring environment)

Use the workspace's pinned Rust toolchain, Node 22 or newer, and a local
qualification Game server. Do not point this at production: the native example
rejects non-loopback Game addresses. Supply an authorized, fresh development
admission grant for the chosen character, schema revision `1`, native
`oteryn_client::CLIENT_BUILD`, and TCP/TLS profile. Do not invent or reuse a grant.
A DER-encoded trust root and matching TLS server name are mandatory; certificate
and ALPN validation are not disabled.

```sh
# From the repository root:
cargo build --locked -p oteryn-client --example threejs_host
cargo test --locked -p oteryn-client --example threejs_host
cargo clippy --locked -p oteryn-client --example threejs_host -- -D warnings

cd tools/dev-client/web
npm install --ignore-scripts
npm test

# Operator supplies these values; credentials themselves never go in the URL.
# HOST_BINARY = absolute target/debug/examples/threejs_host[.exe] path
# GAME_ADDR = actual local Game listener, e.g. loopback IP plus its configured port
# TLS_NAME, ROOT_DER, GRANT_FILE = local qualification certificate/grant material
# CHARACTER_HEX = the selected UUIDv7 as exactly 32 hex digits, without hyphens
# OTERYN_ASSET_DIR = existing authorized asset root (not a download destination)
node bridge.mjs "$HOST_BINARY" "$GAME_ADDR" "$TLS_NAME" "$ROOT_DER" "$GRANT_FILE" "$CHARACTER_HEX"
```

Open the exact `http://127.0.0.1:<ephemeral-port>/` printed by the bridge. Do not
replace its host with `localhost` or forward the port: exact Host/Origin checks
are intentional. Arrow keys/WASD and direction buttons submit steps. `Rozłącz`
terminates the local native session; it does **not** claim a committed gameplay
logout. Refreshing does not obtain or replay credentials. After a closed session,
restart explicitly with valid qualification material.

Three.js is exactly pinned to `0.187.0` (npm package verified 2026-11-10). It is
served locally, not from a runtime CDN. Installation, dependency integrity
lockfile generation and license/supply-chain checks remain release gates; no
unverified registry hash has been invented. No original game assets are bundled.

## Presentation IPC and bounds

Pipe records start with a 4-byte unsigned little-endian body length. All integers
and float32 fields are little-endian. stdout is reserved exclusively for records.

* `OTAT | atlas_id:u32 | width:u32 | height:u32 | RGBA8`: at most 4096 by 4096,
  exact byte length, monotonically increasing nonzero atlas ID.
* `OTFR | frame_id:u32 | atlas_id:u32 | width:u32 | height:u32 | count:u32 |
  [x,y,width,height,u0,v0,u1,v1]:f32*8`: at most 81,920 quads, finite and bounded
  coordinates/UVs, exact length, increasing frame ID. Painter order is preserved.

There is one pending native frame, one queued input and one committed browser
scene plus at most one incoming atlas. A blocked output pipe does not block the
native session future. Intermediate presentation frames may be dropped; native
session events are never dropped by this adapter. Maximum atlas storage is
64 MiB per atlas copy; this is a development IPC path, not a bandwidth-optimized
production protocol. No WebSocket is presented as native transport profile 1.

The bridge binds only `127.0.0.1`, limits connections, checks Host and Origin,
requires an explicit same-origin view header, serves an exact static allowlist,
rejects oversized/chunked input, rate-limits input, and has no arbitrary path,
command, endpoint or credential HTTP API. It clears the scene after native exit,
malformed IPC or explicit disconnect. Browser blur/visibility loss stops held
input. Lack of browser activity ends the native host after the development idle
budget; stale pipe output also ends it. No automatic reconnect or grant retry.

## Checks actually executed during authoring

Node `v22.16.0`, isolated workspace, with no installed Three.js package:

```sh
npm test
node --check bridge.mjs
node --check public/client.mjs
node --check public/scene.mjs
```

The Node suite exercises real local HTTP handlers and the binary parsers, with
synthetic presentation records. It is **not** a Rust Game-server E2E test. The
initial test invocation failed because `bridge.mjs` did not yet exist; after
implementation the suite passes. The native encoder/parser unit tests are
provided but not claimed executed.

## Remaining gates / explicitly missing

1. Rust format, compile, clippy and example tests on this exact tree. The authoring
   container has no Rust toolchain and cannot resolve GitHub/npm registry access;
   no successful build, install or native play is claimed.
2. Install and lock the exact Three.js dependency, browser/WebGL2 visual test,
   reference screenshots, full-frame/pixel orientation and transparency parity.
3. Real local-server admission, map/step/collision/floor/entity update and
   disconnect tests with authorized fixtures. No live listener/grant was used.
4. Native lighting, per-sprite opacity and complete UI/effects/audio parity are
   not implemented in the Three.js renderer. This slice exposes movement only;
   inventory, chat, combat, NPC/quest controls and reconnect UI are not yet exposed.
5. Zero-install browser distribution: accepted transport/admission/Platform design,
   registered browser profile or equivalent reviewed topology, browser asset
   delivery rights/custody, cache integrity and security/performance qualification.
6. Independent Codex review and repository CI before merging. Keep the PR draft
   until the blocking build, visual and real-server checks have evidence.
