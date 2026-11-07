# DYN-ATLAS-001 Game-owned Thais Z7 fixture producer

Status: **bounded proof implementation / non-production physical profile**.

This directory implements the Game-side producer for the DYN-ATLAS-001 Semantic Thais Z7 proof. It converts one exact pinned legacy migration source into an explicit `oteryn-game-atlas-export-v1` semantic projection.

## Authority boundary

The producer may read pinned OTBM and Tibia asset metadata because those inputs are inside the **Game/import boundary**. The emitted fixture is not OTBM-shaped authority and the Atlas consumer must not reopen the legacy inputs to infer missing facts.

The producer owns:

- legacy Z7 -> native `floor=-7` through `oteryn-crystalserver-legacy-spatial-import-v1`;
- visible ground/top-level tile ordering -> explicit `PresentationOrderKey`;
- concrete static frame/phase/pattern/layer/sprite selection;
- conversion to the accepted `oteryn-atlas-15-32-appearance-spatial-v1` visual anchor/coverage/displacement semantics;
- deterministic proof-scope placement identities;
- fail-closed diagnostics and artifact identity.

The producer deliberately does **not** invent canonical Game entity identities for legacy item numeric IDs. Presentation records carry `canonical_entity_id: null` and `entity_identity_state: UNRESOLVED` where no canonical identity has been established.

Nested container child items are source item-tree data, not visible map-stack records, and are not emitted as spatial presentation records.

## Exact proof inputs

```text
migration repository: blakinio/Otheryn@e417c5e7c22986bf4acef0495eb47f7b72c97cce
CrystalServer source: zimbadev/crystalserver@5e89bf8329ea406cb4ea8f4a18f32954f13e5418
world.otbm sha256: 3bd40d14fefec41f24c4b3ae879e420be1a831ef55b95dcbec721e587a09b034
legacy selection: X=32280..32440, Y=32155..32305, Z=7
native selection: x=[32280,32441), y=[32155,32306), floor=-7

asset source: 15.32.zip
Drive file id: 1Dlo3bS4K1nS3mw4BhPZdlHT7lX5zRAvv
ZIP sha256: 1a6bad8b7598cd874f534cd4aae2d249fb3d9b4458b3ccfa75754f91bb27870f
catalog sha256: 35639e000c4c108665a091cfbdf699d549d995b37670bc08de575ab6cd380d85
appearance sha256: dc4f4c01e3701c77877c67895168e4399837046122d6d17e3e608a12a2fed075
```

## Proof-only physical packaging

The implementation emits `dyn-atlas-thais-z7-jsonl-v0`:

```text
manifest.json
  canonical semantic manifest + proof physical-profile declaration + artifact digest

tiles.jsonl
  one canonical JSON object per native tile, y-major/x-minor ordering

diagnostics.json
  canonical diagnostic envelope; acceptance requires an empty diagnostic list

artifact.sha256
  convenience text copy of the semantic artifact digest
```

This is deliberately **not** the permanent Game -> Atlas serializer. It exists because deterministic JSONL is inspectable, straightforward to validate, and sufficient for the bounded proof. FlatBuffers/Protobuf/other binary schemas, compression and final chunk dimensions remain evidence-gated future decisions.

### Artifact digest

The semantic artifact digest is:

```text
SHA256(
  b"OTERYN-DYN-ATLAS-THAIS-Z7-JSONL-V0\\0" +
  canonical_json(manifest_without_artifact_digest) +
  exact_tiles_jsonl_bytes +
  canonical_diagnostics_json_bytes
)
```

Canonical JSON uses UTF-8, sorted object keys, compact separators and exactly one trailing LF. Wall-clock timestamps, runner paths and host metadata are excluded from the canonical artifact.

## Presentation records

Each tile record contains explicit source/native positions and zero or more presentation records. For each visible ground/top-level item the producer exports:

- deterministic proof-scope `export_record_id`;
- `appearance_source_id` as provenance/reference identity only;
- `canonical_entity_id: null` and `entity_identity_state: UNRESOLVED` when no canonical Game entity mapping exists;
- `PresentationOrderKey { plane: 0, order: source_visible_index }`;
- source role (`ground` or `tile_item`);
- producer-resolved appearance primitives.

Each resolved primitive contains explicit frame group, static phase, selected source pattern, layer index, source sprite ID, dimensions, visual coverage and native presentation displacement.

Atlas therefore does not need to reproduce legacy stack-count, hangable-hook, fluid, position-modulo pattern or animation-selection rules.

## Run locally

The proof intentionally requires separately supplied pinned migration inputs and does not vendor them into Game:

```bash
python tools/game-atlas-thais-fixture/export.py \
  --legacy-root /path/to/Otheryn-at-e417c5e \
  --map /path/to/world.otbm \
  --asset-zip /path/to/15.32.zip \
  --assets /path/to/extracted/assets \
  --producer-sha "$(git rev-parse HEAD)" \
  --output /tmp/thais-z7-export

python tools/game-atlas-thais-fixture/verify.py /tmp/thais-z7-export
```

The dedicated GitHub Actions workflow performs the exact pinned checkout/download, two independent producer runs, byte comparison, verifier run, deliberate corruption rejection and workflow-artifact publication.

## Rights boundary

This producer does not commit raw source asset pixels to Game and does not itself grant redistribution rights. The semantic fixture references source sprite IDs and appearance semantics only.

Public Atlas publication of decoded source pixels remains separately gated on explicit authorization for the exact `15.32.zip` digest.

## Proof-harness limits

The scripts contain bounded line/presentation/primitive/total-byte caps to protect this migration harness. They are **not production resource limits** and must not be copied into a production limits registry without independent evidence.
