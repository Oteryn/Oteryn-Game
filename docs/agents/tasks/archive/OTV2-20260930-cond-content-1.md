# OTV2-20260930-cond-content-1

```yaml
task_id: OTV2-20260930-cond-content-1
title: "COND-CONTENT-1 ConditionDefinition authoring package"
mode: CONTENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/cond-content-1
pr: 1400
base_sha: a6a054e
head_sha: "the last #162 FREEZE_SHA entry for PR #1400 (a commit cannot name its own SHA)"
final_head_sha: "the last #162 FREEZE_SHA entry for PR #1400 (a commit cannot name its own SHA)"
final_head_frozen_at: null
owner: claude-code-session-01GSyX7KuFE9reL9gHbJ8if1 (content worker, #162 allocation)
created_at: 2026-09-30
updated_at: 2026-09-30
execution_policy: continuous_progress
owned_paths:
  - tools/content-schema/condition-authoring/**
  - docs/agents/tasks/archive/OTV2-20260930-cond-content-1.md
public_contracts: []
depends_on:
  - "CONDITIONS0-ACTOR-CONDITIONS-V1 (candidate decision; §3 families, keys and rows)"
blocks:
  - "content train car populating content/conditions/ (control plane)"
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

The authoring package for the CONDITIONS-0 `ConditionDefinition` records: schema, hand-authored rows
with Canary `04b83b51` `path:lines` evidence, the captured source facts, the generator (`build`,
`validate`, `content [--check]`) and its tests. 842 definitions: 828 admitted, 14 blocked
(fail-closed). The admitted definitions are 23 authored rows (spells, runes, potions, food, charms),
22 fields and 783 monster attacks mirrored from the committed monster Effects.

No `content/**`, manifest, lock, `tools/content-migration/**`, workflow or runtime change is made;
the control plane's content train runs `content` and registers the family.

## Architecture and source of truth

- `PROVEN`: CONDITIONS-0 §3 and §8 (candidate); ITEM-USE-0 §6.1; the committed monster Effects in
  `content/abilities/`.
- `DERIVED`: Canary `04b83b51` Lua, C++ and `items.xml`; the monster conversion from Canary `47dfd51f`
  (`OTS_HYPOTHESIS_ONLY`).

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: authoring tool and candidate data under `tools/content-schema/` only.

## Acceptance criteria

- [ ] Exact frozen head with passing tool checks and governance validation.
- [ ] Control-plane review of the package and the owner questions on #162.
- [ ] Protected Merge Queue integration.

## Excluded scope

- The COND-1 runtime, `content/conditions/` population and registration, a CI workflow (owner
  authorization pending on #162), drunk, invisible, outfit, attribute, fear and root conditions.

## Validation

- `python condition_authoring.py build --check`: PASS.
- `python condition_authoring.py validate samples/conditions-candidate.json`: PASS.
- `python test_condition_authoring.py`: PASS (10 tests).
- `ruff check .` and `ruff format --check .` (ruff 0.16.1): PASS.
- `python condition_authoring.py capture --canary <04b83b51 checkout>`: every cited needle found.
- `python3 tools/agents/validate_governance.py`: PASS on the final authoring tree.
- `git diff --check`: clean.

## Closeout

- PR: #1400 (https://github.com/Oteryn/Oteryn-Game/pull/1400). Merge commit/result: its squash merge.
- Owner and control-plane questions are on #162 (blocked coefficients, searing fire, Holy Flash,
  the CI workflow, the package size).
- Codex review of `3def6299` (3 findings: P1 unpinned Canary capture, P2 geometric schedule
  invariants, P2 obsolete shards): all fixed in `7e46cd9f` with tests.
- Codex review of `7e46cd9f` (1 finding: P2 placeholder PR in this record): fixed in the next
  candidate.
- This record was archived in the PR's final authoring commit (`docs/agents/tasks/archive/README.md`).

```yaml
last_progress: final authoring commit; archived before freeze
status: completed
branch: claude/cond-content-1
owner_action_required: null
blocker: null
next_action: null
```
