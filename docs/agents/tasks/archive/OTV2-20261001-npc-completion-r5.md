# OTV2-20261001-npc-completion-r5

```yaml
task_id: OTV2-20261001-npc-completion-r5
title: NPC source recovery and guarded content corrections
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/npc-source-audit-r4
issue: 162
pr: 1433
jira: KAN-16
base_sha: ab83bcae5b4331ee82dfe346f56177a13f9f83dd
owner: codex-root-npc-completion-r5
created_at: 2026-10-01
updated_at: 2026-10-01
owned_paths:
  - tools/content-schema/npc-authoring/**
  - tools/content-migration/npc_*.py
  - tools/content-migration/test_npc_*.py
  - tools/content-migration/recovered_dialogue_associations.py
  - apps/game-server/examples/materialize_content_world_project_v2.rs
  - apps/game-server/tests/content_world_project_repository.rs
  - tools/content-migration/world_project_v2_to_tree.py
  - tools/content-migration/validate_world_project_v2_to_tree.py
  - content/encounters/definitions/index.json
  - content/cosmetics/mounts/index.json
  - content/world/**
  - content/npcs/**
  - content/dialogues/**
  - content/services/**
  - content/manifest.json
  - content/content.lock.json
  - content/provenance/imports.json
  - docs/agents/evidence/OTV2-20261001-npc-source-audit-r5/**
  - docs/agents/tasks/archive/OTV2-20261001-npc-completion-r5.md
public_contracts: []
depends_on: []
blocks: []
external_repositories: []
```

## Owner authority

The owner explicitly requested completing the NPC audit with subagents and directed us to find alternative sources when the wiki HTTP proxy denied access. This extends the earlier R4 source-only draft request to concrete, source-qualified NPC content corrections. Root remains the only branch writer. This task grants no merge, queue, paid-review trigger, production or protocol allocation authority.

PR #1358 has merged. Ordinary merges reconcile its definitions and subsequent main documentation; source R4 packet custody stays unchanged. The new R5 packet separates concrete admitted corrections from incomplete identity/transcript observations and source quest associations. Accepted native DTOs and the existing canonical materializer define admission. No legacy storage number is minted as a Game Quest identity.

## Qualification boundary

This closes qualified authoring repairs and alternative-source recovery. It does not claim complete Global NPC coverage, executable quest guards, new placements, conversation/trade/travel runtime or gameplay E2E. The NPC runtime decision explicitly requires separate child allocations (#162); source quest associations are evidence fields with runtime eligibility false. Missing facts and disputed prices remain visible in the R5 evidence rather than being guessed.

Exact-head publication, CI, review and any protected integration remain live PR state. A commit cannot record its own SHA. The final report records focused local validation and a recovery bundle; no worker requests a paid review or enqueues this PR.

## Local validation

130 authoring tests, 51 NPC migration tests, 45 native canonical-project/import/guard tests and seven native NPC admission tests pass. Two original materializer runs produce identical eleven-document packages, tree SHA `bdf3d42d43cde152d93a7871f917566b9fda5193a6045c36841d34e46d70dd08`. Promotion validates 1,112 candidates; immutable R4 custody/root validation, successor-tree roundtrip, all 97 materialized directories, governance, changed-Python Ruff and workspace formatting pass. Strict package Clippy is recorded in the evidence validation after completion.

Independent packet review found an arbitrary supplied-plan family bypass; its repair rejects Item/Quest operations, identity drift, unsupported fields and matcher/action changes before output. Source observations remain evidence and runtime eligibility false. The final exact head is the PR freeze entry; integration and CI must be read from the live PR.
