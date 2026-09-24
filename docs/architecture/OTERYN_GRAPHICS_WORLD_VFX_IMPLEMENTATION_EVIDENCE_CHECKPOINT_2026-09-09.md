# Oteryn Graphics World + VFX Implementation Evidence Checkpoint

- Date: 2026-09-09
- Status: **EVIDENCE CHECKPOINT / IMPLEMENTATION GATE INPUT**
- Scope: renderer-facing implementation constraints discovered after the protected presentation/VFX baseline and Tibia 15.x rendering audit
- Related protected architecture: PR #473, PR #475
- Related candidate renderer evidence: Issue #465 / PR #468
- Related active deep audit: PR #477
- Runtime implementation authority: **NONE**
- Product/live activation authority: **NONE**
- Server/protocol/content mutation authority: **NONE**

## 1. Purpose

This checkpoint serializes the remaining material findings from the 2026-09-09 graphics audit so the next `OTERYN WORLD + VFX PROTOTYPE` can be implemented without depending on chat history.

It does not replace the protected presentation/VFX baseline or the protected Tibia 15.x -> Oteryn audit. It narrows implementation evidence that must be collected next.

The checkpoint deliberately does **not** freeze:

- KTX2 vs DDS;
- texture atlas vs texture arrays vs hybrid;
- final streaming/cache/bundle representation;
- final particle architecture;
- final light implementation;
- final shader/material graph;
- final UI technology;
- final compression/mipmap/filter policy;
- final cross-platform asset package.

## 2. Current protected/candidate state

Protected architecture now establishes:

- project-owned native Rust presentation direction;
- `RenderSnapshot + PresentationEvents + EnvironmentState` semantic separation;
- server/gameplay authority for hit/damage/AoE/timing;
- Tibia-like floor/tile/stack and `object/outfit/effect/missile` semantics;
- composable VFX;
- Classic / Enhanced / HD presentation families with identical gameplay semantics.

PR #468 remains candidate physical evidence for `ADOPT_CUSTOM_WGPU` until protected integration. Its measured workload is intentionally synthetic and static-visible-set-biased; it is not the final world/VFX performance authority.

## 3. Four independent presentation scales

Oteryn must keep four different scale concepts independent:

```text
1. logical world/tile scale
2. source-art density / presentation density
3. rendered world zoom / screen pixels per tile
4. UI / overlay scale
```

Examples:

- a 128x128 source creature does not automatically occupy four times the gameplay/world area of a 32x32 presentation of the same creature;
- changing camera zoom does not change authoritative world coordinates;
- changing DPI does not change world semantics;
- creature names, HP bars, boss markers and gameplay overlays must remain readable independently of source-art density.

The prototype must exercise independent changes to all four axes.

## 4. Multi-floor world-to-screen projection

A Tibia-like renderer does not merely select visible floors. It projects records from different floors into the 2D screen plane with a floor-dependent offset.

The Tibia-compatible reference mapping is equivalent, after converting to native Oteryn floor semantics where larger `FloorId` means geometrically higher, to:

```text
screen_x proportional to (world_x - camera_x) - (floor - camera_floor)
screen_y proportional to (world_y - camera_y) - (floor - camera_floor)
```

Therefore a geometrically higher floor shifts visually north-west relative to the camera floor.

Requirements:

- do not copy legacy Tibia `z` numerics directly;
- use the accepted native Oteryn floor direction;
- test at least camera floor, one floor above and one floor below;
- test stairs/holes/roof transitions;
- verify exact projected tile ownership under camera movement and zoom.

This is presentation projection only and does not redefine canonical world position.

## 5. Floor visibility and occlusion correctness

The prototype must contain a dedicated floor/roof correctness gate.

It must cover:

- first/last visible floor selection;
- roofs and opaque cover;
- look-through openings such as doors/windows where the owning world semantics permit it;
- walls/ground limiting floor visibility;
- entering/leaving a building;
- underground transition;
- camera movement while the visible floor set changes;
- covered/uncovered creature presentation;
- no stale occlusion cache after an opaque record is removed or replaced.

Do not reduce the implementation to `draw only camera.floor`.

## 6. Static PresentationOrderKey is necessary but not sufficient

Existing Game-owned `PresentationOrderKey` closes deterministic same-position ordering for static presentation records.

The production renderer additionally needs dynamic composition rules for:

- walking creatures crossing tile ownership;
- effects;
- missiles/projectiles;
- attached effects;
- particles;
- top effects;
- large sprite overlap;
- temporary decals;
- gameplay overlays.

Requirements:

- do not overload static `PresentationOrderKey` with meanings it does not own;
- preserve static producer order as input truth;
- derive a renderer-local dynamic order key or equivalent ordered render command representation;
- order correctness must win over texture-state sorting and batching efficiency.

## 7. Order-preserving batching

Batching must not reorder visually significant Tibia-like overlap.

A useful benchmark metric is:

```text
order_preserving_batch_count
```

not merely theoretical minimum draw count.

The prototype must report at least:

- semantic draw records before batching;
- resulting order-preserving batches/submissions;
- state/page transitions;
- average and p95 batch size;
- batch fragmentation caused by texture/resource page changes;
- fragmentation caused by shader/material/light/VFX state.

Atlas/array/hybrid verdicts must account for this real ordered workload.

## 8. Walking presentation must be frame-rate independent

Movement interpolation is presentation state only.

A creature walking from tile A to tile B may visually occupy both source/destination tile regions during a step while canonical authoritative position remains controlled by gameplay/server observations.

Requirements:

- walk timing is time/tick based, never `N rendered frames` based;
- equivalent walk duration at 60, 144 and 300 FPS;
- diagonal overlap ordering must be tested;
- elevation/displacement must compose with walk interpolation;
- camera-follow movement must not produce double interpolation;
- prediction/rollback is not implicitly authorized by this visual interpolation.

## 9. All animation and VFX timing must be time-based

The synthetic #468 experiment intentionally advances animation by benchmark frame number. That mechanism must not become runtime production semantics.

Production presentation must not change duration because rendered FPS changes.

This applies to:

- object animation;
- creature idle/walk animation;
- spells;
- projectiles;
- hit effects;
- boss telegraphs;
- decals;
- particles;
- light pulses;
- day/night transitions;
- weather transitions.

The prototype must include a frame-rate invariance check across multiple frame rates.

## 10. Source animation ranges vs runtime playback policy

The normalized Oteryn 15.32 animation contract preserves source timing metadata including:

- `duration_min` / `duration_max`;
- synchronized state;
- start phase;
- ping-pong/infinite/counted loop semantics.

The Atlas-side midpoint policy exists for deterministic Atlas presentation/export purposes. It must not silently become production native-client playback policy.

The production client may require a separately evidence-backed runtime policy, for example deterministic seeded sampling within source ranges.

Requirements:

- preserve raw source ranges;
- preserve synchronized semantics;
- choose runtime sampling policy explicitly;
- make repeated runs reproducible where deterministic testing requires it;
- avoid globally synchronized visual monotony for content whose source semantics are asynchronous;
- do not treat presentation RNG as gameplay RNG.

Decision remains evidence-gated.

## 11. Shared animation programs, not one heavyweight timer per world placement

The exact 15.32 corpus contains thousands of animated appearance programs while the full legacy world contains tens of millions of presentation records.

Do not tick animation state for all world placements.

Preferred evidence target:

```text
shared AnimationProgram
+ minimal visible-instance state
+ shared/global clock where synchronization permits
+ stable instance seed/start state where asynchronous presentation requires it
```

Offscreen static-environment animation should be derivable on demand where possible.

When a record returns to the viewport, visual phase should follow the accepted animation policy rather than automatically restarting because it was culled.

The prototype must measure CPU and memory cost of animation state under large visible workloads.

## 12. Real 15.32 workload must replace synthetic-only animation assumptions

The exact Game-owned 15.32 census includes:

- 43,514 object appearances;
- 5,190 animated object appearances;
- 1,480 outfit appearances;
- 1,732 animated outfit frame groups;
- 243 effect appearances, 240 animated;
- 76 missile appearances;
- object phase counts reaching 125;
- effect phase counts reaching 27.

The prototype may use bounded representative subsets, but the selection must be derived from this real distribution rather than assuming one universal 36-frame shape.

## 13. Effect/missile direction selection and timing are separate concerns

A missile may use appearance direction/pattern semantics independently from its visual travel timing.

For legacy-compatible missile art, directional selection may map to a 3x3 direction layout.

Do not inherit reference-client travel-duration formulas as gameplay authority.

Requirements:

- appearance direction selection is presentation resolution;
- visual interpolation duration is bounded by accepted presentation/authoritative event timing;
- hit/miss/damage remains server/gameplay authority;
- visual projectile completion must not itself cause the hit.

## 14. PresentationEvent identity, deduplication and resync

Transient effects must survive reconnect/resync semantics without becoming duplicate gameplay/presentation noise.

Future event representation needs enough stable semantic information to support:

- duplicate suppression;
- late event handling;
- presentation expiration;
- snapshot replacement;
- reconnect/resync;
- cancellation/replacement of persistent effects;
- deterministic test replay.

A reconnect or replacement snapshot must not replay a fire explosion twice merely because the same authoritative outcome is observed again through a replacement stream.

Exact public wire representation remains deferred.

## 15. Finite, persistent and cancellable effect lifecycle

Effects are not all one-shot animations.

The prototype must support at least:

```text
finite one-shot effect
persistent/looping effect
explicitly cancellable persistent effect
replacement/superseding effect
```

Persistent visual effects must not leak indefinitely after source state disappears.

An effect ending visually does not end gameplay state unless the owning gameplay contract says so.

## 16. Composite VFX visibility policy must apply to the entire recipe

Oteryn VFX is composable.

If a non-critical effect is reduced through source/quality/degradation settings, the same policy must apply coherently to its complete presentation recipe:

- sprite/frame animation;
- particles;
- trail;
- dynamic light;
- decal;
- shader/material contribution;
- bounded post effect;
- sound where user audio policy applies.

Otherwise a nominally hidden spell can still fill the screen through particles/lights/trails.

Critical gameplay telegraphs may explicitly override ordinary presentation reduction.

## 17. Source-aware VFX visibility and critical override

Contemporary Tibia introduced spell-opacity controls separating at least:

- own spell effects;
- other players' spell effects;
- monster spell effects;
- a separate boss-area creature-effect control.

Official Tibia evidence:

- Winter Update 2025 / Spell Opacity;
- January 13, 2026 fix making selected radicular/decaying totem spell effects always visible independently of spell-opacity settings.

This provides useful product precedent for Oteryn.

Recommended Oteryn semantic visibility inputs:

```text
source_class = self | party/ally | other_player | monster | boss | environment
priority = critical | important | character | ambient | decorative
```

Hard requirement:

- a critical boss mechanic/telegraph cannot disappear because the player lowers ordinary spell opacity or decorative quality;
- accessibility settings may alter rendering style, but equivalent gameplay information must remain available.

## 18. Gameplay visibility envelope is independent from camera zoom

A larger monitor, ultrawide viewport, HD presentation family or greater zoom-out must not reveal gameplay information outside the authoritative client-interest/visibility envelope.

Renderer camera capability and network/gameplay visibility are separate constraints.

Requirements:

- clamp/render only information legitimately present in the accepted client projection;
- do not fabricate unseen world records to fill an enlarged viewport;
- test zoom-out beyond current authoritative visible-data bounds;
- provide a clear presentation behavior at viewport edges rather than silently creating information advantage.

## 19. Large visual coverage and culling

Game-owned appearance profiles explicitly separate visual coverage from gameplay footprint.

Renderer culling must account for visual overhang.

A record whose owning tile is just outside the viewport may still have visible pixels inside the viewport.

Requirements:

- use projected visual coverage/displacement for render culling and resource prefetch;
- do not use visual coverage as collision, pathing or interaction authority;
- test 32x64, 64x32 and 64x64 source cells at every viewport edge;
- test large multi-cell object compositions from real content.

## 20. Screen picking must not confuse visual overhang with gameplay footprint

Screen -> world interaction should resolve through semantic world/entity ownership, not by treating all visible sprite pixels as authoritative interaction area.

A future UX policy may intentionally make a visible overhanging region clickable and route that click to the owning entity/tile, but that is presentation/input mapping, not gameplay occupancy.

Requirements:

- screen coordinates convert through current camera/zoom/DPI state;
- resulting interaction intent is semantic;
- server remains authoritative for legality;
- large art density does not enlarge attack/use range.

## 21. World-space and screen-space overlays must be separate

Creature names, HP bars, status indicators and critical overlays should not be raster-scaled as ordinary world sprites.

Prototype must separate:

```text
world-space presentation
screen/overlay-space gameplay information
UI
```

Requirements:

- world zoom does not unintentionally blur/stretch text;
- UI scale is configurable independently;
- critical overlays remain legible at supported zoom levels;
- Classic/Enhanced/HD do not expose different gameplay information.

## 22. Pixel-stable Classic path and sampling quality gate

Classic presentation should preserve crisp Tibia-like pixel readability.

Enhanced/HD may use different sampling where evidence supports it.

Do not freeze nearest/linear/mipmap policy yet.

Prototype visual-quality tests must include:

- integer zoom;
- fractional zoom;
- camera panning at fractional pixel positions;
- texture page boundaries;
- transparent edges;
- atlas padding/bleeding;
- shimmer during motion;
- Classic pixel stability;
- Enhanced/HD perceived quality.

A texture-layout verdict is incomplete if it measures throughput but ignores visible artifacts.

## 23. Color space, alpha and surface configuration

Current production renderer foundation still uses `surface.get_default_config(...)` and is a Windows/DX12 spike. It does not yet freeze final surface format, sRGB handling or present mode.

World+VFX evidence must record:

- adapter/backend;
- selected surface format;
- whether the format is sRGB;
- present mode;
- alpha mode where relevant;
- HDR state if ever exercised;
- texture color-space assumptions;
- blending mode for transparent sprites/VFX;
- whether lighting/color grading is evaluated in an appropriate working space.

Golden visual tests should include translucent sprites, lighting, day/night grading and overlapping semi-transparent effects.

Do not accept a lighting/VFX implementation whose visual correctness depends accidentally on one default surface format.

## 24. Uncapped benchmark mode vs normal gameplay frame pacing

Benchmark measurements must not accidentally measure VSync or a configured FPS cap instead of renderer cost.

The prototype should expose at least two explicit modes:

```text
EVIDENCE_UNCAPPED
NORMAL_GAMEPLAY_PACING
```

Evidence must record:

- present mode;
- frame cap;
- VSync state;
- measured CPU frame time;
- measured GPU frame time where trustworthy;
- end-to-end presented frame pacing separately from uncapped throughput.

## 25. Renderer cache identity and generation safety

The existing renderer resource cache already has process-generation fencing, which must be preserved.

The next prototype must additionally test semantic presentation-resource identity so a stale resource from another content/presentation context cannot be reused accidentally.

Candidate cache key inputs include:

```text
content/presentation revision
presentation family
art density / variant
semantic resource identity
GPU format/variant where needed
```

Exact key shape remains implementation-local.

Required tests:

- Classic -> HD switch;
- presentation/content revision replacement;
- device/surface loss and reconstruction;
- stale generation rejection;
- cache eviction followed by deterministic re-resolution.

## 26. Full-world scale requires a working-set cache

Existing full-world evidence contains approximately:

- 18,997,668 tiles;
- 24,502,036 presentation records;
- 25,198 unique visible appearance source IDs;
- 27,394 unique sprite source IDs;
- 16 floors.

The native renderer must not load or tick the entire world in RAM/VRAM merely because the offline corpus exists.

Prototype must test a bounded viewport/working-set model with:

- camera scrolling;
- floor transitions;
- resource page entrance/exit;
- repeated backtracking;
- cache hits/misses;
- eviction/reload;
- prefetch margin;
- peak RAM;
- peak VRAM where trustworthy;
- upload bytes/time;
- hitching under churn.

## 27. Cold first-cast and critical VFX prewarm

Camera churn alone is insufficient cache evidence.

The prototype must test a cold first use of:

- representative player spell;
- monster spell;
- boss telegraph;
- projectile;
- dynamic light/material effect.

A critical mechanic must not hitch materially because its texture, shader pipeline or material resources are first created at cast time.

Possible implementation directions to measure:

- preload by visible/nearby content capability;
- prewarm known material pipelines;
- small mandatory critical-VFX fallback;
- background upload ahead of use.

No final strategy is selected yet.

## 28. Semantic material classes and bounded pipeline count

`PresentationRecipe` must not imply an unbounded new GPU render pipeline for every effect/content record.

Prefer:

```text
semantic material/effect class
+ bounded shader/pipeline family
+ per-instance parameters
```

Prototype must report:

- number of distinct render pipelines created;
- cold pipeline creation latency;
- pipeline cache/prewarm behavior;
- pipeline changes per frame;
- memory associated with material/pipeline state.

Do not permit content data to instantiate arbitrary unreviewed GPU pipeline topology.

## 29. Weather exposure / indoor-outdoor transition

Rain and snow must not be implemented only as unconditional fullscreen overlays.

Prototype must cover:

```text
outdoor
-> doorway/covered edge
-> roof/interior
-> cave/underground
-> back outside
```

The presentation layer needs sufficient semantic exposure/coverage input to suppress or alter precipitation where the world is visually sheltered.

This does **not** require precipitation particles to become gameplay entities.

If weather changes gameplay, that remains a separate authoritative game contract.

## 30. Atlas vs texture arrays must be tested as a real multi-page problem

The exact accepted source geometry contains at least the following decoded cell classes:

```text
32x32
32x64
64x32
64x64
```

Texture arrays per geometry class are therefore a realistic challenger, but the real corpus requires pagination because one array cannot contain every resource under practical/adapter limits.

Prototype comparisons must include:

- atlas pages;
- texture-array pages per geometry class;
- hybrid strategy;
- realistic page counts;
- page/bind transitions under order-preserving rendering;
- upload/reload cost;
- memory waste from fixed array layers;
- atlas padding/bleeding cost;
- shader/descriptor complexity;
- camera/resource churn.

Do not accept a verdict from one tiny atlas vs one tiny array.

## 31. KTX2 vs DDS remains a separate decision

Texture organization and disk/runtime container are independent questions.

The eventual container benchmark must account for:

- Windows/DX12 current foundation;
- future Vulkan/Metal portability requirements if product scope includes them;
- supported GPU compression families;
- decode/transcode latency;
- CPU staging memory;
- upload time;
- patch/download size;
- mip chain storage;
- tooling maturity;
- corruption validation;
- cross-platform consistency.

The current DX12-only renderer spike is not enough evidence to choose the permanent cross-platform container.

## 32. Proprietary asset/provenance boundary

Real Tibia asset evidence may be used only under the already recorded project authorization/provenance constraints.

A normal GitHub PR/CI job must not copy proprietary Tibia art into the repository merely to exercise the renderer.

Preferred evidence split:

```text
public-safe committed fixture
+ normalized Game-owned metadata/contracts
+ authorized local/private real-15.32 benchmark run
+ committed metrics/digests without redistributing restricted source pixels
```

The prototype must preserve exact source identities/digests in evidence where real-source measurements are cited.

## 33. Prototype input should be normalized Game-owned presentation data

The native renderer prototype should not parse `appearances.dat`, raw Tibia sprite containers or OTBM as runtime authority.

Reuse existing Game-owned normalization where possible:

- native world coordinate/floor profile;
- `PresentationOrderKey`;
- appearance spatial profile;
- visual coverage/displacement;
- normalized animation programs;
- resolved concrete pattern/layer/sprite semantics for fixtures.

For effects/missiles, extend the Game-owned normalized fixture/evidence path rather than teaching the renderer to reinterpret raw legacy source formats.

## 34. Required benchmark/evidence matrix additions

In addition to the completion criteria already recorded by the deep audit, the final prototype evidence must explicitly cover:

- 60/144/300 FPS time-invariance checks;
- integer and fractional zoom;
- DPI/overlay scale independence;
- multi-floor projection parity;
- floor/roof visibility transitions;
- large visual-overhang culling;
- semantic picking under large/HD art;
- source-aware VFX opacity and critical override;
- outdoor/indoor precipitation transition;
- cold first-cast / first-boss-VFX path;
- pipeline prewarm/cold creation;
- cache revision/family replacement;
- order-preserving batch fragmentation;
- multi-page atlas/array/hybrid comparison;
- sRGB/alpha/light visual golden tests;
- uncapped vs gameplay-paced measurements;
- real-corpus-derived animation distribution.

## 35. Decisions that may be made after this gate

Only after the representative prototype may Oteryn issue evidence-backed verdicts for:

```text
atlas vs texture arrays vs hybrid
KTX2 vs DDS vs another container
streaming/cache topology
particle implementation direction
light/VFX budgets
order-preserving batching thresholds
pipeline/material strategy
sampling/mipmap/compression policy
RAM/VRAM budgets
```

Each verdict must be one of:

```text
ADOPT
REJECT
INSUFFICIENT_EVIDENCE
```

No value should be guessed merely to unblock implementation.

## 36. Terminal checkpoint

The graphics architecture is sufficiently bounded to begin the production-shaped `OTERYN WORLD + VFX PROTOTYPE` once repository allocation/authority permits it.

The expected boundary is:

```text
server/gameplay authority
        |
        v
accepted non-authoritative client projection
        |
        +--> RenderSnapshot
        +--> PresentationEvents
        +--> EnvironmentState
                    |
                    v
          Oteryn presentation engine
                    |
          semantic render commands
                    |
            order/cull/batch
                    |
                   wgpu
```

The next material progress should come from executable evidence, not additional speculative renderer-format discussion.

`IMPLEMENTATION_AUTHORITY: NONE`
`RUNTIME_ACTIVATION_AUTHORITY: NONE`
`MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`
