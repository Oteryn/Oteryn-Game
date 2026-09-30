# OTV2-20260930-mergeable-content-tree

```yaml
task_id: OTV2-20260930-mergeable-content-tree
title: Content tree Amendment 01, mergeable layout and the content integration train
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
issue: 162
base_branch: main
branch: claude/modest-sagan-lqsb08
pr: null  # the PR on this branch is authoritative
base_sha: f38d7f3
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: "Oteryn: content tree layout (owner session)"
created_at: 2026-09-30T15:00:00Z
updated_at: 2026-09-30T15:30:00Z
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/OTERYN_FULL_GAME_CONTENT_AND_RULESET_TREE_V1_AMENDMENT_01.md
  - docs/agents/tasks/archive/OTV2-20260930-mergeable-content-tree.md
public_contracts:
  - OTERYN_FULL_GAME_CONTENT_AND_RULESET_TREE_V1 (amendment candidate)
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

A candidate amendment that lets content PRs changing different records merge without
conflicts: key-addressed JSON Lines shards, count-free indexes, a per-shard lock with a
derived package digest, and regenerate-and-compare validation instead of hand-written
pins. It also sets the interim content integration train. Documents only.

## Architecture and source of truth

- `OTERYN_FULL_GAME_CONTENT_AND_RULESET_TREE_V1.md` section 10: consumers switch to the
  successor tree, then the monolith retires (**PROVEN**).
- Conflict causes measured on #1363 and #1351 (**PROVEN**, section 1 of the amendment).
- Feasibility: #162 comments 5913960307 and 5913981487.
- Determinism of census and stage, and chain run time: **UNKNOWN**; not needed by this
  document, needed by slice 2.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: documents only; no mutation, fence, authority or recovery path.

## Acceptance criteria

- [x] Amendment states problem evidence, options, decision, interim operation, slices,
  exclusions and acceptance.
- [ ] Independent review on the exact frozen head.

## Excluded scope

No generator, loader, runtime, CI, test or content change. No change to
`content/world/**` or WorldProject v2 contracts.

## Owner decisions (owner session 2026-09-30)

- Integration train as interim operation: accepted.
- Prepare the mergeable-layout amendment for review: accepted.
- Open: option C (generated content not committed) deferred; revisit only if D is
  insufficient.

## Validation

### Focused

- command/run: `python3 tools/agents/validate_governance.py`, `python3 tools/repository/validate_repository_policy.py`
- result: both pass

### Component/integration

- `NOT_APPLICABLE`: documents only.

### E2E

- `NOT_APPLICABLE`: documents only.

## Independent review

- required: YES, the amendment changes an accepted content contract's physical layout.

## PR and closeout

- merge commit/result: squash merge of the PR on this branch
- ownership release: on merge
