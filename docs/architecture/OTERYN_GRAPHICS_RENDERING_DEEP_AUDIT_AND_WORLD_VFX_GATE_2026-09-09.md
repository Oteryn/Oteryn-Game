# Oteryn graphics rendering deep audit and World + VFX evidence gate

- Date: 2026-09-09
- Status: **CANDIDATE DEEP AUDIT / REVIEW REQUIRED**
- Protected Game base at branch creation: `main@c498b68106bee666e54e5f625b911e4b4efd7ee3`
- Related work: Issue #465, PR #468, PR #473, PR #475
- Scope: dynamic Tibia-like floor/tile/stack rendering, floor visibility, movement presentation, appearance composition, effect/projectile lifecycle, lighting/batching, real-content scale and completion criteria for `OTERYN WORLD + VFX PROTOTYPE`
- Runtime/client/server/protocol/content mutation authority: **NONE**
- Product/live activation authority: **NONE**

## 1. Purpose

This document continues the bounded rendering audit after PR #475 without rewriting or invalidating its exact-head evidence.

It answers the next implementation-relevant questions:

1. Which dynamic rendering semantics are missing from the static Tibia -> Oteryn disposition matrix?
2. Which already-protected Oteryn spatial/appearance contracts can be reused directly by the prototype?
3. Which parts of Tibia-compatible client behavior are useful reference evidence but should not become Oteryn authority?
4. What exact evidence must exist before the project may freeze texture organisation, runtime texture container, streaming/cache policy, VFX budgets or renderer batching thresholds?
5. What constitutes terminal completion of the next `OTERYN WORLD + VFX PROTOTYPE` evidence gate?

This document is an evidence gate definition and audit result. It does not implement the renderer and does not grant production authority.

## 2. Evidence boundary

The CipSoft Tibia renderer remains closed-source. No claim in this document that concerns exact runtime implementation may be attributed to CipSoft unless it is supported by official public evidence.

Reference implementation evidence is pinned to:

- `opentibiabr/otclient@dda0520ac61d1626f7d3a6d255394937038ec8ea`;
- `opentibiabr/canary@d34733e1336f0e4f396b45b4bfb93b681407d0bb`;
- official Tibia documentation/news for exposed graphics backends, light-effect settings and day/night behavior.

Open-source OTClient/Canary behavior is engineering reference evidence, not proof of identical CipSoft internal classes, buffers, render passes, shaders, caches, texture packing or algorithms.

Oteryn-internal semantic authority reused by this document includes protected/current-main copies of:

- `OTERYN_WORLD_SPATIAL_COORDINATE_PROFILE_V1.md`;
- `OTERYN_CRYSTALSERVER_LEGACY_SPATIAL_IMPORT_PROFILE_V1.md`;
- `OTERYN_ATLAS_15_32_APPEARANCE_SPATIAL_PROFILE_V1.md`;
- `OTERYN_TIBIA_15_25_APPEARANCE_SPATIAL_PROFILE_V1.md`;
- `OTERYN_ATLAS_ANIMATED_APPEARANCES_V1.md`;
- `OTERYN_ATLAS_15_32_ANIMATION_CENSUS_V1.json`;
- full-world source-export evidence.

PR #468, PR #473 and PR #475 remain candidate work until protected integration. This follow-up must not promote them to protected authority by prose alone.

## 3. Corrections and precision upgrades from the first audit

### 3.1 Modern Tibia asset cells are not 32x32-only

The exact Game-owned 15.32 source profile proves decoded sprite cell layouts of:

```text
32 x 32
32 x 64
64 x 32
64 x 64
```

Therefore the correct formulation is:

> Tibia has a legacy 32-unit tile/sprite heritage, but the current source corpus used by Oteryn is not limited to physical 32x32 sprite cells.

The Oteryn invariant remains unchanged:

```text
logical world unit != physical source texture resolution
```

### 3.2 The existing Oteryn appearance contracts are stronger than a new ad-hoc renderer model

The 15.32 profile already defines:

- `units_per_tile = 32` for source conversion only;
- owning-tile visual anchor;
- west/north visual coverage for oversized decoded cells;
- explicit presentation displacement;
- deterministic concrete frame/phase/pattern/layer/sprite resolution;
- source IDs as provenance, not canonical Game identity;
- GPU page/atlas coordinates as runtime/cache state only.

The renderer prototype must consume or adapt these semantics rather than invent a second incompatible appearance-spatial model.

### 3.3 Existing animation timing should be reused

`animated-appearances-v1` already preserves:

- default start phase;
- synchronized/random-start metadata;
- ping-pong/infinite/counted loop type;
- loop count;
- raw phase duration ranges;
- effective phase duration ranges;
- deterministic presentation midpoint policy for the bounded Atlas capability.

It also explicitly establishes:

```text
appearance animation != movement / AI / combat / server-observed state
```

The prototype may require a client-specific playback policy later, but it must not create a conflicting interpretation of the same source metadata without explicit evidence and supersession.

## 4. Real 15.32 presentation corpus

The Game-owned 15.32 evidence is large enough to drive realistic prototype cases.

### 4.1 Source profile

```text
catalog entries:             5,090
sprite sheets:               5,084
object appearances:         43,514
object sprite refs:        111,957
unique object sprite IDs:   79,269
catalog sprite ID domain: 0..301,200
```

### 4.2 Animation census

```text
objects:
  appearances              = 43,514
  animated appearances     = 5,190
  maximum observed phases  = 125

outfits:
  appearances              = 1,480
  animated appearances     = 1,466
  animated frame groups    = 1,732

effects:
  appearances              = 243
  animated appearances     = 240
  maximum observed phases  = 27

missiles:
  appearances              = 76
  animated appearances     = 0
  pattern width/height      = 3 x 3 for all 76
```

These values are corpus facts, not recommended runtime hard maxima.

### 4.3 Full-world scale

Qualified offline source evidence records:

```text
floors:                    16
source tiles:              18,997,668
presentation records:     24,502,036
resolved primitives:      24,502,035
unresolved presentations: 1
unique visible appearances: 25,198
unique visible sprite IDs:  27,394
```

The bounded Thais Z7 fixture records:

```text
tiles:        24,311
presentations: 39,282
primitives:    39,282
```

### Derived consequence

The runtime architecture must be based on a bounded visible/near-visible working set plus cache/streaming behavior. Loading the complete static world into GPU resources is not a credible target architecture.

The prototype must use real-content working-set churn rather than treating complete-world counts as simultaneously visible draw counts.

## 5. Static same-tile order is necessary but not sufficient

`oteryn-world-spatial-v1` already defines:

```text
PresentationOrderKey {
  plane: i32,
  order: u32
}
```

for deterministic same-position static presentation ordering.

The CrystalServer import profile currently preserves the evidenced visible legacy static sequence as:

1. ground if present;
2. top-level tile items in source order;
3. no flattening of nested container contents.

This is correct for the bounded static import purpose.

However, a production game frame also contains dynamic presentation classes that are not reducible to one static same-tile order key:

- walking creatures crossing tile boundaries;
- transient effects;
- projectiles;
- creature-attached VFX;
- environment particles;
- large visual coverage crossing neighboring tiles;
- UI/gameplay overlays;
- lighting composition.

### Requirement

Do not reinterpret the current static `PresentationOrderKey` as a complete runtime scene-stack ABI.

The prototype must prove a separate dynamic composition mechanism while preserving existing static order semantics.

Candidate implementation concepts may include render phases, semantic render classes, generated sort keys or equivalent bounded commands. Exact names/bit layouts remain deferred.

## 6. Dynamic tile-stack evidence from the Tibia-compatible reference

The pinned OTClient reference classifies things semantically rather than using only arbitrary `z`:

- ground;
- ground border;
- on-bottom;
- common;
- creature;
- on-top.

Its tile draw path also contains explicit ordering for ground, borders, bottom items, common items, creatures, effects and top items.

### Oteryn disposition

`COPY` the principle that render ordering is semantic.

`REPLACE` the implementation details and legacy workarounds with a project-owned `wgpu` render-command/sort model.

The prototype must not be considered correct if it merely sorts every visible primitive by one floating-point depth value without proving Tibia-like overlap cases.

## 7. Elevation/displacement are presentation, not world-Z authority

The reference renderer accumulates per-object draw elevation and subtracts it from the 2D draw origin. Existing Oteryn appearance profiles separately convert source `shift` and source `height` into explicit presentation displacement.

Required boundary:

```text
presentation elevation/displacement
    != canonical floor identity
    != gameplay collision height
```

The prototype must test stacked elevated items/objects while proving that their draw displacement does not mutate canonical `WorldTilePosition` or gameplay footprint.

## 8. Large visual coverage must not use neighbor-redraw hacks as the target model

The reference OTClient contains legacy overlap handling including redrawing neighboring creatures/top things after some large lying objects/corpses.

This is useful evidence that large sprite coverage creates real ordering problems, but it is not a target architecture.

Oteryn already has explicit visual coverage and anchor metadata in the Game-owned appearance profiles.

### Requirement

The prototype must use explicit coverage/anchor/displacement plus deterministic scene sorting/occlusion to prove large-object overlap.

It must not require arbitrary previous-tile redraw as the primary production design.

Mandatory cases include at least:

- 64x64 visual covering four tiles;
- creature partially behind a large object;
- on-top object crossing another tile visually;
- animated oversized appearance;
- camera scroll while the object crosses viewport/cache boundaries.

## 9. Floor visibility is a first-class correctness problem

The reference client computes visible floors dynamically relative to the camera and surrounding geometry rather than simply drawing one `z` level.

Reference behavior examines local tiles and properties associated with look-through/ground/wall/projectile-blocking behavior to constrain upper visible floors.

### Oteryn consequence

The prototype needs a `FloorVisibilityResolver` or equivalent presentation boundary.

The resolver must be tested independently from static stack order.

Mandatory fixtures include:

- open surface;
- underground room;
- roof/covered tile;
- doorway/opening;
- wall-adjacent camera position;
- floor transition/stairs or equivalent transition fixture;
- camera movement while first/last visible floors change;
- object/effect/projectile near an occluded floor boundary.

### Authority boundary

Renderer floor visibility is presentation state. It must not silently become server line-of-sight, projectile legality or combat visibility authority.

If gameplay visibility rules require the same semantic facts, those facts must come from the owning gameplay/world contract rather than from the result of GPU/presentation culling.

## 10. Walking is a visual transition, not per-frame authoritative movement

The reference client keeps server-observed tile position and separately applies a walk offset/progress during drawing. Camera following can consume that walk offset.

Diagonal walking also has special ordering behavior around obstacles to keep overlap visually correct.

### Oteryn requirement

The prototype should model a presentation transition conceptually equivalent to:

```text
VisualStep {
  entity,
  from,
  to,
  direction,
  presentation_start,
  presentation_end,
  reconciliation identity/context
}
```

Exact fields are deferred.

Hard rules:

- rendered sub-tile interpolation does not mutate authoritative position each frame;
- animation completion cannot authorize movement;
- stale/replaced projection state can cancel/rebuild interpolation;
- resynchronization/snapshot replacement reconstructs scene state safely.

Mandatory cases:

- cardinal walk;
- diagonal walk;
- walk past elevated/large obstacle;
- camera follow during walk;
- floor transition;
- correction/resync during visual transition.

## 11. Creature appearance must support composition

The current reference client demonstrates that creature presentation can include more than one flat sprite resource:

- base outfit;
- color-mask layer;
- addons;
- mount presentation;
- attached effects;
- attached particles;
- creature-local light;
- optional shader/material path.

The Game-owned `animated-appearances-v1` already normalizes important outfit semantics including direction patterns, addon rows and two-layer color masks.

### Requirement

Do not define production creature appearance as one physical sprite ID.

The presentation architecture must permit ordered appearance components/attachments with explicit semantic anchors/layers.

Candidate concepts:

```text
AppearanceComponent
AttachmentAnchor
PresentationLayer
```

Names are not frozen.

Classic/Enhanced/HD may resolve those components differently while preserving the same authoritative outfit/mount/addon state.

## 12. Effect lifetime requires explicit semantic classes

Reference effects can be finite, infinite or explicitly removed. Reference clients may also use local deduplication/wait behavior for overlapping visual effects.

Oteryn should not copy those heuristics as gameplay semantics.

### Required presentation categories

The prototype must distinguish at least:

1. finite fire-and-forget VFX;
2. persistent/cancellable presentation VFX;
3. gameplay-owned persistent world fields represented in snapshot/state;
4. local ancillary VFX derived from an already accepted event.

### Identity

Cancellable/persistent transient effects should have an unambiguous semantic instance/event identity such as `PresentationInstanceId`/`EventId` or equivalent.

Do not rely on only `(position, physical_effect_id)` as the final Oteryn removal identity.

### Hard boundary

```text
visual effect lifetime != gameplay status/effect lifetime
```

A missing, delayed, culled or degraded visual instance must not extend or terminate gameplay state.

## 13. Modern Tibia-compatible magic effects are already composite

For protocol versions represented by the pinned reference at `>=1203`, the magic-effect parser handles a command sequence capable of including:

- create effect;
- create distance effect;
- reversed distance effect;
- delay/delta command classes;
- main/secondary sound commands.

This is important evidence that modern Tibia presentation is not accurately described as only `effect ID + position`.

### Oteryn disposition

`COPY` the ability for one semantic gameplay event to produce multiple coordinated presentation results.

`REPLACE` the low-level wire-command representation as the primary Oteryn presentation contract.

Prefer:

```text
semantic gameplay observation/event
    -> EffectKey / PresentationRecipe
        -> coordinated local timeline/components
```

Server messages may still carry authoritative timing, area, source/target and required semantic variant information. They should not need to encode GPU/resource-level presentation commands.

## 14. Effect pattern semantics must be explicit

The real 15.32 census contains effect appearances with pattern dimensions larger than 1x1:

```text
1x1: 232 appearances
2x2:   5 appearances
3x3:   6 appearances
```

Reference effect drawing can derive pattern indices from world position.

### Requirement

The Oteryn producer/resolver must own the semantic pattern-selection policy for legacy-compatible effects or export enough semantic inputs for one deterministic client rule.

Do not let a low-level shader or atlas coordinate accidentally become the authority for choosing a gameplay-visible effect variant.

Pattern selection remains presentation-only unless a gameplay contract explicitly attaches meaning to it.

## 15. Projectile presentation: preserve source/target semantics, not legacy timing formula

Reference missiles:

- have distinct missile appearance family;
- interpolate from source toward destination locally;
- select direction/pattern visually;
- remove themselves after presentation duration.

The exact 15.32 census records 76 missile appearances, all one-phase and all with 3x3 pattern dimensions.

### Oteryn requirement

`ProjectileEvent` or equivalent should provide semantic source/destination/trajectory and authoritative timing context where required.

The renderer may interpolate orientation/trail/light/particles locally.

Do not copy a reference-client distance-duration formula as gameplay authority.

Hard invariant:

```text
visual missile location != hit/collision authority
```

## 16. Lighting should be a compositing subsystem with a low-cost path

The reference client separates global light from local light sources and composites a light representation over the map. Objects, creatures and attached effects may contribute local lights.

Official Tibia player settings allow light effects to be toggled, demonstrating a product-level need for a lower-cost presentation path.

### Oteryn requirement

The prototype must exercise at least:

- global ambient light;
- time-of-day change;
- static object light;
- creature/attached light;
- short spell light pulse;
- quality mode reducing/disabling non-critical local lighting.

Gameplay semantics must remain identical when optional visual lighting is reduced.

Do not freeze the final light texture/grid/compute/deferred implementation before measurement.

## 17. Batching: copy the principle, not OTClient DrawPool

The reference OTClient groups draws by compatible state and can pack eligible textures into an atlas. State includes factors such as texture, shader, blend/composition, transform, opacity and clipping.

This is useful proof that a Tibia-like scene benefits from command grouping.

### Oteryn target

```text
semantic presentation
    -> RenderCommands / instances
        -> visibility + semantic ordering
            -> batch/group by compatible GPU state
                -> wgpu submissions
```

The `wgpu` implementation may use instancing, storage buffers, bind groups and other evidence-backed mechanisms.

Do not clone an OpenGL-style DrawPool API merely because the reference client has one.

### Required prototype metrics

Record at least:

- semantic visible primitives;
- generated GPU instances;
- draw calls/submissions;
- batch count;
- average/max batch size;
- state breaks by resource page/texture, pipeline/material, blend and clipping where measurable.

## 18. Streaming/cache architecture must follow working-set evidence

Full-world evidence proves the corpus is much larger than one viewport.

The prototype must separate:

```text
complete content corpus
resident CPU cache
resident GPU cache
visible set
prefetch/near-visible set
```

Required churn cases:

- continuous cardinal camera movement;
- diagonal movement;
- zoom change;
- floor transition;
- rapid return to recently visited area;
- cold entry into new area;
- alternating between two areas that exceed a deliberately small test cache;
- missing presentation-family variant fallback.

Measure upload stalls, eviction/reload activity and frame-time tails.

No streaming/cache topology is accepted until this evidence exists.

## 19. Current production-code gap in Oteryn

Current protected-main client/runtime code has useful foundations but no production world renderer.

### Present today

`crates/client-domain` currently provides only a bounded non-authoritative entity projection with approximately:

```text
entity_ref
position { x, y, floor }
display_name
```

`crates/client-simulation` deterministically applies projection upsert/remove and exposes a revisioned snapshot.

`crates/renderer` provides bounded renderer/surface/resource-lifetime foundations including `wgpu` device/surface handling and generation-fenced resource cache behavior.

`apps/client` remains in `PreNativeProtocol` gameplay availability.

### Not present as production runtime

No protected-main implementation currently establishes the complete production versions of:

- world tile render snapshot;
- dynamic tile-stack resolver;
- floor visibility resolver;
- appearance runtime resolver;
- VFX event/instance system;
- projectile renderer;
- lighting/environment system;
- production asset streaming/cache;
- renderer-facing real-world batching path.

This is not a defect. It means the project can still choose the correct seams without migrating a large legacy world renderer.

## 20. Do not prematurely promote the synthetic client projection

The ALPHA-CLIENT architecture candidate explicitly says the current synthetic `client-domain` / `client-simulation` crates are not automatically promoted into the production gameplay model.

The first World + VFX prototype should therefore consume bounded normalized fixtures/exports and semantic test events rather than forcing final world/presentation types into the current synthetic model solely to obtain a demo.

After prototype evidence, a separate accepted runtime contract may define the final seam from client projection to presentation snapshot/events.

## 21. Reuse matrix for the prototype

| Need | Existing Oteryn evidence/contract | Prototype action |
|---|---|---|
| coordinates/floor identity | `oteryn-world-spatial-v1` | reuse |
| static same-tile order | `PresentationOrderKey` | reuse for static records |
| legacy floor/order conversion | CrystalServer import profile | reuse fixture semantics |
| appearance anchor/coverage | 15.32 appearance profile | reuse |
| displacement | 15.32 appearance profile | reuse |
| concrete frame/pattern/layer resolution | 15.32 appearance/export tooling | reuse |
| animation timing metadata | `animated-appearances-v1` | reuse/adapt explicitly |
| real source corpus | 15.32 pinned bundle/census | use for representative cases |
| full-world static corpus | full-world source export | use for streaming/churn source |
| dynamic walking render order | not frozen | prototype/evidence required |
| floor visibility/roof occlusion | not frozen | prototype/evidence required |
| effect/missile runtime | census only in current Atlas contract | prototype/evidence required |
| PresentationRecipe/VFX graph | PR #473 candidate | prototype/evidence required |
| weather/seasons | PR #473 candidate | prototype/evidence required |
| asset GPU layout | intentionally deferred | benchmark |

## 22. Required prototype architecture shape

The evidence prototype should remain bounded/non-production while proving the following conceptual flow:

```text
normalized Game-owned world/appearance fixture
             +
semantic dynamic presentation events
             +
environment state
             |
             v
     presentation resolver
             |
             +--> static world commands
             +--> dynamic entity commands
             +--> VFX/projectile commands
             +--> environment/light commands
             |
             v
 visibility / floor / occlusion / ordering
             |
             v
       batch + resource resolve
             |
             v
            wgpu
```

The prototype must preserve the intended future one-way authority boundary but does not need to define the final production protocol/client-domain type names.

## 23. Mandatory functional slices

The prototype is incomplete unless it includes all of the following.

### World

- real multi-floor tile data;
- ground/borders/items;
- deterministic same-tile static order;
- elevation/displacement;
- oversized visual coverage;
- walls/roofs/occlusion;
- dynamic first/last visible-floor behavior.

### Creatures

- real outfit appearance program;
- four-direction presentation where supported;
- idle animation;
- moving animation;
- cardinal movement interpolation;
- diagonal movement interpolation;
- addon/layer composition;
- one representative mounted/layered case if source semantics are qualified for the chosen fixture.

### Combat/VFX

- finite area spell;
- projectile A -> B;
- physical hit;
- elemental hit;
- heal;
- death effect;
- persistent/cancellable VFX instance;
- boss telegraph with authoritative area/timing input;
- at least one composite recipe using more than one primitive class.

### Presentation primitives

Exercise at minimum:

- frame animation;
- particle emitter;
- dynamic light;
- trail or equivalent projectile accompaniment;
- temporary decal;
- sound event integration boundary (actual final audio implementation is not required if outside prototype scope).

### Environment

- ambient/global light;
- day/night interpolation or bounded transition;
- rain;
- snow;
- wind parameter affecting ambient presentation;
- at least one winter/season appearance variant with deterministic fallback.

### Camera/UI

- continuous camera movement;
- zoom;
- floor transition;
- names;
- HP bars;
- representative gameplay overlay/telegraph.

## 24. Real-content workload construction

The prior #465 bake-off used synthetic 36-frame quads. That was appropriate for engine comparison but is insufficient for the World + VFX gate.

The next workload must mix real normalized source semantics.

Recommended workload families:

### BASIC

Representative ordinary scene with:

- normal viewport tile density;
- items/creatures from real appearance programs;
- small number of animations/lights;
- no artificial mass-spam.

### NORMAL

Representative busy hunt/town scene with:

- multiple creatures;
- animated world objects;
- projectiles/spells;
- names/HP;
- environment effect;
- camera movement and resource churn.

### STRESS

Deliberately heavy but bounded combat scene with:

- many creatures;
- simultaneous area effects/projectiles/hits;
- gameplay-critical telegraphs;
- ambient weather;
- dynamic lights;
- active camera movement/zoom;
- forced cache churn.

The exact counts must be declared and reproducible. They are benchmark parameters, not production hard maxima.

## 25. Presentation density matrix

Continue testing the 32/64/128 product direction, but interpret it correctly:

```text
32 / 64 / 128 = presentation-density experiment classes
```

not:

```text
canonical logical tile sizes
```

Where exact original 15.32 source assets have only the qualified 32/64 source geometry, higher-density test assets must be public-safe/synthetic or separately authorized and must preserve the same semantic footprint/anchor.

## 26. Completion criterion 1 — functional correctness

Do not mark the prototype complete until:

- floor/tile/static order is deterministic;
- dynamic creature/effect/projectile ordering passes named fixtures;
- roof/wall/floor occlusion has no known P0/P1 correctness defect;
- oversized appearance overlap is deterministic;
- walking interpolation does not mutate authoritative position per frame;
- projectile/hit/AoE/telegraph visual timing consumes authoritative semantic timing rather than causing it;
- visual failure/culling cannot create gameplay state.

## 27. Completion criterion 2 — presentation architecture

Required proof:

- no gameplay/domain contract depends on PNG/DDS/KTX path, atlas UV, texture-array layer, GPU handle, bind group, pipeline ID or runtime page ID;
- static world presentation consumes semantic order/appearance data;
- transient effects consume semantic events/recipes;
- environment presentation consumes semantic environment state;
- at least one effect composes multiple presentation primitives;
- Classic/Enhanced/HD-equivalent variants preserve identical gameplay-significant semantics.

## 28. Completion criterion 3 — environment

Required proof:

- day/night does not require complete duplicate day/night worlds;
- rain and snow are presentation-owned rather than thousands of gameplay entities;
- season variants use deterministic fallback;
- environment visuals do not modify gameplay unless a separate authoritative contract explicitly supplies that gameplay effect.

## 29. Completion criterion 4 — performance evidence

Final benchmark evidence must run on a named physical GPU and record, where trustworthy:

- CPU frame p50/p95/p99;
- GPU frame p50/p95/p99;
- mean throughput/FPS;
- frame pacing/tails;
- process RAM;
- VRAM/resident GPU memory;
- visible semantic primitives;
- GPU instances;
- draw submissions/calls;
- batch count and batch sizes;
- texture/resource uploads;
- cache hits/misses/evictions;
- camera-scroll stalls;
- zoom/floor-transition stalls;
- active particles;
- active lights;
- startup/load cost for the chosen fixture.

If the platform cannot provide a trustworthy metric, mark it unavailable rather than substituting an invented estimate.

Raw measurements and exact workload identity must be retained.

## 30. Completion criterion 5 — reliability

The final matrix must:

- complete without unexplained crash/device loss/surface failure;
- have no known P0/P1 renderer correctness defect;
- include repeated camera movement/zoom/floor transition/visibility churn;
- include resource eviction/reload pressure;
- include repeated VFX spawn/despawn and persistent-effect cancellation;
- document and reproduce any failure that remains accepted as a bounded caveat.

A static visible-set stress loop alone is insufficient.

## 31. Completion criterion 6 — asset-system evidence

Required proof:

- more than one resource page/sheet is exercised;
- runtime upload/cache churn is real, not mocked away;
- at least one deterministic eviction/reload scenario exists;
- missing presentation-family/density variant uses deterministic fallback;
- malformed/missing source references fail safely;
- atlas/array/container decisions are not based only on static synthetic workloads.

## 32. Completion criterion 7 — gameplay readability

At STRESS load:

- critical boss/spell telegraphs remain visible/readable;
- gameplay-significant area/timing remains equivalent across presentation families;
- decorative ambient effects degrade before critical VFX;
- optional lighting reduction does not hide gameplay-significant information;
- weather does not obscure required gameplay markers beyond accepted thresholds.

Any quality mode that changes gameplay-significant information fails this criterion.

## 33. Completion criterion 8 — technology decision outputs

After the prototype, issue separate evidence-backed verdicts using exactly:

```text
ADOPT
REJECT
INSUFFICIENT_EVIDENCE
```

for at least:

- atlas vs texture arrays vs justified hybrid;
- KTX2 vs DDS vs another runtime container;
- streaming/cache model;
- particle implementation direction;
- lighting implementation/budget direction;
- VFX budget/degradation direction;
- renderer batching thresholds;
- RAM budget direction;
- VRAM budget direction;
- mip/filter/compression policy by relevant asset class.

A verdict may be conditional by platform/asset class where evidence genuinely requires that result.

## 34. Completion criterion 9 — evidence discipline

Every `ADOPT`/`REJECT` verdict must cite measured evidence from the prototype.

If evidence is missing, contradictory or not trustworthy, the result is `INSUFFICIENT_EVIDENCE`.

Do not infer undocumented CipSoft renderer internals from OTClient/Canary code.

Do not copy a Tibia-compatible reference implementation solely for parity when the same semantic result can be implemented more cleanly through Oteryn-owned contracts.

## 35. Completion criterion 10 — integration discipline

For any repository artifact intended to become protected architecture/implementation evidence:

- fresh protected-main reconciliation is required;
- applicable exact-head review/CI must pass;
- repository Merge Queue/control-plane authority remains intact;
- protected-main readback is required before claiming protected completion.

Review/CI/Merge Queue waiting states are checkpoints rather than evidence that runtime/prototype acceptance has already been achieved.

## 36. Decisions that remain deliberately unfrozen

This deep audit still does **not** choose:

- KTX2;
- DDS;
- one universal texture container;
- one universal atlas layout;
- texture arrays for all sprite classes;
- bindless/virtual texturing;
- final GPU compression;
- final mipmap policy;
- final filtering policy;
- final streaming cache topology;
- final renderer pass graph;
- final UI technology;
- final particle compute path;
- final light representation;
- final Rust public type names for presentation events/recipes;
- production hard resource maxima.

Those are outputs of the evidence gate, not prerequisites for starting it.

## 37. Prototype success does not grant live authority

Even a fully successful World + VFX prototype is still evidence.

It may support follow-up architecture/runtime decisions and bounded implementation allocations.

It does not by itself authorize:

- production server behavior changes;
- protocol authority changes;
- content activation;
- public redistribution of third-party assets beyond existing rights evidence;
- live deployment;
- removal of repository protection or Merge Queue gates.

## 38. Final deep-audit conclusion

The audit no longer supports treating the next graphics step as a generic sprite benchmark.

The project already has enough Game-owned semantic source material to test a credible Tibia-like renderer:

```text
world coordinates/floors/order
+ exact 15.32 appearance spatial semantics
+ exact animation metadata/census
+ full-world source projection
+ current wgpu renderer foundation candidate
```

The remaining hard questions are dynamic presentation questions:

```text
floor visibility
+ walking overlap/interpolation
+ dynamic stack composition
+ effects/projectiles
+ layered outfits
+ lights/environment
+ batching
+ streaming/cache churn
+ readability under stress
```

Therefore the next evidence gate should be:

```text
OTERYN WORLD + VFX PROTOTYPE
```

with the completion criteria in this document treated as mandatory evidence requirements rather than optional polish.
