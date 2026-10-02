# OTV2-20261002-npc-enrichment-r28

```yaml
task_id: OTV2-20261002-npc-enrichment-r28
title: Enrich quest-linked NPC static dialogue and Blue Stone price references
mode: IMPLEMENT
status: ready
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/npc-source-audit-r4
issue: 162
pr: 1433
jira: KAN-16
base_sha: 82a7e1fcee352512bf6c5ea0406271e326192b53
owner: codex-root-npc-enrichment
created_at: 2026-10-02
updated_at: 2026-10-02
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/examples/materialize_content_world_project_v2.rs
  - apps/game-server/examples/npc_materializer/bulk_enrichment.rs
  - apps/game-server/tests/content_world_project_repository.rs
  - tools/content-migration/npc_quest_dialogue_followup.py
  - tools/content-migration/test_npc_quest_dialogue_followup.py
  - tools/content-migration/test_npc_visual_invisible_stage.py
  - tools/content-migration/test_npc_visual_appearance_stage.py
  - tools/content-migration/test_npc_visual_appearance_followup.py
  - tools/content-migration/test_npc_source_refine.py
  - content
  - docs/agents/evidence/OTV2-20261002-npc-enrichment-r28
  - docs/agents/tasks/archive/OTV2-20261002-npc-enrichment-r28.md
public_contracts: []
depends_on: []
blocks: []
external_repositories: []
```

Owner authorizes large source-informed NPC batches and flagged approximations. Sole root writer returns frozen R27 to AUTHORING. Subagents research and review scratch-only. Existing D280 content-authoring continuation; no production authority.

Twenty-one source-selected static greetings plus five Blue Stone documentary prices. Source quest/event/day-night conditions remain documentary. Profiles, roles, Behavior and services remain preserved. Current source, implementation and exact successor generation evidence is retained in validation.json. The existing entry-room test map is reused; runtime NPC placement/interaction/server qualification remains pending.

NOT_APPLICABLE production authority/recovery invariants: source/data/example authoring only, no protocol, persistence, session or authority change. Coordinator162/protectedCI/current-main/Jira/MQ remain pending.
