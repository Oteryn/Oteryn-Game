---
task_id: OTV2-20260924-g4-item-binding-pilot
mode: IMPLEMENT
status: archived
issue: 162
issue_comment: 5822855188
pr: 863
repository: Oteryn/Oteryn-Game
base_commit: c516182255d3ea1724e91671c8d2187eb622e3df
branch: agent/otv2-g4-item-binding-pilot-archive
---

Lifecycle closeout: **MERGED AND ARCHIVED**. PR #863 merged exact head `d5f886404174667ef600cfd404aa176289a504bc` into protected `main` as `c516182255d3ea1724e91671c8d2187eb622e3df`. Hosted revalidation run `36062546418` succeeded and produced artifact `10835326792` (archive SHA-256 `1083f374b8a9f9d0c95c9ea5dd30f49f56fd1809fe2a4a7c616de7aed6065a0d`; evidence SHA-256 `c0f4574590f704409127daa1e1dd3864e4546fb0150b0f2aa716c2f7e0d9b06c`). Independent review passed (PR comment `5822717700`), governed executor run `36063081061` succeeded, and merge-group run `36063133423` succeeded. Terminal integration is recorded at #162 comment `5822854979`. The protected stable-digest drift remains unresolved: 22 rows are `UNRESOLVED_SOURCE_SHAPE`, with 0 typed binding candidates. Implementation paths are released. No Project population, definition, gameplay, presentation, runtime, client-ID, or semantic promotion was performed.

# G4 Item exact-binding pilot

Build a bounded, read-only crosswalk over the 22 protected `WIKI_MATCHED` Item candidates. Reuse the exact #856 source capture artifact and reconstruct the protected 38,157-Item canonical map and current-source comparison from pinned repository inputs. Retain only compact identity/revision/digest and comparison-state evidence.

## Boundaries

- The canonical Item definition identity remains `(Item, ProductionKey, definition revision)`. A wiki page ID, OTS ID, client ID, title, appearance, Asset, or runtime/wire ID never creates or replaces that identity.
- Every result row retains the full #856 identity tuple: collector source and role, `source_namespace`, verbatim decimal `external_id`, namespaced `page_key`, title, exact revision ID/timestamp, and raw-source digest. The #856 tuple remains alongside any differing current observation when drift is found.
- Bind the #856 compact capture to the current-source page observation only when the exact `page_id`, revision ID, timestamp, title metadata, and raw-source digest agree. Drift or missing/invalid infobox structure is `UNRESOLVED_SOURCE_SHAPE`.
- A source identity can be `EXACT` only with one unique canonical target, at least two independently comparable non-title field agreements, and zero stable-signal contradictions. One agreement remains `PROBABLE_MATCH`; competing targets are `CONFLICT`; competing page candidates are `AMBIGUOUS`; no comparable signal is `NO_MATCH`.
- `ACCEPTED_ALIAS` is reserved for separately proven duplicate aliases; this pilot has no alias authority and emits none.
- The typed candidate object must exactly match the `deny_unknown_fields` `ProjectV2SourceIdentityBinding` carrier: `source_key`, `source_revision`, `identity_namespace`, `external_id`, `target`, `disposition`. Keep capture digest, page revision/timestamp, source digest, and candidate-only audit details in adjacent evidence rows, not in the carrier object. Emit at most one exact candidate. Never write the carrier into `provenance/sources.json` or mutate the Project, Items, gameplay values, Presentation/Asset, runtime, or client IDs.
- Keep raw wikitext/prose transient. Second-wiki corroboration remains `UNKNOWN`; OTS remains hypothesis-only.
- One Git Data commit on the allocated branch only; no PR or integration authority.

## Acceptance

- The workflow downloads artifact `10829932700` from #856 run `36050042631`, verifies archive digest and exact manifest/capture/row digests, and uses only that source identity+revision+digest table.
- Reproduce the protected native Item identity map, the exact #763 crosswalk, and the protected current-source 38,157-row partition; fail closed on source, capture, crosswalk, collector, count, or internally inconsistent manifest drift.
- Keep the protected current-source stable digest pin exact. If the real, internally consistent observed digest differs, emit compact revalidation evidence for all 22 observed candidate tuples/signals, mark every row `UNRESOLVED_SOURCE_SHAPE` with the explicit stable-drift reason, emit zero binding candidates, and upload evidence successfully. Do not qualify or bind the drifted observations.
- Synthetic tests cover the two-signal threshold, conflicts, ambiguity, no-match, and filtering of unsupported/nested signals.
- Candidate objects contain exactly the six carrier fields; the hosted result is compact evidence and manifest only; candidate count is at most one and only `EXACT` / separately authorized `ACCEPTED_ALIAS` states can appear there.
