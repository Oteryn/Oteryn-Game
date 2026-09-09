# Molehill final physical evidence — 2026-09-09

Status: **FINAL PHYSICAL ACCEPTANCE EVIDENCE FOR ISSUE #480**

Measurement commit: `280afa3355d4ab7a69ce4c8ebcc6f8f5b3fc1de8`
Host: `Molehill-PC`
GPU/backend: AMD Radeon RX 9070 XT / DX12 / `wgpu 30.0.0`
CPU: AMD Ryzen 7 9800X3D
Toolchain: Rust/Cargo 1.94.0
Surface: `Bgra8UnormSrgb` / `Immediate`

This directory is the final physical qualification set for the isolated, non-production `OTERYN WORLD + VFX PROTOTYPE`. No production renderer/client/server/protocol/content/deployment authority is granted by this evidence.

## Matrix

The primary matrix fixes `PresentationFamily=Enhanced` and independently varies:

- scenarios: BASIC / NORMAL / STRESS;
- presentation density: 32 / 64 / 128;
- resource layout: atlas / texture array / simple hybrid.

It contains **81 primary records**: 27 cells x 3 repetitions, each with **120 warm-up + 600 measured frames**. A separate **3-run family smoke** holds density/layout constant and varies Classic / Enhanced / HD.

All 84 physical runs report reliable GPU timestamps on the RX 9070 XT. The primary matrix has **zero overflow fallbacks**, zero surface timeout/occluded/outdated/lost events and zero device-loss detections.

## Cache-capacity calibration

Before freezing this final matrix, the corrected deterministic workload measured simultaneous active-page maxima of:

| Scenario | Atlas/array max | Atlas/array cap | Hybrid max | Hybrid cap |
|---|---:|---:|---:|---:|
| BASIC | 30 | 32 | 50 | 64 |
| NORMAL | 55 | 64 | 83 | 96 |
| STRESS | 96 | 112 | 164 | 176 |

The caps are above simultaneous demand but below the whole moving-camera working set. This preserves real LRU upload/eviction churn while eliminating capacity-overflow fallback as a confounder. Every final cell records both uploads and evictions.

## Representative STRESS medians

| Density | Layout | CPU p95 ms | GPU p95 ms | Mean FPS | Mean batches | Peak RAM median MiB |
|---:|---|---:|---:|---:|---:|---:|
| 32 | atlas | 14.8007 | 0.51268 | 94.4 | 4,676 | 240.6 |
| 32 | array | 15.1857 | 0.55252 | 93.2 | 4,676 | 243.8 |
| 32 | hybrid | 25.1227 | 1.58604 | 53.4 | 12,628 | 374.9 |
| 64 | atlas | 15.3688 | 0.59072 | 92.3 | 4,676 | 497.3 |
| 64 | array | 15.7774 | 0.61328 | 92.5 | 4,676 | 499.3 |
| 64 | hybrid | 25.3467 | 1.60328 | 53.0 | 12,628 | 631.7 |
| 128 | atlas | 15.8775 | 1.11728 | 88.4 | 4,676 | 1,151.4 |
| 128 | array | 17.1291 | 1.14988 | 82.1 | 4,676 | 1,154.7 |
| 128 | hybrid | 26.0244 | 3.27948 | 51.1 | 12,628 | 1,929.4 |

`Peak RAM median` is the median of per-run process peak working set. The maximum observed process peak in the entire raw matrix is approximately **2,952.8 MiB**. No trustworthy per-process VRAM telemetry was available, so deterministic texture allocation is not relabeled as VRAM.

## Decisions

- **Atlas vs texture arrays — `INSUFFICIENT_EVIDENCE`.** Atlas has lower GPU p95 in 8/9 cells and lower CPU p95 in 7/9, but neither layout satisfies the predeclared decisive >5% CPU+GPU win rule. Do not freeze either physical organization yet.
- **Current simple hybrid — `REJECT`.** It is >25% slower than the better atlas/array candidate on both CPU and GPU in all 9 scenario/density comparisons, with about 2.7x the order-preserving batch count in STRESS. This rejects this exact split layout, not every possible future compiler-repacked hybrid.
- **Streaming/cache direction — `ADOPT bounded_visible_working_set_with_eviction`.** Multi-page camera-driven churn completed across the matrix with uploads/evictions, zero overflow fallback and zero renderer/surface/device failure.
- **KTX2 vs DDS/other container, particle backend, light/VFX hard budgets, exact batching thresholds, RAM/VRAM production budgets and final filtering/mipmap policy — `INSUFFICIENT_EVIDENCE`.** These remain explicitly open.

The 50,000-instance animation micro-evidence also produces the same phase checksum for independent and shared-program evaluation. Shared animation programs reduce deterministic state from **5,200,000 B to 800,416 B (84.6%)**; median evaluation time in this matrix is **6.7876 ms shared vs 7.4364 ms independent**. This supports shared animation-program ownership as a presentation-data direction, without freezing a production animation ABI.

## Family equivalence

Classic / Enhanced / HD family smoke resolves to one gameplay signature:

`a9503cf44e21c9b2`

Therefore presentation family changes presentation complexity only; the evidence harness no longer advances gameplay semantics according to wall-clock render speed.

## Source provenance

The locally authorized `15.32.zip` was hash-verified without committing proprietary bytes:

- size: **246,811,594 B**;
- SHA-256: `1a6bad8b7598cd874f534cd4aae2d249fb3d9b4458b3ccfa75754f91bb27870f`;
- expected SHA-256: identical;
- match: **true**.

`asset-proof.json` intentionally omits the local filesystem path.

## Files

- `raw.jsonl` — 81 primary physical records;
- `family-smoke.jsonl` — Classic/Enhanced/HD equivalence runs;
- `summary.json` / `SUMMARY.md` — deterministic analyzer output;
- `hardware.json` — named hardware/OS/power metadata;
- `asset-proof.json` — public-safe exact-source digest proof;
- `verification.json` — machine-readable acceptance checks and content digests;
- `stderr.log` — 84 pipeline-prewarm diagnostics; no error/panic/device-lost/timeout lines;
- `HASHES.sha256` — SHA-256 for durable evidence files.

`IMPLEMENTATION_AUTHORITY: NON_PRODUCTION_EXPERIMENT_ONLY`
`RUNTIME_ACTIVATION_AUTHORITY: NONE`
