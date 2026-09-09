# OTV2 Visual World Slice

Short invocation:

```text
Oteryn: visual world slice
```

## Outcome

Deliver the next bounded graphics proof defined by:

`docs/architecture/OTERYN_VISUAL_WORLD_SLICE_ARCHITECTURE_AND_EVIDENCE_GATE_2026-09-09.md`

The result must be an inspectable, game-shaped visual slice using the protected custom Rust + `wgpu` foundation and a pinned real Game/Atlas world region, not another procedural-only benchmark.

## Live locators

Refresh from GitHub before mutation:

- current protected `main`;
- merged graphics foundation/evidence lineage #468, #473, #475, #480, #489 and #505;
- current programme #486 / R9 Client-World-Presentation-VFX dependencies and ownership;
- active Issues/PRs/branches that overlap the candidate writable paths.

Historical SHAs in evidence are provenance only; live GitHub state is lifecycle authority.

## Required slice

Implement the architecture gate with these minimum properties:

- one pinned real Atlas FullWorld region, normally 96x96 with three adjacent floors when the selected region supports the required fixtures;
- real tile/static-stack/appearance/displacement/overhang presentation data;
- real creature/outfit presentation data and real spawn metadata where available;
- locally authorized real pixel bytes when available, never committed as proprietary/raw assets;
- deterministic Oteryn-owned movement/interpolation and combat/environment presentation where Atlas intentionally has no live authority;
- explicit semantic fixtures for roof, doorway, stairs/floor transition, lower-floor visibility and overhang-vs-gameplay-footprint cases;
- creature idle/walk/direction/overlay presentation;
- projectile, hit, heal, AoE, boss telegraph, particles, trail/decal and local light;
- day/night, rain, snow, fog and winter/seasonal presentation fixtures;
- camera pan/zoom/floor/roof inspection and deterministic VFX/environment controls;
- Classic / Enhanced / HD switching with identical gameplay/presentation-event semantics;
- live debug instrumentation for stack/source IDs, animation, cache, uploads, batches, primitive/creature/VFX counts, CPU/GPU timing, FPS, memory evidence and floor-transition stalls;
- BASIC / NORMAL / STRESS physical qualification and inspectable visual captures.

## Data boundary

Keep every input visibly classified as one of:

- `REAL_ATLAS`;
- `LOCAL_PRIVATE_PIXEL_BYTES`;
- `SYNTHETIC_DYNAMIC`;
- `EXPLICIT_SEMANTIC_FIXTURE`.

Do not describe synthetic creature movement, combat, AI, occupancy, cloned stress instances or presentation events as real Atlas live state.

Do not infer collision, walkability, roof rules or gameplay footprint from pixels or appearance dimensions.

## Preserved architecture

Do not reopen or bypass:

- custom Rust + `wgpu` current foundation;
- Tibia-like floor/tile/stack readability model;
- server authority for gameplay state, damage, hit/AoE timing and creature movement;
- `RenderSnapshot + PresentationEvents + EnvironmentState` renderer boundary;
- composable VFX;
- Classic / Enhanced / HD gameplay-semantic equality.

## Decisions that remain open

Do not freeze without new qualifying evidence:

- KTX2 vs DDS;
- atlas vs texture arrays vs hybrid;
- final streaming/cache/bundle format;
- final UI;
- compression/mipmap/filtering policy;
- particle backend;
- final RAM/VRAM, light/VFX and batching budgets.

A temporary choice required by the slice must be labelled `SLICE_FIXTURE`.

## Acceptance delta

The allocation is complete only when the architecture document's VWS-0 through VWS-6 gates are satisfied and evidence is bound to exact code, exact Game/Atlas roots and exact hardware.

At minimum prove:

- real world provenance and no committed proprietary source pixels;
- visual correctness for multi-floor, roof/interior, doorway, floor transition, displacement and overhang;
- inspectable creature movement and the minimum VFX/environment package;
- Classic / Enhanced / HD semantic signature equality;
- required live debug counters;
- physical BASIC/NORMAL/STRESS completion without unexplained device/surface loss;
- cache churn is actually exercised;
- every technology-sensitive conclusion is explicitly `ADOPT`, `REJECT` or `INSUFFICIENT_EVIDENCE`.

Do not infer production frame/RAM/VRAM budgets from one workstation unless a later accepted decision gate authorizes that conclusion.

## Authority delta

This reusable prompt is discovery, not implementation authority.

Before writing code, consume the current live Issue/task allocation and obey its owned paths. If no mutating allocation exists, the legal result is a precise readiness packet rather than creating a second control plane or silently claiming production-client authority.

Do not write to Atlas. Do not activate production runtime or deploy anything under this prompt alone.
