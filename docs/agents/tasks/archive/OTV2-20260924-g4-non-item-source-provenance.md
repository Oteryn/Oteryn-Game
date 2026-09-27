> Lifecycle closeout: **ARCHIVED AFTER CANONICAL PR MERGE**. PR #857 merged through the governed Merge Queue as `c6ffbfc348d11eab66344612aab5eb2513ce8aba` from exact candidate head `b2e77ba01677b484fdfa6271c5745d2280c13df6`; protected main readback at the archive allocation is `6185045c20631b8614b1d4ad1dacc5d918f73b0a`. Exact-head governance run `36053194380`, merge gate `36053194469`, architecture audit `36053194395`, and hosted capture `36053194532` succeeded. Hosted artifact `10831362943` has archive SHA-256 `5d8e886e31a0f64a035a61dc5c3f8ce031a8af3b06926117750deebc841394ca`; compact output SHA-256 `f47dbe5832e7b1accd652852303d638a4f19367bab3951f260cc39a0d93b7713`. Results: G1 9,373; protected Item 6,918; overlap 504; unique non-Item rows 8,869. Independent review passed (PR comment `5821527346`); executor run `36054397480`, UUID `9e3cc33e-7d72-48c2-bd00-5dcdc9d44654`; merge_group run `36054459959` succeeded; terminal integration is recorded at #162 comment `5821757624`. Implementation paths are released. No canonical bindings or semantic promotion were performed; this packet is historical lifecycle evidence. The first archive candidate `ad0cbbf7de5b0dd471a8b4d40bad1d2319db0537` was superseded before PR because it retained a pending archive-packet status; this R2 packet replaces that candidate and reuses no candidate-specific qualification.

# G4 non-Item source provenance

```yaml
task_id: OTV2-20260924-g4-non-item-source-provenance
title: G4 non-Item source provenance
mode: IMPLEMENT
status: archived
repository: Oteryn/Oteryn-Game
base: main
base_branch: main
branch: agent/otv2-g4-nonitem-source-provenance-archive-r2
issue: 162
pr: 857
base_sha: 6185045c20631b8614b1d4ad1dacc5d918f73b0a
head_sha: null
final_head_sha: b2e77ba01677b484fdfa6271c5745d2280c13df6
final_head_frozen_at: 2026-09-24
owner: allocated #162 sole writer
owned_paths:
  - tools/content-census/g4_non_item_source_capture.py
  - tools/content-census/g4_non_item_source_capture_self_test.py
  - .github/workflows/g4-non-item-source-capture.yml
  - docs/agents/tasks/archive/OTV2-20260924-g4-non-item-source-provenance.md
  - docs/agents/evidence/OTV2-20260924-g4-non-item-source-provenance.json
```

## Scope and authority

This task created a source-only G4 capture for every live G1 page outside the sealed Item identity set. The hosted gate reran G1, reproduced the protected G2 identity union, subtracted all 6,918 protected Item IDs, fetched exact pinned MediaWiki revisions, hashed transient UTF-8 content, and retained only compact identity/revision/digest/provenance rows. The implementation was allocated in #162 comment `5821413563`, on `agent/otv2-g4-nonitem-source-provenance-r2` from protected `main@4c7e1c8402c721f58c108e4b6912a80aef7458ea`.

The implementation outcome is recorded for lifecycle continuity; this archive-only closeout does not authorize renewed implementation or integration work.

## Explicit non-goals

- No raw article content/prose, normalized fields, canonical assignment/binding/promotion, placements, or gameplay claims.
- No re-fetch of the sealed Item lane: the 6,918 protected Item IDs were derived from pinned G2; exact G1∩Item IDs remain compact manifest observations.
- Second-wiki state remains `UNKNOWN`; OpenTibia Server material remains hypothesis-only.
- No changes to the implementation or workflow paths during archival.

## Owned paths

- `tools/content-census/g4_non_item_source_capture.py`
- `tools/content-census/g4_non_item_source_capture_self_test.py`
- `.github/workflows/g4-non-item-source-capture.yml`
- `docs/agents/tasks/archive/OTV2-20260924-g4-non-item-source-provenance.md`
- `docs/agents/evidence/OTV2-20260924-g4-non-item-source-provenance.json`

## Acceptance and validation

The collector fails closed on malformed or conflicting page identity/revision records, exact-revision drift, malformed continuation, source-row schema and identifier inconsistencies, and protected Item-lane invariant violations. It fetches the G1-pinned revision, hashes raw UTF-8 transiently, and emits compact identity/revision/digest/candidate-family/root/surface provenance only.

Synthetic validation covers digest correctness, page-ID merge and conflicts, exact-revision drift, malformed continuation, and protected Item-lane invariants. The hosted workflow reruns G1, reproduces G2 from pinned verified artifacts, captures exact-revision digests only for pages outside the protected Item ID set, checks compact-only output, and uploads only the capture artifact and manifest. Original implementation outcome was one Git Data commit with five implementation-allocated paths; it was integrated as PR #857. No validation, review, or integration work is requested by this archive packet.

## Lifecycle

This R2 packet was allocated in #162 comment `5821923343` on `agent/otv2-g4-nonitem-source-provenance-archive-r2`, from protected `main@6185045c20631b8614b1d4ad1dacc5d918f73b0a`. That allocation authorizes only the archive move and evidence JSON update; it grants no implementation, PR, or integration authority.
