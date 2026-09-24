# Preliminary detached-worker evidence

This directory preserves a completed 81-run physical RX 9070 XT matrix from the earlier detached #480 worker at source base `75dc6dee53df9945a4a89b2acdb91823919c392f`.

It is retained as provenance and challenger evidence, **not** as the final #480 acceptance matrix. The worker completed 81/81 with exit code 0; `stderr.log` contains only pipeline-prewarm timing lines and no renderer failure.

Material limitations that prevent final acceptance:

- presentation family was derived from density (`32→Classic`, `64→Enhanced`, `128→HD`), so family and density were not independent;
- GPU timing was explicitly marked unreliable for all 81 runs;
- per-process VRAM was unavailable;
- surface failure states were not explicitly counted in the result schema.

The matrix remains useful preliminary evidence: atlas/array were close in many cells, while the hybrid challenger showed substantially higher frame cost and overflow fallbacks under STRESS. Final verdicts must be based on the corrected canonical-head matrix with independent axes and reliable GPU timestamps.

No proprietary Tibia asset bytes are present; all preserved raw records have `asset: null`.
