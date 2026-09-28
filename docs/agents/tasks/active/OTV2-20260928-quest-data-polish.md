# OTV2-20260928-quest-data-polish

```yaml
task_id: OTV2-20260928-quest-data-polish
title: Quest format - typed wiki requirements, chest quest links, runtime-map trigger ids, Banshee data gaps
mode: CONTRACT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/zealous-edison-3ttg1s
issue: 162
pr: null
base_sha: ec70ca9e17548c3ab2e0597b4762169b4cac2c3e
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude-code-session-01AVd6BKKTRbeW1Pub9bg9Jk
created_at: 2026-09-28
updated_at: 2026-09-28
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/OTV2-20260928-quest-data-polish.md
  - docs/agents/tasks/archive/OTV2-20260927-quest-readiness-and-gaps.md
  - docs/architecture/OTERYN_QUEST_AUTHORING_FORMAT_V1.md
  - tools/content-schema/quest-authoring/**
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Owner request (2026-09-28, this session): while the engine lanes wait (#162 B3, architecture A1),
finish the quest data work that needs no runtime:

1. typed quest requirements (minimum level, premium) next to the recorded wiki strings;
2. evidence-based quest links for the chest claims that have none or only a section guess;
3. the trigger ids missing from the startup map, checked against runtime-loaded maps and scripts;
4. the data gaps of The Queen of the Banshees, the first full-quest target.

Runtime, persistence, `content/**`, NPC-owned files and DUR-03/Character paths stay unchanged.

## Architecture and source of truth

- `PROVEN`: pinned Canary and CrystalServer revisions; Canary and CrystalServer maps pinned by sha256.
- `DERIVED`: typed requirements and curated links from the Fandom API with page and revision ids.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: offline tooling and a CANDIDATE format document.

## Acceptance and evidence

- Converters deterministic; `verify_quest_schema.py` and `validate_quest_content.py` pass.
- No narrative text is committed.
- `validate_governance.py` and `validate_repository_policy.py` pass.
- Jira: no mapped Story (pending).
