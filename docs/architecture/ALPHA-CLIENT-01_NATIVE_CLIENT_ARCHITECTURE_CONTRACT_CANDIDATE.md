# ALPHA-CLIENT-01 — Native Client Architecture Contract Candidate

- DecisionStatus: `CANDIDATE`
- DeliveryStatus: `IN_REVIEW`
- ImplementationStatus: `NOT_STARTED`
- Date: 2026-08-15
- Issue: `#263`
- Worker: `DOMAIN ARCHITECTURE DESIGN AGENT / worker E`
- Depends on:
  - `docs/architecture/ADR-0003-platform-identity-game-gateway-and-admission-boundary.md`
  - `docs/architecture/ADR-0011-native-client-pre-protocol-migration-state.md`
  - `docs/architecture/ADR-0016-gameplay-transport-client-mode-runtime-readiness.md`
  - `docs/architecture/FND-02_PROTOCOL_OTERYN_V1_CONTRACT.md`
  - `docs/architecture/FND-04_IDENTITY_GAME_SESSION_ADMISSION_CHARACTER_LEASE_CONTRACT.md`
  - `docs/architecture/DUR-04_CONTENT_WORLD_AND_SCRIPTING_CONTRACT.md`
  - `docs/architecture/ADR-0007-native-end-to-end-test-platform.md`
  - `docs/architecture/ARCHITECTURE_REVIEW_REFINEMENTS_2026-08-07.md`
  - `docs/architecture/CLIENT_CRASH_DIAGNOSTICS_PRIVACY_OWNER_BASELINE.md`
- Companion analysis: `docs/architecture/ALPHA-CLIENT-01_NATIVE_CLIENT_ARCHITECTURE_ANALYSIS.md`
- Authority: candidate for Architecture Coordinator audit; not canonical until coordinator acceptance/merge
- Runtime authorization: **NONE**

## 1. Purpose

This candidate defines the minimum normative client-side boundaries required to evolve the accepted Windows-first `pre-native-protocol` shell into the native gameplay client without creating a second gameplay authority or fabricating runtime readiness.

Responsibility labels in this document are architectural roles. They are **not** frozen Rust trait, type, crate, module or library names unless an already accepted parent contract says otherwise.

## 2. Parent authority

This candidate does not supersede or reinterpret its parent contracts.

`ADR-0003-platform-identity-game-gateway-and-admission-boundary.md` is an explicit consumed parent authority for the client game-entry control-plane chain. It owns the separation of Platform Identity, one-time Game Login Ticket issuance/redemption, Platform-owned Game Gateway route selection and short-lived pre-admission material from final game-domain admission. `FND-04` remains authoritative for final gameplay admission, `GameSessionId` creation/replacement and `CharacterLease`; Platform/Gateway pre-admission material MUST NOT be promoted into final gameplay authority.

Where a conflict exists:

1. accepted protocol/admission/content/security/E2E parent contracts win over this candidate;
2. current machine-readable runtime availability wins over architectural target descriptions;
3. the client MUST fail closed rather than infer missing authority;
4. coordinator acceptance of this document MUST NOT be reported as runtime implementation or production activation.

## 3. Current runtime truth

Until exact implementation and E2E evidence supersede the verified pre-native state:

```text
native Windows shell                  = AVAILABLE
renderer/input/runtime bootstrap      = AVAILABLE
bounded Identity/Platform directory   = AVAILABLE_AS_MIGRATED_BASELINE
native gameplay entry                 = UNAVAILABLE
gameplay transport adapter            = UNAVAILABLE
gameplay listener                     = UNAVAILABLE
TCP/QUIC player modes                 = UNAVAILABLE
```

The production client MUST therefore continue to fail gameplay entry before requesting or consuming gameplay authority material, including a one-time Game Login Ticket for a connection that cannot complete, before gameplay route/credential consumption and before gameplay network connection while those capabilities remain unavailable.

## 4. Production composition contract

### 4.1 Composition root

`apps/client` MUST remain the sole production client composition root unless a later accepted decision introduces a real process/deployment/security boundary.

The composition root owns:

- application lifecycle and ordered shutdown;
- async runtime lifetime;
- renderer/window/input adapter lifetime;
- visual scene/presentation-provider lifetime and its binding to the current non-authoritative projection;
- audio provider and audio-device/session lifetime;
- Identity/Platform provider lifetime;
- client configuration/privacy state;
- client-safe content provider lifetime;
- diagnostics/crash provider lifetime;
- future Game Gateway/pre-admission control-plane and gameplay transport provider lifetime when separately implemented and authorized;
- release/update status presented to the application.

Screens/widgets MUST NOT construct infrastructure clients, async runtimes, scene/render infrastructure, audio devices, gameplay sessions or authority-bearing transports.

### 4.2 Dependency direction

Application and presentation logic MUST depend on semantic ports/state rather than concrete OS, socket, TLS, filesystem, installer, audio-device, scene-engine or renderer-backend APIs.

Infrastructure adapters MAY depend inward on those semantic boundaries. The inward layer MUST NOT depend outward on a concrete installer/updater/network/UI/audio/scene technology solely for convenience.

## 5. Application and screen contract

The client MUST separate:

- process/application lifecycle state;
- Identity/directory/navigation state;
- local gameplay-session **observation** state;
- non-authoritative world projection;
- purely local UI/presentation state;
- visual scene/presentation state derived from the projection and local presentation inputs;
- renderer resource/cache state;
- audio presentation state/resource state.

A screen MUST:

- consume read-only/immutable view state or an equivalent one-way presentation projection;
- emit semantic user intent;
- present bounded operational/failure states without exposing secrets or hidden topology.

A screen MUST NOT:

- create or restore `GameSessionId` authority;
- grant/acquire a `CharacterLease` by local decision;
- treat a selected character/world/channel as accepted gameplay admission;
- mutate authoritative gameplay state;
- apply raw network payloads directly to renderer/UI/scene state.

## 6. Runtime capability contract

Player-facing functionality MUST be gated by **runtime capability evidence**, not by the existence of an architecture target.

The client capability view MUST be able to distinguish conceptually:

```text
architecture target
implementation present
locally usable in this build/environment
currently authorized/eligible under accepted policy
```

Exact representation is deferred.

A transport mode MUST NOT be shown as selectable/usable if its runtime implementation is unavailable. In particular, current `AUTO_TCP_FIRST`, `PREFER_QUIC`, `TCP_ONLY` and `QUIC_ONLY` architecture targets MUST NOT be exposed as working player options while `runtime_available_now=false`.

The UI MUST NOT implement security downgrade/fallback semantics. Any future cross-transport retry/fallback MUST be driven by accepted admission/transport policy and fresh authority where required.

## 7. Identity, directory, Gateway and gameplay boundary

The client MUST preserve a hard semantic separation between:

1. Platform Identity and reusable credential handling;
2. Platform world/channel/character directory data and bounded player selection intent;
3. one-time Game Login Ticket issuance for the selected gameplay attempt;
4. Platform-owned Game Gateway ticket redemption, bounded supported protocol offer where applicable, World Registry route selection and returned selected endpoint/channel/revisions plus short-lived pre-admission session/admission material;
5. FND-02 `protocol-oteryn` transport/bootstrap to the Gateway-selected endpoint using only the selected route/revision context;
6. final game-owned FND-04 admission, `CharacterLease` acquisition/validation and canonical `GameSessionId` authority.

Platform directory responses MUST NOT be reinterpreted as gameplay credentials, Gateway authorization or final game-domain authority. A client MUST NOT bypass Game Gateway ticket redemption/route selection by turning directory selection directly into FND-04 admission or a direct gameplay endpoint choice.

### 7.1 Current Platform-directory parser precision

The current implementation in `crates/platform-client/src/lib.rs` does **not** provide complete-schema or generic unknown-gameplay-field rejection. It recursively rejects only these 12 literal JSON key names:

```text
host
port
endpoint
endpoint_uri
protocol
protocol_profile
ticket
credential
game_session
admission
route
address
```

Unknown compound keys such as `game_session_id`, `admission_token` or `selected_endpoint` are not proven rejected by that denylist and may be ignored by the current manual parser when the otherwise required directory shape is valid. Documentation and conformance evidence MUST describe the current behavior at exactly this scope.

Before a future implementation may claim a complete directory-schema boundary against unknown gameplay-shaped fields, it MUST provide explicit schema/allowlist or equivalent reject-unknown-field evidence that closes that gap. This is a future implementation requirement; this paper contract does not modify `platform-client` runtime code.

The conceptual fresh-entry flow is:

```text
Platform Identity
  -> bounded Platform directory + player selection intent
  -> one-time Game Login Ticket
  -> Platform-owned Game Gateway
       -> redeem ticket through Platform authority
       -> consume bounded supported protocol offer where applicable
       -> apply World Registry route/channel policy
       -> select endpoint/channel/revisions
       -> return short-lived pre-admission session/admission material
  -> establish FND-02 protocol-oteryn transport/bootstrap to the selected endpoint
  -> final game-owned FND-04 admission
       -> validate/consume pre-admission material and route/revision bindings
       -> acquire/validate CharacterLease
       -> create/replace canonical GameSession authority only on successful final admission
  -> synchronized non-authoritative client projection
```

Opening the selected transport or presenting valid Gateway material is not final gameplay admission. Platform/Gateway remain the pre-admission/control-plane boundary; final `GameSessionId`/`CharacterLease` gameplay authority remains in the Oteryn-v2 game domain under FND-04.

This contract does not freeze the future API/token/route representation at those seams.

## 8. Local gameplay-session observation contract

Client session state is an observation/progress model only.

The client MAY record that it is locally:

- requesting admission;
- connecting;
- synchronizing;
- active under an observed binding;
- reconnecting/resynchronizing;
- terminally rejected/disconnected.

Those states MUST NOT independently create, extend, resume or replace server-owned gameplay authority.

An open socket MUST NOT mean “gameplay active”. A selected character MUST NOT mean “lease acquired”. A cached `GameSessionId` MUST NOT mean “session resumable”. The owning server/admission contract remains authoritative.

## 9. Gameplay protocol egress contract

Future protocol egress MAY accept only semantic client/player intent from the application boundary.

Before encoding/sending an authority-relevant command, the client MUST have positive local evidence of the binding/phase required by FND-02/FND-04 and MUST satisfy parent protocol resource/shape limits.

Client-side legality/UX checks are advisory/bounding checks. They MUST NOT replace server gameplay legality.

`CommandId`, command ordering, retry, duplicate and terminal-result semantics MUST follow accepted FND-02/FND-03/FND-04 contracts and MUST NOT be redefined by UI/input code.

Raw OS key codes, widget callbacks and renderer events MUST NOT be serialized directly as protocol commands.

## 10. Gameplay protocol ingress and reconciliation contract

Before a server observation can alter the client world projection, the future ingress path MUST enforce all applicable accepted boundaries, including:

1. bounded frame/schema/type/phase validation;
2. active `connection_generation` fencing;
3. `server_sequence` rules where applicable;
4. typed state-domain revision preconditions;
5. snapshot begin/chunk/commit barriers where applicable;
6. accepted protocol resource ceilings.

Stale-generation server traffic MUST NOT mutate the active projection.

A state delta MUST be applied only when the client's current domain revision equals its required `base_revision`. A mismatch MUST cause the accepted reconciliation/resync behavior. The client MUST NOT fabricate missing authoritative deltas or skip unknown authoritative state.

An incomplete/invalid replacement snapshot MUST NOT partially replace the active authoritative-observation baseline.

## 11. Non-authoritative client projection contract

The interactive client MUST maintain a representation that is explicitly non-authoritative from the server/gameplay perspective.

The representation MUST distinguish:

- state observed from accepted server protocol messages;
- purely local presentation/UI state;
- any future speculative presentation state.

Local presentation state MAY include camera/layout/selection/interpolation/pending-indicator data. It MUST NOT advance or synthesize authoritative `server_sequence`, state-domain revision, command result, GameSession/lease or durable gameplay state.

This candidate does not authorize a prediction/rollback algorithm and does not promote the current synthetic `client-domain` / `client-simulation` crates into production.

## 12. Input contract

Physical/OS input MUST pass through a semantic action boundary before application/gameplay handling.

The application MAY route a semantic action to:

- UI/navigation;
- text/IME editing;
- settings/accessibility;
- future gameplay intent when gameplay is actually available.

Focus/modal/text-entry routing MUST prevent the same physical event from accidentally becoming both UI text/navigation and an unintended gameplay command unless explicitly designed and tested.

Exact keybind/controller/accessibility policy is deferred.

## 13. Renderer, scene and UI contract

Renderer/UI code MUST consume client view/projection state and MUST NOT own gameplay admission, transport or server-authoritative mutation.

Renderer-local caches and GPU resources MUST be reconstructable without creating gameplay rights or changing authoritative observation state.

The Windows interactive shell MUST keep OS/window/surface lifecycle handling at the platform adapter boundary, including implementation-appropriate handling of resize/DPI/focus/suspend/resume/device/surface loss/shutdown.

The exact UI toolkit, scene framework and renderer implementation remain unfrozen by this candidate.

### 13.1 Visual scene and presentation contract

The visual scene is a **presentation projection**, not a second gameplay/world model. Camera state, animation playback state, lighting state, particles, transient effects, interpolation state and renderer-facing scene instances MAY be derived from the accepted non-authoritative client projection, compatible client-safe content and purely local presentation inputs. They MUST NOT create, infer or advance authoritative gameplay state, `StateRevision`, `server_sequence`, RNG outcomes, combat results, movement legality, session/admission state or durable world state.

The `apps/client` composition/application boundary MUST own construction and lifetime of the scene/presentation provider or equivalent coordinator. Screens/widgets MAY request presentation changes through semantic presentation intent, but they MUST NOT create independent network/world replicas or directly mutate server-observed projection state. A scene implementation MAY cache presentation-friendly transforms or GPU-ready instances only when their provenance is reconstructable from the current projection/content/presentation state and they are invalidated or rebuilt on authoritative-observation/revision replacement as required.

Camera, animation, lighting, particle and effect systems MUST consume typed or otherwise semantically bounded presentation inputs. Gameplay-relevant meaning MUST come from the accepted projection/content contracts rather than from animation completion, particle timing, camera position, frame timing or renderer success. Missing/delayed visual assets, shader/device loss, animation interruption or effect-budget exhaustion MUST degrade or reconstruct presentation without changing gameplay authority or synthesizing a gameplay outcome.

Visual assets and presentation semantics MUST come from the client-safe release/content projection and remain compatible with the active build/content/release revision. Staged updates MUST NOT mutate active scene semantics in place for a running compatibility context. Scene work MUST be bounded before production acceptance, including applicable visible/active entities, animation tracks, particle/effect instances, lights, transient queues, GPU/CPU staging data and recovery/rebuild work. Exact numeric ceilings remain measurement-driven.

The minimum decision is the authority/ownership boundary above, **not** a choice of scene graph, animation system, lighting model, particle library, renderer API or effect framework. Those technologies remain reversible until implementation evidence exists.

Scene/presentation conformance evidence belongs to the native-client implementation/test owner and MUST prove at minimum: projection-to-scene derivation without a second authoritative world model; revision/snapshot replacement invalidation; camera/animation/effect non-authority; client-safe asset/revision compatibility; bounded-resource behavior; device/resource loss and recovery; representative Tier 2 visual journeys; and Tier 3 packaged smoke where the feature is shipped.

Decision timing: this ownership/non-authority boundary MUST be decided now because allowing scene/camera/animation/effects to grow as an unowned parallel client-state model would make later reconciliation and testing unsafe. Concrete scene/render technologies and visual algorithms are deliberately deferred. Superseding evidence must demonstrate an alternative that preserves the same one-way authority, reconstructability, compatibility and bounded-resource properties.

### 13.2 Audio presentation contract

Audio is a client-side **presentation-only** subsystem. It MUST NOT own or decide gameplay legality, authoritative simulation, RNG, combat outcomes, timers, session/admission state or server reconciliation. Audio cues MAY be derived from already-accepted client observations and local presentation/UI events, but loss or delay of audio MUST NOT alter the authoritative or client-observed gameplay state machine.

The `apps/client` production composition root MUST own audio-provider construction, selected output-device/session lifetime, ordered shutdown and recovery. Screens, widgets and gameplay reducers MUST NOT open devices or own long-lived audio infrastructure directly.

Audio assets MUST come only from the same client-safe content/release projection boundary used by the shipping client. Audio lookup/activation MUST be compatible with the active build/content/release revision; an incompatible or missing required audio asset MUST fail/degrade as presentation rather than reinterpret content semantics or authorize gameplay. Staged release/content updates MUST NOT mutate the active audio asset set in place for a running release/session compatibility context.

The implementation MUST bound audio resource use, including simultaneously active voices/streams, decoded/streaming buffers, queued events and retry/reopen work. Exact numeric ceilings are implementation/evidence-driven and are deliberately not fixed here. Device loss, initialization failure, unsupported format or unavailable output MUST degrade to a clear recoverable/muted presentation state; gameplay entry and server authority MUST NOT depend on audio availability.

Settings/accessibility MUST expose the semantic controls needed by product requirements, including at least master/effect/music class controls or an equivalent extensible category model, mute, selected output where supported, and accessibility-safe operation when audio is disabled. Exact UX and category taxonomy remain product/implementation decisions.

Audio conformance evidence belongs to the native-client implementation/test owner. It MUST prove provider/device lifecycle and recovery, bounded-resource behavior, client-safe asset/revision compatibility, deterministic mapping from eligible presentation events to requested cues where determinism is claimed, settings persistence/accessibility behavior, and non-interference with gameplay authority. This candidate intentionally does **not** select an audio library, codec stack, mixer, OS backend or vendor.

## 14. Client-safe content contract

The client MUST consume only the DUR-04-approved client-safe projection of content/world semantics.

Client release/content activation MUST:

- verify artifact integrity through the accepted release/content mechanism;
- verify required build/protocol/content/capability compatibility metadata;
- fail closed on incompatible required semantics;
- prevent server-only data from entering the client merely to be UI-hidden;
- keep activated gameplay-relevant content immutable for the active compatibility/session context.

A staged update MUST NOT mutate the active gameplay content projection in place.

Gameplay-relevant hot reload is not authorized by this candidate.

Physical bundle encoding, patch/delta format, CDN and signing implementation remain with their owning programmes.

### 14.1 Oteryn Studio low-level sharing boundary

Oteryn Studio and the native game client MAY share **low-level, representation-neutral, non-authoritative components** when doing so removes duplicate interpretation without turning either product composition root into the other's API surface.

A shared low-level component MAY contain only responsibilities that remain valid without a live player session or Studio authoring UI, for example:

- stable content identifiers, revisions and client-safe schema/value types;
- deterministic content decoding/validation and read-only package/index primitives;
- math/geometry/color/image/asset primitives with no gameplay authority;
- renderer-backend-neutral resource/format descriptors and bounded utility algorithms;
- platform-neutral input normalization primitives that do not choose gameplay actions or product keybind policy;
- generic bounded serialization, validation and compatibility utilities.

The following MUST remain product-specific and MUST NOT become a shared Studio contract merely for reuse convenience:

- `apps/client` or Studio composition/application roots;
- Platform Identity, Game Login Ticket, Game Gateway, gameplay transport, admission, `GameSessionId`, `CharacterLease` or reconnect/session authority;
- the live client's protocol reconciliation reducer and non-authoritative world projection lifecycle;
- player navigation/screens, gameplay intent mapping, product accessibility/UX policy and player-specific settings resolution;
- live scene/camera/animation/effects lifetime tied to an active gameplay projection;
- diagnostic/privacy upload policy, updater/install authority or production release activation;
- Studio authoring commands, draft/editor state, undo/redo/history, validation UX or authoring-only metadata.

Dependency direction MUST remain acyclic: the game client and Studio MAY both depend on explicitly shared low-level crates/packages, but shared low-level components MUST NOT depend on `apps/client`, a Studio application root, live-session state or product UI. The client MUST NOT depend on the Studio application to interpret a shipped client-safe bundle, and Studio MUST NOT import client-only composition/session types as its durable authoring model.

Where Studio authors content that is later consumed by the client, authoring-only state MUST be projected/exported through an accepted revisioned content schema; the shipping client consumes only the client-safe projection. Shared parsing/validation code MAY be used on both sides, but the exported bytes/schema and compatibility rules remain the independent product boundary and MUST be testable without either UI application.

If Studio is later hosted in another repository/process/package boundary, cross-repository publication/versioning is a separate owner/coordinator decision; this candidate grants no cross-repository write or release authority.

Conformance evidence MUST include dependency-graph/layer checks preventing client↔Studio application coupling, compile/test coverage of shared low-level components without either product composition root, client-safe content compatibility fixtures across Studio export/client ingest where applicable, and negative tests proving authoring-only/server-only fields cannot enter the runtime client-safe projection.

Decision timing: the **sharing boundary** MUST be decided now because accidental sharing of client session/UI types would turn implementation convenience into a long-lived Studio API, while duplicated content interpretation would create drift. Exact shared crate/package names and concrete renderer/input libraries remain implementation decisions.

## 15. Configuration and filesystem contract

The client filesystem model MUST distinguish at least:

```text
installed immutable release payload
account-scoped portable preference state when an accepted account-profile owner exists
OS-user-scoped durable privacy/local-user preference state
installation-scoped updater/install state
device-scoped hardware/presentation state
rebuildable cache/staging state
bounded diagnostic/crash spool
```

Every durable setting MUST declare a semantic scope in its schema. A setting MUST NOT silently migrate between `ACCOUNT`, `OS_USER`, `INSTALLATION` and `DEVICE` scopes merely because its physical storage path changes.

Minimum scope model:

- `ACCOUNT` — portable, non-secret preferences intentionally designed to follow the authenticated account across devices, such as semantic action bindings or accessibility preferences when product policy marks them portable. Account persistence/synchronization requires an accepted Platform/profile owner and contract; until that exists, the client MUST treat the account layer as absent rather than inventing local account authority.
- `OS_USER` — preferences/privacy choices that belong to the local operating-system user across accounts on that OS profile. The durable diagnostics automatic-upload opt-out belongs here by default so switching game accounts cannot silently re-enable uploads. Local language/accessibility choices MAY also live here when not declared portable-account settings.
- `INSTALLATION` — install/update/release-channel and installation-wide state that describes the installed product instance, not a player identity. Installation state MUST NOT override a more restrictive privacy choice or impersonate account/device preferences.
- `DEVICE` — hardware/presentation choices tied to the current machine/device identity or capability, including selected audio output, monitor/window placement, renderer/GPU preference and device-specific input identifiers. A missing device-scoped value falls back without being uploaded as account identity.

For user-preference keys that are explicitly allowed at more than one scope, deterministic resolution MUST be schema-declared. The default precedence is:

```text
ephemeral session override
  > DEVICE
  > OS_USER
  > ACCOUNT
  > product default
```

`INSTALLATION` is not a generic higher-precedence user-settings overlay; it applies only to keys explicitly declared installation-scoped and to compatibility/update constraints owned by the release mechanism.

Privacy/security fail-closed rules override ordinary preference precedence. In particular, when multiple applicable scopes can express permission versus opt-out, the **most restrictive valid privacy choice wins**; an account or device preference MUST NOT re-enable diagnostics disabled at OS-user/installation policy scope. Mandatory release/protocol/content compatibility constraints also win over convenience settings and MUST NOT be bypassed by a local override.

A semantic key that changes scope or precedence requires an explicit versioned migration. Migration MUST define source scope, destination scope, conflict resolution and rollback/recovery; it MUST NOT silently copy device identifiers, secrets or privacy consent into an account-portable profile.

OS-specific paths MUST be resolved by the platform/filesystem boundary, not hard-coded throughout application/UI logic.

Durable client configuration MUST be versioned and migration-aware. Writes/migrations MUST avoid leaving a partially updated configuration as the only recoverable copy.

A normal update/restart/settings migration MUST NOT silently re-enable diagnostics after the user has disabled them unless a separately accepted privacy decision explicitly requires a new choice.

Reusable credentials, gameplay admission material and reconnect secrets MUST NOT be written to general configuration, logs, content cache or crash spool.

Exact directories, registry behavior, install scope, physical account-profile synchronization mechanism and credential-vault technology are deferred.

## 16. Logging and crash diagnostics contract

Client logging/crash collection MUST follow an allowlist/data-minimization model.

Automatic eligible crash diagnostics MUST follow the accepted owner baseline:

- enabled by default;
- clear durable global opt-out;
- opt-out stops new automatic uploads and queued retries while disabled;
- opt-out cannot block client startup/authentication/gameplay;
- opt-out is not adverse security/abuse evidence;
- local redaction/allowlisting occurs before transmission;
- report/attachments are bounded;
- reusable credentials/tokens, private chat, arbitrary user files and unrelated personal data are excluded.

Client diagnostic evidence MUST remain optional corroborating evidence. It MUST NOT weaken, suppress or replace required server-side authoritative/security evidence.

Exact retention/deletion, upload transport/backend and crash package schema are deferred.

## 17. Release/update/install contract

### 17.1 Running-release immutability

An active authority-bearing gameplay process MUST NOT replace its own executable or gameplay-relevant release/content semantics in place.

Release activation MUST occur at a safe lifecycle boundary outside the active gameplay session/process mutation path.

### 17.2 Verification

Before external alpha, a candidate release MUST be verified according to the separately accepted signed-artifact/updater and provenance/SBOM policy before activation.

This candidate does not define the cryptographic/signing/vendor implementation.

### 17.3 Atomicity

Release activation MUST prevent a mixed partial version from being treated as valid. The activation mechanism MUST yield either:

- a complete verified candidate release; or
- retention/recovery of a complete verified prior release.

Rollback MUST NOT bypass current Platform/protocol/content compatibility policy merely because an older payload exists locally.

### 17.4 User state separation

Update/install/uninstall mechanics MUST keep release payload identity separate from per-user settings/cache/diagnostics. Destruction/migration of user data requires an explicit product/retention action rather than being an accidental side effect of binary replacement.

Installer/updater technology remains deferred.

## 18. Windows-first platform contract

The first interactive product target remains Windows-first `x86_64-pc-windows-msvc`.

Windows-specific concerns MUST be contained at shell/platform adapters where practical, including:

- event loop/window lifecycle;
- DPI/monitor/focus/text/IME behavior;
- filesystem known-folder/permission resolution;
- process handoff for update/install activation;
- crash integration;
- later selected package/signature UX.

This boundary MUST NOT be represented as a commitment to Linux/macOS parity. It exists to keep application/session logic independent of unnecessary Win32 details and testable without a window where appropriate.

## 19. Headless and interactive test contract

The accepted ADR-0007 tiers remain distinct.

### 19.1 Tier 1 headless system E2E

The future headless system client MUST:

- traverse the supported Platform Identity -> Game Login Ticket -> Game Gateway -> selected route/pre-admission -> gameplay transport/protocol -> final FND-04 game-domain admission/server boundaries;
- speak the production gameplay transport and `protocol-oteryn` path supported by the product;
- use the same accepted **production protocol schemas, production codecs, sequencing and admission contracts** as the native client;
- emit normal player intent/commands;
- expose deterministic semantic observations;
- never call authoritative game-domain mutation APIs directly.

The production schemas/codecs are a mandatory part of Tier 1 product-path coverage; a test-only replacement codec MUST NOT substitute for exercising the shipping codec path. In addition, shared production code MUST NOT be the only oracle for wire correctness. Tier-1/future protocol evidence MUST therefore also consume the independent-proof property required by FND-02 and the owner-accepted 2026-08-07 refinement, including as applicable:

- canonical byte-level golden fixtures for framing/messages;
- malformed/adversarial corpus coverage;
- property/invariant encode/decode round-trip tests;
- fuzzing of externally controlled framing/decoders/parsers;
- cross-version compatibility fixtures for permitted same-major evolution;
- explicit resource ceilings and stable failure classes/dispositions.

The independent oracle supplements production-codec E2E coverage; it does **not** authorize replacing the production codecs in Tier 1, and it MUST NOT be interpreted as requiring or authorizing a second production protocol implementation or duplicated production stack.

The current `tools/synthetic-client-harness` MUST NOT be reported as Tier 1 system E2E under its current synthetic/no-production-network classification.

### 19.2 Tier 2 native-client E2E

A test-only native-client adapter MAY invoke normal input/client-command paths and read semantic observations/UI/frame/log evidence.

It MUST NOT:

- forge Platform/Game Sessions;
- teleport or mutate server-owned state;
- inject authoritative snapshots/results;
- bypass normal networking/protocol/server legality;
- ship in production-default builds.

### 19.3 Tier 3 release binary

Release-candidate proof MUST run the actual packaged product artifacts without the Tier 2 in-process adapter and prove the named product journey required by ADR-0007 once gameplay exists.

A window-only smoke test MUST NOT be reported as gameplay E2E.

## 20. Player-facing failure-state contract

Without exposing credentials or hidden topology, the client MUST be able to present bounded distinct states for applicable future failures including:

- gameplay runtime unavailable;
- version/content incompatibility;
- maintenance/queue/readiness state when supplied by owning authority;
- session/lease conflict;
- transport connection/security failure;
- reconnect/resynchronization progress;
- terminal gameplay-session failure;
- update required/activation failure;
- audio unavailable/degraded where audio is implemented;
- visual presentation degraded/recovering where scene/renderer recovery is in progress.

A lower-level socket, renderer or presentation error MUST NOT automatically become a security downgrade or a claim that the server ended/accepted gameplay authority. Error mapping must respect the owning contract.

## 21. Shutdown contract

Shutdown MUST be coordinated by the production composition root.

It MUST:

1. stop accepting new user/gameplay intents;
2. cancel/close application-owned asynchronous work through bounded ownership;
3. close/release future transport/session client resources without inventing server success;
4. stop/release scene/presentation transient work without mutating gameplay/session state;
5. stop/release audio playback/device resources without blocking gameplay/session authority cleanup;
6. flush only bounded, policy-permitted local state/diagnostics;
7. release renderer/window resources;
8. terminate without orphaned client worker ownership.

Exact timeout values are implementation/SRE decisions unless already fixed by a parent contract.

## 22. Readiness contract

The following statuses MUST remain distinct:

```text
architecture accepted
implementation present
implementation proven at exact revision
production/external-alpha enabled
```

### 22.1 Current pre-native readiness

The current shell MAY claim only the capabilities proven by the migrated baseline. It MUST NOT claim native gameplay readiness.

### 22.2 Native gameplay enablement

A product build MUST NOT change gameplay availability to enabled until exact-revision evidence proves, together:

- ADR-0003-compatible Platform Identity/Game Login Ticket/Game Gateway route/pre-admission client integration;
- gameplay transport client implementation to the Gateway-selected endpoint;
- FND-04 final game-owned admission/reconnect and `CharacterLease` integration without moving canonical GameSession authority to Platform/Gateway;
- FND-02 codec/registry/reconciliation implementation;
- authoritative server counterpart;
- compatible client-safe content projection;
- native client projection/input/scene/render integration;
- required Tier 1 and Tier 2 E2E journeys with production codec-path coverage plus FND-02 independent wire evidence.

Scene/presentation and audio implementations are not gameplay-authority readiness. When those features are in the shipped alpha scope, their provider/content/degradation/bounded-resource evidence MUST be satisfied without making gameplay availability depend on presentation success.

### 22.3 External alpha

External-alpha production enablement additionally requires accepted release/security/SRE/privacy evidence, including signed artifact/updater strategy and implementation, build provenance/SBOM strategy, threat model, rollback/operability, required runbooks/SLOs and Tier 3 release-binary evidence.

Coordinator acceptance of this document alone satisfies none of those implementation/proof gates.

## 23. Required future evidence

Implementation claiming conformance to this candidate should prove at minimum:

1. application/screen code cannot bypass provider/capability gates to create gameplay network activity while runtime is unavailable;
2. current Platform directory parsing recursively rejects the 12 literal denylisted keys `host`, `port`, `endpoint`, `endpoint_uri`, `protocol`, `protocol_profile`, `ticket`, `credential`, `game_session`, `admission`, `route`, `address`; any stronger complete-schema/reject-unknown-fields claim is made only after explicit evidence for that stronger implementation, and directory handling cannot bypass the ADR-0003 Game Login Ticket -> Game Gateway redemption/route-selection boundary;
3. fresh gameplay entry follows the authorized chain through selected endpoint/channel/revisions and short-lived pre-admission material before final FND-04 admission, while final `GameSessionId`/`CharacterLease` authority remains game-owned;
4. stale `connection_generation` traffic cannot mutate the active projection;
5. state revision mismatch enters resync and never applies a speculative authoritative delta;
6. incomplete snapshot cannot become active world state;
7. Tier 1 traverses the actual supported production transport and the same production schemas/codecs/sequencing/admission contracts as the native client;
8. canonical byte-level protocol/framing golden fixtures provide an independent wire oracle in addition to production-codec E2E coverage;
9. malformed/adversarial corpus and property/invariant tests cover bounded decode/encode semantics;
10. externally controlled decoders/parsers are fuzzed under explicit resource ceilings and stable failure classes;
11. cross-version fixtures prove permitted same-major compatibility without duplicating the production protocol stack;
12. semantic input routing prevents UI/text events from unintended gameplay egress;
13. projection-to-scene derivation, camera/animation/lighting/particles/effects and renderer/device/resource recreation remain non-authoritative, revision-safe, reconstructable and bounded;
14. incompatible client-safe content fails closed before world activation and scene/audio assets cannot escape active release/revision compatibility;
15. shared low-level client/Studio components remain representation-neutral and non-authoritative, dependency direction prevents application-root coupling, and Studio export/client ingest fixtures prove one revisioned client-safe content interpretation without leaking authoring-only/server-only fields;
16. settings schema assigns account/OS-user/installation/device scope, deterministic precedence and versioned scope migration; selected audio output remains device-scoped; diagnostics opt-out cannot be re-enabled by a less restrictive account/device setting;
17. audio provider/device lifecycle, bounded-resource behavior, compatible client-safe audio assets, non-authoritative cue mapping, degradation/recovery and settings/accessibility behavior are proven when audio is implemented;
18. config migration is recoverable and preserves diagnostic opt-out;
19. logs/crash packages reject/redact prohibited secret/private data before upload;
20. update activation cannot launch an accepted mixed partial release and rollback remains compatibility-checked;
21. Tier 1 headless, Tier 2 native and Tier 3 release evidence are classified distinctly with exact build/Platform/protocol/content revisions retained;
22. no hidden retry converts failed physical E2E attempts into pass evidence;
23. exact-head implementation review and CI prove the claimed build, not a parent commit.

## 24. Decision timing

### Must these boundaries be decided now?

**YES** for composition/authority/projection/scene-presentation/content/filesystem/settings-scope/Studio-sharing/update/test/audio-ownership boundaries. Client implementation would otherwise hard-code cross-domain ownership, create ambiguous account/device persistence or allow product-specific types to become an accidental Studio API before the missing gameplay runtime exists.

**NO** for the concrete GUI/scene/render/network/updater/installer/content-packaging/audio libraries, exact shared crate names or account-profile synchronization mechanism. Those choices remain safely reversible and require implementation evidence.

### Downstream work blocked without this contract

- safe native gameplay client implementation after ADR-0003/FND-02/FND-04 runtime integration exists;
- stable UI/navigation/provider composition;
- scene/camera/animation/lighting/particles/effects implementation without authority leakage or duplicate world state;
- client content loader/update packaging work;
- Oteryn Studio/client low-level reuse without circular product coupling or duplicated runtime content interpretation;
- settings persistence/account-device synchronization design with deterministic ownership and conflict resolution;
- client audio provider/content/settings implementation without authority leakage;
- Tier 1 production-codec system E2E and Tier 2 native-client automation design;
- honest external-alpha client readiness gate.

### What becomes expensive if delayed

- direct screen-to-network coupling;
- UI/scene state accidentally becoming gameplay state authority;
- a second unreviewed scene/world model diverging from protocol reconciliation;
- client-specific session/UI types becoming a de facto Studio API;
- duplicate Studio/client content parsers drifting in schema interpretation;
- account/device settings overwriting each other nondeterministically or privacy opt-out being re-enabled by a weaker scope;
- direct screen/reducer ownership of audio devices and unbounded playback queues;
- irreversible loose-content/file layouts;
- updater/install mutation mixed into the gameplay process;
- test hooks or test-only codecs that bypass product boundaries;
- unsupported transport settings becoming user-visible compatibility promises.

### Superseding evidence

A future change may supersede these boundaries only with explicit accepted evidence showing an alternative preserves or improves:

- server/gameplay authority separation;
- protocol revision/reconciliation safety;
- one-way, reconstructable scene/presentation derivation without parallel authority;
- client/Studio dependency direction and single revisioned client-safe content interpretation;
- deterministic settings scope/precedence and privacy fail-closed behavior;
- content allowlisting/compatibility;
- credential/privacy isolation;
- release atomicity/rollback;
- production-codec real-boundary E2E plus independent wire proof;
- presentation/audio non-authority and resource boundedness;
- implementation simplicity/operability on measured production constraints.

Framework preference alone is insufficient superseding evidence.

## 25. DECISIONS_NOT_TAKEN

This candidate intentionally does not select:

- exact UI toolkit or renderer replacement;
- scene graph/entity presentation framework, camera implementation, animation runtime, lighting model, particle/effects engine or shader architecture;
- exact shared client/Studio crate/package names, publication mechanism or cross-repository release model;
- exact account-profile synchronization transport/backend or physical settings file/registry layout;
- exact Rust trait/module/crate layout for proposed ports;
- promotion/replacement of synthetic client-domain/simulation crates;
- gameplay prediction/interpolation/rollback semantics;
- gameplay transport adapter implementation or TCP/QUIC activation;
- QUIC library/profile/fallback timing/default;
- Game Gateway/admission/reconnect credential/API representation;
- protocol/TLS/protobuf implementation libraries;
- client bundle/patch/CDN format;
- installer/updater framework or code-signing provider;
- Windows directory/registry/install-scope details;
- credential vault technology;
- crash backend/retention/legal text;
- audio library, codec/mixer stack, device backend/vendor or exact category taxonomy;
- release channel/version-skew/forced-update product policy;
- Linux/macOS support plan;
- exact numeric retry/cache/log/spool/scene/effect/audio-voice/buffer limits outside accepted parent contracts;
- any server/gameplay/persistence/balance authority.

## 26. CROSS_DOMAIN_FINDINGS

Normative cross-domain findings for this worker remain in the companion analysis section `CROSS_DOMAIN_FINDINGS`. They are `REPORT_ONLY` and do not grant this contract authority to solve those owner domains.

## 27. Acceptance boundary

If accepted by the Architecture Coordinator, this candidate should become the client implementation baseline for ALPHA-CLIENT-01 while all runtime implementation remains `NOT_STARTED` until separately authorized tasks provide exact code and proof.

`MERGE_AUTHORITY: ARCHITECTURE_COORDINATOR_ONLY`