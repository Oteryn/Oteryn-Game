# ALPHA-CLIENT-01 — Native Client Architecture Analysis

- DecisionStatus: `PROPOSED`
- DeliveryStatus: `IN_REVIEW`
- ImplementationStatus: `NOT_STARTED`
- Date: 2026-08-15
- Issue: `#263`
- Worker: `DOMAIN ARCHITECTURE DESIGN AGENT / worker E`
- Branch: `docs/arch-e-alpha-client`
- Authority: analysis for Architecture Coordinator audit; not canonical until accepted/merged by coordinator
- Runtime authorization: **NONE**

## 1. Executive summary

Oteryn-v2 already has a real Windows-first native Rust shell and a deliberately fail-closed `pre-native-protocol` product state. The alpha-client problem is therefore not “choose a GUI framework and build a client from zero”. It is to define the boundaries that let the existing shell grow into a production gameplay client without allowing UI state, local simulation, content files, transport convenience, audio presentation or diagnostics to become a second gameplay authority.

This analysis recommends one production client composition root in `apps/client`, with explicit application/screen coordination and provider boundaries around Identity/Platform directory access, the ADR-0003 Game Login Ticket/Game Gateway pre-admission control-plane chain, future gameplay admission and transport, client-safe content, audio presentation, configuration/filesystem, diagnostics/crash handling and release/update state. Renderer, audio-device backend and OS input remain adapters. Network ingress is reduced into a **non-authoritative client projection** only after protocol generation, sequence, revision and snapshot rules have been satisfied. Player input becomes semantic intent before it can reach future protocol egress. No widget, renderer, audio subsystem, local cache or synthetic simulation may write authoritative gameplay state.

The current `pre-native-protocol` behavior remains the only runtime truth. `ADR-0016` and `docs/contracts/PROTOCOL_OTERYN_TRANSPORT_POLICY.json` explicitly say that the gameplay transport adapter, listener and native-client gameplay entry are not implemented and every named client transport mode is unavailable now. This design therefore defines seams and evidence gates without exposing fake TCP/QUIC selectors or requesting/consuming a one-time gameplay credential for a path that cannot complete.

For alpha readiness, the client architecture must also treat content compatibility, audio degradation and resource ownership, configuration migration, crash privacy, update/rollback, Windows packaging and E2E evidence as first-class product boundaries. A green renderer or synthetic harness is insufficient. `ADR-0007` requires separate Tier 1 headless system E2E through the production Platform/protocol/server path, Tier 2 real native-client E2E and Tier 3 release-binary smoke evidence. Tier 1 may share production schemas/codecs, but FND-02 and the owner-accepted 2026-08-07 refinement require independent wire evidence so that a common codec defect cannot become its own oracle.

## 2. Scope and authority boundary

### 2.1 In scope

This work decides or proposes only client-side architecture needed to make later implementation unambiguous:

- application/screen/composition boundaries;
- provider boundaries and dependency direction;
- local runtime/session observation state;
- future protocol ingress/egress and reconciliation seams;
- renderer/UI/input integration;
- audio provider/device lifetime and presentation-only boundary;
- client-safe content projection and active-content boundary;
- configuration, filesystem, logging and crash diagnostics boundaries;
- update, packaging, install and rollback boundaries;
- headless versus interactive testing paths;
- Windows-first platform assumptions;
- client production-readiness and E2E evidence gates.

### 2.2 Explicitly outside this worker's authority

This analysis does **not**:

- implement or authorize a gameplay transport adapter/listener;
- implement or modify Platform Identity, Game Login Ticket or Game Gateway services;
- change `protocol-oteryn` framing, message schemas, sequencing, TLS, transport-profile or capability semantics;
- construct ADR-0003/FND-04 pre-admission, admission or reconnect grants or change GameSession/CharacterLease authority;
- change server/gameplay legality, deterministic simulation or persistence semantics;
- promote synthetic client simulation to production gameplay authority;
- select a new UI toolkit, audio library/backend/vendor, protobuf/TLS library, updater framework, installer technology or credential vault;
- define production numeric limits that belong to protocol, security, release or SRE owners;
- modify external repositories, global architecture overlays, executable client code, DDL or production configuration.

## 3. Verified baseline

| Source | Verified fact | Consequence for ALPHA-CLIENT-01 |
|---|---|---|
| `docs/architecture/ADR-0003-platform-identity-game-gateway-and-admission-boundary.md` | Platform Identity owns reusable credentials and one-time Game Login Tickets; Platform-owned Game Gateway redeems the ticket, applies World Registry route policy and returns selected endpoint/channel/revisions plus short-lived pre-admission material. Final canonical `GameSessionId`/`CharacterLease` authority remains game-domain/FND-04. | Client architecture must preserve the entire pre-admission chain and must not shortcut directory selection directly into final admission or move final gameplay authority into Platform/Gateway. |
| `docs/architecture/ADR-0011-native-client-pre-protocol-migration-state.md` | The canonical migrated client is a launchable Windows-first production shell and gameplay is intentionally unavailable/fail-closed. | Alpha architecture must evolve this shell; it must not fabricate a native gameplay path or consume a one-shot gameplay credential for a path that cannot complete. |
| `apps/client/src/lib.rs` | `ClientBootstrap` composes runtime/renderer/input and `request_gameplay_entry()` returns `NativeProtocolUnavailable`. | Preserve a single product composition root and an explicit gameplay-availability gate. |
| `apps/client/src/windows_shell.rs` | The current product shell is a real `winit` Windows window with renderer resize/render/suspend/close handling and `--smoke`. | Interactive lifecycle is already concrete; later UI belongs above this OS/render adapter boundary. |
| `crates/client-runtime/src/lib.rs` | The client owns an async runtime and cancellation/shutdown boundary. | Runtime ownership belongs to application composition, not individual screens/providers; current thread count is implementation detail, not new architecture authority. |
| `crates/platform-client/src/lib.rs` | Platform directory traffic is bounded. Its recursive forbidden-field check rejects only the 12 literal keys `host`, `port`, `endpoint`, `endpoint_uri`, `protocol`, `protocol_profile`, `ticket`, `credential`, `game_session`, `admission`, `route`, `address`; it is not a complete-schema or generic unknown-field rejection mechanism. | Preserve the bounded directory/Gateway separation, but do not claim compound unknown keys such as `game_session_id`, `admission_token` or `selected_endpoint` are currently rejected unless implementation evidence proves it. A stronger complete-schema boundary is a future requirement, not current runtime truth. |
| `workspace-boundaries.toml` | `client-domain`, `client-simulation`, synthetic assets and synthetic harness are classified synthetic, not production. | Do not silently add them to the shipping client or call synthetic proof native gameplay proof. |
| `crates/client-domain/src/lib.rs` | Existing synthetic model explicitly describes itself as a protocol-neutral, non-authoritative client projection. | Its semantics are useful evidence for projection separation, but production promotion requires separate authorization. |
| `crates/client-simulation/src/lib.rs` | Existing synthetic simulation deterministically mutates only the non-authoritative projection. | It is not a server-authority substitute and is not a prediction contract. |
| `docs/architecture/FND-02_PROTOCOL_OTERYN_V1_CONTRACT.md` | Server authority, `connection_generation`, server sequence, state revisions, snapshot/delta/resync and bounded parsing are accepted wire semantics. FND-02 also requires independent byte/malformed/property/fuzz/cross-version evidence beyond shared production codecs. | Future ingress/reconciliation must enforce these semantics before presentation state changes, and Tier-1/shared-code E2E cannot be the only wire-correctness oracle. |
| `docs/architecture/FND-04_IDENTITY_GAME_SESSION_ADMISSION_CHARACTER_LEASE_CONTRACT.md` | Final gameplay admission/control belongs to current Oteryn-v2 game-domain authority; client evidence is corroborative only. | Local “session state” is an observation/progress state machine, never authority to create/restore a GameSession or lease. |
| `docs/contracts/PROTOCOL_OTERYN_TRANSPORT_POLICY.json` | TCP profile 1 is registered architecturally, but gameplay transport adapter/listener/native-client entry are unavailable; all client modes have `runtime_available_now=false`. | No player-visible transport selector or optimistic runtime capability may appear before exact implementation/proof. |
| `docs/architecture/DUR-04_CONTENT_WORLD_AND_SCRIPTING_CONTRACT.md` | Client artifacts are explicit allowlisted projections; incompatible revision/capability combinations fail closed; client content is non-authoritative. | Client content loading, including audio assets, needs immutable compatible projection activation, not “load anything and hide server fields”. |
| `docs/architecture/CLIENT_CRASH_DIAGNOSTICS_PRIVACY_OWNER_BASELINE.md` | Eligible crash uploads are enabled by default with a persistent global opt-out; payloads are allowlisted/redacted/bounded and optional diagnostics cannot gate gameplay. | Diagnostics must be isolated from authority and respect durable privacy state before upload/retry. |
| `docs/architecture/ADR-0007-native-end-to-end-test-platform.md` | Three E2E tiers are accepted: headless production-protocol system E2E, instrumented native client, production-binary smoke. | Alpha client proof must include all relevant tiers; synthetic-only evidence cannot close gameplay readiness. |
| `docs/architecture/ARCHITECTURE_REVIEW_REFINEMENTS_2026-08-07.md` | Owner-accepted protocol evidence refinement says shared Tier-1 production schemas/codecs cannot be the only proof; canonical byte goldens, malformed/adversarial corpus, properties, fuzzing, cross-version fixtures, limits and stable failures are required as applicable. | Tier 1 remains allowed to share production codecs, but it must be complemented by independent evidence rather than a duplicated production stack. |
| `docs/architecture/ARCHITECTURE_REVIEW_REFINEMENTS_2026-08-10.md` | External alpha requires maintained threat model, signed artifacts/updater and provenance/SBOM strategy, SRE baselines and player-readable operational states. | Packaging/update/security/operability are release gates, not post-alpha cleanup. |
| `docs/architecture/GAME-VISION-01_EVOLVED_RELIABILITY_UX_FIRST_OWNER_BASELINE.md` | Reliability/UX-first is owner-accepted for first Evolved differentiation; semantic-neutral quality may be shared with Reference. | Client reliability, scaling/input clarity, errors and recovery UX should remain shared foundation unless they intentionally change product semantics. |

## 4. Problem statement

The migrated shell has correct fail-closed behavior but does not yet have the architecture needed to connect future product flows safely. The risk is not primarily renderer selection. The risk is accidental authority leakage while implementation grows:

1. a screen could directly call Platform and later gameplay networking, coupling UI lifecycle to authority-bearing credentials;
2. a local client model could be mistaken for authoritative simulation or accept deltas over a missing revision;
3. Platform world/character selection could be coupled directly to final admission or gameplay route material, bypassing ADR-0003 Game Login Ticket redemption and Game Gateway route-selection security boundaries;
4. update/content mutation could change assets or client semantics under a live session without a compatible revision boundary;
5. crash/logging convenience could capture credentials, private chat or unrelated files;
6. a test harness could bypass normal admission/networking or validate client/server with one shared defective codec and still be reported as end-to-end/wire proof;
7. a current architectural transport target could be rendered as a runtime option despite no adapter existing;
8. screen/gameplay code could directly own an audio device or use audio timing/cue success as gameplay state, while unbounded voice/buffer queues create product instability.

The architecture must make these errors structurally difficult while remaining reversible on technologies that are not yet evidenced.

## 5. Architectural invariants

The following invariants are proposed for coordinator acceptance:

1. **One production composition root.** `apps/client` owns construction, lifecycle and shutdown of production client subsystems. Screens do not construct infrastructure.
2. **UI is not authority.** Screens render view state and emit semantic intent; they do not create GameSession, lease, route or authoritative gameplay mutations.
3. **Directory selection cannot bypass Game Gateway.** Platform Identity/directory selection is followed, when gameplay exists, by one-time Game Login Ticket issuance, Platform-owned Game Gateway ticket redemption and authorized route/revision/pre-admission selection before `protocol-oteryn` transport/bootstrap. Final GameSession/CharacterLease admission authority remains game-domain/FND-04, never Platform/Gateway.
4. **Local session state is observational.** It records what the client has requested/observed, never what the server “must” consider authoritative.
5. **Protocol ingress precedes projection mutation.** Connection-generation fencing, message/server sequencing, state revisions and snapshot barriers are validated before authoritative observations are reflected in the local projection.
6. **Revision mismatch fails closed.** Missing/contradictory state triggers resync or terminal handling according to FND-02; no guessed delta application.
7. **Input becomes intent before wire.** OS events are normalized by input adapters into semantic actions; future gameplay egress maps allowed intent to protocol commands only while an accepted binding is active.
8. **Client content is a projection.** Only verified, client-safe, compatible projected artifacts are exposed to renderer/UI/audio; server-only content never enters the client package merely to be hidden later.
9. **Active release payload is immutable.** A running gameplay client does not self-mutate its executable/content payload under an active session.
10. **Optional diagnostics never grant or deny gameplay authority.** Crash/log upload state is independent of server security/audit evidence and cannot be used as an admission prerequisite.
11. **Capabilities are evidence-backed.** UI exposes gameplay/transport functionality only when an implementation-backed capability state says it is available; architectural targets alone are not runtime features.
12. **Tests do not create a hidden product path or sole common-mode oracle.** Test-only adapters may observe/invoke normal client paths but cannot forge admission, mutate server state, inject authoritative snapshots or ship enabled by default; shared production codecs must be supplemented by the independent wire evidence required by FND-02.
13. **Audio is presentation only.** Audio device/cue state never owns gameplay legality, RNG, simulation, admission, protocol sequencing or authoritative state; audio failure degrades presentation rather than gameplay authority.

## 6. Recommended logical architecture

### 6.1 Interactive product path

```text
Windows process / event loop
        |
        v
apps/client composition root
        |
        +--> application lifecycle + cancellation
        +--> configuration/privacy/release state
        +--> Identity + Platform directory provider
        +--> client-safe content provider
        +--> audio provider/device adapter
        +--> diagnostics/crash provider
        +--> future Game Login Ticket + Gateway/pre-admission provider [UNAVAILABLE NOW]
        +--> future gameplay transport provider                    [UNAVAILABLE NOW]
        |
        v
application/screen coordinator
        |
        +--> screen/view models ------------------------------+
        |                                                     |
        +--> semantic client intent                           v
        |                                               renderer adapter
        +--> local session-observation state                  ^
        |                                                     |
        +--> non-authoritative world/presentation projection--+
        |                                                     |
        +--> presentation events ----------------------> audio adapter
        ^
        |
future protocol reconciliation boundary
        ^
        |
future protocol ingress/egress [ADR-0003/FND-02/FND-04 authority preserved]
```

The names above are responsibility labels, not frozen Rust trait/crate/API names. The Gateway/pre-admission provider label does not move the external Platform-owned Gateway into this repository or make it a gameplay authority; it denotes only the future client-side seam that invokes the accepted control-plane boundary. The audio label is likewise a presentation adapter boundary, not a gameplay/event authority.

### 6.2 Dependency direction

The preferred dependency direction is:

```text
OS/window/input/network/filesystem/audio-device adapters
             -> application ports
             -> client application state / projection semantics
             -> immutable view/presentation data
             -> renderer/UI/audio presentation
```

Infrastructure may satisfy application ports. Application/screen logic must not depend on concrete socket, TLS, installer, registry, crash-upload, audio-backend or renderer-backend details.

### 6.3 Current-to-target mapping

- `apps/client`: remains the production composition root and owner of interactive lifecycle, including future audio provider/device lifetime.
- `crates/client-runtime`: remains an application-owned async execution/cancellation facility; no screen-specific runtime ownership.
- `crates/platform-client` + `crates/identity`: remain bounded Platform/Identity adapters and must not absorb gameplay transport or be treated as the final game-domain admission owner.
- `crates/input-platform`: remains physical/OS event translation.
- `crates/input-actions`: remains the semantic-action boundary suitable for UI and future gameplay intent.
- `crates/renderer`: remains presentation infrastructure; it receives renderable state and never owns network/session authority.
- audio implementation: no production library/crate/backend is selected here; when implemented it belongs behind an application-owned presentation provider/device adapter and consumes only client-safe compatible assets/presentation events.
- `crates/client-domain` and `crates/client-simulation`: remain synthetic under current workspace classification. A later implementation may propose promoting or replacing suitable non-authoritative projection semantics, but this task does not change their production status.
- `tools/synthetic-client-harness`: remains synthetic/non-release and is **not** equivalent to ADR-0007 Tier 1 headless system E2E.

## 7. Application and screen composition

The client should separate **navigation/application state** from **server-authoritative world state**.

Conceptual application states may include:

```text
Bootstrapping
InteractiveNoIdentity
IdentityFlowPending
DirectoryReady
GameplayUnavailable            # current canonical path

# Future states; not runtime-authorized by this document:
AdmissionPending
TransportConnecting
WorldSynchronizing
WorldActive
Reconnecting
TerminalSessionFailure

ShuttingDown
```

These labels are descriptive, not a frozen enum. The important rule is that transition to a future gameplay-active presentation requires positive evidence from the owning Gateway/admission/transport/protocol path; it cannot be inferred from a selected character, valid pre-admission material, an open socket or a UI button press.

Screens should consume immutable/read-only snapshots of application/view state and emit semantic commands such as “start identity flow”, “select world/channel/character”, “request gameplay entry”, “move north”, “use selected action”, “open settings”, or “shutdown”. The coordinator decides whether an intent is legal in the current local state and delegates to the owning provider. Server legality remains server-owned.

Player-facing operational states should be distinguishable without exposing secrets: world/channel selection, gameplay unavailable, queue/maintenance when later provided by authority, reconnect progress, version/content incompatibility, session conflict, audio unavailable/degraded and bounded transport diagnostics. Error wording must not invent topology or credential detail that upstream contracts intentionally hide.

## 8. Future gameplay admission and transport seam

### 8.1 Separation from Platform directory and preservation of ADR-0003

The current Platform client recursively rejects only the 12 literal forbidden key names `host`, `port`, `endpoint`, `endpoint_uri`, `protocol`, `protocol_profile`, `ticket`, `credential`, `game_session`, `admission`, `route`, `address`. It does not currently prove complete-schema rejection of unknown compound gameplay-shaped keys. Preserve the existing fail-closed denylist safety property without overstating it, and require stronger complete-schema/reject-unknown-field evidence before claiming a broader boundary. In every case, directory selection is not an alternate route to final admission.

A future **fresh-entry** client flow should conceptually be:

```text
Platform Identity
        -> bounded Platform directory + player selection intent
        -> one-time Game Login Ticket
        -> Platform-owned Game Gateway
             -> ticket redemption through Platform authority
             -> bounded supported protocol offer where applicable
             -> World Registry route/channel policy
             -> selected endpoint/channel/revisions
             -> short-lived pre-admission session/admission material
        -> FND-02 protocol-oteryn transport/bootstrap to the selected endpoint
        -> final game-owned FND-04 admission
             -> validate/consume pre-admission route/revision bindings
             -> acquire/validate CharacterLease
             -> establish canonical GameSession authority only after successful final admission
        -> synchronized non-authoritative world projection
```

The Gateway is a pre-admission/control-plane security boundary. It does not own the canonical logical `GameSessionId` or `CharacterLease`, and a successful ticket redemption/route selection/open transport is not proof of final gameplay admission. The exact client API, token shape, route field and provider implementation are **not** selected here.

### 8.2 Runtime capability gate

The client must have one semantic runtime capability view that distinguishes at least:

- architecture target exists;
- implementation exists;
- locally usable under the current build/configuration;
- currently authorized by accepted control-plane policy.

A target transport mode is not “available” merely because it appears in ADR-0014 or the transport policy. Under the verified current policy every gameplay mode is unavailable. Therefore the current client continues to present gameplay unavailability rather than a transport picker and fails before requesting/consuming a one-time ticket for an unavailable path.

If future transport modes become available, UI selection must consume the accepted capability/policy result. It must not itself implement fallback/downgrade logic. Fallback requires a fresh authorized attempt under the accepted transport/admission contract.

## 9. Protocol ingress, egress and reconciliation

### 9.1 Egress

The application produces semantic player intent. A future gameplay command adapter may encode that intent only after:

- an accepted current GameSession/binding has been observed;
- the current `connection_generation` is active;
- the command is legal to send under client-side phase/shape checks;
- protocol resource limits can be satisfied.

Client-side checks improve UX and bound malformed output; they do not replace server legality. `CommandId` ownership/order, retry and duplicate semantics remain those accepted by FND-02/FND-03/FND-04.

### 9.2 Ingress

Before any server message changes the local world projection, the future ingress boundary must enforce the accepted protocol rules relevant to the message:

1. transport/frame/schema validation;
2. current connection-generation fence;
3. accepted message phase/direction/type;
4. server-sequence ordering where applicable;
5. state-domain revision preconditions;
6. snapshot begin/chunk/commit barrier semantics where applicable;
7. bounded payload/resource handling.

Stale-generation traffic is discarded as specified by FND-02. A delta whose `base_revision` does not equal the client's current domain revision is not applied; the client requests/enters resynchronization instead. Missing authoritative deltas are never fabricated.

### 9.3 Local projection model

The production gameplay client needs a protocol-neutral **non-authoritative projection** concept, but this analysis deliberately does not decide whether the existing synthetic `oteryn-client-domain` types are promoted, replaced or split.

The projection may contain only state required to present and interact with the current client-visible world. It may additionally hold purely local presentation state such as selection/highlight, camera interpolation, UI panel state and pending-input indicators. Those local fields must be distinguishable from server-observed state so they cannot be serialized back as authority or used to “repair” authoritative mismatch.

### 9.4 Prediction and correction

No gameplay prediction/rollback algorithm is accepted by this task. Visual interpolation and input responsiveness may be studied later, but any speculative representation of gameplay outcome must remain reversible presentation state and must never advance authoritative `StateRevision`, `server_sequence`, `CommandId` result or GameSession/lease state.

## 10. Renderer, UI, input and audio integration

### 10.1 Main-thread shell boundary

The Windows event loop/window lifecycle remains at the process shell boundary. OS/window events must be handled without moving Identity/network/content authority into `windows_shell.rs`.

The interactive lifecycle must handle, at minimum, the already-relevant classes of:

- initial window/surface creation;
- DPI/scale and resize changes;
- focus and text/IME input;
- suspend/minimize/resume;
- graphics device/surface loss and recreation where supported by renderer implementation;
- close/shutdown and async cancellation.

Exact framework APIs remain implementation choices.

### 10.2 Semantic input

Physical keyboard/mouse/controller/text events should continue through `input-platform` -> `input-actions` before application/gameplay dispatch. Screen-specific routing decides whether a semantic action is consumed by UI navigation, text editing or future gameplay intent. Raw OS key codes must not become protocol commands.

### 10.3 UI/render model

UI widgets and renderer passes should receive view models/render descriptions derived from local application/projection state. They may keep renderer-local caches (GPU resources, glyph atlases, layout caches), but those caches have no gameplay/session authority and can be reconstructed after device/surface loss.

No exact retained/immediate UI framework is selected here. The current `winit`/renderer foundation is preserved until evidence requires supersession.

### 10.4 Audio presentation boundary

Audio belongs to the same presentation side of the client authority boundary as rendering. It may consume already-accepted server observations and local UI/presentation events and translate them into requested cues, but it must not decide whether an ability landed, a timer fired authoritatively, RNG succeeded, a session exists, or any gameplay mutation is legal. Missing, delayed, dropped or muted audio can affect presentation only.

The `apps/client` composition root owns construction and lifetime of the audio provider and selected output-device/session adapter. Screens, widgets and gameplay reducers do not open devices or hold long-lived backend state. Device initialization/reopen and shutdown must be bounded and coordinated with application lifecycle.

Audio assets must be part of the client-safe allowlisted content projection and remain compatible with the active build/content/release revision. Incompatible/missing audio assets fail or degrade as presentation; they do not cause content semantics to be guessed. Staged updates must not mutate the active audio asset set in place for the running release/session compatibility context.

Resource use must be bounded: active voices/streams, decoded or streaming buffers, queued cue requests and device-recovery work all need explicit implementation ceilings. Numeric limits remain evidence-driven. Audio device loss, unavailable output or unsupported format must degrade to a recoverable muted/degraded state without blocking gameplay authority or corrupting client state.

Settings/accessibility must support an extensible semantic control surface such as master/effects/music classes or equivalent, mute, output selection where supported and correct operation when audio is disabled. Exact category taxonomy and UI are not frozen here.

The native-client implementation/test owner must prove lifecycle/recovery, bounded resources, client-safe asset/revision compatibility, presentation-event-to-cue mapping where deterministic behavior is claimed, settings persistence/accessibility and non-interference with gameplay authority. No audio library, mixer/codec stack, OS backend or vendor is selected by this architecture gate.

## 11. Client-safe content projection and patch boundary

DUR-04 requires an explicit allowlisted client projection. The alpha client should therefore treat content, including audio assets, as a verified artifact set rather than loose source files.

Recommended lifecycle:

```text
installed/staged candidate projection
        -> integrity + compatibility verification
        -> immutable local availability
        -> selected active projection revision
        -> session/view/audio consumption
```

Rules:

- server-only fields must be absent from client artifacts rather than hidden at UI time;
- content/build/protocol/capability compatibility metadata must be checked before activation;
- unknown/incompatible required semantics fail closed;
- an active gameplay session pins the accepted client projection/release compatibility context needed for that session;
- staged updates do not mutate the active gameplay projection or audio asset set in-place;
- hot-reload of gameplay-relevant content is not authorized by this task;
- caches may be deleted/rebuilt, but authoritative or compatibility meaning may not depend on cache survival.

The physical bundle encoding, patch delta format, CDN and signing mechanism remain DUR/release decisions.

## 12. Configuration and filesystem model

The Windows client should distinguish four logical storage classes:

1. **Installed release payload** — executable/libraries/resources and shipped client-safe content; immutable while running.
2. **Durable per-user state** — user settings, privacy choice, accessibility/input/layout/audio preferences and versioned migration metadata.
3. **Rebuildable cache** — derived/download staging/cache data that can be removed without changing authority.
4. **Diagnostic spool** — bounded local logs/crash packages subject to redaction/privacy/upload policy.

The implementation should resolve Windows paths through OS-appropriate known-folder/platform abstractions instead of scattering hard-coded `%APPDATA%`, registry or install paths through domain/UI code. Exact directories and installer registry behavior are deferred.

Configuration needs an explicit schema/version and atomic migration/write behavior. A failed migration must preserve a recoverable prior configuration or fall back to safe defaults without silently re-enabling opted-out diagnostics.

Reusable Identity credentials, gameplay admission material and reconnect secrets must not be stored in general configuration, logs, crash packages or content caches. Whether any reusable sign-in material later uses a Windows credential facility is an Identity/security decision, not selected here.

## 13. Logging and crash diagnostics

The accepted privacy baseline is binding input to the client architecture:

- automatic eligible crash diagnostics are enabled by default;
- a clear global durable opt-out must exist;
- opt-out stops new uploads and pending automatic retries while disabled;
- optional client diagnostics cannot block launch/authentication/gameplay;
- opt-out itself is not abuse evidence;
- collection is allowlist-based and redaction occurs before transmission;
- reusable credentials, tokens, private chat content, arbitrary files and unrelated personal data are excluded;
- client diagnostics remain untrusted corroborating evidence, not server authority.

Architecture consequence: logging/crash collection should receive structured, already-classified diagnostic events rather than unrestricted access to every application object or filesystem directory. Secret-bearing provider types should not implement casual debug serialization into logs.

A bounded local crash package may include build/content/protocol identifiers, failure category and narrowly scoped client/session correlation identifiers only where the accepted diagnostics/privacy contracts allow them. Exact retention, upload endpoint and uploader implementation are deferred.

## 14. Update, packaging, install and rollback

### 14.1 Immutable running version

Release mutation must occur outside an active authority-bearing gameplay process. A running client may detect that an update is required/available and may stage non-active bytes through a dedicated release/update boundary, but it must not replace its live executable/content/audio semantics mid-session.

### 14.2 Verification boundary

Before external alpha, accepted programme refinement requires signed artifacts/updater plus provenance/SBOM strategy. Therefore a future updater/installer must have a verification step before candidate bytes become an activatable release. This document does not select the signature format, trust-root layout or build service.

### 14.3 Atomic activation and rollback

Activation should be versioned and atomic at release granularity: either the verified candidate becomes the next active client release, or the previous known-good release remains available. Partial executable/content mixes must not be launched as a valid release.

Rollback is allowed only to a verified release that remains compatible with the current Platform/protocol/content policy. “Last file on disk” is not sufficient rollback authority.

### 14.4 Packaging

The current supported product target remains Windows-first `x86_64-pc-windows-msvc`. The install package should keep immutable release payload separate from user configuration/cache/diagnostics so update/uninstall does not silently destroy user privacy/settings state unless an explicit user action or retention rule requires it.

MSI/MSIX/portable bootstrapper/updater framework, install scope, Start Menu integration, code-signing provider and patch algorithm remain deliberately undecided.

## 15. Headless versus interactive paths

`ADR-0007` already decides that one test shape is insufficient.

### 15.1 Tier 1 — future headless system E2E client

A future QA-owned headless client must traverse the same accepted product authority chain as the native client:

```text
Platform Identity
-> one-time Game Login Ticket
-> Platform-owned Game Gateway ticket redemption/route selection
-> selected endpoint/channel/revisions + short-lived pre-admission material
-> FND-02 protocol-oteryn transport/bootstrap
-> final game-owned FND-04 admission / CharacterLease / GameSession authority
-> authoritative game server
```

It may reuse the same accepted production protocol schemas/codecs, sequencing and admission contracts as the native client and has no renderer, audio-device or physical input requirement, but it cannot call authoritative domain mutation APIs directly.

Shared production schemas/codecs are useful but cannot be the only proof of wire correctness. Independent evidence appropriate to FND-02 and the owner-accepted 2026-08-07 refinement must include, as applicable:

- canonical byte-level golden fixtures for framing and protocol messages;
- malformed/adversarial fixture corpus;
- property/invariant encode/decode round-trip tests;
- fuzzing of externally controlled framing/decoders/parsers;
- cross-version compatibility fixtures for permitted same-major evolution;
- explicit resource ceilings and stable failure classes/dispositions.

This independent proof requirement does **not** require or authorize a second production protocol implementation or duplicated production stack.

This is a **real network/system client**, not the current synthetic harness.

### 15.2 Tier 2 — instrumented interactive native client

The real `apps/client` product path may later expose a test-only adapter that invokes normal input/client-command paths and reads semantic observations/screenshots/logs. It must not forge sessions, teleport/mutate server state, inject snapshots/results or exist in production-default builds. When audio is implemented, Tier 2 owns representative real-client audio lifecycle/settings/degradation evidence without using audible output as a gameplay oracle.

### 15.3 Tier 3 — release-binary smoke

The exact release candidate must prove startup through supported product entry, world entry and bounded gameplay/relog/cleanup once native gameplay exists. The current `--smoke` window-launch path is useful shell evidence but does not by itself satisfy the future Tier 3 gameplay journey. Shipped audio presence/degradation may be checked as a presentation/release property, not as gameplay authority proof.

### 15.4 Existing synthetic harness

`tools/synthetic-client-harness` and synthetic projection/simulation crates remain useful deterministic rendering/input/migration fixtures. Their current workspace classification and absence of production networking mean they must not be reported as Tier 1 protocol E2E or native gameplay proof.

## 16. Windows-first assumptions

Windows is the first supported interactive product target. Architecture should optimize that path while confining OS-specific behavior to adapters so future platforms are not impossible by construction.

Alpha Windows concerns that belong to the shell/platform layer include:

- window/event-loop and graphics surface lifecycle;
- DPI/scaling and multi-monitor transitions;
- focus/capture and keyboard layout behavior;
- text/IME integration;
- audio output-device/session lifecycle and device-change recovery;
- filesystem known-folder resolution and permissions;
- process activation/exit/update handoff;
- crash capture integration;
- code-signing/installer presentation and SmartScreen/reputation considerations once release tooling is selected.

No Linux/macOS parity commitment or platform abstraction rewrite is made here.

## 17. Production-readiness gates

Architecture acceptance is not runtime readiness. Suggested alpha gate model:

### Gate AC0 — current pre-native shell

Already supported by accepted migration evidence:

- real Windows shell launches;
- renderer/input/runtime initialize;
- Platform directory boundary remains bounded to the current parser contract;
- gameplay entry fails closed before requesting/consuming one-time gameplay authority material, route selection or gameplay network use;
- shutdown is bounded.

This gate does **not** prove native gameplay or audio implementation.

### Gate AC1 — client product substrate implemented

Future implementation evidence should prove:

- versioned safe configuration/privacy persistence;
- client-safe content artifact verification/activation;
- audio provider/device lifetime, bounded-resource behavior, client-safe asset compatibility, degradation/recovery and settings/accessibility when audio is shipped;
- structured bounded diagnostics/crash spool/redaction;
- release identity/version compatibility presentation;
- Windows packaging/install/update/rollback mechanics on disposable environments;
- no gameplay capability is surfaced when unavailable.

### Gate AC2 — native gameplay entry implemented

Blocked until owning programmes provide and prove at exact revisions:

- ADR-0003-compatible Platform Identity -> one-time Game Login Ticket -> Platform-owned Game Gateway ticket redemption/route selection -> selected endpoint/channel/revisions + short-lived pre-admission material client integration;
- gameplay transport adapter to the Gateway-selected endpoint;
- FND-04 final game-owned admission/reconnect and CharacterLease integration without transferring canonical GameSession authority to Platform/Gateway;
- FND-02 protocol codec/registry/reconciliation implementation;
- FND-02 independent wire evidence beyond a shared production codec oracle;
- authoritative server endpoint;
- compatible content projection;
- client reconciliation path;
- Tier 1 and Tier 2 E2E across real boundaries.

Only then can gameplay availability change from `pre-native-protocol` for a supported build. Audio success is never a precondition for game-domain authority.

### Gate AC3 — external alpha release readiness

In addition to gameplay proof, external alpha needs the accepted security/SRE/release controls: threat model, signed artifacts/updater, provenance/SBOM strategy, incident/update rollback, required SLO/runbooks/operational states, privacy/legal review and Tier 3 release-candidate smoke.

This document does not declare any of AC1–AC3 implemented.

## 18. Required failure behavior

The client should fail closed and remain diagnostically clear for at least these classes once the owning components exist:

- gameplay runtime unavailable -> remain in non-gameplay UI; do not request/consume a one-time gameplay ticket, route or network attempt for a path that cannot complete;
- Game Gateway ticket redemption/route selection unavailable or rejected -> no direct directory-to-game-server bypass;
- unsupported protocol/transport/content capability -> no world activation;
- TLS/service identity/ALPN rejection -> terminal connection error; no same-authority downgrade;
- stale connection generation -> stale binding cannot mutate local active projection;
- server-sequence/revision gap -> reconciliation/resync instead of guessed application;
- snapshot incomplete/invalid -> no commit of partial snapshot;
- session/lease conflict -> show bounded user state while server remains authoritative;
- content/release mismatch -> do not enter incompatible world;
- renderer/device loss -> rebuild presentation resources without changing gameplay authority;
- audio initialization/device/asset failure -> degrade to bounded muted/unavailable presentation and recover where supported without changing gameplay authority;
- config corruption/migration failure -> safe recoverable defaults/backup strategy without secret leakage or privacy reset;
- diagnostics opted out -> no automatic upload/retry, no gameplay impact;
- update activation failure -> preserve/recover verified previous release rather than launch a mixed payload.

## 19. E2E evidence matrix

| Concern | Unit/component | Tier 1 headless system | Tier 2 native | Tier 3 release |
|---|---|---|---|---|
| semantic input routing | required | semantic command coverage | required through normal input path | representative smoke |
| fail-closed unavailable gameplay | required | N/A before protocol runtime | required | required for pre-native builds |
| ADR-0003 ticket/Gateway route/pre-admission chain + final FND-04 authority | adapter negative tests | required | required | required |
| admission/session binding | adapter negative tests | required | required | required |
| connection generation / sequencing / revision resync | property/golden/negative | broad required | representative required | representative |
| independent FND-02 wire oracle | byte goldens + adversarial/property/fuzz/cross-version/limits/failures | required alongside shared-code E2E | representative product-code confirmation | packaged compatibility evidence |
| renderer/UI operational state | renderer/view tests | N/A | required | required smoke |
| audio lifecycle/content/degradation/settings | provider/asset/settings tests | N/A to game-authority proof | required when shipped | packaged presentation smoke/degradation |
| client-safe content compatibility | loader/manifest tests | exact revision declared | required | exact packaged artifact |
| disconnect/reconnect | reducer/adapter tests | broad fault injection | required user journey | release smoke/relog |
| diagnostics opt-out/redaction | required | N/A | required settings/restart path | packaged smoke as appropriate |
| updater/install/rollback | package harness | N/A | install-to-native journey | release gate |
| cleanup/evidence envelope | required where applicable | mandatory | mandatory | mandatory |

No hidden retry converts a failed attempt into a pass. Exact client/server/Platform/protocol/content/build revisions must be retained according to ADR-0007. Shared production schemas/codecs may be exercised by Tier 1 and Tier 2, but they are not sufficient alone to satisfy FND-02 wire correctness; independent evidence must remain capable of catching common-mode encode/decode defects without becoming a second production implementation. Audio evidence remains presentation evidence and is never substituted for game/protocol proof.

## 20. Security and privacy consequences

The client is an untrusted edge from the server's perspective and a secret-bearing process from the player's perspective. The architecture therefore minimizes authority and exposure:

- server never trusts client projection as gameplay truth;
- client does not keep general-purpose copies of admission/reconnect secrets;
- directory data, one-time Game Login Ticket/Gateway pre-admission material and final game-domain authority remain different trust classes;
- Game Gateway route selection cannot be bypassed by direct client endpoint choice;
- protocol parsing is bounded before expensive work and validated through independent malformed/property/fuzz evidence;
- content, including audio assets, is allowlisted and compatibility-checked;
- audio events/devices cannot confer gameplay rights or become a hidden RNG/timing oracle;
- update activation requires verified provenance/signing policy before external alpha;
- test-only control surfaces are compile/profile-bounded and cannot ship enabled by default;
- logs/crash packages are structured, bounded and redacted before transmission;
- optional diagnostics cannot be security authority or abuse-score input;
- renderer/UI/audio caches and devices are reconstructable and cannot confer gameplay rights.

## 21. Recommendation decision tests

### R1 — keep `apps/client` as the sole production composition root

- Must decide now? **YES**.
- Downstream blocked without it: screen/provider implementation and lifecycle ownership.
- Becomes harder later: multiple screens/providers can create runtimes/network clients and circular shutdown ownership.
- Superseding evidence: a proven process/deployment/security boundary requiring another production process/composition root.
- Deliberately not decided: exact module/DI framework or Rust trait layout.

### R2 — separate application/screen coordination from adapters

- Must decide now? **YES**.
- Downstream blocked: UI navigation, headless/native testability and provider substitution.
- Becomes harder later: networking/filesystem/render APIs leak into UI state and become expensive to disentangle.
- Superseding evidence: a simpler proven architecture that preserves the same authority/test boundaries with less coupling.
- Deliberately not decided: retained-mode versus immediate-mode UI toolkit.

### R3 — preserve directory -> ticket/Gateway -> gameplay transport -> final admission authority boundaries

- Must decide now? **YES**; ADR-0003 is already accepted parent authority.
- Downstream blocked: safe login-to-gameplay vertical slice.
- Becomes harder later: Platform directory begins carrying forbidden gameplay route/secret semantics or client code bypasses one-time ticket redemption/Gateway route selection; alternatively Gateway pre-admission material can be mistaken for final GameSession/lease authority.
- Superseding evidence: an accepted ADR/FND/Platform cross-repository contract explicitly changes ownership while preserving or strengthening the same security invariants.
- Deliberately not decided: future provider API, ticket/token, route or grant representation.

### R4 — use a non-authoritative client projection behind FND-02 reconciliation

- Must decide now? **YES**.
- Downstream blocked: gameplay UI/render state, reconnect and resync implementation.
- Becomes harder later: local state mutations become indistinguishable from server observations.
- Superseding evidence: a later accepted protocol/client model with equally explicit authority/revision semantics and proven migration.
- Deliberately not decided: production crate/type promotion and gameplay prediction algorithm.

### R5 — keep raw OS input and UI widgets out of protocol egress

- Must decide now? **YES**.
- Downstream blocked: deterministic input mapping and Tier 2 E2E control.
- Becomes harder later: key codes/widget callbacks become wire-coupled commands.
- Superseding evidence: none expected short of replacing the accepted input/action architecture with an equivalent tested semantic boundary.
- Deliberately not decided: complete keybind/controller/accessibility policy.

### R6 — require verified immutable client-safe content activation

- Must decide now? **YES**.
- Downstream blocked: content loader, version mismatch UX and release packaging.
- Becomes harder later: loose content/server-only fields become part of shipping compatibility surface.
- Superseding evidence: a later DUR-04 contract with a different but equally explicit projection/compatibility mechanism.
- Deliberately not decided: bundle encoding, delta patch format, CDN and signing format.

### R7 — separate installed payload, durable user state, rebuildable cache and diagnostic spool

- Must decide now? **YES**.
- Downstream blocked: installer/updater, privacy persistence and supportability.
- Becomes harder later: uninstall/update can destroy settings or mix mutable data with signed payload.
- Superseding evidence: platform packaging evidence showing a different layout preserves equivalent immutability/privacy semantics.
- Deliberately not decided: exact Windows paths/registry/install scope.

### R8 — make release activation atomic and outside an active gameplay process

- Must decide now? **YES** for the boundary, **NO** for updater technology.
- Downstream blocked: safe alpha package/update design.
- Becomes harder later: in-place partial updates become normalized and rollback becomes unreliable.
- Superseding evidence: a proven update mechanism that can safely guarantee equivalent atomicity and active-session immutability.
- Deliberately not decided: installer/updater vendor/framework, signature/trust-root implementation and patch algorithm.

### R9 — implement crash diagnostics as privacy-bounded optional client evidence

- Must decide now? **YES**, because owner baseline already exists.
- Downstream blocked: settings model, crash spool/uploader and privacy UX.
- Becomes harder later: unrestricted logging becomes entrenched before privacy controls.
- Superseding evidence: explicit owner/privacy/legal decision changing the accepted default/control while preserving security independence.
- Deliberately not decided: retention duration, upload backend and exact package schema.

### R10 — keep Windows-first OS details behind shell/platform adapters

- Must decide now? **YES** for boundary, **NO** for other-platform support.
- Downstream blocked: coherent Windows alpha implementation without cross-platform abstraction churn.
- Becomes harder later: OS APIs leak into application/session logic and prevent future porting/testing.
- Superseding evidence: accepted product decision to make the client permanently Windows-exclusive with measured simplification benefit.
- Deliberately not decided: Linux/macOS support dates or abstraction framework.

### R11 — preserve ADR-0007's three distinct E2E tiers plus FND-02 independent wire evidence

- Must decide now? **YES**; the three tiers and independent-protocol-evidence property are already accepted architecture.
- Downstream blocked: honest alpha proof, test-surface design and credible wire conformance evidence.
- Becomes harder later: synthetic/native/headless results become conflated, or shared client/server codec defects can pass both sides without an external byte/adversarial/property/fuzz oracle.
- Superseding evidence: explicit accepted architecture that supersedes ADR-0007/FND-02 evidence requirements with equivalent or stronger real-boundary and independent wire proof.
- Deliberately not decided: test orchestration/UI automation libraries and final directory/crate layout; no second production protocol implementation is selected.

### R12 — expose only implementation-backed runtime capabilities

- Must decide now? **YES**.
- Downstream blocked: transport/settings UX and pre-native fail-closed correctness.
- Becomes harder later: users/configs start depending on fake or unsupported modes.
- Superseding evidence: none expected; future capability availability changes through implementation/proof, not removal of this invariant.
- Deliberately not decided: exact capability-query API and future QUIC activation/default.

### R13 — keep audio application-owned, bounded and presentation-only

- Must decide now? **YES** for authority/lifetime/content/degradation boundaries, **NO** for implementation technology.
- Downstream blocked: safe audio provider/device implementation, release-content projection, settings/accessibility and Tier-2 evidence.
- Becomes harder later: screens/game reducers own devices, cue queues become unbounded, or audio timing/device success becomes coupled to gameplay state.
- Superseding evidence: a later accepted client architecture with an equivalent or stronger non-authoritative presentation/resource boundary and migration evidence.
- Deliberately not decided: audio library/vendor, codec/mixer stack, OS backend, exact voice/buffer limits and final UX taxonomy.

## 22. Alternatives considered

### A. Screen-owned services/networking

Rejected. It is fast initially but couples navigation/window lifecycle to credentials, retries and authority-bearing connection state and is difficult to test headlessly.

### B. Treat current synthetic client simulation as the production gameplay model now

Rejected. Workspace policy classifies it synthetic, it has no accepted FND-02 wire/reconciliation implementation, and this worker is not authorized to promote executable components.

### C. One “client state” store mixing UI, server state and protocol state

Rejected. It makes it too easy for UI/prediction/cache mutation to masquerade as authoritative revision progress.

### D. Let the gameplay process self-update in place

Rejected. It risks mixed executable/content versions and mutating semantics during an authority-bearing session.

### E. Only graphical E2E

Rejected by ADR-0007 because it is too slow/fragile for broad protocol and failure-injection coverage.

### F. Only headless E2E or shared-code round-trip as the only wire oracle

Rejected. Headless-only cannot prove Windows UI, renderer, input, packaging and release behavior; shared production codec on both ends also cannot independently detect common-mode wire defects required by FND-02.

### G. Show TCP/QUIC choices now because architecture names them

Rejected. Current machine-readable policy says every client mode is unavailable and there is no gameplay adapter/listener/entry implementation.

### H. Treat directory selection or Gateway pre-admission as final gameplay authority

Rejected by ADR-0003/FND-04. Directory selection must not bypass one-time ticket redemption and Game Gateway route selection, while Gateway pre-admission material authorizes only a bounded admission attempt; final GameSession/CharacterLease authority remains game-owned.

### I. Let gameplay/screens own audio devices or make audio success part of gameplay flow

Rejected. Audio is optional/degradable presentation, and coupling it to gameplay authority would create a client-side state/timing dependency unrelated to server truth while making lifecycle/resource recovery harder.

## 23. Risks requiring later evidence

- **Client/server version skew:** compatibility policy exists at protocol/content layers but release/update skew matrix still needs an implementation/release owner.
- **Credential lifetime in process memory:** exact secret container/zeroization/OS credential reuse policy requires security/Identity implementation review.
- **Renderer recovery:** actual device-loss and multi-monitor/DPI behavior requires Windows hardware evidence.
- **Audio device/backend variance:** device hotplug/loss, format support, latency and resource ceilings require Windows hardware/driver evidence after a backend is selected.
- **Content footprint/patch cost:** physical format, caching and delta strategy require measured bundle/download data.
- **Update trust:** signed-artifact/updater/provenance/SBOM requirement is accepted, but trust root/toolchain is not selected.
- **Gateway/client integration drift:** future client implementation must preserve one-time ticket redemption, selected route/revision bindings and pre-admission/final-admission separation across repository revisions.
- **Headless/native semantic drift and common-mode codec defects:** shared protocol/generated semantics need cross-tier scenario equivalence, while independent byte goldens, adversarial/property/fuzz and cross-version fixtures must remain capable of catching defects shared by both production endpoints.
- **Error information disclosure:** useful diagnostics must be reconciled with hidden topology/credential constraints.
- **Accessibility/input/audio policy:** architecture supports semantic input/audio settings separation but final product accessibility and category requirements remain later owner/product decisions.

## 24. DECISIONS_NOT_TAKEN

The following are deliberately **not** decided by ALPHA-CLIENT-01:

1. exact UI/widget framework or retained/immediate composition model;
2. replacement of `winit`, renderer backend or GPU API;
3. production promotion/replacement of `client-domain` or `client-simulation` crates;
4. gameplay prediction, interpolation, rollback or client-side combat simulation algorithm;
5. gameplay transport adapter implementation or activation of TCP/QUIC client modes;
6. QUIC profile ID, QUIC library, fallback timings or default promotion;
7. ADR-0003/FND-04 Game Gateway/pre-admission/admission/reconnect ticket/grant/token/route API representation or server authority semantics;
8. protocol message schemas, codec/TLS/protobuf Rust libraries or resource limits beyond accepted parent contracts;
9. physical client content bundle, patch/delta format, CDN or content-signing implementation;
10. exact Windows install directories, registry keys, install scope or installer technology;
11. updater framework, code-signing provider, trust-root mechanism or SBOM generator;
12. secure storage technology for any future reusable sign-in credential;
13. exact log/crash retention durations, upload backend or legal/privacy wording;
14. audio library/vendor, codec/mixer stack, OS device backend, exact voice/buffer/queue limits or final category taxonomy;
15. production timeouts, retry/backoff, cache/log/spool byte ceilings unless owned by an accepted parent contract;
16. Linux/macOS support commitment;
17. final accessibility/keybind/controller/audio UX policy;
18. release-channel/version-skew/mandatory-update product policy;
19. server/gameplay legality, persistence, simulation or balance behavior;
20. any second production protocol implementation for testing.

## 25. CROSS_DOMAIN_FINDINGS

```yaml
cross_domain_finding:
  id: ALPHA-CLIENT-01-XD-01
  observed_in_domain: native-client
  target_owner: protocol-network-runtime
  severity: P1
  evidence: docs/architecture/ADR-0016-gameplay-transport-client-mode-runtime-readiness.md + docs/contracts/PROTOCOL_OTERYN_TRANSPORT_POLICY.json
  conflict_or_gap: Accepted TCP profile semantics exist, but gameplay transport adapter/listener/native-client gameplay entry are not implemented and all client transport modes are runtime-unavailable.
  required_before: Any client build may advertise or exercise native gameplay entry or transport-mode selection.
  worker_action: REPORT_ONLY
```

```yaml
cross_domain_finding:
  id: ALPHA-CLIENT-01-XD-02
  observed_in_domain: native-client
  target_owner: admission-session-integration
  severity: P1
  evidence: docs/architecture/ADR-0003-platform-identity-game-gateway-and-admission-boundary.md + docs/architecture/FND-04_IDENTITY_GAME_SESSION_ADMISSION_CHARACTER_LEASE_CONTRACT.md + crates/platform-client/src/lib.rs
  conflict_or_gap: The accepted target chain is Platform Identity -> one-time Game Login Ticket -> Platform-owned Game Gateway ticket redemption/route selection -> selected endpoint/channel/revisions + short-lived pre-admission material -> protocol-oteryn transport -> final game-owned FND-04 admission/CharacterLease/GameSession authority, while the current production client intentionally implements none of that gameplay handoff beyond bounded directory selection. Current platform-client forbidden-field enforcement is the exact recursive 12-key denylist, not complete-schema unknown-field rejection.
  required_before: Real interactive Identity/directory selection can continue through Gateway pre-admission and authoritative gameplay admission/reconnect E2E without bypassing the security boundary or moving final authority into Platform/Gateway; any stronger directory-field rejection claim must have implementation/schema evidence.
  worker_action: REPORT_ONLY
```

```yaml
cross_domain_finding:
  id: ALPHA-CLIENT-01-XD-03
  observed_in_domain: native-client
  target_owner: content-release-toolchain
  severity: P1
  evidence: docs/architecture/DUR-04_CONTENT_WORLD_AND_SCRIPTING_CONTRACT.md
  conflict_or_gap: Client-safe projection and compatibility semantics are accepted, but physical client artifact/bundle encoding, patch distribution and production signing/activation implementation are deliberately not selected or implemented.
  required_before: Packaged gameplay client content can be updated and activated for external alpha.
  worker_action: REPORT_ONLY
```

```yaml
cross_domain_finding:
  id: ALPHA-CLIENT-01-XD-04
  observed_in_domain: native-client
  target_owner: security-release-sre
  severity: P1
  evidence: docs/architecture/ARCHITECTURE_REVIEW_REFINEMENTS_2026-08-10.md
  conflict_or_gap: External alpha requires signed artifacts/updater, build provenance/SBOM strategy, threat model and rollback/operability evidence, but this worker has no accepted concrete release trust/update implementation to consume.
  required_before: External alpha packaging/update channel is production-enabled.
  worker_action: REPORT_ONLY
```

```yaml
cross_domain_finding:
  id: ALPHA-CLIENT-01-XD-05
  observed_in_domain: native-client
  target_owner: qa-e2e
  severity: P2
  evidence: docs/architecture/ADR-0007-native-end-to-end-test-platform.md + docs/architecture/FND-02_PROTOCOL_OTERYN_V1_CONTRACT.md + docs/architecture/ARCHITECTURE_REVIEW_REFINEMENTS_2026-08-07.md + workspace-boundaries.toml + tools/synthetic-client-harness
  conflict_or_gap: ADR-0007 requires a headless client that traverses production admission/transport/protocol boundaries, while the current synthetic harness is classified synthetic and does not constitute that Tier 1 path. Even a future Tier-1 client sharing production schemas/codecs must be complemented by FND-02 independent byte/adversarial/property/fuzz/cross-version/limit/failure evidence rather than using the shared codec as its only wire oracle.
  required_before: Native gameplay readiness is reported as system-E2E and protocol-wire proven.
  worker_action: REPORT_ONLY
```

```yaml
cross_domain_finding:
  id: ALPHA-CLIENT-01-XD-06
  observed_in_domain: native-client
  target_owner: diagnostics-privacy-platform
  severity: P2
  evidence: docs/architecture/CLIENT_CRASH_DIAGNOSTICS_PRIVACY_OWNER_BASELINE.md
  conflict_or_gap: Client privacy/default/opt-out/redaction rules are accepted, but crash-package transport backend, retention/deletion policy and production ingestion contract are not finalized by this client architecture task.
  required_before: Automatic client crash upload is production-enabled.
  worker_action: REPORT_ONLY
```

## 26. Coordinator audit questions

The Architecture Coordinator should explicitly verify that this proposal:

1. does not promote the synthetic client model into production by documentation alone;
2. preserves current pre-native fail-closed behavior and ADR-0016 runtime unavailability, including no one-time gameplay-ticket consumption for an unavailable path;
3. preserves ADR-0003 Platform Identity -> Game Login Ticket -> Game Gateway ticket redemption/route selection -> selected route/pre-admission -> protocol transport -> final game-owned FND-04 admission chain without moving canonical GameSession/CharacterLease authority to Platform/Gateway;
4. states current Platform directory rejection exactly as the recursive 12-literal-key denylist and does not present complete-schema unknown-field rejection as implemented;
5. does not redefine FND-02/FND-04 server/protocol authority and preserves generation/sequence/revision/snapshot reconciliation;
6. is compatible with DUR-04 client-safe projection and future bundle/toolchain work, including audio assets;
7. preserves ADR-0007's distinction among synthetic, Tier 1 headless, Tier 2 native and Tier 3 release evidence;
8. requires independent FND-02 wire evidence so shared production schemas/codecs are not the only correctness oracle, without creating a second production protocol stack;
9. keeps audio provider/device lifetime application-owned, resource-bounded, client-safe/revision-compatible, degradable and strictly non-authoritative without prematurely selecting a library/vendor;
10. places external-alpha update/signing/security requirements in the correct owner domain rather than pretending they are solved here;
11. keeps technology choices reversible while freezing the authority/dependency boundaries needed before implementation.

## 27. Proposed disposition

Recommend coordinator acceptance of the boundary model as the implementation-guiding native-client baseline, while keeping all runtime implementation status `NOT_STARTED` and all cross-domain findings report-only until their owning programmes deliver accepted evidence.