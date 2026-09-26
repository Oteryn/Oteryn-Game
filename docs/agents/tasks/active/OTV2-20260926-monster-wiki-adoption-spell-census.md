# OTV2-20260926-monster-wiki-adoption-spell-census

```yaml
task_id: OTV2-20260926-monster-wiki-adoption-spell-census
title: Adopt reference-date wiki values and census Canary monster spell scripts
mode: CONTRACT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/nice-edison-h9aqh0
issue: 162
pr: 947
jira: KAN-16
base_sha: bbfdfdbfb92edae587fc1ec4c556ab1bf20b16b8
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude-code-session-01UiMEDawVAZ3kxLCLepWZnG
created_at: 2026-09-26
updated_at: 2026-09-26
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/OTV2-20260926-monster-wiki-adoption-spell-census.md
  - docs/agents/tasks/active/OTV2-20260926-monster-authoring-batch-2.md
  - docs/agents/tasks/archive/OTV2-20260926-monster-authoring-batch-2.md
  - docs/architecture/OTERYN_MONSTER_AUTHORING_SCHEMA_V1.md
  - tools/content-schema/monster-authoring/**
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Owner requests after PR #943: (1) take the reference-date wiki values where they differ from
Canary (mitigation, fire elemental `pushable`, dragon and ghost loot), (2) define how registered
monster spell scripts are stored and maintained, starting with a census of the Canary spell
scripts, (3) record the approved decisions D10–D15.
Authority: direct owner request in this session; product/runtime implementation stays unallocated.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: offline authoring evidence and Python tooling only; no production mutation,
fence, session, authority or persisted-recovery evidence is touched.

## Acceptance and evidence

- D15 applied to batch 1 from `wiki-2026-07-28.json`: 9 mitigations, fire elemental `pushable`,
  dragon trophy (item 44158 by the wiki item page) and stone skin amulet with probabilities
  from the `Loot Statistics` cut revisions. The comparison still runs against the plain Canary
  conversion (239 MATCH / 12 DIFF / 8 WIKI_UNKNOWN, unchanged).
- The import manifest accepts MediaWiki sources pinned by page id, revision id and wikitext
  SHA-256; `verify_formal_schema.py` 177/177. Batch 1 10/10 bundles and manifests resolve; batch 2
  unchanged.
- `spell_census.py` over 1,656 Canary monster files: 274 referenced spells, P1 15 / P2 166 /
  P3 48 / P4 44 / NOOP 1 / MISSING 0. Owner decisions D10–D14 recorded in the architecture
  document §3 and §8.
