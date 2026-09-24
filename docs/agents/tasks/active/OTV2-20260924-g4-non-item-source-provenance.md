---
task_id: OTV2-20260924-g4-non-item-source-provenance
mode: IMPLEMENT
status: implementing
issue: 162
issue_comment: 5821413563
pr: null
repository: Oteryn/Oteryn-Game
base_commit: 4c7e1c8402c721f58c108e4b6912a80aef7458ea
branch: agent/otv2-g4-non-item-source-provenance-r2
supersedes_commit: 7ad58d9e7ce0d450d1ca9e8b3852dae929c36ca2
---

# G4 non-Item source provenance

Capture SHA-256 digests of raw UTF-8 content at the exact G1-pinned MediaWiki revision for every live G1 source page. Keep content transient; retain only compact page identity, revision, digest, and candidate-family/root/surface provenance.

## Boundaries

- Reuse G1 discovery and the protected Item source-universe lane. The Item lane is sealed and is not re-fetched by this gate. Reproduce the pinned G2 union, derive its 6,918 protected Item page IDs, subtract every page-ID overlap from the G1 capture lane, and preserve the exact current overlap IDs as manifest observations.
- Deduplicate page records by MediaWiki `page_id`; reject any conflicting title, revision, or timestamp for one page ID.
- Emit and validate the namespaced source identity on each row: `source_namespace=mediawiki/tibiawiki.com.br`, `external_id` as the exact decimal `page_id`, and `page_key=mediawiki/tibiawiki.com.br/page_id/<external_id>`. Reject duplicate identifiers, malformed row schemas, and any inconsistent identity mapping.
- Fetch content by the exact `revision_id` from G1 and fail closed on missing pages, drift, malformed responses, or unexpected continuation.
- Do not emit raw content, normalized fields, canonical family assignments, bindings, identity minting, semantic promotion, or gameplay claims. Placement classifications remain source observations only.
- Treat second-wiki state as unknown and OpenTibia Server material as a hypothesis only.
- Emit a temporary capture artifact plus a compact manifest; do not commit a bulk content corpus.

## Acceptance

- Synthetic tests cover digest correctness, page-ID merge and conflicts, exact-revision drift, malformed continuation, and protected Item-lane invariants.
- Hosted workflow reruns G1 into runner scratch, reproduces G2 from its pinned verified artifacts, captures exact revision digests only for G1 pages outside the protected Item ID set, validates compact-only output, and uploads only the artifact and manifest.
- One Git Data commit on the allocated branch; exactly five allocated paths; no PR or integration action in this task.
