# Rust Client Atomic Cutover Transformation Evidence

- Coordination ID: `OTV2-RUST-CLIENT-CUTOVER-20260806`
- Source: `blakinio/otclient@c923ad8a1dff17b4933a6110931b0823cec2c590`
- Source subtree tree: `c0928dafca6df19ff11d7901e503ed85a5199439`
- Destination branch: `migrate/rust-client-cutover-c923ad8`
- Contract authority: accepted FND-01 and merged VSL-02

## Resulting workspace

The cutover creates exactly the nineteen FND-01 members in one pull request. There is one root lockfile and one machine-readable dependency-boundary policy. The production app and the synthetic harness have separate dependency closures.

The production closure contains only:

```text
apps/client
foundation
diagnostics
client-runtime
platform-contracts
platform-client
identity
input-actions
input-platform
renderer
```

It does not reach client-domain, client-simulation, synthetic-assets, test-support, the harness, Canary, protocol-core, transport, Game Session or a placeholder native protocol.

## Source-member transformation matrix

| Source member | Disposition | Destination result |
|---|---|---|
| `apps/client` | REWRITE | visible Windows pre-native shell; no gameplay route, credential or protocol |
| `account-session` | MERGE | safe account context values in `platform-contracts` |
| `app-runtime` | MIGRATE_AND_RENAME | application-owned Tokio runtime in `client-runtime` |
| `asset-decode` | MERGE | bounded synthetic RGBA validation |
| `asset-types` | MERGE | synthetic-v1 schema only |
| `asset-runtime` | MERGE | generation-fenced synthetic runtime |
| `diagnostics` | MIGRATE_AS_IS | structured diagnostics retained with provisional client generation names |
| `foundation` | REWRITE | clocks, deadlines, cancellation and technical generations retained; authoritative-looking source names removed |
| `game-domain` | MIGRATE_AND_RENAME | explicit non-authoritative `client-domain` projection |
| `simulation-core` | MIGRATE_AND_RENAME | deterministic `client-simulation` projection mutation |
| `input-actions` | MIGRATE_AS_IS | exact source blobs and source test suite retained |
| `input-platform` | MIGRATE_AS_IS | exact source blobs and source test suite retained |
| `game-session` | SPLIT | only safe client selection lifecycle concepts retained; credentials/admission omitted |
| `identity` | REWRITE | PKCE/state/callback security on cancellable Tokio boundaries |
| `platform` | SPLIT | safe values in contracts; bounded reqwest/rustls I/O in client; gameplay routing rejected |
| `protocol-canary` | REFERENCE_ONLY | absent from destination workspace |
| `protocol-core` | REFERENCE_ONLY | absent; FND-02 owns future framing |
| `renderer` | MIGRATE_AS_IS | source surface state machine and DX12 backend retained |
| `renderer-resource` | SPLIT | neutral generation-fenced cache in renderer; synthetic adapters stay non-release |
| `test-support` | MIGRATE_AS_IS | source timeline/diagnostic fixture role retained with provisional generation names |
| `transport` | REFERENCE_ONLY | absent; FND-02 owns future transport |
| `world-directory` | MERGE | display-only worlds/channels/characters; host and port rejected |
| `technical-login` | REFERENCE_ONLY | no test copied; destination negative pre-native acceptance authored separately |
| `security/auth` | REWRITE | async/cancellation/redaction/route-negative tests |
| `architecture-check` | REWRITE | parses `workspace-boundaries.toml` and locked cargo metadata |
| `asset-compiler` | MIGRATE_AND_RENAME | explicitly synthetic fixture compiler |

## Product state

The migrated client is a deliberate pre-native-protocol product state:

1. the Windows application compiles and opens a visible native window;
2. the renderer uses the retained DX12 backend during normal launch;
3. the shell title states that native gameplay is unavailable;
4. `request_gameplay_entry` returns a closed `NativeProtocolUnavailable` result;
5. no gameplay endpoint, port, protocol profile, ticket, session credential or admission DTO exists in the production public contracts;
6. Platform directory parsing recursively rejects such fields rather than exposing them.

This is not a fake `protocol-oteryn` implementation and does not claim gameplay compatibility.

## Async and security boundary

- the application owns one Tokio runtime;
- Platform HTTP uses reqwest/rustls with redirects disabled and proxy discovery disabled;
- HTTPS is mandatory except bounded loopback HTTP for local testing;
- responses are read incrementally under a fixed byte ceiling;
- cancellation races the owned async operation and fails closed;
- Identity callback reception is bounded by timeout and cancellation;
- authorization code and PKCE verifier debug/display output is redacted;
- diagnostics accept reviewed static text or typed/redacted runtime values, not arbitrary secret-bearing strings.

## Synthetic evidence

The standalone harness constructs a project-owned projection, applies deterministic events, validates a synthetic-v1 RGBA fixture, routes a semantic input action and exercises the renderer state machine. It has no network dependency and is not reachable from the production client closure.

No Tibia/CipSoft asset, protocol fixture or proprietary binary is included.

## Provenance truthfulness

`docs/migration/rust-client-provenance.json` records exact source blob identities where byte equality is preserved and explicit transformations elsewhere. This migration does not preserve cross-repository Git ancestry and does not claim transformed files are byte-identical.

## Required validation

- locked metadata and exact nineteen-member boundary check;
- formatting and strict Clippy;
- Linux workspace build and tests;
- Windows release client build and visible shell smoke;
- deterministic synthetic harness on Linux and Windows;
- authentication and pre-native negative tests;
- production dependency closure negatives;
- cargo-deny advisories, licenses, bans and sources;
- independent final audit against the complete diff and exact head.
