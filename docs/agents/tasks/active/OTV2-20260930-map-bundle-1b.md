# OTV2-20260930-map-bundle-1b

```yaml
task_id: OTV2-20260930-map-bundle-1b
title: "MAP-BUNDLE-1b World Bundle compiler: families, teleport split, parity, key resolution"
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/map-bundle-1b
issue: 162
pr: null   # 1b-1: the PR named in the #162 FREEZE_SHA entry; 1b-2 not opened
base_sha: 7be0677
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: null
final_head_frozen_at: null
owner: claude-code task worker (hard), allocated by the #162 control plane
created_at: 2026-09-30
updated_at: 2026-09-30
execution_policy: continuous_progress
owned_paths:
  - tools/world-bundle-compiler/**
  - docs/contracts/OTERYN_WORLD_BUNDLE_FORMAT_V1.md
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json (MAP-BUNDLE rows only)
  - docs/agents/tasks/active/OTV2-20260930-map-bundle-1b.md
public_contracts:
  - docs/contracts/OTERYN_WORLD_BUNDLE_FORMAT_V1.md
depends_on:
  - MAP-BUNDLE-1a (#1338, merged)
  - "#1160 and #1170 (open; read-only evidence for the parity run)"
blocks:
  - MAP-LOAD-1
cross_repository_coordination_id: null
external_repositories: []
```

## Split

The scope is above the ~500-line batch guide, so it lands in two PRs:

- **1b-1 (this PR).** Placements checked against the World, Transition.Teleport and House
  families and the minimap draft areas (`project.rs`); the Q3 teleport split in the compiler,
  with the `dropped_teleports` manifest field; the `parity` report and command; OPEN-3 counts
  and the OPEN-4 decision in the format document; limits confirmed on the real map; the
  `boundary_tests` wording of the four `ReadCaps` rows (carried LOW, 5913062154).
- **1b-2 (next).** Key resolution against the Item registry and the Terrain and WorldObject
  catalogues under A12 §4.6 (Q1b, Q2a), the revision-scoped compact ids, the `compile` command
  and the tile-by-tile source-to-bundle equivalence on the real map. It needs WO-2b `routed_to`
  on the Item records or a ruling to use the catalogues' `item_pointer`, and the #1170 palette
  regenerated to canonical A12 keys.

## Evidence

- Parity run: `oteryn-world-bundle-compiler parity` on #1170 head `98ba6938` (stacked on #1160,
  872 Transition.Teleport records) with the House catalogue of `main` (995 records), 2.4 s:
  2,455 teleport attributes; 872 matched; 1,577 zero destination (dropped); 6 real destination
  without a record (fail); 0 mismatched; 0 records without an attribute; 0 unknown house ids;
  19,373,519 tiles; 24,983,331 entries; at most 26 entries per tile; longest text 3,859 bytes;
  7 draft areas.

## Validation

- `cargo fmt --check`; `cargo clippy --locked -p oteryn-world-bundle-compiler --all-targets --
  -D warnings`; `cargo test --locked -p oteryn-world-bundle-compiler` (15 tests).
- `python3 tools/agents/validate_governance.py`; `python3
  tools/repository/validate_repository_policy.py`; `git diff --check`.

## Context checkpoint

```yaml
last_progress: 1b-1 authored and frozen; FREEZE_SHA posted on #162
status: implementing
branch: claude/map-bundle-1b
owner_action_required: null
blocker: "1b-2 needs WO-2b routed_to (or an architect ruling on item_pointer) and #1170 merged with canonical A12 keys; the real map compiles only after the 6 real-destination orphan teleports are fixed in content"
next_action: "independent exact-head review of 1b-1 (control plane)"
```
