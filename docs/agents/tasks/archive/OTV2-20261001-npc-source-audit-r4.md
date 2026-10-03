# OTV2-20261001-npc-source-audit-r4

```yaml
task_id: OTV2-20261001-npc-source-audit-r4
title: NPC schema repair and source-authoring audit draft
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: claude/dazzling-brown-1u2xxo
branch: codex/npc-source-audit-r4
issue: 162
pr: 1433
jira: KAN-16
base_sha: 25d79fb52104f1d2ce006f080f4bec665cc346ee
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: codex-root-npc-source-audit-r4
created_at: 2026-10-01
updated_at: 2026-10-01
execution_policy: continuous_progress
owned_paths:
  - tools/content-schema/npc-authoring/npc.schema.json
  - tools/content-schema/npc-authoring/validate_npc.py
  - tools/content-schema/npc-authoring/validate_promotion.py
  - tools/content-schema/npc-authoring/promotion_candidates.py
  - tools/content-schema/npc-authoring/tibiawiki_br_crosscheck.py
  - tools/content-schema/npc-authoring/test_schema_regressions.py
  - tools/content-schema/npc-authoring/export_source_audit.py
  - tools/content-schema/npc-authoring/validate_source_audit.py
  - tools/content-schema/npc-authoring/samples/promotion-candidates-v1.json
  - docs/agents/evidence/OTV2-20261001-npc-source-audit-r4/**
  - docs/agents/tasks/archive/OTV2-20261001-npc-source-audit-r4.md
public_contracts: []
depends_on:
  - OTV2-20260930-npc-reviewed-definitions
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Authority and outcome

The owner explicitly requested a draft PR after the offline audit on 2026-10-01 ("mozezz zrobic draft pr w repo?"). This supersedes the prior no-repository-write instruction only for this bounded publication. Root is the unique writer of the new branch; the existing #1358 writer branch, protected main, map work and runtime are not mutated. The control plane's closeout-only integration train is not extended or enqueued by this task.

The deliverable is draft #1433, stacked on #1358. It contains typed authoring validation repairs and a portable source-only packet: 1,112 NPC candidates, 902 nonempty Dialogue programs, 20,872 source roots, 39 empty dependency holds and 71 Service corrections. Five unallocated proposals are excluded. Full counts, source custody, attribution and limitations are in the packet README and manifest.

This record closes draft preparation, not NPC Global parity, native admission, playable runtime or protected integration. Before any integration, retarget after #1358, reconcile current content dependencies and qualify the exact candidate through required CI/native admission. No accepted public contract or production trust boundary changes here.

## Validation

Authoring checks: 100/100 NPC schema tests; 1,112/1,112 promotion candidates; 16/16 sample bundles; portable packet custody and all 20,872 source-root semantic hashes; BR crosscheck; changed-file Ruff; governance; diff whitespace. The old invalid fixture's 58 name-only variant assertions are replaced by the deterministic successor with holds preserved. No guard is removed.

Final candidate SHA, repeated exact-head checks and PR lifecycle state belong to the PR FREEZE_SHA entry; a commit cannot contain its own SHA. Historical R4 native checks are explicitly historical, and the Item-only Reference playable limit failure is retained. This stacked draft does not claim main-targeted game-gate or merge-group success. Independent agents checked patch compatibility, DTO selection and provenance; none approved the whole PR or triggered paid review.

## Closeout

- Draft publication acceptance: prepared; final exact-head publication/readback follows this commit.
- Required admission/CI/integration: pending, outside the draft-publication acceptance.
- Merge commit/result: pending; resolve #1433 only if it is later integrated.
- Integration, queue and review-trigger authority: retained by the active control plane.
- Jira synchronization: pending with the programme coordinator, mapping KAN-16.
- Preservation: frozen offline source packets remain unchanged; exact Git candidate recovery bundles are outside the worktree.
