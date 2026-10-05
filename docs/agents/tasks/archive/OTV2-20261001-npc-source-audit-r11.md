# OTV2-20261001-npc-source-audit-r11

```yaml
task_id: OTV2-20261001-npc-source-audit-r11
title: Compile all held NPC service source dependencies
mode: REPAIR
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/npc-source-audit-r4
issue: 162
pr: 1433
jira: KAN-16
base_sha: eeb62e54fb87fb32dc174b80d9bceaf854ef8260
owner: codex-root-npc-r11
created_at: 2026-10-01
updated_at: 2026-10-01
execution_policy: continuous_progress
owned_paths:
  - tools/content-schema/npc-authoring/build_service_relations.py
  - tools/content-schema/npc-authoring/service_source_conditions.py
  - tools/content-schema/npc-authoring/test_service_relations.py
  - docs/agents/evidence/OTV2-20261001-npc-source-audit-r11/**
  - docs/agents/tasks/archive/OTV2-20261001-npc-source-audit-r11.md
public_contracts: []
depends_on: []
blocks: []
external_repositories: []
```

The continuing owner request covers every unfinished NPC area with subagents. This bounded offline tool completes dependency/custody compilation for 457 offers, three routes and 171 discounts. It retains holds and null native bindings, validates the actual source branches and distinguishes pre-commit refusal from recovery after a known durable commit. Independent review closed the single P2 custody finding across all 171 sibling rows; dedicated regressions and deterministic output pass.

No native content, stable identifiers, protocol, persistence or runtime authority changes. The original five-area request remains incomplete, with qualified actor profiles/movement and accepted NPC/quest registries/runtime owners still required. Root alone integrates/publishes after fresh unchanged-head transition to AUTHORING, verifies the complete remote delta, freezes the exact successor and reruns selected checks. Review dispatch, Merge Queue/integration and Jira stay with the control plane. High-risk runtime/production mutation qualification: NOT_APPLICABLE. See the evidence manifest for checks and repaired independent review.
