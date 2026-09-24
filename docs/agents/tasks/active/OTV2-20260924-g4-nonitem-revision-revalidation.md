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
- [ ] Hosted workflow verifies pinned artifact metadata and runs exact-ID current TibiaWiki reads.
- [ ] Exact single commit is pushed and live branch head is read back.
- [ ] Final path delta contains only the five allocated paths.

## Excluded scope

No broader G4 non-Item crosswalk rerun, no title identity, no repository target inventory, no identity or semantic promotion, and no PR/comment/integration action. Coordinator retains PR and protected integration authority.

## Context checkpoint

```yaml
status: implementing
branch: agent/otv2-g4-nonitem-revision-revalidation
base_sha: c516182255d3ea1724e91671c8d2187eb622e3df
head_sha: null
pr: null
next_action: finish five-path implementation, validate, publish one commit, and read back live branch head
```
