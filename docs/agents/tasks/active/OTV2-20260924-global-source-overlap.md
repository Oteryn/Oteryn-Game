# OTV2-20260924-global-source-overlap

**Status:** Implementing
**Allocation:** Game #162, comment 5811429656
**Base:** `dbc7fe950fa2eb5188e51860bac02e8889463261`
**Branch:** `agent/full-content-global-source-overlap-20260924`

## Objective

Produce a deterministic union of the G1 live non-Item source universe and protected Item page IDs from the pinned #807 crosswalk artifact. Deduplicate only by exact MediaWiki `page_id`. Retain each lane's title observation and provenance, including title divergence for one ID across snapshots. Same-title pages with different IDs remain separate.

## Scope and limits

- Consume G1 artifact 10798668295 / run 35977349690 / head `f1d7dbd6577b033c53545d8650ffba05aafc9480` and #807 artifact 10778892407 / run 35925860576 / head `61d051a13329c51ae04d8a011655e279664c334c` only after exact metadata, ZIP, schema, and embedded digest checks.
- Preserve lane-specific titles, source observations, run/head/artifact provenance and archive digests. Item page revision remains `UNKNOWN` because the crosswalk has no revision field.
- Do not consume crosswalk dispositions, selected native keys, or mapping conclusions. Do not select canonical identities, infer gameplay semantics, or promote classifications to gameplay truth.
- G1 hard exclusions must be absent as attested by the pinned manifest. Do not add exclusion-lane inputs.
- Keep the full merged corpus in a 14-day workflow artifact; do not commit it.
- Use a focused synthetic test for duplicate IDs within a lane (fail closed), cross-lane same-ID title divergence (preserve), and same-title distinct IDs (remain distinct).

## Validation

Run Python syntax checks and focused synthetic tests, then run the hosted workflow on the exact final PR head. The pinned-artifact job must prove the `GITHUB_TOKEN` Actions-read route, verify the resulting merge counts and arithmetic, and upload only the generated corpus plus compact manifest. Preserve the final workflow run and artifact metadata/digests in the handoff record. Keep this task record below 12,000 characters.

## Acceptance

- Exactly six allocated paths; no files outside the allocation.
- Exact page-ID union, provenance retained, title divergence explicitly marked, and no semantic or identity promotion.
- Both pinned upstream artifacts verified; output counts derived from those inputs.
- Focused tests and exact-head hosted artifact job succeed.
- Draft PR only; coordinator owns review, integration ordering, and Merge Queue.
