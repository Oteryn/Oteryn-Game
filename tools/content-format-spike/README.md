# OTV2 Content Format Spike tooling

Evidence-only prototypes for Issue #95. This directory is deliberately isolated from production runtime and does not define the permanent World Project or World Bundle format.

Invariant:

```text
SPIKE_RESULT != OWNER_FORMAT_DECISION
```

Candidates:
- `chunked-json-tree`: pretty, canonical per-chunk editable project prototype;
- `sqlite-project`: single-file transactional-container candidate;
- `indexed-zlib-bundle`: read-only indexed per-chunk zlib runtime-bundle prototype.

Focused verification:

```powershell
python tools/content-format-spike/self_test.py
```

## Content/World D3 real-batch measurement

D3 adds a separate opt-in measurement path over the accepted Game-owned fullworld
producer and CW2 typed/pre-promotion source-batch adapter. It does not replace or
reinterpret the historical Issue #95 synthetic benchmark.

The D3 path measures exactly two runtime carriers with one logical/index model:

- integrity-checked uncompressed baseline;
- the same logical/index structure with bounded zlib level-6 compression.

The real input remains `MIGRATION_EVIDENCE / OTS_HYPOTHESIS_ONLY`. Source
occurrences without an accepted `SourceIdentityBinding` remain explicitly
unresolved; source appearance IDs are provenance only. Target coordinates,
collision, ordered placement, footprints and other target-sensitive truth remain
`DEFERRED_REQUIRES_PHASE_B`.

The real run requires the exact admitted source profile and explicit local paths:

```powershell
python tools/content-format-spike/spike.py --d3-real-batch `
  --game-root . --legacy-root <exact-parser-checkout> --map <fresh-world.otbm> `
  --asset-zip <15.32.zip> --assets <extracted-assets> `
  --source-generation-profile-id oteryn-crystalserver-fresh-source-generation-v2 `
  --work-dir <scratch> --results <evidence.json> --dossier <evidence.md> `
  --base-sha <exact-game-head> --iterations 9
```

Measured chunk dimensions and harness limits are evidence configuration only, not
production hard maxima. `SPIKE_RESULT != OWNER_FORMAT_DECISION` remains binding.

Full reproducible evidence run from repository root:

```powershell
python tools/content-format-spike/spike.py `
  --work-dir C:\Temp\oteryn-content-format-spike\bench-final `
  --results docs\agents\evidence\OTV2-20260824-content-format-spike-results.json `
  --dossier docs\agents\evidence\OTV2-20260824-content-format-spike.md `
  --base-sha 22a3eb866dae19d048969edff1e1fa5012a429b6 `
  --iterations 9
```
