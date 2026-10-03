# OTV2-20261001-charm-ordered-effect-plan

```yaml
task_id: OTV2-20261001-charm-ordered-effect-plan
title: Preserve declared sequential effect indices
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/charm-ordered-effect-plan-20261001
pr: 1498
issue: 162
base_sha: e225b3f76e152d195f75cb757b3d639a3ff5577a
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Codex root, sole publisher; damage source preparer
created_at: 2026-10-01
updated_at: 2026-10-01
owned_paths:
  - apps/game-server/src/ability/plan.rs
  - apps/game-server/src/foundation/channel_owner_ability_commit_tests.rs
  - apps/game-server/tests/ability_engine.rs
  - docs/agents/tasks/archive/OTV2-20261001-charm-ordered-effect-plan.md
public_contracts: []
depends_on: [ABILITY-0, 1476]
blocks: []
external_repositories: []
```

## Bounded result

EffectPlan::ordered_sequential preserves caller-declared effect indices while sharing every
existing identifier, target, magnitude, count, stage and retained-byte validation. Calculation
stages remain canonical. Existing new/immediate constructors remain unchanged in behavior.
This enables a committed primary prefix at index0 and post-hit Charm descendant at index1.
No new resource limit, owner, protocol, persistence or gameplay activation is introduced.
The natural prerequisite split contains98 changed handwritten Rust lines across three paths.

## Validation and self-review

Native order comparison1PASS; full server library1280PASS,2existing ignored; standalone Ability
order comparison1PASS. Workspace fmt, diff check, strict workspace all-target Clippy and
governance/lifecycle13PASS. The initial all-target unused-constructor diagnostic in the standalone
Ability suite was repaired by extending its existing meaningful order oracle; all prior assertions
were retained. Production source stayed unchanged during that fixture repair.

AuthorityInvariant x ConsumerBoundary x MutationOperator is NOT_APPLICABLE to this pure plan
constructor: it grants no current authority and writes no runtime or durable state. Its existing
validation remains shared; the actual descendant owner mutation is the next independently
reviewed child. Root inspected the entire delta and both canonical and declared order assertions.

The full nine-row Charm task remains IMPLEMENTING. Root publishes actual local Git identity via
guarded normal pushes; final freeze, independent source review and exact-head CI are external
PR packets. Protected integration is control-plane owned. This record reaches main only if
PR1498 merges; a commit cannot contain its own final SHA.
