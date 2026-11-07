# DYN-ATLAS-001 bounded Game Thais Z7 export fixture — 2026-08-18

## Status

**VALIDATING / deterministic integration proven; final hardened exact-head rerun required**.

This record owns executable evidence for the first bounded Game-produced semantic Atlas fixture. It must not be interpreted as a permanent public serialization profile or full-world exporter.

## Accepted upstream authority

- Game -> Atlas semantics: `oteryn-game-atlas-export-v1`
- native spatial semantics: `oteryn-world-spatial-v1`
- CrystalServer legacy mapping: `oteryn-crystalserver-legacy-spatial-import-v1`
- appearance presentation profile: `oteryn-atlas-15-32-appearance-spatial-v1`
- Game base: `blakinio/Oteryn-v2@457df3772a7aaf648c1a048b2db2caa409fcf974`
- migration implementation evidence: `blakinio/Otheryn@e417c5e7c22986bf4acef0495eb47f7b72c97cce`

## Exact source selection

```text
CrystalServer revision = 5e89bf8329ea406cb4ea8f4a18f32954f13e5418
world.otbm sha256 = 3bd40d14fefec41f24c4b3ae879e420be1a831ef55b95dcbec721e587a09b034
legacy bounds = X 32280..32440, Y 32155..32305, Z 7
native bounds = x [32280,32441), y [32155,32306), floor -7

Drive object = 15.32.zip
Drive file id = 1Dlo3bS4K1nS3mw4BhPZdlHT7lX5zRAvv
asset ZIP sha256 = 1a6bad8b7598cd874f534cd4aae2d249fb3d9b4458b3ccfa75754f91bb27870f
asset catalog sha256 = 35639e000c4c108665a091cfbdf699d549d995b37670bc08de575ab6cd380d85
asset appearance sha256 = dc4f4c01e3701c77877c67895168e4399837046122d6d17e3e608a12a2fed075
```

## Implementation boundary

`tools/game-atlas-thais-fixture/export.py` is the producer/import boundary. It reads pinned migration OTBM and asset metadata but emits only explicit Game-owned semantic presentation records.

Enforced behavior:

- only the fixed Thais Z7 rectangle is exported;
- legacy Z7 becomes native `floor=-7`;
- ground + top-level tile items are the visible presentation source;
- container descendants are not emitted as visible spatial stack records;
- same-tile source order becomes explicit `{plane: 0, order: index}`;
- Game resolves frame group, static default phase, source pattern, layer and sprite source ID;
- source appearance/sprite IDs remain provenance/reference IDs;
- unresolved canonical Game entity identity remains explicitly `UNRESOLVED`;
- visual coverage is presentation-only, not gameplay occupancy/collision;
- no source pixels are included in the semantic fixture.

## Proof-only physical profile

The proof uses `dyn-atlas-thais-z7-jsonl-v0`:

- canonical `manifest.json`;
- canonical y-major/x-minor `tiles.jsonl`;
- canonical `diagnostics.json`;
- convenience `artifact.sha256`.

This does not select the permanent Game -> Atlas serializer, compression or chunk dimensions.

Artifact identity is domain-separated SHA-256 over canonical manifest semantics excluding its digest field, exact tile JSONL bytes and canonical diagnostics bytes.

## First complete integration proof

Workflow run `32117767429`, job `95651058334`, executed against producer head `ee242eb10b0d91cc1d6a0a812bc6f9d8af3d4711` and completed **SUCCESS**.

It proved:

- exact Game head checkout: PASS;
- exact Otheryn migration SHA and `world.otbm` digest: PASS;
- exact 15.32 ZIP/catalog/appearance digests: PASS;
- 7 pure producer self-tests: PASS;
- two independent exporter runs: byte-identical PASS;
- verifier on both outputs: PASS;
- deliberate corruption rejection: PASS;
- semantic artifact upload: PASS.

Observed semantic counts:

```text
tiles = 24311
ground_items = 24292
top_level_tile_items = 14990
source_item_tree_without_ground = 14993
presentation_records = 39282
resolved_primitives = 39282
unique_appearance_source_ids = 862
unique_sprite_source_ids = 990
```

The `14993` source item-tree count versus `14990` top-level visible item count proves that three nested descendants exist in the bounded source and are intentionally excluded from visible spatial presentation records.

Observed canonical file facts on that run:

```text
manifest bytes = 2357
tiles.jsonl bytes = 28040344
tiles.jsonl sha256 = ff14efee3fc376d8f18432c628294c64ffe89450a59aaa498a28e6d705815984
diagnostics.json bytes = 19
diagnostics.json sha256 = 60326e4e048106d4366a2fd8fe472ccfdf06667fcd0f234977febfeaa38f31b8
canonical bytes verified = 28042720
```

Pre-hardening artifact identity for that exact producer head:

```text
semantic artifact digest = sha256:46a34d77f770ed2259cff96bd7bd53b1001ccf5466488409d7fbfdefc6cd2142
workflow artifact id = 9317706518
workflow artifact zip sha256 = 3f582665fcaa6ee91ce832326085ea4fa6f29dfa58686c472ee84e11956d3884
workflow artifact compressed bytes = 1723168
```

## Governance hardening discovered by the same PR

The first semantic integration run passed, but repository policy correctly rejected unpinned action tags in the new workflow. The workflow was subsequently hardened to exact action commit SHAs matching repository policy:

- `actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1`;
- `actions/setup-python@5fda3b95a4ea91299a34e894583c3862153e4b97`;
- `actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02`.

The final PR head therefore requires a new exact-head integration run even though the producer semantics already passed.

## Self-reference guardrail for terminal digest evidence

The canonical manifest contains the exact `producer_repository_sha`, so changing an evidence/task file changes the PR head and intentionally changes the semantic artifact digest.

To avoid an impossible self-referential loop, the **final exact-head** workflow run ID, artifact ID and semantic artifact digest are recorded in PR #335 metadata after the head is frozen, then copied into the post-merge archived task record. Updating PR metadata does not change the producer commit. This evidence file therefore records the stable semantic/count/file facts and the successful pre-hardening proof, while terminal identity belongs to the frozen-head run + closeout.

## Rights boundary

This fixture is semantic metadata only and does not publish decoded source pixels. Public source-pixel publication for the exact 15.32 asset digest remains a separate DYN-ATLAS gate and is not inferred from this producer implementation.
