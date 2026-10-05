# OTV2-20261001-npc-source-audit-r19

```yaml
task_id: OTV2-20261001-npc-source-audit-r19
title: Admit eight source-qualified NPC definitions in one batch
mode: REPAIR
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/npc-source-audit-r4
issue: 162
pr: 1433
jira: KAN-16
base_sha: 61fb530104d42800aecbebbd2feb7f816b450b46
owner: codex-root-npc-r19
created_at: 2026-10-01
updated_at: 2026-10-01
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/examples/materialize_content_world_project_v2.rs
  - apps/game-server/examples/npc_materializer/qualified_bounded.rs
  - apps/game-server/tests/content_world_project_repository.rs
  - tools/content-migration/world_project_v2_to_tree.py
  - tools/content-migration/test_world_project_v2_to_tree.py
  - tools/content-migration/validate_world_project_v2_to_tree.py
  - content/**
  - imports/tibiawiki/batches.json
  - imports/tibiawiki/sources.json
  - docs/agents/evidence/OTV2-20261001-npc-source-audit-r19/**
  - docs/agents/tasks/archive/OTV2-20261001-npc-source-audit-r19.md
public_contracts: []
depends_on: []
blocks: []
external_repositories: []
```

Root is the sole repository/branch writer; five external source groups and an independent reviewer support the owner-requested faster, larger batches. One complete candidate adds Ben,Arenamaster,Gerib,Lai,Alberto,Emilio,Fabiana,Lorenzo. Qualification, source access, repaired source key, actual24-source/25-empty-batch quota semantics and remaining27/133 original160 definitions are recorded in evidence/r19/README.md and the exact-source packets. Historical holds and guarded services are preserved except explicitly superseded appearance/role findings. No runtime or public contract is altered.

The qualified61fb predecessor is retained externally before generation. One complete owned delta is validated before normal guarded-Git candidate selection/publication. A commit cannot contain its own SHA: exact freeze/readback, frozen tests and independent head review are retained separately. No write occurs while frozen. Formal paid review dispatch, protected CI/main reconciliation/Merge Queue/Jira remain with control plane#162.

Authoring validation and reproducible generation are recorded in evidence/r19/authoring-validation.json. Initial sourcekey rejection and invalid negative harness fixture are retained with explicit repaired dispositions, not hidden retries.

Selected authoring validation PASS:70 native tests,4 repository-package tests,172 NPC-authoring tests with zero skips,59 migration tests,36 governance tests,format/strict all-target Clippy. Source compiler16 and typed16 plus one full1141 canonical scenario pass. Full/fast parity and deterministic fast repeat cover all11 documents. Full generation elapsed378.729seconds. Canonical tree a91bd820d8ea96044ec456bce9264a103d8ae1e8fe9b40c3cd7bd6e50360b4b6. Earlier corrected failures remain retained. Exact remote freeze/frozen-head checks are separate.
