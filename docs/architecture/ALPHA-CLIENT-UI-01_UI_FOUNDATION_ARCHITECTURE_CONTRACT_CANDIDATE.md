# ALPHA-CLIENT-UI-01 — UI Foundation Architecture Contract Candidate

- DecisionStatus: `CANDIDATE`
- DeliveryStatus: `IN_REVIEW`
- ImplementationStatus: `NOT_STARTED`
- Date: 2026-09-10
- Decision ID: `ALPHA-CLIENT-UI-01`
- Base: `main@149e1e5cc3dd09b4bbf53e9213b92901233da210`
- Extends:
  - `ALPHA-CLIENT-01` owner-accepted native-client architecture
  - `docs/architecture/ALPHA-CLIENT-01_NATIVE_CLIENT_ARCHITECTURE_CONTRACT_CANDIDATE.md` as historical source artifact for the accepted scope
  - `docs/architecture/OTERYN_V2_REMAINING_FIRST_WAVE_OWNER_ACCEPTANCE_BASELINE_20260816.md`
- Preserves:
  - server/gameplay authority boundaries
  - `apps/client` as the production composition root
  - semantic input before gameplay intent
  - non-authoritative client projection
  - client-safe content/release compatibility
  - capability-gated player-visible functionality
- Authority: candidate for architecture review; not canonical until accepted and merged
- Runtime authorization: **NONE**

## 1. Purpose

This candidate defines the minimum UI foundation boundaries required to evolve the accepted native Windows client architecture into a maintainable, high-performance gameplay UI without allowing widgets, layout state, renderer caches, local convenience models or OS input to become a second gameplay authority.

The decision deliberately separates **UI architecture** from **final product skin**. It freezes the ownership and data-flow seams that are expensive to repair later, while leaving toolkit/library choice, final HUD composition, visual style, exact numeric budgets and content-specific feature surfaces reversible until implementation evidence exists.

The intended implementation shape is:

```text
validated protocol / application observations
                |
                v
      non-authoritative client projection
             /          \
            /            \
           v              v
  render projection    UI view models
           |              |
           |          retained UI tree
           |              |
           |        layout / interaction
           |              |
           |          UiDrawList
           |              |
           +-------+------+
                   v
             renderer adapter
                   |
                   v
                  GPU
```

Input follows a separate one-way route:

```text
OS / winit events
      |
      v
input-platform normalization
      |
      v
input-actions / semantic contexts
      |
      +----> UI focus / navigation / text / drag-drop
      |
      `----> application gameplay intent -> accepted game-command boundary
```

## 2. Scope

### 2.1 In scope

This candidate defines:

- UI ownership and dependency direction;
- the UI-facing view-model boundary;
- retained-tree and incremental-layout responsibilities;
- renderer-neutral UI extraction through a `UiDrawList`-like contract;
- logical UI units, OS DPI and user-scale responsibilities;
- focus, capture, modal, text/IME and drag/drop routing;
- docking and layout persistence semantics;
- virtualized collection requirements;
- text shaping, localization and glyph-cache boundaries;
- UI asset and theme/design-token boundaries;
- accessibility and developer-diagnostics hooks;
- performance measurement responsibilities;
- test requirements;
- a bounded first gameplay HUD slice used to prove the foundation.

Responsibility labels such as `ui-core`, `UiViewModel`, `DockTree` and `UiDrawList` describe architectural roles. They are **not frozen Rust crate, trait, type or module names**.

### 2.2 Explicitly out of scope

This candidate does **not**:

- authorize native gameplay transport, admission, production traffic or deployment;
- change server-authoritative gameplay, protocol sequencing/revision rules or persistence semantics;
- promote any synthetic client projection into gameplay authority;
- choose a concrete third-party UI toolkit;
- require porting OTUI/OTML/Lua or the legacy C++ widget hierarchy;
- freeze a final HUD layout, pixel dimensions, art style, font family, color palette or animation style;
- freeze a specific atlas, texture-array or vector-icon strategy;
- freeze exact production cache sizes, widget-count ceilings, overscan counts or frame budgets without measured evidence;
- define product behavior for gameplay systems whose domain/view contracts do not yet exist.

## 3. Decision timing

### Must decide now?

**YES.**

### Concrete downstream work blocked without this decision

- native UI foundation implementation under the accepted ALPHA-CLIENT programme;
- safe wiring of `input-platform` / `input-actions` into interactive UI handling;
- renderer work that needs a stable distinction between world projection and UI extraction;
- first native HUD vertical slice;
- persistence design for movable/dockable panels;
- text/glyph and UI-asset integration;
- meaningful UI performance and interaction tests.

### What becomes harder later if left implicit

- widgets directly reading or mutating gameplay/network state;
- renderer/backend handles leaking throughout feature code;
- unversioned panel persistence that becomes expensive to migrate;
- conflicting input/focus rules distributed across features;
- full-tree relayout or full-list materialization becoming baked into feature APIs;
- screen-specific font/asset caches and inconsistent DPI handling;
- a second client-side world model emerging inside UI or scene code.

### Evidence that may justify superseding this decision

- measured implementation evidence showing a different retained/incremental representation materially improves correctness or performance while preserving the same ownership boundaries;
- accessibility or localization requirements that require stronger semantics;
- renderer evidence that requires a different backend-neutral extraction shape;
- product testing showing a different docking/persistence model is materially better;
- canonical PERF evidence replacing provisional UI benchmark targets;
- a newer accepted client architecture ADR/contract explicitly superseding this scope.

### Deliberately not decided now

- final UI framework/library;
- exact Rust package names;
- final art direction and HUD geometry;
- exact reference resolution or reference DPI;
- exact production numeric budgets;
- complete set of game-feature view models;
- mobile/touch-specific product layout;
- plugin/mod scripting surface for UI.

## 4. Architectural invariants

1. **UI is presentation, not gameplay authority.** UI may render observed state and emit semantic intent. It MUST NOT create, restore or advance authoritative gameplay state.
2. **One-way authoritative observation.** Validated application/projection state feeds UI view models. Widgets MUST NOT apply raw protocol payloads.
3. **Presentation state is separately owned.** Focus, hover, selection, scroll offsets, docking, tabs, temporary animation state and local panel preferences are UI state, not authoritative world state.
4. **Input is semantic before gameplay egress.** Raw OS events MUST NOT be serialized or translated directly by feature widgets into network messages.
5. **Renderer backends stay outside feature/UI semantics.** UI feature code MUST NOT own `wgpu` device/surface/pipeline/resource handles.
6. **UI extraction is reconstructable.** Losing renderer/GPU resources may invalidate presentation caches, but MUST NOT mutate game/session authority.
7. **Collections are bounded and virtualizable.** Feature APIs MUST NOT require one persistent widget per logical row/item for potentially large collections.
8. **Layout persistence is versioned and recoverable.** Invalid, stale or off-screen layouts MUST degrade to a safe default rather than blocking client startup/gameplay.
9. **DPI and user scale are first-class inputs.** Feature widgets MUST NOT encode OS pixel assumptions into semantic layout.
10. **Text remains semantic.** Localized strings are text data shaped at runtime; ordinary UI images MUST NOT embed language-specific text merely to avoid text layout.
11. **UI assets are client-safe content.** UI rendering consumes only compatible, verified client-safe release/content assets.
12. **Feature-neutral core.** Foundation primitives MUST NOT depend on concrete game features such as inventory, chat, market, battle list or skills.
13. **Capability truth wins over desired UX.** A control representing an unavailable runtime capability MUST remain unavailable/hidden according to accepted capability policy rather than simulating functionality.

## 5. UI state and view-model boundary

The client architecture MUST distinguish at least four state classes:

```text
server-observed / reconciled projection
        |
        +--> renderer-facing world projection
        |
        `--> UI-facing semantic view models

application/navigation state
        `--> screen availability, loading, failures, capability state

UI presentation state
        `--> focus, hover, tabs, dock tree, scroll, local selection, overlays

renderer resource state
        `--> glyph/texture/pipeline caches and transient draw resources
```

A UI-facing model SHOULD contain the minimum semantic data needed to render and interact with a feature. It SHOULD NOT mirror raw wire structures or expose renderer resources.

View models MUST be generation/revision aware where stale data could outlive a replaced session or projection. A replaced gameplay/session observation MUST make stale UI actions harmless through the accepted application/domain fencing rather than through widget identity alone.

Feature UI MAY own transient optimistic presentation such as a pending indicator, pressed state or drag preview. Such state MUST NOT synthesize authoritative success.

## 6. UI core model

The preferred foundation model is:

```text
retained hierarchy
+ explicit semantic state
+ dirty/incremental style and layout propagation
+ immediate render-command extraction
```

### 6.1 Retained hierarchy

The retained tree owns:

- parent/child hierarchy;
- stable local widget identity;
- visibility/enabled state;
- focusability and accessibility semantics;
- computed layout boxes;
- clipping/scroll containers;
- interaction state needed across events;
- feature-neutral style references.

The tree MUST NOT own game-domain authority or network clients.

### 6.2 Layout

Foundation layout MUST support the classes needed by the first gameplay UI:

- row/column flow equivalent to flex behavior;
- grid placement;
- minimum/maximum sizes;
- intrinsic/content sizing;
- percentage/relative sizing where required;
- margins, padding and gaps;
- clipping and nested scrolling;
- overlays/absolute placement for tooltips, menus and drag previews;
- anchors/constraints only where they materially simplify gameplay HUD composition.

Layout invalidation SHOULD be dirty/incremental. A small local change SHOULD NOT require unconditional full-tree layout every frame.

No concrete CSS/HTML/OTUI syntax is selected by this decision.

## 7. Docking and window/panel model

The target docking model is a versioned **dock tree**, not a hard-coded set of legacy vertical columns.

Conceptually:

```text
DockRoot
  |
  +--> Split(horizontal|vertical, ratio)
  |      +--> child
  |      `--> child
  |
  +--> TabStack(active_tab, panels...)
  |
  `--> PanelSlot(panel_id)

FloatingLayer
  `--> FloatingPanel(panel_id, logical_rect)
```

A gameplay panel MAY transition between:

```text
docked <-> tabbed <-> floating <-> hidden/minimized
```

The model MUST preserve stable semantic `panel_id` values independently of transient widget instances.

### 7.1 Persistence

Persisted UI layout MUST use a versioned schema with explicit migration/reset behavior.

Persistable state MAY include:

- dock-tree structure;
- panel identity and visibility;
- active tabs;
- split orientation and normalized ratio;
- floating logical rectangle;
- panel-specific presentation settings;
- selected UI scale where product policy allows it.

Persistence scope MUST be explicit. Global/account/character-specific state MUST NOT be mixed accidentally. A character-specific scope may be used for gameplay-panel layout only when product requirements choose it; this candidate does not freeze that product choice.

Recovery MUST handle:

- removed/renamed panels;
- schema upgrades;
- invalid ratios/sizes;
- monitor/resolution changes;
- floating windows entirely outside the current work area;
- corrupted local settings.

Recovery MUST fall back to a known safe layout without changing authoritative gameplay state.

## 8. DPI, logical units and resolution handling

UI layout MUST operate in logical units separate from physical framebuffer pixels.

Conceptually:

```text
physical window / framebuffer pixels
        ^
        | OS scale factor
        |
logical UI coordinate space
        ^
        | optional product/user UI scale
        |
semantic widget dimensions
```

The platform/window layer owns observation of OS scale-factor and physical-size changes. UI owns conversion into its logical coordinate system and invalidation of affected layout/text caches. Renderer owns final snapping/rasterization details where appropriate.

The implementation MUST define deterministic rounding/snapping rules so repeated scale/resize events do not cause cumulative geometry drift.

Representative validation MUST include multiple DPI factors, window sizes and ultrawide/aspect-ratio cases. This decision does not freeze one reference DPI as canonical product truth.

## 9. Input, focus, capture and modal routing

The UI MUST consume normalized/semantic input from the accepted input boundary rather than concrete `winit` event types inside feature widgets.

Input routing MUST distinguish at least:

- global/system actions;
- gameplay actions;
- UI navigation actions;
- text/IME context;
- modal context;
- pointer capture/drag context.

### 9.1 Focus

The UI core MUST own deterministic focus traversal and focused-widget identity for UI interactions. Keyboard-only navigation MUST be possible for applicable controls.

A text or modal context MUST be able to suppress incompatible gameplay actions. Focus loss MUST clear/cancel held transient interaction state according to the accepted input lifecycle.

### 9.2 Pointer capture and drag/drop

Pointer capture MUST have explicit ownership and release rules, including focus loss, widget destruction and session/screen replacement.

Drag/drop MUST separate:

```text
drag presentation
        |
        v
semantic drop intent
        |
        v
application/domain validation
```

Dropping an item visually MUST NOT itself mutate authoritative inventory state. The UI emits semantic intent; accepted game/application logic decides whether and how a command is issued.

### 9.3 Context menus and overlays

Tooltips, context menus, popovers, modal dialogs and drag previews belong to an overlay layer with deterministic z-order, clipping escape rules and focus/capture semantics.

## 10. Virtualized collections

Virtualization is a foundation requirement for collections whose logical size can materially exceed the visible viewport.

Expected consumers include:

- chat history;
- battle list;
- market/search results;
- large inventories/containers;
- logs/diagnostics;
- future bestiary/cyclopedia-style lists.

A virtualized collection contract MUST support:

- stable logical item keys;
- visible-range calculation;
- bounded overscan;
- widget/row reuse or equivalent bounded realization;
- incremental insert/remove/update;
- preserved selection/focus semantics;
- deterministic scrolling to an item/key;
- variable row height only when supported by measured implementation evidence.

Feature code MUST NOT depend on all logical rows having permanent widget objects.

## 11. Text, localization and IME

The text subsystem MUST separate:

```text
localized semantic string
   -> shaping/layout
   -> glyph references
   -> glyph atlas/cache
   -> UI draw commands
```

It MUST support the product's required Unicode/localization scope and MUST leave extension room for BiDi/script shaping where required. IME composition and committed text are distinct input states.

Font/glyph caches are renderer/presentation resources and are reconstructable. Font or glyph-cache loss MUST NOT alter application/game state.

Text layout MUST participate in DPI/user-scale changes and localization-expansion tests.

## 12. Renderer boundary and UI draw extraction

UI core MUST produce a renderer-neutral immutable or frame-bounded draw representation, referred to here as `UiDrawList`.

A `UiDrawList`-equivalent MAY contain primitives such as:

- solid/gradient rectangles where supported;
- textured/nine-slice quads;
- glyph runs;
- clip/scissor stack operations;
- transform/opacity state;
- ordered layers.

It MUST NOT contain long-lived backend objects such as `wgpu::Device`, `wgpu::Texture`, `wgpu::BindGroup` or platform window handles.

The renderer adapter owns:

- mapping typed asset/glyph handles to GPU resources;
- batching compatible UI primitives;
- clip/scissor implementation;
- buffer reuse/transient arenas;
- pipeline/bind-group caching;
- GPU submission and recovery.

World rendering and UI rendering MAY share renderer infrastructure, but their semantic extraction paths remain distinct:

```text
world projection -> world render extraction -+
                                           +-> renderer graph/passes -> present
UI view/presentation -> UiDrawList ----------+
```

Neither path owns authoritative game state.

## 13. UI assets, themes and design tokens

UI presentation MUST consume typed logical assets from the client-safe content/release boundary rather than arbitrary filesystem paths in feature logic.

The architecture MUST leave room for typed concepts equivalent to:

```text
UiImageId
IconId
FontId
Material/StyleId
LocalizationKey
```

Exact type names are deferred.

The asset/style layer SHOULD support metadata required by reusable UI, including where applicable:

- nine-slice/border-image regions;
- intrinsic dimensions;
- animation frames/timing;
- sampling policy;
- icon semantic identity;
- font metadata;
- theme/style token references.

Reusable primitives SHOULD consume semantic design tokens rather than feature code repeating hard-coded colors, spacing, border radii, font sizes and state colors.

A theme layer MAY override tokens, but theme selection MUST NOT change gameplay semantics or capability/security truth.

## 14. Accessibility and developer diagnostics

The retained UI tree MUST preserve enough semantic information to construct an accessibility representation for applicable controls, including stable role/name/state/value relationships where supported by the platform implementation.

The foundation SHOULD expose inspectable diagnostics for development/testing, including:

- widget identity/path;
- computed logical rectangle;
- visibility/focus/capture state;
- layout invalidation reason;
- clip bounds;
- realized/virtualized item counts;
- draw primitive/batch counts;
- text/glyph cache statistics;
- UI update/layout/extraction timing.

Diagnostics MUST remain bounded and MUST NOT expose secrets/private content by default.

## 15. Performance contract

Performance claims require a named build, hardware class, display mode, UI scenario and measurement method. Average FPS alone is insufficient.

The foundation MUST make it possible to measure separately:

```text
view-model update
style/layout update
virtualization
text shaping/cache work
UI draw extraction
renderer CPU batching
GPU UI pass
```

The previous greenfield client design carried a **1.0 ms p95** engineering target for combined UI update/layout/extraction in a recommended 144 Hz class. For this canonical repository that value is recorded only as a **provisional benchmark target for the first UI spike**, not as a product SLO or accepted production capacity limit.

Any production numeric UI budget MUST be reconciled with canonical PERF/resource policy and measured native-client scenes before acceptance.

First benchmark scenes SHOULD include:

- mostly static HUD over an active world scene;
- multiple open containers;
- battle-list churn;
- high-volume chat with virtualization;
- tooltip/context-menu churn;
- repeated docking/resizing;
- DPI/scale transitions;
- localization-expanded text;
- long-session open/close/recreate cycles.

## 16. Legacy-client evidence boundary

The existing OTClient implementation is useful **behavioral evidence only**. It MUST NOT become a runtime dependency of the native client and MUST NOT be structurally ported merely because a legacy class already exists.

Audit snapshot reviewed for this candidate: `blakinio/otclient@53646cfa1957ce75f18547424a9e9177c4ad8cc2`.

Observed reusable behavioral evidence includes:

- retained widget hierarchy, focus and clipping;
- flex/grid/anchor/box layout behaviors;
- drag/drop and mouse/keyboard routing;
- miniwindow dragging, minimizing, resizing and persistence;
- multiple gameplay side panels and action-bar regions;
- tooltip, popup, input, scroll, splitter, tab and table behaviors.

Observed limitations that MUST NOT be copied as architectural constraints include:

- hard-coded column-oriented miniwindow docking;
- persistence coupled directly to legacy widget/module identity;
- potentially large collections materialized as one widget per row;
- feature behavior spread across C++/Lua/OTUI callbacks;
- renderer/input/UI coupling patterns that conflict with the accepted native-client ownership model.

For each legacy behavior considered during implementation, classify it as:

```text
PRESERVE_BEHAVIOR
REDESIGN_BEHAVIOR
DROP_BEHAVIOR
```

Do not classify legacy code as `PORT_CODE` by default.

## 17. First proof: HUD Slice 01

The first UI implementation proof SHOULD be one representative gameplay HUD slice, not a complete final product skin.

The target composition is:

```text
+----------------------------------------------------------------+
| status/resources                                               |
+-------------+--------------------------------------+-----------+
| dock area   |                                      | dock area |
|             |           GAME VIEWPORT              |           |
| battle list |                                      | minimap   |
| container   |                                      | equipment |
|             |                                      |           |
+-------------+--------------------------------------+-----------+
| action bar(s)                                                   |
+----------------------------------------------------------------+
| virtualized chat                                                |
+----------------------------------------------------------------+
```

Representative interactions:

- focus transition between gameplay, chat and modal UI;
- one tooltip and one context menu;
- one draggable/dockable panel;
- one virtualized list;
- one container/inventory-style drag intent routed semantically;
- resize and DPI change;
- save/reload of UI presentation layout;
- renderer recreation without changing game/application authority.

The slice MUST use only view-model data that is actually available through accepted application/domain contracts. Missing battle-list, equipment, cooldown, condition, item-detail or other domain data MUST remain explicit implementation dependencies rather than being fabricated inside UI code.

## 18. Implementation phases

### UI-0 — contract reconciliation

Before runtime implementation:

- reconcile exact current ALPHA-CLIENT implementation state;
- inventory current renderer/input/content APIs;
- define the minimum UI view-model contracts needed by HUD Slice 01;
- define `UiDrawList`-equivalent ownership/lifetime;
- select test fixtures and measurement scene;
- register any new stable IDs/contracts through the normal coordination path.

### UI-1 — foundation core

Implement only the feature-neutral primitives needed for the slice:

- retained tree and identity;
- dirty layout/style invalidation;
- minimum flex/row-column/grid behavior;
- clipping/scrolling;
- focus/navigation/modal/capture;
- text/glyph boundary;
- renderer-neutral draw extraction.

### UI-2 — docking, persistence and virtualization

Add:

- dock tree;
- floating/stacked/tabbed panels;
- versioned persistence and safe recovery;
- virtualized collection primitive;
- developer diagnostics required to inspect these systems.

### UI-3 — HUD Slice 01

Compose the representative gameplay HUD using real accepted view-model contracts and semantic input/actions. Do not broaden into full product feature parity.

### UI-4 — evidence and hardening

Prove:

- focused unit/property tests;
- DPI/resolution/localization cases;
- keyboard/focus/modal/IME behavior;
- drag/drop semantic boundary;
- persistence migration/recovery;
- virtualization boundedness;
- renderer resource-loss reconstruction;
- representative UI timing and memory evidence;
- applicable native-client E2E tiers.

### UI-5 — product skin and broader feature surfaces

Only after representative world/content/gameplay data exists, expand to final visual polish, complete panel set, animation treatment and product-specific UX. Do not freeze those choices in the foundation solely to make architecture look complete.

## 19. Test and acceptance requirements

A UI foundation implementation is not complete from isolated unit tests alone.

At minimum, applicable evidence MUST cover:

### Layout and state

- deterministic layout for fixed fixtures;
- dirty invalidation after local changes;
- min/max/intrinsic constraints;
- nested clipping/scrolling;
- invalid layout recovery.

### Input and interaction

- focus traversal;
- modal suppression of gameplay actions;
- text/IME separation;
- pointer capture release on focus loss/destruction;
- drag/drop emits semantic intent without authoritative local mutation.

### DPI and localization

- multiple scale factors;
- representative small/large/ultrawide windows;
- localization expansion;
- text reflow after scale change;
- no cumulative rounding drift across repeated resize/scale cycles.

### Persistence

- round-trip of a versioned dock layout;
- migration or explicit reset from older schema;
- removed panel handling;
- off-screen floating-panel recovery;
- corrupt settings fallback.

### Virtualization

- bounded realized rows with a much larger logical data set;
- stable focus/selection across insert/remove/scroll;
- no dependency on permanent widget objects for non-visible rows.

### Renderer integration

- stable draw ordering;
- nested clipping;
- batching/extraction correctness;
- renderer/device resource recreation without game-state mutation;
- no backend handles exposed through feature UI contracts.

### Product proof

- representative HUD Slice 01 journey through the real native UI path;
- exact-head native-client validation required by repository policy;
- applicable ADR-0007 Tier 2 / Tier 3 evidence once the relevant gameplay/runtime capability exists.

A synthetic/mock UI fixture can prove layout and interaction semantics, but MUST NOT be reported as native gameplay E2E or Reference parity.

## 20. Risks and mitigations

| Risk | Impact | Mitigation |
|---|---|---|
| UI view model becomes a second mutable world model | reconciliation bugs and authority ambiguity | one-way projection, explicit presentation-only state, generation/revision fencing |
| UI toolkit choice leaks into feature contracts | expensive future migration | freeze semantic contracts, defer library choice |
| full-tree layout work each frame | frame-time spikes | dirty/incremental invalidation and measurement |
| one widget per large-list row | memory/CPU growth | virtualization as foundation primitive |
| raw input handled independently by widgets | accidental gameplay commands and inconsistent hotkeys | centralized semantic contexts, focus/modal/capture ownership |
| pixel-based layout assumptions | broken DPI/ultrawide UX | logical units + explicit scale inputs + tests |
| unversioned docking persistence | broken upgrades and off-screen panels | versioned schema + migration/reset/recovery |
| renderer handles in UI state | device-loss coupling | backend-neutral draw extraction and typed resource lookup |
| final art decisions frozen before representative gameplay exists | rework and delayed proof | prove wireframe/HUD slice first; defer final skin |
| provisional performance number treated as product SLO | false capacity claim | label as benchmark target; replace only with canonical measured evidence |

## 21. Consequences

### Positive

- UI can be developed in parallel with world rendering without creating a shared mutable world/UI model.
- Renderer and UI can be profiled independently.
- Docking, input and persistence behavior become reusable foundation rather than per-feature hacks.
- Large lists remain scalable by contract.
- DPI, text and accessibility requirements enter the foundation before the final skin hardens assumptions.
- Legacy OTClient behavior can be mined safely without making Lua/OTUI part of the native runtime.

### Costs

- the first HUD requires deliberate view-model contracts instead of direct feature reads;
- a proper dock/persistence schema is more work than fixed columns;
- virtualization and text shaping increase foundation complexity before every feature screen exists;
- backend-neutral draw extraction requires an explicit renderer integration layer.

These costs are accepted because postponing the boundaries would make later feature work substantially more coupled and expensive to correct.

## 22. Acceptance boundary

Accepting this candidate would authorize **architecture only**. It would not mean:

```text
UI runtime implemented          = NO
HUD Slice 01 proven             = NO
native gameplay available       = NO
final visual design accepted    = NO
production performance proven   = NO
Reference parity proven         = NO
```

Implementation must be separately allocated under the live coordinator/ownership model, use the exact current base at start, satisfy repository build/test/review rules, and preserve all parent authority boundaries.
