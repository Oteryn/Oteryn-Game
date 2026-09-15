# OTV2-20260910-reference-world-corridor-census-511

```yaml
task_id: OTV2-20260910-reference-world-corridor-census-511
title: Measure bounded Newhaven and Targuna migration-corpus windows
mode: MIGRATE
status: active
repository: Oteryn/Oteryn-Game
issue: 511
base_branch: main
branch: agent/reference-world-corridor-census-511
pr: 525
base_sha: 43ff3341e079f2883b78d01db5cee649290d90be
head_sha: pending_final_exact_head_freeze
final_head_sha: null
final_head_frozen_at: null
owner: REFERENCE_WORLD_CORRIDOR_CENSUS_511
created_at: 2026-09-10T08:40:00+02:00
execution_policy: continuous_progress
owned_paths:
  - tools/reference-world-corridor-census/**
  - docs/agents/evidence/OTV2-20260910-reference-world-corridor-census.md
  - docs/agents/tasks/active/OTV2-20260910-reference-world-corridor-census-511.md
public_contracts: []
depends_on:
  - Oteryn/Oteryn-Game#162
  - Oteryn/Oteryn-Game#511
  - Oteryn/Oteryn-Game#523
  - Oteryn/Oteryn-Game#526
blocks:
  - Oteryn/Oteryn-Game#504
cross_repository_coordination_id: null
external_repositories:
  - blakinio/Otheryn@e417c5e7c22986bf4acef0495eb47f7b72c97cce (READ_ONLY_MIGRATION_EVIDENCE)
```

## Protected authority and resume

The single canonical worker is this existing Draft PR #525 / branch `agent/reference-world-corridor-census-511`.

Protected lifecycle evidence:

- original allocation PR #523 integrated as protected `81ff001609c730b1dca1b28c431199045dec00f3`;
- original #162 activation comment `5614294048`;
- allocation repair PR #526 exact head `254fcd098549d5630357e26f0689ab66b7498aea`;
- #526 real merge-group run `34455053189` and aggregate `game-gate` = SUCCESS;
- repaired protected `main@1fdc37fdb8b8faf2aaab99f17a25c7a6e986f7c5`;
- explicit post-repair #162 resume comment `5615644586` after fresh overlap/custody reconciliation.

Every material result produced after hold `5614417475` and before resume `5615644586` remains `PRE_REPAIR_UNQUALIFIED / NOT_ACCEPTED_EVIDENCE`. Preserve that history, but never consume those pre-resume counts/digests/classifications as #511/#504 acceptance.

No replacement worker/branch/PR is authorized. Existing producer/reconstruction/runtime/registry/workflow/Cargo/Platform/Atlas paths remain read-only.

## Outcome

Produce the smallest deterministic Phase-A migration-corpus census needed for the first Reference Newhaven/Targuna corridor evidence. Measure exactly two separate authorized 32x32 source shards on native floor `-7` through the existing protected Game fullworld producer APIs and emit public-safe, machine-readable lower-bound evidence for #504.

This is migration/source evidence only. It is not Global target geometry, a playable world import, production resource maxima, a production chunk/world format or Evolved content.

## Exact source contract

```text
legacy repository:
  blakinio/Otheryn@e417c5e7c22986bf4acef0495eb47f7b72c97cce
world.otbm SHA-256:
  3bd40d14fefec41f24c4b3ae879e420be1a831ef55b95dcbec721e587a09b034
Drive asset file id:
  1Dlo3bS4K1nS3mw4BhPZdlHT7lX5zRAvv
15.32.zip SHA-256:
  1a6bad8b7598cd874f534cd4aae2d249fb3d9b4458b3ccfa75754f91bb27870f
catalog SHA-256:
  35639e000c4c108665a091cfbdf699d549d995b37670bc08de575ab6cd380d85
appearance SHA-256:
  dc4f4c01e3701c77877c67895168e4399837046122d6d17e3e608a12a2fed075
```

The accepted execution must use a fresh one-shot Python interpreter, prove the exact legacy Git top-level and HEAD, require a completely clean legacy worktree including untracked files, and reject pre-existing `tools` / `tools.otbm_atlas*` modules. Immediately after `load_runtime(...)`, every actually loaded legacy parser module must resolve under the pinned legacy `tools` tree, be tracked there, and have a working-tree blob identical to the pinned Git blob. Any mismatch is `LEGACY_PARSER_REVISION_MISMATCH` before evidence acceptance.

## Exact authorized footprints

```text
Newhaven
  locator=(32536,32514)
  native_floor=-7
  authoritative_start_shard=f-7-r1016-c1016
  lookup_only_region=fm000007_rxp000127_ryp000127

Targuna
  locator=(31934,31925)
  native_floor=-7
  authoritative_start_shard=f-7-r997-c997
  lookup_only_region=fm000007_rxp000124_ryp000124
```

Only the named 32x32 start shard is measured for each location. The 256x256 fullworld region is lookup/retrieval metadata only and must not be counted wholesale.

All coordinates and identities remain `MIGRATION_EVIDENCE / OTS_HYPOTHESIS_ONLY` for Global geometry.

## Required implementation shape

Reuse read-only `tools/game-atlas-fullworld-source/producer.py`; do not create another OTBM parser, semantic projection or fullworld exporter.

A thin consumer may live only under `tools/reference-world-corridor-census/**` and must:

- retain/project only records in the two exact authorized start shards;
- preserve deterministic producer order;
- preserve explicit unresolved appearances rather than substituting them;
- report tile/non-empty/ordered-presentation/max-per-cell/floor/identity dimensions separately;
- retain town/waypoint and transition-like records only as structural migration observations;
- retain composite/multi-cell visuals only as diagnostics;
- never commit raw source, map, archive, pixel, sprite or canonical record bytes.

For byte evidence, unpack `record_bytes, record_stats = project_tile_bytes(runtime, tile)` for every selected tile in producer order. Hash/count only `record_bytes`. The canonical multi-record stream is the direct concatenation of those exact bytes with zero added delimiter/prefix/suffix and no stripping of producer-defined trailing newlines. Public evidence may retain only aggregate byte count, maximum single-record byte length and lowercase SHA-256 of that stream.

## Fail-closed expansion rule

Ordinary occupancy of a shard edge is **diagnostic only** and is not clipping proof.

Do not traverse or count any adjacent shard or additional floor under this task. If an exercised structure/path/service fixture is demonstrably clipped, or if expansion necessity is materially ambiguous, record the evidence plus the exact minimal proposed adjacent shard/floor and return:

```text
WINDOW_EXPANSION_REQUIRED
```

Then stop before measuring the proposed footprint. Any expansion requires a separately protected #162 allocation amendment. Recursive/flood-fill widening and a single Newhaven-to-Targuna rectangle are forbidden.

## Post-resume validation evidence

The post-resume worker report on PR #525 (`5616108211`) records:

- focused `self_test.py` PASS;
- Python compile PASS for census helper/tests;
- protected fullworld producer compile/self-test PASS;
- exact pinned source/parser validation PASS;
- two independent exact-corpus census executions;
- byte-identical summaries via `cmp`;
- canonical summary SHA-256 `242ca3df871ba7946c0f86f0b503e29f269c8756c72ac2a3eaf5228e93b52091`;
- `git diff --check` PASS.

Independent readback also verified the claimed pinned parser blobs for `tools/otbm_atlas/{__init__.py,assets.py,nodefile.py,semantic.py}` against `blakinio/Otheryn@e417c5e7c22986bf4acef0495eb47f7b72c97cce`.

Measured public-safe result:

| Dimension | Newhaven | Targuna |
|---|---:|---:|
| cells / tile records | 1,024 | 1,024 |
| ordered presentations | 1,199 | 1,264 |
| max presentations per cell | 5 | 5 |
| unique appearance source IDs | 70 | 156 |
| unique resolved sprite IDs | 84 | 190 |
| aggregate encoded bytes | 899,694 | 939,759 |
| max encoded record bytes | 3,080 | 3,079 |
| ordered stream SHA-256 | `bebce3eca44dfcf3ab4044fd7a4a66bf6c1f7a80e2b0d375de14c3eb3898faac` | `5e9a498a37570662f57d24ec8ca020c521ccf56091b03e33970aa6e2a1394438` |
| unresolved presentations | 0 | 0 |

Both shards have four-edge occupancy classified `EDGE_OCCUPANCY_ONLY_NOT_CLIPPING_PROOF`. No exercised clipping/expansion requirement was established, `window_expansion_required=[]`, and no adjacent shard was counted.

`phase_a_result=PASS`, `phase_b_target_parity=NOT_PERFORMED`, `registry_maxima_selected=false`, `production_authority=NONE`.

## Acceptance criteria

- [x] Exact pinned map/asset/catalog/appearance identities verified on the execution surface before measurements were accepted.
- [x] Exact legacy parser checkout/imported-module provenance verified fail-closed.
- [x] Existing fullworld producer API reused; no duplicate parser/importer/schema.
- [x] Newhaven and Targuna measured as separate exact 32x32 floor -7 windows.
- [x] 256x256 source regions retained as lookup-only envelopes.
- [x] Tile/non-empty/placement/max-per-cell/floor dimensions independently reported.
- [x] Resolved/unresolved identifiers explicit and machine-readable.
- [x] Canonical stream count/max/SHA-256 framing reproduced deterministically without tracked raw bytes.
- [x] Edge occupancy is not promoted to clipping/expansion evidence.
- [x] No unallocated expansion occurred.
- [x] Transition-like records remain structural observations only.
- [x] #504 receives lower-bound dimensions only; no production maximum selected.
- [x] No migration/OTS value promoted to Global Reference truth.
- [x] No proprietary map/source/pixel/screenshot/canonical-record bytes committed.
- [x] No permanent world/chunk/bundle/GPU/resource layout selected.
- [x] No registry/runtime/client/server/Cargo/workflow/Platform/Atlas/META write.
- [ ] Final exact-head whole-diff self-review has zero open material findings.
- [ ] Final exact-head repository checks are green.
- [ ] Required independent review, if selected by the current review policy/task gate, is satisfied on the final material head.
- [ ] Unresolved review threads/requested changes are zero before integration handoff.

## Failure behavior

- source unavailable: `SOURCE_CORPUS_REQUIRED`;
- legacy checkout/import provenance mismatch: `LEGACY_PARSER_REVISION_MISMATCH`;
- source digest mismatch: fail closed before evidence acceptance;
- unresolved critical mapping/appearance: preserve explicit unresolved evidence;
- count/byte arithmetic failure: fail closed;
- clipping/ambiguous expansion need: `WINDOW_EXPANSION_REQUIRED` and stop before measuring the proposed footprint;
- required unallocated path: `SHARED_LEASE_REQUIRED`;
- material architecture decision: `ARCHITECTURE_ESCALATION_REQUIRED`.

## Final gate state

The tracked task intentionally does not embed its own final commit SHA because doing so would require a self-referential follow-up commit. Live PR #525, exact-head reviews/checks and Issue #162 are the lifecycle authority for final SHA/status.

```yaml
last_progress: post-repair exact-corpus evidence regenerated and task wording reconciled to protected allocation
status: active
branch: agent/reference-world-corridor-census-511
pr: 525
final_head_sha: pending_live_exact_head_freeze
phase_a_result: PASS
phase_b_target_parity: NOT_PERFORMED
production_authority: NONE
next_action: freeze the resulting exact head, run whole-diff self-review and exact-head repository checks, resolve the independent-review requirement from current policy/task authority, then hand off for separate integration authorization; do not begin Phase B
```
