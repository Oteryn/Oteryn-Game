# Tibia 15.x -> Oteryn rendering architecture audit

- Date: 2026-09-09
- Status: **EVIDENCE-BACKED DESIGN AUDIT / REVIEW REQUIRED**
- Scope: Tibia-like world rendering semantics, appearances, animation, effects, missiles/projectiles, lighting, day/night, weather, seasons, VFX, batching and asset/runtime boundaries
- Protected Game base at branch creation: `main@9afb7cbb538674408bc7d2eaaaaa1e8917b04640`
- Related Oteryn evidence: Issue #465 / PR #468 graphics-engine bake-off; PR #473 presentation/VFX architecture baseline
- Runtime implementation authority: **NONE**
- Product/live activation authority: **NONE**
- Server/protocol/content mutation authority: **NONE**

## 1. Purpose

This audit answers one bounded question:

> Which rendering concepts from contemporary Tibia and the Tibia-compatible client ecosystem should Oteryn preserve, improve, replace, add or deliberately defer while building its project-owned Rust/`wgpu` native client?

The goal is **not** to clone CipSoft's closed renderer. The goal is to preserve the rendering semantics that make a Tibia-like world readable and recognisable while replacing legacy physical constraints with a modern Oteryn presentation architecture.

This document should be read with:

- `GRAPHICS_APPEARANCE_ANIMATION_AND_SEASONALITY_HORIZON_NOTE.md`;
- PR #468 while its custom-`wgpu` versus Bevy result remains candidate evidence until protected integration;
- PR #473 while its presentation/VFX baseline remains candidate architecture until protected integration.

This audit does not promote either open PR to protected authority by prose alone.

## 2. Evidence boundary

The original CipSoft renderer is closed-source. Therefore this audit distinguishes observable/public evidence from inference.

Evidence anchors used here:

1. Official Tibia client announcements/documentation for supported graphics backends and day/night behaviour.
2. `opentibiabr/canary@d34733e1336f0e4f396b45b4bfb93b681407d0bb`, especially `src/protobuf/appearances.proto` (blob `f39de45976e372b2306f6ff9c4a40412f4debe4e`).
3. `opentibiabr/otclient@dda0520ac61d1626f7d3a6d255394937038ec8ea`, including protocol code separation and rendering implementation as Tibia-compatible engineering reference.
4. Existing Oteryn graphics horizon notes and the physical #465/#468 benchmark.

Open-source Canary/OTClient code is **reference evidence**, not proof that CipSoft implements identical internal classes, GPU buffers, render passes, batching, texture packing or caches.

Do not claim as facts without new evidence:

- CipSoft's exact GPU buffer topology;
- exact render-pass graph;
- exact batching algorithm;
- exact runtime texture atlas/array representation;
- exact shader architecture;
- exact VRAM cache/streaming topology;
- exact physical asset repacking performed after client startup.

## 3. Executive disposition matrix

| Area | Observable Tibia/Tibia-compatible model | Oteryn target | Disposition |
|---|---|---|---|
| Logical world | floor/tile based | floor/tile based | `COPY` |
| Tile stack semantics | specialised ordering/flags | project-owned semantic resolver | `COPY + IMPROVE` |
| Appearance families | object/outfit/effect/missile | semantic equivalents | `COPY` |
| Appearance identity | numeric appearance/sprite references | stable semantic presentation identity | `IMPROVE` |
| Creature animation | frame groups/phases/patterns | semantic animation -> backend | `IMPROVE` |
| Spell effect | effect appearance/event | composable `PresentationRecipe` | `IMPROVE` |
| Missile | distinct event/presentation object | `ProjectileEvent` + interpolation/VFX | `COPY + IMPROVE` |
| Hit/damage authority | server/game state | server/game state | `COPY` |
| World light | ambient/world light | environment + lighting state | `COPY + IMPROVE` |
| Creature/object light | appearance/entity light | typed local light sources | `COPY + IMPROVE` |
| Day/night | world shading; selected gameplay usage | split presentation/gameplay semantics | `COPY + IMPROVE` |
| Weather | no public universal CipSoft contract established | generic Environment/VFX subsystem | `ADD` |
| Seasons | no public universal appearance resolver established | seasonal presentation resolver | `ADD` |
| Generic particles | no public universal CipSoft contract established | renderer-owned batched particles | `ADD` |
| Shader/material VFX | CipSoft internals unknown | project-owned bounded effects | `ADD` |
| BMP/LZMA sprite-sheet ecosystem | legacy/current Tibia asset transport/reference | migration/reference only | `REPLACE` |
| Fixed 32px physical assumption | legacy sprite-grid heritage | logical world independent of density | `REPLACE` |
| Atlas vs texture array | CipSoft runtime unknown | representative benchmark | `DEFER` |
| KTX2 vs DDS | not inherited from Tibia | representative benchmark | `DEFER` |
| Asset streaming/cache | CipSoft internals unknown | evidence-based Oteryn design | `DEFER` |

## 4. Backend independence: preserve the principle

Modern Tibia exposes multiple graphics backend choices across platforms/hardware, including DirectX, Vulkan/OpenGL and Metal paths. The exact internal CipSoft abstraction is not public, but the product behaviour demonstrates that presentation/game semantics are not tied to one public GPU API.

Oteryn should preserve that property through `wgpu`:

```text
Oteryn presentation
        |
        v
       wgpu
        |
        +--> DX12
        +--> Vulkan
        +--> Metal
        +--> other supported wgpu backend
```

Disposition:

- `COPY`: graphics-backend independence;
- `IMPROVE`: use `wgpu` rather than maintaining separate Oteryn renderer implementations for every native GPU API.

The #465/#468 physical bake-off is the current evidence-backed renderer-foundation candidate for this direction, not a decision about every presentation subsystem.

## 5. Core Tibia identity: floor -> tile -> semantic stack

A Tibia-like world is not a generic collection of sprites sorted only by one arbitrary numeric `z` value.

The rendering model must understand at least:

```text
World
  -> Floor
      -> Tile
          -> semantic stack/order
```

Current appearance metadata exposes rendering-relevant concepts including `clip`, `bottom`, `top`, `light`, `translucent`, `shift`, `height/elevation`, `lying_object` and `topeffect`.

This is strong evidence that presentation order is semantically richer than `object.z` alone.

### Oteryn requirement

Introduce a dedicated semantic resolver, conceptually:

```text
TileStackResolver
```

It must be capable of resolving:

- ground;
- ground borders/overlays;
- bottom-order objects;
- regular items/world objects;
- lying objects;
- displacement/elevation;
- creatures/players;
- gameplay world effects;
- top effects;
- roofs/walls;
- floor occlusion;
- above/below-floor visibility;
- projectile integration where applicable.

Do not reduce the production architecture to:

```rust
things.sort_by_key(|thing| thing.z);
```

The exact final algorithm is implementation evidence-gated, but semantic stack resolution is **P0** for a credible Oteryn world renderer.

Disposition: `COPY + IMPROVE`.

## 6. Appearance family separation: keep it

The current appearance schema explicitly separates:

```text
object
outfit
effect
missile
```

This separation is valuable and should survive conceptually in Oteryn.

Candidate Oteryn concepts:

```text
WorldObjectAppearance
OutfitAppearance
EffectAppearance
ProjectileAppearance
```

Exact Rust type names remain deferred.

Disposition: `COPY`.

## 7. Appearance composition: preserve semantics, modernise identity

Current appearance metadata contains:

- `Appearance.id`;
- frame groups;
- `pattern_width`;
- `pattern_height`;
- `pattern_depth`;
- layers;
- sprite IDs;
- animation phases;
- opacity/bounding information.

Animation metadata contains concepts including:

- default start phase;
- synchronized animation;
- random start phase;
- ping-pong/infinite/counted loops;
- min/max phase durations.

Those are useful logical presentation concepts.

Oteryn should resolve:

```text
AppearanceKey
    -> PresentationDefinition
        -> SemanticAnimationState
            -> PresentationFamily
                -> Quality/DensityVariant
                    -> compiled runtime resource
```

Example:

```text
Demon
  |- idle
  |   |- north
  |   |- east
  |   |- south
  |   `- west
  |- walk
  |- attack
  |- cast
  |- hit/react
  `- death
```

The physical resource can then vary independently:

```text
Classic   -> lower-density/frame-oriented presentation
Enhanced  -> richer frame/light/VFX composition
HD        -> higher-density presentation resources
```

Disposition: `IMPROVE`.

## 8. Never make physical pixel density world semantics

A logical tile/grid coordinate is not a mandatory physical texture size.

Required invariant:

```text
logical tile/world unit != physical source texture pixel size
```

A creature or item may preserve the same authoritative footprint while resolving to different presentation density:

```text
Classic   -> 32-class source density where appropriate
Enhanced  -> 64-class source density where appropriate
HD        -> 128-class or higher where appropriate
```

Changing presentation density must not require changing:

- map coordinates;
- collision;
- pathing;
- interaction range;
- authoritative footprint;
- protocol position;
- combat timing.

Disposition: `REPLACE` legacy physical assumption.

## 9. Direction/pattern semantics

Direction and presentation pattern are presentation state, not separate gameplay identities.

Oteryn should resolve:

```text
semantic direction/state
       -> appearance resolver
           -> physical frame/resource selection
```

Do not create independent authoritative identities such as `DemonNorth`, `DemonEast`, etc.

Disposition: `COPY + IMPROVE`.

## 10. Spell presentation: preserve authority split, expand composition

The Tibia-compatible protocol model already has the correct fundamental separation:

```text
server/game event
    -> effect semantic/ID + position/timing
        -> client presentation
```

The server does not need to drive every visual frame.

Oteryn should preserve that authority boundary while replacing the assumption that one spell effect equals one frame sequence.

### Oteryn target

```text
EffectKey
    -> PresentationRecipe
```

A recipe may compose zero or more:

```text
frame/sprite animation
particle emitter(s)
dynamic light pulse
projectile/trail
temporary decal
material/shader animation
bounded post-process
sound cue
```

Example:

```text
FireExplosion
  |- frame animation
  |- sparks
  |- smoke
  |- short light pulse
  |- optional scorch decal
  `- sound
```

Presentation-family variants can then differ without changing gameplay:

```text
Classic  -> classic frames/minimal extras
Enhanced -> frames + bounded lights/particles
HD       -> high-density art + richer bounded VFX
```

Disposition: `IMPROVE`.

## 11. Damage/hit authority remains server-owned

Hard invariant:

```text
visual frame != damage authority
```

Authoritative game/server state owns:

- hit/miss;
- target;
- damage;
- damage element/type;
- affected area;
- activation timing;
- cooldown;
- resource cost;
- status effects;
- death/loot/gameplay mutation.

Client visual markers may trigger only non-authoritative local presentation such as:

- sound;
- particles;
- flash;
- decal;
- camera feedback.

Disposition: `COPY` server-authority principle.

## 12. Area spells stay tile/gameplay semantic

For an area spell, authoritative gameplay owns the affected area or sufficient deterministic semantics to derive it on the authoritative side.

Presentation may consume that area to draw:

- tile highlights;
- wave geometry;
- synchronized effect sprites;
- particles;
- dynamic light;
- impact effects;
- boss telegraphs.

The client must never infer gameplay hit coverage from the visible dimensions of a sprite.

Disposition: `COPY AUTHORITY + IMPROVE PRESENTATION`.

## 13. Missile/projectile separation is worth preserving

The Tibia-compatible protocol separates graphical effects from missile effects. This is a strong architectural signal: a projectile is not simply another static tile thing.

Oteryn should use a semantic event conceptually containing:

```text
ProjectileEvent
  |- source
  |- destination / trajectory semantics
  |- launch timing
  |- authoritative impact timing where applicable
  `- ProjectileKey / EffectKey
```

Client presentation may derive:

- interpolated position;
- orientation;
- trail;
- particles;
- dynamic light;
- impact VFX.

Hard invariant:

```text
visual projectile position != authoritative collision/hit state
```

Disposition: `COPY + IMPROVE`.

## 14. Hit presentation should be semantic

Presentation should consume semantic damage/heal categories rather than physical sprite IDs.

Candidate categories may include:

```text
physical
fire
ice
energy
earth
holy
death
healing
```

The exact gameplay taxonomy remains owned by gameplay contracts.

Example presentation resolution:

```text
FireHit
  -> impact frames
  -> sparks
  -> small light pulse
```

Do not leak a contract such as `fire_damage -> sprite_id=18273` into gameplay/domain state.

Disposition: `IMPROVE`.

## 15. Boss telegraphs should become a first-class Oteryn primitive

Boss telegraphs are gameplay-critical presentation.

Conceptually:

```text
authoritative area/timing
      -> BossTelegraphEvent
          -> marker
          -> wind-up/pulse
          -> activation
          -> impact
```

Different presentation families may look different, but must preserve equivalent:

- affected-area readability;
- timing readability;
- visibility information.

A Classic player must not receive less gameplay-significant information than an HD player.

Disposition: `ADD`.

## 16. Lighting: copy the semantic model and expand it

The observed Tibia-compatible model separates world ambient lighting from creature/local lighting, and appearance metadata can carry light brightness/color.

Oteryn should generalise this into a typed lighting system:

```text
LightingSystem
  |- AmbientLight
  |- TimeOfDayLight
  |- ObjectLight
  |- CreatureLight
  |- SpellLight
  |- EnvironmentLight
  `- TemporaryLight
```

Examples:

```text
torch -> persistent warm light + optional subtle flicker
fire spell -> short local light pulse
storm -> bounded world flash
```

Lighting must enhance Tibia/Oteryn readability rather than cover it with bloom/flashing.

Disposition: `COPY + IMPROVE`.

## 17. Day/night: split visual and gameplay consequences

Official Tibia behaviour establishes day/night as more than separate painted copies of every asset: environmental shading changes, while selected content can also have gameplay consequences tied to time.

Oteryn should therefore model two distinct concerns.

Presentation:

```text
time_of_day
  -> ambient intensity
  -> colour treatment
  -> optional environment/fog tuning
```

Gameplay consequences, if any:

```text
spawn/NPC/access/combat/visibility/etc.
  -> authoritative server/gameplay contract
```

Do not require `*_day`, `*_evening`, `*_night` artwork for every object. Specific art variants remain allowed where art direction requires them.

Disposition: `COPY + IMPROVE`.

## 18. Weather: add a generic Oteryn environment system

The public appearance schema does not establish a generic universal CipSoft `WeatherState` equivalent. Therefore this audit does not claim one exists internally.

Oteryn should add a presentation-oriented environment state capable of expressing:

```text
weather
intensity
wind
precipitation
fog
region/biome context
optional event overlay
```

Examples:

```text
Rain
  -> precipitation particles
  -> wind response
  -> light/colour treatment
  -> optional wet visual treatment

Snow
  -> particles
  -> wind
  -> fog
  -> season/context interaction
```

Do not represent thousands of decorative precipitation particles as authoritative gameplay entities.

Disposition: `ADD`.

## 19. Seasons: resolve variants, do not clone the whole map

The public evidence does not establish a generic CipSoft runtime resolver of the form `Appearance + Season -> Variant`. Oteryn should add one because it directly serves the desired product direction.

Concept:

```text
AppearanceKey
+ PresentationFamily
+ QualityTier
+ Season
+ Region/Biome
+ EventOverlay
      -> ResolvedPresentation
```

Example:

```text
OakTree
  |- default
  |- autumn optional
  `- winter optional
```

Missing seasonal art must follow a defined fallback path. The world must not require four independent canonical maps.

Visual-only seasons are the preferred first slice. Any movement/combat/resource/access consequence is separate authoritative gameplay work.

Disposition: `ADD`.

## 20. Weather and spell composition

Environment and spell presentation may compose without changing gameplay.

Examples:

```text
FireExplosion + Rain
  -> same damage/range/timing
  -> optional steam
  -> reduced decorative smoke

Lightning + Storm
  -> same gameplay
  -> coordinated temporary lighting

IceEffect + Winter
  -> same gameplay
  -> optional winter presentation variant
```

Presentation composition must never silently modify damage, range, cooldown, resistance, AoE or hit chance.

Disposition: `ADD PRESENTATION-ONLY COMPOSITION`.

## 21. Particles: renderer-owned, bounded and measurable

High-count spell/environment particles should not become one game-domain entity per particle.

Target shape:

```text
ParticleEmitter
     -> renderer/GPU-oriented particle data
         -> batched/instanced presentation
```

Future implementation must measure:

- emitter CPU cost;
- GPU simulation cost if used;
- buffer update cost;
- overdraw/fill-rate;
- sorting cost where needed;
- RAM/VRAM;
- count caps;
- degradation behaviour.

No particular compute-shader design or particle library is accepted by this audit.

Disposition: `ADD`, physical implementation `DEFERRED`.

## 22. Temporary decals

The renderer should support bounded presentation-only marks such as:

- blood;
- scorch;
- frost;
- poison residue.

Conceptually:

```text
TemporaryDecal
  |- semantic appearance
  |- world position
  |- lifetime
  `- fade
```

If such an object affects gameplay, it is no longer a presentation-only decal and needs authoritative game state.

Disposition: `ADD`.

## 23. Animation backend extensibility

Oteryn should not assume every animation is a frame sequence.

The presentation layer should remain capable of resolving semantic animation into techniques such as:

```text
FrameSequence
LayeredFrameSequence
ParticleAnimation
Material/ShaderAnimation
future evidence-backed backend
```

Examples:

```text
Classic creature -> FrameSequence
creature aura -> particles/material
HD water -> frame sequence OR shader/material path
```

No skeletal or shader-heavy implementation is accepted merely by listing it here.

Disposition: `IMPROVE`.

## 24. Batching: preserve the principle, implement it for Oteryn/`wgpu`

Modern Tibia-compatible clients demonstrate useful techniques such as draw pools, texture atlases, state grouping, framebuffers and shaders. They are engineering reference, not proof of identical CipSoft internals.

The transferable principle is:

```text
many semantic presentation objects
          -> render commands
              -> cull/sort/group
                  -> few compatible GPU submissions
```

Gameplay/domain objects must not directly issue GPU draws.

Disposition: `COPY PERFORMANCE PRINCIPLE + REIMPLEMENT`.

## 25. Required render-command boundary

Recommended architecture:

```text
Simulation/client-domain state
        -> RenderSnapshot + PresentationEvents + EnvironmentState
            -> Presentation resolution
                -> RenderCommands
                    -> cull / sort / batch
                        -> wgpu
```

Gameplay/domain state must not retain physical renderer details such as:

- GPU texture handles;
- bind groups;
- pipeline handles;
- atlas UV rectangles;
- texture-array layer numbers;
- GPU buffer offsets.

This is a required Oteryn boundary.

## 26. Conceptual render ordering

The final GPU pass graph is not frozen. The architecture must nevertheless support the semantic equivalent of:

```text
1. visible floors/world
2. ground
3. borders/world overlays
4. bottom-order objects
5. regular items/world objects
6. creatures/players
7. world-attached gameplay VFX
8. projectiles
9. creature-attached VFX
10. environment/ambient particles
11. lighting composition
12. fog/environment composition
13. bounded post-processing
14. names/HP/gameplay overlays
15. UI
```

The actual order must be derived from:

- floor;
- tile;
- stack semantics;
- elevation;
- walls/roofs;
- occlusion;
- appearance semantics;
- gameplay readability.

This list is not a frozen render-pass ABI.

## 27. Classic / Enhanced / HD presentation families

Oteryn should preserve one gameplay model while allowing several art/presentation families.

```text
Gameplay semantics
      -> AppearanceKey / EffectKey
          -> Classic | Enhanced | HD
```

Candidate product intent:

### Classic

- Tibia-like frame presentation;
- minimal decorative particles;
- simpler lighting path.

### Enhanced

- optional higher-density art;
- bounded particles;
- dynamic lighting where useful.

### HD

- higher-density authored art;
- richer but bounded particles/materials/lights.

All supported families must preserve the same gameplay-significant footprint, timing, interaction and visibility information.

## 28. Technical quality must remain separate from art family

Do not conflate:

```text
Classic / Enhanced / HD
```

with:

```text
Low / Medium / High / Ultra / Auto
```

A player should be able to prefer Classic art while retaining high technical quality, or HD art while reducing particles/lights to fit hardware constraints.

This separation improves performance scaling, accessibility and product flexibility.

## 29. VFX priority and graceful degradation

Introduce conceptual priority classes:

```text
CRITICAL_GAMEPLAY_VFX
IMPORTANT_GAMEPLAY_VFX
CHARACTER_VFX
AMBIENT_VFX
DECORATIVE_VFX
```

When frame budget is under pressure, reduce first:

- precipitation density;
- ambient particles;
- decorative trails;
- optional post-processing;
- non-critical dynamic lights.

Preserve:

- boss telegraphs;
- danger fields;
- critical spell markers;
- target/readability feedback.

Exact budgets/hard maxima require measurement.

Disposition: `ADD`.

## 30. Do not copy BMP/LZMA as Oteryn's target runtime pipeline

Tibia client sprite-sheet/appearance resources are useful migration and reference evidence. They are not sufficient reason to preserve a legacy physical asset transport as Oteryn's final runtime architecture.

Target direction:

```text
human-friendly source artwork
        -> Oteryn asset/appearance compiler
            -> validated compiled presentation resources
                -> runtime resource organisation
                    -> wgpu upload/cache
```

Legacy Tibia formats may remain import/migration inputs.

Disposition: `REPLACE` physical pipeline.

## 31. Atlas vs texture array remains unresolved

A sprite sheet on disk does not prove that the correct Oteryn GPU representation is one giant atlas.

Both remain candidates:

```text
Atlas
  -> useful for irregular/grouped resources

TextureArray
  -> attractive for compatible fixed-shape content
```

A possible future organisation could use density/category-specific arrays such as:

```text
creatures_32[]
creatures_64[]
creatures_128[]
items_32[]
effects_32[]
effects_64[]
...
```

but no thresholds are accepted here.

Disposition: `DEFER` until real world/VFX benchmark.

## 32. KTX2 vs DDS remains unresolved

Atlas/array and KTX2/DDS answer different questions:

```text
atlas / texture array
  = runtime resource organisation

KTX2 / DDS
  = storage/container representation
```

Final selection requires evidence for:

- startup/load latency;
- CPU decode/transcode cost;
- GPU upload cost;
- RAM/VRAM;
- patch/download size;
- BCn/ASTC/etc. target support;
- cross-platform behaviour;
- Rust tooling maturity;
- corruption/oversize validation;
- operational complexity.

Disposition: `DEFER`.

## 33. Required next graphics evidence gate

The next graphics task should be a production-shaped:

```text
OTERYN WORLD + VFX PROTOTYPE
```

not another broad engine-selection study.

### Mandatory world slices

- real floor handling;
- tile visibility;
- stack ordering;
- borders;
- items;
- elevation;
- walls;
- roofs;
- occlusion.

### Mandatory creature slices

- real appearance-shaped metadata;
- directions;
- idle/walk animation;
- movement interpolation;
- outfit composition where relevant.

### Mandatory combat/VFX slices

- area spell;
- projectile/missile;
- physical hit;
- elemental hit;
- heal;
- death presentation;
- boss telegraph;
- frame effect;
- particle effect;
- trail;
- temporary light;
- decal.

### Mandatory environment slices

- ambient light;
- day/night transition;
- rain;
- snow;
- fog;
- wind parameter;
- at least one winter appearance variant with fallback.

### Mandatory camera/UI slices

- continuous camera movement;
- scroll;
- zoom;
- floor transition;
- visibility-set churn;
- creature names;
- HP bars;
- basic combat overlays;
- representative UI draw load.

### Mandatory asset-pressure slices

- multiple resource pages/sheets;
- ongoing uploads;
- cache pressure;
- missing-variant fallback;
- corrupted/oversized resource rejection where applicable.

## 34. Required measurements for that prototype

Measure at minimum:

```text
CPU frame p50/p95/p99
GPU frame p50/p95/p99 where trustworthy
frame pacing
render submissions/draw calls
batch sizes
visible quads
RAM
VRAM where trustworthy
texture upload latency
visible-set churn cost
cache hit/miss behaviour
asset decode/load latency
startup
camera-scroll stalls
particle counts
light counts
overdraw/fill-rate where measurable
```

Run representative Classic/Enhanced/HD source-density scenarios and multiple workload tiers.

## 35. Decisions allowed only after the prototype

Only representative evidence should freeze:

- atlas vs texture arrays;
- array segmentation;
- KTX2 vs DDS/other container;
- GPU compression policy;
- mipmap/filtering policy;
- asset streaming/cache architecture;
- particle implementation;
- dynamic light caps;
- RAM/VRAM budgets;
- batching thresholds;
- final render-pass decomposition.

## 36. Final architecture direction

The intended direction is:

```text
                 TIBIA SEMANTICS
                       |
       +---------------+----------------+
       |               |                |
    tile stack     appearances     effects/missiles
       |               |                |
       +---------------+----------------+
                       |
                       v
             OTERYN PRESENTATION
                       |
        +--------------+--------------+
        |              |              |
      World            VFX        Environment
        |              |              |
  floors/items      spells        day/night
  creatures         projectiles   weather
  walls/roofs       particles     seasons
  occlusion         lights        fog/wind
                    decals
        |              |              |
        +--------------+--------------+
                       |
                       v
                 Render Commands
                       |
                 cull/sort/batch
                       |
                       v
                      wgpu
                       |
              DX12 / Vulkan / Metal
```

The product goal is:

> Preserve the rendering semantics that make Tibia recognisable while replacing legacy physical constraints with a specialised modern Oteryn renderer.

## 37. Final dispositions

### `COPY`

- floor/tile world model;
- semantic tile-stack concept;
- separate object/outfit/effect/missile concepts;
- server authority for combat;
- client-side visual animation;
- semantic world/creature/object lighting;
- missile/projectile as a distinct presentation concept.

### `IMPROVE`

- appearance identity;
- animation state resolution;
- effect composition;
- projectile presentation;
- lighting;
- day/night;
- batching;
- resource resolution;
- multiple presentation families.

### `REPLACE`

- fixed 32px-as-world-semantics assumptions;
- physical sprite IDs/paths/UVs as gameplay architecture;
- BMP/LZMA as target runtime format;
- direct gameplay dependence on GPU/resource representation.

### `ADD`

- `PresentationRecipe` / composable VFX;
- renderer/GPU-oriented particles;
- temporary presentation decals;
- first-class boss telegraphs;
- generic environment state;
- weather system;
- season resolver;
- Classic/Enhanced/HD families;
- VFX priority/degradation.

### `DEFER`

- KTX2 vs DDS;
- texture atlas vs texture arrays;
- final GPU compression;
- final streaming/cache topology;
- final UI technology;
- particle compute architecture;
- exact renderer pass graph;
- exact performance budgets/hard maxima.

## 38. Architecture checkpoint

Until the next production-shaped renderer evidence exists:

```text
Renderer foundation candidate:
  project-owned Rust + wgpu, per #465/#468 evidence

Tibia rendering semantics:
  preserve where they define world readability/identity

Modern Oteryn VFX/environment capabilities:
  add behind semantic presentation boundaries

Permanent physical texture/runtime format:
  DEFERRED
```

This audit grants no runtime, protocol, deployment or live activation authority.