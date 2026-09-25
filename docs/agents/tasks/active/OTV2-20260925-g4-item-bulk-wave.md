---
task_id: OTV2-20260925-g4-item-bulk-wave
mode: IMPLEMENT
status: implementing
issue: 162
issue_comment: 5826892380
pr: 877
repository: Oteryn/Oteryn-Game
base_commit: 1680eb5dc6145aa3e271ac8665f33a50ed837b76
branch: agent/otv2-g4-item-bulk-wave
---

# G4 Item bulk crosswalk and safe semantic revalidation

One hosted batch uses one fresh TibiaWiki category census, the complete 38,157-target canonical identity map and the protected OTS-hypothesis crosswalk. It evaluates all 6,918 current source pages against the complete target set with indexed, multi-signal comparison. The full census and target corpus remain runner-local; the retained output contains only exact source tuples, bounded signal comparisons, typed binding candidates and typed safe-field candidates.

## Owned paths

- `tools/content-census/g4_item_bulk_population.py`
- `tools/content-census/g4_item_bulk_population_self_test.py`
- `.github/workflows/g4-item-bulk-population.yml`
- `docs/agents/tasks/active/OTV2-20260925-g4-item-bulk-wave.md`
- `docs/agents/evidence/OTV2-20260925-g4-item-bulk-wave.json`
- `docs/agents/evidence/OTV2-20260925-g4-item-bulk-semantic-promotion.json`
- `apps/game-server/src/content/cw2_b1_import.rs` (only if live output contains safe, unapplied exact fields)
- `apps/game-server/src/content/tests.rs` (only with such an importer update)

## Hard gates

- Recompute current source page revisions, timestamps, and content digests from one live census invocation; do not reuse #856 page content as current.
- Reconstruct and cross-check all 38,157 exact target keys/revisions from the protected exporter and crosswalk.
- Require at least two distinct non-title field agreements, one unique page-to-target mapping, one unique target-to-page mapping, and zero strong contradictory multi-signal candidates before emitting an `EXACT` binding.
- Never compare unqualified numeric IDs, canonicalize from page titles, or mint a target from Wiki/OTS IDs. `PROBABLE_MATCH`, `AMBIGUOUS`, `CONFLICT`, `NO_MATCH`, and `SOURCE_ONLY` are evidence only.
- Field candidates are considered only after an `EXACT` binding and are independently typed against the existing nine Reference Item destinations. No overwrite of known Reference values. The result remains a candidate until consumed by the existing protected Reference Item importer.
- Keep Wiki normalized/unmapped fields, raw wikitext and source prose out of committed files and uploaded artifacts. No ProjectV2 snapshot exists on `main`; do not claim durable `provenance/sources.json` population.
- Do not modify the importer or tests just to manufacture a diff. If the live batch produces no applicable promotions, retain only its evidence/tooling result.

## Validation and integration boundary

Run Python compile/self-tests, exact-head hosted census/crosswalk workflow, affected Rust content tests if importer paths change, repository governance/semantic audits and full `game-gate`. Freeze/read back one candidate head; coordinator retains review readiness, Merge Queue and integration authority. The full per-page evidence and typed candidate arrays are bounded workflow artifacts; tracked JSON is compact result evidence only.
