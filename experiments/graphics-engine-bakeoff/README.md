# Oteryn graphics engine bake-off

Issue: #465. This directory is a **non-production experiment**. It does not select a production engine, change the Oteryn client runtime, or define a permanent asset format.

## Fairness boundary

Both candidates consume `oteryn-graphics-bakeoff-shared`, which generates the same deterministic benchmark-only `RenderSnapshot` and the same in-memory 6x6 / 36-frame synthetic RGBA8 atlas. The 36-frame shape is used only as a representative animation workload; it is not a claim that every Oteryn creature has 36 frames.

The synthetic atlas is generated at 32, 64 or 128 pixels per source frame. All variants render at the same 32 logical pixels so density changes texture cost rather than world footprint.

## Workloads

| Scenario | Static quads | Animated quads |
| --- | ---: | ---: |
| BASIC | 10,000 | 50 |
| NORMAL | 25,000 | 200 |
| STRESS | 50,000 | 500 |

Default window: 1920x1080. Default run: 180 warm-up frames followed by 900 measured frames. Both candidates request no-vsync presentation with graceful fallback.

## Candidates

- `oteryn-graphics-bakeoff-wgpu`: custom `wgpu 30.0.0` specialization. One shared texture atlas, one storage buffer, static instances uploaded once, only animated instance records rewritten per frame, one renderer-owned draw submission per frame.
- `oteryn-graphics-bakeoff-bevy`: `Bevy 0.19.1` challenger. The same quads become presentation-only Bevy sprite entities sharing one texture atlas; only animated atlas indices change each frame. Bevy ECS is not used as Oteryn gameplay authority.

The two implementations intentionally represent the real architectural choice: specialized renderer code versus a general engine presentation layer. They are not expected to have identical internal data structures.

## Metrics

The executables emit one JSON object at completion with the scenario, density, startup time, frame-time mean/p50/p95/p99, mean FPS, animation-preparation timing and a draw-call value only where the backend itself can state it truthfully. Unsupported counters are `null`, never inferred.

Window-loop frame times are useful throughput evidence but are not a substitute for GPU timestamp or external capture. Final engine selection requires a physical Windows run recording GPU/driver/OS plus external RAM/VRAM/GPU evidence where available. Hosted CI is compile/functional qualification only.

## Commands

From this directory:

```text
cargo +1.95.0 run --release -p oteryn-graphics-bakeoff-wgpu -- --scenario normal --sprite-px 32 --warmup 180 --frames 900
cargo +1.95.0 run --release -p oteryn-graphics-bakeoff-bevy -- --scenario normal --sprite-px 32 --warmup 180 --frames 900
```

Repeat each backend for `basic|normal|stress` and `32|64|128`. Compare candidates on the same machine, power profile, resolution, driver and run parameters. Close background GPU-heavy applications before a counted population.

## Decision rule

The terminal decision vocabulary is:

- `ADOPT_CUSTOM_WGPU`
- `ADOPT_BEVY`
- `INSUFFICIENT_EVIDENCE`

A hosted build, a single run, average FPS alone, or third-party engine benchmark is insufficient to select the production Oteryn engine. No result from this experiment freezes KTX2/DDS, atlas/array policy, asset streaming, or other deferred presentation choices.
