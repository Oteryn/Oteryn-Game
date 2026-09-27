# OTV2-20260927-vlarkorth-wiki-capture

```yaml
task_id: OTV2-20260927-vlarkorth-wiki-capture
title: Count Vlarkorth and Dark Merudri from the 2026-09-27 wiki
mode: CONTRACT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/nice-edison-h9aqh0
issue: 162
pr: 1047
jira: KAN-16
base_sha: 6fc6868ff8e5c8840c03d253049a6dc106c151e7
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude/nice-edison-h9aqh0
created_at: 2026-09-27
updated_at: 2026-09-27
execution_policy: continuous_progress
owned_paths:
  - .github/workflows/monster-wiki-capture.yml
  - docs/agents/tasks/active/OTV2-20260927-vlarkorth-wiki-capture.md
  - docs/agents/tasks/active/OTV2-20260927-fifteenth-encounter-slice.md
  - docs/agents/tasks/archive/OTV2-20260927-fifteenth-encounter-slice.md
  - docs/architecture/OTERYN_ENCOUNTER_AUTHORING_FORMAT_V1.md
  - docs/architecture/OTERYN_MONSTER_AUTHORING_SCHEMA_V1.md
  - tools/content-schema/encounter-authoring/**
  - tools/content-schema/monster-authoring/**
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

The owner chose to add Dark Merudri from the wiki ("Dodaj Dark Merudri z wiki"). At the 2026-09-27 target, Count
Vlarkorth calls one dark creature per player vocation, and the monk's is Dark Merudri. Neither Canary nor Crystal
has that creature or its remains item.

- TibiaWiki (Fandom) gives only its health, experience and remains.
- TibiaWiki BR answers the build container with a Cloudflare bot check. A hosted-runner workflow therefore captures
  the BR infobox fields of the Grave Danger pages at the target date.

Authority: owner answers in this session. Runtime behaviour stays unallocated. The Item registry is untouched.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: evidence tooling, a read-only capture workflow and documentation only.

## Acceptance and evidence

- `wiki_br_capture.py self-test` passes.
- The workflow uploads the BR capture artifact.
- The governance and policy validators pass.
