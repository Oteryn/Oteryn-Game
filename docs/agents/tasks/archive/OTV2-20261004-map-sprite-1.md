# OTV2-20261004-map-sprite-1

```yaml
task_id: OTV2-20261004-map-sprite-1
title: MAP-SPRITE-1 - the 15.30 appearance and sprite pipeline
mode: IMPLEMENT
status: frozen
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/map-sprite-1-20261004
decision: ARCH-MAP-WIRE-1 section 2.2 (docs/architecture/reviews/OTERYN_GAME_ARCH_MAP_WIRE_PACKETS_2026-10-04.md)
pr: null
owned_paths:
  - crates/client-assets/**
  - crates/renderer/src/batch.rs
  - crates/renderer/src/scene_gpu.rs
  - crates/renderer/src/lib.rs
  - Cargo.toml
  - Cargo.lock
  - docs/agents/tasks/archive/OTV2-20261004-map-sprite-1.md
  - workspace-boundaries.toml  # only the oteryn-client-assets entries (members, paths, production, edges); CP D635 option a
```

Measured values:

- `MAX_ENTRY_CELLS`: 16. Over all 43,516 pinned appearances the widest pattern (all layers, phase 0) is 4 cells (e.g. id 31055), so 16 leaves 4x headroom and matches the packet's 2,016 x 10 x 16 budget. A test asserts none exceeds it.
- Sheet decode (manifest sha256 + LZMA + BMP to RGBA, one 384x384 sheet): about 20 ms warm and about 110 ms cold in a release build; the test bound is 10 s to cover unoptimised builds.
- Appearance 104 is a ground with a 4x4 pattern of one 32x32 cell each: 16 patterns, not a 16-cell object. One cell is drawn per tile; the test asserts 16 distinct sprites over the 4x4 positions.
- Renderer: `MAX_BATCH_QUADS` is 81,920; `AtlasPage` (2048x2048, 4096 cells, plus one placeholder row in the texture) evicts least-recently-drawn cells; `QuadBatches` splits at the cap; `SpriteFrame` takes the entry cell cap as a parameter so the renderer does not depend on client-assets.

Limits and notes:

- `scene_gpu.rs` is Windows-only and could not be built or run here (no Windows target). It gained a `write_cell` sub-rectangle upload; `windows.rs` is not owned, so wiring `SpriteFrame` and multi-batch draws into the frame loop is left to its owner.
- `workspace-boundaries.toml` gained only the `oteryn-client-assets` entries (CP D635 option a).
