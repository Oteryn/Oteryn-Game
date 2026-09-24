# Oteryn graphics rendering deep-audit addendum

- Date: 2026-09-09
- Parent: `OTERYN_GRAPHICS_RENDERING_DEEP_AUDIT_AND_WORLD_VFX_GATE_2026-09-09.md`
- Status: **CANDIDATE AUDIT ADDENDUM / REVIEW REQUIRED**
- Scope: multi-floor screen projection, presentation-vs-gameplay source flags, VFX source visibility/critical overrides, independent rendering scales, animation-program sharing, and texture-array pagination evidence
- Runtime/product activation authority: **NONE**

## 1. Purpose

Record material findings discovered after the first #477 deep-audit file was frozen so they are not left only in chat and are not lost by replacing an already-reviewed document wholesale.

This addendum narrows prototype correctness/performance requirements. It does not freeze final Rust type names, render passes, GPU layout, texture container or production hard maxima.

## 2. Multi-floor world-to-screen projection is separate from floor identity

The pinned Tibia-compatible reference `opentibiabr/otclient@dda0520ac61d1626f7d3a6d255394937038ec8ea` uses a floor-relative 2D transform conceptually equivalent to:

```text
screen_x = center_x + (position.x - camera.x) - (camera.z - position.z)
screen_y = center_y + (position.y - camera.y) - (camera.z - position.z)
```

The accepted Oteryn native floor semantics are deliberately different from legacy Tibia Z numbering:

```text
native.floor = -legacy.z
larger native FloorId = geometrically higher / above
```

Therefore a native Oteryn-compatible equivalent is conceptually:

```text
floor_delta = position.floor - camera.floor

screen_tile_x = center_x + (position.x - camera.x) - floor_delta
screen_tile_y = center_y + (position.y - camera.y) - floor_delta
```

A position one native floor above the camera therefore projects one tile north-west in the Tibia-like view.

### Requirements

- this is presentation projection only;
- it must not redefine canonical `WorldTilePosition`;
- it must not be derived by blindly copying legacy `z` signs;
- camera/floor projection and floor-visibility selection are separate operations;
- higher/lower-floor projectiles, effects and large appearances must use the same projection convention as world primitives;
- screen projection must be tested through camera movement and floor transition.

### Mandatory prototype fixtures

At minimum prove:

1. same `(x,y)` records on camera floor, one floor above and one floor below;
2. multi-floor wall/roof overlap;
3. projectile/effect on a non-camera floor;
4. walking creature while camera floor changes;
5. zoom while multiple floors remain visible;
6. round-trip/reference screenshots or deterministic coordinate assertions sufficient to detect sign inversion.

The final renderer may use matrices, integer/fixed-point transforms or another representation; this formula is a semantic parity target rather than a frozen implementation API.

## 3. Source appearance metadata mixes presentation and gameplay-like flags

The modern Tibia appearance schema/reference contains properties spanning multiple concerns.

Presentation-relevant examples include concepts equivalent to:

```text
ground / border / bottom / top
light
translucent
shift / displacement
height / elevation
lying object
animate always
top effect
```

The same source family also contains attributes equivalent to:

```text
not walkable
not moveable
block projectile
not pathable / avoid
```

This coupling is useful legacy conversion evidence but must not become the Oteryn renderer authority model.

### Oteryn split

The project must preserve a hard separation:

```text
Presentation metadata
    -> draw order / coverage / displacement / visual light / animation

Game-domain world/content semantics
    -> occupancy / collision / movement legality / line-of-sight / projectile legality / interaction
```

Existing Game-owned appearance spatial profiles already establish the crucial invariant:

```text
visual coverage != gameplay footprint
```

This addendum extends that discipline to source appearance flags generally.

### Prototype requirement

The prototype must not ask a renderer/GPU asset object whether a tile is walkable, blocks projectiles or is authoritative line-of-sight geometry.

If the floor-visibility presentation resolver needs a semantic fact resembling wall/coverage/visibility behavior, that fact must be provided by a Game-owned normalized presentation/world fixture with explicit scope. It must not be inferred ad hoc from a raw legacy asset bit in the render backend.

## 4. Spell/VFX visibility should be source-aware

Official Tibia product behavior provides direct evidence that large-fight effect readability is important enough to expose source-specific controls.

Official evidence:

- `2025-10-22`, Tibia news `Spell Opacity` (`id=8552`) introduced independent opacity controls for the player's own spells, other players' spells and monster spell effects;
- `2025-11-24`, Winter Update 2025 (`id=8570`) shipped those controls and added a separate opacity setting for creature effects in boss areas;
- `2026-01-13`, fixes/changes (`id=8644`) made specified totem spell effects always visible regardless of spell-opacity settings.

The pinned open reference also exposes source categories equivalent to:

```text
DEFAULT
OWN
OTHER_PLAYER
MONSTER
BOSS
```

and applies source-specific effect alpha.

### Oteryn conclusion

The VFX system should preserve enough semantic source/context metadata to support source-aware readability policy without encoding that policy in gameplay mechanics.

Candidate presentation source classes may include:

```text
SELF
PARTY_OR_ALLY
OTHER_PLAYER
MONSTER
BOSS
ENVIRONMENT
SYSTEM
```

Exact taxonomy is deferred and must follow product/gameplay evidence.

### Critical override

Some presentation events must be able to declare or resolve to a gameplay-readability class whose minimum visibility cannot be reduced below a safe threshold by decorative-quality or generic spell-opacity settings.

Conceptually:

```text
VfxVisibilityPolicy {
    source_class,
    readability_class,
    user_preference,
    technical_quality,
    critical_minimum
}
```

This is conceptual only.

Required behavior:

```text
boss danger telegraph / mandatory mechanic marker
    -> never fully hidden by generic effect-opacity preference

non-critical other-player decorative spell particles
    -> may be strongly reduced
```

### Authority boundary

`critical visibility` means client presentation priority only.

It does not authorize damage, AoE, target selection or timing. The server/game simulation remains authority for those facts.

## 5. Presentation family, technical quality and VFX visibility are separate axes

Do not collapse these settings into one graphics preset.

Conceptually separate:

```text
PresentationFamily
  Classic / Enhanced / HD / future

TechnicalQuality
  Low / Medium / High / Ultra / Auto

VfxVisibilityPreference
  source/category-specific readability controls
```

Examples that should be supportable:

```text
Classic art + high lighting + reduced other-player spell opacity
HD art + low particles + full boss telegraph visibility
Enhanced art + no decorative weather + full critical VFX
```

Gameplay-significant information must remain equivalent.

## 6. Three independent spatial/rendering scales

The renderer and asset pipeline must explicitly distinguish three independent scale concepts.

### 6.1 Logical world scale

```text
World tile / semantic footprint
```

This belongs to world/game semantics and is independent from artwork pixel density.

### 6.2 Source/presentation density

Examples:

```text
32-class
64-class
128-class
future density
```

This controls source detail/resource size, not gameplay footprint.

### 6.3 Screen projection scale

```text
screen pixels per logical tile
```

This changes with camera zoom, window size, DPI and presentation policy.

### Required invariant

A 128x128 high-density asset for a one-tile logical appearance does not become four times larger in world space simply because it contains four times the source resolution of a 32x32 asset.

The prior #465 bake-off already followed this principle by keeping logical rendered sprite size fixed while changing source density.

### Prototype measurements

The World + VFX prototype must distinguish measurements for:

- resource density change at fixed logical view;
- zoom change at fixed source density;
- DPI/window change where supported;
- combined zoom + density changes.

This avoids falsely attributing fill-rate, texture bandwidth or visible-entity differences to the wrong axis.

## 7. Shared animation-program evaluation should be a benchmark candidate

The Game-owned `animated-appearances-v1` product deduplicates animation definitions into content-addressed animation programs. World placements may reference an `animation_program_id` rather than duplicating phase arrays.

The exact 15.32 census includes substantial repeated animation workload:

```text
animated object appearances: 5,190
animated outfit frame groups: 1,732
synchronized object appearances: 1,931
```

### Candidate runtime direction

Benchmark a model conceptually equivalent to:

```text
AnimationProgram
    shared phase/timing metadata

AnimationInstance
    program reference
    minimal local phase/start/seed state only where required
```

Synchronized programs should be able to evaluate from a shared presentation clock or equivalent deterministic shared state instead of maintaining one independent timer object per placed tile solely because it is animated.

Random-start/non-synchronized programs may require bounded per-instance state.

### Evidence requirement

Compare at least:

- per-instance independent timer/update baseline;
- shared program + minimal instance-state evaluation;
- synchronized mass-world-animation scene.

Measure CPU preparation cost and memory, not only GPU frame time.

No exact animation scheduler/data layout is accepted by this addendum.

## 8. Texture arrays require real pagination evidence

The exact 15.32 appearance profile currently qualifies only four decoded sprite cell geometries:

```text
32x32
32x64
64x32
64x64
```

This makes `texture arrays partitioned by geometry class` a credible challenger to atlas packing.

However, the corpus contains tens of thousands of unique visible sprite identities. An array design therefore cannot be evaluated as though one array necessarily contains the whole corpus.

`wgpu` exposes `max_texture_array_layers` as an adapter limit. Its portable default limit is 256 layers; a physical adapter may expose a higher value, but the prototype must query and record the actual supported limit rather than assume it.

### Required candidates

The asset/runtime benchmark should compare at least:

1. atlas/page approach;
2. geometry-class texture arrays with realistic multiple-page switching;
3. justified hybrid if it reduces state breaks/resource waste.

### Required measurements

Record:

- physical adapter `max_texture_array_layers` and relevant texture/binding limits;
- number of array pages required by the tested working set;
- page/resource state breaks;
- batch fragmentation;
- upload and eviction cost;
- wasted texture memory from padding/unoccupied layers;
- mip/filter/compression interactions where applicable;
- performance during camera-driven working-set churn.

No array/atlas verdict is valid if it tests only one small array or one small synthetic atlas while the real working set requires multiple pages.

## 9. Final additions to the World + VFX completion gate

In addition to the parent deep-audit criteria, prototype completion now also requires:

- multi-floor screen-projection parity with native Oteryn floor direction explicitly tested;
- no raw appearance gameplay-like flag becoming renderer-owned collision/LoS/movement authority;
- source-aware VFX visibility policy demonstrated with at least self/other/monster/boss or justified equivalent categories;
- at least one critical gameplay VFX that remains visible when ordinary spell opacity/quality is minimized;
- logical tile scale, source density and screen zoom/DPI treated as separate variables;
- shared-animation-program evaluation included in CPU/memory evidence;
- atlas-vs-array benchmark exercising realistic multi-page behavior and recording adapter array-layer limits.

## 10. Addendum disposition

These findings strengthen, but do not replace, the parent deep audit.

They do **not** change the bounded renderer-foundation direction:

```text
project-owned Rust + wgpu
```

They do **not** freeze:

```text
KTX2 vs DDS
atlas vs arrays vs hybrid
streaming/cache topology
final animation scheduler
final VFX visibility taxonomy
final lighting implementation
final UI
compression/mip/filter policy
```

Those remain evidence outputs of `OTERYN WORLD + VFX PROTOTYPE`.
