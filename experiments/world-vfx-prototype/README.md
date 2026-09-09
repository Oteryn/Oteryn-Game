# Oteryn World + VFX Prototype

Issue: #480

This is an isolated, non-production Rust + `wgpu` evidence harness. It does not mutate or link the production client, renderer, server, protocol, persistence, content authority, Platform, Atlas, or root Cargo workspace.

## What it proves

The executable builds a deterministic Tibia-like presentation slice from semantic world/appearance state rather than physical asset paths or GPU identifiers. It exercises:

- native multi-floor projection and a bounded roof/occlusion resolver;
- semantic ground/border/bottom/common/creature/effect/projectile/attached/top/overlay ordering;
- visual coverage/elevation independent from gameplay footprint;
- layered outfit/mount/addon-style creature composition and sub-tile movement interpolation;
- projectile A→B, area spell, physical/elemental hit, persistent VFX and a critical boss telegraph;
- composite VFX using sprite frames, particles, trails, decals and dynamic local lights;
- day/night ambient state, rain, snow, fog/wind and a winter presentation variant;
- source-aware critical/decorative degradation policy with critical presentation retained;
- screen-space name glyphs and HP bars independent from world zoom;
- camera movement, fractional zoom, floor transitions and visibility-set churn;
- multiple GPU resource pages with real upload/cache/eviction churn;
- atlas-page and geometry-compatible texture-array challengers;
- GPU timestamp queries when the physical adapter exposes trustworthy support;
- BASIC/NORMAL/STRESS and 32/64/128 presentation-density classes.

## Semantic boundary

The model owns semantic `AppearanceRef`, `WorldPosition`, presentation class, event identity/timing inputs, environment state and presentation-family selection. The GPU layer owns page IDs, UV/layer resolution, cache residency, bind groups, pipelines, batching, texture uploads and sampling. Visual missile position, VFX lifetime, floor culling and degradation never authorize gameplay state.

Classic / Enhanced / HD use the same gameplay signature. They may change procedural presentation resolution/tint/sampling only. Missing HD variants use a deterministic fallback and are counted.

## Real 15.32 workload shape

The harness can read the protected Game-owned `OTERYN_ATLAS_15_32_ANIMATION_CENSUS_V1.json` at runtime. Only counts/provenance shape workload identity; no proprietary Tibia sprite bytes are committed by this prototype.

Protected corpus anchors currently include 43,514 objects, 1,480 outfits, 243 effects and 76 missiles. Full-world counts remain evidence for working-set scale, not a request to put the whole world in GPU memory.

## Local validation

From this directory:

```powershell
cargo +1.95.0 generate-lockfile
cargo +1.95.0 test --locked
cargo +1.95.0 fmt --all --check
cargo +1.95.0 clippy --locked --all-targets -- -D warnings
cargo +1.95.0 build --locked --release
```

A representative single run:

```powershell
.\target\release\oteryn-world-vfx-prototype.exe `
  --scenario normal --density 64 --layout array --family enhanced `
  --prewarm none --warmup 120 --frames 600 `
  --census ..\..\docs\contracts\OTERYN_ATLAS_15_32_ANIMATION_CENSUS_V1.json
```

The executable emits one JSON record. Physical RAM is added by the host harness; VRAM remains unavailable unless a trustworthy per-process counter is proven.

## Physical matrix

`run-matrix.ps1` runs:

- 3 scenarios × 3 densities × 2 resource layouts × configurable repetitions;
- Classic/Enhanced/HD semantic-equivalence smoke;
- cold vs critical-page-prewarm evidence;
- hardware/power metadata;
- raw JSONL plus an aggregated `summary.json` and technology verdicts.

The analyzer uses only `ADOPT`, `REJECT` or `INSUFFICIENT_EVIDENCE`. It deliberately returns insufficient evidence for questions not actually compared (for example KTX2 vs DDS and CPU particles vs compute particles).

## Authority

`IMPLEMENTATION_AUTHORITY: NON_PRODUCTION_EXPERIMENT_ONLY`

`RUNTIME_ACTIVATION_AUTHORITY: NONE`

`MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`
