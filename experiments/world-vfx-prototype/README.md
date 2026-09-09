# Oteryn World + VFX Prototype

Issue: #480. This directory is an isolated non-production Rust + `wgpu` evidence harness. It does not link or mutate the production client, renderer crate, server, protocol, persistence, content authority, Platform, Atlas, deployment, or root Cargo workspace.

## Authority boundary

The prototype consumes semantic world/presentation inputs and keeps physical GPU identity downstream of those semantics. Visual projectile position, VFX lifetime, floor visibility, cache state, texture coordinates, resource pages, bind groups, pipelines and quality degradation never authorize gameplay.

The model exposes a semantic `RenderSnapshot`, `PresentationEvent` stream and `EnvironmentState`. It exercises native multi-floor projection, deterministic Tibia-like stack classes, roof/weather exposure, elevation and visual overhang, layered creatures, event dedup/resync, movement interpolation, projectile A→B, area/hit/heal VFX, a critical boss telegraph, particles, decals, trails, local lights, day/night, rain, snow, fog/wind, winter appearance and screen-space overlays.

## Independent presentation axes

`PresentationFamily` (`classic`, `enhanced`, `hd`) is independent from source/presentation density (`32`, `64`, `128`) and from resource organization (`atlas`, `array`, `hybrid`). The final primary matrix fixes the family to Enhanced while changing density and resource organization; a separate fixed-density family smoke proves equivalent gameplay signatures for Classic/Enhanced/HD.

Classic uses pixel-stable nearest sampling in this evidence harness; Enhanced/HD use linear sampling. That does not freeze final filtering or mip policy.

## Asset and corpus discipline

The protected Game-owned 15.32 census supplies workload shape and provenance. No proprietary Tibia pixels are committed. A locally authorized asset ZIP may be verified by SHA-256 and exercised without entering Git; result records only evidence metadata.

For the real-world qualification path, `tools/prepare-atlas-slice.py` reads only the pinned Atlas FullWorld publication and creature shards, verifies the protected publication/semantic/pixel/runtime roots plus each fetched semantic range/creature shard digest, and writes a bounded semantic replay under `target/`. The replay carries real tile positions, presentation order, appearance/sprite provenance, decoded 32/64-unit geometry/displacement, real spawn positions and verified creature animation timing. It never commits source pixel bytes.

Real FullWorld static semantics and creature presentation templates are combined with synthetic presentation-only movement, combat VFX and environment transitions. Those synthetic events do not claim server movement/combat/weather authority. Physical GPU page IDs remain candidate cache state derived downstream from semantic source IDs, never gameplay identity.

## Evidence and metrics

The final Molehill matrix measures BASIC/NORMAL/STRESS × 32/64/128 × atlas/array/hybrid with repeated runs on the named RX 9070 XT. It records CPU p50/p95/p99, reliable GPU timestamp p50/p95/p99 when supported, mean throughput/FPS, host peak working set, visible primitives, order-preserving batches, upload/cache/eviction activity, scroll/zoom/floor-transition frame tails, first-frame and pipeline-prewarm cost, environment/readability counters and explicit surface-failure counters.

VRAM remains `INSUFFICIENT_EVIDENCE` unless a trustworthy per-process counter is available. Estimated cache GPU bytes are not promoted to measured VRAM.

The harness also compares 50,000 animation instances using independent cloned timer/program state versus shared animation programs with minimal per-instance references. CPU evaluation time and actual allocated state bytes are reported; both paths must produce the same phase checksum.

`analyze-results.py` emits only `ADOPT`, `REJECT`, or `INSUFFICIENT_EVIDENCE` verdicts. Missing or unreliable evidence is never replaced with an estimate.


## Physical qualification lifecycle

The earlier procedural matrix remains provenance/baseline evidence only. Terminal Issue #480 qualification is rerun against a digest-pinned real Atlas FullWorld semantic slice so that static world density, presentation ordering, decoded geometry/displacement and creature presentation templates come from the real publication while dynamic movement/VFX/environment events remain explicitly synthetic presentation workload.

The final evidence directory must bind one exact code head, the generated semantic-slice SHA/root manifest, 81 primary runs and 3 independent-family smoke runs. `ADOPT`/`REJECT` decisions are valid only after that real-world matrix passes reliability, GPU timestamp, cache-churn, family-equivalence and exact-head checks.

## Final real-Atlas qualification

The first full real-world pass exposed cache-capacity overflow in NORMAL/STRESS. That diagnostic run was not accepted as terminal evidence. Capacities were recalibrated above the measured simultaneous active working set and the full matrix was repeated from exact code head `de1843fd75066ad3671864cbdf030913a36a81f7`.

Final evidence is under `evidence/molehill-real-atlas-final-20260909/`: 81/81 primary runs plus 3/3 Classic/Enhanced/HD family-smoke runs on AMD Radeon RX 9070 XT / DX12. All runs use the digest-pinned 96x96 Atlas FullWorld slice (`20,797` tiles, `30,974` resolved presentation primitives, `112` resolved creature records, slice SHA-256 `a72448fd04481a24fe1d34e284d7e9504738d5c6c169f43fef19077fe2b3186f`).

Terminal checks: reliable GPU timestamps, surface/device reliability, multi-page usage, cache churn in 27/27 primary cells, critical-VFX readability, exact-head consistency, family-axis completeness and gameplay-signature equality all pass; resource overflow fallback is `0` across all 84 physical runs. BASIC CPU/GPU p95 ranges are `1.531-1.832 ms` / `0.079-0.140 ms`; NORMAL `3.224-4.174 ms` / `0.147-0.353 ms`; STRESS `6.007-9.124 ms` / `0.368-0.898 ms`.

Evidence supports `ADOPT` for bounded visible-working-set residency with eviction. Atlas vs texture arrays, the simple hybrid challenger, KTX2 vs DDS, particle backend, production light/VFX limits, batching thresholds, RAM/VRAM budgets and final filtering/mipmap policy remain `INSUFFICIENT_EVIDENCE` under the real-world matrix.
## Validation

```powershell
cargo test --locked
cargo fmt --all --check
cargo clippy --locked --all-targets -- -D warnings
cargo build --locked --release
.\run-matrix.ps1 -Repetitions 3 -Warmup 120 -Frames 600 `
  -EvidenceDir evidence/molehill-real-atlas-final-20260909 `
  -AtlasOrigin http://192.168.1.2:8097
```

`IMPLEMENTATION_AUTHORITY: NON_PRODUCTION_EXPERIMENT_ONLY`

`RUNTIME_ACTIVATION_AUTHORITY: NONE`

`MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`
