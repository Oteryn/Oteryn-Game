# G4 Item exact-165 canonical population

```yaml
task_id: OTV2-20260925-g4-item-exact-165-population
title: Populate 165 exact Wiki Item bindings and independently verified fields
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/full-content-item-g4-165-population-20260925
pr: 895
base_sha: 14b48406ade8c6984fd725175990d8b524ba020f
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: allocated sole Item package writer, #162 comment 5830832033
created_at: 2026-09-25T10:34:00Z
updated_at: 2026-09-25T10:34:00Z
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/examples/materialize_content_world_project_v2.rs
  - apps/game-server/tests/content_world_project_repository.rs
  - docs/agents/evidence/OTV2-20260925-g4-item-exact-165-selected.json
  - docs/agents/tasks/active/OTV2-20260925-g4-item-exact-165-population.md
  - content/world/definitions/reference.json
  - content/world/editor/author.json
  - content/world/provenance/imports.json
  - content/world/provenance/sources.json
  - content/world/manifest.json
  - content/world/content.lock.json
  - content/world/project.json
public_contracts: []
depends_on: ["#874", "#877 artifact 10846904696"]
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Product delta

Consume the exact #877 hosted artifact 10846904696, ZIP SHA-256 `da83dcdb992b7cae3827ea1b12c95959160dd81eec831d61f6ef90ca5e43a8b2`, population JSON SHA-256 `853aa2d1cc3cde0e3b77ae7d4f16eaaf21f1f63eaf45ef19e87e7ad12ba1cf16`. The selected compact repository input is SHA-256 `c624a978bfc83d2dc57865126c6cda63dc21eb4134ee6eb7dc91a6b198ec945c` and preserves exact page ID, revision, timestamp and digest beside each exact binding/typed field. Existing 38,157 Item identities are retained. Write 165 exact source bindings, 165 discoverable editor entries with the established `oteryn:editor.item` tag, and 526 atomic ReferenceItem fields previously UNKNOWN; preserve 35 already-known equal fields. The one post-target page (6171) adds only its binding: its three semantic candidates are already known and equal.

The Wiki source batch carries full-census digest `583a0b0080f3e08633c8d6cde11d9fd073b47088d84774bfdf851382569dd675` and stable census revision `tibiawiki-item-census:389875abd364aa9bcb0b09a591989c82ece5098d63b3c23376274048f6ac2f5a`. This is current structured Wiki evidence, separate from the existing OTS hypothesis source. No Item identity mint, client/runtime ID, Presentation/Asset binding, world placement, or activation.

## Validation and custody

One writer owns the seven affected package documents and four support files. Canonical typed writer generates the same eleven document locations twice and compares bytes with the tracked package. Check exact 165/526/35 partition, round-trip parse, unchanged four other documents, focused WorldProject/v2 and B1 tests, strict Clippy/format and full hosted `game-gate`. Frozen candidate requires independent exact-head identity and semantic review before coordinator-owned Merge Queue. No direct protected merge.
