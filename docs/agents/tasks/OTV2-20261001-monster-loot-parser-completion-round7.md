# OTV2-20261001-monster-loot-parser-completion-round7

```yaml
task_id: OTV2-20261001-monster-loot-parser-completion-round7
title: Correct Wiki loot quantities and complete source presence records
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/monster-loot-parser-completion-round7-20261001
pr: null
base_sha: d3cfb2468510255d2495514ec975c23c1d76f32a
head_sha: d3cfb2468510255d2495514ec975c23c1d76f32a
final_head_sha: null
final_head_frozen_at: null
owner: codex-user-directed-monster-completion
created_at: 2026-10-01T22:27:00.270070+00:00
updated_at: 2026-10-01T22:27:00.270070+00:00
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/OTV2-20261001-monster-loot-parser-completion-round7.md
  - docs/agents/tasks/OTV2-20261001-monster-dependency-completion-round6.md
  - tools/content-schema/monster-authoring/wiki_compare.py
  - tools/content-schema/monster-authoring/test_wiki_loot_item_quantities.py
  - tools/content-schema/monster-authoring/samples/wiki-population-2026-09-27.json
  - tools/content-schema/monster-authoring/samples/wiki-population-crystal-00ce02a5-2026-09-27.json
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

Owner explicitly requests continued parallel completion of all safe monster defects.
A complete loot sweep found a concrete research-parser defect: uncertain quantities
such as 1-? and 0-5+ were treated as Item names. This isolated local successor starts
from the complete byte-verified 93-path Round6 candidate; its source/data and actual
native qualifications remain immutable. No remote publication or runtime allocation
is selected, and real architecture decisions remain in #162.

## Disjoint authoring ownership

- Parser worker owns wiki_compare.py and the new quantity regression test only.
  Parse actual positional Loot Item names and uncertain count syntax correctly;
  retain actual uncertain Items as presence facts without inventing probabilities.
  Qualified Item names containing digits remain valid. Prove normal/reversed Item
  argument order, named metadata, missing/ambiguous arguments and source examples.
- Data worker owns the two Wiki population samples only. After parser freeze,
  batch-correct only proven loot.items comparisons from exact pinned captures;
  preserve statistics/probabilities, all other fields, source identities/revisions,
  Bestiary, mitigation and previous changes. Do not silently move source cuts or
  claim observational Wiki-only names are proven missing native Item admissions.
- Root owns this record, candidate freeze, population regeneration and packaging.
  Independent review owns no project sources.

## Qualification

Verify every changed comparison against its exact source content digest. Complete
population regeneration must demonstrate either unchanged native payloads or an
explicit source-qualified data delta. Only provenance changes do not justify a
full Rust rerun; unchanged371 Rust inputs inherit the qualified code checks, and
byte-identical stage data inherits actual native validation only with exact SHA
readback. Broader Global probability and gameplay completeness remain unqualified.

## Research availability and count clarification

The 619 missing mitigation records comprise 618 donor actors and one Wiki-authored
D44 Dark Merudri; the source scan's 619 matching files include duplicate legacy
registrations and are a different unit. Root owns the one-line clarification in
this successor's inherited Round6 task record; frozen Round6 remains unchanged.

Desktop browser research is unavailable because the authorized device is offline.
Exact revision requests remain prepared. Available raw captures may be replayed;
missing raw source never becomes new name, absence, probability or Global truth.
Malformed quantities in previous derived comparisons are separately reported as
unparsed source observations, preserving the boundary rather than inventing data.

The current qualified dependency/data packet and actual remaining architecture
boundaries are recorded, with exact remote body readback, in
https://github.com/Oteryn/Oteryn-Game/issues/162#issuecomment-5942200530 .
No private computer changes, source publication, commit, push or merge occurred.

## Completed bounded corrections

The parser is independently reviewed after its nested-template repair. Exactly
52 prepared loot-presence comparisons are corrected: Imperial has an exact raw
MATCH; 51 comparisons lacking raw article prose are explicitly WIKI_UNPARSED with
derived-only diagnostic evidence. All 82 known quantity tokens are separated from
Item observations; no new Item name, absence or probability is invented. There
are 120 identity-qualified full loot replays among 146 exact raw captures; scoped
identity, non-COMPARED, evaluator and contradictory-identity cases remain held.
All statistics, loot probabilities, identities/revisions and unrelated fields are
preserved in both samples. Complete native-payload equality is checked explicitly
against Round6, alongside current source/data hashes and actual materialization.

Final qualifications and the reviewable full patch/package are recorded outside
this local uncommitted source tree in /workspace/monster-round7-output. They do
not assert full Global, full future schema or production mechanics qualification.
