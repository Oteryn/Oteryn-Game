# Game-owned full-world Atlas source producer

Status: **offline full-world source-projection adapter; migration/import boundary only**.

This directory exposes the Game-owned producer API used by the `ATLAS-FULLWORLD-LOCAL-GENERATION-FABRIC` programme. It exists because the earlier `tools/game-atlas-thais-fixture/` implementation is explicitly bounded to the DYN-ATLAS-001 Thais Z7 proof and must not silently become full-world authority through an Atlas-side private-function call.

## Authority boundary

`Oteryn/Oteryn-Game` remains the World/Content semantic authority. `Oteryn/Oteryn-Atlas` may orchestrate local sharding, checkpointing, hashing and downstream publication, but the per-tile semantic projection stays Game-owned.

The adapter intentionally reuses the already-qualified DYN producer's exact presentation transform and broadens **only spatial selection** from one bounded rectangle to every tile/floor present in the exact pinned migration source. It does not add or guess new gameplay semantics.

The following remain unchanged:

- legacy Z -> native `floor=-z` import transform;
- visible ground/top-level ordering and `PresentationOrderKey` semantics;
- static frame/phase/pattern/layer/sprite resolution;
- visual coverage/displacement semantics;
- unresolved canonical entity identity when no canonical mapping exists;
- nested container children are not visible map-stack presentations.

## Exact accepted migration inputs

The API delegates input validation to the qualified DYN producer. Current accepted proof/import inputs are therefore still exact-digest scoped:

```text
legacy migration repository: blakinio/Otheryn@e417c5e7c22986bf4acef0495eb47f7b72c97cce
world.otbm sha256: 3bd40d14fefec41f24c4b3ae879e420be1a831ef55b95dcbec721e587a09b034
15.32.zip sha256: 1a6bad8b7598cd874f534cd4aae2d249fb3d9b4458b3ccfa75754f91bb27870f
catalog sha256: 35639e000c4c108665a091cfbdf699d549d995b37670bc08de575ab6cd380d85
appearance sha256: dc4f4c01e3701c77877c67895168e4399837046122d6d17e3e608a12a2fed075
```

A different source or asset digest must fail closed until Game/import policy explicitly accepts it.

## API

`producer.py` exposes:

- `load_runtime(...)` — validate exact pinned inputs and load Game/import projection state;
- `iter_records(runtime, strict=True)` — stream the pinned source once;
- record classifiers for map header/tile/town/waypoint records;
- `native_floor(tile)` — accepted legacy spatial import transform;
- `project_tile(...)` / `project_tile_bytes(...)` — Game-owned semantic projection for any tile;
- `canonical_tile_bytes(...)` — exact canonical tile JSON encoding used by the qualified DYN proof.

The API does **not** define the final Atlas serializer, chunk dimensions, publication root, browser loader, pixel pack or runtime cache. Those remain Atlas-side derived-publication concerns.

## Required qualification

Before this adapter is accepted for the full-world hand-off:

1. Python compile/static checks must pass.
2. The exact pinned Thais Z7 selection produced through this API must be byte-identical to the already-qualified DYN `tiles.jsonl` (`ff14efee3fc376d8f18432c628294c64ffe89450a59aaa498a28e6d705815984`).
3. Full-source generation must run fail-closed on unresolved/missing appearance inputs rather than invent records.
4. The caller must record the exact Game revision, legacy revision and all input digests in every downstream shard manifest.

## Non-goals

- no browser/runtime OTBM access;
- no new canonical entity mapping;
- no towns/NPC/spawn/raid/quest-layer inference;
- no final Atlas publication format;
- no pixel redistribution decision;
- no claim that the old bounded physical JSONL package is a permanent serializer.
