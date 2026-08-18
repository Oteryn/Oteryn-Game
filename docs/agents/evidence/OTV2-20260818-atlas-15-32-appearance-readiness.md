# Atlas 15.32 Drive appearance readiness — 2026-08-18

## Verdict

**READY_FOR_EXACT_HEAD_REVIEW** for the bounded Game-owned `oteryn-atlas-15-32-appearance-spatial-v1` source profile.

The technical source identity and presentation conversion are evidence-backed. Public pixel redistribution remains separately gated on explicit project authorization for the exact 15.32 ZIP digest.

## Pinned authorities

- Game base: `blakinio/Oteryn-v2@2c77f0c373157312b9979dc01806acd56f4949a4`
- Game -> Atlas contract: `oteryn-game-atlas-export-v1`
- native spatial profile: `oteryn-world-spatial-v1`
- CrystalServer legacy spatial importer: `oteryn-crystalserver-legacy-spatial-import-v1`
- repository latest-build selection: `blakinio/Otheryn@e417c5e7c22986bf4acef0495eb47f7b72c97cce`, `tools/otbm_atlas/build_latest_local.sh`
- metadata-only 15.32 verification: `Oteryn/Oteryn-Atlas@0b56d9a95279f1ec02fddd0dfcf8bd6ffd16b539`
- Google Drive object metadata: file ID `1Dlo3bS4K1nS3mw4BhPZdlHT7lX5zRAvv`

## Source identity proof

The Otheryn latest-build script defaults to:

```text
ATLAS_ASSET_LABEL = 15.32
Google Drive file id = 1Dlo3bS4K1nS3mw4BhPZdlHT7lX5zRAvv
expected ZIP SHA-256 = 1a6bad8b7598cd874f534cd4aae2d249fb3d9b4458b3ccfa75754f91bb27870f
```

Direct Google Drive `files.get` metadata for that exact file ID additionally proves:

```text
name = 15.32.zip
mime type = application/x-zip-compressed
size = 246811594 bytes
created = 2026-08-17T18:16:59.210Z
modified = 2026-08-17T18:18:00.781Z
shared = true
```

Therefore `15.32` is corroborated both by the repository's selected source label and by the exact Drive object's own filename. It is not merely an inferred local label.

One-shot Atlas verification downloaded that exact public Drive object and proved the expected SHA before extraction/decoding.

Exact verification evidence:

```text
run = 32115353176
job = 95643523890
artifact = 9316452985
result = PASS
```

The ZIP contains `assets/` as its only top-level entry and no `package.json`. Consequently `15.32` is proven as the repository-selected / Drive-object source identity, while this work still does **not** claim an internal package metadata field independently reports `15.32`. Byte compatibility is owned by the exact immutable digests below.

## Decoded source facts

```text
zip sha256 = 1a6bad8b7598cd874f534cd4aae2d249fb3d9b4458b3ccfa75754f91bb27870f
catalog sha256 = 35639e000c4c108665a091cfbdf699d549d995b37670bc08de575ab6cd380d85
catalog entries = 5090
appearance file = appearances-dc4f4c01e3701c77877c67895168e4399837046122d6d17e3e608a12a2fed075.dat
appearance sha256 = dc4f4c01e3701c77877c67895168e4399837046122d6d17e3e608a12a2fed075
object appearances = 43514
object frame records = 43514
sprite sheets = 5084
total object sprite refs = 111957
unique object sprite ids = 79269
shift-bearing appearances = 567
height-bearing appearances = 2192
max object-frame sprite refs = 855
catalog sprite id domain = 0..301200
```

Decoded sprite cell layouts are exactly `32x32`, `32x64`, `64x32`, `64x64`.

## Separation from the 15.25 fixture

The separately uploaded archive previously analyzed as `15.25.bd5a04` has a different ZIP digest and materially different decoded counts (42,107 object appearances, 4,927 sprite sheets, 75,623 unique object sprite IDs).

Therefore:

- the accepted 15.25 profile remains a bounded contract for its own exact fixture;
- it is not evidence for 15.32 byte identity or counts;
- DYN-ATLAS-001 latest-source routing selects the independently verified 15.32 Drive profile instead;
- no 15.25 digest/count is copied into the 15.32 contract.

## Presentation-conversion evidence

The pinned asset decoder and renderer operate on the same source-format families exercised by the verified 15.32 catalog/appearance bundle. The verified 15.32 layouts remain integral multiples of the 32-unit tile scale.

The bounded profile therefore makes the following source-conversion choices explicit:

- `units_per_tile = 32`;
- owning map tile is the south-east/bottom-right visual anchor cell for source projection;
- decoded visual coverage extends west/north from that visual anchor;
- visual coverage is presentation-only and cannot become gameplay occupancy/collision authority;
- effective 2D displacement is `(-(shift_x + height), -(shift_y + height))` under native axis semantics;
- concrete frame/phase/pattern/layer/sprite selection remains Game/producer-owned and must be emitted explicitly to Atlas;
- Atlas does not reproduce source stack-count/hook/fluid/position-pattern/animation heuristics as world truth.

These are bounded source-profile semantics, not a universal Oteryn appearance origin or gameplay model.

## Rights classification

### PROVEN

- exact repository-selected Drive ID and ZIP digest;
- exact Drive object name `15.32.zip`, MIME type, byte size and timestamps;
- exact decoded catalog/appearance digests and counts;
- technical provenance to the latest Atlas build path.

### UNKNOWN / explicit gate

The previous project rights attestation was recorded against the different 15.25 ZIP digest. It is not silently widened to this distinct 15.32 ZIP.

Before public publication/redistribution of source pixels from this exact bundle, project authorization must explicitly cover:

```text
1a6bad8b7598cd874f534cd4aae2d249fb3d9b4458b3ccfa75754f91bb27870f
```

This does not block semantic profile acceptance or metadata-only/tooling work; it blocks claiming public pixel publication authority.

## DYN-ATLAS effect

Once the profile merges, the remaining Game executable seam is the bounded deterministic Thais semantic export fixture/artifact for:

```text
legacy selection: X=32280..32440, Y=32155..32305, Z=7
native selection: x=[32280,32441), y=[32155,32306), floor=-7
world profile: oteryn-world-spatial-v1
legacy importer: oteryn-crystalserver-legacy-spatial-import-v1
appearance profile: oteryn-atlas-15-32-appearance-spatial-v1
```

The fixture/export must resolve concrete presentation primitives before Atlas consumption and retain deterministic diagnostics for unsupported/ambiguous source records.

## Classification

- latest source label `15.32`: **PROVEN by repository selection and exact Drive object filename `15.32.zip`**
- 15.32 internal package metadata version: **NOT PRESENT / NOT CLAIMED**
- Drive ZIP/catalog/appearance identities: **PROVEN**
- Drive object size/timestamps: **PROVEN by Drive metadata**
- appearance/sprite counts: **PROVEN**
- 32-unit source projection and bounded conversion: **PROVEN + DERIVED GAME-OWNED PROFILE CHOICE**
- public redistribution authority for exact 15.32 ZIP: **EXPLICIT_DIGEST_ATTESTATION_REQUIRED**
- bounded Game Thais semantic export artifact: **NOT YET IMPLEMENTED**
