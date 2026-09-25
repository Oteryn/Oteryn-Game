# G4 Non-Item second-source crosswalk

```yaml
task_id: OTV2-20260925-g4-nonitem-second-source-crosswalk
title: G4 Non-Item second-source crosswalk
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base: main
base_branch: main
branch: agent/otv2-g4-nonitem-second-source-crosswalk
issue: 162
pr: null
base_sha: bfc8b54548a09c59e062873a5dfa48739c477420
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: allocated #162 sole writer
owned_paths:
  - tools/content-census/g4_nonitem_second_source_crosswalk.py
  - tools/content-census/g4_nonitem_second_source_crosswalk_self_test.py
  - .github/workflows/g4-nonitem-second-source-crosswalk.yml
  - docs/agents/tasks/active/OTV2-20260925-g4-nonitem-second-source-crosswalk.md
  - docs/agents/evidence/OTV2-20260925-g4-nonitem-second-source-crosswalk.json
```

## Scope and authority

Allocation: Issue #162 comment `5828248468`, amended by comment `5828278808`. This is one evidence-only G4 source lane on protected admission `main@bfc8b54548a09c59e062873a5dfa48739c477420`. The exact primary cohort is the merged #875 artifact `10848111721`, run `36097714916`, archive SHA-256 `0e340f9e77987f4dea3f5ca77f875d3f36dbfc7a2b08822b0c55716e01e5c293`, and canonical output SHA-256 `38d827ba66bb7a04a3f5a94bbb873ca4485d4b7c873de957812a9c1be20a67c5`.

The hosted workflow fetches and verifies that exact artifact, preserves every primary source tuple and family row, and performs one bounded Fandom MediaWiki endpoint/terms preflight. If accessible under the same-origin Fandom endpoint and displayed CC BY-SA terms, it enumerates each family’s infobox pages with continuation, captures page/revision/timestamp/SHA-1 and raw-byte SHA-256, then retains compact allowlisted facts only. The crosswalk uses non-title facts and emits only `EXACT_CANDIDATE`, `PROBABLE`, `AMBIGUOUS`, `CONFLICT`, `NO_MATCH`, or `SOURCE_UNAVAILABLE`; exact candidates require at least two matching non-title facts.

The amended stop condition is explicit: if the hosted Fandom API or license/about preflight is blocked, return a compact `SOURCE_UNAVAILABLE` artifact, preserve all 4,800 #875 source tuples, mark all seven family corroboration states `UNKNOWN`, and pass the evidence-only workflow. Local/browser Fandom HTTP 402/tool denial is not treated as corroboration. Official public Tibia library HTML receipts are attempted only for creature, achievement and spell lists; their absence/403 remains a receipt state and does not block the Fandom path.

## Source and legal limits

- Fandom TibiaWiki is the independent candidate; it is not authoritative proof that every fact is independently originated.
- TibiaData, TibiaWikiSQL, tibiawiki.dev and other mirrors/derived feeds are not independent evidence and are not used.
- Fandom license/about receipts are captured before content. Each retained Fandom fact is short, structured and allowlisted; raw page content is hashed in memory and discarded. No article prose, spoilers, transcripts or images/assets are retained.
- `Ability` is the spell subset only (`Template:Infobox Spell`). No broader ability completeness claim is made.
- Numeric IDs are kept qualified by source key, namespace and revision. Equal decimal IDs across wikis do not join identities.
- No title-only exact match; no canonical identity, ProductionKey, source binding, definition population, presentation identity or semantic field promotion.
- If the preflight blocks, all second-source statuses stay `UNKNOWN`; no source family absence is inferred.

## Validation

The local synthetic self-test covers continuation/repeated-token handling, independent-source qualification, duplicate-title collisions, alias/non-title-only behavior, exact revision SHA-1/timestamp validation, unsupported Ability shapes, and title-only rejection. The hosted workflow verifies the exact #875 cohort, runs the endpoint/terms preflight once, applies the bounded capture if accessible, validates source-unavailable evidence if blocked, and uploads only compact artifacts. No hosted capture or source availability is asserted before the exact candidate run.
