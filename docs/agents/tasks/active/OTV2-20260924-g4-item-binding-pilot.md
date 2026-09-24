---
task_id: OTV2-20260924-g4-item-binding-pilot
mode: IMPLEMENT
status: implementing
issue: 162
issue_comment: 5821976322
pr: null
repository: Oteryn/Oteryn-Game
base_commit: 6185045c20631b8614b1d4ad1dacc5d918f73b0a
branch: agent/otv2-g4-item-binding-pilot-r2
---

# G4 Item exact-binding pilot

Build a bounded, read-only crosswalk over the 22 protected `WIKI_MATCHED` Item candidates. Reuse the exact #856 source capture artifact and reconstruct the protected 38,157-Item canonical map and current-source comparison from pinned repository inputs. Retain only compact identity/revision/digest and comparison-state evidence.

## Boundaries

- The canonical Item definition identity remains `(Item, ProductionKey, definition revision)`. A wiki page ID, OTS ID, client ID, title, appearance, Asset, or runtime/wire ID never creates or replaces that identity.
- Every result row retains the full #856 identity tuple: collector source and role, `source_namespace`, verbatim decimal `external_id`, namespaced `page_key`, title, exact revision ID/timestamp, and raw-source digest. The #856 tuple remains alongside any differing current observation when drift is found.
- Bind the #856 compact capture to the current-source page observation only when the exact `page_id`, revision ID, timestamp, title metadata, and raw-source digest agree. Drift or missing/invalid infobox structure is `UNRESOLVED_SOURCE_SHAPE`.
- A source identity can be `EXACT` only with one unique canonical target, at least two independently comparable non-title field agreements, and zero stable-signal contradictions. One agreement remains `PROBABLE_MATCH`; competing targets are `CONFLICT`; competing page candidates are `AMBIGUOUS`; no comparable signal is `NO_MATCH`.
- `ACCEPTED_ALIAS` is reserved for separately proven duplicate aliases; this pilot has no alias authority and emits none.
- Emit at most one typed binding candidate, selected deterministically from eligible exact rows; use the valid source key `oteryn:source.tibiawiki` and exact #856 batch ID as source revision, while retaining the page revision and digest in the evidence. It remains candidate-only. Never write `ProjectV2SourceIdentityBinding` into `provenance/sources.json` or mutate the Project, Items, gameplay values, Presentation/Asset, runtime, or client IDs.
- Keep raw wikitext/prose transient. Second-wiki corroboration remains `UNKNOWN`; OTS remains hypothesis-only.
- One Git Data commit on the allocated branch only; no PR or integration authority.

## Acceptance

- The workflow downloads artifact `10829932700` from #856 run `36050042631`, verifies archive digest and exact manifest/capture/row digests, and uses only that source identity+revision+digest table.
- Reproduce the protected native Item identity map, the exact #763 crosswalk, and the protected current-source 38,157-row partition; fail closed on source, digest, count, or candidate drift.
- Fail closed when the stable current-source output digest (retrieval timestamp removed) differs from the protected #770 manifest; this pins the complete `(source_item_id, canonical key, page_id)` mapping and its compared fields.
- Synthetic tests cover the two-signal threshold, conflicts, ambiguity, no-match, and filtering of unsupported/nested signals.
- The hosted result is a compact crosswalk artifact and manifest only; candidate binding count is at most one and only `EXACT` / separately authorized `ACCEPTED_ALIAS` states can appear there.
