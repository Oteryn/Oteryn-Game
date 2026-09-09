# Oteryn World + VFX Prototype â€” physical summary

Primary matrix: **81 runs**, complete=True, fixed Enhanced=True.
Workload=real_atlas_fullworld_slice; real Atlas=True; exact-head consistent=True.
GPU timestamps reliable=True; reliability pass=True; cache churn=True across 27/27 cells; overflow-free=True.
Classic/Enhanced/HD independent family smoke: complete=True, gameplay signatures equal=True.

| Scenario | Density | Mode | CPU p95 ms | GPU p95 ms | FPS | RAM MiB | Batches mean | Evictions | Overflow |
|---|---:|---|---:|---:|---:|---:|---:|---:|---:|
| basic | 32 | array | 1.5664 | 0.0790 | 791.0 | 240.6 | 500.9 | 19 | 0 |
| basic | 32 | atlas | 1.5308 | 0.0842 | 824.1 | 243.7 | 500.9 | 19 | 0 |
| basic | 32 | hybrid | 1.8317 | 0.0870 | 687.7 | 244.7 | 687.1 | 7 | 0 |
| basic | 64 | array | 1.5883 | 0.0956 | 772.8 | 374.9 | 500.9 | 19 | 0 |
| basic | 64 | atlas | 1.5364 | 0.0857 | 772.4 | 374.9 | 500.9 | 19 | 0 |
| basic | 64 | hybrid | 1.8073 | 0.1015 | 662.8 | 375.6 | 687.1 | 7 | 0 |
| basic | 128 | array | 1.6031 | 0.1382 | 664.3 | 767.1 | 500.9 | 19 | 0 |
| basic | 128 | atlas | 1.6084 | 0.1398 | 638.2 | 770.8 | 500.9 | 19 | 0 |
| basic | 128 | hybrid | 1.8304 | 0.1347 | 564.8 | 1027.9 | 687.1 | 7 | 0 |
| normal | 32 | array | 3.2955 | 0.1505 | 379.2 | 247.5 | 1216.0 | 35 | 0 |
| normal | 32 | atlas | 3.2242 | 0.1472 | 383.4 | 246.8 | 1216.0 | 35 | 0 |
| normal | 32 | hybrid | 3.6928 | 0.1717 | 327.8 | 373.0 | 1556.5 | 74 | 0 |
| normal | 64 | array | 3.5867 | 0.1650 | 353.8 | 506.6 | 1216.0 | 35 | 0 |
| normal | 64 | atlas | 3.3358 | 0.1590 | 364.5 | 505.9 | 1216.0 | 35 | 0 |
| normal | 64 | hybrid | 4.1590 | 0.1832 | 307.8 | 633.0 | 1556.5 | 74 | 0 |
| normal | 128 | array | 3.3136 | 0.3400 | 299.6 | 1286.9 | 1216.0 | 35 | 0 |
| normal | 128 | atlas | 3.4847 | 0.3407 | 279.4 | 1285.8 | 1216.0 | 35 | 0 |
| normal | 128 | hybrid | 4.1737 | 0.3533 | 250.7 | 1927.6 | 1556.5 | 74 | 0 |
| stress | 32 | array | 6.1759 | 0.3938 | 196.2 | 380.9 | 2784.0 | 70 | 0 |
| stress | 32 | atlas | 6.0073 | 0.3678 | 201.7 | 374.0 | 2784.0 | 70 | 0 |
| stress | 32 | hybrid | 7.1262 | 0.4349 | 171.4 | 505.5 | 3397.9 | 70 | 0 |
| stress | 64 | array | 6.7205 | 0.4336 | 189.1 | 763.2 | 2784.0 | 70 | 0 |
| stress | 64 | atlas | 6.1579 | 0.4152 | 192.9 | 761.4 | 2784.0 | 70 | 0 |
| stress | 64 | hybrid | 7.0791 | 0.4822 | 166.8 | 1024.4 | 3397.9 | 70 | 0 |
| stress | 128 | array | 7.5969 | 0.8657 | 152.0 | 2572.0 | 2784.0 | 70 | 0 |
| stress | 128 | atlas | 6.8549 | 0.8487 | 153.9 | 2568.5 | 2784.0 | 70 | 0 |
| stress | 128 | hybrid | 9.1242 | 0.8982 | 134.5 | 3596.3 | 3397.9 | 70 | 0 |

## Verdicts

- **atlas_vs_texture_arrays**: `INSUFFICIENT_EVIDENCE`
  - hybrid challenger: `INSUFFICIENT_EVIDENCE`
- **ktx2_vs_dds_or_other_runtime_container**: `INSUFFICIENT_EVIDENCE`
  - decoded GPU page layout was measured; container IO/decode/transcode challengers were not
- **streaming_cache_model**: `ADOPT` â€” bounded_visible_working_set_with_eviction
  - real Atlas FullWorld viewport semantics drove multi-page upload/eviction churn without renderer failure or resource overflow
- **particle_implementation_direction**: `INSUFFICIENT_EVIDENCE`
  - CPU-prepared instanced particles were exercised without a compute/GPU-simulation challenger
- **light_vfx_budgets**: `INSUFFICIENT_EVIDENCE`
  - physical cost/readability counts exist but no accepted cross-platform product frame budget exists
- **renderer_batching_thresholds**: `INSUFFICIENT_EVIDENCE`
  - order-preserving batching was measured without a threshold challenger matrix
- **ram_budget**: `INSUFFICIENT_EVIDENCE`
  - Molehill process working-set evidence alone cannot freeze a production cross-platform RAM budget
- **vram_budget**: `INSUFFICIENT_EVIDENCE`
  - no trustworthy per-process VRAM counter was available
- **filtering_mipmap_direction**: `INSUFFICIENT_EVIDENCE`
  - Classic nearest and Enhanced/HD linear sampling are exercised but no mip/filter challenger matrix is complete
