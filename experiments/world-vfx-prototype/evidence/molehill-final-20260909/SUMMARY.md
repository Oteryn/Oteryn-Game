# Oteryn World + VFX Prototype — physical summary

Primary matrix: **81 runs**, complete=True, fixed Enhanced=True.
GPU timestamps reliable=True; reliability pass=True; cache churn=True.
Classic/Enhanced/HD independent family smoke: complete=True, gameplay signatures equal=True.

| Scenario | Density | Mode | CPU p95 ms | GPU p95 ms | FPS | RAM MiB | Batches mean | Evictions | Overflow |
|---|---:|---|---:|---:|---:|---:|---:|---:|---:|
| basic | 32 | array | 2.3595 | 0.0746 | 585.3 | 230.3 | 661.8 | 29 | 0 |
| basic | 32 | atlas | 2.1937 | 0.0729 | 595.6 | 229.9 | 661.8 | 29 | 0 |
| basic | 32 | hybrid | 3.1754 | 0.1593 | 422.5 | 234.4 | 1337.9 | 37 | 0 |
| basic | 64 | array | 2.8226 | 0.0777 | 552.9 | 364.7 | 661.8 | 29 | 0 |
| basic | 64 | atlas | 2.4935 | 0.0759 | 564.1 | 364.5 | 661.8 | 29 | 0 |
| basic | 64 | hybrid | 3.5703 | 0.1624 | 387.7 | 365.5 | 1337.9 | 37 | 0 |
| basic | 128 | array | 2.4243 | 0.0994 | 486.3 | 632.9 | 661.8 | 29 | 0 |
| basic | 128 | atlas | 2.5073 | 0.0946 | 458.4 | 632.5 | 661.8 | 29 | 0 |
| basic | 128 | hybrid | 3.4239 | 0.1616 | 337.3 | 889.5 | 1337.9 | 37 | 0 |
| normal | 32 | array | 5.1634 | 0.1875 | 255.9 | 236.2 | 1709.2 | 14 | 0 |
| normal | 32 | atlas | 5.0849 | 0.1820 | 257.7 | 235.6 | 1709.2 | 14 | 0 |
| normal | 32 | hybrid | 7.1380 | 0.4073 | 181.3 | 237.6 | 3445.1 | 32 | 0 |
| normal | 64 | array | 5.1022 | 0.1941 | 254.4 | 367.5 | 1709.2 | 14 | 0 |
| normal | 64 | atlas | 5.1517 | 0.1950 | 260.1 | 366.5 | 1709.2 | 14 | 0 |
| normal | 64 | hybrid | 7.1060 | 0.4244 | 179.0 | 493.3 | 3445.1 | 32 | 0 |
| normal | 128 | array | 5.1333 | 0.2928 | 241.0 | 763.9 | 1709.2 | 14 | 0 |
| normal | 128 | atlas | 5.1030 | 0.2912 | 237.7 | 763.0 | 1709.2 | 14 | 0 |
| normal | 128 | hybrid | 11.5119 | 0.4480 | 159.0 | 1149.3 | 3445.1 | 32 | 0 |
| stress | 32 | array | 15.1857 | 0.5525 | 93.2 | 243.8 | 4676.4 | 8 | 0 |
| stress | 32 | atlas | 14.8007 | 0.5127 | 94.4 | 240.6 | 4676.4 | 8 | 0 |
| stress | 32 | hybrid | 25.1227 | 1.5860 | 53.4 | 374.9 | 12628.1 | 43 | 0 |
| stress | 64 | array | 15.7774 | 0.6133 | 92.5 | 499.3 | 4676.4 | 8 | 0 |
| stress | 64 | atlas | 15.3688 | 0.5907 | 92.3 | 497.3 | 4676.4 | 8 | 0 |
| stress | 64 | hybrid | 25.3467 | 1.6033 | 53.0 | 631.7 | 12628.1 | 43 | 0 |
| stress | 128 | array | 17.1291 | 1.1499 | 82.1 | 1154.7 | 4676.4 | 8 | 0 |
| stress | 128 | atlas | 15.8775 | 1.1173 | 88.4 | 1151.4 | 4676.4 | 8 | 0 |
| stress | 128 | hybrid | 26.0244 | 3.2795 | 51.1 | 1929.4 | 12628.1 | 43 | 0 |

## Verdicts

- **atlas_vs_texture_arrays**: `INSUFFICIENT_EVIDENCE`
  - hybrid challenger: `REJECT`
- **ktx2_vs_dds_or_other_runtime_container**: `INSUFFICIENT_EVIDENCE`
  - decoded GPU page layout was measured; container IO/decode/transcode challengers were not
- **streaming_cache_model**: `ADOPT` — bounded_visible_working_set_with_eviction
  - camera-driven multi-page upload/eviction churn completed without renderer failure
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
