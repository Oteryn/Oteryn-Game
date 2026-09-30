# OTV2-20260930-soul-war-mechanics-design

```yaml
task_id: OTV2-20260930-soul-war-mechanics-design
title: Soul War mechanics design (SW-1..6) and the creatures without a Canary file, with the owner answers
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
issue: 162
base_branch: main
branch: claude/friendly-albattani-rfzvku
pr: null  # the PR on this branch is authoritative
base_sha: eba9a271
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: "Oteryn: content world import"
created_at: 2026-09-30T07:00:00Z
updated_at: 2026-09-30T09:00:00Z
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/OTERYN_MONSTER_AUTHORING_SCHEMA_V1.md (new section 10)
  - docs/architecture/OTERYN_ENCOUNTER_AUTHORING_FORMAT_V1.md (new section 13, header status line)
  - tools/content-schema/monster-authoring/samples/wiki-only-candidates-2026-09-30.json
  - docs/agents/tasks/archive/OTV2-20260930-soul-war-mechanics-design.md
public_contracts: []
depends_on:
  - "owner answers in the task session 2026-09-30: 5a 6a 7a 8a 9c 10a 11c 12a 13a 14a 15b 16a (section 13.6 Q1-Q6, the Infernal Demon and Many Faces answers, section 10.3)"
blocks:
  - "SW-2 implementation (first), then SW-1, SW-5, SW-3, SW-4, SW-6"
  - "crystal_batch.py widening for the 50 creatures with a Crystal file elsewhere"
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Design only (tasks C and D preparation of the monster-unblocking plan). Monster schema section 10 holds SW-1 (fear
windup) and SW-2 (magic wall removal, no schema change) and the answers for the creatures without a Canary file.
Encounter format section 13 holds SW-3..6, the owner questions with the answers, and the second source check
(official 2020 news through Tibiopedia, Fandom page histories, Tibiopedia, TibiaQA, Gudii's guide transcripts,
CrystalServer, the TibiaWiki BR capture). The answer numbering in the session (5-16) maps to section 13.6 Q1-Q6, the
Infernal Demon and Many Faces answers, section 10.3 (11-13) and the BR decision (16).

## Validation

`git diff --check`; `validate_governance.py`. No code or content changes.
