# OTV2-20260928-item-target-date

```yaml
task_id: OTV2-20260928-item-target-date
title: Item evidence chain at the 2026-09-27 target date (target-date step 3a)
mode: CONTRACT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/nice-edison-h9aqh0
issue: 162
pr: 1061
jira: KAN-16
base_sha: 45b6cc73d8dbd2988ec0153cfa5ad03318367d51
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude/nice-edison-h9aqh0
created_at: 2026-09-28
updated_at: 2026-09-28
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/OTV2-20260928-item-target-date.md
  - docs/agents/tasks/active/OTV2-20260928-tile-damage.md
  - docs/agents/tasks/archive/OTV2-20260928-tile-damage.md
  - docs/agents/programs/OTERYN_TARGET_DATE_20260927_DECISION.md
  - docs/agents/evidence/OTV2-20260927-content-world-item-*.json
  - docs/agents/evidence/OTV2-20260927-g4-*.json
  - docs/agents/evidence/OTV2-20260927-item-*.json
  - tools/reference-world-corridor-census/item_current_source_tibiawiki.py
  - tools/reference-world-corridor-census/item_current_source_tibiawiki_self_test.py
  - tools/reference-world-corridor-census/item_field_verification.py
  - tools/reference-world-corridor-census/item_field_verification_self_test.py
  - tools/reference-world-corridor-census/item_target_continuity.py
  - tools/reference-world-corridor-census/item_target_continuity_self_test.py
  - tools/reference-world-corridor-census/item_semantic_promotion.py
  - tools/reference-world-corridor-census/item_semantic_promotion_self_test.py
  - tools/content-census/g4_item_binding_pilot.py
  - tools/content-census/g4_item_wave1_stage.py
  - tools/content-census/g4_item_wave1_capture.py
  - .github/workflows/item-content-verification.yml
  - .github/workflows/item-content-continuity.yml
  - .github/workflows/item-content-promotion.yml
  - .github/workflows/g4-item-binding-pilot.yml
  - .github/workflows/g4-item-wave1-capture.yml
  - apps/game-server/**
  - content/world/**
  - content/project.json
  - content/manifest.json
  - content/content.lock.json
  - content/items/**
  - content/cosmetics/**
  - content/npcs/**
  - content/services/**
  - content/creatures/definitions/**
  - content/presentations/definitions/**
  - content/behaviors/**
  - content/loot/**
  - content/abilities/**
  - imports/**
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

This is step 3a of `OTERYN_TARGET_DATE_20260927_DECISION.md`. The owner approved it, including edits to the protected
item workflows ("Tak, masz zgodę").

The protected Item evidence chain moves from the 2026-07-28 cut to the 2026-09-27 target:

1. the TibiaWiki BR current-source collector;
2. the field verification;
3. the target-day continuity;
4. the semantic promotion packet (since #1064 no longer read by `cw2_b1_import`, which now takes Item promotion from the #1048 lowering pass).

The G4 binding pilot pins the new current source. Each re-collected stage has the same partition as at 2026-07-28, and
the 69 promoted field values on 23 Items are unchanged, so `content/world` does not change. The G4 wiki captures (165
exact Items, Item Wave 1, Mounts, Outfits) and the materializer post-cut limits are step 3b, a separate change.

Each stage is re-collected on a hosted runner, because the build container gets a Cloudflare bot check. Its fresh
manifest is protected as new 2026-09-27 evidence, and the next stage is pinned to it. The 2026-07-28 evidence files
stay as the historical record.

## High-risk authority/recovery qualification

Protected evidence and `content/world` Item values change. The exact-head review runs before the Merge Queue.

## Acceptance and evidence

- Every item workflow passes against the new protected evidence.
- The Rust tests pass; `content/world` is unchanged.
- The governance and policy validators pass.
