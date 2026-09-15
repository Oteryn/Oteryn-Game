# OTV2 Content Format Spike — Decision Dossier

- Exact worker base: `22a3eb866dae19d048969edff1e1fa5012a429b6`
- Spike invariant: **`SPIKE_RESULT != OWNER_FORMAT_DECISION`**
- Authority: evidence only; permanent World Project / World Bundle format remains owner-gated.

## Reproducibility

- Python: `3.12.0`
- SQLite: `3.42.0`
- zlib: `1.2.13`
- Platform: `Windows-11-10.0.26200-SP0`
- Load iterations per cell: `9`
- Decompression ratio hard fence in spike: `64.0:1`

## Measured evidence

| Candidate | Role | Side | Chunk | Bytes | Build ms | Load ms | Edit units | Patch bytes | Diff lines | Deterministic | Corruption rejected |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---|---|
| `chunked-json-tree` | editable-project | 32 | 32 | 142220 | 28.007 | 11.401 | 2 | 142221 | 4 | yes | yes |
| `sqlite-project` | editable-project | 32 | 32 | 94208 | 14.418 | 11.083 | 1 | 94208 | - | yes | yes |
| `indexed-zlib-bundle` | compiled-runtime | 32 | 32 | 3659 | 14.007 | 9.506 | 1 | 2942 | - | yes | yes |
| `chunked-json-tree` | editable-project | 64 | 32 | 566606 | 120.162 | 10.992 | 2 | 143063 | 4 | yes | yes |
| `sqlite-project` | editable-project | 64 | 32 | 339968 | 69.563 | 12.237 | 1 | 339968 | - | yes | yes |
| `indexed-zlib-bundle` | compiled-runtime | 64 | 32 | 12543 | 65.431 | 10.418 | 1 | 2915 | - | yes | yes |
| `chunked-json-tree` | editable-project | 64 | 64 | 565784 | 136.955 | 40.545 | 2 | 565785 | 4 | yes | yes |
| `sqlite-project` | editable-project | 64 | 64 | 339968 | 85.058 | 38.084 | 1 | 339968 | - | yes | yes |
| `indexed-zlib-bundle` | compiled-runtime | 64 | 64 | 11685 | 61.461 | 35.270 | 1 | 10968 | - | yes | yes |
| `chunked-json-tree` | editable-project | 128 | 32 | 2272665 | 489.414 | 8.044 | 2 | 145354 | 4 | yes | yes |
| `sqlite-project` | editable-project | 128 | 32 | 1355776 | 243.896 | 8.132 | 1 | 1355776 | - | yes | yes |
| `indexed-zlib-bundle` | compiled-runtime | 128 | 32 | 48596 | 199.658 | 7.120 | 1 | 2932 | - | yes | yes |
| `chunked-json-tree` | editable-project | 128 | 64 | 2269377 | 481.484 | 55.532 | 2 | 568784 | 4 | yes | yes |
| `sqlite-project` | editable-project | 128 | 64 | 1335296 | 375.054 | 49.653 | 1 | 1335296 | - | yes | yes |
| `indexed-zlib-bundle` | compiled-runtime | 128 | 64 | 45639 | 230.498 | 29.298 | 1 | 10957 | - | yes | yes |

## Trade-off matrix

| Concern | `chunked-json-tree` | `sqlite-project` | `indexed-zlib-bundle` |
|---|---|---|---|
| Primary fit | Editable source/project | Editable transactional container | Compiled runtime artifact |
| Git review / merge | Strong: per-chunk canonical text files | Weak: single binary database | Weak as source; not intended for authoring |
| Partial / atomic save | Per-file atomic replace; journal still needed for multi-file save | Transaction-capable in principle; benchmark uses `journal_mode=OFF`, so crash recovery is not proven | Read-only build artifact; compiler atomically replaces whole artifact |
| Random chunk access | Direct file lookup after manifest | Indexed SQL primary key | Explicit bounded binary index |
| Corruption fence | Manifest per-chunk SHA-256 | Per-row SHA-256 checked by loader | Per-chunk SHA-256 plus zlib decode bounds |
| Patch locality | Changed chunk files | Container-level unless SQLite-aware delta tooling exists | Chunk payloads are independently indexed; patch protocol remains unselected |
| Interoperability | Very high | High | Requires published schema/container contract |
| Studio ergonomics | Simple inspectability; many-file lifecycle complexity | Strong transactional query/edit model | Runtime-oriented, not an editor source |
| Schema evolution | Explicit versions/critical features; final unknown-field policy unfrozen | Same semantic envelope, DB migrations required | Explicit bundle version/critical features; final compatibility policy unfrozen |
| Crash recovery | Multi-file recovery journal not implemented | Not evaluated with WAL/rollback journal in this deterministic-byte benchmark | Immutable rebuild/replace model only; rollout recovery not evaluated |

## Fail-closed and projection evidence

- Corruption rejected for every measured candidate: **yes**.
- Decompression-ratio adversarial case rejected: **yes**.
- Truncated bundle rejected: **yes**.
- Oversized string rejected: **yes**.
- Unknown critical feature rejected: **yes**.
- Nesting-depth overflow rejected: **yes**.
- Collection-count overflow rejected: **yes**.
- JSON chunk path traversal rejected: **yes**.
- Client projection excludes `server_only` data for every measured scale: **yes**.

## Migration and provenance boundary

The fixtures are deterministic project-owned synthetic data. This spike does **not** prove Crystal/OTBM semantic parity, broad import completeness, or redistribution rights. Any real importer must retain pinned source digests, conversion diagnostics, unresolved/lossy semantics and zero-silent-loss reporting before format selection.

## Not proven by this spike

- SQLite crash recovery/WAL behavior is not measured; the deterministic-byte prototype disables journaling during one-shot artifact construction.
- The binary prototype has per-chunk SHA-256 and bounded zlib decoding, but no separate manifest checksum/signature, release signing, CDN layout or production patch protocol.
- Real Crystal/OTBM import parity, exact item/appearance catalog binding and zero-silent-loss corpus conversion remain outside this synthetic benchmark.
- Final unknown-optional-field compatibility rules, schema migration tooling, Studio concurrent-edit UX and autosave journals remain unfrozen.
- The synthetic composite fountain proves semantic/visual-footprint separation can be represented; renderer correctness and real multi-tile import recognition are not evaluated here.

## Evidence candidate recommendation

**RECOMMENDATION — not a format decision:** keep the editable-project and runtime-bundle concerns separate. The measured `chunked-json-tree` is the clearest baseline for Git review and bounded parallel authoring; `sqlite-project` is a credible Studio-oriented alternative when transactional multi-object edits dominate; `indexed-zlib-bundle` is the strongest of these three prototypes for a compiled runtime artifact because it is deterministic, indexed, bounded and per-chunk integrity checked.

This recommendation does not freeze extensions, physical schemas, chunk dimensions, compression, patch protocol, signing, CDN layout, or compatibility policy.

## Owner decision required

The owner must separately **select / rework / defer** the permanent World Project and World Bundle physical formats after reviewing this dossier and any additional Studio/import/runtime evidence. `SPIKE_RESULT != OWNER_FORMAT_DECISION` remains binding.
