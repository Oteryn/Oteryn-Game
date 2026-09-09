# Oteryn graphics engine bake-off — physical evidence

- Issue: #465
- Date: 2026-09-09
- Runtime evidence head: `11c568639484d6ecc0284239b6f1dfbb1755f5e8`
- Clean-build evidence head: `90e95dcf52926c74a61de33ec0cdb85bf7576ea6`
- Decision: **`ADOPT_CUSTOM_WGPU`**
- Decision scope: renderer foundation for the next production native-client rendering slices.
- Production/runtime activation authority: **NONE** from this experiment alone.

## Decision

Use the project-owned Rust renderer on `wgpu` as Oteryn's native-client rendering foundation. Do not adopt Bevy as the primary client engine/rendering ownership layer on the evidence measured here.

This decision does **not** freeze KTX2, DDS, PNG runtime use, texture arrays, atlas policy, asset streaming, animation schema, final UI technology, or any permanent content/world bundle format. Those remain separately gated decisions.

## Physical environment

- Machine: `Molehill-PC`
- OS: Windows 11 Pro `10.0.26200` build `26200`
- CPU: AMD Ryzen 7 9800X3D, 16 logical processors
- GPU used by both candidates: AMD Radeon RX 9070 XT
- GPU driver: `32.0.31035.1003`
- Backend: DX12
- Power preference: high-performance discrete adapter
- Power scheme: Ultimate Performance
- Isolated benchmark Rust: `rustc 1.95.0 (59807616e 2026-04-14)`

## Workload and sampling

Both candidates consumed the same deterministic benchmark-only `RenderSnapshot`, the same 6x6 / 36-frame synthetic RGBA8 atlas, and the same 1920x1080 window workload. Source densities were 32, 64 and 128 pixels while the logical rendered sprite size remained 32 pixels.

| Scenario | Static quads | Animated quads |
| --- | ---: | ---: |
| BASIC | 10,000 | 50 |
| NORMAL | 25,000 | 200 |
| STRESS | 50,000 | 500 |

Each cell used 180 warm-up frames, 1,200 measured frames and 5 successful repetitions. The final matrix contains 90/90 successful measurements. The hardened run used a 1,000 ms delay between processes and required no retries.

The custom candidate uses one shared atlas, one storage instance buffer, static upload once, per-frame writes only for animated records, and one renderer-owned draw submission. Bevy uses presentation-only sprite entities sharing the same atlas; gameplay/domain ownership was not moved into Bevy ECS.

## Median physical results

| Scenario | px | custom p95 ms | Bevy p95 ms | custom p99 ms | Bevy p99 ms | custom FPS | Bevy FPS | custom RAM MiB | Bevy RAM MiB |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| BASIC | 32 | 0.454 | 1.616 | 0.591 | 1.783 | 4084.9 | 743.4 | 166.0 | 265.5 |
| BASIC | 64 | 0.493 | 1.700 | 0.586 | 1.873 | 3643.5 | 714.4 | 166.2 | 266.0 |
| BASIC | 128 | 0.506 | 1.770 | 0.624 | 1.976 | 3490.2 | 726.6 | 166.5 | 267.9 |
| NORMAL | 32 | 0.502 | 3.130 | 0.604 | 3.916 | 3134.9 | 360.9 | 166.2 | 294.9 |
| NORMAL | 64 | 0.513 | 3.182 | 0.623 | 3.891 | 2985.3 | 359.5 | 166.1 | 294.8 |
| NORMAL | 128 | 0.590 | 3.126 | 0.697 | 3.456 | 2415.0 | 360.4 | 166.1 | 297.2 |
| STRESS | 32 | 0.708 | 6.510 | 0.827 | 7.759 | 1967.0 | 168.9 | 166.7 | 344.1 |
| STRESS | 64 | 0.769 | 6.636 | 0.894 | 8.191 | 1807.9 | 168.7 | 166.7 | 345.6 |
| STRESS | 128 | 0.939 | 6.498 | 1.073 | 7.484 | 1397.4 | 169.2 | 166.7 | 347.1 |

Across the nine cells, Bevy/custom p95 ranges from **3.45x to 9.20x** and p99 from **3.02x to 9.39x**. Custom/Bevy mean-FPS throughput ranges from **4.80x to 11.64x**.

Peak working-set ratio grows with scene size: roughly **1.60x Bevy/custom** in BASIC, **1.77–1.79x** in NORMAL and **2.06–2.08x** in STRESS. Median startup is roughly **1.30–1.38x** longer for Bevy.

## Build and binary evidence

The clean release builds used the same machine, isolated Rust 1.95 toolchain and Cargo lock graph.

| Metric | custom wgpu | Bevy | ratio Bevy/custom |
| --- | ---: | ---: | ---: |
| Clean release build | 23.436 s | 140.405 s | 5.99x |
| Release executable | 5,400,064 B | 55,859,712 B | 10.34x |

The isolated dependency graph resolves direct `wgpu 30.0.0` for the custom candidate and Bevy 0.19.1's own `wgpu 29.0.4` stack. Adopting Bevy today would therefore add a second renderer dependency generation rather than replace the existing direct `wgpu` dependency in-place.

## Reliability observation

An earlier aggressive no-delay matrix observed one Bevy STRESS/32 startup failure in DX12 `ResizeBuffers` / invalid-surface handling after rapid process teardown/restart. The hardened final matrix introduced a 1-second inter-process delay and retry capture; it completed all 90 measurements on attempt 1 with no failure file.
That transient is recorded as teardown/restart sensitivity evidence, not counted as a frame-time result and not generalized into a normal-play failure claim.

## Interpretation

The result favors a specialized Oteryn renderer strongly enough to choose the next implementation direction. The custom path wins every measured workload while also using materially less process memory, building much faster and producing a much smaller executable.

The observed gap becomes larger as sprite count rises, which is directly relevant to a tile/sprite-heavy Oteryn client. Bevy's general engine abstractions remain useful capabilities in other products, but this benchmark does not show enough compensating runtime or integration value to justify making Bevy the primary Oteryn client ownership layer.

## Explicit limitations and follow-up gates

This is not a complete game benchmark. It deliberately does not yet measure:

- real Oteryn floor/stack ordering and occlusion; `z` was neutralized so both candidates paid the same base sprite-throughput workload;
- production UI, text/nameplates, lighting, particles, missiles or post-processing;
- real content streaming, texture eviction or large multi-sheet working sets;
- GPU timestamp-query frame attribution or trustworthy VRAM usage;
- final asset compression/container choices;
- authoring/editor productivity economics.

Before a production renderer is considered feature-complete, separate representative probes must cover stack ordering, real appearance/item workload, UI/text, lighting/effects, texture upload/cache behavior and VRAM budgets. Those probes may refine renderer implementation, but this bake-off does not justify adopting Bevy as the primary engine merely to reach them.

## Preserved evidence

- `evidence/molehill-20260909/raw.jsonl` — all 90 successful physical measurements.
- `evidence/molehill-20260909/summary.json` — deterministic median aggregation.
- `evidence/molehill-20260909/clean-build.json` — clean build and binary-size evidence.
- `evidence/molehill-20260909/physical-run.json` — final run configuration and hardware metadata.
- `evidence/molehill-20260909/adapter-proof.txt` — direct adapter/backend proof for both candidates.
- raw SHA-256: `1a115de0f3b4629a7181d2922a12b1d3725bf0f948f4b4f0bd0d071d5fe72388`
- summary SHA-256: `b10ff724dfac5a659c3a1ef3ce33483a5f2eefcc79eb397823868cd773c1b16b`

No permanent graphics asset format or live product activation is authorized by this decision.
