# OTV2-20261001-npc-source-audit-r8

```yaml
task_id: OTV2-20261001-npc-source-audit-r8
title: Admit two source-qualified Rapanaio static dialogues
mode: REPAIR
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/npc-source-audit-r4
issue: 162
pr: 1433
jira: KAN-16
base_sha: e9e69656dbe33ca2666248e23f7638be880f6bc9
owner: codex-root-npc-r8
created_at: 2026-10-01
updated_at: 2026-10-01
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/examples/materialize_content_world_project_v2.rs
  - apps/game-server/examples/npc_materializer/transcript_repairs.rs
  - apps/game-server/tests/content_world_project_repository.rs
  - tools/content-migration/world_project_v2_to_tree.py
  - tools/content-migration/validate_world_project_v2_to_tree.py
  - tools/content-migration/test_world_project_v2_to_tree.py
  - content/**
  - docs/architecture/OTERYN_NPC_AUTHORING_SCHEMA_V1.md
  - docs/agents/evidence/OTV2-20261001-npc-source-audit-r8/**
  - docs/agents/tasks/archive/OTV2-20261001-npc-source-audit-r8.md
public_contracts: []
depends_on: []
blocks: []
external_repositories: []
```

## Outcome and authority

The current owner's existing direction explicitly uses Fandom, the named wikis, TibiaSecrets and other internet sources for the continuing NPC audit. After clarification, root reused that existing source-authoring authorization instead of requesting it again. D10 adds one pinned Fandom page/revision for two existing Rapanaio variants only. This source qualification does not change production/runtime/Quest authority or the accepted Dialogue key scheme. Subagents read sources, produced isolated prototypes and independently reviewed; root alone authors and publishes on the inherited branch.

R8 adds two whole, source-bound static Crystal Dialogues and updates only two existing NPC references. The successor has696 Dialogues and1110 NPCs. Native writer/parser regeneration proves every other existing declaration/profile unchanged and two byte-identical materializations. The prototype includes atomic drift/duplicate rejection; integrated checks and immutable packet digest qualify the actual authoring.

The original five-area task remains incomplete: three researched literal conflicts, other missing dialogue classes,158 new plus2 deferred actors needing qualified profiles/movement, and accepted native quest/service behavior dependencies remain. R4–R7 history remains immutable. TibiaSecrets's refreshed catalogue/transcript custody gives no new held-source winner and no additional independent vote.

## Validation and publication

See the R8 evidence manifest and validation packet. The tracked evidence excludes raw wiki prose/assets. Remote Desktop is confined to public browser research; Linux workspace owns all edits and checks. Root explicitly returned to AUTHORING after fresh unchanged-head read, validates locally, uses guarded high-level expected-head publication, verifies the complete remote delta and freezes its exact SHA. Candidate-specific formal review/CI remain fresh-head obligations; root triggers no paid review, queue, merge or Jira action.

High-risk production authority/recovery qualification: NOT_APPLICABLE. The delta is static source authoring and reproducible reference data, with no runtime guard, session, persistence, protocol or native Quest allocation change.
