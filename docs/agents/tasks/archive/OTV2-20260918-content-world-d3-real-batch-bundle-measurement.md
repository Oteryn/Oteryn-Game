> Lifecycle closeout: **ARCHIVED / OWNERSHIP RELEASED**. Delivery PR #662 merged as `4b5377f4caa765321477011df859cf20de32786a`. Any active/checkpoint language below is historical provenance only; live GitHub and protected current state supersede it.

# OTV2-20260918-content-world-d3-real-batch-bundle-measurement

Task: CONTENT_WORLD_D3_REAL_BATCH_BUNDLE_MEASUREMENT_504
Repository: Oteryn/Oteryn-Game
Branch: agent/content-world-d3-real-batch-measurement-504
issue: 162
pr: 662
status: validating
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

## Whole-diff self-review checkpoint

The first published material candidate is PR #662 head e946eb1d52bef7f03b0e55fa7a2c65e9bf2e32fd. Self-review before final evidence found and repaired:

- P1: logical identity included retained source-shard metadata, violating the explicit requirement that source sharding must not change logical identity. Logical identity now excludes physical selection/shard metadata while retaining exact source-generation identity and logical records; focused regression covers shard relabel plus enumeration reorder and rechunk.
- P2: client projection removed server_only after copying the server fixture instead of using a positive allowlist. The client projection now constructs only explicitly shared top-level fields, and D3 validation rejects non-allowlisted provenance/definition/cell/placement fields before projection.
- P2: the baseline logical records carried a synthetic measurement_revision field only for locality probing. It is removed from real carriers; the scratch-only update probe now changes one source-role field only in the temporary mutated carrier.
- P2 evidence completeness: add encoded record-byte maxima and an oversized raw-chunk negative in addition to corruption/truncation/profile/version/critical/index/ratio negatives.

Post-repair Python compile and the complete focused suite remain PASS, 16/16, including all 12 historical Issue #95 tests.

## Final real measurement on repaired committed seam

Repair head 6ba0e9964f3e1182f4bf6da12e478ee3bfa70f88 was published through the guarded META 3.1 publication path before the final real run. The exact-source run then completed PASS.

- typed CW2/pre-promotion input: 1,321,309 bytes, SHA-256 4838be7abf70a390b61f4a7578626c1a0b61725c1313e2046606110065d47ec7;
- 2,048 source tile records / 2,463 source occurrences;
- 0 bound definitions / 0 bound placements / 2,463 unresolved source occurrences;
- normalized logical input: 1,953,208 bytes; logical identity 662b3ed148aff5646b95114c5859eac72355cb25b8ec8af6eb2d77abe88cd333;
- 183 source-definition references; max 5 source placements per cell;
- encoded maxima: cell 3,737 bytes; placement 738 bytes; source definition 162 bytes.

Uncompressed carrier:
- artifact 2,106,802 bytes;
- raw/stored chunk bytes 1,916,220 / 1,916,220;
- max raw/stored chunk 982,068 / 982,068 bytes;
- build 680.778 ms; median cell lookup 75.678 ms;
- one-record scratch update changes one chunk plus manifest, 1,172,666 rebuilt/patch bytes.

zlib level-6 carrier:
- artifact 265,659 bytes;
- raw/stored chunk bytes 1,916,220 / 75,079;
- raw/stored compression ratio 25.522716:1;
- max raw/stored chunk 982,068 / 38,812 bytes;
- build 703.895 ms; median cell lookup 74.854 ms;
- one-record scratch update changes the same one chunk plus manifest, 229,427 rebuilt/patch bytes.

Determinism PASS: source enumeration order, source-shard metadata, rechunk identity, same logical index across both physical carriers and exact-byte repeated builds. Negative evidence PASS: wrong source profile, wrong carrier version, unknown critical feature, malformed placement index, oversized raw chunk, decompression-ratio fence, plus per-carrier corruption and truncation rejection. Client projection uses a positive shared-field allowlist and excludes server_only metadata.

No production hard maxima are selected. Production authority and Reference parity remain NONE. SPIKE_RESULT != OWNER_FORMAT_DECISION remains binding.

## Independent review classification

NOT_REQUIRED under the current META AI review policy and the task-specific D3 trigger. This is ordinary offline measurement tooling, not control-plane/high-risk work. It does not add a new parser, downloader, signing boundary, production/runtime loader or new decompression primitive: zlib decoding already existed in the historical spike and D3 reuses the same bounded_decompress implementation only for carriers generated inside the local measurement workflow. The new container checks add fail-closed size/hash/profile/version/index fences rather than broadening runtime trust.

Next action: commit the two generated evidence files plus this final checkpoint, publish the exact candidate, then run exact-head repository qualification and whole-diff final self-review.
