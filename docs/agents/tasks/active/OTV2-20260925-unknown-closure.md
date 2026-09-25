# OTV2-20260925-unknown-closure

~~~yaml
task_id: OTV2-20260925-unknown-closure
title: Deterministic single-wave disposition of G3 UNKNOWN pages
mode: IMPLEMENT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/otv2-g3-unknown-closure-wave
issue: 162
pr: 876
base_sha: 1680eb5dc6145aa3e271ac8665f33a50ed837b76
head_sha: 94869dc86eebabf1f9800ce7b6e6296eea4e7ad4
final_head_sha: null
owner: delegated Luna writer
allocation_comment_id: 5827058358
owned_paths:
  - tools/content-census/unknown_closure.py
  - tools/content-census/unknown_closure_self_test.py
  - .github/workflows/unknown-closure.yml
  - docs/agents/tasks/active/OTV2-20260925-unknown-closure.md
  - docs/agents/evidence/OTV2-20260925-unknown-closure.json
created_at: 2026-09-25
updated_at: 2026-09-25
~~~

## Summary

Run one reproducible, artifact-only closure wave over the pinned exact 5,512-row UNKNOWN ledger. Rebuild that ledger from its immutable G3 artifact, verify the committed predecessor manifest, refresh exact page IDs/revisions/templates/categories from the primary current Wiki when reachable, and emit an explicit disposition for every row.

Only the registered exact World Change template + root + surface signature routes a source definition to the existing Encounter family. Relationship, editorial, dynamic-state, source-only, redirect, parse-error and ambiguous dispositions remain separate. Every candidate relationship is carried forward. Page titles and discovery roots alone never route a family.

Every output row separately records family, relationship, placement, source-only and exclusion dispositions. Missing coordinate/placement evidence is explicitly not promoted as a placement, and all hard-exclusion rows must remain zero.

## Scope and authority

No canonical Oteryn target selection, ProductionKey minting, source-identity binding, field/semantic promotion, placement reconciliation, runtime authority or production data mutation. This does not block G4 work already classified. Full row output is a 14-day workflow artifact; only compact counts/contract stay in Git.

Current Wiki access failures, drift and malformed responses fail closed per row. Pinned source observations are never labelled current without exact revision-ID match. A redirect is not resolved without exact target page-ID evidence; the input ledger has no such proof.

## Expected pinned counts

- 5,512 unique UNKNOWN page IDs and 0 dropped/duplicate rows.
- Source shapes: 3,803 structured primary; 1,399 no Item infobox; 187 alternate; 104 redirects; 12 classifier-unresolved; 7 parse-error.
- Structured-primary partition: 3,121 Object-only; 1 Object × World Quest overlap; 521 Hunts; 12 World Change; 17 World Quest-only; 131 residual.
- Offline deterministic fixture: 12 exact Encounter routes; 539 relationship-only; 3,121 multi-family definition candidates; 131 ambiguous residual; 1,709 other source-shape-specific unresolved rows.
- Hosted counts may route fewer rows when current revisions drift or the Wiki is unavailable; those remain explicitly unverified/unresolved and do not fail the cohort partition gate.

## Validation and closeout

The hosted workflow verifies the exact G3 artifact run, archive digest, manifest, universe digest and 5,512 count; regenerates the prior ledger and byte-compares its compact manifest; runs offline counterexample tests; refreshes current source page IDs/revisions/shapes; verifies the exact row partition and authority invariants; and uploads complete rows plus manifest with 14-day retention.

## Reviewer disposition

- P1: R2 counted current rows without pinned revision IDs as revision drift. The repair separately counts all absent baselines, reachable current rows without a baseline, verified same-revision rows, and true drift (a non-empty pinned baseline that differs from the current revision). Focused regressions cover empty-baseline changes and true drift. R2 run/artifact results are superseded pending exact-head rerun.
- P2: Batch failure fanout remains fail-closed. Further hardening is non-blocking HARDENING and is not part of this repair.

Exact branch freeze, draft PR, workflow run, artifact details, and final row counts are recorded at coordinator handoff. Writer does not mark ready or enqueue Merge Queue.
