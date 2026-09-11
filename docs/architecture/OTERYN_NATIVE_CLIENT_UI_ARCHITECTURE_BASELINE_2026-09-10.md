# Oteryn native client UI architecture baseline

- Date: 2026-09-10
- Repository: `Oteryn/Oteryn-Game`
- Protected source base at branch creation: `main@149e1e5cc3dd09b4bbf53e9213b92901233da210`
- Status: **OWNER-REQUESTED CANDIDATE ARCHITECTURE BASELINE / REVIEW REQUIRED**
- Runtime/product activation authority: **NONE**
- Server/protocol/content/production mutation authority: **NONE**
- Addon/mod/plugin architecture: **DEFERRED / FUTURE CONCEPT / OUT OF CURRENT IMPLEMENTATION SCOPE**

## 1. Purpose

Define the native-client UI architecture early enough that the renderer, gameplay presentation and future HUD work do not become coupled through ad-hoc widgets, direct gameplay-state access or per-screen rendering shortcuts.

This document intentionally fixes **semantic UI boundaries and the first implementation shape**, while leaving the final visual skin, final widget styling, final asset set and final UX polish open until representative world, creature, VFX and gameplay content can be judged together.

This baseline complements rather than replaces:

- `docs/architecture/ALPHA-CLIENT-01_NATIVE_CLIENT_ARCHITECTURE_ANALYSIS.md`;
- `docs/architecture/ALPHA-CLIENT-01_NATIVE_CLIENT_ARCHITECTURE_CONTRACT_CANDIDATE.md`;
- `docs/architecture/OTERYN_GRAPHICS_PRESENTATION_VFX_ARCHITECTURE_BASELINE_2026-09-09.md`;
- `docs/architecture/OTERYN_VISUAL_WORLD_SLICE_ARCHITECTURE_AND_EVIDENCE_GATE_2026-09-09.md`.

The graphics baseline already separates authoritative game state from presentation and adopts custom Rust + `wgpu` as the current renderer foundation. The Visual World Slice deliberately did not freeze final UI architecture. This document fills that specific gap without reopening world-renderer authority or claiming a final visual design.

## 2. Architecture decision timing

### Must decide now? **YES, for UI semantic boundaries and foundation shape**

The next useful native-client proof needs a representative gameplay HUD on top of the world renderer. Implementing that HUD before fixing state ownership, input routing, layout, render extraction, scaling and persistence would create expensive coupling that would later be difficult to remove.

### Concrete downstream work unblocked

This baseline unblocks a bounded UI foundation and one representative HUD vertical slice containing the world viewport frame, player status, chat, minimap, battle list, containers, equipment, action bar and tooltip/drag interaction.

It also gives renderer qualification a realistic UI load so world performance is not measured in isolation from the interface the player will actually use.

### What becomes harder later if decided incorrectly

The high-cost mistakes are:

- allowing widgets to read or mutate authoritative gameplay state directly;
- coupling world and UI rendering into one inseparable pass graph;
- encoding physical GPU resources in application/domain state;
- using physical pixels as the only layout coordinate system;
- implementing docking, focus, modal handling and drag/drop separately in individual panels;
- rebuilding all UI structure every frame;
- persisting raw screen coordinates without versioned layout semantics;
- making large lists render every row regardless of visibility.

### Evidence that may justify supersession

Reopen implementation details when physical client evidence shows a material problem in:

- frame-time or memory cost;
- high-DPI behavior;
- text quality or shaping coverage;
- accessibility/readability;
- docking ergonomics;
- input latency;
- controller/touch support requirements;
- browser-client reuse requirements;
- platform/window-system constraints.

The authority boundary, gameplay separation and deterministic UI-state principles require an explicit later architecture decision to change.

### Deliberately not decided now

This baseline does **not** freeze:

- final HUD proportions;
- final theme, colors, textures, borders or ornamentation;
- final iconography;
- final typography family;
- exact panel defaults at every resolution;
- final animation/motion language;
- controller or touch UX;
- a public mod/plugin API for arbitrary UI widgets;
- a final browser-client rendering technology;
- a final serialization format for all user preferences;
- exact GPU batching thresholds before measurement.

Addon/mod/plugin/community customization is explicitly deferred. No addon runtime, scripting VM, manifest/package format, community API, sandbox/capability system or distribution flow is part of the current UI foundation programme. Current architecture should avoid gratuitous lock-in where cheap, but future extension flexibility must not introduce current implementation complexity.

## 3. Primary authority boundary

The UI is a presentation and interaction surface. It is not gameplay authority.

Conceptually:

```text
Authoritative server/gameplay state
              |
              v
      Client domain/projection
              |
       +------+-------+
       |              |
       v              v
 RenderSnapshot    UiViewModel
       |              |
       v              v
 World Renderer     UI Tree
       |              |
       |          Layout/Input
       |              |
       |          UiDrawList
       |              |
       +-------+------+
               v
        Presentation Compositor
               |
               v
             wgpu
```

User interaction flows in the opposite direction only through typed intents/commands:

```text
OS/window input
      |
      v
UiInputRouter
      |
      v
UiIntent / ClientIntent
      |
      v
Client application/domain command boundary
      |
      +--> local presentation-only state
      |
      +--> validated gameplay/network command path
```

A button, widget, tooltip, panel, drag gesture or hotkey must never directly mutate server-authoritative gameplay state.

## 4. Required separation from the world renderer

World rendering and UI rendering remain separately measurable and separately evolvable.

Required logical split:

```text
Game/client state -> RenderSnapshot -> WorldRenderer
Game/client state -> UiViewModel    -> UiSystem -> UiRenderer

WorldRenderer output + UiRenderer output -> PresentationCompositor
```

The UI may display world-derived data such as target information, health, minimap state or selected objects, but it consumes presentation-safe projections rather than renderer internals.

The world renderer must not know about backpack widgets, chat tabs, battle-list rows, action bars or modal dialogs. The UI renderer must not become the owner of world tile ordering, creature movement, combat timing or gameplay visibility rules.

## 5. Core UI model

### 5.1 Retained UI tree

Adopt a **retained UI tree** for the native gameplay client.

Widgets/panels retain stable identity across frames. The tree is updated when application/UI state changes rather than reconstructed wholesale every frame.

Rationale:

- MMO HUDs contain long-lived panels and stateful interactions;
- focus, docking, text selection, scrolling and drag/drop require stable identities;
- retained nodes permit dirty/invalidation-based layout and rendering;
- stable widget identity supports deterministic automation and diagnostics;
- it avoids turning every frame into a complete allocation/rebuild cycle.

This does not forbid immediate-style helpers for debug tooling or small isolated engineering overlays. Those helpers must remain implementation-local and may not define the production gameplay UI state model.

### 5.2 Reactive view models

UI widgets consume typed **view models** derived from client application/projection state.

Examples:

```text
PlayerStatusViewModel
TargetViewModel
BattleListViewModel
ContainerViewModel
EquipmentViewModel
SkillsViewModel
ChatViewModel
MinimapViewModel
ActionBarViewModel
ConnectionViewModel
```

View models are presentation-safe, read-mostly projections. They may contain already-resolved labels, percentages, semantic presentation keys and interaction availability, but not writable gameplay authority.

The UI update layer should support targeted invalidation so a changed health value does not force unrelated chat/container/layout work.

### 5.3 Local UI state

Purely local UI state is owned by the UI/application presentation layer.

Examples:

```text
focused_widget
hovered_widget
captured_pointer
open_panel_set
active_chat_tab
scroll_offsets
dock_tree
panel_sizes
selected_local_tab
pending_drag
local_tooltip_state
user_ui_scale
```

Local UI state must not synthesize or advance authoritative server sequence, durable inventory state, combat state, movement state, cooldown state or session/lease state.

## 6. Render extraction: `UiDrawList`

UI layout/state resolution and GPU submission must be separated through a renderer-neutral extracted representation, conceptually named `UiDrawList`.

The exact Rust type names remain implementation-local, but the boundary should support primitives such as:

```text
Quad
NineSlice
Image/Sprite
TextRun
Line/Border
ClipPush / ClipPop
TransformPush / TransformPop
```

A frame should conceptually execute:

```text
UiViewModel + local UiState
          |
          v
Retained UiTree update
          |
          v
Measure / layout / interaction resolution
          |
          v
UiDrawList extraction
          |
          v
UiRenderer batching + wgpu submission
```

`UiDrawList` contains logical presentation information, not gameplay authority. Domain/application state must not contain texture handles, bind groups, atlas UVs, pipeline IDs or other GPU-local resources.

Renderer-local caches may map semantic image/font keys to compiled GPU resources.

## 7. Layout coordinate system and DPI

All production UI layout is expressed in **logical UI units**, not raw physical pixels.

Conceptually:

```text
physical_px = logical_ui_unit * platform_dpi_scale * user_ui_scale
```

The implementation must support:

- platform DPI changes while the client is running;
- moving a window between monitors with different scale factors;
- resize without rebuilding unrelated application state;
- user-selected UI scaling independent of world-camera zoom;
- 1080p, 1440p and 4K-class layouts without assuming one fixed canvas;
- crisp pixel-oriented assets through an explicit pixel-alignment policy where required.

World zoom and UI scale are independent settings.

## 8. Layout primitives

The foundation should provide a small set of composable primitives rather than panel-specific geometry code.

Required concepts:

```text
Row / Column
Stack / Overlay
Padding / Margin
Min / Preferred / Max size
Fixed / Content / Fractional sizing
Alignment
Scroll region
Clip region
Anchor
Resizable split
Dock node
```

Layout evaluation should be deterministic for identical inputs, viewport size, DPI scale and user scale.

Expensive measure/layout passes should be invalidated only when relevant content or geometry changes.

## 9. Docking architecture

Tibia-like information density and long play sessions require first-class docking rather than hard-coded sidebars.

Represent the workspace as a **dock tree**, not raw absolute coordinates.

Candidate node model:

```text
DockRoot
  Split(horizontal | vertical, ratio)
  Panel(panel_id)
  TabGroup(active_panel, panels[])
  FloatingPanel(panel_id, logical_rect)
```

Required behaviors:

- left/right/bottom dock regions;
- nested splits;
- tab groups;
- resizable boundaries;
- optional floating panels;
- panel min/max constraints;
- viewport receiving remaining workspace area;
- safe restoration when a saved layout no longer fits the current monitor/resolution.

The default gameplay HUD is one preset built on this system, not a special-case layout engine.

## 10. Layout persistence

Persist semantic layout state, not transient renderer state.

Persistence should be versioned and contain stable identifiers such as:

```text
layout_schema_version
panel_id
dock topology
split ratios
logical floating rectangles
visibility
selected tabs
selected preset
user_ui_scale
```

Do not persist GPU handles, widget pointers, physical monitor coordinates without normalization, temporary hover/focus state or gameplay state.

On incompatible/corrupt/out-of-bounds data, recover to a known safe preset rather than making the client unusable.

## 11. Input routing

All native input enters through one UI/application routing boundary.

Required event families include:

```text
PointerMove
PointerDown / PointerUp
Wheel
KeyDown / KeyUp
TextInput
IME composition
WindowFocus
WindowResize / ScaleChanged
```

Routing order must be deterministic and account for z-order, clipping and modal state.

A useful conceptual order is:

```text
OS event
  -> normalize to logical coordinates
  -> pointer capture if active
  -> modal layer if active
  -> hit test front-to-back
  -> target widget
  -> optional bubble to ancestors
  -> produce UiIntent / consume / pass through
```

Gameplay hotkeys and world interaction receive input only when UI routing does not consume or reserve it under the current focus/modal/capture state.

## 12. Focus, capture and modal rules

The UI foundation must centrally own:

- keyboard focus;
- text-entry focus;
- pointer capture;
- modal stack;
- drag capture;
- tooltip hover target;
- tab/navigation order.

A text box must not leak typed characters into gameplay hotkeys. A drag operation must keep receiving pointer events after leaving the original widget. A modal confirmation must block unintended interaction with the world or panels behind it.

Focus/capture state must be observable in debug tooling so input failures are diagnosable.

## 13. Drag and drop

Drag/drop is a framework-level interaction primitive because it will span inventory, equipment, containers, action bars and world interaction.

Conceptually:

```text
DragPayload
DragSource
DropTarget
DropPreview
DropIntent
```

The UI may optimistically show a local drag preview, but item movement/equipment actions become authoritative only through the normal validated client/game command path and resulting server/client-domain state.

A rejected or changed server result must reconcile the view model cleanly.

## 14. Tooltips and contextual information

Tooltips are managed by one service/layer rather than individually spawned windows.

Required properties:

- delayed hover activation;
- bounded screen placement;
- semantic content model;
- rich text/icon support where needed;
- deterministic lifetime;
- no ownership of gameplay truth;
- ability to show item/equipment/skill details from presentation-safe projections.

Tooltip layout participates in normal DPI scaling and clipping rules.

## 15. Lists and virtualization

Potentially large repeated UI surfaces must support **virtualization**.

Examples:

- battle list;
- containers/inventory grids;
- chat/history;
- skill/stat lists;
- future market/search results.

Only visible or near-visible rows/cells should create expensive layout/render work. Stable item/row identity must be separate from the recycled visual node used to display it.

The implementation must not assume that acceptable performance with ten rows proves acceptable behavior with hundreds or thousands of entries.

## 16. Text and font pipeline

Text is part of the renderer foundation, not a per-widget afterthought.

Required conceptual stages:

```text
Text content
 -> shaping/layout
 -> glyph lookup/rasterization
 -> glyph atlas/cache
 -> TextRun extraction
 -> batched draw
```

The foundation must support:

- Unicode text;
- Polish and other Latin diacritics;
- chat-scale dynamic text;
- multiple logical sizes/weights;
- deterministic measurement;
- clipping/wrapping/ellipsis;
- high-DPI rasterization;
- fallback fonts where required;
- cache eviction/rebuild after renderer/device loss.

Exact shaping/rasterization libraries and final font families remain evidence-gated implementation choices.

## 17. Theme and design tokens

Visual styling should be resolved through semantic tokens rather than hard-coded values in panel logic.

Candidate token families:

```text
spacing.*
radius.*
border.*
font.*
text.*
surface.*
state.hover
state.pressed
state.selected
state.disabled
status.health
status.mana
rarity.*
```

Theme tokens may resolve to colors, textures, nine-slices, fonts, shadows or other presentation resources.

Widget behavior and layout semantics remain reusable across future Classic/Enhanced/Oteryn visual skins where appropriate.

A theme must not change gameplay-significant information availability.

## 18. UI asset boundary

Application/domain code refers to semantic UI asset keys, not physical file paths or GPU addresses.

Conceptually:

```text
UiImageKey / UiIconKey / UiFontKey
      -> theme/presentation resolver
      -> compiled runtime resource
      -> renderer-local handle
```

Physical PNG/KTX2/DDS choices, atlas placement and GPU bindings remain renderer/asset-pipeline concerns.

This mirrors the existing world/VFX rule that gameplay contracts do not encode physical graphics identifiers.

## 19. Performance model

UI must be measured as part of the client frame, not assumed to be cheap.

Initial engineering target for a representative gameplay HUD on the qualified desktop path:

```text
UI CPU update + layout + extraction: <= 1.0 ms p95 target
```

This is an **initial qualification target**, not a production SLO or owner-sensitive global hardware promise. It may be revised by measured evidence.

Required counters/timings should include at minimum:

```text
ui_update_ms
ui_layout_ms
ui_extract_ms
ui_render_submit_ms
ui_draw_primitives
ui_batches
ui_vertices
ui_glyphs
ui_clip_changes
ui_texture_bind_changes
ui_active_widgets
ui_visible_widgets
ui_virtualized_rows
ui_layout_invalidations
```

Performance work should prioritize:

- stable retained nodes;
- dirty/invalidation-driven updates;
- list virtualization;
- glyph/image caches;
- batching by compatible material/texture/clip state;
- bounded clip/scissor changes;
- avoiding per-frame allocation churn;
- avoiding full-tree layout when only local content changed.

## 20. Frame composition and batching

UI rendering uses the same project-owned `wgpu` foundation but keeps a logically distinct UI render path.

Candidate frame ordering:

```text
world opaque/ordered sprite presentation
world VFX/environment
world-space readable overlays where appropriate
UI docked panels/HUD
floating panels
modal layer
tooltips/cursor/debug overlay
```

Exact render-pass merging is a physical optimization and may change after profiling as long as world/UI semantic separation, ordering and observability remain intact.

## 21. Renderer/device loss and reconstruction

UI GPU resources are reconstructible caches.

Device/surface loss must not destroy authoritative application/game state. The UI may temporarily rebuild:

- pipelines;
- bind groups;
- glyph atlases;
- image atlases/caches;
- vertex/index buffers;
- render targets.

Retained semantic UI state and view models survive independently of those resources where the surrounding client lifecycle allows it.

## 22. Testing strategy

### 22.1 Pure deterministic tests

Test without a GPU where possible:

- layout math;
- docking tree transformations;
- persistence round-trip/version migration;
- hit testing;
- focus traversal;
- modal blocking;
- pointer capture;
- drag/drop state transitions;
- tooltip placement;
- virtualization range calculation;
- DPI/user-scale coordinate conversion.

### 22.2 Render extraction tests

Given a deterministic `UiViewModel + UiState + viewport`, verify the extracted logical draw representation where this gives stronger signal than raw image snapshots.

Examples:

- primitive count/range;
- clip hierarchy;
- text-run placement;
- stable z-order;
- expected semantic image keys;
- absence of out-of-bounds geometry.

### 22.3 Physical GPU/client tests

Use the real `wgpu` path for:

- glyph/image cache behavior;
- clipping/scissor correctness;
- resize and DPI changes;
- full representative HUD frame cost;
- world + UI composition;
- device/surface reconstruction where testable;
- high-DPI screenshot/readability evidence.

Visual snapshots are evidence for pixel/layout regressions but must not be the only oracle for interaction or state correctness.

## 23. Representative HUD Slice 01

The first implementation slice should be **game-shaped but deliberately not final-art complete**.

Required visible regions:

```text
+---------------------------------------------------------------+
| top/status area                                               |
+-----------------------------------------+---------------------+
|                                         | minimap             |
|                                         +---------------------+
|                                         | battle list         |
|              WORLD VIEWPORT             +---------------------+
|                                         | equipment / stats   |
|                                         +---------------------+
|                                         | container(s)        |
+-----------------------------------------+---------------------+
| chat / channels                                               |
+---------------------------------------------------------------+
| action bars / contextual interaction                          |
+---------------------------------------------------------------+
```

The exact side, widths and heights are preset defaults, not permanent product decisions.

Minimum functional coverage:

- player HP/mana/status presentation;
- minimap shell;
- battle-list virtualization;
- at least one container grid;
- equipment slots;
- chat tabs/history/input;
- action bar with hotkey labels;
- hover tooltip;
- drag payload between UI targets;
- modal example;
- resize/docking example;
- 100%, 125%, 150% and 200% UI-scale qualification where practical;
- world interaction blocked correctly by focused/modal/captured UI.

## 24. UI qualification scene

Renderer qualification must include a UI stress configuration rather than benchmarking only an empty interface.

Representative load should combine:

- active world viewport from the current visual-world evidence line;
- normal creature/VFX/environment presentation;
- chat receiving repeated messages;
- battle list with enough entries to exercise virtualization;
- multiple open containers;
- minimap updates;
- action bars;
- tooltip churn;
- drag/drop interaction;
- one modal transition;
- resize/DPI-scale transition in a dedicated test.

Synthetic UI stress data must be labelled as workload/fixture evidence and must not be represented as authoritative live gameplay data.

## 25. Proposed implementation ownership shape

Exact crate names may be adjusted to the current workspace, but responsibilities should remain separable.

Conceptually:

```text
client application/domain projection
        |
        v
ui-model        - presentation-safe view models and UI intents
ui-core         - retained tree, layout, docking, focus/input, drag/drop
ui-text         - shaping/glyph/cache abstraction if separation proves useful
ui-render       - UiDrawList extraction consumption and wgpu batching
ui-theme        - semantic tokens/assets/theme resolution
client-shell    - window integration, composition, persistence wiring
```

This is a responsibility map, not authorization to create these exact crates immediately. A bounded implementation slice may begin inside fewer modules if dependency direction and extraction boundaries remain explicit and later separation is low-cost.

## 26. Dependency direction

Desired dependency direction:

```text
server/gameplay/domain
        X
        | no dependency on UI
        v
client projection -> ui-model -> ui-core -> ui-render -> wgpu adapter
                            \-> ui-theme
```

The UI may depend on presentation-safe client types. Gameplay/domain crates must not depend on widgets, layout, windowing, fonts, GPU resources or UI persistence.

## 27. Browser-client future compatibility

The native UI must not be distorted today to force one implementation to run in a browser. However, semantic boundaries should preserve future reuse of:

- view-model contracts;
- UI intents;
- panel identities;
- layout/docking semantics where suitable;
- theme/design tokens;
- gameplay/UI authority separation.

Native `wgpu`, native window/input APIs and native text/resource implementations may remain platform-specific.

A future browser client can implement another physical renderer while consuming equivalent presentation-safe models.

## 28. Legacy/reference policy

Legacy OTClient/Tibia-family UI behavior may be inspected as **reference evidence** for interaction density, discoverability, hotkeys, panel workflows and player expectations.

Do not structurally port Lua/OTUI architecture, widget ownership patterns or renderer coupling merely because they exist in the reference client.

Reference behavior informs requirements; Oteryn owns the new Rust architecture.

## 29. Risks and mitigations

### Risk: retained tree becomes over-engineered

Mitigation: implement only primitives required by HUD Slice 01, measure before broad widget framework expansion.

### Risk: final visual design is frozen too early

Mitigation: stabilize semantic tokens/layout behavior now while treating the first skin as qualification art until representative world/gameplay review.

### Risk: UI steals world-renderer performance budget

Mitigation: separate timers/counters, representative combined qualification, virtualization and invalidation from the first slice.

### Risk: input conflicts cause gameplay errors

Mitigation: one central input router with explicit focus/capture/modal state and deterministic tests.

### Risk: docking persistence becomes brittle

Mitigation: semantic/versioned dock tree, safe defaults, schema migration and bounds recovery.

### Risk: client UI accidentally becomes game authority

Mitigation: typed view models + intents, one-way state projection, validated command boundary, no direct widget mutation of authoritative state.

## 30. Acceptance criteria for the architecture baseline

This architecture baseline is ready for owner/review acceptance when reviewers can confirm that it:

- preserves the existing client/gameplay/presentation authority boundaries;
- does not reopen the custom Rust + `wgpu` renderer decision;
- clearly separates world and UI render paths;
- defines retained UI state, reactive view models and `UiDrawList` extraction;
- centralizes docking, DPI scaling, input/focus/capture/modal and drag/drop semantics;
- defines virtualization, text/font pipeline and theme/asset boundaries;
- makes UI performance independently observable;
- defines deterministic and physical test requirements;
- specifies one bounded representative gameplay HUD slice;
- keeps addon/mod/plugin architecture deferred outside the current programme;
- does not claim final visual design or production readiness.

## 31. Recommended next safe execution sequence

```text
UI-0  inspect current client/render/window/input implementation boundaries
  ->
UI-1  implement minimum ui-core + ui-model + UiDrawList foundation
  ->
UI-2  implement HUD Slice 01 with qualification styling
  ->
UI-3  run deterministic + physical world/UI performance and interaction qualification
  ->
UI-4  produce high-fidelity visual design using proven HUD geometry/content density
  ->
UI-5  polish theme/assets/animation after representative gameplay/world review
```

This ordering intentionally places **functional architecture and wireframe-quality HUD behavior before final visual rendering/design**, while still producing a visually inspectable client early enough to guide implementation.

## 32. Terminal statement

The client UI should be treated as a first-class presentation subsystem with its own state, layout, interaction and rendering boundaries:

```text
Game State -> Client Projection -> UiViewModel -> Retained UI/Layout -> UiDrawList -> UI Renderer
Game State -> RenderSnapshot -------------------------------------------> World Renderer

UI Renderer + World Renderer -> Presentation Compositor -> wgpu
```

The next step is not to build every final panel. The next step is to prove the foundation through one representative gameplay HUD slice while the world/creature/VFX line continues to mature.