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

Full reproducible evidence run from repository root:

```powershell
python tools/content-format-spike/spike.py `
  --work-dir C:\Temp\oteryn-content-format-spike\bench-final `
  --results docs\agents\evidence\OTV2-20260824-content-format-spike-results.json `
  --dossier docs\agents\evidence\OTV2-20260824-content-format-spike.md `
  --base-sha 22a3eb866dae19d048969edff1e1fa5012a429b6 `
  --iterations 9
```
