# Oteryn Visual World Slice — launch line

This is a programme launch line, not a lifecycle-registered reusable prompt and not independent implementation authority.

Short invocation:

```text
Oteryn: visual world slice
```

## Objective

Execute the next bounded graphics proof defined by:

`docs/architecture/OTERYN_VISUAL_WORLD_SLICE_ARCHITECTURE_AND_EVIDENCE_GATE_2026-09-09.md`

The result must be an inspectable, game-shaped scene using the protected custom Rust + `wgpu` foundation and pinned real Game/Atlas world data rather than another procedural-only benchmark.

## Startup

Refresh from GitHub before mutation:

- current protected `main`;
- merged graphics lineage #468, #473, #475, #480, #489 and #505;
- current programme #486 / R9 Client-World-Presentation-VFX dependencies and ownership;
- active Issues/PRs/branches overlapping the intended writable paths.

The launch line itself grants no path ownership. Consume the live Issue/task allocation before tracked-file mutation.

## Required slice

Implement the architecture gate with at least:

- one pinned real Atlas FullWorld region, normally 96x96 with three adjacent floors when the selected region supports the correctness fixtures;
- real tile/static-stack/appearance/displacement/overhang presentation data;
- real creature/outfit presentation and real spawn metadata where available;
- locally authorized real pixel bytes when available, never committed as proprietary/raw assets;
- deterministic Oteryn-owned movement/interpolation and combat/environment presentation where Atlas intentionally has no live authority;
- explicit semantic fixtures for roof, doorway, stairs/floor transition, lower-floor visibility and overhang-vs-gameplay-footprint cases;
- creature idle/walk/direction/name/HP presentation;
- projectile, hit, heal, AoE, boss telegraph, particles, trail/decal and local light;
- day/night, rain, snow, fog and winter/seasonal presentation fixtures;
- camera pan/zoom/floor/roof inspection and deterministic VFX/environment controls;
- Classic / Enhanced / HD switching with identical gameplay/presentation-event semantics;
- live debug instrumentation for stack/source IDs, animation, cache, uploads, batches, primitive/creature/VFX counts, CPU/GPU timing, FPS, memory evidence and floor-transition stalls;
- BASIC / NORMAL / STRESS physical qualification and inspectable visual captures.

## Data boundary

Keep each input visibly classified as one of:

- `REAL_ATLAS`;
- `LOCAL_PRIVATE_PIXEL_BYTES`;
- `SYNTHETIC_DYNAMIC`;
- `EXPLICIT_SEMANTIC_FIXTURE`.

Do not describe synthetic creature movement, combat, AI, occupancy, cloned stress instances or presentation events as real Atlas live state.

Do not infer collision, walkability, roof rules or gameplay footprint from pixels or appearance dimensions.

## Preserve

Do not reopen or bypass:

- custom Rust + `wgpu` current foundation;
- Tibia-like floor/tile/stack readability model;
- server authority for gameplay state, damage, hit/AoE timing and creature movement;
- `RenderSnapshot + PresentationEvents + EnvironmentState` renderer boundary;
- composable VFX;
- Classic / Enhanced / HD gameplay-semantic equality.

## Keep open

Do not freeze without new qualifying evidence:

- KTX2 vs DDS;
- atlas vs texture arrays vs hybrid;
- final streaming/cache/bundle format;
- final UI;
- compression/mipmap/filtering policy;
- particle backend;
- final RAM/VRAM, light/VFX and batching budgets.

Any temporary choice used only to make the slice executable must be labelled `SLICE_FIXTURE`.

## Acceptance

The work is terminal only when the architecture document's VWS-0 through VWS-6 gates are satisfied and evidence is bound to exact code, exact Game/Atlas roots and exact hardware.

At minimum prove:

- real world provenance and no committed proprietary source pixels;
- visual correctness for multi-floor, roof/interior, doorway, floor transition, displacement and overhang;
- inspectable creature movement and the minimum VFX/environment package;
- Classic / Enhanced / HD semantic-signature equality;
- required live debug counters;
- physical BASIC/NORMAL/STRESS completion without unexplained device/surface loss;
- cache churn is actually exercised;
- every technology-sensitive conclusion is explicitly `ADOPT`, `REJECT` or `INSUFFICIENT_EVIDENCE`.

Do not infer production frame/RAM/VRAM budgets from one workstation unless a later accepted decision gate authorizes that conclusion.

## Authority boundary

If no live mutating allocation exists, produce a readiness packet and stop before tracked runtime mutation rather than creating a second control plane.

Do not write to Atlas. Do not activate production runtime or deploy anything under this launch line alone.
