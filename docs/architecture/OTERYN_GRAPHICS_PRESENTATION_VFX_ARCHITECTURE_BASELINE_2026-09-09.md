# Oteryn graphics presentation and VFX architecture baseline

- Date: 2026-09-09
- Scope: native-client presentation architecture, world rendering, spell/VFX composition, projectiles, hit feedback, environment, day/night, weather, seasons, lighting, quality tiers and presentation fallback
- Source: owner-directed design discussion plus physical graphics-engine bake-off evidence from Issue #465 / PR #468
- Protected source base at branch creation: `main@ae22132fd9ddb6c83f5bf386fdc267cb670d16aa`
- Status: **CANDIDATE ARCHITECTURE BASELINE / REVIEW REQUIRED**
- Runtime/product activation authority: **NONE**
- Server/protocol/content/production mutation authority: **NONE**

## 1. Purpose

Record the architecture direction needed to continue native-client graphics work without prematurely freezing physical texture/container/storage choices.

This baseline is intended to prevent expensive future rework by fixing semantic boundaries now while leaving implementation technology choices evidence-gated.

The document must be read together with:

- `GRAPHICS_APPEARANCE_ANIMATION_AND_SEASONALITY_HORIZON_NOTE.md`;
- Issue #465 `Graphics engine bake-off: custom wgpu vs Bevy`;
- PR #468 `test(graphics): record wgpu vs Bevy engine bake-off` while that PR remains unprotected candidate evidence.

PR #468 is evidence, not protected architecture authority, until integrated to protected `main`.

## 2. Evidence status

### PROVEN by the physical bake-off evidence

The #465/#468 experiment compared the same deterministic benchmark-only `RenderSnapshot` workload through:

1. a project-owned Rust renderer using direct `wgpu`; and
2. a minimal Bevy 0.19.1 presentation-only challenger.

On Molehill-PC / Windows / DX12 / Radeon RX 9070 XT the custom `wgpu` candidate won all nine measured sprite-throughput cells and used materially less process memory, built substantially faster and produced a substantially smaller executable.

The measured bounded verdict recorded by the experiment is:

```text
ADOPT_CUSTOM_WGPU
```

This verdict applies only to the **renderer foundation for the next native-client rendering slices**.

It does not prove a final world renderer, final UI implementation, final VFX implementation or final asset/runtime packaging scheme.

### BOUND DIRECTION from this baseline

Subject to normal review/integration, the client presentation architecture should be built around a project-owned Rust presentation/rendering layer using `wgpu`, while authoritative game state remains outside the renderer.

### DEFERRED

This baseline does **not** choose:

- KTX2 versus DDS or another runtime texture container;
- texture atlas versus texture array thresholds;
- final texture compression;
- final mipmap/filtering policy;
- final asset streaming/cache/bundle format;
- final UI technology;
- a permanent animation graph schema;
- a permanent particle implementation;
- a permanent post-processing stack;
- a permanent Studio authoring format.

## 3. Primary architecture boundary

Oteryn gameplay/domain state and Oteryn presentation must remain separate.

Conceptually:

```text
Authoritative server/gameplay state
            |
            +-----------------------------+
            |                             |
            v                             v
      RenderSnapshot              PresentationEvents
  persistent/current state          transient events
            |                             |
            +---------------+-------------+
                            |
                            v
                    Presentation Engine
                            |
              +-------------+-------------+
              |             |             |
              v             v             v
          World path      VFX path     Environment
              |             |             |
              +-------------+-------------+
                            |
                            v
                          wgpu
```

A renderer or visual animation frame must never become the authority that causes damage, movement, collision, loot, cooldown progression, target selection, spawn mutation or another gameplay effect.

## 4. Semantic input contracts

The presentation layer should consume three conceptually distinct classes of input.

### 4.1 RenderSnapshot

Represents state that exists now and must be reproducibly renderable from the current simulation/client-domain view.

Candidate contents include:

- camera/world view;
- visible tiles and floor information;
- items and world objects;
- creatures and players;
- direction and presentation-relevant semantic state;
- persistent fields/area effects where gameplay-owned;
- presentation-visible equipment/outfit state;
- persistent environment state required by the current view.

Exact Rust type names and ownership details remain implementation decisions.

### 4.2 PresentationEvents

Represents bounded transient presentation opportunities derived from authoritative events.

Candidate event families include:

```text
SpellEvent
ProjectileEvent
HitEvent
HealEvent
DeathEvent
LevelUpEvent
BossTelegraphEvent
TemporaryEnvironmentEvent
```

An event should carry semantic identity and authoritative timing/position information where needed, not a physical render resource address.

### 4.3 EnvironmentState

Represents the environment context needed to resolve visual presentation.

Candidate semantic fields include:

```text
time_of_day
season
weather
weather_intensity
wind
region_or_biome_context
optional_event_overlay
```

The exact schema is deferred.

Visual-only environment state may be presented locally. If any environmental condition changes movement, traversal, combat, spawn behavior, resource rules, visibility rules or another gameplay fact, that effect requires a separate authoritative server/gameplay contract.

## 5. Never encode gameplay in physical graphics identifiers

Authoritative or shared gameplay/domain contracts must not depend on:

- PNG paths;
- DDS/KTX2 paths;
- atlas coordinates;
- UV rectangles;
- texture-array layer numbers;
- GPU handles;
- renderer pipeline IDs;
- physical sprite-sheet positions;
- client quality tier.

Use semantic presentation identities instead.

Conceptually:

```text
Gameplay semantic event/state
    -> EffectKey / AppearanceKey
        -> PresentationRecipe
            -> presentation-family variant
                -> quality/density variant
                    -> compiled runtime resources
```

Exact names remain implementation-local until a later accepted contract requires public/shared names.

## 6. PresentationRecipe / VFX composition model

A spell or effect should not be assumed to equal one sprite animation.

A semantic effect may resolve to a recipe composed from zero or more presentation techniques:

```text
frame/sprite animation
+ particle emitter(s)
+ dynamic light pulse
+ projectile/trail
+ temporary decal
+ shader/material animation
+ bounded post-process effect
+ sound cue
```

Examples:

```text
FireExplosion
  -> frame animation
  -> sparks
  -> smoke
  -> short dynamic light pulse
  -> optional scorch decal
  -> sound

EnergyProjectile
  -> projectile sprite/material
  -> trail
  -> glow/light
  -> impact effect

ClassicFireExplosion
  -> classic frame animation
```

The semantic gameplay event remains the same regardless of presentation family.

## 7. Classic / Enhanced / HD presentation families

One gameplay identity should be able to resolve into different client presentation families.

Candidate concept:

```text
EffectKey::FireExplosion
   + Classic
      -> Tibia-like frame sequence

   + Enhanced
      -> frame sequence + bounded light/particles

   + HD
      -> higher-density art + particles/material effects
```

Classic, Enhanced and HD variants must preserve gameplay-significant readability, footprint, timing interpretation, interaction target and visibility policy.

A presentation family must not create a competitive information advantage that another supported family does not expose.

## 8. Spell and combat VFX authority

### 8.1 Spell timing

The server/game simulation owns authoritative spell start, activation, damage, cooldown and affected-area semantics.

The client may interpolate or animate between authoritative facts.

A client visual frame marker may trigger local non-authoritative presentation such as:

- sound;
- particles;
- flash;
- decal;
- camera feedback.

It must not trigger authoritative gameplay.

### 8.2 Area spells

For an area spell the server/gameplay side must own the authoritative affected area or sufficient deterministic gameplay semantics to derive it in the authoritative simulation.

The presentation layer may use that semantic area to draw:

- tile highlights;
- wave geometry;
- synchronized effects;
- boss telegraphs;
- impact effects.

The client must never infer gameplay hit coverage solely from what a visual sprite appears to cover.

### 8.3 Hit effects

Hit presentation should be driven by semantic damage/heal/event categories rather than hard-coded physical sprite identifiers.

Candidate presentation families may include:

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

Exact gameplay damage taxonomy remains owned by gameplay contracts; the presentation resolver consumes the semantic category supplied by those contracts.

## 9. Projectile model

A visual projectile is presentation, not collision authority.

Conceptually:

```text
ProjectileEvent
  -> source
  -> destination/trajectory semantics
  -> launch timing
  -> impact timing where authoritative
  -> EffectKey
```

Presentation may add:

- orientation;
- interpolation;
- trail;
- particles;
- light;
- impact VFX.

The visual projectile position must not decide whether the authoritative attack hit.

## 10. Boss telegraphs and gameplay readability

Boss telegraphs are gameplay-critical presentation.

They should be able to resolve from authoritative timing/area semantics into presentation such as:

```text
area marker
-> pulse/wind-up
-> activation
-> impact
```

The visual implementation may vary by presentation family, but it must preserve clear timing and affected-area readability.

Gameplay-critical telegraphs must receive higher degradation priority than decorative weather or ambient effects.

## 11. Lighting architecture

The renderer should remain capable of supporting:

- global ambient light;
- time-of-day lighting;
- local persistent lights such as torches;
- short-lived spell/impact light pulses;
- bounded environment/lightning flashes.

Lighting should enhance the Tibia/Oteryn presentation rather than overwhelm readability.

A renderer implementation should support a low-cost Classic path where advanced lighting can be reduced or disabled without changing gameplay semantics.

## 12. Day/night

Day/night should primarily be presentation state, not separately authored complete world copies.

The preferred architecture is:

```text
base appearance
+ time-of-day environment state
+ lighting/color treatment
-> final presentation
```

Separate day/night sprite sets may still exist where art direction genuinely requires them, but they must not become the default requirement for every world asset.

Time-of-day changes should be capable of smooth visual interpolation where the product chooses it.

## 13. Weather

Weather belongs to the environment/VFX presentation system.

Candidate visual-only weather includes:

- rain;
- snow;
- ash;
- leaves;
- sand/dust;
- fog;
- storm effects;
- fireflies/ambient particles.

Weather may compose:

```text
particle systems
+ wind
+ color/lighting treatment
+ optional bounded screen/material effects
```

Do not model thousands of decorative precipitation particles as authoritative gameplay entities.

A later GPU-oriented particle path should be evaluated for high-count ambient effects.

## 14. Seasons

Seasons should not require four independent canonical maps.

Conceptually:

```text
stable content/appearance identity
+ presentation family
+ quality tier
+ season/environment state
+ region/biome context
+ optional event overlay
-> resolved visual variant
```

Seasonal art may exist only where it materially improves presentation.

Examples:

```text
tree
  -> default
  -> autumn foliage optional
  -> winter/bare/snow optional

roof
  -> default
  -> snow-covered optional
```

A fallback resolver must permit partial seasonal coverage rather than requiring every object to have every seasonal variant.

Visual-only seasons are the preferred first implementation. Any gameplay consequence of a season requires separate authoritative design and implementation.

## 15. Weather and spell composition

Environment and spell presentation should be composable without changing gameplay semantics.

Examples of acceptable visual-only composition:

- fire spell during rain may add steam/suppress some smoke;
- lightning spell during a storm may coordinate with environment lighting;
- ice effects in winter may use a presentation variant;
- snow particles may be reduced around dense gameplay-critical VFX for readability.

These presentation interactions must not silently modify damage, range, cooldown, resistance or another gameplay value.

## 16. Temporary decals

The renderer should be able to support short-lived non-authoritative world decals such as:

- scorch marks;
- blood marks;
- frost residue;
- poison residue.

A candidate local presentation record may contain semantic appearance, position, spawn time, lifetime and fade behavior.

If a field/decal changes gameplay, it is no longer a presentation-only decal and must have an authoritative gameplay representation.

## 17. Particle system direction

High-count particles should not require one game-domain entity per particle.

The presentation architecture should permit renderer-owned batched/GPU-oriented particle resources.

A future implementation should evaluate:

- GPU buffers;
- instanced/batched rendering;
- deterministic or seeded emission where useful;
- bounded emitter lifetimes;
- quality scaling;
- strict caps to prevent runaway resource use.

No specific particle library or compute-shader design is accepted now.

## 18. Animation backend extensibility

Presentation should support semantic animation resolving to different techniques.

Candidate techniques include:

```text
FrameSequence
LayeredFrameSequence
ParticleAnimation
Shader/MaterialAnimation
future evidence-backed techniques
```

Classic Tibia-like creatures may remain frame-sequence based while an aura, projectile trail, water surface or HD environment object uses another technique.

Do not require all presentation content to use the same animation backend.

## 19. Renderer composition / pass model

The production pass structure remains implementation-dependent, but the architecture must support coherent ordering across Tibia-like tile stacks, entities and VFX.

Candidate conceptual order:

```text
world/floor/ground
borders and world overlays
world objects/items
creatures/players
world-attached gameplay VFX
projectiles
above-world particles/environment
lighting/environment composition
bounded post-processing
names/HP/gameplay overlays
UI
```

This is not a frozen render-pass ABI.

The real renderer must additionally respect floor, stack order, elevation, walls, roofs and occlusion rules from the world/presentation contracts.

## 20. VFX priority and graceful degradation

Under GPU/CPU pressure, decorative presentation should degrade before gameplay-critical presentation.

Candidate priority classes:

```text
CRITICAL_GAMEPLAY_VFX
IMPORTANT_GAMEPLAY_VFX
CHARACTER_VFX
AMBIENT_VFX
DECORATIVE_VFX
```

Degradation examples may include:

- reduce snow/rain particle density;
- reduce ambient particles;
- reduce decorative trails;
- reduce optional post effects;
- cap non-critical dynamic lights.

Do not remove or materially obscure critical spell/boss telegraphs merely to maintain decorative effects.

Exact budgets and thresholds require representative measurements and are deferred.

## 21. Quality tiers

Technical quality and art/presentation family should remain separate concepts.

A Classic player may still use high technical quality, while an HD player may lower particle/light quality to fit hardware limits.

Candidate scalable properties include:

- particle density;
- number/range of dynamic lights;
- trail quality;
- optional distortion/post effects;
- texture density;
- shadow/lighting detail where later applicable;
- environment density.

Exact tier names and numeric limits are deferred.

## 22. Failure and fallback behavior

The presentation layer must fail safely without changing authoritative gameplay.

Examples:

- missing HD variant -> explicit fallback to compatible lower-density/default presentation;
- unavailable optional shader/effect -> simpler visual fallback;
- VFX emitter allocation pressure -> degrade lower-priority presentation first;
- renderer/device recovery -> rebuild presentation resources from semantic snapshot/event-safe state where possible;
- unsupported high-tier resource -> reject/fallback rather than corrupting gameplay state.

A presentation failure must not produce a different authoritative simulation outcome.

## 23. Asset and GPU-resource boundary

The renderer may ultimately organize compiled assets using:

- texture arrays;
- atlases;
- standalone textures;
- streamed pages/bundles;
- combinations by asset class.

The disk/runtime container may ultimately use:

- KTX2;
- DDS;
- another evidence-backed format.

Those are **physical resource decisions**, not gameplay/presentation semantic identities.

They remain deferred until representative real-content measurements exist.

## 24. Why KTX2/DDS and atlas/texture-array remain separate decisions

These concepts solve different problems.

Conceptually:

```text
source art
  -> asset compiler
      -> resource organization
           atlas / texture array / standalone / streamed pages
      -> runtime container
           KTX2 / DDS / other
      -> wgpu resources
```

The project must not conflate a container choice with a resource-organization choice.

## 25. Next evidence gate: Oteryn World + VFX Prototype

The next renderer evidence slice should no longer be another general-engine bake-off.

It should exercise the selected project-owned `wgpu` direction against a production-shaped Oteryn workload.

Minimum representative probe should include:

1. real Tibia-like tile stack/floor ordering and occlusion;
2. representative appearance/item/creature workload;
3. animated creature(s);
4. projectile;
5. area spell;
6. hit/impact effect;
7. dynamic light;
8. day/night transition;
9. rain and snow presentation;
10. UI/text/nameplate/HP overlays;
11. camera movement, scrolling and zoom;
12. visibility-set churn/tile-page replacement;
13. texture upload/cache behavior;
14. RAM and trustworthy VRAM measurement where available;
15. GPU timing/timestamp evidence where reliable;
16. large-fight VFX stress and graceful degradation.

The probe should use the same semantic inputs across presentation variants wherever comparison is required.

## 26. Evidence required before freezing the asset runtime pipeline

Before choosing atlas/texture-array thresholds or KTX2/DDS/container/compression policy, measure at least:

- startup/load latency;
- texture upload latency;
- visible-world churn stalls;
- RAM/VRAM working set;
- draw-call/batching behavior;
- package/patch size;
- 32/64/128+ density impact;
- pixel-art visual quality;
- filtering/mipmap artifacts;
- cross-adapter/cross-platform support for intended `wgpu` targets;
- corrupted/unsupported resource rejection;
- Studio/compiler workflow cost.

## 27. Explicit non-decisions

Do not infer from this baseline that Oteryn has accepted:

- KTX2;
- DDS;
- texture arrays as universal storage;
- atlases as universal storage;
- bindless rendering;
- virtual texturing;
- compute particles;
- a specific UI crate;
- a specific shader framework;
- a specific sound library;
- a fixed logical pixel size;
- a fixed season calendar;
- gameplay effects from weather/seasons;
- a fixed set of quality tiers;
- a permanent VFX recipe serialization format.

## 28. Decision summary

The intended architecture direction is:

```text
Authoritative game/server semantics
            |
            +--> RenderSnapshot
            +--> PresentationEvents
            +--> EnvironmentState
                         |
                         v
                Presentation Resolver
                         |
              +----------+----------+
              |          |          |
          Appearance    VFX       Lighting/
          resolution   recipes    Environment
              |          |          |
              +----------+----------+
                         |
                         v
               project-owned wgpu renderer
```

This architecture is designed so that Oteryn can remain immediately recognizable as a Tibia-like game while supporting progressively richer rendering, HD presentation, spell effects, projectiles, particles, lighting, day/night, rain, snow and seasons without rewriting gameplay semantics or the canonical world model.

`IMPLEMENTATION_AUTHORITY: NONE`
`RUNTIME_ACTIVATION_AUTHORITY: NONE`
`MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`
