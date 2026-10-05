# OTV2-20261002-npc-data-import-r29

```yaml
task_id: OTV2-20261002-npc-data-import-r29
title: Import prepared NPC data into a pinned native server catalogue
mode: IMPLEMENT
status: ready
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/npc-source-audit-r4
issue: 162
pr: 1433
jira: KAN-16
base_sha: 6d50b05b47d7351043e93b495d24d645a3d974b3
owner: codex-root-npc-enrichment
created_at: 2026-10-02
updated_at: 2026-10-02
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/content/npc_catalogue.rs
  - apps/game-server/src/content/mod.rs
  - apps/game-server/src/main.rs
  - apps/game-server/src/node/serve.rs
  - apps/game-server/tests/content_npc_catalogue.rs
  - docs/migration/NPC_DATA_ONLY_IMPORT.md
  - docs/agents/evidence/OTV2-20261002-npc-data-import-r29
  - docs/agents/tasks/archive/OTV2-20261002-npc-data-import-r29.md
public_contracts: []
depends_on: []
blocks: []
external_repositories: []
```

Owner directly requests importing prepared NPC data into the server, accepting incomplete runtime systems. Root explicitly returns frozen R28 to AUTHORING as sole writer; subagents propose/review scratch only. Scope: immutable data-only catalogue over existing accepted WorldProject/v2 types, source-tree pin, CLI and opt-in process retention. No database/world/production mutation, activation/spawn, protocol/identity/authority/persistence change or handlers. Existing NPC data and native entry-room map remain unchanged.

Qualification and independent review are recorded in evidence/validation.json. Formal coordinator162 acceptance, protected CI/main integration, Jira/Merge Queue and live-runtime qualification remain pending. NOT_APPLICABLE production authority/recovery invariants: no authority-bearing or persistent mutation boundary is added.
