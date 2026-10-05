# OTV2-20261001-npc-source-audit-r9

```yaml
task_id: OTV2-20261001-npc-source-audit-r9
title: Compile byte-qualified official NPC marker observations
mode: REPAIR
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/npc-source-audit-r4
issue: 162
pr: 1433
jira: KAN-16
base_sha: 03530a12f8832c2915b79bd3128f9ab967bfbcce
owner: codex-root-npc-r9
created_at: 2026-10-01
updated_at: 2026-10-01
execution_policy: continuous_progress
owned_paths:
  - tools/content-schema/npc-authoring/official_map_markers.py
  - tools/content-schema/npc-authoring/test_official_map_markers.py
  - docs/agents/evidence/OTV2-20261001-npc-source-audit-r9/**
  - docs/agents/tasks/archive/OTV2-20261001-npc-source-audit-r9.md
public_contracts: []
depends_on: []
blocks: []
external_repositories: []
```

The owner asked to execute all remaining NPC work using subagents. This bounded batch implements an offline source compiler and validator for the pinned official client map: 117 qualified target names, 135 observations and exact byte custody. Eleven focused tests cover provenance drift, malformed input, alias qualification, deterministic output, changed facts/authority and variant boundaries. Independent review additionally validates the eight manually qualified page/name bindings and 675 byte witnesses.

The compiler creates evidence only. Unknown appearance/movement and native frame/allocation dependencies remain explicit; no native content or runtime code changes. The overall five-area task stays open. Root alone authors/publishes the inherited branch, returning explicitly to AUTHORING after fresh unchanged R8 head, then freezing and validating the complete successor. Review dispatch, queue/merge and Jira remain with the active control plane. See the evidence manifest for exact checks. High-risk runtime/persistence/production authority changes: NOT_APPLICABLE.
