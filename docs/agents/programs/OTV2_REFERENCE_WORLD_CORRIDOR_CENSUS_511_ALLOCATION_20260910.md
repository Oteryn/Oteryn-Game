# OTV2 Reference World Corridor Census #511 — allocation

Refs #162 #486 #511 #504 #483 #64 #502.

```yaml
classification: COORDINATOR_ALLOCATION
allocation_state: NOT_ACTIVE_CONDITIONAL
repository: Oteryn/Oteryn-Game
programme: 486
control_plane: 162
active_control_plane_profile: OTV2_WORK_DELIVERY_COORDINATOR
admission_main_sha: 1e993fb62cb9912c833f466a021070d9d93d3892
lane_id: R1_WORLD_MAP
worker_task_id: REFERENCE_WORLD_CORRIDOR_CENSUS_511
worker_branch: agent/reference-world-corridor-census-511
runtime_authority: NONE
content_runtime_authority: NONE
registry_mutation_authority: NONE
production_authority: NONE
live_deployment_authority: NONE
```

## Purpose

Allocate exactly one future bounded Phase-A evidence worker to measure the smallest useful migration-source windows around the already-qualified Newhaven and Targuna locator hypotheses. The worker exists only to produce deterministic source-corpus cardinalities and clipping evidence needed by #504.

This allocation does not select Global target geometry, a permanent world schema, chunk format, `.omap`/`.owb`, production Content maxima, renderer layout or any Evolved behavior.

## Repair checkpoint and activation hold

Protected PR #523 originally integrated this allocation as `81ff001609c730b1dca1b28c431199045dec00f3`, and #162 activation comment `5614294048` subsequently admitted the single worker branch. A later independently authorized review of the concurrently-created but superseded PR #522 produced four P2 findings. That review is **not** an exact-head review of PR #523, but the findings were independently reproduced against this canonical allocation and the protected producer implementation.

The reproduced findings are:

1. the pinned map/asset checks do not pin the legacy parser code imported from `legacy_root`;
2. the `32x32` semantic shard and containing `256x256` source region were both named without declaring one authoritative initial measured footprint;
3. adjacent-window expansion lacked a fail-closed terminal boundary;
4. raw encoded semantic-record-byte wording conflicted with the no-source-payload boundary.

The active worker is therefore held on the **same** branch/PR before material census work. The earlier activation does not authorize material census under the superseded wording. After this repair is protected-integrated and read back, #162 must perform a fresh ownership/custody check and explicitly resume the same worker branch. Do not create a replacement worker.

## Activation gate

This allocation is **NOT ACTIVE** merely because this file or its PR exists.

Material writes by `REFERENCE_WORLD_CORRIDOR_CENSUS_511` are permitted only after all of the following are true:

1. this exact allocation repair candidate receives producer whole-diff self-review and genuinely independent exact-head control-plane review with no open material findings;
2. applicable exact-head Agent Governance, Architecture Semantic Audit and Merge Gate are successful;
3. the repair is integrated through the protected META 3.1 native exact-head Merge Queue path and real `merge_group` aggregate `game-gate` succeeds;
4. protected-main readback proves this repaired allocation file is canonical;
5. Work/#162 performs a fresh ownership/overlap/custody readback and explicitly resumes the **same** `agent/reference-world-corridor-census-511` branch/PR.

No direct worker alias, issue comment, branch creation, historical activation, review result or CI success substitutes for protected repaired allocation plus explicit Work resume.

## Exact future owned paths

After the activation gate, the sole material worker may own only:

```text
tools/reference-world-corridor-census/**
docs/agents/evidence/OTV2-20260910-reference-world-corridor-census.md
docs/agents/tasks/active/OTV2-20260910-reference-world-corridor-census-511.md
```

The new tool directory is optional. If a reproducible one-shot consumer of existing protected APIs is sufficient, omit it rather than creating code for its own sake.

Everything else remains read-only unless a later separately protected allocation amendment says otherwise. In particular, this worker must not mutate:

```text
tools/game-atlas-fullworld-source/**
tools/game-atlas-thais-fixture/**
tools/tibia-worldmap-reconstruction/**
apps/**
docs/contracts/**
docs/architecture/**
docs/agents/programs/**
RESOURCE_LIMITS_REGISTRY.json
.github/workflows/**
Cargo.toml
Cargo.lock
workspace-boundaries.toml
```

Platform, Atlas and every external repository remain read-only. This allocation does not authorize Remote Desktop or any other host access by itself.

## Existing producer boundary — mandatory reuse

Do not create another OTBM parser, legacy intermediate representation, full-world exporter or semantic projection implementation.

Consume the current protected Game-owned producer boundary:

- `tools/game-atlas-fullworld-source/producer.py`
  - `load_runtime(...)` validates the exact accepted map/catalog/assets through the existing qualified producer;
  - `iter_records(...)` streams the pinned source once;
  - tile/town/waypoint discrimination and `native_floor(...)` preserve the accepted spatial transform;
  - `project_tile(...)` / `project_tile_bytes(...)` preserve explicit unresolved appearances fail-closed and reuse qualified deterministic semantic bytes;
- `tools/tibia-worldmap-reconstruction/**` remains the later Phase-B normalized comparison family and is read-only in this Phase-A worker.

`load_runtime(...)` does **not** establish the identity of parser code actually imported from the supplied legacy checkout. Census execution must therefore use a fresh one-shot Python interpreter that has not previously imported `tools`, `tools.otbm_atlas` or any `tools.otbm_atlas.*` module. Before calling `load_runtime(...)`, prove that `legacy_root` is the exact pinned Git worktree and that the complete worktree is clean:

```text
git -C <legacy_root> rev-parse --show-toplevel
  -> must resolve to <legacy_root>
git -C <legacy_root> rev-parse HEAD
  -> e417c5e7c22986bf4acef0495eb47f7b72c97cce
git -C <legacy_root> status --porcelain=v1 --untracked-files=all
  -> empty
```

Immediately after `load_runtime(...)` and before accepting any census record, enumerate the actually loaded parent/package/parser modules: `tools`, `tools.otbm_atlas` and every loaded `tools.otbm_atlas.*` entry in `sys.modules`. For every such module, its import origin/resolved `__file__` (or package search location when applicable) must be under the exact `<legacy_root>/tools` tree; every loaded file must be tracked by the pinned checkout; and its working-tree blob must equal the blob recorded at `e417c5e7c22986bf4acef0495eb47f7b72c97cce` for that path. Any pre-existing module, origin outside `legacy_root`, untracked loaded file, blob mismatch, missing/unverifiable origin, HEAD mismatch or dirty worktree is `LEGACY_PARSER_REVISION_MISMATCH`. Do not repair a contaminated interpreter with reload/monkeypatching; terminate it and start a fresh one-shot process. Stop before accepting census evidence. The map/ZIP/catalog/appearance digests do not substitute for parser-code identity.

Existing workflows do not themselves perform the required bounded Newhaven/Targuna tile/placement census. Do not create a trigger-only/no-op PR or run an unrelated creature census and relabel it as #511 evidence.

## Pinned source and locator inputs

The Phase-A source contract is the existing migration evidence only:

```text
legacy repository: blakinio/Otheryn@e417c5e7c22986bf4acef0495eb47f7b72c97cce
world.otbm SHA-256: 3bd40d14fefec41f24c4b3ae879e420be1a831ef55b95dcbec721e587a09b034
15.32.zip SHA-256: 1a6bad8b7598cd874f534cd4aae2d249fb3d9b4458b3ccfa75754f91bb27870f
```

Catalog/appearance digests must be those accepted by the current protected producer itself. Do not duplicate them into a competing authority table.

Initial source locators:

```text
Newhaven
  semantic record: 3a6d2a5cfd599439efccce35017e12b8
  locator: x=32536, y=32514, floor=-7
  authoritative initial measured footprint: 32x32 semantic shard f-7-r1016-c1016
  lookup/source-retrieval envelope only: 256x256 fullworld region fm000007_rxp000127_ryp000127

Targuna
  semantic record: 57d9ef64607b0360d0c01d338f7c70ba
  locator: x=31934, y=31925, floor=-7
  authoritative initial measured footprint: 32x32 semantic shard f-7-r997-c997
  lookup/source-retrieval envelope only: 256x256 fullworld region fm000007_rxp000124_ryp000124
```

For each location, the named `32x32` semantic shard on native floor `-7` is the **only** initial measured footprint. The containing `256x256` fullworld region is locator/retrieval metadata and must not be counted wholesale. This is allocation authority, not a worker choice.

All coordinates, shards and legacy identities above are `MIGRATION_EVIDENCE / OTS_HYPOTHESIS_ONLY` for Global geometry. They never become `GLOBAL_REFERENCE_TARGET` merely by being measured.

## Required Phase-A method

Treat Newhaven and Targuna as two independent bounded windows.

1. Verify the exact legacy parser checkout before parser import, then verify all pinned map/asset/catalog/appearance identities on the execution surface.
2. Measure exactly the stated `32x32` semantic start shard on native floor `-7` for each location.
3. Enumerate source records in deterministic producer order and retain/project only records inside that authorized footprint.
4. Do **not** automatically traverse an adjacent shard or additional floor.
5. If an exercised structure/path/service fixture is demonstrably clipped at an authorized boundary, record the clipping evidence and exact minimal proposed adjacent shard/floor, return `WINDOW_EXPANSION_REQUIRED` for that location, and stop before counting the additional footprint.
6. Any expansion requires a separately protected #162 allocation amendment naming the exact additional shard(s)/floor(s). Recursive or flood-fill expansion is forbidden.
7. Never create one giant Newhaven-to-Targuna rectangle. Their later relationship is a typed semantic transition/travel boundary, not a requirement for one contiguous coordinate span.
8. Do not scan or materialize the whole world merely because the producer can stream it if a bounded source/window route is available. If the producer API necessarily performs one source stream internally, keep retained/projected output bounded and record that execution fact honestly.

Ambiguous expansion need is also `WINDOW_EXPANSION_REQUIRED`; ambiguity never authorizes widening.

## Required deterministic outputs

For **each** authorized `32x32` start shard report independently:

1. exact Game producer revision, exact legacy parser checkout identity and all accepted source identities/digests;
2. exact authorized `32x32` start shard and its lookup-only `256x256` source-region envelope;
3. exact floors touched inside the authorized footprint;
4. total tile records and non-empty tile records;
5. ordered presentation/placement total and maximum ordered placements per cell;
6. resolved vs unresolved appearance/presentation totals and exact unresolved source identifiers;
7. unique source item/appearance identifiers and unique resolved presentation/sprite identities, migration evidence only;
8. town/waypoint and door/teleport/relocation-like source records encountered, classified only to the strength the existing producer/source supports;
9. candidate composite/multi-cell visual patterns as diagnostics only, with no adjacency-based semantic merge;
10. clipping/connectivity evidence and exact minimal proposed adjacent shard/floor when `WINDOW_EXPANSION_REQUIRED` is reached; do not count that additional footprint;
11. for each selected projected tile in exact deterministic producer order, unpack the producer result as `record_bytes, record_stats = project_tile_bytes(runtime, tile)` and use only the first tuple element for byte-stream framing; `record_stats` remains available for the separately reported semantic counters. Define `record_bytes_in_order` as those byte values in that same order, then define the canonical multi-record stream as the direct byte concatenation `stream = b"".join(record_bytes_in_order)` with **zero additional delimiter, prefix or suffix**: preserve every byte returned in each `record_bytes`, including its producer-defined trailing newline, and add/strip nothing. `aggregate_encoded_byte_count = len(stream) = sum(len(record_bytes))`; `max_record_encoded_bytes = max(len(record_bytes))`, or `0` when there are no selected projected tile records; `ordered_stream_sha256` is the lowercase hexadecimal SHA-256 of exactly `stream` (including the empty-stream case). There are no extra framing bytes, so none are added to the aggregate count. The stream/records remain ephemeral; tracked/public evidence may retain only these count/max/digest values, never raw encoded records;
12. a deterministic machine-readable summary sufficient for #504 to derive lower-bound resource requirements.

Keep these resource dimensions separate: `cells`, `total ordered placements`, `max ordered placements per cell`, `unique identities`, and `encoded bytes`. None may substitute for another.

## Required validation

If a thin consumer is added, use deterministic test-first validation for at least:

- legacy parser checkout HEAD/full-worktree mismatch, pre-import module contamination, loaded-module origin mismatch and loaded-file blob mismatch all fail closed before evidence acceptance;
- stable results independent of container/hash enumeration;
- exact `32x32` start-shard inclusion/exclusion at coordinate boundaries;
- `256x256` source region cannot silently become the measured footprint;
- floor filtering;
- boundary clipping returns `WINDOW_EXPANSION_REQUIRED` without traversing the proposed adjacent footprint;
- explicit unresolved appearance propagation;
- duplicate/counter safety and checked arithmetic;
- source/digest mismatch failure before evidence acceptance;
- no silent dropping of ordered presentations;
- machine-readable summary determinism;
- canonical stream framing is direct concatenation of exact `project_tile_bytes()` outputs, with aggregate count/max/digest reproduced exactly and no raw canonical record bytes written to tracked evidence.

Also run the existing producer self-test/compile checks applicable to the reused API. Do not mutate the producer merely to satisfy the new consumer.

Before worker handoff require full diff review, applicable focused tests and exact evidence/source identity. Before protected integration of any tracked worker result require repository-policy review/CI/Merge Queue appropriate to its final scope.

## Failure behavior

- exact pinned source/archive unavailable: `SOURCE_CORPUS_REQUIRED`;
- legacy checkout HEAD/full-worktree mismatch, contaminated interpreter, loaded-module origin mismatch or loaded-file blob mismatch: `LEGACY_PARSER_REVISION_MISMATCH` before evidence acceptance;
- map/asset/catalog/appearance digest mismatch: fail closed before accepting output;
- boundary clipping or ambiguous expansion need: `WINDOW_EXPANSION_REQUIRED` with exact proposed next footprint; do not measure it without a separately protected allocation amendment;
- unresolved critical mapping/appearance: preserve explicit unresolved evidence; do not substitute another source;
- checked count/byte arithmetic failure: fail closed;
- required unallocated path: `SHARED_LEASE_REQUIRED` with exact path/reason;
- material architecture/product decision: `ARCHITECTURE_ESCALATION_REQUIRED` for only the affected lane;
- proprietary source/pixel bytes or raw canonical encoded records: never commit them.

## Source availability / host boundary

The worker may consume an exact already-authorized source/running environment only through a currently permitted repository/runner mechanism. This allocation does not itself authorize a remote host, credentials, proprietary source redistribution or publication of extracted pixels/assets.

If the exact pinned source/archive/catalog assets needed by `load_runtime(...)` are unavailable on every currently authorized execution surface, return exactly:

```text
SOURCE_CORPUS_REQUIRED
```

with the missing input/execution boundary. Do not guess cardinalities, substitute current Global/Canary/Crystal data, weaken digest/parser checks, or commit proprietary source bytes.

## Phase boundary and downstream use

This allocation is **Phase A migration-corpus census only**.

It does not authorize:

- Phase-B controlled Global target observation/capture;
- target parity classification beyond existing #483 evidence;
- production world/content implementation;
- mutation of `RESOURCE_LIMITS_REGISTRY.json`;
- turning measured lower bounds into hard maxima without a separate evidence-backed resource decision;
- Movement/Combat/runtime activation;
- production deployment;
- Evolved geography/content/balance.

After protected Phase-A evidence, #504 may use the measured cardinalities only as lower-bound/resource-decision inputs. Any production hard maxima remain a separate reviewed registry decision.

## Collision and anti-duplication rule

Immediately before material resume, #162 must re-read open PRs, branches and active tasks. If another canonical #511/corridor census writer exists, this allocation must bind/resume that lineage or remain inactive; it must not create a replacement.

The canonical lineage is the existing Draft PR #525 on `agent/reference-world-corridor-census-511`. Preserve it. Do not create another material branch or worker.

## Completion handoff

A successful worker return must contain:

```yaml
worker_task_id: REFERENCE_WORLD_CORRIDOR_CENSUS_511
exact_head_sha: <sha>
changed_paths: []
source_identity: <exact pinned identities including legacy parser checkout>
newhaven_window_summary: <machine-readable reference>
targuna_window_summary: <machine-readable reference>
window_expansion_required: []
unresolved_source_records: []
focused_validation: []
whole_diff_review: PASS|BLOCKED
phase_a_result: PASS|SOURCE_CORPUS_REQUIRED|LEGACY_PARSER_REVISION_MISMATCH|WINDOW_EXPANSION_REQUIRED|BLOCKED_MATERIAL_FINDING
phase_b_target_parity: NOT_PERFORMED
registry_maxima_selected: false
production_authority: NONE
next_action: <one exact coordinator action>
```

`MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`
