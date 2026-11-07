---
task_id: OTV2-20260924-g4-nonitem-revision-revalidation
mode: IMPLEMENT
status: implementing
issue: 162
issue_comment: 5822760897
pr: null
repository: Oteryn/Oteryn-Game
base_commit: c516182255d3ea1724e91671c8d2187eb622e3df
branch: agent/otv2-g4-nonitem-revision-revalidation
owner: delegated non-Item drift agent
owned_paths:
  - tools/content-census/g4_nonitem_revision_revalidation.py
  - tools/content-census/g4_nonitem_revision_revalidation_self_test.py
  - .github/workflows/g4-nonitem-revision-revalidation.yml
  - docs/agents/tasks/active/OTV2-20260924-g4-nonitem-revision-revalidation.md
  - docs/agents/evidence/OTV2-20260924-g4-nonitem-revision-revalidation.json
---

# G4 non-Item revision revalidation

Revalidate only the two revision-drift rows carried from merged #861: Creature page ID `63947` and Quest page ID `46925`. Join only by exact MediaWiki page ID. Reuse the immutable pinned G3 classification and merged #857 source capture, fetch each current TibiaWiki page by page ID, hash the exact current revision bytes in memory, and evaluate only whether the existing G3 direct-family source shape remains supported. The initial task base was refreshed by allocation comment `5822889938`; this branch is based exclusively on the refreshed protected `main@c516182255d3ea1724e91671c8d2187eb622e3df`.

## Scope and authority

- Inputs: G3 artifact `10801778929` / run `35986883931`, and #857 G4 artifact `10831362943` / run `36053194532`; exact archive digests, heads and revision tuples are retained in the compact evidence file and re-verified by the workflow.
- Rows: only Creature `63947` and Quest `46925`. No other G3/G4 rows are revalidated.
- Preserve namespaced source identity and pinned G3/G4 revision/timestamp/digest plus current revision/timestamp/digest. Do not use or emit a title as identity.
- Current TibiaWiki is primary. The second wiki remains `UNKNOWN`; OTS evidence remains hypothesis-only.
- Fail closed on redirects, page-ID or namespace mismatch, missing/malformed revision or digest, source drift during paginated read, and unsupported or ambiguous direct-family source shape.
- Keep article text transient in memory for the SHA-256 and discard it. The compact two-row result is workflow-artifact-only.
- No canonical key/ProductionKey, target selection, binding, definition population, field promotion, Presentation/Asset/runtime/client-ID claim, or gameplay truth.

## Validation

- [x] Exact G3 and #857 artifact specifications are pinned to immutable IDs, runs, heads, sizes, archive digests, member sets and payload digests.
- [x] Only the two named page IDs are selected after strict validation of the full immutable source artifacts.
- [x] Focused synthetic tests cover direct-shape retention and fail-closed identity, namespace, redirect, revision, content, unsupported-shape and ambiguity cases.
- [x] Prior exact-head hosted run `36064987451` on `71f71733dda4ec18a314c99e5912cf6a86d0674c` passed pinned-artifact verification and exact-ID current TibiaWiki reads; artifact `10836120790` (archive SHA-256 `7e8280a036c23593027e996db4862876bb748f7bca221882812eabec1834d11b`).
- [x] Prior hosted result: Creature `63947` current revision `443993` / `2026-09-24T09:09:13Z` / SHA-256 `435ac55fd2663b4c756364e01282efa7ef5888ecd710e6d17a1985b8c51efc41`; Quest `46925` current revision `443994` / `2026-09-24T13:54:17Z` / SHA-256 `9ee5951111700ab8628639ec2521cdf01eafa96cf10b36f62d6d698cce33192a`; both direct-family source shapes remained supported.
- [ ] Recovery successor over live predecessor `b285aa12880876b670b37ca47a2d5f6def0e5f24` is read back; exact-head hosted rerun is pending.
- [x] Publication-integrity incident recorded under allocation `5826888633`: db8 `db8f12ca1b4f12a493f7690c0fec625a23248b4c` failed the first-parent gate; ref was force-moved to sibling b285 before no-force direction arrived. Both remain retrievable; b285 is an ineligible recovery predecessor, not an integration candidate.
- [x] Recovery workflow requires allocated base `c516182255d3ea1724e91671c8d2187eb622e3df` and R1 `71f71733dda4ec18a314c99e5912cf6a86d0674c` as ancestors, exact sole parent b285, and exact five-path aggregate diff from the allocated base.

## Excluded scope

No broader G4 non-Item crosswalk rerun, no title identity, no repository target inventory, no identity or semantic promotion, and no PR/comment/integration action. Coordinator retains PR and protected integration authority.

## Context checkpoint

```yaml
status: recovery-hosted-rerun-pending
branch: agent/otv2-g4-nonitem-revision-revalidation
base_sha: c516182255d3ea1724e91671c8d2187eb622e3df
recovery_predecessor: b285aa12880876b670b37ca47a2d5f6def0e5f24
superseded_attempts: db8f12ca1b4f12a493f7690c0fec625a23248b4c (workflow failed at direct-parent gate), b285aa12880876b670b37ca47a2d5f6def0e5f24 (recovery predecessor only; not integration-eligible)
publication_integrity_allocation: 5826888633
prior_hosted_run: 36064987451 (success for pre-recovery R1 head; not recovery qualification)
failed_recovery_run: 36095496024 (failed before source checks; exact-parent assertion)
pr: null
next_action: read back sole-parent successor over b285, rerun hosted workflow from scratch, verify five-path aggregate diff, and retain resulting artifact
```
