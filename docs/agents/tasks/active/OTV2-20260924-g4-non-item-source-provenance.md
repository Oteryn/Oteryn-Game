---
task_id: OTV2-20260924-g4-non-item-source-provenance
mode: IMPLEMENT
status: implementing
issue: 162
issue_comment: 5821139398
pr: null
repository: Oteryn/Oteryn-Game
base_commit: 6a7b11a697bb1d921c98e88e069e4ec2e359ef60
branch: agent/otv2-g4-non-item-source-provenance
---

# G4 non-Item source provenance

Capture SHA-256 digests of raw UTF-8 content at the exact G1-pinned MediaWiki revision for every live G1 source page. Keep content transient; retain only compact page identity, revision, digest, and candidate-family/root/surface provenance.

## Boundaries

- Reuse G1 discovery and the protected Item source-universe lane. The Item lane is sealed and is not re-fetched by this gate. Reproduce the pinned G2 union, derive its 6,918 protected Item page IDs, subtract every page-ID overlap from the G1 capture lane, and preserve the exact current overlap IDs as manifest observations.
- Deduplicate page records by MediaWiki `page_id`; reject any conflicting title, revision, or timestamp for one page ID.
- Fetch content by the exact `revision_id` from G1 and fail closed on missing pages, drift, malformed responses, or unexpected continuation.
- Do not emit raw content, normalized fields, canonical family assignments, bindings, identity minting, semantic promotion, or gameplay claims. Placement classifications remain source observations only.
- Treat second-wiki state as unknown and OpenTibia Server material as a hypothesis only.
- Emit a temporary capture artifact plus a compact manifest; do not commit a bulk content corpus.

## Acceptance

- Synthetic tests cover digest correctness, page-ID merge and conflicts, exact-revision drift, malformed continuation, and protected Item-lane invariants.
- Hosted workflow reruns G1 into runner scratch, reproduces G2 from its pinned verified artifacts, captures exact revision digests only for G1 pages outside the protected Item ID set, validates compact-only output, and uploads only the artifact and manifest.
- One Git Data commit on the allocated branch; exactly five allocated paths; no PR or integration action in this task.
