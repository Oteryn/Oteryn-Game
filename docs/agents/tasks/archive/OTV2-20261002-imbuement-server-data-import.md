---
task_id: OTV2-20261002-imbuement-server-data-import
title: Import reviewed imbuement data into the static server ruleset tree
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/imbuement-server-data-import-20261002
pr: null
base_sha: e225b3f76e152d195f75cb757b3d639a3ff5577a
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: root-imbuement-data-import
created_at: 2026-10-02T18:06:13Z
updated_at: 2026-10-02T18:06:13Z
execution_policy: continuous_progress
owned_paths:
  - tools/content-schema/imbuement-authoring/**
  - rulesets/items/imbuements/**
  - tools/content-migration/world_project_v2_to_tree.py
  - tools/content-migration/validate_world_project_v2_to_tree.py
  - tools/content-migration/regenerate_content.py
  - content/project.json
  - content/manifest.json
  - content/content.lock.json
  - .github/workflows/content-tree-migration.yml
  - docs/agents/tasks/archive/OTV2-20261002-imbuement-server-data-import.md
public_contracts: []
depends_on:
  - IMBUE-FORGE-0
blocks: []
---

## Outcome

Owner-directed data-only import: populate the accepted server ruleset directory
with 24 families / 72 tiers and 627 bound equipment profiles. Preserve all source
qualifications, ten owner-approved operational policies and held observations.

## Authority and scope

Accepted tree contract places Imbuements under `rulesets/items/imbuements/`.
Source: unmerged draft PR #1438, exact head
`8ae14d820500bd94a9da06c2323e3ef27d1b4018`; its branch remains frozen and untouched.
Source data is captured for 2026-10-01, not newly observed on import day.
PROVEN: input/output byte identities and current Item bindings are checked.
DERIVED: community-selected type/tier data retain their qualifications.
UNKNOWN/CONFLICT: public research gaps and architecture reconciliation stay visible.
Owner selections do not become public Global certification.

High-risk authority/recovery: NOT_APPLICABLE, static data only; no production,
runtime activation, identity admission, persistence or protocol mutation.

## Implementation

Separate ruleset registration, never a false ContentFamily. Import validates
reviewed source schema, current bindings, eligibility reproduction and runtime
flags; the index and content lock pin emitted files. Legacy tree regeneration
reproduces the import. CI checks generation and negative cases.

Held: 2 unregistered equipment Items, 4 missing eligibility records, 30 retired
source Items. Basic tier One codec support and typed runtime quest bindings are
separate owning lanes. All 72 tiers remain in the static catalogue.
Shared generated registry paths overlap the owner-directed Quest import; #162
was notified before publication. Integration must rebuild from current main and
preserve both registries. The coordinator owns paid review and Merge Queue.

## Validation and closeout

Local prepublication checks PASS: 238 authoring/import tests, 36 governance
tests, materialized tree 97/97, tree equivalence/replay, regeneration helper
4 checks, deterministic regeneration and import --check. Qualification repeats
on the frozen exact head.
Independent exact-head review and CI evidence are recorded in the PR/#162 after
freeze. This document cannot contain its own commit SHA. No merge or activation
is claimed by the data import. See `server-data-import.md` for reproducible commands.

## D312 reconciliation

The owner routed canonical-binding drift back to the imbuement lane on 2026-10-03.
156 recipe/scroll/utility/shrine Items and 72 cumulative recipe/tier rows were
compared against accepted main. Four Items have real materialization/physical/stack
enrichment; recipe identities and quantities are unchanged. Eligibility regeneration
changes canonical fingerprints, not source-selected slots/types/tiers. Preserve
accepted main fields and the workflow union, then regenerate both evidence packets,
their two EVIDENCE_PINS and the catalogue/ruleset/lock cascade. Final publication and
qualification must follow the content carrier (#1599+#1630+#1433); the integration
writer retains sole remote-branch ownership until transferred by the control plane.
