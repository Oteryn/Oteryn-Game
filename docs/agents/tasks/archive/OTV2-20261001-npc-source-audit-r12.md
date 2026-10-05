# OTV2-20261001-npc-source-audit-r12

```yaml
task_id: OTV2-20261001-npc-source-audit-r12
title: Correct Commoner shop eligibility and missed fluid offers
mode: REPAIR
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/npc-source-audit-r4
issue: 162
pr: 1433
jira: KAN-16
base_sha: 3af48922633e30acd8da2f97c9ac29676a894b96
owner: codex-root-npc-r12
created_at: 2026-10-01
updated_at: 2026-10-01
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/examples/materialize_content_world_project_v2.rs
  - apps/game-server/examples/npc_materializer/service_scope_repairs.rs
  - apps/game-server/tests/content_world_project_repository.rs
  - tools/content-schema/npc-authoring/rendered_wiki_prices.py
  - tools/content-schema/npc-authoring/test_rendered_wiki_prices.py
  - tools/content-schema/npc-authoring/trade_variant_repair.py
  - tools/content-schema/npc-authoring/test_trade_variant_repairs.py
  - content/world/**
  - content/services/trade/**
  - content/cosmetics/mounts/index.json
  - content/dialogues/definitions/index.json
  - content/encounters/definitions/index.json
  - content/npcs/definitions/index.json
  - content/services/travel/index.json
  - content/project.json
  - content/manifest.json
  - content/content.lock.json
  - docs/agents/evidence/OTV2-20261001-npc-source-audit-r12/**
  - docs/agents/tasks/archive/OTV2-20261001-npc-source-audit-r12.md
public_contracts: []
depends_on: []
blocks: []
external_repositories: []
```

Owner-directed continuation repairs two classified finding families. Thirty-three source tuples stay held rather than being loaded as unconditional plain offers. Prices, known charge counts, identities and all unrelated fields are preserved. Root is the only repository/branch writer; subagents independently qualify sources and prepare external prototypes.

Publication follows AUTHORING, exact remote delta verification, FREEZE_SHA and selected validation/review. Review dispatch, integration and Jira remain with the active control plane. No runtime, protocol, persistence, public identity allocation or production mutation occurs. The original completion request remains open; the next bounded batch qualifies additional summer-update NPC definitions.
