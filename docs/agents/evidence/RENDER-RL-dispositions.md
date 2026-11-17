# RENDER-RL-1 — #502 dispositions

Budget basis: 2026-10-27 floor, 2 GB RAM, 256 MB VRAM,
Vulkan 1.0 / DX12. These are implementation proposals, not owner numeric
acceptance or measured whole-client compatibility. No registry rows are added.

| Row | Disposition | Evidence / floor derivation |
| --- | --- | --- |
| RENDER-RL-01 | 4096 resident cells per sprite page | `PAGE_CELLS`: 64 × 64; 32 × 32 × 4 bytes each = 16 MiB, 1/16 of a 256 MiB VRAM budget. Max/max+1 test covers admission. |
| RENDER-RL-02 | 17,039,360 bytes per sprite texture | `AtlasPage::texture_size`: 2048 × 2080 × 4, including the placeholder row. Actual implementation uses 2080, not 2112. About 6.35% of 256 MiB; same payload in RAM if mirrored. |
| RENDER-RL-03 | 4096 new cells per frame | `MAX_PAGE_UPLOAD_CELLS`, enforced before allocation or eviction; existing residents need no upload. Overflow returns Placeholder and can retry next frame. |
| RENDER-RL-04 | 16,777,216 upload bytes per frame | `MAX_PAGE_UPLOAD_BYTES`: 4096 × 32 × 32 × 4; 1/16 of 256 MiB, 1/128 of 2 GiB. Fixed cell sizes make the cell and byte limits coincide. |
| RENDER-RL-05 | 0 retained atlas slots after generation change | `AtlasPage::set_generation` discards slots and index and resets admission counters; generation-change test. Owners must call this boundary or replace the page on generation change. |
| RENDER-RL-06 | EXCLUDED_FAIL_CLOSED | Unused generic `ResourceCache` and its public export removed; repository search found no consumers. No unbounded generic cache API remains. |
| RENDER-RL-07 | EXCLUDED_FAIL_CLOSED | Aggregate visible quad storage is not bounded by the per-batch cap: `QuadBatches` can allocate multiple batches. No whole-renderer RAM acceptance claimed. Requires a separately scoped aggregate admission decision. |
| RENDER-RL-08 | EXCLUDED_FAIL_CLOSED | No GPU/floor hardware measurement or aggregate VRAM accounting established by GPU-free library tests. Vulkan 1.0 / DX12 floor qualification remains unaccepted. |

The packet supplies no canonical row-to-resource mapping beyond RL-01/02;
RL-03..08 above explicitly name the resources assessed and must be reconciled
with the owner's row definitions before registration. EXCLUDED_FAIL_CLOSED is
an exclusion from acceptance, not a claim that unrelated runtime paths are disabled.

TODO #502: obtain owner numeric acceptance and canonical row mapping before
allocating `docs/contracts/resource-limits/entries/<next>-RENDER-RL-*.json`.

Validation: `cargo test -p oteryn-renderer --lib`: 42 passed, 0 failed.
Red run: generation-release test failed to compile before `set_generation`
was implemented. Existing capacity tests remain strict. PG not applicable.
