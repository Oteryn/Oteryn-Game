# Oteryn World + VFX real-content visual slice

Status: **ACTIVE NON-PRODUCTION EXPERIMENT / CONTINUATION CHECKPOINT**

Issue: #509

Short invocation:

```text
Oteryn: real-content visual slice
```

## Purpose

Continue the protected World + VFX line after #480/#489 by replacing the synthetic coloured benchmark placeholders with a bounded real-content visual slice. The experiment must use the same custom Rust + `wgpu` renderer direction while consuming Game-owned normalized appearance/world semantics and the exact locally authorized Tibia 15.32 pixels only as physical presentation input.

This directory grants no production renderer/client/server/protocol/content/deployment authority.

## Protected predecessor

#480 is terminally complete through PR #489. The real Merge Queue composition commit was:

```text
e6dfcef4fb6c270a1b2af09a2fe02c6b5bbdc108
```

The corresponding `Game Merge Queue gate` / `merge_group` run completed SUCCESS, and protected-main readback confirmed the final #480 evidence pack.

The protected #480 verdicts remain binding evidence for this continuation:

- custom Rust + `wgpu` remains the renderer foundation;
- bounded visible working set with eviction: `ADOPT`;
- measured simple hybrid resource layout: `REJECT` for that workload;
- atlas vs texture arrays: `INSUFFICIENT_EVIDENCE`;
- KTX2 vs DDS, hard RAM/VRAM budgets, final particle/light/batching/filtering policy: still unresolved.

## Authority and owned paths

This worker owns only:

```text
experiments/world-vfx-real-content/**
```

and bounded public-safe evidence/docs specifically for #509.

Do not mutate:

- `apps/client/**`;
- `crates/renderer/**`;
- `apps/game-server/**`;
- protocol/persistence/CONTENT;
- root Cargo/workspace;
- workflows/rulesets/META;
- `RESOURCE_LIMITS_REGISTRY.json`;
- Platform/Atlas runtime;
- production or deployment state.

#502 is a separate production resource-bound architecture gate with `implementation_authority: NONE`; do not consume or replace it.

## Exact source evidence already verified

Locally authorized source archive:

```text
15.32.zip
size: 246811594 bytes
sha256: 1a6bad8b7598cd874f534cd4aae2d249fb3d9b4458b3ccfa75754f91bb27870f
```

Catalog/appearance anchors:

```text
catalog-content.json sha256:
35639e000c4c108665a091cfbdf699d549d995b37670bc08de575ab6cd380d85

appearances DAT sha256:
dc4f4c01e3701c77877c67895168e4399837046122d6d17e3e608a12a2fed075

sprite sheets in catalog: 5084
sprite geometry classes: 32x32, 32x64, 64x32, 64x64
```

The source archive/pixels/decoded sheets must never be committed to Git.

## Game-owned semantic source

Do not make the renderer reinterpret legacy Tibia semantics from scratch.

Use the existing Game-owned producers/contracts, especially:

- `tools/game-atlas-thais-fixture/**` for semantic Thais Z7 tile/presentation projection;
- `tools/game-atlas-appearances/**` for normalized 15.32 object/outfit/effect/missile appearance programs;
- `oteryn-world-spatial-v1` and `oteryn-atlas-15-32-appearance-spatial-v1` semantics.

Raw `appearance_source_id` and `sprite_source_id` are provenance/presentation identities, not gameplay authority.

## Thais Z7 fixture already generated and verified locally

Pinned world input:

```text
world.otbm sha256:
3bd40d14fefec41f24c4b3ae879e420be1a831ef55b95dcbec721e587a09b034

legacy selection:
X=32280..32440
Y=32155..32305
Z=7

native floor:
-7
```

The existing Game-owned producer completed and its verifier passed with:

```text
artifact digest:
sha256:4b340053f72b3522a9fe644c9afdf08d9c7b9b686aec0b57cef764a7cb7dc468

tiles: 24311
presentation records: 39282
resolved primitives: 39282
unique appearance source IDs: 862
unique sprite source IDs: 990
canonical bytes: 28042720
```

The generated local fixture is evidence input only and is not committed.

A dense representative 40x30 viewport search identified:

```text
x = 32400..32439
y = 32239..32268
presentation primitives in window: 2215
```

This is a candidate visual-smoke viewport, not a permanent gameplay/map boundary.

## Physical sprite decoding direction

The 15.x catalog stores sprite sheets as CIP-specific `*.bmp.lzma` files. The validated decode structure is:

```text
0..23   padding
24..31  metadata
32..36  LZMA1 properties
37..44  little-endian compressed payload size
45..    raw LZMA1 payload
```

Each decompressed sheet is a 384x384 BMP. `spritetype` determines the logical cell geometry:

```text
0 -> 32x32
1 -> 32x64
2 -> 64x32
3 -> 64x64
```

Implement a bounded experiment-local decoder/adapter. Do not introduce this physical format into gameplay/domain identity.

For a local renderer atlas, a valid prototype approach is a 64x64 carrier cell with smaller source sprites anchored consistently to the accepted south-east/bottom-right visual anchor semantics. This is an experiment packing choice only and must not freeze production atlas policy.

## Immediate next work

1. Produce/load normalized 15.32 appearance programs for object/outfit/effect/missile categories using the existing Game-owned producer.
2. Add a bounded experiment-local `catalog-content.json` + `*.bmp.lzma` physical-pixel adapter.
3. Resolve the 990 Thais fixture sprite IDs on demand and build a local GPU resource set without committing decoded pixels.
4. Render the dense Thais viewport using the fixture's explicit `PresentationOrderKey`, primitive dimensions, displacement and visual coverage.
5. Add at least one real animated outfit/creature path using normalized direction/frame/timing semantics.
6. Add at least one real effect and one real missile path.
7. Preserve movement interpolation, names/HP overlay, light/day-night and critical VFX overlay from the #480 architecture.
8. Exercise zoom/fractional zoom, large-sprite overhang and bounded resource decode/cache churn.
9. Run smoke on Molehill-PC / RX 9070 XT / DX12 and store only public-safe measurements/digests/IDs in Git.
10. Verify the Git diff contains zero proprietary pixel bytes or extracted sprite sheets.

## Completion gate

Do not call #509 complete until:

- exact 15.32 digest is checked before pixel access;
- real sprite decoding is deterministic and bounded;
- object + outfit + effect + missile real-content paths are visibly exercised;
- Game-owned semantic programs drive composition/timing;
- real Thais scene respects ordering/coverage/displacement/overhang semantics;
- presentation remains non-authoritative;
- critical VFX remains readable;
- local Molehill smoke has no unexplained crash/device loss;
- only public-safe evidence is committed;
- exact-head CI/review succeeds;
- integration uses the current bound META 3.1 native exact-head Merge Queue contract and protected-main readback.

## Continuation directive

GitHub LIVE state is the only current authority. Continue the **same #509 worker and branch**; do not create replacement Issues/branches/PRs unless the canonical worker is genuinely unavailable. Treat CI/review/Merge Queue as checkpoints, not architecture redesign opportunities. Do not reopen engine selection or convert #480 calibration cache sizes into production hard limits.

`IMPLEMENTATION_AUTHORITY: NON_PRODUCTION_EXPERIMENT_ONLY`
`RUNTIME_ACTIVATION_AUTHORITY: NONE`
`MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`
