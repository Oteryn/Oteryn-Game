# Oteryn native client UI architecture and implementation plan

- Date: 2026-09-10
- Repository: `Oteryn/Oteryn-Game`
- Source baseline: `main@149e1e5cc3dd09b4bbf53e9213b92901233da210`
- Planning branch: `agent/client-ui-architecture-baseline-20260910`
- Parent architecture: `docs/architecture/OTERYN_NATIVE_CLIENT_UI_ARCHITECTURE_BASELINE_2026-09-10.md`
- Viewport experiment: `docs/architecture/OTERYN_NATIVE_CLIENT_VIEWPORT_AB_EXPERIMENT_PLAN_2026-09-10.md`
- Status: **EXECUTION PLAN CANDIDATE / INDEPENDENT AUDIT REQUIRED**
- Runtime implementation authority from this document: **NONE**
- Final FOV policy: **UNDECIDED / EVIDENCE-GATED**
- Addon/mod/plugin architecture: **DEFERRED / FUTURE CONCEPT / OUT OF CURRENT IMPLEMENTATION SCOPE**

## 1. Purpose

Turn the accepted native-client direction and the current UI architecture candidate into a concrete, reviewable implementation programme that can be executed as small pull requests without prematurely freezing product decisions that require physical evidence.

This plan deliberately separates:

1. architectural invariants that must exist before UI implementation expands;
2. reversible implementation structure;
3. the bounded `responsive FOV` versus `fixed gameplay FOV` experiment;
4. later gameplay-domain integration;
5. final visual polish.

The immediate objective is not to build every gameplay screen. The immediate objective is to create a production-quality UI foundation, prove it with one representative HUD slice, and generate evidence strong enough to decide viewport/FOV behavior.

## 2. Decision timing

### Must decide now?

**YES** for:

- UI/gameplay authority separation;
- retained UI ownership;
- renderer-neutral draw extraction;
- input/focus/capture/modal routing;
- logical UI coordinates and DPI handling;
- world viewport as a first-class layout output;
- deterministic testing and observability boundaries.

**NO** for:

- final responsive-versus-fixed gameplay FOV policy;
- exact maximum tile extent;
- exact server relevance/interest limits;
- final HUD proportions and skin;
- addon/mod/plugin runtime or community extension API;
- final text shaping library, atlas strategy or package format where the contract can remain implementation-neutral.

### Concrete downstream work blocked without this plan

- creation of the first native UI foundation crate/module;
- deterministic shell-to-UI input routing;
- first UI renderer pass;
- representative HUD Slice 01;
- controlled A/B viewport qualification;
- later safe connection of production gameplay projections to UI view models.

### What becomes expensive if done incorrectly now

- renderer and UI becoming inseparable;
- widgets directly mutating gameplay state;
- physical pixels becoming persistent layout state;
- every panel inventing its own focus, drag/drop and modal semantics;
- hard-coded `15x11`, `60x60`, or another tile extent spreading through renderer/UI code;
- tying server relevance to monitor resolution before fairness and load are measured;
- introducing many UI crates before actual dependency pressure exists;
- allowing synthetic qualification structures to become accidental production authority.

### Evidence that may supersede this plan

- physical frame-time and memory measurements;
- resize/DPI failures;
- renderer/device-loss findings;
- accessibility/text-shaping evidence;
- A/B player-experience and fairness evidence;
- server/network interest-window cost;
- later accepted gameplay/client projection contracts.

## 3. Explicit non-goals

The current programme does **not** implement or design:

- addons;
- mods;
- plugin manifests;
- community extension APIs;
- Lua/WASM or another scripting runtime;
- plugin sandboxing/permissions;
- addon package distribution;
- theme marketplace/community packaging.

Those remain a future concept only. Current code should avoid gratuitous lock-in where easy, but **future addon flexibility is not allowed to add current crates, abstractions, capability systems, package formats, loaders or runtime complexity**.

This programme also does not:

- authorize production traffic;
- change server authority;
- change protocol semantics merely to make the viewport experiment possible;
- select a final field-of-view policy without evidence;
- claim Reference parity from synthetic fixtures;
- port legacy OTClient Lua/OTUI architecture.

## 4. Current repository facts that shape the plan

At the source baseline:

- `apps/client` is the production client composition root;
- `apps/client` already depends on `client-runtime`, `input-actions`, `input-platform` and `renderer`;
- `client-runtime` remains intentionally narrow;
- `client-domain` and `client-simulation` are `synthetic-only`;
- `renderer` is production-scoped and owns the Windows/DX12 `wgpu` surface lifecycle;
- the physical Windows shell currently handles window creation, resize, redraw and renderer lifecycle but does not yet compose a production UI;
- the current renderer `present()` path is still a clear pass rather than a world/UI pass graph;
- `tools/synthetic-client-harness` already composes synthetic client-domain/simulation, input, renderer and synthetic assets, making it the preferred first qualification surface;
- there is no current UI foundation crate in the workspace.

These facts mean the first UI work should extend existing seams rather than introducing a parallel client runtime.

## 5. Frozen architectural invariants

The following are required throughout all implementation slices.

### 5.1 Authority

```text
server/gameplay authority
        |
        v
client projection / view-safe state
        |
        +--------------------+
        |                    |
        v                    v
world/render projection   UI view models
        |                    |
        v                    v
world renderer          retained UI tree
                             |
                             v
                         UiDrawList
                             |
                             v
                        UI render path
```

UI widgets never become gameplay authority.

### 5.2 Input

```text
winit / OS events
      |
      v
input-platform
      |
      v
input-actions / semantic contexts
      |
      +----> UI route: focus / modal / capture / text / drag
      |
      `----> gameplay intent only when not consumed/reserved by UI
```

No individual panel may create a second global input-routing system.

### 5.3 Rendering

World and UI remain logically separate and independently measurable even if profiling later shows that a physical GPU pass can be merged safely.

```text
WorldRenderInput ----> World extraction ----+
                                           |
UiDrawList ----------> UI extraction -------+--> frame composition --> present
```

### 5.4 Scaling

```text
OS physical pixels
      <-> platform DPI scale
      <-> logical client units
      <-> user UI scale
```

`world zoom`, `UI scale`, `window size`, and `world viewport size` are independent concepts.

### 5.5 Viewport

No production subsystem may assume a universal fixed map dimension such as `15x11`, `18x14`, `60x60`, or another magic tile count.

The current product FOV choice remains evidence-gated.

## 6. Initial implementation ownership model

Avoid an early five-crate UI hierarchy. Start with one reusable UI foundation crate and app-local composition. Split only when measured dependency pressure justifies it.

### 6.1 New foundation surface

Proposed initial path:

```text
crates/ui-core/
```

Responsibility only:

- stable widget/node IDs;
- retained tree lifecycle;
- logical geometry and layout primitives;
- invalidation/dirty propagation;
- focus and navigation state;
- pointer capture and modal stack;
- drag/drop interaction state;
- docking tree data model and deterministic transformations;
- UI scale/DPI-independent geometry types;
- virtualized collection range calculation;
- renderer-neutral draw primitives / `UiDrawList`;
- text measurement/shaping interfaces, not necessarily the physical shaping implementation;
- diagnostics counters/events needed to measure UI work.

Hard dependency prohibitions:

```text
ui-core -X-> wgpu
ui-core -X-> winit
ui-core -X-> protocol/server crates
ui-core -X-> client gameplay/domain authority
ui-core -X-> product-specific panels such as battle/market/quest
```

`ui-core` may depend only on genuinely foundational, framework-neutral types needed by the implementation. Any dependency addition must be justified in the slice PR.

### 6.2 App-local production composition

Proposed path:

```text
apps/client/src/ui/
    mod.rs
    composition.rs
    view_model.rs
    viewport.rs
    persistence.rs
    hud_slice01.rs
```

These names are an execution starting point, not an eternal public API.

This layer owns:

- assembly of production UI nodes;
- translation from presentation-safe client state to view models;
- connection of normalized input to UI interaction;
- construction of the world viewport rectangle from remaining layout space;
- persistence adapter for local layout/UI settings;
- initial HUD Slice 01 composition.

### 6.3 Existing renderer

`crates/renderer` remains GPU/surface/resource owner.

It may gain a bounded UI rendering entry point that consumes renderer-neutral draw data from `ui-core`. Product widgets, docking rules, gameplay state and view-model logic must not move into `renderer`.

### 6.4 Existing input crates

`crates/input-platform` remains platform normalization.

`crates/input-actions` remains semantic input/action contracts.

Do not add UI-specific duplicate physical-key abstractions unless a concrete missing primitive is demonstrated. UI-specific focus/capture state belongs in `ui-core`; application mapping belongs in `apps/client`.

### 6.5 Synthetic qualification harness

`tools/synthetic-client-harness` is the preferred first end-to-end qualification host for:

- deterministic HUD data;
- UI load/stress fixtures;
- resize sequences;
- A/B viewport policy;
- frame/performance counters;
- screenshot/reference evidence when useful.

Synthetic evidence must remain labelled synthetic/qualification evidence.

## 7. Core contracts to implement

Exact Rust identifiers can evolve inside a slice, but the semantic contracts below are required.

### 7.1 `UiFrameContext`

Conceptually contains:

```text
logical_window_size
physical_window_size
platform_scale_factor
user_ui_scale
delta/frame timing needed by presentation
input batch / current interaction facts
```

It does not contain writable gameplay authority.

### 7.2 UI view-model boundary

Initial representative view-model families:

```text
PlayerStatusViewModel
TargetViewModel
BattleListViewModel
ContainerViewModel
EquipmentViewModel
ChatViewModel
MinimapViewModel
ActionBarViewModel
ConnectionViewModel
```

For the synthetic qualification phase these may be fed by synthetic adapters. Production adapters arrive only after matching client/domain contracts exist.

### 7.3 Local UI state

```text
focus
hover
pointer capture
modal stack
active drag
open panel set
dock tree
panel sizes
selected tabs
scroll positions
user UI scale
```

This state survives ordinary redraw and does not become part of authoritative simulation state.

### 7.4 `UiDrawList`

Minimum semantic primitive set required before HUD Slice 01:

```text
Quad
Border/Line
NineSlice or equivalent scalable panel primitive
Image/Sprite reference
TextRun
ClipPush
ClipPop
Transform or translated group if proven necessary
```

The draw list must not contain:

- gameplay commands;
- authoritative entities;
- raw file paths as the only asset identity;
- persistent GPU handles owned outside the renderer;
- wgpu bind groups/pipelines as domain/UI state.

### 7.5 Dock tree

Minimum node semantics:

```text
Root
Split(horizontal | vertical, ratio)
Panel(stable_panel_id)
TabGroup(active, children)
FloatingPanel(stable_panel_id, logical_rect)
```

Required operations are deterministic and unit-testable:

- split;
- tab;
- detach/floating;
- resize ratio;
- close/hide;
- restore;
- safe recovery when persisted geometry is invalid for the current monitor/window.

HUD Slice 01 does not require every sophisticated docking affordance before first render; it requires the data model and enough behavior to prove left/right/bottom dock movement and viewport reflow.

### 7.6 Virtualization

Define a reusable visible-range contract independent of battle/chat/market details.

Inputs:

```text
item_count
viewport_extent
scroll_offset
estimated_or_known_item_extent
overscan
```

Output:

```text
visible/recycled item range
```

First required consumers: battle list and chat history. Containers may use grid virtualization when the tested capacity justifies it.

### 7.7 Text

Required contract before final typography selection:

```text
text + style + width constraints
        -> measurement/shaping result
        -> glyph/image resource requests
        -> TextRun draw primitives
```

Must support Unicode, Polish diacritics, wrapping/clipping/ellipsis and DPI-correct measurement. Physical library selection is allowed inside the implementation PR if it is reversible behind this boundary.

## 8. World viewport architecture

The world viewport is an output of workspace layout, not a fixed-size child with a hard-coded tile matrix.

### 8.1 Layout output

Conceptually:

```text
WorldViewportLayout {
    logical_rect,
    physical_rect,
    platform_scale,
    world_zoom,
}
```

The actual Rust shape may differ.

Docking, chat height, action bars, side panels and manual window resize can change the `logical_rect` without changing world zoom.

### 8.2 Separate extents

Never collapse these into one field:

```text
RenderExtent            - what the client renderer would like to cover
AvailableProjection     - world data currently available client-side
GameplayVisibility      - rules/LOS that determine meaningful visibility
ServerRelevanceExtent   - what the server is willing/required to project
```

A renderer may draw only data that exists in the valid client projection. Empty/unavailable outer regions must not synthesize hidden gameplay data.

### 8.3 Resize processing

Required sequence:

```text
WindowEvent::Resized / scale-factor change
        |
        v
shell physical dimensions
        |
        v
logical UI workspace recomputation
        |
        v
WorldViewportLayout changes
        |
        +--> UI layout invalidation
        |
        +--> renderer viewport/scissor/projection update
        |
        `--> qualification FOV policy evaluation
```

Continuous resize must be safe under rapid event sequences and zero/minimized dimensions.

### 8.4 FOV policy is not yet a product setting

During qualification only, implement an internal policy switch conceptually equivalent to:

```text
FixedGameplayFov
ResponsiveFov
```

It is a developer/qualification seam, not a promised end-user preference.

## 9. Viewport A/B experiment implementation

### Variant A — responsive FOV

At fixed world zoom, increasing the usable `WorldViewportLayout` increases requested visible tile extent.

Expected behavior:

```text
larger world viewport
   -> more logical world coverage
   -> more tiles/entities potentially rendered
   -> if current projection is insufficient, record projection deficit
```

Stage A1 is client/synthetic only. It **must not** silently change server/protocol behavior.

If A wins the client-side evaluation, a later separately authorized server/relevance spike measures the real cost before product acceptance.

### Variant B — fixed gameplay FOV

Keep one baseline gameplay tile extent for the comparison and uniformly fit it into the available world viewport.

Requirements:

- no non-uniform stretching of world geometry;
- preserve aspect ratio;
- letterbox/pillarbox or another explicitly measured presentation fallback is preferable to distortion during this experiment;
- UI can still reflow around the map;
- larger resolution primarily improves physical image quality/UI space rather than world information.

The baseline tile extent used by the experiment is a fixture value, not automatically a product constant.

### Test matrix

Minimum physical/logical scenarios:

```text
1280x720
1920x1080
2560x1440
3440x1440
3840x2160
```

Plus:

- continuous manual-style resize path small -> large -> small;
- at least 100%, 125%, 150% and 200% effective UI/platform scaling combinations where practical;
- at least one narrow/tall pathological window within the supported minimum window size;
- minimized/zero-size transition and restore;
- same world zoom in A and B for direct comparison.

### Common synthetic scene

Use one deterministic scene for both variants:

- local player centered/followed;
- dense ground/obstacle tiles extending beyond both candidate viewports;
- enough creatures to expose visible-entity-count differences;
- animated/effect load representative of current visual-world qualification ability;
- minimap active;
- battle list populated;
- multiple containers open;
- chat activity;
- action bars;
- tooltip churn;
- one drag/drop path.

No A/B result is valid if the variants use different scene state or workload timing.

### Required metrics

Client:

```text
window_physical_width/height
window_logical_width/height
world_viewport_logical_width/height
world_viewport_physical_width/height
fov_policy
world_zoom
requested_tile_width/height
effective_tile_width/height
rendered_tiles
visible_creatures
world_extract_ms
world_render_ms
ui_update_ms
ui_layout_ms
ui_extract_ms
ui_render_ms
cpu_frame_ms
gpu_frame_ms where available
frame allocations where practical
peak transient UI/world buffers where practical
```

If Variant A later reaches a server/relevance spike, additionally record:

```text
server_relevance_width/height
entities_projected
map/update bytes per second
messages/updates per second
server projection CPU time
server projection allocation/memory impact
```

### Fairness review

The A/B evidence package must explicitly answer:

- Does a larger monitor/window reveal enemies/players substantially earlier?
- Does it change target acquisition opportunity?
- Does it materially alter PvP reaction time or scouting?
- Does it materially improve PvE safety beyond presentation comfort?
- Is the advantage bounded and acceptable for the intended product profile?
- Could the same benefit be made available through normal zoom/settings on common hardware?

Do not select Variant A purely because it looks better or Variant B purely because it matches legacy behavior.

## 10. Implementation DAG

```text
UI-P0  architecture + implementation plan independent audit
  |
  v
UI-P1  ui-core deterministic foundation
  |
  +--------------------+
  |                    |
  v                    v
UI-P2 shell/input/DPI   UI-P3 renderer UiDrawList path
  |                    |
  +---------+----------+
            v
UI-P4 text/assets/theme minimum
            |
            v
UI-P5 HUD Slice 01 on synthetic qualification data
            |
            v
UI-P6 viewport A/B physical qualification
            |
            v
UI-P7 FOV decision checkpoint
            |
            +--> Fixed FOV accepted -> bind fixed policy
            |
            +--> Responsive preferred -> separate server/relevance spike required
            |
            `--> Neither acceptable -> bounded hybrid follow-up decision
            |
            v
UI-P8 production view-model adapters as gameplay/client contracts become available
            |
            v
UI-P9 high-fidelity visual polish and broader panels
```

`UI-P8` and `UI-P9` may advance incrementally by feature. They are not permission to bulk-port all legacy modules.

## 11. Pull-request slice plan

Each slice is intended to be independently reviewable and mergeable through normal repository governance. Exact PR numbers are not assigned here.

### UI-P0 — independent architecture audit

Scope:

```text
docs/architecture/OTERYN_NATIVE_CLIENT_UI_ARCHITECTURE_BASELINE_2026-09-10.md
docs/architecture/OTERYN_NATIVE_CLIENT_VIEWPORT_AB_EXPERIMENT_PLAN_2026-09-10.md
docs/architecture/OTERYN_NATIVE_CLIENT_UI_ARCHITECTURE_IMPLEMENTATION_PLAN_2026-09-10.md
```

Required outcome:

- reviewer classifies `KEEP`, `FIX`, or `NEEDS_DECISION`;
- no architecture status promotion merely because CI is green;
- audit confirms addon work is out of current scope;
- audit confirms final FOV remains evidence-gated;
- audit identifies any conflict with accepted ALPHA-CLIENT / renderer / visual-world contracts.

Exit: architecture plan is accepted for execution or corrected before runtime work.

### UI-P1 — deterministic `ui-core` foundation

Primary paths:

```text
Cargo.toml
crates/ui-core/**
```

Initial modules/responsibilities:

```text
id / tree
geometry / scale
layout
focus / capture / modal
input interaction state
dock model
virtualization
draw primitives
text interfaces
diagnostics
```

Must not touch:

```text
server gameplay
protocol
production networking
client-domain semantics
visual art assets
final HUD design
```

Tests:

- retained node identity/lifecycle;
- deterministic layout;
- dirty propagation;
- nested clipping geometry;
- focus traversal;
- pointer capture;
- modal blocking;
- docking transformations;
- invalid persistence geometry recovery helpers if included;
- virtualization visible-range math;
- DPI/logical conversion;
- deterministic draw extraction ordering.

Exit:

- crate has no `wgpu`/`winit`/gameplay authority dependency;
- pure tests pass;
- public surface is sufficient for P2/P3 without product-specific widgets.

### UI-P2 — shell, input and DPI composition

Primary paths:

```text
apps/client/Cargo.toml
apps/client/src/windows_shell.rs
apps/client/src/ui/composition.rs
apps/client/src/ui/viewport.rs
crates/input-actions/** only if a demonstrated semantic primitive is missing
crates/input-platform/** only if a demonstrated platform event mapping is missing
```

Implement:

- create UI runtime/composition state beside renderer state;
- normalize resize and scale-factor changes into logical workspace dimensions;
- route keyboard/pointer/wheel/text/IME/focus events through existing input foundation into UI;
- UI consume/pass-through decision before gameplay intent;
- update `WorldViewportLayout` on every valid geometry change;
- preserve renderer resize lifecycle;
- fail safely on zero/minimized dimensions.

Exit:

- resize updates UI/world viewport without recreating gameplay state;
- focused text/modal UI cannot leak input to gameplay route;
- scale-factor transition is deterministic and test-covered;
- shell remains composition root, not UI framework owner.

### UI-P3 — renderer-neutral UI draw path

Primary paths:

```text
crates/renderer/Cargo.toml
crates/renderer/src/**
crates/ui-core/src/draw* or equivalent
apps/client/src/ui/** only for composition glue
```

Implement:

- consume `UiDrawList` through a bounded renderer entry point;
- basic quad/border/scissor/clip support;
- stable z/order semantics;
- UI timing/counter hooks;
- surface loss/reconfigure does not destroy semantic UI state.

Do not implement final skin.

Exit:

- deterministic draw-list fixture renders through physical `wgpu` path;
- renderer owns GPU resources;
- `ui-core` has no `wgpu` dependency;
- world and UI timings remain logically separable.

### UI-P4 — minimum text and presentation assets

Primary paths are determined by the asset/text implementation selected by the audited architecture, but ownership remains:

```text
UI semantic text/layout -> ui-core/application
physical glyph/image resources -> renderer/asset layer
```

Implement only what HUD Slice 01 needs:

- Unicode text measurement and rendering;
- font fallback sufficient for target test text;
- glyph cache/atlas reconstruction;
- semantic UI image keys for initial qualification assets;
- minimal design tokens for spacing/text/surfaces/status.

Exit:

- Polish diacritics render and measure correctly;
- resize/DPI produces stable text geometry;
- renderer/device reconstruction can rebuild physical caches;
- panel logic contains no raw GPU objects.

### UI-P5 — HUD Slice 01

Primary product composition:

```text
apps/client/src/ui/hud_slice01.rs
apps/client/src/ui/view_model.rs
apps/client/src/ui/persistence.rs
tools/synthetic-client-harness/**
```

Visible functional slice:

- world viewport region;
- HP/mana/status;
- minimap shell;
- battle list;
- equipment slots;
- at least two containers/backpacks if fixture complexity permits;
- action bars;
- chat tabs/history/input;
- target state;
- buffs/debuffs placeholder/view-model surface where source data exists;
- item tooltip;
- one modal;
- drag/drop between UI targets;
- docking resize/reflow demonstration.

Data source for this phase:

**synthetic qualification adapters unless a production-safe projection already exists.**

Exit:

- no widget reads/writes server/protocol authority directly;
- representative scene exercises virtualization, text, drag/drop, docking and modal input;
- synthetic fixture is clearly labelled and cannot be mistaken for Reference parity.

### UI-P6 — viewport A/B implementation and qualification

Primary paths:

```text
apps/client/src/ui/viewport.rs
tools/synthetic-client-harness/**
renderer/world presentation paths needed only for measured viewport extraction
```

Implement internal qualification switch:

```text
A = responsive FOV
B = fixed gameplay FOV
```

Run full matrix from Section 9.

Required evidence artifact:

- exact code SHA;
- exact hardware/runner class where physical GPU evidence is gathered;
- exact window/resolution/scaling matrix;
- machine-readable metrics where practical;
- screenshots/video only as supplemental UX evidence;
- summary of fairness implications;
- explicit limitations of synthetic evidence.

Exit:

- A and B compared on identical scene/workload;
- no final policy selected by implementation author alone;
- evidence is sufficient for independent architecture/product review.

### UI-P7 — FOV decision checkpoint

No implementation is authorized by this plan beyond the experiment until the evidence is reviewed.

Possible outcomes:

#### Outcome B — fixed gameplay FOV

Adopt fixed policy and define the fixture-independent product extent/zoom behavior through a later accepted decision.

#### Outcome A — responsive FOV preferred

Before acceptance, create a separate bounded spike for:

- server relevance/interest cost;
- network update volume;
- gameplay fairness;
- maximum negotiated/requested viewport bounds;
- failure behavior when server-approved projection is smaller than requested render extent.

No protocol/server expansion is implicit in UI-P6.

#### Outcome C — neither acceptable

A hybrid may be evaluated only after evidence shows why both A and B fail product needs. Do not design the hybrid pre-emptively.

Exit: an explicit reviewed decision names the selected product behavior and what evidence supports it.

### UI-P8 — production view-model integration

Integrate one gameplay surface at a time as production-safe client projections become available.

Recommended order:

```text
player status
connection/session presentation
target
containers/equipment
chat
battle list
cooldowns/buffs
skills/progression
party/loot/quest and later screens
```

Each feature PR must define:

- source projection;
- view-model translation;
- allowed UI intents;
- authoritative command/result path;
- stale/disconnect/reconnect behavior;
- deterministic tests.

Synthetic models may remain in the harness but must not leak into production authority.

### UI-P9 — high-fidelity design and broader UI

Only after foundation and representative density are proven:

- final HUD proportions;
- final typography;
- production textures/nine-slices/icons;
- motion/animation language;
- additional game panels and large screens;
- accessibility polish;
- broader resolution presets.

Do not bulk-port legacy OTClient modules. Use legacy behavior as coverage/reference evidence only.

## 12. Validation strategy by layer

### Pure/unit

Run continuously in P1+:

- layout;
- focus;
- docking;
- virtualization;
- DPI math;
- draw ordering;
- persistence migration/recovery;
- input consume/pass semantics.

### Integration without physical GPU

- view-model -> tree update;
- tree -> draw-list extraction;
- deterministic resize sequence;
- deterministic A/B requested extent calculation;
- synthetic HUD behavior.

### Physical client/GPU

- Windows surface create/resize/minimize/restore;
- DPI monitor transition where test infrastructure permits;
- UI/world composition;
- text/glyph rendering;
- clipping;
- continuous resize;
- device/surface recovery;
- A/B performance matrix.

### CI principle

Focused checks during development, then the repository-required exact-head gate for every integration candidate. Local/synthetic PASS is not equivalent to protected/integration PASS.

## 13. Performance and observability

Keep the UI baseline initial target:

```text
UI update + layout + extraction <= 1.0 ms p95
```

This remains an engineering qualification target, not an immutable hardware SLO.

At minimum expose counters/timers for:

```text
ui_update_ms
ui_layout_ms
ui_extract_ms
ui_render_ms
ui_active_nodes
ui_visible_nodes
ui_layout_invalidations
ui_draw_primitives
ui_batches
ui_glyph_count
ui_clip_changes
virtualized_visible_rows
world_viewport_logical_size
world_viewport_physical_size
requested/effective tile extent
```

A viewport policy cannot be accepted from FPS alone; frame-time components and workload scale must be visible.

## 14. Persistence and rollback

Persist only local presentation state:

- dock topology;
- split ratios;
- panel visibility;
- floating logical rectangles;
- selected tabs;
- user UI scale;
- stable layout schema version.

Do not persist:

- GPU handles;
- transient focus/hover/capture;
- server-authoritative gameplay state;
- synthetic fixture state.

Recovery behavior:

```text
missing settings -> safe default layout
unknown schema -> migrate if supported, otherwise safe default
invalid/out-of-bounds floating rect -> clamp/rehome
corrupt layout -> safe default, not client startup failure
```

Each runtime slice must be independently revertible without requiring protocol/server rollback unless that later slice explicitly changes such a contract.

## 15. Failure semantics

Required fail-safe behavior:

- UI render failure must not fabricate gameplay state;
- missing optional UI asset should use a bounded fallback or explicit degraded presentation, not crash uncontrolled;
- invalid layout state recovers to safe defaults;
- text shaping/render resource failure is observable and bounded;
- zero-size/minimized window suspends relevant physical work safely;
- viewport request larger than available projection must not expose unknown world state;
- A/B qualification flag is internal and cannot silently mutate server policy;
- any later relevance negotiation fails closed to server-approved bounds.

## 16. Security and trust boundaries

Even though addon work is deferred, current architecture must preserve these simple invariants:

- UI does not receive session credentials unnecessarily;
- renderer does not receive account/auth authority;
- UI intents go through the normal validated application/game command path;
- presentation state cannot bypass gameplay validation;
- text/chat/item labels are treated as untrusted presentation inputs for bounded allocation/layout purposes;
- large lists and text runs have bounded work/resource behavior.

Do not create a generalized extension/plugin capability system in this programme.

## 17. Review checkpoints

Independent review is required at the following architecture-sensitive boundaries:

1. P0 architecture/execution plan before runtime implementation starts.
2. P1 public `ui-core` dependency direction and contracts.
3. P3 renderer/UI ownership seam.
4. P6 A/B evidence package.
5. P7 final FOV decision, especially if it implies server/protocol/relevance changes.

Routine panel implementation after those boundaries may use normal risk-based repository review.

## 18. Independent audit checklist for the next agent

The auditing agent should read live GitHub state first and audit the exact PR head, not this document in isolation.

Required questions:

### Architecture

- Does the plan preserve accepted ALPHA-CLIENT authority boundaries?
- Does `ui-core` remain framework-neutral and free of gameplay/GPU authority?
- Is app composition kept in `apps/client`?
- Is the renderer limited to physical rendering/resource responsibilities?
- Are input-platform/input-actions reused instead of duplicated?

### Scope

- Are addons/mods/plugins truly deferred with no hidden implementation burden?
- Does the plan avoid bulk-porting legacy OTClient?
- Are final art/skin decisions deferred until representative HUD/world evidence exists?

### Viewport/FOV

- Is any magic fixed tile count accidentally made architectural?
- Are render extent, available projection, gameplay visibility and server relevance distinct?
- Can both A and B be implemented without changing server/protocol in the first experiment?
- Is Variant B free from non-uniform world stretching?
- Does Variant A require a later server/relevance spike before acceptance?
- Is fairness reviewed independently from rendering performance?

### Delivery

- Are PR slices small enough to review and revert?
- Does every slice have objective entry/exit criteria?
- Is synthetic evidence labelled correctly?
- Are exact-head repository CI and normal protected integration preserved?
- Are there any unnecessary new crates or abstractions?

### Audit terminal classification

Return one of:

```text
KEEP
FIX
NEEDS_DECISION
```

For `FIX`, identify exact document section and smallest required correction.

For `NEEDS_DECISION`, identify the precise owner-sensitive decision that cannot be resolved from accepted architecture or evidence. Do not broaden the blocker to unrelated implementation work.

The independent audit should explicitly bind its result to:

```text
repository: Oteryn/Oteryn-Game
pull_request: #549
base: main
exact_head: <live PR head at audit time>
```

A review of an older head does not qualify a later materially changed candidate. The auditor must not implement UI-P1 or later runtime slices in the same audit pass; P0 is review/correction only.

## 19. Execution sequencing constraints

- P1 must not start until P0 architecture audit is resolved sufficiently for implementation.
- P2 and P3 may proceed in parallel only after P1 contracts are stable enough and their path ownership is disjoint.
- P4 requires P3 draw path and P1 text interfaces.
- P5 requires P2 + P3 + minimum P4.
- P6 requires a functioning P5 representative HUD/scene and measurable world viewport seam.
- P7 requires complete A/B evidence.
- server/protocol relevance expansion, if any, cannot start merely because Variant A exists; it requires the P7 decision path and separate authority.
- P8 production view-model work is dependency-driven by available client/gameplay projections, not by a requirement to complete every feature before UI proof.
- P9 polish must not destabilize unproven foundations to chase screenshot quality.

## 20. Recommended first implementation package after audit

If P0 returns `KEEP`, the first worker should implement only **UI-P1**.

Exact target:

```text
NEW: crates/ui-core/**
MODIFY: Cargo.toml workspace membership
```

No app shell, renderer, server, protocol, game-domain or production gameplay changes in that first runtime PR.

Required proof:

```text
cargo fmt --check
cargo clippy for affected workspace target with repository lints
cargo test -p <ui-core package>
repository path-selected checks
fresh exact-head required gate before integration
```

The P1 PR should finish with a small public API sufficient for P2/P3 and should resist adding product panels "while already here".

## 21. Terminal states for the programme

### Foundation terminal

```text
UI_FOUNDATION_PROVEN
```

Requires:

- ui-core merged;
- shell/input/DPI wiring merged;
- UI draw path merged;
- minimum text/assets merged;
- HUD Slice 01 physically demonstrated;
- deterministic and physical checks green;
- performance observable.

### Viewport decision terminal

One of:

```text
VIEWPORT_FIXED_FOV_ACCEPTED
VIEWPORT_RESPONSIVE_FOV_ACCEPTED_AFTER_SERVER_RELEVANCE_PROOF
VIEWPORT_HYBRID_REQUIRES_FOLLOWUP_DECISION
```

No viewport terminal may be claimed from implementation existence alone.

### Current document terminal

This document itself reaches:

```text
UI_IMPLEMENTATION_PLAN_AUDITED
```

only after an independent agent has reviewed the exact PR head and returned `KEEP`, or all `FIX` findings have been resolved and re-reviewed.

## 22. Final execution summary

The intended path is:

```text
AUDIT CURRENT PLAN
    ->
ONE SMALL UI FOUNDATION CRATE
    ->
WIRE EXISTING INPUT + DPI + RESIZE
    ->
ADD RENDERER-NEUTRAL UI DRAW PATH
    ->
ADD MINIMUM TEXT/ASSETS
    ->
BUILD REPRESENTATIVE SYNTHETIC HUD
    ->
TEST RESPONSIVE FOV VS FIXED GAMEPLAY FOV
    ->
REVIEW PERFORMANCE + FAIRNESS + SERVER COST IF NEEDED
    ->
FREEZE FOV POLICY ONLY FROM EVIDENCE
    ->
CONNECT PRODUCTION GAMEPLAY VIEW MODELS IN SMALL SLICES
    ->
HIGH-FIDELITY UI POLISH
```

Addons/mods/plugins stay outside this programme. The client should preserve clean internal boundaries, but no addon architecture work is justified before the native UI, viewport behavior and representative gameplay HUD are proven.