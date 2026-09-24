# Oteryn CrystalServer Fresh Source Generation Profile v2

- Contract ID: `oteryn-crystalserver-fresh-source-generation-v2`
- Profile revision: `2`
- Canonical owner: `Oteryn-Game`
- Status: **PROPOSED until this exact content is merged to protected `main`; ACCEPTED thereafter unless explicitly superseded**
- Production status: `NOT_ENABLED`

## 1. Purpose

This profile admits one later, exact CrystalServer source generation into the existing Game-owned offline import/projection boundary. It exists to unblock truthful Content/World real-batch measurement without weakening the accepted source-integrity checks.

It does **not** supersede or reinterpret `oteryn-crystalserver-legacy-spatial-import-v1`. The accepted coordinate, floor and visible-order mapping remains the semantic import rule. This profile changes only which exact source generation may enter the already-owned producer when explicitly selected.

## 2. Decision timing

**Must decide now? YES.**

`CONTENT_WORLD_D3_REAL_BATCH_BUNDLE_MEASUREMENT_504` cannot consume the fresh public map truthfully while the producer accepts only the historical map digest. Bypassing that validation would invalidate provenance evidence.

The minimum reversible change is therefore an explicit additional source-generation profile. No permanent serializer, compression, chunking or production-maxima decision is required.

## 3. Exact CrystalServer source

The admitted source generation is exactly:

```text
repository: zimbadev/crystalserver
repository_sha: ff7ede593c69d4c658b382c97443e8155926924a
path: data-global/world/world.otbm
bytes: 52267895
sha256: 09cce62af6c86644b5579fba460c674585261eb987ca5aa1f52baef9e91f8bbb
git_blob_sha1: e95e8f7c7a95d1b634b49a5dea5a5dc76021406b
```

SHA-256 is the integrity identity. The byte length and Git blob are corroborating provenance and MUST NOT replace the SHA-256 check.

A moving branch, tag alias, `main`, `latest`, or an unlisted source digest is not this profile.

## 4. Exact parser generation

The profile reuses the existing pinned migration parser from `blakinio/Otheryn@e417c5e7c22986bf4acef0495eb47f7b72c97cce`.

The required tracked parser blobs are:

- `tools/otbm_atlas/__init__.py` — `047d1274022e1d2a71d6aa23d6efcf420a24535d`
- `tools/otbm_atlas/assets.py` — `25ed2400813bb3ccdc54482967ed05197eb1a850`
- `tools/otbm_atlas/semantic.py` — `a11343a472145aee4d9cf65c6ce28b3e4a71a2b3`
- `tools/otbm_atlas/nodefile.py` — `bed6f7a803d9de485c1f03cbdca4be0cb1521d30`

The parser repository MUST resolve its Git top-level to the supplied parser root, be at the exact commit, have a clean worktree including untracked files, and contain each required tracked file at the pinned Git blob. The fresh profile additionally requires a one-shot import context: `tools`, `tools.otbm_atlas`, every `tools.otbm_atlas.*` module and the qualified bounded-producer module must not already exist in `sys.modules` before admission. Pre-existing in-memory code is not trusted merely because its `__file__` points at the pinned checkout. Ignored executable bytecode is also outside the trust boundary: `tools/otbm_atlas/**` MUST contain no `__pycache__` directory and no `.pyc`/`.pyo` file before parser import. Fresh-profile parser import MUST run with Python bytecode writes disabled, and the bytecode-cache absence MUST be checked again immediately after import. The modules loaded by the producer after those checks MUST resolve under that exact parser root.

This is parser reuse, not parser adoption as Oteryn runtime authority and not permission to fork or rewrite it.

## 5. Exact appearance asset generation

The admitted source uses the existing exact 15.32 appearance source:

```text
Drive file id: 1Dlo3bS4K1nS3mw4BhPZdlHT7lX5zRAvv
15.32.zip sha256: 1a6bad8b7598cd874f534cd4aae2d249fb3d9b4458b3ccfa75754f91bb27870f
catalog-content.json sha256: 35639e000c4c108665a091cfbdf699d549d995b37670bc08de575ab6cd380d85
appearance dat sha256: dc4f4c01e3701c77877c67895168e4399837046122d6d17e3e608a12a2fed075
```

The profile accepts no inferred or substituted appearance generation. Any different asset identity requires an explicit later admission.

## 6. Producer selection and compatibility

`tools/game-atlas-fullworld-source/producer.py::load_runtime(...)` keeps its historical behavior when `source_generation_profile_id` is omitted. The default path continues to delegate to the previously qualified bounded validator and therefore continues to require the historical `world.otbm` SHA-256 `3bd40d14fefec41f24c4b3ae879e420be1a831ef55b95dcbec721e587a09b034`. The fresh-profile one-shot import-context requirement is not imposed on callers that omit the selector.

The fresh source is admitted only when the caller explicitly selects:

```text
source_generation_profile_id = oteryn-crystalserver-fresh-source-generation-v2
```

Unknown profile IDs fail closed. After exact input and parser validation, the producer reuses the same parser, appearance loader and semantic projection path. No alternate OTBM parser or duplicate projection is introduced.

## 7. Evidence classification and semantic boundary

CrystalServer remains migration/reference evidence. This profile does not convert CrystalServer data into Global target truth.

Existing source-position, visible-order and appearance provenance may be retained as migration evidence under the accepted import semantics. Target-sensitive values whose authority is not established remain `DEFERRED_REQUIRES_PHASE_B` or otherwise unresolved under their owning contract.

The existing typed source-identity boundary is unchanged. Legacy numeric IDs, coordinates and source order cannot mint stable Content identity.

This profile does not authorize promotion of:

- target world coordinates or floor assignment;
- collision or walkability truth;
- target placement/order truth;
- presentation or collision footprint truth;
- gameplay behavior, quest, economy or NPC authority;
- production resource maxima.

## 8. Failure behavior

The fresh profile fails closed on map byte-length, SHA-256 or Git-blob mismatch; asset ZIP, catalogue or appearance digest mismatch; missing or multiple appearance DAT candidates; parser Git top-level or repository revision mismatch; dirty parser worktree; untracked/missing or wrong required parser blobs; any `__pycache__`, `.pyc` or `.pyo` under the pinned parser package; any pre-existing parser/package/bounded-producer module in the fresh import context; loaded parser module outside the pinned parser root; malformed source rejected by strict parsing; and unknown or floating source-generation profile selection.

No heuristic repair or fallback to a different generation is permitted.

## 9. Qualification evidence

On 2026-09-18 the exact fresh map was exhausted through the exact pinned parser with `strict=True` without parse failure:

```text
MapHeader: 1
Tile: 18997668
Town: 33
Waypoint: 18
```

The fresh header is `version=4`, `width=35143`, `height=34812`, `items_major=4`, `items_minor=4`.

The repaired producer exhausted the same exact source through `iter_records(..., strict=True)` with the identical counts. The same qualification first proved that a Git-clean pinned parser checkout containing ignored `tools/otbm_atlas/__pycache__/*.pyc` fails closed, then removed only those ignored caches and completed the fresh import/strict stream with zero parser bytecode caches created. The historical default producer path separately accepted the historical exact map and rejected the fresh map with `canonical world.otbm SHA-256 mismatch`.

The explicit fresh profile initialized the existing producer successfully and projected a real fresh-map tile at `x=1356`, `y=3298`, `floor=0`; the canonical projected tile encoded to 774 bytes in that qualification run.

A full visible-presentation census over all 18,997,668 tiles observed 24,502,036 ground/top-level presentations. Against the exact 15.32 appearance catalogue, exactly one presentation was unresolved: server ID `2141` occurred once. This is the same explicit unresolved appearance case already supported by the existing fullworld producer; no new appearance-compatibility class was discovered.

These observations prove source/parser/producer compatibility for this exact generation. They do not prove Global target parity.

## 10. Non-decisions

This profile does not decide final World Project / World Bundle serialization, baseline versus compressed bundle carrier, chunk dimensions, runtime spatial sectors, source authoring partition policy, production hard maxima, CW2/CW3 semantic expansion, D3 format acceptance, or runtime/live-deployment authority.

## 11. Supersession

Any different CrystalServer commit or `world.otbm` bytes, parser generation, or appearance-asset generation requires an explicit later versioned source-generation admission. The historical v1 import contract remains preserved as history and semantic authority until separately superseded.
