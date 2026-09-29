---
task_id: OTV2-20260925-g4-nonitem-bulk-crosswalk
mode: IMPLEMENT
status: implementing
issue: 162
issue_comment: 5826871400
pr: null
repository: Oteryn/Oteryn-Game
base_commit: 1680eb5dc6145aa3e271ac8665f33a50ed837b76
branch: agent/otv2-g4-nonitem-bulk-crosswalk
---

# G4 direct Non-Item bulk crosswalk

Implement one current-source fetch/parse/dedupe/crosswalk engine for the 4,800 G3 direct-family rows, partitioned in one artifact into Creature, NPC, Achievement, Quest, Ability, Outfit and Mount.

## Scope and invariants

- Pin and validate the exact G3 classification and G4 source-provenance artifacts, then join only by exact MediaWiki page ID.
- Preserve G3, G4 and live page IDs, titles, exact revision IDs/timestamps, and SHA-256 of the exact current UTF-8 page bytes.
- Fetch current source by page ID; preserve a row for unavailable, redirected, unsupported, drifted, malformed or failed pages. Never silently drop one.
- Before parsing any current content-bearing record, require an exact positive integer `revid` (excluding booleans), a valid UTC MediaWiki timestamp (`YYYY-MM-DDTHH:MM:SSZ`), a bounded non-empty title, and valid response/page/revision/slot object shapes. Malformed provenance remains explicit unresolved evidence and can never be marked stable or revalidated.
- Parse one recognized family infobox and retain only short scalar values from the allowlisted family fields. Long-form article text, loot prose/tables, service rows, shop lists, exact formulas and asset payloads are excluded.
- Detect same-family candidate duplicate/conflict clusters only when title and infobox name both agree. This is candidate evidence, not a target match.
- Keep canonical target, ProductionKey and source binding null. The output is a workflow artifact; no tracked production project package exists at admission `main`.
- Do not use a Wiki/OTS/client/page ID as an Oteryn canonical identity or store an unqualified generic external ID in content fields.

## Expected partition

Creature 2,149; NPC 1,253; Achievement 569; Quest 272; Ability 171; Outfit 134; Mount 252. Total exactly 4,800 unique page IDs, zero dropped/duplicate rows.

## Validation and authority

Focused parser/dedup and malformed-provenance regression tests, exact pinned-artifact verification, hosted live fetch of current source revisions, exact partition/provenance gate and workflow artifact upload are required. `current_revision_revalidated_rows` must equal the number of output rows in `CURRENT_REVISION_REVALIDATED`; the two known prior-drift pages (63947 and 46925) must be counted after exact current fetch. The earlier hosted attempt/artifact is superseded by the repair candidate and must not be used as final validation. This task does not mint canonical identities, emit `ProjectV2SourceIdentityBinding`, promote semantic fields, populate definitions/Presentation/Asset/runtime records, or resolve a production package destination. Independent review, PR readiness, Merge Queue and integration remain coordinator-owned.
