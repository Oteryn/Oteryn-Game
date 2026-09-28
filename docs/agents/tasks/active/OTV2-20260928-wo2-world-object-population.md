# OTV2-20260928-wo2-world-object-population

```yaml
task_id: OTV2-20260928-wo2-world-object-population
title: "WO-2 slice 1 - Terrain population by the D93 key rule"
mode: MIGRATE
status: ready
repository: Oteryn/Oteryn-Game
issue: 162
pr: null
base_branch: main
branch: claude/wo2-world-object-population
base_sha: ff498b5
head_sha: null
final_head_sha: null
owner: Content/World lane (Claude Code)
created_at: 2026-09-28
updated_at: 2026-09-28
execution_policy: continuous_progress
owned_paths:
  - content/world/terrain/**
  - content/world/objects/**
  - tools/content-schema/world-object-authoring/**   # one new emitter and its test only
  - docs/agents/tasks/active/OTV2-20260928-wo2-world-object-population.md
public_contracts: []
depends_on:
  - "docs/architecture/reviews/OTERYN_GAME_WO0_WORLD_OBJECT_AND_TERRAIN_AUTHORING_FORMAT_DECISION_2026-09-28.md"
  - "tools/content-schema/world-object-authoring/README.md (WO-1, #1169)"
blocks: [WO-2 slice 2 (WorldObject), WO-3]
cross_repository_coordination_id: null
external_repositories:
  - "zimbadev/crystalserver@ff7ede593c69d4c658b382c97443e8155926924a (pinned facts, read-only)"
```

## Outcome

Slice 1 of WO-2 mints the Terrain family only. The full population is 21,370 records (32 MB of
JSON), too large to review as one PR, so it is split by family. WorldObject (12,789 routed
records) is slice 2 and needs its own allocation.

- **Keys.** `oteryn:terrain.registry.iNNNNNNNN` is the D93 pure function of the frozen CW2-B1 Item
  key. The emitter cross-checks each id against `imports/crystalserver/bindings/items.json`. No
  allocator, no renumbering, and no Item key changes.
- **Facts.** Records are the WO-1 builders' output, unchanged, and each passes the WO-1 validator.
  Unset facts stay UNKNOWN. Upstream (Crystal, `OTS_HYPOTHESIS_ONLY`) is facts only.
- **Minted: 8,121 Terrain records** in 17 shards of at most 500, ascending source id: border 3,755,
  ground 2,091, wall 2,171, field 104. Five walls have no client appearance and keep UNKNOWN
  appearance facts.
- **Held: 460 ids, no key.** They are listed by their existing Item key in
  `content/world/terrain/definitions/held.json`.
  - `kind_unresolved` 330: roofs, floors and shallow water with no ground, border, wall or field
    rule. The kind is a closed typed field, and a family key is frozen once minted.
  - `type_outside_family` 135: `items.xml` type trashholder (106), carpet (18) and teleport (11)
    routed to Terrain, though WO-0 §4.3 assigns those types to WorldObject or Interaction. This is
    the family itself, so it is key-affecting.
  - Five ids carry both reasons.
- **Layout (proposal, not fixed by WO-0).** `content/world/terrain/definitions/` holds
  `terrain-NNNNN-NNNNN.json` shards, an `OTERYN_FAMILY_INDEX/v1` `index.json`, and `held.json`.
  Shards are compact sorted-key JSON, in the other family trees' envelope.
- **New tooling.** `populate_content.py` (emitter, holds, `--check`) and
  `test_populate_content.py`. WO-1 has no content emitter, and no existing WO-1 file is edited.

## Determinism

Two full regenerations into separate roots are byte-identical (`diff -r` empty). `--check` diffs the
committed files against an in-memory regeneration. The WO-1 census `--check` is unchanged and PASS.

## Excluded scope and follow-ups

- **WorldObject slice.** Also holds the 21 `magicfield` ids routed to WorldObject, in conflict with
  the 104 Terrain fields.
- **Item `routed_to` pointers.** They need the Rust `ProjectReferenceRecord::Item` amendment and
  `DefinitionFamily::WorldObject`: `SHARED_LEASE_REQUIRED` (`apps/game-server/**`).
- **Marker state.** `content/world/terrain/index.json` stays `READY_UNPOPULATED`. Only its notes are
  updated, because `validate_materialized_game_tree.py`, the Rust tree test and the materialization
  evidence admit only `READY_UNPOPULATED` or `LEGACY_COMPAT_PRESENT`. A populated state needs a
  shared lease.
- **CI wiring.** `test_populate_content.py` is not yet a CI step, since that needs `.github/**`.
- **Relations.** `source_item_id` relations (rotateto, bed parts) resolve in the WorldObject slice.
- **Donor ids.** B1b epoch-2 donor ids are outside this Crystal slice.
- **Holds.** They need an owner or architect call, then a later additive mint.
- **Identity review.** WO-2 requires independent identity review of the exact frozen head.

## Validation

Bound to the frozen final head of the PR.

- `python3 tools/content-schema/world-object-authoring/test_populate_content.py`: PASS, 50,640 checks.
- `python3 tools/content-schema/world-object-authoring/test_world_objects.py`: PASS, 140 checks.
- `python3 tools/content-schema/world-object-authoring/world_objects.py --source <ff7ede5> --check`: PASS, 21,370.
- `python3 tools/content-schema/world-object-authoring/populate_content.py --source <ff7ede5> --family Terrain --check`: PASS.
- `python3 tools/content-schema/validate_materialized_game_tree.py`: PASS.
- `cargo test -p oteryn-game-server --test content_world_project_repository full_game_tree_contract_nodes_are_materialized`: see the PR.
- `python3 tools/agents/validate_governance.py`, `python3 tools/repository/validate_repository_policy.py`, `git diff --check`: see the PR.

## Context checkpoint

```yaml
last_progress: Terrain slice generated, deterministic, validated
status: ready
branch: claude/wo2-world-object-population
pr: null
owner_action_required: "decide the 460 Terrain holds; allocate WO-2 slice 2 (WorldObject) and the shared lease for marker state and Item routed_to"
blocker: null
next_action: exact-head freeze, independent identity review, Merge Queue
```
