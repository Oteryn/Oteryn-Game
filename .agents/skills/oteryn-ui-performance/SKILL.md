---
name: oteryn-ui-performance
description: Optional measurement-led investigation of Oteryn native Rust/wgpu UI regressions or requested profiling, only after explicit user authorization to use this skill or select relevant optional skills for the current task. Not for routine feature implementation or visual redesign.
---

# Oteryn native UI performance

Check current-task authorization in root AGENTS.md. Without a grant, work normally without this optional procedure. Required tests remain required.

1. Identify the reproducible interaction, scene, resolution, DPI, backend and requested metric. Read the owning client/UI architecture and existing benchmark instructions.
2. Use the real client and existing representative harness. Keep CPU frame work, GPU duration and input-to-presentation latency separate; state which measurement is available.
3. Record revision, build profile, environment, workload, warmup and raw repeated measurements before changing code. Measure production-equivalent builds and distinguish cold startup from steady state.
4. Trace the measured cost to its owner: input, view models, layout, text, resource uploads or rendering. Source inspection alone supports a hypothesis, not a measured bottleneck.
5. If fixes are in scope, change the smallest demonstrated cause and preserve gameplay authority, retained UI state, clipping, docking, ordering and resource lifetime. An audit request does not authorize implementation.
6. Compare equivalent before/after workloads, including slow frames, variability and correctness. An inconclusive result is not a speedup. Report missing GPU or native execution evidence explicitly.

Prefer existing tooling; do not introduce a profiling framework, threads, unsafe code or speculative caches without evidence. Apply the repository's playable-first policy.

This is an Oteryn-owned procedure (MPL-2.0), not a vendored or installed copy of Swoorup's Rux-specific skill. The external reference is recorded in the catalog; do not load Rux paths, scripts or assumptions into Oteryn.
