# Oteryn Visual World Slice — architecture and evidence gate

- Date: 2026-09-09
- Status: **OWNER-REQUESTED NEXT-SLICE SPECIFICATION / NOT RUNTIME AUTHORITY**
- Follows: graphics architecture #468/#473/#475, World + VFX prototype #480, merged physical qualification #489/#505
- Primary purpose: turn the proven renderer foundation into a directly inspectable, game-shaped visual scene before additional renderer technology is frozen.

## 1. Outcome

Build one bounded **OTERYN VISUAL WORLD SLICE** that looks and behaves like a real Oteryn game scene rather than a synthetic benchmark.

The slice must combine:

- real Game/Atlas full-world semantic data;
- real appearance and creature/outfit presentation data;
- locally consumed real pixel bytes when the exact authorized source is available;
- Oteryn-owned dynamic presentation for movement, combat VFX and environment state;
- interactive camera/floor/roof inspection;
- Classic / Enhanced / HD presentation profiles with identical gameplay semantics;
- live debug and performance instrumentation;
- reproducible physical evidence on the supported Windows GPU path.

This is a **visual vertical slice and evidence gate**, not a production-content release and not a second game/runtime authority.

## 2. Proven foundation that must be preserved

The slice consumes the protected graphics direction rather than reopening it:

- custom Rust + `wgpu` remains the current renderer foundation;
- Tibia-like floor/tile/stack presentation remains the world readability model;
- `object / outfit / effect / missile` presentation semantics remain distinct;
- authoritative gameplay damage, hit, AoE, combat timing, creature state and movement remain server-owned;
- renderer-facing state remains based on `RenderSnapshot + PresentationEvents + EnvironmentState`;
- VFX remain composable rather than encoded as one monolithic spell renderer;
- Classic / Enhanced / HD may differ in presentation quality but must not change gameplay state, hit timing, movement timing, AoE meaning or server authority.

Protected #505 established that the current renderer can exercise a digest-pinned real Atlas FullWorld workload on RX 9070 XT without overflow fallback after cache-capacity correction. The next slice uses that evidence as a starting point; it does not reinterpret it as final production performance budgets.

## 3. Explicit non-decisions

The Visual World Slice must **not** freeze any of the following merely to make the slice complete:

- KTX2 vs DDS or another final texture/container format;
- atlas vs texture arrays vs a future hybrid arrangement;
- final streaming/cache/bundle format;
- final client UI architecture or visual design;
- final compression, mipmap, anisotropy or filtering policy;
- final particle backend;
- final production RAM/VRAM limits;
- final global light/VFX population limits;
- final batching thresholds.

A choice may be used locally as an implementation fixture, but it must be labelled `SLICE_FIXTURE`, not an accepted product decision, unless new evidence separately satisfies the architecture decision discipline.

## 4. Source classification — never mix these categories

Every datum used by the slice must be attributable to one of these classes.

### 4.1 `REAL_ATLAS`

Read-only, pinned Game/Atlas-derived world presentation data:

- tile coordinates and floor;
- ordered static presentations;
- appearance source IDs;
- resolved presentation primitives;
- sprite source IDs;
- width/height and 32x32 / 32x64 / 64x32 / 64x64 layout;
- displacement and visual coverage;
- animation metadata already accepted by Game/Atlas contracts;
- static creature/NPC records and real spawn positions when available;
- real creature/outfit presentation templates.

`REAL_ATLAS` is presentation evidence. It must not be promoted into collision, walkability, AI, live occupancy, combat or movement authority.

### 4.2 `LOCAL_PRIVATE_PIXEL_BYTES`

Exact authorized source pixel bytes or pinned Atlas pixel publication may be consumed locally for the physical visual slice.

Rules:

- proprietary/raw source archives are never committed;
- the repository may retain public-safe hashes, source roots, counts, dimensions and measurements;
- generated screenshots/evidence must respect the same rights boundary as the source data;
- absence of local pixel authorization must fail closed for a real-pixel claim rather than silently substitute invented artwork and call it real.

### 4.3 `SYNTHETIC_DYNAMIC`

Oteryn-owned deterministic presentation state used where Atlas intentionally has no live gameplay authority:

- player/creature `VisualStep` movement A→B;
- direction changes driven by the slice fixture;
- projectile travel A→B;
- hit/heal/AoE presentation events;
- boss telegraph timing;
- particles;
- trails and decals;
- local lights;
- rain/snow/fog/wind/day-night/winter state;
- controlled creature population amplification for stress testing.

Synthetic dynamic records must never be described as real Atlas combat, real AI or real live spawns.

### 4.4 `EXPLICIT_SEMANTIC_FIXTURE`

Small deterministic fixtures used for cases that must not be inferred from pixels:

- roof visibility and indoor/outdoor transition;
- doorway boundary;
- stairs/floor transition;
- lower-floor visibility hole;
- wall-adjacent camera case;
- elevation/displacement vs world-floor distinction;
- visual overhang without gameplay-footprint inference.

These fixtures exist because Atlas visual products do not claim collision/walkability/terrain authority.

## 5. Canonical scene target

The default target is one **96x96** world region with **three adjacent floors**, preferably a qualified Thais-like region already represented by the pinned FullWorld product.

A 64x64 region is acceptable only if it contains the full correctness set below. The selected region must be recorded by exact world bounds and source roots in evidence.

Required real-world content coverage:

- outdoor ground and decoration;
- at least one building/interior;
- roof/upper-floor content;
- doorway or equivalent indoor/outdoor boundary;
- at least one floor transition/stairs-like case;
- multi-floor camera visibility;
- ordinary 32x32 objects;
- representative 32x64 / 64x32 / 64x64 visual coverage where available;
- displacement/overhang cases;
- a non-trivial static stack on a tile;
- animated object appearances where available;
- real creature/outfit presentation data.

The slice must use a visible/near-visible working set. It must **not** upload the entire FullWorld corpus to the GPU.

## 6. Creature population

The visual slice should expose enough creature variety to judge readability and animation without pretending to run server AI.

Target:

- at least 5 distinct real creature/outfit presentation templates;
- preferably 10 when available in the selected region/source;
- a normal scene with roughly 20 active visible or near-visible creatures;
- a stress scene with up to roughly 50 presentation instances when useful for measurement.

Source rules:

- real spawn records may seed names, appearances and initial positions;
- real positions remain labelled real only while unchanged;
- cloned/amplified instances are `SYNTHETIC_DYNAMIC` workload;
- movement and facing changes are presentation fixtures until connected to an authoritative server snapshot/event stream.

Required creature presentation behavior:

- idle animation;
- walking animation;
- four-direction presentation where the source supports it;
- smooth visual interpolation independent of gameplay authority;
- correct tile anchoring, displacement and floor ordering;
- name and HP overlay anchoring;
- no gameplay collision/pathfinding inference from appearance size.

## 7. Dynamic VFX package

The minimum inspectable VFX set is:

1. projectile from source to target;
2. hit effect;
3. heal effect;
4. area/AoE spell;
5. boss telegraph with clearly readable danger region;
6. particle emission;
7. trail and/or decal;
8. local dynamic light.

The slice may use real source sprite shapes locally where rights and source contracts permit, but semantic event meaning and timing are Oteryn-owned.

Critical combat information must remain readable under load. Cosmetic VFX may degrade before critical hit/telegraph/readability signals.

## 8. Environment package

The scene must support deterministic toggles for:

- day/night progression;
- rain;
- snow;
- fog;
- wind response where implemented;
- winter/seasonal presentation state;
- local/interior light behavior.

Indoor/roofed areas must prove suppression or transformation of environment effects where the explicit semantic fixture says they should be suppressed. Do not derive this rule from pixel colour or sprite shape.

## 9. Camera and inspection controls

The slice needs a simple engineering interaction shell, not a final UI.

Required controls:

- pan/move camera;
- zoom in/out over a bounded useful range;
- switch or follow floor visibility state;
- enter/leave roofed/interior fixture;
- pause/resume animation;
- toggle environment states;
- switch Classic / Enhanced / HD;
- trigger representative VFX;
- select BASIC / NORMAL / STRESS workload.

The camera must demonstrate that floor projection, roof occlusion and sprite overhang remain stable while moving, not only in static screenshots.

## 10. Presentation profiles

### Classic

Purpose: closest restrained presentation to the Tibia-like readability baseline.

May reduce cosmetic particles/lights/filtering complexity but must preserve all gameplay-readable events.

### Enhanced

Purpose: default modern Oteryn presentation target for this slice.

May use richer lighting, particles, weather and interpolation.

### HD

Purpose: highest presentation-quality fixture that stresses the same gameplay scene.

May increase visual quality/cosmetic density but must not change gameplay semantics.

### Mandatory equality gate

For an identical scripted scenario, all three profiles must produce the same gameplay/presentation-event semantic signature for:

- entity positions at authoritative snapshot boundaries;
- movement start/end semantics;
- hit/heal/AoE event identity and timing;
- projectile logical source/target;
- critical telegraph timing;
- world/floor selection.

Only presentation implementation may differ.

## 11. Debug overlay

The engineering overlay must make the slice diagnosable without external guesswork.

Required live/debug views:

- tile/floor coordinates;
- stack class/order;
- appearance source ID;
- sprite source ID / pixel content ID when available;
- active animation phase;
- resource page/slot or equivalent residency identifier;
- cache hit/miss/eviction counters;
- active/required pages;
- upload count and bytes;
- batch/draw-call count;
- visible primitive count;
- active creature count;
- active critical/cosmetic VFX counts;
- CPU frame time;
- GPU frame time where timestamp support is reliable;
- FPS/frame pacing summary;
- RAM and available GPU-memory evidence where the platform exposes reliable measurements;
- floor-transition stall marker;
- first-frame and prewarm timing.

The overlay is an engineering surface only and creates no final UI architecture commitment.

## 12. Data flow

The intended flow is:

```text
pinned Game/Atlas publication
        |
        +--> REAL_ATLAS semantic slice -----------+
        |                                         |
        +--> optional local pixel publication ----+--> slice presentation adapter
                                                  |
explicit semantic fixtures -----------------------+
                                                  |
SYNTHETIC_DYNAMIC scripted events ----------------+
                                                  v
                                      RenderSnapshot
                                      PresentationEvents
                                      EnvironmentState
                                                  |
                                                  v
                                         custom Rust + wgpu
                                                  |
                                                  +--> interactive scene
                                                  +--> debug overlay
                                                  +--> reproducible evidence
```

The adapter may translate source records into renderer-facing structs. It must not become a second world/gameplay model.

## 13. Implementation sequence and evidence gates

### VWS-0 — input provenance

Prove before rendering:

- protected Game source revision;
- pinned Atlas publication/semantic/pixel roots used;
- exact world bounds/floors;
- real tile/presentation/creature counts;
- whether real pixel bytes are present and authorized locally;
- no proprietary pixel/source archive is staged for commit.

Gate: `INPUT_PROVEN`.

### VWS-1 — real static world

Render the selected real scene with:

- floor projection;
- static presentation order;
- displacement/overhang;
- representative large-cell appearances;
- animation programs already supported by source contracts;
- bounded working-set residency.

Gate: visual correctness fixtures pass and source identity remains traceable.

### VWS-2 — creature presentation and movement

Add real creature/outfit presentation plus synthetic deterministic movement/interpolation.

Gate: anchoring, direction, animation, overlays and semantic movement signature pass.

### VWS-3 — combat/VFX

Add the minimum VFX package and critical-vs-cosmetic classification.

Gate: critical telegraphs/hits remain readable in NORMAL and STRESS fixtures; server authority is not duplicated.

### VWS-4 — environment and roof/floor behavior

Add day/night/weather/season fixtures and explicit roof/interior rules.

Gate: indoor/outdoor, roof, doorway and floor-transition scenarios are reproducible and do not infer semantics from pixels.

### VWS-5 — profiles and debug surface

Enable Classic / Enhanced / HD and live instrumentation.

Gate: semantic signature equality across profiles; required counters visible and exportable.

### VWS-6 — physical qualification

Run the exact pushed candidate on supported Windows hardware, including the RX 9070 XT path where available.

Gate: raw evidence, hardware identity, code/source roots and analyzer summary are reproducible and bound together.

## 14. Physical benchmark matrix

At minimum measure:

- BASIC / NORMAL / STRESS;
- Classic / Enhanced / HD comparison at a fixed representative workload;
- warm and cold/first-frame behavior where meaningful;
- camera motion through the real scene;
- roof/interior transition;
- floor transition;
- cache churn across the selected region;
- VFX burst;
- creature movement load.

Record:

- CPU p50/p95/p99 frame time;
- GPU p50/p95/p99 where reliable;
- FPS/frame pacing;
- peak process RAM;
- reliable GPU/VRAM measurements when available;
- visible primitive count;
- batch/draw count;
- cache hits/misses/evictions;
- uploads and uploaded bytes;
- overflow/fallback count;
- floor-transition max/p95 stall;
- first-frame/prewarm timing;
- device/surface loss or unexplained renderer errors.

Do **not** convert one machine's measurements into final production budgets unless a separate decision gate explicitly does so.

## 15. Visual acceptance evidence

Performance alone cannot complete this slice.

Evidence must include inspectable capture(s) for at least:

- outdoor real world;
- indoor/roof transition;
- multi-floor case;
- moving creatures;
- projectile + hit;
- AoE/boss telegraph;
- rain or snow;
- night/local light;
- Classic / Enhanced / HD comparison from the same camera/world state;
- debug overlay showing source IDs and performance counters.

Captures using locally authorized proprietary source pixels must be handled under the project's rights/evidence policy and must not silently widen redistribution authority.

## 16. Completion criteria

The Visual World Slice is terminal only when all are true:

- the selected region is sourced from pinned real Game/Atlas world data;
- real appearance/creature presentation is used where available;
- any real source pixels are local-only and provenance-bound;
- static floor/stack/displacement/overhang behavior is visually correct;
- explicit roof/door/floor-transition fixtures pass;
- creature idle/walk/interpolation is inspectable;
- projectile/hit/heal/AoE/boss-telegraph/particle/light/decal behavior is inspectable;
- day/night and weather/season state is inspectable;
- Classic / Enhanced / HD semantic signatures are equal;
- debug overlay exposes the required instrumentation;
- BASIC/NORMAL/STRESS physical runs complete without unexplained crash/device loss;
- cache churn is exercised without hidden overflow/fallback in the accepted run, unless an overflow is itself the recorded blocking finding;
- raw evidence is bound to exact code, world roots and hardware;
- every technology-sensitive conclusion is labelled `ADOPT`, `REJECT` or `INSUFFICIENT_EVIDENCE` with its evidence boundary.

## 17. Decision timing

### Keep custom Rust + `wgpu`

- Must decide now? **Already decided for the current foundation.**
- Reopen only on material product/performance/correctness evidence that the foundation cannot satisfy.

### Bounded visible-working-set residency + eviction

- Must decide now? **YES at the invariant level.**
- Reason: the real FullWorld corpus cannot be treated as one GPU-resident asset set.
- Exact cache size/algorithm remains benchmark-sensitive and open.

### Atlas vs texture arrays vs hybrid

- Must decide now? **NO.**
- The slice must preserve comparable resource abstractions and collect evidence.

### KTX2 vs DDS / compression / mip/filter policy

- Must decide now? **NO.**
- Use a fixture sufficient to render the slice; preserve replacement boundary.

### Final particle/light/VFX budgets

- Must decide now? **NO.**
- Measure real scenes first.

### Final UI

- Must decide now? **NO.**
- The debug/interaction shell is intentionally disposable.

## 18. Relationship to Reference and Evolved work

This slice validates rendering/presentation capability; it must not blur the product-profile order:

1. Global Tibia Reference evidence remains the first gameplay/reference truth source where programme #486 requires it.
2. Oteryn Reference uses that evidence under accepted Oteryn contracts.
3. Oteryn Evolved presentation/product differences come only after the Reference boundary is explicit.

A richer Enhanced/HD effect is not permission to introduce Evolved gameplay semantics into Reference.

## 19. Authority boundary

This document does not itself allocate a mutating worker and does not authorize production activation.

A future implementation allocation must name its writable paths and may reuse the existing non-production prototype when that is the safest path. Any change to production client/runtime paths requires its own live allocation and applicable repository review/CI authority.

Atlas is a read-only derived source for this slice. The slice must not write back to Atlas or make Atlas authoritative for Game semantics.

## 20. Short operational definition

`OTERYN VISUAL WORLD SLICE` means:

> one real, pinned Game/Atlas world region rendered by the custom Rust + `wgpu` foundation, with real appearances/creature presentation, locally authorized real pixels when available, deterministic Oteryn-owned movement/VFX/environment fixtures, interactive camera/floor/roof inspection, Classic/Enhanced/HD parity, debug instrumentation and physical evidence strong enough to guide the next renderer decisions.
