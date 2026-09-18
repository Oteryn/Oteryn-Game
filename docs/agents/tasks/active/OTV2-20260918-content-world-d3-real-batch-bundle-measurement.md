# OTV2-20260918-content-world-d3-real-batch-bundle-measurement

Task: CONTENT_WORLD_D3_REAL_BATCH_BUNDLE_MEASUREMENT_504
Repository: Oteryn/Oteryn-Game
Branch: agent/content-world-d3-real-batch-measurement-504
issue: 162
status: implementing
Admission SHA: 7b61accf6148981cadaacebe707cae35bd4c0ae5
Reconciled protected main: 5f8146e2d4fce7b1c0f8cabe9bd49e937ee7ea3d
Owner: Oteryn: content world build
Production authority: NONE

## Exact six-path custody

1. tools/content-format-spike/README.md
2. tools/content-format-spike/spike.py
3. tools/content-format-spike/self_test.py
4. docs/agents/evidence/OTV2-20260918-content-world-d3-real-batch-bundle-measurement.json
5. docs/agents/evidence/OTV2-20260918-content-world-d3-real-batch-bundle-measurement.md
6. docs/agents/tasks/active/OTV2-20260918-content-world-d3-real-batch-bundle-measurement.md

Fresh pre-write census enumerated all 27 open Game PRs and found zero overlap with these paths. The same canonical branch was fast-forward reconciled to protected main; no replacement branch, reset, rebase or force update was used.

## Exact source inputs

CrystalServer: zimbadev/crystalserver@ff7ede593c69d4c658b382c97443e8155926924a.
world.otbm: 52,267,895 bytes; SHA-256 09cce62af6c86644b5579fba460c674585261eb987ca5aa1f52baef9e91f8bbb; Git blob e95e8f7c7a95d1b634b49a5dea5a5dc76021406b.
Parser: blakinio/Otheryn@e417c5e7c22986bf4acef0495eb47f7b72c97cce from a clean sparse checkout.
15.32 ZIP SHA-256: 1a6bad8b7598cd874f534cd4aae2d249fb3d9b4458b3ccfa75754f91bb27870f.
Catalog SHA-256: 35639e000c4c108665a091cfbdf699d549d995b37670bc08de575ab6cd380d85.
Appearance SHA-256: dc4f4c01e3701c77877c67895168e4399837046122d6d17e3e608a12a2fed075.

The exact fresh source exhausted with strict parsing: 1 MapHeader, 18,997,668 Tile, 33 Town and 18 Waypoint records. Without window expansion, Newhaven yielded 1,024 tiles / 1,199 presentations and Targuna 1,024 tiles / 1,264 presentations.

## Semantic boundary

The real batch remains MIGRATION_EVIDENCE / OTS_HYPOTHESIS_ONLY. No protected production SourceIdentityBinding exists for this fresh corpus, so D3 does not invent canonical definition or placement identity. Real source occurrences remain unresolved and appearance IDs are source provenance only.

Target coordinates, floor parity, collision, ordered placement, presentation/collision footprints and transition endpoints remain DEFERRED_REQUIRES_PHASE_B. Production authority and Reference parity remain NONE. SPIKE_RESULT != OWNER_FORMAT_DECISION remains binding.

## Implementation and validation

The historical Issue #95 synthetic spike remains the default CLI. D3 is a separate opt-in real-batch mode and measures exactly two physical carriers sharing one logical/index structure: an integrity-checked uncompressed baseline and the same structure with zlib level-6 compression.

The D3 seam adds exact read-only producer/CW2 adapter provenance, strict fresh-source materialization, deterministic typed/pre-promotion batching without synthetic bindings, server/client allowlists, per-chunk raw/stored SHA-256, bounded decompression, indexed cell/occurrence/definition lookup, one-record locality measurement, repeat-byte determinism, enumeration/rechunk identity, and fail-closed corruption/truncation/profile/version/critical-feature/index/ratio negatives.

Validation so far:
- Python compile: PASS.
- tools/content-format-spike/self_test.py: PASS, 16/16.
- All 12 historical Issue #95 tests remain green.
- Four focused D3 tests cover carrier equivalence/round-trip, random access, client allowlist, negative boundaries, enumeration/rechunk identity and one-record locality.
- git diff --check: PASS after the first material implementation increment.

Independent whole-diff review is treated as required before final integration because the final diff exercises a new D3 loading/decompression path over real source-derived data, even though it reuses the existing bounded zlib decoder.

Next action: publish the first material commit and open an early Draft PR, then execute the real measurement from that committed exact head and add the two evidence files.
