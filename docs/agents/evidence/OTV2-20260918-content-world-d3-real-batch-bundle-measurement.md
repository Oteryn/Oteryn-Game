# Content/World D3 real-batch bundle measurement

- Exact Game measurement head: `6ba0e9964f3e1182f4bf6da12e478ee3bfa70f88`
- Source: `zimbadev/crystalserver@ff7ede593c69d4c658b382c97443e8155926924a`
- Fresh source profile: `oteryn-crystalserver-fresh-source-generation-v2` revision `2`
- world.otbm SHA-256: `09cce62af6c86644b5579fba460c674585261eb987ca5aa1f52baef9e91f8bbb`
- Parser: `blakinio/Otheryn@e417c5e7c22986bf4acef0495eb47f7b72c97cce`
- Classification: **MIGRATION_EVIDENCE / OTS_HYPOTHESIS_ONLY**
- Spike invariant: **`SPIKE_RESULT != OWNER_FORMAT_DECISION`**
- Authority: measurement evidence only; production authority and Reference parity remain NONE.

## Fresh real input

- newhaven: 1024 source cells, 1199 source occurrences, ordered stream SHA-256 `bebce3eca44dfcf3ab4044fd7a4a66bf6c1f7a80e2b0d375de14c3eb3898faac`.
- targuna: 1024 source cells, 1264 source occurrences, ordered stream SHA-256 `5e9a498a37570662f57d24ec8ca020c521ccf56091b03e33970aa6e2a1394438`.
- Typed CW2/pre-promotion batch: 1321309 bytes, SHA-256 `4838be7abf70a390b61f4a7578626c1a0b61725c1313e2046606110065d47ec7`.
- Typed counts: `{"bound_definitions": 0, "bound_placements": 0, "source_occurrences": 2463, "source_tile_records": 2048, "unresolved_source_occurrences": 2463}`.
- D3 normalized logical input: 1953208 bytes, SHA-256 `6f2df596271d58c08c26c2d3a65be10a48f41704bdbaa723228afea73b90b2fa`.
- Source cells: 2048; source definitions: 183; source placements: 2463; max source placements/cell: 5.
- Encoded record maxima: cell 3737 B; placement 738 B; source definition 162 B.

No canonical target identity, target coordinates, collision, order or footprint truth is inferred by this measurement. When CW2 has no accepted SourceIdentityBinding, the carrier records the real source occurrence as unresolved and the appearance ID as source provenance only.

## Two measured runtime carriers

| Carrier | Compression | Artifact B | Raw chunk B | Stored chunk B | Ratio raw/stored | Build ms | Cell ms | Placement ms | Definition ms | Build peak B | Load peak B | Patch B | Changed units |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `indexed-uncompressed-baseline` | none | 2106802 | 1916220 | 1916220 | 1.0 | 680.778 | 75.678 | 74.719 | 18.362 | 8746522 | 5510030 | 1172666 | 2 |
| `indexed-zlib` | zlib level 6 | 265659 | 1916220 | 75079 | 25.522716 | 703.895 | 74.854 | 74.611 | 18.344 | 8885014 | 5545621 | 229427 | 2 |

Both carrier types use the same logical/index structure. The second changes only bounded payload compression to zlib level 6.

## Determinism and locality

- Source enumeration reorder preserves the typed/logical batch: **PASS**.
- Source-shard metadata does not alter logical identity: **PASS**.
- Rechunking preserves logical identity: **PASS**.
- Both physical carriers have the same logical index signature: **PASS**.
- Independent repeated builds are byte-identical per carrier: **PASS**.
- Random-access probes resolve one source cell, one source occurrence and one source-definition reference by index without decoding every chunk.
- The one-record update probe mutates one source-role field only in scratch output; it measures physical rebuild locality and is not claimed as source or gameplay truth.

## Fail-closed evidence

- decompression_ratio_rejected: **PASS**
- malformed_placement_index_rejected: **PASS**
- oversized_raw_chunk_rejected: **PASS**
- unknown_critical_feature_rejected: **PASS**
- wrong_carrier_version_rejected: **PASS**
- wrong_source_profile_rejected: **PASS**
- indexed-uncompressed-baseline: corruption=PASS, truncation=PASS, server/client allowlist=PASS.
- indexed-zlib: corruption=PASS, truncation=PASS, server/client allowlist=PASS.

## Boundary / disposition

- The measured 32-cell chunk dimension is a bounded evidence configuration, not a production hard maximum.
- The 64 MiB spike artifact fence and 2 MiB raw-chunk fence are harness safety limits, not selected production resource maxima.
- zlib is the single mature compression candidate measured here; no serializer/compressor zoo was introduced.
- No Phase-B target parity was performed. All target-sensitive facts remain `DEFERRED_REQUIRES_PHASE_B`.
- No production registry, contract, runtime, client, CW2/CW3 implementation or source-profile file is changed.
- `SPIKE_RESULT != OWNER_FORMAT_DECISION`: this result does not select the permanent World Project/World Bundle format.

## Result

D3 now has reproducible physical-layout/resource evidence for the fresh exact CrystalServer source generation. The evidence can inform a later owner/control-plane format decision, but it does not itself make that decision or grant CW4/production authority.
