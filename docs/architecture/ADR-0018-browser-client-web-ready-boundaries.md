# ADR-0018: Browser client web-ready boundaries

- Status: **Proposed architecture; not yet accepted**
- Date: 2026-09-09
- Tracking: Issue #519
- Applies to: native client composition, future browser client, `protocol-oteryn` transport adapters, renderer/platform boundaries, asset delivery and client validation
- Depends on and is subordinate to: ADR-0001, ADR-0003, ADR-0011, ADR-0014, ADR-0016, FND-02 and FND-04
- Does not authorize: browser runtime implementation, production browser distribution, Platform changes, new transport-profile registration, WebSocket/WebTransport admission, QUIC activation, production listeners, deployment, credentials, live data, or direct changes to protected `main`

## 1. Context

Oteryn already owns one canonical Rust gameplay repository containing the authoritative server and native Rust client. The client has separate runtime, domain, simulation, input and renderer components, but the current composition is intentionally native/Windows oriented: the root workspace enables the DX12 `wgpu` backend, the application depends on Windows `winit`, and the workspace/client runtime use a native multithread Tokio runtime whose workspace feature set includes `net`. As ADR-0016 and transport-policy revision 4 record, those dependency facts do **not** mean that gameplay networking exists: the gameplay transport adapter, gameplay listener and native-client gameplay entry are not implemented or runtime-available.

The product may later benefit from a browser client with zero-install entry, while preserving the native desktop client as a first-class surface. The expensive failure mode would be to continue implementation until protocol, renderer, input, asset and client-runtime abstractions silently assume Windows, raw TCP sockets, OS filesystem paths or blocking native lifecycle semantics. Retrofitting those assumptions after the first complete gameplay path would require a second client architecture or invasive rewrites.

This candidate proposes only the minimal boundaries that would remain web-ready if separately accepted. It does not govern current work or start browser implementation merely by being present or merged with proposed status.

## 2. Decision summary

Oteryn will preserve a future browser client as a **second presentation/runtime surface of the same Rust client architecture**, not as a separate gameplay product.

The target shape is:

```text
                            Oteryn Platform
                     Identity / Gateway / Registry
                                |
                    transport-bound admission offer
                                |
                     +----------+----------+
                     |                     |
              native transport       browser transport
                     |                     |
                     +----------+----------+
                                |
                         protocol-oteryn
                                |
                       authoritative Game

 shared client semantics
 +-------------------------------------------------------+
 | client domain | client simulation | input actions     |
 | protocol semantics | reconciliation | presentation VM |
 +-------------------------------------------------------+
             |                              |
     native platform edge             browser platform edge
     desktop window/audio             WASM/browser APIs
     DX12 renderer path               WebGPU renderer path
```

Binding principles:

1. **One gameplay application protocol.** Browser support must not create `protocol-web`, Canary compatibility, JSON gameplay APIs, translated gameplay semantics or a browser-only state model.
2. **One server authority model.** The Game server remains authoritative. Browser/native differences cannot change gameplay truth, ordering, leases, persistence or anti-duplication semantics.
3. **Platform ownership is unchanged.** Browser convenience cannot bypass Identity, Game Gateway, transport-bound pre-admission material or final Game-domain admission.
4. **Browser transport is a distinct transport profile when it differs on the wire.** A browser cannot reinterpret the existing TCP + TLS 1.3 profile `1` as WebSocket or WebTransport.
5. **Shared Rust first.** Protocol-neutral client state, simulation/prediction, reconciliation, input actions and presentation state should compile independently of desktop-only APIs.
6. **Platform edges own platform APIs.** Window/surface creation, browser lifecycle, realtime transport implementation, persistent cache, clipboard, fullscreen/pointer lock, audio and browser storage belong behind explicit boundaries.
7. **No correctness dependency on browser trust.** WASM, JavaScript, DOM state and browser storage are untrusted client state.
8. **No speculative empty crates.** Logical boundaries are defined now; physical crates are split only when an accepted implementation task has a real consumer.

## 3. Current constraints that must be preserved

This ADR does not supersede existing authority:

- ADR-0001: Rust server/client and one `protocol-oteryn` family remain the target.
- ADR-0011: incomplete gameplay paths fail closed before credential consumption or misleading success.
- FND-02: protocol major, framing, message semantics, sequencing, `CommandId`, `server_sequence`, snapshot/reconciliation, input limits and registered TCP transport-profile semantics remain authoritative.
- ADR-0014: gameplay/session code remains transport-neutral; TCP profile `1` is the registered initial/default architecture and admission profile, while QUIC remains a separately gated future transport target.
- ADR-0016 and `PROTOCOL_OTERYN_TRANSPORT_POLICY.json` revision 4: no gameplay transport adapter/listener or native-client gameplay entry is currently implemented or runtime-available. The registered TCP profile is not a claim of usable gameplay networking.
- FND-04: Platform/Game admission, reconnect, recovery and generation fencing remain authoritative.

A browser client can be introduced only by consuming those contracts or by separately accepted amendments. It cannot silently reinterpret them.

## 4. Browser target and execution model

### 4.1 Preferred target

The preferred future browser composition is:

```text
Rust shared client code
        |
      wasm32
        |
 browser platform adapter
        |
 WebGPU / browser input / browser audio / browser storage
```

The baseline browser design must be correct without requiring WebAssembly threads, `SharedArrayBuffer` or cross-origin isolation. Optional multithreaded WASM may be evaluated later after measurement and security/hosting review.

The browser main loop must be asynchronous and browser-driven. Shared client logic must not require:

- a blocking process main loop;
- `pollster` as a correctness dependency;
- raw OS window handles;
- filesystem paths as asset identity;
- native socket types;
- OS thread identity for authoritative client behavior.

### 4.2 Desktop remains first-class

Browser readiness does not demote the native client. Desktop may retain platform-specific optimizations such as DX12, native filesystem/update integration, richer diagnostics and platform APIs. Shared abstractions must not force the native build to enable every browser or GPU backend.

The intended relationship is:

```text
same protocol + same client semantics
                 |
        +--------+--------+
        |                 |
 desktop composition   browser composition
 native optimized       browser constrained
```

## 5. Logical client layers

The exact physical crate split remains implementation-owned. The architecture requires these logical layers regardless of crate boundaries.

### 5.1 Shared semantic layer

Must remain platform-neutral:

- client domain state;
- deterministic/predictive client simulation where accepted;
- protocol message semantics and codecs that are transport-independent;
- command/result tracking;
- state revision and reconciliation state;
- `connection_generation` handling;
- input **actions** rather than physical key codes;
- presentation/view-model state;
- asset/content logical keys and revisions;
- gameplay UI state that is not tied to DOM or Win32 controls;
- diagnostics events that contain no platform secret material.

Existing components such as `client-domain`, `client-simulation` and `input-actions` are natural shared candidates and must not acquire unnecessary desktop-only dependencies.

### 5.2 Runtime orchestration layer

Owns lifecycle semantics independent of the concrete event source:

- bootstrap state;
- transition into authenticated gameplay capability;
- transport state machine;
- reconnect/reconciliation orchestration;
- frame/update scheduling inputs;
- renderer-facing snapshots/view models;
- asset availability states;
- deterministic shutdown/failure outcomes.

The runtime may use different executors per target, but target-specific executor types must not leak into shared gameplay/domain APIs.

### 5.3 Platform adapter layer

Target-specific responsibilities:

- native window or browser canvas/surface;
- native socket or browser transport API;
- native filesystem/updater or browser HTTP/cache storage;
- native clipboard/fullscreen/input APIs or browser equivalents;
- native audio backend or Web Audio integration;
- monotonic-clock and visibility/focus events;
- browser URL/origin lifecycle;
- crash/panic integration and coarse platform telemetry.

### 5.4 Renderer split

The renderer must separate:

```text
presentation semantics / render graph inputs
                |
        GPU resource abstraction
                |
       +--------+--------+
       |                 |
 native surface       browser surface
 DX12 today           WebGPU target
```

Current DX12 ownership is valid for the native product. It must not become a type-level requirement of shared presentation/domain code.

A future implementation may retain one `wgpu`-based renderer with target-specific initialization, or split renderer-core/native/web crates if evidence shows that is cleaner. This ADR does not mandate empty crates now.

## 6. Rendering policy

### 6.1 WebGPU first

WebGPU is the preferred browser rendering backend because it offers a GPU model close enough to the existing Rust `wgpu` direction to maximize shared renderer code.

Browser support must be capability-detected at runtime. A supported browser build cannot infer support only from user-agent strings.

### 6.2 Fallback

WebGL2 or another fallback is **not automatically authorized**. A fallback may be added only if a later task proves:

- acceptable visual correctness;
- bounded resource behavior;
- clear capability reduction rather than silent semantic divergence;
- sufficient browser coverage benefit;
- maintainable test cost.

Unsupported GPU capability must fail with a clear unsupported-client result, not a broken or partially rendered gameplay state.

### 6.3 Rendering invariants

Native and browser presentation should consume the same logical state. Differences in backend may change performance or optional visual quality, but must not change:

- authoritative gameplay state;
- visibility rules received from the server;
- command generation semantics;
- collision/pathfinding authority;
- combat outcomes;
- item/container truth;
- security or admission decisions.

Local visual-quality modes may reduce particles, post-processing, lighting detail, filtering or animation density only where presentation semantics remain safe.

## 7. Transport architecture

### 7.1 No raw-TCP assumption in shared code

Browsers do not expose the native TCP socket contract used by current transport profile `1`. Therefore shared client/session code must depend on an abstract reliable transport interface rather than native `TcpStream`, Tokio `net` types or TLS implementation details.

### 7.2 Browser transport is not profile 1

The following equivalences are forbidden:

```text
WebSocket == tcp_tls13_alpn_v1       NO
WebTransport == tcp_tls13_alpn_v1    NO
browser QUIC == ADR-0014 QUIC profile by inference    NO
```

If browser transport bytes, handshake, endpoint semantics or security binding differ from profile `1`, a separate stable transport profile must be registered through the owning FND-02/FND-04 process.

### 7.3 Candidate browser transports

Later evidence may evaluate:

1. **WebTransport over HTTP/3** as the preferred long-term browser transport candidate;
2. **secure WebSocket** as a compatibility candidate/fallback if independently justified;
3. an explicit edge relay topology if direct browser-to-GameNode transport is operationally undesirable.

No candidate is activated by this ADR.

### 7.4 One application protocol above transport

A future browser profile must carry the same `protocol-oteryn` application semantics. It may adapt outer framing to the transport only through an accepted profile definition; it may not introduce browser-specific gameplay JSON, duplicate commands, alternate state semantics or translation to Canary/Tibia packets.

### 7.5 Ordering

Browser transports must preserve all existing FND-02 ordering and snapshot barriers. A transport with multiple streams cannot process authoritative state by arrival order across streams.

Until a separately accepted cross-lane scheme exists, browser authoritative state should use one logical ordered server lane and one logical ordered client-command lane, consistent with ADR-0014's conservative QUIC ordering baseline.

### 7.6 Relay/topology boundary

A browser edge relay, reverse proxy or transport terminator may be introduced later, but it is not gameplay authority. If used it must:

- preserve exact transport-profile identity;
- preserve authenticated route/service identity;
- enforce bounded ingress/resource policy;
- never manufacture Game admission success;
- never mint GameSession identity;
- never reinterpret application rejection as transport fallback;
- preserve end-to-end evidence needed to diagnose ordering/reconnect failures.

Whether browser transport terminates at GameNode or at an explicit edge component is deferred.

## 8. Identity, Gateway and admission

Browser login convenience must preserve Platform authority.

The target remains conceptually:

```text
browser client
-> Platform identity/session
-> Game Gateway / route selection
-> transport-bound pre-admission material
-> browser gameplay transport
-> Game-domain final admission
-> canonical GameSessionId
```

Rules:

- Game Login Tickets are never sent directly to the GameNode unless the owning accepted Platform/Game contract explicitly says so; existing separation remains binding.
- Browser storage must not persist reusable gameplay secrets merely for convenience.
- Admission/recovery material is purpose- and transport-bound.
- Switching from one browser transport profile to another requires the accepted fresh/continuation flow; credentials are not silently rebound.
- URL query strings/fragments must not become a default carrier for reusable gameplay secrets.
- `localStorage` must not hold durable authentication or gameplay authority secrets.
- a browser refresh, crash or service-worker update does not prove reconnect eligibility; FND-04 remains authoritative.

## 9. Browser storage and asset architecture

### 9.1 Asset identity

Gameplay asset identity must be logical/content-addressed, never an absolute desktop filesystem path.

A future browser artifact should use an immutable manifest that binds at least:

- client build/release revision;
- asset bundle revision/digest;
- content/map presentation revision where applicable;
- file/object digests;
- encoded and decoded size metadata needed for limits;
- compatibility metadata required by the client.

### 9.2 Delivery and cache

Browser assets may use normal HTTPS/CDN delivery and browser cache facilities. Cache Storage and IndexedDB are candidate mechanisms; the exact physical storage choice is implementation-owned.

Requirements:

- content-addressed or revision-fenced objects;
- no silent mixing of incompatible asset revisions;
- checksum/digest validation before trusted use where the owning artifact contract requires it;
- bounded concurrent downloads;
- bounded encoded and decoded bytes;
- bounded image/texture dimensions and object counts;
- fail-closed decompression/parse limits;
- deterministic invalidation when the manifest changes;
- no dependence on persistent cache for correctness.

A cold cache must remain a valid execution path.

### 9.3 Service worker

A service worker may later own static shell/asset caching and atomic update staging. It must not become gameplay authority and should not proxy or reinterpret the realtime gameplay protocol unless a separate design explicitly requires and reviews that behavior.

Offline cached assets do **not** imply offline gameplay support.

## 10. Input, focus and browser lifecycle

Physical browser events must map into the shared `input-actions` vocabulary.

The browser adapter must explicitly handle:

- keyboard scan/code normalization;
- key-repeat semantics;
- pointer/touch/gamepad mapping where supported;
- wheel/gesture normalization;
- text/IME composition for chat and text entry;
- pointer-lock/fullscreen user-gesture requirements;
- focus loss and `visibilitychange`;
- synthetic release of held movement/actions on focus loss to avoid stuck input;
- resize/DPI/device-pixel-ratio changes;
- suspend/resume and background throttling.

A hidden/background tab may reduce or pause presentation work, but it cannot fabricate server liveness, advance authoritative world state locally or bypass AFK/reconnect rules.

Multi-tab coordination may improve UX, but the server-side CharacterLease/GameSession model remains the security authority. `BroadcastChannel`, storage locks or similar browser mechanisms cannot be relied upon for single-session safety.

## 11. Audio and media

Browser audio must respect user-gesture/autoplay constraints. Audio failure must degrade presentation cleanly without corrupting gameplay state.

Shared presentation state should describe sounds/music/effects semantically; target-specific adapters own actual device graph, resume/suspend and browser policy handling.

## 12. UI strategy

Gameplay UI state should remain shared and protocol-neutral.

The browser may use HTML/DOM for bootstrap, login shell, unsupported-capability messages, accessibility surfaces or integration chrome. DOM components must not become a second owner of gameplay truth.

If the native client renders gameplay UI through the GPU renderer, browser gameplay UI should preferentially reuse the same semantic/view-model layer. A later task may decide whether specific browser-native controls materially improve accessibility or mobile usability.

## 13. Security model

The browser client is fully untrusted from the server's perspective.

No security property may depend on:

- WASM obscurity;
- JavaScript minification;
- hidden DOM state;
- browser cache integrity without verification;
- anti-debugging;
- local clock honesty;
- local movement/combat simulation honesty.

Browser-specific security requirements include:

- strict Content Security Policy appropriate to the final hosting topology;
- no inline-secret injection into generated HTML;
- no long-lived reusable gameplay secret in browser-persistent storage;
- explicit CORS/CORP/COEP policy if later features require them;
- dependency and supply-chain review for WASM/JS glue and bundling tools;
- bounded WASM memory growth and browser-side decode/allocation work;
- origin and service-identity validation for every privileged network boundary;
- redaction of credentials/tickets/grants from logs, crash reports and telemetry.

Cross-origin isolation must not be enabled solely to obtain threads without evaluating hosting/security consequences.

## 14. Resource budgets

Browser support introduces new externally and locally controlled resource dimensions. Exact numeric maxima require measurements and registry ownership; this ADR does not invent values.

Before production browser admission, accepted limits must cover as applicable:

- initial HTML/JS/WASM bootstrap bytes;
- WASM linear-memory ceiling/growth policy;
- asset manifest entries;
- concurrent asset requests;
- encoded asset bytes;
- decoded image/texture bytes;
- texture dimensions/layers and GPU resident working set;
- browser persistent cache budget;
- pending render uploads;
- pending realtime transport bytes/messages;
- transport stream/session counts;
- reconnect attempts/backoff work;
- protocol decode work;
- UI/log/telemetry queue bounds.

Unknown/unmeasured limits fail closed at the gate that requires production use; they are not filled with arbitrary headroom.

## 15. Capability vocabulary

Three categories must remain separate:

1. **Protocol capabilities** — negotiated only when server/client semantics require an optional protocol feature.
2. **Local presentation capabilities** — GPU features, texture formats, audio features, touch, pointer lock, etc.
3. **Product support policy** — which browser/OS/GPU combinations Oteryn chooses to support.

A WebGPU feature bit must not automatically become a `protocol-oteryn` capability. Server behavior should not branch on browser-vs-native unless an accepted gameplay/product contract requires a semantic difference.

## 16. Version and release fencing

The browser composition must carry explicit versions for:

- client build;
- protocol major/schema evidence;
- active transport profile;
- asset/presentation manifest;
- content/ruleset/map revisions received from the authoritative route/session flow;
- browser shell/runtime artifact revision.

Cached files from two releases must not be composed into one unqualified client.

A future web release mechanism should prefer immutable hashed assets plus a small mutable release manifest. Rollback must select a coherent prior artifact set rather than partially downgrading individual files.

Native launcher/update lifecycle remains separately owned.

## 17. Observability

Browser diagnostics must make target-specific failures distinguishable without changing gameplay semantics.

At minimum later implementation should be able to report bounded/redacted evidence for:

- build and artifact revision;
- browser runtime family/version class;
- selected GPU backend/capabilities at a coarse non-identifying level;
- transport profile and lifecycle phase;
- asset/cache cold/warm state;
- frame CPU/GPU timing where available;
- dropped/late presentation frames;
- transport queue pressure;
- reconnect/reconciliation reason class;
- WebGPU device/surface loss;
- WASM panic/trap class;
- unsupported-capability failure.

Telemetry must not record reusable credentials or unnecessary fingerprinting data.

## 18. Performance and quality policy

This ADR deliberately does not freeze FPS, memory or download-size numbers before representative browser measurements exist.

Production support requires a named benchmark matrix with at least:

- cold and warm startup;
- representative basic/normal/stress scenes using real approved world/presentation data;
- GPU/CPU frame-time distributions rather than average FPS alone;
- asset download/decode/upload time;
- memory and GPU residency;
- transport latency/queue behavior under loss and reconnect;
- device/surface recovery;
- background/foreground transitions;
- supported browser/OS/GPU cells.

Native and browser results must be compared on matched hardware where possible. Browser release is not blocked on matching native peak FPS if it meets separately accepted product quality targets.

## 19. Test architecture

Browser implementation must extend, not replace, the existing QA model.

Required layers when the implementation exists:

### 19.1 Compile/policy gate

- prove shared crates compile for the selected `wasm32` target;
- prove desktop-only dependencies do not leak into shared dependency closure;
- prove browser-only dependencies do not enter native production artifacts unintentionally;
- validate feature combinations and release packaging.

### 19.2 Deterministic shared tests

The same domain/simulation/protocol-neutral tests should execute natively and, where useful, under WASM test infrastructure to catch target-specific integer/time/serialization assumptions.

### 19.3 Browser component tests

Test:

- input mapping/focus loss;
- cache/version behavior;
- renderer surface/device loss;
- manifest corruption/limit failure;
- transport adapter framing/order/backpressure;
- browser lifecycle and update fencing.

### 19.4 Browser E2E

Add a browser-client E2E tier only when there is an actual browser gameplay composition. It must prove the exact browser artifact and should cover:

```text
Platform identity
-> Gateway/route
-> browser transport
-> Game admission
-> world entry
-> movement
-> combat/presentation
-> reconnect/reconciliation
-> persistence observation
-> clean shutdown/reload
```

Synthetic renderer evidence is not browser gameplay proof. Headless server E2E is not browser presentation proof.

### 19.5 Adversarial/failure tests

At minimum:

- malformed/oversized transport input;
- asset manifest tamper and oversized decode;
- stale cached release mixing;
- offline/partial CDN failure;
- GPU device loss;
- background throttling;
- transport interruption during snapshot;
- stale generation after reconnect;
- multi-tab competing session attempts;
- unsupported browser capability;
- service-worker rollback/update race if a service worker is used.

## 20. Candidate anti-coupling requirements

Until a separately accepted owning architecture decision or protected adoption makes these requirements binding, they are non-authoritative design evidence only. Current Native UI, Server Seam, WP3, renderer-cache and actor-carrier work follows existing protected authority and cannot be blocked, rejected, seized or superseded by this candidate. If this boundary is accepted, the following requirements apply prospectively to work within its accepted scope because violating them would make future browser support materially harder.

### MUST preserve

- gameplay/domain code does not branch on TCP vs QUIC vs browser transport;
- protocol application messages do not contain native socket concepts;
- shared client semantic APIs do not expose Win32 handles, DX12 types, native filesystem paths or browser DOM types;
- input-domain APIs consume normalized actions, not only Windows key codes;
- asset identity is revision/key based rather than absolute-path based;
- runtime correctness does not require a blocking native event loop;
- renderer presentation inputs are independent of the concrete surface backend;
- admission/recovery consumes an explicit transport-profile identity;
- server authority is identical for native and browser clients;
- server resource accounting treats browser ingress as untrusted and separately bounded when that ingress exists;
- client kind is not an authorization primitive.

### MUST NOT do now

- add a fake `wasm32` crate with no consumer;
- enable every `wgpu` backend globally just to claim browser readiness;
- expose current TCP profile `1` through a WebSocket translator and call it the same profile;
- add browser-specific gameplay message schemas;
- store login/admission secrets in persistent browser storage;
- make Server Seam depend on a future browser gateway;
- delay native gameplay delivery solely because browser implementation is not started.

## 21. Preferred future physical composition

The following names are illustrative implementation guidance, not allocated paths and not proof that new crates must be created:

```text
apps/
  client/                  # native desktop composition
  web-client/              # future WASM/browser composition

shared logical layers
  client-domain
  client-simulation
  input-actions
  protocol-core / protocol-oteryn
  presentation state

platform edges, split only when justified
  client-runtime-core
  client-platform-native
  client-platform-web
  transport-native
  transport-web
  renderer-core
  renderer-native
  renderer-web
  asset-runtime
```

Implementation should prefer the smallest refactor that achieves dependency isolation. Target-specific modules inside an existing crate are acceptable when they keep dependency closure and tests clear.

## 22. Server Seam impact

Server Seam remains allowed to proceed independently.

If this candidate is separately accepted, its prospective architectural hygiene is:

```text
protocol semantics
!= transport implementation
!= client platform
```

Before acceptance, Server Seam follows its existing protected contracts; this candidate supplies non-authoritative evidence only. If accepted, Server Seam should expose/consume the authoritative protocol/session boundary without depending on desktop-client details. No WebSocket, WebTransport, WASM, browser route or browser asset work is required to complete Server Seam.

A later browser implementation must adapt to the protected Server Seam contract rather than forcing browser-specific semantics into gameplay domain code.

## 23. Staged adoption

Browser work should be introduced through separately allocated stages:

1. accept the web-ready boundary;
2. prove shared crate dependency/compile readiness for `wasm32` without gameplay networking;
3. extract only the platform/runtime seams proven necessary;
4. qualify WebGPU rendering against synthetic and real approved scenes;
5. qualify browser asset manifest/cache/update behavior;
6. perform transport candidates as non-production experiments;
7. accept a browser transport profile and FND-04 grant/recovery reconciliation;
8. implement bounded browser gameplay vertical slice in test environments;
9. add browser E2E, performance, security and resource evidence;
10. only then consider player opt-in/public rollout.

Each stage must have its own live issue/allocation and exact authority.

## 24. Decision timing

- **Must decide now? YES — but only before these minimal anti-coupling rules become binding.** A separately accepted owning decision is needed before reviewers may enforce one application protocol, transport-neutral gameplay/session semantics, platform-neutral shared semantics, normalized input actions, logical/revision asset identity, renderer-surface separation, and explicit transport-profile identity. The concrete work otherwise blocked is authoritative review and allocation of shared APIs whose public shape would commit Native UI, Server Seam, renderer or client-runtime consumers to a platform or transport boundary. This proposed ADR does not itself block that current work: before acceptance, existing protected authority governs and teams may use this analysis only as evidence.
- **Later burden if those minimal rules are deferred:** public shared APIs may acquire Win32, DX12, native-socket, absolute-filesystem-path or blocking-loop assumptions. Removing those assumptions after multiple consumers exist would require coordinated API and dependency-graph migrations, could split protocol/client semantics, and would increase compatibility and regression risk across native and future browser compositions.
- **Must decide now? NO — speculative browser implementation choices.** WebTransport versus WebSocket, direct GameNode versus relay topology, exact crate split, bundler, service-worker use, WebGL2 fallback, WASM threads, browser matrix, numeric budgets, CDN, texture format, and DOM/GPU UI balance do not block the current native vertical slice, Server Seam, Native UI, WP3, renderer-cache or actor-carrier work. They remain horizon items and require bounded implementation/product evidence rather than being frozen here.
- **Evidence that would justify selection or supersession:** exact-revision dependency/`wasm32` compile audits; measured native and browser frame, memory, download and latency data; browser capability and support-matrix results; transport bake-off and adverse-network evidence; security/origin/admission review; asset cache/update/rollback proof; real gameplay E2E; or a changed accepted product, Platform, protocol or deployment requirement. Supersession requires an explicit newer accepted ADR or owning contract identifying which rules change and which remain binding.
- **Deliberately not decided:** all choices listed in the next section, any physical split, any runtime/profile activation, and any implementation or production allocation.

## 25. Deferred decisions

This ADR intentionally does not freeze:

- exact browser hostname/origin;
- WebTransport vs WebSocket final choice;
- direct GameNode vs edge relay topology;
- QUIC library;
- exact physical crate split;
- JS/WASM bundler/toolchain;
- service-worker requirement;
- WebGL2 fallback;
- WASM threads;
- mobile browser support tier;
- PWA/installability;
- exact support browser matrix;
- exact FPS/memory/download budgets;
- final CDN/provider;
- final texture container/compression format;
- final DOM vs GPU UI balance.

Those are measured implementation/product decisions, not prerequisites for keeping the architecture open today.

## 26. Consequences

### Positive

- the native client can continue without foreclosing a zero-install browser client;
- one Rust semantic codebase can serve two client surfaces;
- protocol and server authority remain singular;
- browser transport limitations are handled explicitly instead of disguised as TCP;
- renderer/asset/input/platform seams become easier to test and evolve;
- future browser work can be staged without blocking current Server Seam delivery.

### Costs

- after protected acceptance, reviews within the adopted scope must reject accidental desktop/transport coupling in shared code; before acceptance this candidate cannot supply that rejection authority;
- the client architecture must keep platform dependencies at the edge;
- browser support will eventually add another build, renderer, transport, E2E and operational matrix;
- production browser support requires additional resource/security budgets and a separately accepted transport/admission profile.

## 27. Acceptance criteria for this ADR

This ADR can become accepted architecture only after review confirms that it:

- does not weaken ADR-0001/FND-02/FND-04 authority;
- does not reinterpret TCP profile `1`;
- does not accidentally activate QUIC/WebTransport/WebSocket;
- preserves Platform/Game Gateway ownership;
- requires no browser implementation to unblock Server Seam;
- defines enough anti-coupling rules to prevent avoidable native-only lock-in;
- leaves numeric/resource choices evidence-gated;
- creates no production or deployment authority.

Until accepted or otherwise adopted through protected owning architecture, it is a non-authoritative design candidate attached to Issue #519. Its presence cannot block, reject, seize or supersede Native UI, Server Seam, WP3, renderer-cache or actor-carrier work.

`IMPLEMENTATION_AUTHORITY: NONE`
`PRODUCTION_AUTHORITY: NONE`
`LIVE_DEPLOYMENT_AUTHORITY: NONE`
