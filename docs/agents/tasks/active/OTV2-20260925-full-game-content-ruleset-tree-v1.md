---
task_id: OTV2-20260925-full-game-content-ruleset-tree-v1
title: Full Game Content and Ruleset Tree v1
mode: CONTRACT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/full-game-content-ruleset-tree-v1-20260925
base_sha: 2389c6671000b8b0efe341540a62e303e307ad15
issue: 162
pr: 904
jira: KAN-16
allocation_comment: 5835217784
created_at: 2026-09-25T17:30:00+02:00
owned_paths:
  - docs/agents/tasks/active/OTV2-20260925-full-game-content-ruleset-tree-v1.md
  - docs/architecture/OTERYN_FULL_GAME_CONTENT_AND_RULESET_TREE_V1.md
  - docs/agents/evidence/OTV2-20260925-full-game-content-ruleset-tree-v1.json
  - tools/content-schema/validate_full_game_content_tree.py
  - tools/content-schema/test_validate_full_game_content_tree.py
---

# Full Game Content and Ruleset Tree v1

## Outcome

Define one complete human-readable target tree for Oteryn game data while preserving
semantic ownership boundaries.

The tree covers:

- static authored world/content definitions;
- reusable rulesets and progression definitions;
- economy rules;
- Platform-owned commercial/Store boundaries;
- runtime/durable state ownership that must not be serialized as static content;
- import/provenance inputs;
- current TibiaWiki structured domain/system coverage.

## Acceptance

- all 19 protected wiki-wide domain groups are assigned;
- all current system surfaces selected by this task have one explicit owner/disposition;
- Store/commercial data remains Platform-owned;
- static definition, ruleset and durable/runtime state are never conflated;
- current WorldProject/v2 remains untouched in this slice;
- validator reports `unassigned_domains=0` and `unassigned_systems=0`.

## Boundaries

This task is architecture/schema/validator only. It does not move or rewrite
`content/world/**`, does not migrate the 38,157 Item corpus, does not change
runtime/compiler/protocol/persistence code and does not create commercial data in
Oteryn-Game.

The target tree is the owner-directed successor authoring organization candidate.
Physical migration requires a separately qualified migration slice with round-trip
proof and protected-main readback.

PR #903 Item Master Schema is complementary Item-detail evidence, not a merge
dependency for this directory/ownership contract.

## Authoring readback

Pre-freeze remote readback before canonical PR binding:

- five allocated paths only;
- protected domains: 19/19;
- selected system surfaces: 39/39;
- target Game tree nodes: 100 unique paths;
- Store owner: Oteryn/Oteryn-Platform with no Game static/ruleset path;
- durable state scopes: ItemInstance, Character, Account, World/Channel, House, Market;
- `unassigned_domains=0`;
- `unassigned_systems=0`.

Canonical PR: #904. The successor commit containing this task metadata is the candidate
that must receive fresh exact-head validation.
