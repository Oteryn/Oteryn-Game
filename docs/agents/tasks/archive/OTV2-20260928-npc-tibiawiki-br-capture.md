# OTV2-20260928-npc-tibiawiki-br-capture

```yaml
task_id: OTV2-20260928-npc-tibiawiki-br-capture
title: NPC TibiaWiki BR capture - exact-revision wikitext snapshot of every NPC page for Tibia Global fidelity checks
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/dazzling-brown-1u2xxo
issue: 162
pr: 1103
jira: KAN-16
base_sha: 43807fb40549ba9aa7c102862c705ddf146c4866
head_sha: null
final_head_sha: d17a38266d7197b807ff4d68c064eb1394849e2b
final_head_frozen_at: null
owner: claude-code-session-01RTD1d7GsT7uFSBHg5syB4T
created_at: 2026-09-28
updated_at: 2026-09-28
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/OTV2-20260928-npc-tibiawiki-br-capture.md
  - docs/agents/tasks/active/OTV2-20260928-npc-dialogue-transcripts.md
  - docs/agents/tasks/archive/OTV2-20260928-npc-dialogue-transcripts.md
  - tools/content-schema/npc-authoring/wiki_br.py
  - tools/content-schema/npc-authoring/README.md
  - .github/workflows/npc-tibiawiki-br-capture.yml
  - imports/tibiawiki/npc-br/**
public_contracts: []
depends_on:
  - OTV2-20260928-npc-dialogue-transcripts
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

`wiki_br.py` captures every NPC page of TibiaWiki BR (`Categoria:NPCs no Tibia` and its subcategories,
plus their `<NPC>/...` subpages) as an exact-revision wikitext snapshot: page id, revision, timestamp,
SHA-256 and wikitext per page, plus a digest over all pages. The site refuses this session's network but
answers the repository's runners, as it did for the G4 non-Item capture, so
`npc-tibiawiki-br-capture.yml` runs the capture and publishes the snapshot as an artifact.

So that the data does not expire with the artifact, the first capture is committed immutable under
`imports/tibiawiki/npc-br/2026-09-28/` (1,253 NPC pages, 3.5 MB), with a README recording its run,
artifact, file SHA-256 and `pages_digest`. 259 pages carry a dialogue transcript in `falas`, and about
440 carry trade lists.

The snapshot is evidence only. It gives the position, trade lists and dialogue that Fandom only stubs for
recent NPCs (for example the 15.30 Marapur NPCs), so later NPC work can check Canary, Crystal and branch
facts against Tibia Global. The pages carry no outfit field. Nothing is admitted into `content/` by this task.

The D10 dialogue task record (#1095) is archived with its merge commit.

Authority: owner request in this session to take all needed NPC data from TibiaWiki BR and keep it stored.

## Acceptance and evidence

- `wiki_br.py self-test` passes offline.
- The workflow run on this PR (run 36418989024) captured the snapshot. The committed file matches the
  artifact byte for byte, every page's SHA-256 matches its wikitext, and `pages_digest` recomputes.
- Governance and the repository policy pass.
