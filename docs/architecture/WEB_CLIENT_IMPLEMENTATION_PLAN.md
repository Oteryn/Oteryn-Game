# Oteryn Web Client — staged implementation plan

Status: **Planning candidate; no implementation authority**
Tracking: Issue #519
Architecture candidate: `ADR-0018-browser-client-web-ready-boundaries.md`

This plan translates the proposed browser-client architecture into independently gated stages. Until a protected owning decision accepts or adopts it, current native-client and Server Seam work may use it only as non-authoritative evidence and must follow existing protected authority.

## 1. Objective

Deliver, in a later programme, a browser client that:

- reuses the maximum practical amount of the canonical Rust client semantics;
- talks to the same authoritative Oteryn Game through `protocol-oteryn`;
- preserves Platform/Game Gateway and FND-04 admission ownership;
- uses a browser-appropriate, separately registered gameplay transport profile;
- uses WebGPU as the preferred browser renderer;
- preserves native desktop as an independently optimized first-class client;
- is measurable, bounded, fail-closed and supportable as a production surface.

This plan does **not** authorize any stage to begin merely because this file exists.

## 2. Current baseline to protect

At plan creation the canonical workspace already contains:

- `apps/client` as the native client composition;
- `crates/client-runtime`;
- `crates/client-domain`;
- `crates/client-simulation`;
- `crates/input-actions`;
- `crates/input-platform`;
- `crates/renderer`;
- Platform client/contracts and Identity support;
- one canonical Rust workspace shared with the authoritative Game server.

The current dependency shape is intentionally desktop-oriented in several places. In particular, the workspace-level `wgpu` dependency selects DX12, the current application uses Windows `winit`, and the workspace/client runtime use native multithread Tokio while the workspace Tokio feature set includes `net`. These are dependency facts to audit, not proof of gameplay networking and not reasons to create parallel browser semantics. Under ADR-0016 and transport-policy revision 4, the gameplay transport adapter/listener and native-client gameplay entry are not implemented or runtime-available; TCP profile `1` is only the registered initial/default architecture and admission profile, and QUIC remains separately gated.

## 3. Programme rules

Every implementation stage must obey:

1. one active writer per owned path;
2. GitHub live state as lifecycle authority;
3. exact protected-main readback before allocation;
4. normal branch/PR/CI/Merge Queue protection;
5. no direct push to protected `main`;
6. no browser work broadening Server Seam authority;
7. no new protocol or transport profile without owning accepted contract;
8. no production browser admission before resource/security/E2E evidence exists;
9. no empty crates created solely to match a target diagram;
10. fail closed on unknown browser/runtime capability.

## 4. Stage W0 — architecture acceptance and dependency map

### Goal

Protect the architecture boundary before implementation pressure makes native-only coupling expensive.

### Deliverables

- accepted/superseded disposition for ADR-0018;
- exact dependency graph of current client crates;
- classification of every current client dependency as:
  - `SHARED_WASM_CANDIDATE`;
  - `NATIVE_EDGE`;
  - `REQUIRES_SPLIT`;
  - `UNKNOWN_REQUIRES_SPIKE`;
- exact list of desktop assumptions currently visible in public/shared APIs;
- explicit confirmation that Server Seam requires no browser implementation.

### Exit gate

No runtime mutation required. Architecture review must show no conflict with FND-02/FND-04/ADR-0014.

## 5. Stage W1 — `wasm32` compile-readiness audit

### Goal

Measure how much of the client already compiles for the selected browser/WASM target before designing refactors.

### Work

Attempt target-specific compile/check of the smallest real shared subgraph first:

```text
foundation
client-domain
client-simulation
input-actions
protocol-neutral support
```

Then expand toward runtime/platform/renderer edges.

### Required output

For each failure record:

- exact crate/path;
- dependency responsible;
- native API/type leaking across boundary;
- whether the fix is `cfg` isolation, trait/interface extraction, dependency replacement, or a real semantic redesign;
- whether native behavior changes.

### Prohibited shortcuts

- feature-disable shared behavior merely to make `wasm32` compile;
- browser stubs that return success without capability;
- removing native validation;
- replacing one real shared implementation with duplicated JS logic without evidence.

### Exit gate

A reviewed dependency-cut plan exists with the smallest justified refactor set.

## 6. Stage W2 — platform/runtime seam extraction

### Goal

Make shared client semantics independent of desktop process/window/network lifecycle.

### Candidate abstractions

Only create abstractions proven necessary by W1. Typical boundaries:

```text
Clock / frame scheduling
Surface lifecycle
Realtime transport
Asset fetch/open
Persistent cache/settings
Clipboard
Fullscreen / pointer lock
Audio sink
Browser/native lifecycle notifications
```

### Design rule

Prefer capability-oriented interfaces over one giant `Platform` trait. A large platform object tends to become a dumping ground and makes deterministic testing harder.

### Runtime model

Shared runtime should accept asynchronous events and produce deterministic state transitions. The native shell may drive it from `winit`/Tokio; the browser shell may drive it from browser callbacks/futures.

### Exit gate

Native behavior remains unchanged and tested; the extracted shared subgraph compiles for the chosen WASM target without fake implementations.

## 7. Stage W3 — browser shell without gameplay transport

### Goal

Create the first real browser composition while retaining ADR-0011-style fail-closed gameplay unavailability.

### Expected capability

The artifact may:

- bootstrap WASM;
- create a canvas/surface;
- initialize diagnostics;
- load approved synthetic assets;
- map browser input to shared actions;
- render an offline synthetic scene;
- show explicit unsupported/unavailable states.

It must not:

- request/consume gameplay credentials;
- connect to a production Game endpoint;
- report gameplay admission success;
- invent a protocol adapter.

### Packaging

Select a minimal reproducible JS/WASM build pipeline and pin its tool/dependency versions. Generated glue is build output, not a second source of gameplay semantics.

### Exit gate

Deterministic cold start, failure handling and shutdown/reload behavior are proven in at least one supported development browser cell.

## 8. Stage W4 — WebGPU renderer qualification

### Goal

Reuse the canonical presentation model through a browser WebGPU surface.

### Refactor target

Move only surface/backend initialization and truly platform-specific resource lifecycle to the edge. Preserve shared:

- render input/view model;
- sprite/atlas selection semantics;
- animation state;
- camera/floor composition;
- effect/light presentation semantics;
- batching logic where backend-neutral;
- resource identifiers and cache keys.

### Qualification matrix

Use both synthetic fixtures and approved real-world presentation evidence.

Record:

- CPU frame time p50/p95/p99;
- GPU frame time where available;
- upload/cache churn;
- active texture/resource working set;
- peak WASM memory;
- GPU resource loss/recovery;
- resize/DPI changes;
- cold/warm asset behavior.

### WebGL2 decision

Do not add a fallback during this stage unless a separate evidence task proves it is worth the complexity. Unsupported WebGPU should remain an explicit state.

### Exit gate

The browser renders the selected scenes correctly under measured hard bounds without changing gameplay semantics.

## 9. Stage W5 — browser asset manifest, cache and update model

### Goal

Make browser asset delivery deterministic and revision-safe.

### Preferred model

```text
small release manifest
        |
 immutable hashed WASM/JS/assets
        |
 browser HTTP cache / Cache Storage / IndexedDB
```

Exact storage APIs remain implementation-owned.

### Requirements

- immutable/revision-fenced URLs or object identities;
- manifest digest and artifact digest validation as required by the owning asset contract;
- encoded/decompressed hard limits;
- bounded concurrency;
- retry policy with bounded work;
- cold-cache correctness;
- coherent update/rollback;
- no mixed-revision client;
- corruption and partial-download failure tests;
- service-worker update races covered if a service worker is adopted.

### Exit gate

A browser reload/update cannot compose stale assets with a new incompatible client unnoticed.

## 10. Stage W6 — browser transport bake-off, non-production only

### Goal

Measure browser transport candidates without creating admission authority.

### Candidates

Primary candidate:

- WebTransport over HTTP/3.

Compatibility candidate if justified:

- secure WebSocket.

Possible topology variants:

- direct browser transport to GameNode;
- explicit Game-owned/Platform-owned edge relay, subject to ownership decision.

### Mandatory evidence

For each candidate measure/test:

- connect/setup latency;
- reconnect latency;
- ordered-lane behavior;
- snapshot transfer under loss;
- backpressure;
- browser background/foreground behavior;
- memory/queue pressure;
- malformed ingress cost;
- service identity/origin behavior;
- proxy/CDN/LB compatibility;
- constrained network/UDP-block behavior where relevant;
- deployment/observability complexity.

### Hard rule

The bake-off uses synthetic or explicitly non-production credentials/endpoints. It cannot reuse transport profile `1` by name or authority.

### Exit gate

One preferred browser transport and zero or one justified fallback are proposed with evidence. No production activation follows.

## 11. Stage W7 — protocol/admission transport-profile contract

### Goal

Create the real browser gameplay transport authority.

This is a contract/architecture stage, not merely client code.

### Required decisions

- stable browser transport profile ID and name;
- exact application framing mapping;
- ordered lane semantics;
- TLS/HTTP/ALPN/origin/service identity requirements;
- pre-admission grant binding;
- recovery grant binding;
- fallback rules;
- downgrade/replay prevention;
- maximum streams/connections/queued bytes;
- handshake/admission timeouts;
- error mapping;
- relay ownership if a relay exists;
- telemetry/redaction requirements.

### Relationship to ADR-0014

If WebTransport is selected, do not assume it is automatically the same profile as a future native QUIC adapter. Reuse shared QUIC principles where valid, but register and prove exact browser transport semantics.

### Exit gate

FND-02/FND-04 and machine-readable registries are coherently reconciled and protected. Only then may a later implementation consume the profile.

## 12. Stage W8 — browser gameplay vertical slice

### Goal

Connect the exact browser artifact to an authorized non-production Game environment.

### Minimum journey

```text
browser start
-> Platform identity
-> world/route discovery
-> Gateway admission material
-> browser gameplay transport
-> final Game admission
-> world entry
-> movement
-> one combat/action path
-> state reconciliation
-> reconnect/reload case
-> persisted result observed after relog
```

The exact gameplay slice should reuse the same accepted VSL behavior as native, not create a browser-specific feature slice.

### Failure cases

At minimum:

- expired/invalid grant;
- wrong transport profile;
- stale connection generation;
- snapshot interruption;
- reconnect while old transport is stale;
- multi-tab competing login;
- GPU device loss while session continues;
- asset fetch failure before/after admission;
- browser refresh during active session;
- unsupported browser capability.

### Exit gate

The browser path is functionally correct in bounded test environments. This is not yet public-release authority.

## 13. Stage W9 — production-quality browser qualification

### Goal

Prove that the browser surface is supportable, secure and sufficiently performant.

### Support matrix

Choose explicit browser/OS/GPU classes from evidence. Do not promise broad browser support from API detection alone.

### Required quality evidence

- startup cold/warm;
- basic/normal/stress real scenes;
- p50/p95/p99 frame times;
- memory and GPU residency;
- long-session soak;
- reconnect/loss/recovery;
- asset-cache churn and eviction;
- WebGPU device loss;
- background/foreground transitions;
- browser update during idle/active states;
- transport/resource pressure;
- crash/panic diagnostics;
- accessibility/input baseline;
- security headers/origin policy;
- dependency/supply-chain review.

### Product thresholds

Numeric thresholds must be selected from real measurements and target hardware classes. Do not copy desktop thresholds blindly.

## 14. Stage W10 — player opt-in rollout

Only after explicit production authority.

Suggested rollout sequence:

1. internal/dev origin;
2. authenticated staff/test cohort;
3. small opt-in player cohort;
4. expanded opt-in cohort;
5. stable public browser client if metrics remain within accepted thresholds.

Native client remains available throughout. Browser rollout must have a remote-disable mechanism at the route/product-policy level that does not weaken or alter native TCP security.

## 15. CI architecture to add only when stages require it

### W1–W2

Add a focused `wasm32` compile lane only when real shared code is expected to compile. Do not add permanently red speculative CI.

### W3–W5

Add deterministic browser build plus component tests. Cache build dependencies but never treat cache hit as correctness evidence.

### W6–W8

Add transport/browser E2E with exact endpoint/profile fixtures. Separate synthetic transport lab evidence from production-like Game admission evidence.

### W9+

Add supported-browser smoke matrix and periodic performance/soak campaigns. Keep heavy matrices outside every small PR unless affected-path policy requires them.

### Merge Queue

When browser code becomes production-relevant, update the impact classifier and Merge Queue composition deliberately. Browser-required checks must not be introduced in a way that creates missing-status deadlocks or weakens current FULL queue verification.

## 16. Suggested ownership boundaries when implementation starts

Actual path ownership must be issued from live state. Likely independent areas are:

- shared client-domain/simulation audit;
- runtime/platform abstraction;
- renderer WebGPU surface;
- asset/cache/update;
- browser transport experiment;
- protocol/admission contract;
- browser shell/package;
- QA/E2E/performance.

Do not run simultaneous writers through shared Cargo/workspace files without an explicit serialized composition lease.

## 17. Server changes expected later

A browser client should not require gameplay-domain forks. Possible server-side work is limited to the accepted transport edge and operational support, for example:

- new browser transport listener/profile or relay integration;
- transport-specific resource accounting;
- route advertisement;
- transport telemetry;
- accepted recovery/fallback integration.

The following are **not** legitimate browser reasons for server divergence:

- different combat rules;
- different item truth;
- different movement authority;
- weaker authentication;
- weaker resource limits;
- alternate persistence semantics;
- different `CommandId` or `server_sequence` meaning.

## 18. Platform changes expected later

Platform work, under separate repository authority, may eventually be required for:

- browser-compatible route offers;
- transport-profile-specific pre-admission material;
- web-origin/session integration;
- CSP/origin hosting policy;
- release/rollout cohort control.

Oteryn-Game architecture must describe the required consumer contract but must not implement or claim Platform changes without separate authorization.

## 19. Asset/CDN deployment shape

A likely production topology is:

```text
browser
  |-- HTTPS -> immutable web shell/WASM/assets/CDN
  |-- HTTPS -> Platform identity/Gateway
  `-- browser realtime transport -> Game transport endpoint/edge
```

This is a topology class, not a selected vendor or hostname.

Gameplay assets served from a CDN remain presentation inputs. The authoritative server does not trust client possession or asset contents as gameplay authority.

## 20. Browser-specific operational risks

Track explicitly:

- browser API/version changes;
- GPU driver/WebGPU implementation variation;
- hidden-tab timer throttling;
- memory pressure and tab eviction;
- service-worker stale-cache incidents;
- CDN partial-region failure;
- cross-origin policy mistakes;
- XSS turning browser-held short-lived credentials into exfiltration targets;
- extension/injected-script interference;
- multi-tab session confusion;
- WebTransport/UDP blocking or intermediary incompatibility;
- mobile thermal/memory limits if mobile support is added.

These are operational/support risks, not reasons to weaken server authority.

## 21. Definition of browser-client readiness

The programme is `READY_FOR_PUBLIC_OPT_IN` only when all are true:

- accepted browser transport/admission profile exists;
- exact production browser artifact builds reproducibly;
- supported browser matrix is explicitly declared;
- security/resource limits are accepted and tested;
- asset release/cache/update path is coherent and rollback-safe;
- browser E2E passes the required exact-head journey;
- real scene performance meets selected targets;
- reconnect/recovery/multi-tab cases are proven;
- diagnostics can separate transport/render/asset/admission failures;
- native client remains unaffected or regressions are explicitly accepted;
- production rollout authority is separately granted.

Anything less must use a narrower state such as `EXPERIMENTAL`, `NON_PRODUCTION`, `INTERNAL_ONLY`, or `BLOCKED` with the exact missing gate.

## 22. Immediate action for current development

No browser implementation should be started merely from this plan.

Only after a separately accepted owning architecture decision or protected adoption may reviewers enforce the ADR-0018 anti-coupling checklist. Before then, current Native UI, Server Seam, WP3, renderer-cache and actor-carrier work follows existing protected authority and may use this checklist only as non-authoritative design evidence; this plan cannot block, reject, seize or supersede that work:

- transport-neutral gameplay/session logic;
- platform-neutral shared client semantics;
- normalized input actions;
- renderer surface separation;
- logical asset identity;
- no browser/native server semantic split;
- explicit transport-profile binding in admission/recovery;
- no desktop-only types leaking into shared interfaces without necessity.

That is the lowest-cost action now and preserves the later browser path without delaying the present programme.

`IMPLEMENTATION_AUTHORITY: NONE`
`PRODUCTION_AUTHORITY: NONE`
`LIVE_DEPLOYMENT_AUTHORITY: NONE`
