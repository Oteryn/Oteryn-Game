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
## Evidence and metrics

The final Molehill matrix measures BASIC/NORMAL/STRESS × 32/64/128 × atlas/array/hybrid with repeated runs on the named RX 9070 XT. It records CPU p50/p95/p99, reliable GPU timestamp p50/p95/p99 when supported, mean throughput/FPS, host peak working set, visible primitives, order-preserving batches, upload/cache/eviction activity, scroll/zoom/floor-transition frame tails, first-frame and pipeline-prewarm cost, environment/readability counters and explicit surface-failure counters.

VRAM remains `INSUFFICIENT_EVIDENCE` unless a trustworthy per-process counter is available. Estimated cache GPU bytes are not promoted to measured VRAM.

The harness also compares 50,000 animation instances using independent cloned timer/program state versus shared animation programs with minimal per-instance references. CPU evaluation time and actual allocated state bytes are reported; both paths must produce the same phase checksum.

`analyze-results.py` emits only `ADOPT`, `REJECT`, or `INSUFFICIENT_EVIDENCE` verdicts. Missing or unreliable evidence is never replaced with an estimate.


## Final physical qualification

Final accepted physical evidence for Issue #480 is under `evidence/mollehill-final-20260909/` and was measured on `280afa3355d4ab7a69ce4c8ebcc6f8f5b3fc1de8`. The set contains 81 primary runs plus 3 independent-family smoke runs with reliable RX 9070 XT GPU timestamps, zero primary cache-overflow fallbacks, zero surface/device failures and equivalent Classic/Enhanced/HD gameplay signatures.

Bounded decisions from this gate are: keep atlas vs texture arrays open (`INSUFFICIENT_EVIDENCE`), reject the exact simple hybrid tested here (`REJECT`), and adopt bounded visible-working-set residency with eviction as the streaming/cache direction (`ADOPT`). KTX2/DDS, final filtering/mips, particle backend, hard VFX/light limits and production RAM/VRAM budgets remain evidence-gated.

## Validation

```powershell
cargo test --locked
cargo fmt --all --check
cargo clippy --locked --all-targets -- -D warnings
cargo build --locked --release
.\run-matrix.ps1 -Repetitions 3 -Warmup 120 -Frames 600
```

`IMPLEMENTATION_AUTHORITY: NON_PRODUCTION_EXPERIMENT_ONLY`

`RUNTIME_ACTIVATION_AUTHORITY: NONE`

`MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`
