# G4 Item source-provenance pilot

```yaml
task_id: OTV2-20260924-g4-item-source-provenance-pilot
title: G4 Item source-provenance pilot
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base: main
base_branch: main
branch: agent/otv2-g4-item-source-provenance-pilot-r3
issue: 162
pr: null
base_sha: 42a9a440d17293aa7746c3581596fab1a588f77f
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: allocated #162 sole writer
owned_paths:
  - tools/content-census/g4_item_source_capture.py
  - tools/content-census/g4_item_source_capture_self_test.py
  - .github/workflows/g4-item-source-capture.yml
  - docs/agents/tasks/active/OTV2-20260924-g4-item-source-provenance-pilot.md
  - docs/agents/evidence/OTV2-20260924-g4-item-source-provenance-pilot.json
```

## Scope and authority

This task creates a deterministic, source-only provenance capture for the direct namespace-0 TibiaWiki `Categoria:Itens` census already established by protected PR #803. It has a fresh lifecycle repair allocation in #162 comment `5820965458` on branch `agent/otv2-g4-item-source-provenance-pilot-r3`, from protected base `main@42a9a440d17293aa7746c3581596fab1a588f77f`, with exact custody of the five paths listed below.

Prior candidate PR #854 at `20f465f5b9b44035e7c7dfeb345a273de9d0b9c2` was superseded after Agent Governance run `36048919542` required standard task YAML metadata. Candidate PR #855 at `0808708b02bf7255587d95c4c7527a323ad70c42` is also superseded after Agent Governance run `36049603372` required a positive PR when status is `validating`. This r3 successor has a fresh #162 allocation in comment `5820965458`; no validation evidence from #854/#855 is reused.

The capture preserves the upstream MediaWiki page ID as a decimal string `external_id`, a namespaced MediaWiki page key, title, revision ID/timestamp, and the collector's raw-source SHA-256. It strips source-field content and all prose. The compact artifact is generated only in hosted workflow scratch and uploaded with a deterministic manifest; no bulk corpus is committed.

## Explicit non-goals

- No canonical Oteryn binding, identity minting, identity resolution, or source reimport.
- No field, balance, or gameplay-semantic promotion.
- No raw wikitext, article prose, book text, or retained source field values.
- No inference from a second wiki: corroboration stays `UNKNOWN`. English TibiaWiki/Fandom is only a candidate independent source; its pages/API returned HTTP 402 in this environment, so no exact revision or digest is verified and no corroboration is claimed.
- OTS/Canary/Crystal observations remain hypothesis-only and are not consulted as authority.
- No changes to paths owned by PR #807.

## Owned paths

- `tools/content-census/g4_item_source_capture.py`
- `tools/content-census/g4_item_source_capture_self_test.py`
- `.github/workflows/g4-item-source-capture.yml`
- `docs/agents/tasks/active/OTV2-20260924-g4-item-source-provenance-pilot.md`
- `docs/agents/evidence/OTV2-20260924-g4-item-source-provenance-pilot.json`

## Acceptance and validation

The transformer must fail closed for malformed census/manifest digests, incomplete census counts, duplicate or conflicting page identifiers, empty titles, absent/invalid revision identifiers, timestamps, or source digests, and any per-page source-shape partition mismatch. It derives allowed source-shape counts from page rows and requires exact equality with the census/manifest partition plus `source_shape_partition_complete=true`. Output rows are stable-sorted by page ID, use the explicit `mediawiki/tibiawiki.com.br/page_id/` namespace, and preserve the raw `source_digest` value unchanged.

The synthetic test covers deterministic row shape/order, retained digest and external ID, removal of field/prose data, duplicate/conflicting IDs, missing/invalid revision metadata, census-digest/count drift, and source-shape mismatch despite equal page totals. The hosted workflow reruns the protected #803 collector, runs the transformer, independently validates count/schema/digest/non-promotion invariants, and uploads only the compact capture plus manifest for 14 days.

## Lifecycle

This is authoring state until the single Git Data commit is freshly read back on the allocated branch and its exact bounded delta is verified. Candidate-specific CI/review begins only after the candidate SHA is frozen. The coordinator retains PR/integration authority; no merge or protected-main mutation is delegated here.
