# Graphics, appearance, animation and seasonality horizon note

- Status: **DEFERRED DESIGN NOTE / NOT ACCEPTED ARCHITECTURE**
- Date: 2026-08-24
- Scope: future native client rendering, asset pipeline, creature/NPC/environment animation, presentation profiles and seasonal world presentation
- Implementation authority: **NONE**
- Runtime/product activation authority: **NONE**
- Decision timing: **Must decide now? NO**
- Related accepted architecture: `ADR-0005-native-world-format-and-oteryn-studio.md`, `ADR-0010-reference-and-evolved-world-product-profiles.md`
- Related contract/horizon: `DUR-04_CONTENT_WORLD_AND_SCRIPTING_CONTRACT.md`, `GAMEPLAY_AND_PRODUCT_ARCHITECTURE_HORIZON.md`, `OTERYN_ATLAS_ANIMATED_APPEARANCES_V1.md`

## Purpose

Preserve the owner discussion about how Oteryn should remain capable of starting with a Tibia-like pixel presentation while later supporting higher-resolution or substantially remastered graphics, richer animation and seasonal world presentation without rewriting the canonical map, server domain model or gameplay protocol.

This note deliberately records **invariants, desired extension points, candidate approaches and future decision questions**. It does not accept KTX2, Basis Universal, skeletal animation, shader animation, a specific texture size, a specific asset bundle encoding, a specific season schema or any other physical implementation choice.

The governing architecture discipline is to freeze only what blocks the next safe proof, preserve later extension points and benchmark technology choices when they become implementation-relevant.

## Existing architectural boundary

ADR-0005 already establishes that:

- Oteryn uses a native canonical world/content model rather than OTBM as the target runtime model;
- legacy OTBM/OTB/appearance/sprite content is bounded migration input;
- source/editor representation is separate from deterministic runtime bundles;
- stable namespaced content identity is canonical while legacy/compact numeric IDs are mappings;
- Oteryn Studio is intended to edit world, content, appearances, sprites and animations through one project-owned workflow.

DUR-04 further requires the canonical content model to remain a typed semantic graph independent of its final serializer and deliberately defers permanent physical World Project/Bundle encoding until evidence exists.

This note must not weaken those boundaries.

# 1. Core presentation invariant: world semantics are independent of pixels

The canonical world must not define object size or identity in terms of one fixed artwork resolution such as `32x32` pixels per tile.

A tile/grid coordinate is a **logical world unit**, not a mandatory pixel dimension.

Conceptually:

```text
World placement
  -> stable content/object identity
  -> logical footprint / collision / interaction semantics
  -> presentation identity
  -> client-selected render family and quality
  -> runtime GPU resources
```

A logical object may therefore keep the same world footprint while its presentation changes from, for example, a classic low-resolution asset to a 2x/4x/8x remastered asset.

Changing art density must not require rewriting:

- map coordinates;
- collision;
- pathing semantics;
- interaction semantics;
- authoritative server state;
- gameplay protocol semantics.

## Rescaling caveat

Simple upscaling of a low-resolution raster cannot reconstruct detail that never existed. Nearest-neighbour preserves pixel character, linear filtering blurs it, and AI/image reconstruction creates new inferred visual information.

Therefore a future HD/remastered mode should be able to use separately authored or regenerated high-resolution assets rather than assuming that `32px -> 256px` scaling alone produces an HD art set.

# 2. Separate semantic identity from render identity

The server and canonical world should refer to stable semantic/content identities. They must not depend on:

- PNG paths;
- sprite-sheet positions;
- atlas UV coordinates;
- GPU texture handles;
- KTX2/DDS/etc. container choices;
- client-selected quality tiers.

Candidate conceptual separation:

```text
ContentKey / ObjectPrototype
  -> presentation/appearance identity
      -> render family
          -> quality/density variant
              -> compiled runtime texture/animation resources
```

Exact Rust type names are deliberately deferred.

The server may need semantic presentation state where gameplay requires it (for example `door=open` or an authoritative attack action), but it should not tell the client which texture frame to draw.

# 3. Player-selectable presentation families

The native client should preserve the option to support multiple presentation families for the same world/content identity.

Candidate product model:

```text
Style / presentation family:
- Classic
- Enhanced
- Remastered / HD
- future families

Technical quality:
- Low
- Medium
- High
- Ultra
- Auto
```

Style and technical quality should remain conceptually separable. For example, a player may prefer Classic art while still using high-quality texture storage/filtering, or prefer HD art with a lower memory tier.

The system should allow a fallback chain when a preferred art family or density is unavailable for one asset. Exact fallback policy is a later product/client decision.

Optional presentation packs may eventually be downloaded/streamed independently so players are not forced to install the largest art tier. Packaging, CDN and patching policy are deferred.

## Fairness/readability invariant

Different art families must not silently create gameplay advantage.

Where presentation can affect visibility or interpretation, variants should preserve compatible gameplay-significant properties such as:

- footprint;
- interaction target;
- occlusion/readability class;
- authoritative state;
- timing semantics;
- visibility policy.

A prettier asset must not expose information hidden by another supported presentation family.

# 4. Runtime texture/storage direction to evaluate later

The project should keep source artwork human-friendly and runtime artwork compiled for GPU use.

Candidate taxonomy:

```text
source artwork
  PNG / Aseprite / layered source / vector where appropriate

render primitive
  textured quad / sprite / layered primitive / other specialized renderer path

resource organization
  texture atlas / texture array / standalone texture / streamed bundle

runtime container
  candidate: KTX2 or another evidence-backed container

GPU compression
  candidate per platform/asset class: BCn / ASTC / ETC2 / uncompressed formats / Basis-derived paths
```

A sprite is a rendering abstraction, not a file format. PNG is suitable as an authoring/interchange format but should not be assumed to be the final runtime packaging model for thousands of assets.

KTX2 is currently a strong candidate because it can contain mipmapped/array textures and multiple GPU-oriented compression paths, but **KTX2 is not accepted by this note**. The eventual choice must be validated against Oteryn's supported `wgpu` targets, Rust tooling maturity, loading/streaming cost, VRAM use, patch size, visual quality and operational complexity.

Pixel-critical art must not be blindly pushed through aggressive lossy block compression. Compression/filtering policy should be selectable by asset class when the pipeline is designed.

# 5. Large world objects should be logical objects, not necessarily fragmented art pieces

Legacy content may represent a fountain, tree or other large prop as several independently placed tile/item/sprite pieces.

The native target should remain capable of representing one logical object with:

- one stable object/prototype identity;
- position/anchor;
- multi-tile footprint;
- occupancy/collision mask where needed;
- sorting/occlusion semantics;
- interaction points;
- presentation definition.

Its Classic renderer may still reproduce a legacy multi-piece look if required, while a future HD renderer may draw one larger asset or another representation.

Legacy import must remain faithful and deterministic. Automatic consolidation of imported multi-piece structures into one native logical object is a later normalization feature and must only occur where recognition is safe and unambiguous.

# 6. Creature, NPC and player animation

Creature/NPC animation should be modeled semantically rather than as server-controlled sprite-frame numbers.

Candidate animation vocabulary may include states/actions such as:

```text
idle
walk
attack
cast
hit/react
death
special
interaction/talk/emote
```

The exact vocabulary belongs to later gameplay/client/content contracts.

The client should be able to resolve one semantic animation state into different presentation backends.

Candidate backends:

```text
FrameSequence
LayeredFrameSequence
Skeletal2D
other future presentation backend
```

Classic pixel art can remain conventional frame animation. A future HD family may use more frames, layered animation or evidence-backed skeletal 2D where it improves quality/storage/authoring economics.

No commitment to skeletal animation is made now.

## Server authority boundary

The server should own authoritative facts such as:

- entity position/direction where gameplay-significant;
- movement/action state;
- attack/cast/action start and authoritative simulation timing;
- damage, collision and all gameplay effects.

The client may interpolate or animate presentation locally from those facts.

A visual frame or animation marker must never become the authority that causes damage, movement, loot, collision or another gameplay mutation.

Visual animation markers may later drive non-authoritative presentation such as sound, particles, impact flash or camera feedback.

The current Atlas animated-appearance contract already preserves the important separation that appearance animation does not imply movement, AI, combat or server-observed state; the native runtime design should preserve this principle while being free to use a different presentation implementation.

# 7. Environment/world animation

Environment animation should support distinct classes rather than giving every animated tile/object an independent gameplay timer.

Candidate classes:

1. **continuous visual** — water, lava, fire, fountains, waterfalls, smoke, magic fields;
2. **stateful** — doors, levers, machines, chests, bridges and other state transitions;
3. **reactive** — grass, foliage, dust or other local responses to nearby activity;
4. **ambient/global** — wind, rain, snow, fog, cloud/light effects and similar environmental presentation.

Candidate synchronization modes:

```text
global
seeded/deterministic phase offset
local presentation
server/event driven
```

Large numbers of identical continuous animations should be able to share timing/program state instead of allocating one independent timer per tile.

For visually organic effects, a deterministic seed derived from stable placement identity/coordinates may provide phase variation without per-object authoritative state. Exact formula and determinism requirements are deferred.

## Future HD environment paths

The architecture should allow different presentation techniques by art family, for example:

```text
Classic water -> frame sequence
HD water      -> frame sequence OR shader/flow/normal-map animation

Classic tree  -> static/frame asset
HD tree       -> layered/procedural wind presentation
```

Shaders, particles, flow maps and procedural movement are candidate presentation techniques, not accepted requirements.

Whenever environment state affects gameplay rather than appearance, authoritative state remains a server/domain concern and must be separately modeled.

# 8. Seasonal world presentation

The presentation architecture should remain capable of supporting seasons without creating four independent canonical maps.

Desired future capability includes examples such as:

- flowers blooming and later fading;
- trees changing foliage colour;
- leaves falling;
- winter/bare/snow-covered vegetation;
- snow or frost overlays on roofs/props;
- seasonal ground/grass variations;
- regional ambient/weather changes;
- seasonal or event decorations.

Candidate conceptual model:

```text
stable appearance/content identity
  + client presentation family
  + technical quality tier
  + world season/environment state
  + region/biome context
  + optional event overlay
  -> resolved presentation variant
```

This model is deliberately conceptual. Exact resolver ordering, schemas and cache keys are deferred.

## Global and regional season state

A future season/environment system should be able to distinguish:

- a broad/global season progression;
- region/biome overrides;
- locations whose climate does not follow the global cycle;
- temporary event overlays.

This prevents a single rigid rule such as "winter means identical snow everywhere".

## Visual-only first principle

The safest initial seasonal capability is visual/ambient only.

If a future design makes seasons affect gameplay — for example frozen traversal, movement friction, access, resource availability or combat — that becomes authoritative game logic and requires an explicit server/gameplay contract. Presentation variants alone must not create gameplay state.

## Seasonal fallback

Not every object needs four unique seasonal variants. A later resolver should support explicit fallback/default behavior so content production can add seasonal coverage incrementally rather than requiring a complete world-wide art replacement before activation.

# 9. Presentation and gameplay revision boundaries

Future implementation should keep distinct concepts for:

- content/world revision;
- gameplay/ruleset revision;
- presentation/art revision;
- client renderer/runtime capability;
- optional presentation-pack revision.

A presentation-only revision should not force gameplay semantic migration when the logical content contract is unchanged.

Conversely, a visual asset must not be used to smuggle an authoritative semantic change past normal content/gameplay revision controls.

This aligns with the accepted Reference/Evolved model: different supported world/product profiles still use one canonical engine/client/protocol family, while versioned content/asset/presentation profiles may vary explicitly.

# 10. Rust and renderer compatibility goal

The target remains a native Rust client/server stack. Graphics/runtime choices should therefore be evaluated for:

- robust Rust integration;
- `wgpu` compatibility across intended targets;
- deterministic/project-owned asset compilation where applicable;
- bounded loading and validation;
- asynchronous streaming/upload capability;
- predictable RAM/VRAM budgeting;
- toolchain maintainability;
- absence of unnecessary platform lock-in unless product scope explicitly accepts it.

Server code should not depend on renderer/GPU libraries merely because both applications are written in Rust. Shared crates should expose semantic contracts, not renderer implementation details.

# 11. Future asset streaming and GPU execution goals

When the real client/content scale requires it, the renderer/asset manager should be evaluated around:

- chunk/visibility-driven asset demand;
- RAM and VRAM caches with explicit budgets;
- preload of nearby content;
- eviction of unused pages/assets;
- bundled/content-addressed runtime resources;
- batching and GPU instancing where appropriate;
- texture arrays for compatible fixed-shape content;
- atlases for irregular content where beneficial;
- standalone resources only where justified.

Virtual texturing, bindless designs and similarly advanced techniques should remain optional future optimizations until representative measurements demonstrate a need.

# 12. Evidence required before freezing implementation technology

When these subjects become near-term implementation gates, the decision should be based on representative evidence rather than architecture preference alone.

At minimum measure/validate as applicable:

- client load/startup latency;
- asset streaming latency and stall behavior;
- RAM and VRAM use;
- package/download/patch size;
- texture upload cost;
- draw-call/batching/instancing behavior;
- visual quality for pixel-critical and HD art;
- filtering/mipmap artifacts;
- animation CPU/GPU cost with representative creature/environment counts;
- deterministic asset compiler output where required;
- corrupted/oversized/unsupported asset rejection;
- support across intended `wgpu` target adapters;
- fallback behavior when a high-tier asset is missing;
- parity of gameplay-significant visibility/footprint/occlusion across supported presentation families;
- representative Studio authoring/edit/preview workflow.

# 13. Future decision gates / trigger points

This note should be reopened when one of these becomes concrete:

1. **Native client real-content renderer** — when the client must render substantial production-like world/creature/item content rather than synthetic fixtures.
2. **Permanent asset/runtime bundle design** — alongside the DUR-04 physical-format evidence gate and World Bundle decision.
3. **Oteryn Studio Asset Editor** — before freezing source artwork formats, animation authoring representation or asset compiler interfaces.
4. **Presentation-family product feature** — before exposing Classic/Enhanced/HD/Auto choices to players.
5. **HD/remaster production** — before committing to frame-heavy HD, skeletal 2D, shader-heavy environment paths or a particular compression/container pipeline.
6. **Season/environment system** — before adding authoritative season state, regional climate rules or seasonal content activation.
7. **Cross-platform expansion** — before selecting a runtime texture/compression path whose portability assumptions become public product constraints.

# 14. Explicitly deferred decisions

Do **not** infer acceptance of any of the following from this note:

- fixed logical tile pixel size;
- KTX2 as mandatory runtime container;
- DDS or another container rejection;
- Basis Universal as mandatory compression/transcoding;
- exact BC/ASTC/ETC/uncompressed policy;
- texture-array versus atlas thresholds;
- exact mipmap/filtering rules;
- specific Rust image/texture crates;
- skeletal 2D engine/library;
- exact animation graph schema or clip vocabulary;
- exact animation timing protocol fields;
- exact appearance/presentation Rust type names;
- exact quality tier names;
- exact downloadable pack boundaries;
- exact season duration/calendar/progression;
- exact biome/climate model;
- exact weather implementation;
- gameplay effects of seasons;
- exact procedural shader/particle techniques;
- virtual texturing or bindless requirements.

# 15. Decision-discipline checkpoint

## Must decide now?

**NO.** The current implementation programme can proceed without freezing these physical/runtime presentation technologies.

## What downstream work would eventually be blocked?

Real production-content client rendering, final asset compilation/bundling, Oteryn Studio asset authoring, player-selectable presentation families, HD/remaster production and a season/environment system will each require portions of this note to become explicit contracts or bounded technology decisions.

## What becomes harder if the extension points are lost now?

The costly mistakes to avoid now are:

- making `32x32` pixel dimensions canonical world semantics;
- placing raw sprite IDs/file paths/atlas UVs in canonical map state;
- coupling server gameplay to client animation frames;
- treating one art family as the only legal appearance representation;
- making large objects permanently dependent on legacy multi-piece decomposition;
- mixing visual season state with gameplay authority by accident.

Avoiding those couplings is valuable now even though the final implementation technologies remain deferred.

## What evidence could change the preferred direction?

Representative renderer/asset benchmarks, cross-platform target changes, Studio authoring evidence, art-production economics, download/patch constraints, visual-quality testing, legal/provenance findings or a concrete product requirement may justify choosing different texture containers, compression families, animation backends or seasonal-resolution mechanisms.

## What is deliberately not decided?

All concrete physical encodings, libraries, schemas, quality thresholds, compression formulas, art-production commitments and gameplay-season rules listed in the deferred section remain open.

# 16. Re-entry checklist for a future architecture task

When this horizon becomes active, the responsible task should at minimum:

1. re-read current ADR-0005, DUR-04, native-client and renderer contracts rather than assuming this note is still current;
2. inspect the then-current Rust/`wgpu` target matrix;
3. measure representative real/synthetic world and creature workloads;
4. define semantic appearance and animation contracts before physical packing details;
5. define presentation-family compatibility/fairness rules;
6. decide authoring-source versus runtime-compiled asset boundaries;
7. decide asset bundle/container/compression strategy from evidence;
8. define creature/NPC/environment animation backend boundaries;
9. if seasons are in scope, separate visual environment state from authoritative gameplay state;
10. capture the accepted result in a dedicated ADR/contract that explicitly supersedes or incorporates this note.
