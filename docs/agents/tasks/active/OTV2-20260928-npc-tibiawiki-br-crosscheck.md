# OTV2-20260928-npc-tibiawiki-br-crosscheck

```yaml
task_id: OTV2-20260928-npc-tibiawiki-br-crosscheck
title: NPC TibiaWiki BR facts and cross-check - replace the raw BR capture with compared facts and report every admitted NPC against them
mode: IMPLEMENT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/dazzling-brown-1u2xxo
issue: 162
pr: 1107
jira: KAN-16
base_sha: d17a38266d7197b807ff4d68c064eb1394849e2b
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude-code-session-01RTD1d7GsT7uFSBHg5syB4T
created_at: 2026-09-28
updated_at: 2026-09-28
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/OTV2-20260928-npc-tibiawiki-br-crosscheck.md
  - docs/agents/tasks/active/OTV2-20260928-npc-tibiawiki-br-capture.md
  - docs/agents/tasks/archive/OTV2-20260928-npc-tibiawiki-br-capture.md
  - tools/content-schema/npc-authoring/wiki_br.py
  - tools/content-schema/npc-authoring/README.md
  - .github/workflows/npc-tibiawiki-br-capture.yml
  - imports/tibiawiki/npc-br/**
  - tools/content-schema/npc-authoring/tibiawiki_br_crosscheck.py
  - tools/content-schema/npc-authoring/samples/tibiawiki-br-crosscheck-v1.json
  - docs/architecture/OTERYN_NPC_AUTHORING_SCHEMA_V1.md
public_contracts: []
depends_on:
  - OTV2-20260928-npc-tibiawiki-br-capture
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

#1103 added `wiki_br.py fetch`, which captures every NPC page of TibiaWiki BR (`Categoria:NPCs no Tibia`
and its subcategories, plus their `<NPC>/...` subpages) at exact revisions. Now a page that loses its
revision between enumeration and fetch is retried once and then recorded in `missing_pages`. The site refuses this
session's network but answers the repository's runners, as it did for the G4 non-Item capture, so
`npc-tibiawiki-br-capture.yml` runs the capture. The raw wikitext stays a CI artifact.

`wiki_br.py facts` reduces a capture to the compared facts, and those are committed immutable under
`imports/tibiawiki/npc-br/2026-09-28/` (1,253 pages, run 36418989024):

- each page's ids and raw SHA-256;
- `implemented` and `removed`;
- map positions;
- trade lists;
- the lines the NPC itself speaks in its transcript.

No wiki prose is stored. D3 records the access route and what is kept (the source-profile decision
forbids bulk-copying TibiaWiki prose).

`tibiawiki_br_crosscheck.py` compares every admitted NPC with those facts and writes
`samples/tibiawiki-br-crosscheck-v1.json`. The report changes nothing that is admitted.

| Check | Result |
| --- | --- |
| BR page found | 1,060 of 1,094 NPCs |
| Removed on BR (13.12) | 5 |
| Positions | 220 same tile, 677 within 3 tiles, 94 within 10, 25 on another floor, 41 far |
| Trade | 89 agree, 207 differ (28 explicit price differences), 63 with BR offers but none admitted |
| Dialogue | 346 NPCs checked: of 5,161 texts, 2,583 match a transcript line exactly and 327 nearly; 67 NPCs match none |

#1103 committed the raw capture itself before a review finding about this could be applied. This task
removes it from the tree (it remains in the history of `d17a3826`) and archives the #1103 task record.

Authority: owner request in this session to take all needed NPC data from TibiaWiki BR, keep it stored
and cross-check every NPC against it.

## Acceptance and evidence

- `wiki_br.py self-test` passes offline, including facts extraction (every speaker form, piped links in trade
  lists, formatted prices) and missing-page records.
- The committed facts were extracted from the run 36418989024 artifact. Every page's SHA-256 was checked
  against its wikitext before extraction.
- The cross-check is byte-identical on re-run, and it gives the same result from the facts as from the
  raw capture.
- Governance and the repository policy pass.
